//! The only module coupled to Asterline's UI-independent session API.
//!
//! Keeping this seam small lets the bridge DTO remain versioned independently
//! from the core runtime vocabulary.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

use asterline::app::{self, AppSession, BootstrapOutcome, SessionOptions, TeamSetup};
use asterline::domain::config::default_team;
use asterline::domain::event::{
    AgentSessionId, ApprovalDecision, ApprovalId, ImportedMessage, MessageTarget, RunId,
    RunStepStatus, RuntimeEvent, UiCommand,
};
use asterline::domain::mode::{CollabMode, TerminalMode};
use asterline::domain::team::{MemberId, TeamSettings};
use asterline::domain::{resolve_mode_roles, resolve_team_coordinator, resolve_team_limits};
use asterline::runtime::RuntimeHandle;

use crate::bridge::{
    ApprovalChoiceV2, DesktopCommandV2, MessageTargetV2, ModesConfigV2, RunStepStatusV2,
    TeamSettingsV2, TerminalModeV2, modes_config_to_domain, team_settings_from_domain,
    team_settings_to_domain,
};

/// Advanced launch options collected by the desktop project picker. They map
/// one-to-one onto the shared [`SessionOptions`] (the TUI's `--team`,
/// `--pick-team`, `--db`, `--no-restore`, `--debug`, `--fake`, and update
/// policy). `debug` and `fake` are session-scoped: they are never persisted.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct DesktopLaunchOptionsV2 {
    /// Explicit team roster file (`--team <path>`).
    pub team_path: Option<String>,
    /// Skip the saved roster and collect a team for this launch
    /// (`--pick-team`; the desktop advanced launcher asks for the roster).
    pub pick_team: bool,
    /// SQLite database path override (`--db <path>`).
    pub db_path: Option<String>,
    /// Restore the last conversation on open. Desktop defaults to on.
    pub restore: Option<bool>,
    /// Show native Codex tool asks in the composer instead of auto-approving.
    #[serde(alias = "approvals")]
    pub manual_approvals: Option<bool>,
    /// `--debug`: enables developer diagnostics without changing approvals.
    pub debug: bool,
    /// `--fake`: run offline fake agents instead of real CLIs, this launch only.
    pub fake: bool,
    /// Automatic update checks. Desktop stays manual by default (the GUI's
    /// trusted release-page flow satisfies `--no-auto-update`).
    pub auto_update: Option<bool>,
}

impl DesktopLaunchOptionsV2 {
    fn to_session_options(&self, workspace: PathBuf) -> SessionOptions {
        SessionOptions {
            team_path: self.team_path.as_ref().map(PathBuf::from),
            workspace: Some(workspace),
            db_path: self.db_path.as_ref().map(PathBuf::from),
            restore: self.restore.unwrap_or(true),
            manual_approvals: self.manual_approvals.unwrap_or(false),
            fake: self.fake,
            pick_team: self.pick_team && self.team_path.is_none(),
            auto_update: self.auto_update.unwrap_or(false),
        }
    }
}

pub enum BootstrapAdapterOutcome {
    Ready(Box<PreparedSession>),
    NeedsTeamSetup {
        pending: Box<PendingTeamSetup>,
        draft: Box<TeamSettingsV2>,
    },
}

pub struct PendingTeamSetup {
    setup: TeamSetup,
}

impl PendingTeamSetup {
    pub fn start(self, settings: &TeamSettingsV2) -> io::Result<PreparedSession> {
        ensure_workspace_matches(settings, &self.setup.workspace)?;
        let settings: TeamSettings = team_settings_to_domain(settings)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let workspace = self.setup.workspace.clone();
        PreparedSession::new(self.setup.start(settings.into_config(workspace))?)
    }
}

pub struct PreparedSession {
    session: AppSession,
    pub workspace: String,
    pub team: TeamSettingsV2,
    pub initial_chat: Vec<asterline::domain::event::ChatItem>,
    pub initial_logs: Vec<asterline::domain::event::LogEntry>,
}

impl PreparedSession {
    fn new(mut session: AppSession) -> io::Result<Self> {
        let workspace = session.workspace().to_string_lossy().into_owned();
        let team = team_settings_from_domain(&session.team_settings(), Some(&workspace))
            .map_err(io::Error::other)?;
        let initial_chat = session.take_initial_chat();
        let initial_logs = session.take_initial_logs();
        Ok(Self {
            session,
            workspace,
            team,
            initial_chat,
            initial_logs,
        })
    }

