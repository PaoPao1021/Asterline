mod attach;
mod attachments;
mod bridge;
mod catalog;
mod composer;
mod diagnostics;
mod platform_env;
mod recent;
mod session_adapter;
mod update_check;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use asterline::domain::event::RuntimeEvent;
use asterline::tui::skills::{self, SkillInfo};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WindowEvent};

use crate::attach::{AttachCapabilitiesV2, ExternalAttachLaunchV2};
use crate::attachments::{AttachmentRegistry, StagedAttachmentV2};
use crate::bridge::{
    DESKTOP_EVENT_CHANNEL, DesktopCommandV2, DesktopEventV2, DesktopModel, DesktopPhase,
    DesktopRuntimeEventV2, DesktopSnapshotV2, DiffResultV2, LogEntryV2, LogLevelV2, RunStatusV2,
    SkillSummaryV2, envelope,
};
use crate::catalog::{BackendAvailabilityV2, ModelSummaryV2};
use crate::composer::{CompletionV2, ComposerActionV2};
use crate::diagnostics::{Diagnostics, DiagnosticsStatusV2};
use crate::recent::RecentWorkspaceV2;
use crate::session_adapter::{
    ActiveSession, BootstrapAdapterOutcome, DesktopLaunchOptionsV2, PendingTeamSetup,
    PreparedSession,
};
use crate::update_check::DesktopUpdateV2;

/// Desktop composer budget, mirrored from the TUI composer.
const MAX_COMPOSER_BYTES: usize = 256 * 1024;

#[derive(Default)]
struct HostInner {
    model: DesktopModel,
    active: Option<ActiveSession>,
    pending: Option<Box<PendingTeamSetup>>,
    workspace: Option<PathBuf>,
    generation: u64,
    sequence: u64,
    attaching: std::collections::HashSet<String>,
    attach_waiters: std::collections::HashMap<String, mpsc::SyncSender<Result<(), String>>>,
    attachments: AttachmentRegistry,
    skills: Vec<SkillInfo>,
}

#[derive(Default)]
struct DesktopHost {
    inner: Mutex<HostInner>,
}

impl DesktopHost {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HostInner>, String> {
        self.inner
            .lock()
            .map_err(|_| "desktop host state is unavailable".to_string())
    }
}

#[tauri::command]
fn bootstrap_desktop(
    app: AppHandle,
    state: State<'_, DesktopHost>,
    workspace: Option<String>,
    options: Option<DesktopLaunchOptionsV2>,
) -> Result<DesktopSnapshotV2, String> {
    let Some(workspace) = workspace else {
        return Ok(state.lock()?.model.snapshot());
    };
    let options = options.unwrap_or_default();
    // Debug-build escape hatch for smoke tests: run the offline fake agents
    // without walking the advanced launcher. Release builds ignore it, and it
    // is never persisted.
    #[cfg(debug_assertions)]
    let options = {
        let mut options = options;
        if std::env::var_os("ASTERLINE_DESKTOP_FAKE").as_deref() == Some(std::ffi::OsStr::new("1"))
        {
            options.fake = true;
        }
        options
    };
    // Debug and fake launches are session-scoped: they never persist anywhere.
    let workspace = validate_workspace(&workspace)?;

    let (active, generation) = {
        let mut inner = state.lock()?;
        if inner.workspace.as_deref() == Some(workspace.as_path())
            && (inner.active.is_some() || inner.pending.is_some())
        {
            return Ok(inner.model.snapshot());
        }
        inner.generation = inner.generation.saturating_add(1);
        inner.pending = None;
        inner.attach_waiters.clear();
        inner.attachments.discard_all();
        inner.workspace = Some(workspace.clone());
        let previous_sequence = inner.sequence;
        inner.model = DesktopModel::default();
        inner.model.snapshot.sequence = previous_sequence;
        inner
            .model
            .set_workspace(workspace.to_string_lossy().into_owned());
        let event = inner.model.set_phase(DesktopPhase::Starting, None);
        emit_locked(&app, &mut inner, event);
        (inner.active.take(), inner.generation)
    };
    if let Some(active) = active {
        active.shutdown();
    }

    match session_adapter::bootstrap_workspace(workspace.clone(), &options) {
        Ok(BootstrapAdapterOutcome::Ready(prepared)) => {
            install_prepared(&app, &state, *prepared, generation)?;
            if let Err(error) = recent::record(&app, &workspace) {
                emit_notice_for_generation(
                    &app,
                    generation,
                    format!("workspace opened, but could not save it to recent projects: {error}"),
                );
            }
        }
        Ok(BootstrapAdapterOutcome::NeedsTeamSetup { pending, draft }) => {
            let mut inner = state.lock()?;
            if inner.generation != generation {
                return Err("workspace changed while it was opening".to_string());
            }
            inner.model.set_team(*draft);
            inner.pending = Some(pending);
            let event = inner.model.set_phase(DesktopPhase::NeedsTeamSetup, None);
            emit_locked(&app, &mut inner, event);
        }
        Err(error) => {
            let message = error.to_string();
            let phase = if is_lock_error(&message) {
                DesktopPhase::Locked
            } else {
                DesktopPhase::Error
            };
            let mut inner = state.lock()?;
            if inner.generation != generation {
                return Err("workspace changed while it was opening".to_string());
            }
            let event = inner.model.set_phase(phase, Some(message));
            emit_locked(&app, &mut inner, event);
        }
    }
    Ok(state.lock()?.model.snapshot())
}

