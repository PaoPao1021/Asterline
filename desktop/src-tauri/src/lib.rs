mod attach;
mod bridge;
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
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WindowEvent};

use attach::{AttachCapabilitiesV1, ExternalAttachLaunchV1};
use bridge::{
    DESKTOP_EVENT_CHANNEL, DesktopCommandV1, DesktopEventV1, DesktopModel, DesktopPhase,
    DesktopRuntimeEventV1, DesktopSnapshotV1, DiffResultV1, LogEntryV1, LogLevelV1,
    RunStatusV1, SkillSummaryV1, envelope,
};
use recent::RecentWorkspaceV1;
use session_adapter::{ActiveSession, BootstrapAdapterOutcome, PendingTeamSetup, PreparedSession};
use update_check::DesktopUpdateV1;

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
) -> Result<DesktopSnapshotV1, String> {
    let Some(workspace) = workspace else {
        return Ok(state.lock()?.model.snapshot());
    };
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

    match session_adapter::bootstrap_workspace(workspace.clone()) {
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
fn get_desktop_snapshot(state: State<'_, DesktopHost>) -> Result<DesktopSnapshotV1, String> {
    Ok(state.lock()?.model.snapshot())
}

#[tauri::command]
fn dispatch_desktop_command(
    app: AppHandle,
    state: State<'_, DesktopHost>,
    command: DesktopCommandV1,
) -> Result<(), String> {
    if matches!(command, DesktopCommandV1::Shutdown) {
        return shutdown_desktop(app, state);
    }

    if matches!(
        command,
        DesktopCommandV1::RequestLogs { .. }
            | DesktopCommandV1::RequestDiff { .. }
            | DesktopCommandV1::RequestSkills { .. }
    ) {
        let mut inner = state.lock()?;
        let event = utility_query(&inner, command)?;
        emit_locked(&app, &mut inner, event);
        return Ok(());
    }

    // Upstream 0.2.9 makes the complete team settings document the single
    // authoritative mutation path. Preserve the compact Desktop effort action
    // by translating it into that validated atomic update.
    let command = match command {
        DesktopCommandV1::SetEffort { member, effort } => {
            let mut settings = state
                .lock()?
                .model
                .snapshot()
                .team
                .ok_or_else(|| "team settings are not available".to_string())?;
            let target = settings
                .members
                .iter_mut()
                .find(|candidate| candidate.id == member)
                .ok_or_else(|| format!("unknown member: {member}"))?;
            target.effort = Some(effort);
            DesktopCommandV1::ReplaceTeamSettings {
                settings: Box::new(settings),
            }
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
        DesktopCommandV1::ReplaceTeamSettings { settings } => Some(settings.clone()),
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
    let resolves_paused = matches!(&command, DesktopCommandV1::ResolvePausedRoute { .. });
    let runtime_command = session_adapter::command_to_runtime(command, workspace)?;
    inner
        .active
        .as_ref()
        .ok_or_else(|| "the workspace runtime is not ready".to_string())?
        .send(runtime_command)?;
    if resolves_paused {
        inner.model.clear_next_paused_route();
    }
    Ok(())
}

fn utility_query(
    inner: &HostInner,
    command: DesktopCommandV1,
) -> Result<DesktopRuntimeEventV1, String> {
    let workspace = inner
        .workspace
        .as_deref()
        .ok_or_else(|| "open a workspace before requesting utility data".to_string())?;
    match command {
        DesktopCommandV1::RequestLogs {
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
            Ok(DesktopRuntimeEventV1::LogsReplaced {
                request_id,
                entries,
                truncated,
            })
        }
        DesktopCommandV1::RequestDiff { request_id } => {
            let text = asterline::tui::compute_git_diff(&workspace.to_string_lossy());
            let truncated = text.contains("[diff output truncated at 2 MiB]")
                || text.contains("[untracked file list truncated]");
            let file_count = text
                .lines()
                .filter(|line| line.starts_with("diff --git ") || line.starts_with("  "))
                .count();
            Ok(DesktopRuntimeEventV1::DiffReplaced {
                request_id,
                result: DiffResultV1 {
                    text,
                    truncated,
                    file_count,
                },
            })
        }
        DesktopCommandV1::RequestSkills {
            request_id,
            backend,
            query,
            limit,
        } => {
            let query = query.unwrap_or_default().to_lowercase();
            let max = limit.unwrap_or(512).clamp(1, 512);
            let mut skills = asterline::tui::skills::discover(Path::new(workspace))
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
                .map(|skill| SkillSummaryV1 {
                    name: skill.name,
                    description: skill.description,
                    backend: bridge::backend_kind(skill.backend.as_str()),
                    invocation: skill.invocation,
                })
                .collect::<Vec<_>>();
            let truncated = skills.len() > max;
            skills.truncate(max);
            Ok(DesktopRuntimeEventV1::SkillsReplaced {
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
    let active = {
        let mut inner = state.lock()?;
        inner.generation = inner.generation.saturating_add(1);
        inner.pending = None;
        inner.attaching.clear();
        inner.attach_waiters.clear();
        let event = inner.model.set_phase(DesktopPhase::ShuttingDown, None);
        emit_locked(&app, &mut inner, event);
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
    let event = DesktopRuntimeEventV1::SnapshotReplaced {
        snapshot: inner.model.snapshot(),
    };
    emit_locked(&app, &mut inner, event);
    Ok(())
}

#[tauri::command]
fn list_recent_workspaces(app: AppHandle) -> Result<Vec<RecentWorkspaceV1>, String> {
    recent::list(&app)
}

#[tauri::command]
fn forget_recent_workspace(app: AppHandle, workspace: String) -> Result<(), String> {
    recent::forget(&app, &workspace)
}

#[tauri::command]
async fn check_desktop_update() -> DesktopUpdateV1 {
    tauri::async_runtime::spawn_blocking(update_check::check)
        .await
        .unwrap_or_else(|error| DesktopUpdateV1 {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            available_version: None,
            release_url: None,
            update_available: false,
            error: Some(format!("update worker failed: {error}")),
        })
}

#[tauri::command]
fn get_attach_capabilities() -> AttachCapabilitiesV1 {
    attach::capabilities()
}

#[tauri::command]
fn open_native_session(
    app: AppHandle,
    state: State<'_, DesktopHost>,
    member: String,
) -> Result<ExternalAttachLaunchV1, String> {
    let (attach_tx, attach_rx) = mpsc::sync_channel(1);
    let (summary, generation) = {
        let mut inner = state.lock()?;
        if inner.active.is_none() {
            return Err("the workspace runtime is not ready".to_string());
        }
        if inner.model.snapshot.members.iter().any(|candidate| {
            matches!(
                candidate.status,
                bridge::MemberStatusV1::Queued
                    | bridge::MemberStatusV1::Running
                    | bridge::MemberStatusV1::Waiting
                    | bridge::MemberStatusV1::NeedsApproval
            )
        }) || inner
            .model
            .snapshot
            .runs
            .iter()
            .any(|run| matches!(run.status, RunStatusV1::Running | RunStatusV1::Verifying))
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
        .map(|entry| LogEntryV1 {
            level: match entry.level.as_str() {
                "debug" => LogLevelV1::Debug,
                "warn" => LogLevelV1::Warn,
                "error" => LogLevelV1::Error,
                _ => LogLevelV1::Info,
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
        inner.model.set_workspace(workspace);
        inner.model.set_team(team);
        inner.model.seed_chat(chat);
        inner.model.seed_logs(logs);
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
    mut event: DesktopRuntimeEventV1,
) -> DesktopEventV1 {
    inner.sequence = inner.sequence.saturating_add(1);
    let sequence = inner.sequence;
    inner.model.snapshot.sequence = inner.sequence;
    if let DesktopRuntimeEventV1::SnapshotReplaced { snapshot } = &mut event {
        snapshot.sequence = sequence;
    }
    let event = envelope(sequence, event);
    let _ = app.emit(DESKTOP_EVENT_CHANNEL, &event);
    event
}

fn command_can_dispatch_backend(command: &DesktopCommandV1) -> bool {
    matches!(
        command,
        DesktopCommandV1::UserMessage { .. }
            | DesktopCommandV1::Retry
            | DesktopCommandV1::Approve {
                decision: bridge::ApprovalChoiceV1::Approve,
                ..
            }
            | DesktopCommandV1::ResolvePausedRoute { resume: true }
            | DesktopCommandV1::ReplaceTeamSettings { .. }
            | DesktopCommandV1::NewSession
            | DesktopCommandV1::ResumeConversation { .. }
            | DesktopCommandV1::ContinueRun { .. }
            | DesktopCommandV1::VerifyRun { .. }
            | DesktopCommandV1::RunMode { .. }
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
        .invoke_handler(tauri::generate_handler![
            bootstrap_desktop,
            get_desktop_snapshot,
            dispatch_desktop_command,
            shutdown_desktop,
            list_recent_workspaces,
            forget_recent_workspace,
            check_desktop_update,
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
                        inner.active.take()
                    });
                    if let Some(active) = active {
                        api.prevent_close();
                        active.shutdown();
                        window.app_handle().exit(0);
                    }
                }
                WindowEvent::Destroyed => {
                    // Fallback for platform-forced destruction. Shutdown is
                    // synchronous so process exit cannot strand backend child
                    // processes or leave SQLite cleanup to a detached thread.
                    let state = window.state::<DesktopHost>();
                    if let Ok(mut inner) = state.lock()
                        && let Some(active) = inner.active.take()
                    {
                        active.shutdown();
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
        assert!(command_can_dispatch_backend(&DesktopCommandV1::Retry));
        assert!(command_can_dispatch_backend(
            &DesktopCommandV1::UserMessage {
                target: bridge::MessageTargetV1::All,
                body: "hello".to_string(),
            }
        ));
        assert!(command_can_dispatch_backend(&DesktopCommandV1::VerifyRun {
            run_id: None,
            command: None,
        }));
        assert!(!command_can_dispatch_backend(&DesktopCommandV1::SetMode {
            mode: bridge::TerminalModeV1::Plan,
        }));
        assert!(!command_can_dispatch_backend(&DesktopCommandV1::Cancel {
            member: None,
        }));
    }
}