    pub fn activate(self, events_tx: Sender<RuntimeEvent>) -> io::Result<ActiveSession> {
        let handle = self.session.handle();
        let join = thread::Builder::new()
            .name("asterline-desktop-events".to_string())
            .spawn(move || {
                // Keep draining the core's bounded event channel through
                // shutdown. Stopping the drain first could leave the runtime
                // blocked on a final event and make the subsequent join hang.
                while let Ok(event) = self.session.events().recv() {
                    if events_tx.send(event).is_err() {
                        break;
                    }
                }
                let _ = self.session.shutdown();
            })?;
        Ok(ActiveSession {
            handle,
            worker: Some(join),
        })
    }
}

pub struct ActiveSession {
    handle: RuntimeHandle,
    worker: Option<JoinHandle<()>>,
}

impl ActiveSession {
    pub fn send(&self, command: UiCommand) -> Result<(), String> {
        if self.handle.send(command) {
            Ok(())
        } else {
            Err("Asterline runtime is no longer available".to_string())
        }
    }

    pub fn request_attach(&self, member: &str) -> Result<(), String> {
        self.send(UiCommand::RequestAttach {
            member: MemberId::new(member),
        })
    }

    /// Atomically finish the runtime's attach reservation, bind any proven
    /// native session identity, and import the bounded transcript delta.
    pub fn finish_attach(
        &self,
        member: &str,
        session: Option<String>,
        items: Vec<(bool, String)>,
    ) -> Result<(), String> {
        let session = if let Some(session) = session {
            validate_session_id(&session)?;
            Some(AgentSessionId(session))
        } else {
            None
        };
        let items = items
            .into_iter()
            .map(|(from_user, text)| ImportedMessage { from_user, text })
            .collect();
        self.handle
            .finish_attach_with_session(MemberId::new(member), session, items)
            .then_some(())
            .ok_or_else(|| "Asterline runtime is no longer available".to_string())
    }