#[tauri::command]
fn get_desktop_snapshot(state: State<'_, DesktopHost>) -> Result<DesktopSnapshotV2, String> {
    Ok(state.lock()?.model.snapshot())
}

/// Parse composer text with the shared contract parser. The WebView keeps no
/// parsing rules of its own; unknown commands stay draftable and report a
/// structured error.
#[tauri::command]
fn parse_composer_text(
    state: State<'_, DesktopHost>,
    text: String,
) -> Result<ComposerActionV2, String> {
    if text.len() > MAX_COMPOSER_BYTES {
        return Err(format!(
            "composer input is limited to {} KiB",
            MAX_COMPOSER_BYTES / 1024
        ));
    }
    let inner = state.lock()?;
    Ok(composer::parse_composer_text(
        &text,
        &inner.model.snapshot.members,
        &inner.skills,
    ))
}

/// Shared completion (commands, modes, members, targeted skills).
#[tauri::command]
fn complete_composer(
    state: State<'_, DesktopHost>,
    head: String,
) -> Result<Option<CompletionV2>, String> {
    let inner = state.lock()?;
    Ok(composer::complete_composer(
        &head,
        &inner.model.snapshot.members,
        &inner.skills,
    ))
}

/// The shared command catalog for palettes and `/help`.
#[tauri::command]
fn command_catalog() -> Vec<composer::CommandSpecV2> {
    composer::command_catalog()
}

#[tauri::command]
fn get_backend_availability() -> BackendAvailabilityV2 {
    catalog::backend_availability()
}

#[tauri::command]
fn list_models(backend: bridge::BackendKindV2, cwd: String) -> Result<Vec<ModelSummaryV2>, String> {
    catalog::list_models(backend, &cwd)
}

#[tauri::command]
fn list_native_sessions(
    backend: bridge::BackendKindV2,
    cwd: String,
) -> Result<Vec<asterline::tui::native_sessions::NativeSessionSummary>, String> {
    Ok(catalog::list_native_sessions(backend, &cwd))
}

// --- attachment lifecycle ---------------------------------------------------

#[tauri::command]
fn stage_clipboard_image(state: State<'_, DesktopHost>) -> Result<StagedAttachmentV2, String> {
    let workspace = current_workspace(&state)?;
    let image = asterline::tui::clipboard_image::paste_clipboard_image(&workspace)?;
    let mut inner = state.lock()?;
    inner.attachments.stage(image)
}

#[tauri::command]
fn stage_image_path(
    state: State<'_, DesktopHost>,
    path: String,
) -> Result<StagedAttachmentV2, String> {
    let workspace = current_workspace(&state)?;
    let image = asterline::tui::clipboard_image::import_image_file(&workspace, Path::new(&path))?;
    let mut inner = state.lock()?;
    inner.attachments.stage(image)
}

#[tauri::command]
fn stage_image_bytes(
    state: State<'_, DesktopHost>,
    bytes_base64: String,
) -> Result<StagedAttachmentV2, String> {
    let bytes = attachments::decode_base64(&bytes_base64)
        .ok_or_else(|| "attachment payload is not valid base64".to_string())?;
    if bytes.is_empty() {
        return Err("attachment is empty".to_string());
    }
    if bytes.len() > asterline::adapter::prompt_images::MAX_IMAGE_BYTES {
        return Err("images are limited to 10 MiB".to_string());
    }
    let workspace = current_workspace(&state)?;
    let image = asterline::tui::clipboard_image::persist_image_bytes(&workspace, &bytes)?;
    let mut inner = state.lock()?;
    inner.attachments.stage(image)
}

#[tauri::command]
fn remove_staged_attachment(state: State<'_, DesktopHost>, token: String) -> Result<(), String> {
    let mut inner = state.lock()?;
    inner.attachments.remove(&token)
}

/// Drop every staged attachment and wipe this session's managed paste
/// directory (workspace switch, new session, or close).
#[tauri::command]
fn discard_staged_attachments(state: State<'_, DesktopHost>) -> Result<(), String> {
    let mut inner = state.lock()?;
    inner.attachments.discard_all();
    Ok(())
}

fn current_workspace(state: &State<'_, DesktopHost>) -> Result<String, String> {
    state
        .lock()?
        .workspace
        .as_ref()
        .map(|workspace| workspace.to_string_lossy().into_owned())
        .ok_or_else(|| "open a workspace before attaching images".to_string())
}

// --- runtime commands -------------------------------------------------------

#[tauri::command]
fn dispatch_desktop_command(
    app: AppHandle,
    state: State<'_, DesktopHost>,
    command: DesktopCommandV2,
) -> Result<(), String> {
    if matches!(command, DesktopCommandV2::Shutdown) {
        return shutdown_desktop(app, state);
    }

    if matches!(
        command,
        DesktopCommandV2::RequestLogs { .. }
            | DesktopCommandV2::RequestDiff { .. }
            | DesktopCommandV2::RequestSkills { .. }
    ) {
        let mut inner = state.lock()?;
        let event = utility_query(&inner, command)?;
        emit_locked(&app, &mut inner, event);
        return Ok(());
    }

    // Clearing a mode's conversation overrides is computed against the live
    // snapshot (the runtime only accepts full replacement maps).
    let command = match command {
        DesktopCommandV2::ResetModeOverrides { mode } => {
            let inner = state.lock()?;
            let mut overrides = inner.model.snapshot.mode_overrides.clone();
            bridge::clear_mode_overrides(&mut overrides, mode);
            let workspace = inner
                .workspace
                .as_deref()
                .ok_or_else(|| "open a workspace before sending commands".to_string())?;
            let runtime_command = session_adapter::command_to_runtime(
                DesktopCommandV2::SetModeOverrides { overrides },
                workspace,
                &|token| inner.attachments.resolve(token),
            )?;
            inner
                .active
                .as_ref()
                .ok_or_else(|| "the workspace runtime is not ready".to_string())?
                .send(runtime_command)?;
            return Ok(());
        }
        command => command,
    };

    {
        let inner = state.lock()?;
        if !inner.attaching.is_empty() && command_can_dispatch_backend(&command) {
            return Err(format!(
                "finish the native session before dispatching runtime work ({})",
                attaching_members(&inner)
            ));
        }
    }

    let pending_settings = match &command {
        DesktopCommandV2::ReplaceTeamSettings { settings } => Some(settings.clone()),
        _ => None,
    };
    if let Some(settings) = pending_settings {
        let (pending, generation) = {
            let mut inner = state.lock()?;
            (inner.pending.take(), inner.generation)
        };
        if let Some(pending) = pending {
            match pending.start(&settings) {
                Ok(prepared) => {
                    let workspace = prepared.workspace.clone();
                    install_prepared(&app, &state, prepared, generation)?;
                    if let Err(error) = recent::record(&app, Path::new(&workspace)) {
                        emit_notice_for_generation(
                            &app,
                            generation,
                            format!(
                                "workspace opened, but could not save it to recent projects: {error}"
                            ),
                        );
                    }
                    return Ok(());
                }
                Err(error) => {
                    let mut inner = state.lock()?;
                    let event = inner
                        .model
                        .set_phase(DesktopPhase::Error, Some(error.to_string()));
                    emit_locked(&app, &mut inner, event);
                    return Err(error.to_string());
                }
            }
        }
    }

    let mut inner = state.lock()?;
    let workspace = inner
        .workspace
        .as_deref()
        .ok_or_else(|| "open a workspace before sending commands".to_string())?;
    let resolves_paused = matches!(
        &command,
        DesktopCommandV2::ResolvePausedRoute { resume: true }
    );
    // Sending a message consumes its staged attachments: they belong to the
    // persisted prompt now and are cleaned up with the session, like the TUI.
    let consumes_attachments = matches!(&command, DesktopCommandV2::UserMessage { attachments, .. } if !attachments.is_empty());
    let runtime_command = session_adapter::command_to_runtime(command, workspace, &|token| {
        inner.attachments.resolve(token)
    })?;
    inner
        .active
        .as_ref()
        .ok_or_else(|| "the workspace runtime is not ready".to_string())?
        .send(runtime_command)?;
    if consumes_attachments {
        inner.attachments.retain_only(&[]);
    }
    if resolves_paused {
        inner.model.clear_next_paused_route();
    }
    Ok(())
}