    fn stop_and_join(&mut self) {
        let _ = self.handle.send(UiCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    pub fn shutdown(mut self) {
        self.stop_and_join();
    }
}

impl Drop for ActiveSession {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

pub fn bootstrap_workspace(
    workspace: PathBuf,
    options: &DesktopLaunchOptionsV2,
) -> io::Result<BootstrapAdapterOutcome> {
    let session_options = options.to_session_options(workspace.clone());
    match app::bootstrap(session_options, &workspace)? {
        BootstrapOutcome::Ready(session) => Ok(BootstrapAdapterOutcome::Ready(Box::new(
            PreparedSession::new(session)?,
        ))),
        BootstrapOutcome::NeedsTeamSetup(setup) => {
            let workspace_text = setup.workspace.to_string_lossy().into_owned();
            let draft = match default_team(&setup.workspace, setup.detected) {
                Some(team) => team_settings_from_domain(&team, None).map_err(io::Error::other)?,
                None => TeamSettingsV2 {
                    name: "new-team".to_string(),
                    workspace: workspace_text,
                    members: Vec::new(),
                    default_target: None,
                    max_auto_relays: asterline::domain::team::DEFAULT_MAX_AUTO_RELAYS,
                    modes: ModesConfigV2::default(),
                    approvals: Default::default(),
                    extra: Default::default(),
                },
            };
            Ok(BootstrapAdapterOutcome::NeedsTeamSetup {
                pending: Box::new(PendingTeamSetup { setup }),
                draft: Box::new(draft),
            })
        }
    }
}

/// Resolve a managed attachment token into its file inside the managed paste
/// directory. `Err` names the problem; unknown or unsafe tokens are rejected
/// before any runtime traffic.
pub type AttachmentResolver<'a> = &'a dyn Fn(&str) -> Result<PathBuf, String>;

pub fn command_to_runtime(
    command: DesktopCommandV2,
    workspace: &Path,
    resolve_attachment: AttachmentResolver<'_>,
) -> Result<UiCommand, String> {
    let run_id = |id: Option<u64>| id.map(RunId);
    match command {
        DesktopCommandV2::RequestLogs { .. }
        | DesktopCommandV2::RequestDiff { .. }
        | DesktopCommandV2::RequestSkills { .. }
        | DesktopCommandV2::ResetModeOverrides { .. } => {
            Err("utility queries are handled by the desktop host".to_string())
        }
        DesktopCommandV2::SetMode { mode } => Ok(UiCommand::SetMode {
            mode: terminal_mode(mode),
        }),
        DesktopCommandV2::SetModeOverrides { overrides } => Ok(UiCommand::SetModeOverrides {
            overrides: modes_config_to_domain(&overrides)?,
        }),
        DesktopCommandV2::SaveModeDefaults { mode } => Ok(UiCommand::SaveModeDefaults {
            mode: terminal_mode(mode),
        }),
        DesktopCommandV2::UserMessage {
            target,
            body,
            attachments,
        } => {
            if attachments.is_empty() {
                require_text(&body, "message")?;
                Ok(UiCommand::UserMessage {
                    target: message_target(target),
                    body,
                })
            } else {
                // Image-only sends keep an empty text body, like the TUI.
                if !body.trim().is_empty() {
                    require_text(&body, "message")?;
                }
                if attachments.len() > asterline::adapter::prompt_images::MAX_PROMPT_IMAGES {
                    return Err(format!(
                        "at most {} images can be attached per message",
                        asterline::adapter::prompt_images::MAX_PROMPT_IMAGES
                    ));
                }
                let mut full_body = body;
                for token in &attachments {
                    let path = resolve_attachment(token)?;
                    if !full_body.is_empty() && !full_body.ends_with('\n') {
                        full_body.push('\n');
                    }
                    full_body.push_str("[asterline-image]: ");
                    full_body.push_str(&path.to_string_lossy());
                }
                Ok(UiCommand::UserMessage {
                    target: message_target(target),
                    body: full_body,
                })
            }
        }
        DesktopCommandV2::Cancel { member } => Ok(UiCommand::Cancel {
            member: member.map(MemberId::new),
        }),
        DesktopCommandV2::EditQueuedPrompt { member } => Ok(UiCommand::EditQueuedPrompt {
            member: member.map(MemberId::new),
        }),
        DesktopCommandV2::Retry => Ok(UiCommand::Retry),
        DesktopCommandV2::Approve { id, decision } => Ok(UiCommand::Approve {
            id: ApprovalId(id),
            decision: match decision {
                ApprovalChoiceV2::Approve => ApprovalDecision::Approve,
                ApprovalChoiceV2::Reject => ApprovalDecision::Reject,
            },
        }),
        DesktopCommandV2::SetRelayPaused { paused } => Ok(UiCommand::SetRelayPaused(paused)),
        DesktopCommandV2::ResolvePausedRoute { resume } => {
            Ok(UiCommand::ResolvePausedRoute { resume })
        }
        DesktopCommandV2::ReplaceTeamSettings { settings } => {
            ensure_workspace_matches(&settings, workspace).map_err(|error| error.to_string())?;
            let settings: TeamSettings = team_settings_to_domain(&settings)?;
            validate_team_settings(&settings, workspace)?;
            Ok(UiCommand::ReplaceTeamSettings {
                settings: Box::new(settings),
            })
        }
        DesktopCommandV2::RequestTeamSettings => Ok(UiCommand::RequestTeamSettings),
        DesktopCommandV2::NewSession => Ok(UiCommand::NewSession),
        DesktopCommandV2::RequestResume => Ok(UiCommand::RequestResume),
        DesktopCommandV2::ResumeConversation { conversation } => {
            Ok(UiCommand::ResumeConversation { conversation })
        }
        DesktopCommandV2::ImportSession { member, session_id } => {
            require_text(&session_id, "session id")?;
            Ok(UiCommand::ImportSession {
                member: member.map(MemberId::new),
                session_id,
            })
        }
        DesktopCommandV2::ExportSession { format } => Ok(UiCommand::ExportSession { format }),
        DesktopCommandV2::ContinueRun { run_id: id, note } => Ok(UiCommand::ContinueRun {
            run_id: run_id(id),
            note,
        }),
        DesktopCommandV2::NoteRun { run_id: id, note } => {
            require_text(&note, "run note")?;
            Ok(UiCommand::NoteRun {
                run_id: run_id(id),
                note,
            })
        }
        DesktopCommandV2::BlockRun { run_id: id, reason } => {
            require_text(&reason, "block reason")?;
            Ok(UiCommand::BlockRun {
                run_id: run_id(id),
                reason,
            })
        }
        DesktopCommandV2::AddRunStep {
            run_id: id,
            owner,
            title,
        } => {
            require_text(&title, "step title")?;
            Ok(UiCommand::AddRunStep {
                run_id: run_id(id),
                owner: owner.map(MemberId::new),
                title,
            })
        }
        DesktopCommandV2::UpdateRunStep {
            run_id: id,
            step,
            status,
            note,
        } => {
            positive_step(step)?;
            Ok(UiCommand::UpdateRunStep {
                run_id: run_id(id),
                step,
                status: run_step_status(status),
                note,
            })
        }
        DesktopCommandV2::RenameRunStep {
            run_id: id,
            step,
            title,
        } => {
            positive_step(step)?;
            require_text(&title, "step title")?;
            Ok(UiCommand::RenameRunStep {
                run_id: run_id(id),
                step,
                title,
            })
        }
        DesktopCommandV2::RemoveRunStep { run_id: id, step } => {
            positive_step(step)?;
            Ok(UiCommand::RemoveRunStep {
                run_id: run_id(id),
                step,
            })
        }
        DesktopCommandV2::AssignRunStep {
            run_id: id,
            step,
            owner,
        } => {
            positive_step(step)?;
            Ok(UiCommand::AssignRunStep {
                run_id: run_id(id),
                step,
                owner: owner.map(MemberId::new),
            })
        }
        DesktopCommandV2::RunMode { mode, task } => {
            require_text(&task, "mode task")?;
            let mode = collab_mode(mode)
                .ok_or_else(|| "normal is not a collaboration run mode".to_string())?;
            Ok(UiCommand::RunMode { mode, task })
        }
        DesktopCommandV2::Shutdown => Ok(UiCommand::Shutdown),
    }
}

fn message_target(target: MessageTargetV2) -> MessageTarget {
    match target {
        MessageTargetV2::Default => MessageTarget::Default,
        MessageTargetV2::All => MessageTarget::All,
        MessageTargetV2::Member { member } => MessageTarget::Member(MemberId::new(member)),
        MessageTargetV2::Members { members } => {
            MessageTarget::Members(members.into_iter().map(MemberId::new).collect())
        }
    }
}

fn terminal_mode(mode: TerminalModeV2) -> TerminalMode {
    match mode {
        TerminalModeV2::Normal => TerminalMode::Normal,
        TerminalModeV2::Review => TerminalMode::Review,
        TerminalModeV2::Plan => TerminalMode::Plan,
        TerminalModeV2::Brainstorm => TerminalMode::Brainstorm,
        TerminalModeV2::Team => TerminalMode::Team,
    }
}

fn collab_mode(mode: TerminalModeV2) -> Option<CollabMode> {
    match mode {
        TerminalModeV2::Normal => None,
        TerminalModeV2::Review => Some(CollabMode::Review),
        TerminalModeV2::Plan => Some(CollabMode::Plan),
        TerminalModeV2::Brainstorm => Some(CollabMode::Brainstorm),
        TerminalModeV2::Team => Some(CollabMode::Team),
    }
}

fn run_step_status(status: RunStepStatusV2) -> RunStepStatus {
    match status {
        RunStepStatusV2::Todo => RunStepStatus::Todo,
        RunStepStatusV2::Doing => RunStepStatus::Doing,
        RunStepStatusV2::Done => RunStepStatus::Done,
        RunStepStatusV2::Blocked => RunStepStatus::Blocked,
    }
}

fn ensure_workspace_matches(settings: &TeamSettingsV2, workspace: &Path) -> io::Result<()> {
    let expected = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let supplied = PathBuf::from(&settings.workspace)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&settings.workspace));
    let equal = if cfg!(windows) {
        expected
            .to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&supplied.to_string_lossy().replace('/', "\\"))
    } else {
        expected == supplied
    };
    if equal {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "team settings cannot change the open workspace",
        ))
    }
}