fn utility_query(
    inner: &HostInner,
    command: DesktopCommandV2,
) -> Result<DesktopRuntimeEventV2, String> {
    let workspace = inner
        .workspace
        .as_deref()
        .ok_or_else(|| "open a workspace before requesting utility data".to_string())?;
    match command {
        DesktopCommandV2::RequestLogs {
            request_id,
            member,
            level,
            query,
            limit,
        } => {
            let query = query.unwrap_or_default().to_lowercase();
            let max = limit.unwrap_or(400).clamp(1, 400);
            let mut entries = inner
                .model
                .logs()
                .iter()
                .filter(|entry| member.as_deref().is_none_or(|value| value == entry.source))
                .filter(|entry| level.as_ref().is_none_or(|value| value == &entry.level))
                .filter(|entry| {
                    query.is_empty()
                        || entry.source.to_lowercase().contains(&query)
                        || entry.message.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>();
            let truncated = entries.len() > max;
            if truncated {
                let start = entries.len() - max;
                entries = entries.split_off(start);
            }
            Ok(DesktopRuntimeEventV2::LogsReplaced {
                request_id,
                entries,
                truncated,
            })
        }
        DesktopCommandV2::RequestDiff { request_id } => {
            let text = asterline::tui::compute_git_diff(&workspace.to_string_lossy());
            let truncated = text.contains("[diff output truncated at 2 MiB]")
                || text.contains("[untracked file list truncated]");
            let file_count = text
                .lines()
                .filter(|line| line.starts_with("diff --git ") || line.starts_with("  "))
                .count();
            Ok(DesktopRuntimeEventV2::DiffReplaced {
                request_id,
                result: DiffResultV2 {
                    text,
                    truncated,
                    file_count,
                },
            })
        }
        DesktopCommandV2::RequestSkills {
            request_id,
            backend,
            query,
            limit,
        } => {
            let query = query.unwrap_or_default().to_lowercase();
            let max = limit.unwrap_or(512).clamp(1, 512);
            let mut skills = skills::discover(Path::new(workspace))
                .into_iter()
                .filter(|skill| {
                    backend
                        .as_ref()
                        .is_none_or(|value| bridge::backend_kind(skill.backend.as_str()) == *value)
                })
                .filter(|skill| {
                    query.is_empty()
                        || skill.name.to_lowercase().contains(&query)
                        || skill.description.to_lowercase().contains(&query)
                        || skill.invocation.to_lowercase().contains(&query)
                })
                .map(|skill| SkillSummaryV2 {
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    backend: bridge::backend_kind(skill.backend.as_str()),
                    invocation: skill.invocation.clone(),
                })
                .collect::<Vec<_>>();
            let truncated = skills.len() > max;
            skills.truncate(max);
            Ok(DesktopRuntimeEventV2::SkillsReplaced {
                request_id,
                skills,
                truncated,
            })
        }
        _ => Err("unsupported utility query".to_string()),
    }
}

#[tauri::command]
fn shutdown_desktop(app: AppHandle, state: State<'_, DesktopHost>) -> Result<(), String> {
    shutdown_inner(&app, &state)?;
    Ok(())
}

/// `/exit`: gracefully stop the runtime, then close the window.
#[tauri::command]
fn exit_desktop(app: AppHandle, state: State<'_, DesktopHost>) -> Result<(), String> {
    shutdown_inner(&app, &state)?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    } else {
        app.exit(0);
    }
    Ok(())
}

fn shutdown_inner(app: &AppHandle, state: &State<'_, DesktopHost>) -> Result<(), String> {
    let active = {
        let mut inner = state.lock()?;
        inner.generation = inner.generation.saturating_add(1);
        inner.pending = None;
        inner.attaching.clear();
        inner.attach_waiters.clear();
        inner.attachments.discard_all();
        let event = inner.model.set_phase(DesktopPhase::ShuttingDown, None);
        emit_locked(app, &mut inner, event);
        inner.active.take()
    };
    if let Some(active) = active {
        active.shutdown();
    }
    let mut inner = state.lock()?;
    inner.workspace = None;
    let sequence = inner.sequence;
    inner.model = DesktopModel::default();
    inner.model.snapshot.sequence = sequence;
    let event = DesktopRuntimeEventV2::SnapshotReplaced {
        snapshot: inner.model.snapshot(),
    };
    emit_locked(app, &mut inner, event);
    Ok(())
}

#[tauri::command]
fn list_recent_workspaces(app: AppHandle) -> Result<Vec<RecentWorkspaceV2>, String> {
    recent::list(&app)
}

#[tauri::command]
fn forget_recent_workspace(app: AppHandle, workspace: String) -> Result<(), String> {
    recent::forget(&app, &workspace)
}

#[tauri::command]
async fn check_desktop_update() -> DesktopUpdateV2 {
    tauri::async_runtime::spawn_blocking(update_check::check)
        .await
        .unwrap_or_else(|error| DesktopUpdateV2 {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            available_version: None,
            release_url: None,
            update_available: false,
            error: Some(format!("update worker failed: {error}")),
        })
}

#[tauri::command]
fn get_desktop_diagnostics_status(
    diagnostics: State<'_, Diagnostics>,
) -> Result<DiagnosticsStatusV2, String> {
    diagnostics.status()
}

#[tauri::command]
fn export_desktop_diagnostics(
    app: AppHandle,
    diagnostics: State<'_, Diagnostics>,
) -> Result<String, String> {
    diagnostics.export(&app)
}

#[tauri::command]
fn open_desktop_update(url: String) -> Result<(), String> {
    update_check::open_release(&url)
}

#[tauri::command]
fn get_attach_capabilities() -> AttachCapabilitiesV2 {
    attach::capabilities()
}

#[tauri::command]
fn open_native_session(
    app: AppHandle,
    state: State<'_, DesktopHost>,
    member: String,
) -> Result<ExternalAttachLaunchV2, String> {
    let (attach_tx, attach_rx) = mpsc::sync_channel(1);
    let (summary, generation) = {
        let mut inner = state.lock()?;
        if inner.active.is_none() {
            return Err("the workspace runtime is not ready".to_string());
        }
        if inner.model.snapshot.members.iter().any(|candidate| {
            matches!(
                candidate.status,
                bridge::MemberStatusV2::Queued
                    | bridge::MemberStatusV2::Running
                    | bridge::MemberStatusV2::Waiting
                    | bridge::MemberStatusV2::NeedsApproval
            )
        }) || inner
            .model
            .snapshot
            .runs
            .iter()
            .any(|run| matches!(run.status, RunStatusV2::Running | RunStatusV2::Verifying))
        {
            return Err(
                "wait for or cancel all runtime work before opening a native session".to_string(),
            );
        }
        let summary = inner
            .model
            .snapshot
            .members
            .iter()
            .find(|value| value.id == member)
            .cloned()
            .ok_or_else(|| format!("unknown member: {member}"))?;
        if !inner.attaching.insert(member.clone()) {
            return Err(format!("an attach is already open for {member}"));
        }
        inner.attach_waiters.insert(member.clone(), attach_tx);
        if let Err(error) = inner
            .active
            .as_ref()
            .expect("active session checked above")
            .request_attach(&member)
        {
            inner.attach_waiters.remove(&member);
            inner.attaching.remove(&member);
            return Err(error);
        }
        (summary, inner.generation)
    };
    match attach_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            return Err(
                "timed out waiting for the runtime to reserve the native session".to_string(),
            );
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            return Err("the runtime attach handshake stopped unexpectedly".to_string());
        }
    }
    let (launch, watch) = match attach::launch(&app, &summary) {
        Ok(value) => value,
        Err(error) => {
            let mut inner = state.lock()?;
            if let Some(active) = inner.active.as_ref() {
                let _ = active.finish_attach(&member, None, Vec::new());
            }
            inner.attaching.remove(&member);
            return Err(error);
        }
    };
    let watch_member = watch.member.clone();
    let watch_app = app.clone();
    if let Err(error) = thread::Builder::new()
        .name(format!("asterline-attach-{watch_member}"))
        .spawn(move || {
            let result = watch.wait();
            let state = watch_app.state::<DesktopHost>();
            let current = state
                .lock()
                .map(|inner| inner.generation == generation)
                .unwrap_or(false);
            if !current {
                return;
            }
            match result {
                Some(Ok(import)) => {
                    let imported_count = import.items.len();
                    let discovered_session = import.session.is_some();
                    let state = watch_app.state::<DesktopHost>();
                    let result = state.lock().ok().and_then(|inner| {
                        if inner.generation != generation {
                            return None;
                        }
                        let active = inner.active.as_ref()?;
                        Some(active.finish_attach(&watch_member, import.session, import.items))
                    });
                    match result {
                        Some(Ok(())) if imported_count == 0 && discovered_session => {
                            emit_notice_for_generation(
                                &watch_app,
                                generation,
                                format!(
                                    "returned from {watch_member}; session continuity was saved"
                                ),
                            );
                        }
                        Some(Ok(())) if imported_count == 0 => emit_notice_for_generation(
                            &watch_app,
                            generation,
                            format!("returned from {watch_member}; no new transcript messages"),
                        ),
                        Some(Ok(())) => {}
                        Some(Err(error)) => {
                            emit_notice_for_generation(&watch_app, generation, error);
                        }
                        None => {}
                    }
                }
                Some(Err(error)) => {
                    if let Ok(inner) = watch_app.state::<DesktopHost>().lock()
                        && inner.generation == generation
                        && let Some(active) = inner.active.as_ref()
                    {
                        let _ = active.finish_attach(&watch_member, None, Vec::new());
                    }
                    emit_notice_for_generation(&watch_app, generation, error);
                }
                None => {
                    if let Ok(inner) = watch_app.state::<DesktopHost>().lock()
                        && inner.generation == generation
                        && let Some(active) = inner.active.as_ref()
                    {
                        let _ = active.finish_attach(&watch_member, None, Vec::new());
                    }
                }
            }
            if let Ok(mut inner) = watch_app.state::<DesktopHost>().lock()
                && inner.generation == generation
            {
                inner.attaching.remove(&watch_member);
            }
        })
    {
        let mut inner = state.lock()?;
        if let Some(active) = inner.active.as_ref() {
            let _ = active.finish_attach(&member, None, Vec::new());
        }
        inner.attaching.remove(&member);
        return Err(format!("could not start attach watcher: {error}"));
    }
    Ok(launch)
}