fn require_text(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{name} cannot be empty"))
    } else if value.len() > 4 * 1024 * 1024 {
        Err(format!("{name} exceeds the 4 MiB bridge limit"))
    } else {
        Ok(())
    }
}

fn validate_session_id(session: &str) -> Result<(), String> {
    if session.trim().is_empty() || session.len() > 4096 || session.chars().any(char::is_control) {
        Err("attached backend returned an invalid session id".to_string())
    } else {
        Ok(())
    }
}

fn validate_team_settings(settings: &TeamSettings, workspace: &Path) -> Result<(), String> {
    let config = settings.clone().into_config(workspace.to_path_buf());
    config
        .validate()
        .map_err(|error| format!("invalid team settings: {error}"))?;
    if config.modes.review.is_some() {
        resolve_mode_roles(&config, CollabMode::Review)
            .map_err(|error| format!("invalid review mode: {error}"))?;
    }
    if config.modes.plan.is_some() {
        resolve_mode_roles(&config, CollabMode::Plan)
            .map_err(|error| format!("invalid plan mode: {error}"))?;
    }
    if config.modes.brainstorm.is_some() {
        resolve_mode_roles(&config, CollabMode::Brainstorm)
            .map_err(|error| format!("invalid brainstorm mode: {error}"))?;
    }
    if config.modes.team.is_some() {
        resolve_team_coordinator(&config).map_err(|error| format!("invalid team mode: {error}"))?;
        resolve_team_limits(&config).map_err(|error| format!("invalid team mode: {error}"))?;
    }
    Ok(())
}

fn positive_step(step: u32) -> Result<(), String> {
    if step == 0 {
        Err("run step numbers are 1-based".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_attachments(token: &str) -> Result<PathBuf, String> {
        Err(format!("unknown attachment: {token}"))
    }

    fn dispatch(command: DesktopCommandV2) -> Result<UiCommand, String> {
        command_to_runtime(command, Path::new("/workspace"), &no_attachments)
    }

    #[test]
    fn normal_cannot_be_started_as_a_mode_run() {
        let command = DesktopCommandV2::RunMode {
            mode: TerminalModeV2::Normal,
            task: "task".to_string(),
        };
        assert!(dispatch(command).is_err());
    }

    #[test]
    fn zero_step_is_rejected_at_bridge_boundary() {
        let command = DesktopCommandV2::RemoveRunStep {
            run_id: None,
            step: 0,
        };
        assert!(dispatch(command).is_err());
    }

    #[test]
    fn unsafe_attached_session_ids_are_rejected() {
        assert!(validate_session_id("thread-123").is_ok());
        assert!(validate_session_id("thread\n123").is_err());
    }

    #[test]
    fn invalid_settings_are_rejected_before_runtime_enqueue() {
        let workspace = std::env::temp_dir();
        let command = DesktopCommandV2::ReplaceTeamSettings {
            settings: Box::new(TeamSettingsV2 {
                name: "empty".to_string(),
                workspace: workspace.to_string_lossy().into_owned(),
                members: Vec::new(),
                default_target: None,
                max_auto_relays: 1,
                modes: ModesConfigV2::default(),
                approvals: Default::default(),
                extra: Default::default(),
            }),
        };
        let resolve = |token: &str| -> Result<PathBuf, String> {
            Err(format!("unknown attachment: {token}"))
        };
        let error = command_to_runtime(command, &workspace, &resolve).unwrap_err();
        assert!(error.contains("team has no members"));
    }

    #[test]
    fn attachments_become_managed_marker_lines() {
        let command = DesktopCommandV2::UserMessage {
            target: MessageTargetV2::Member {
                member: "builder".to_string(),
            },
            body: String::new(),
            attachments: vec!["att-1.png".to_string()],
        };
        let resolve = |token: &str| -> Result<PathBuf, String> {
            Ok(PathBuf::from(format!("/tmp/pasted/{token}")))
        };
        let UiCommand::UserMessage { body, .. } =
            command_to_runtime(command, Path::new("/w"), &resolve).unwrap()
        else {
            panic!("expected user message");
        };
        assert_eq!(body, "[asterline-image]: /tmp/pasted/att-1.png".to_string());
    }

    #[test]
    fn image_only_messages_keep_an_empty_text_body() {
        let command = DesktopCommandV2::UserMessage {
            target: MessageTargetV2::All,
            body: String::new(),
            attachments: vec!["att-1.png".to_string()],
        };
        let resolve = |token: &str| -> Result<PathBuf, String> {
            Ok(PathBuf::from(format!("/tmp/pasted/{token}")))
        };
        let result = command_to_runtime(command, Path::new("/w"), &resolve);
        assert!(result.is_ok(), "image-only sends must not require text");
    }

    #[test]
    fn unknown_attachment_tokens_are_rejected_before_the_runtime() {
        let command = DesktopCommandV2::UserMessage {
            target: MessageTargetV2::All,
            body: "hi".to_string(),
            attachments: vec!["../escape.png".to_string()],
        };
        let error = dispatch(command).unwrap_err();
        assert!(error.contains("unknown attachment"));
    }

    #[test]
    fn more_than_four_images_are_rejected() {
        let command = DesktopCommandV2::UserMessage {
            target: MessageTargetV2::All,
            body: "hi".to_string(),
            attachments: (0..5).map(|index| format!("att-{index}.png")).collect(),
        };
        let resolve = |token: &str| -> Result<PathBuf, String> {
            Ok(PathBuf::from(format!("/tmp/pasted/{token}")))
        };
        let error = command_to_runtime(command, Path::new("/w"), &resolve).unwrap_err();
        assert!(error.contains("at most 4 images"));
    }

    #[test]
    fn launch_options_map_onto_shared_session_options() {
        let options = DesktopLaunchOptionsV2 {
            team_path: Some("D:\\teams\\core.json".to_string()),
            pick_team: true,
            db_path: Some("D:\\db\\asterline.sqlite".to_string()),
            restore: Some(false),
            manual_approvals: None,
            debug: true,
            fake: true,
            auto_update: Some(true),
        };
        let session = options.to_session_options(PathBuf::from("/workspace"));
        assert_eq!(
            session.team_path,
            Some(PathBuf::from("D:\\teams\\core.json"))
        );
        assert_eq!(session.workspace, Some(PathBuf::from("/workspace")));
        assert_eq!(
            session.db_path,
            Some(PathBuf::from("D:\\db\\asterline.sqlite"))
        );
        assert!(!session.restore);
        assert!(!session.manual_approvals);
        assert!(session.fake);
        // An explicit roster file wins over the picker.
        assert!(!session.pick_team);
        assert!(session.auto_update);

        let debug_options = DesktopLaunchOptionsV2 {
            debug: false,
            manual_approvals: Some(true),
            ..Default::default()
        };
        assert!(
            debug_options
                .to_session_options(PathBuf::from("/w"))
                .manual_approvals
        );
    }

    #[test]
    fn mode_overrides_map_onto_the_runtime_command() {
        let command = DesktopCommandV2::SetModeOverrides {
            overrides: ModesConfigV2 {
                team: Some(crate::bridge::TeamModeSettingsV2 {
                    max_iterations: Some(7),
                    ..Default::default()
                }),
                ..Default::default()
            },
        };
        let UiCommand::SetModeOverrides { overrides } = dispatch(command).unwrap() else {
            panic!("expected SetModeOverrides");
        };
        assert_eq!(overrides.team.unwrap().max_iterations, Some(7));
    }

    #[test]
    fn queue_import_and_export_commands_map_through() {
        assert!(matches!(
            dispatch(DesktopCommandV2::EditQueuedPrompt { member: None }).unwrap(),
            UiCommand::EditQueuedPrompt { member: None }
        ));
        assert!(matches!(
            dispatch(DesktopCommandV2::ImportSession {
                member: Some("builder".to_string()),
                session_id: "sess-1".to_string(),
            })
            .unwrap(),
            UiCommand::ImportSession { .. }
        ));
        assert!(
            dispatch(DesktopCommandV2::ImportSession {
                member: None,
                session_id: "  ".to_string(),
            })
            .is_err()
        );
        assert!(matches!(
            dispatch(DesktopCommandV2::ExportSession {
                format: Some("claude".to_string()),
            })
            .unwrap(),
            UiCommand::ExportSession { .. }
        ));
        assert!(matches!(
            dispatch(DesktopCommandV2::RequestTeamSettings).unwrap(),
            UiCommand::RequestTeamSettings
        ));
    }
}