fn install_prepared(
    app: &AppHandle,
    state: &State<'_, DesktopHost>,
    prepared: PreparedSession,
    expected_generation: u64,
) -> Result<(), String> {
    let workspace = prepared.workspace.clone();
    let team = prepared.team.clone();
    let chat = prepared.initial_chat.clone();
    let logs = prepared
        .initial_logs
        .iter()
        .map(|entry| LogEntryV2 {
            level: match entry.level.as_str() {
                "debug" => LogLevelV2::Debug,
                "warn" => LogLevelV2::Warn,
                "error" => LogLevelV2::Error,
                _ => LogLevelV2::Info,
            },
            source: entry.source.clone(),
            message: entry.message.clone(),
        })
        .collect();
    let (runtime_tx, runtime_rx) = mpsc::channel();
    let active = prepared
        .activate(runtime_tx)
        .map_err(|error| format!("could not start desktop event bridge: {error}"))?;
    let generation = {
        let mut inner = state.lock()?;
        if inner.generation != expected_generation {
            drop(inner);
            active.shutdown();
            return Err("workspace changed while its runtime was starting".to_string());
        }
        inner.workspace = Some(PathBuf::from(&workspace));
        inner.pending = None;
        inner.model.set_workspace(workspace.clone());
        inner.model.set_team(team);
        inner.model.seed_chat(chat);
        inner.model.seed_logs(logs);
        inner.skills = skills::discover(Path::new(&workspace));
        inner.active = Some(active);
        inner.generation
    };
    let pump_app = app.clone();
    thread::Builder::new()
        .name("asterline-desktop-bridge".to_string())
        .spawn(move || {
            while let Ok(event) = runtime_rx.recv() {
                let state = pump_app.state::<DesktopHost>();
                let Ok(mut inner) = state.lock() else {
                    break;
                };
                if inner.generation != generation {
                    break;
                }
                match &event {
                    RuntimeEvent::AttachGranted { member } => {
                        let member = member.as_str().to_string();
                        if let Some(waiter) = inner.attach_waiters.remove(&member)
                            && waiter.send(Ok(())).is_err()
                        {
                            if let Some(active) = inner.active.as_ref() {
                                let _ = active.finish_attach(&member, None, Vec::new());
                            }
                            inner.attaching.remove(&member);
                        }
                    }
                    RuntimeEvent::AttachDenied { member, reason } => {
                        let member = member.as_str().to_string();
                        if let Some(waiter) = inner.attach_waiters.remove(&member) {
                            let _ = waiter.send(Err(reason.clone()));
                        }
                        inner.attaching.remove(&member);
                    }
                    _ => {}
                }
                let event = inner.model.apply_runtime(event);
                emit_locked(&pump_app, &mut inner, event);
            }
        })
        .map_err(|error| format!("could not start desktop bridge: {error}"))?;
    Ok(())
}

fn emit_notice_for_generation<R: Runtime>(app: &AppHandle<R>, generation: u64, message: String) {
    let state = app.state::<DesktopHost>();
    if let Ok(mut inner) = state.lock() {
        if inner.generation != generation {
            return;
        }
        let event = inner.model.apply_runtime(RuntimeEvent::Notice(message));
        emit_locked(app, &mut inner, event);
    }
}

fn emit_locked<R: Runtime>(
    app: &AppHandle<R>,
    inner: &mut HostInner,
    mut event: DesktopRuntimeEventV2,
) -> DesktopEventV2 {
    inner.sequence = inner.sequence.saturating_add(1);
    let sequence = inner.sequence;
    inner.model.snapshot.sequence = inner.sequence;
    if let DesktopRuntimeEventV2::SnapshotReplaced { snapshot } = &mut event {
        snapshot.sequence = sequence;
    }
    let event = envelope(sequence, event);
    if let DesktopRuntimeEventV2::RuntimeLog {
        level,
        source,
        message,
    } = &event.event
        && let Some(diagnostics) = app.try_state::<Diagnostics>()
    {
        diagnostics.write(level, source, message);
    }
    let _ = app.emit(DESKTOP_EVENT_CHANNEL, &event);
    event
}

fn command_can_dispatch_backend(command: &DesktopCommandV2) -> bool {
    matches!(
        command,
        DesktopCommandV2::UserMessage { .. }
            | DesktopCommandV2::Retry
            | DesktopCommandV2::Approve {
                decision: bridge::ApprovalChoiceV2::Approve,
                ..
            }
            | DesktopCommandV2::ResolvePausedRoute { resume: true }
            | DesktopCommandV2::ReplaceTeamSettings { .. }
            | DesktopCommandV2::NewSession
            | DesktopCommandV2::ResumeConversation { .. }
            | DesktopCommandV2::ImportSession { .. }
            | DesktopCommandV2::ContinueRun { .. }
            | DesktopCommandV2::RunMode { .. }
    )
}

fn attaching_members(inner: &HostInner) -> String {
    let mut members = inner.attaching.iter().cloned().collect::<Vec<_>>();
    members.sort();
    members.join(", ")
}

fn validate_workspace(workspace: &str) -> Result<PathBuf, String> {
    if workspace.trim().is_empty() {
        return Err("workspace cannot be empty".to_string());
    }
    let path = PathBuf::from(workspace);
    if !path.is_absolute() {
        return Err("workspace must be an absolute path".to_string());
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("could not open workspace: {error}"))?;
    if !path.is_dir() {
        return Err("workspace is not a directory".to_string());
    }
    Ok(path)
}

fn is_lock_error(message: &str) -> bool {
    message.contains("another Asterline instance") || message.contains("could not lock")
}

pub fn run_attach_helper_from_args() -> Option<Result<(), String>> {
    attach::run_helper_from_args()
}

pub fn run() {
    platform_env::restore_desktop_path();
    tauri::Builder::default()
        .manage(DesktopHost::default())
        .setup(|app| {
            let diagnostics =
                Diagnostics::initialize(app.handle()).map_err(std::io::Error::other)?;
            diagnostics.install_panic_hook();
            app.manage(diagnostics);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_desktop,
            get_desktop_snapshot,
            parse_composer_text,
            complete_composer,
            command_catalog,
            get_backend_availability,
            list_models,
            list_native_sessions,
            stage_clipboard_image,
            stage_image_path,
            stage_image_bytes,
            remove_staged_attachment,
            discard_staged_attachments,
            dispatch_desktop_command,
            shutdown_desktop,
            exit_desktop,
            list_recent_workspaces,
            forget_recent_workspace,
            check_desktop_update,
            open_desktop_update,
            get_desktop_diagnostics_status,
            export_desktop_diagnostics,
            get_attach_capabilities,
            open_native_session,
        ])
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let state = window.state::<DesktopHost>();
                    let active = state.lock().ok().and_then(|mut inner| {
                        inner.generation = inner.generation.saturating_add(1);
                        inner.attaching.clear();
                        inner.attach_waiters.clear();
                        inner.attachments.discard_all();
                        inner.active.take()
                    });
                    if let Some(active) = active {
                        api.prevent_close();
                        active.shutdown();
                        if let Some(diagnostics) = window.try_state::<Diagnostics>() {
                            diagnostics.mark_clean();
                        }
                        window.app_handle().exit(0);
                    } else if let Some(diagnostics) = window.try_state::<Diagnostics>() {
                        diagnostics.mark_clean();
                    }
                }
                WindowEvent::Destroyed => {
                    // Fallback for platform-forced destruction. Shutdown is
                    // synchronous so process exit cannot strand backend child
                    // processes or leave SQLite cleanup to a detached thread.
                    let state = window.state::<DesktopHost>();
                    if let Ok(mut inner) = state.lock() {
                        inner.attachments.discard_all();
                        if let Some(active) = inner.active.take() {
                            active.shutdown();
                        }
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Asterline Desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_failures_have_a_distinct_phase() {
        assert!(is_lock_error(
            "could not lock store (another Asterline instance may be using this store)"
        ));
        assert!(!is_lock_error("team config is invalid"));
    }

    #[test]
    fn relative_workspace_is_rejected() {
        assert!(validate_workspace("relative/path").is_err());
    }

    #[test]
    fn attach_blocks_commands_that_can_start_backend_work() {
        assert!(command_can_dispatch_backend(&DesktopCommandV2::Retry));
        assert!(command_can_dispatch_backend(
            &DesktopCommandV2::UserMessage {
                target: bridge::MessageTargetV2::All,
                body: "hello".to_string(),
                attachments: Vec::new(),
            }
        ));
        assert!(command_can_dispatch_backend(
            &DesktopCommandV2::ImportSession {
                member: None,
                session_id: "sess-1".to_string(),
            }
        ));
        assert!(!command_can_dispatch_backend(&DesktopCommandV2::SetMode {
            mode: bridge::TerminalModeV2::Plan,
        }));
        assert!(!command_can_dispatch_backend(&DesktopCommandV2::Cancel {
            member: None,
        }));
        assert!(!command_can_dispatch_backend(
            &DesktopCommandV2::EditQueuedPrompt { member: None }
        ));
    }

    #[test]
    fn launch_options_default_to_safe_values() {
        let options = DesktopLaunchOptionsV2::default();
        assert!(options.team_path.is_none());
        assert!(!options.pick_team);
        assert!(options.db_path.is_none());
        assert!(!options.debug);
        assert!(!options.fake);
    }
}
