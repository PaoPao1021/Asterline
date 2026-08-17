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
    ApprovalChoiceV1, DesktopCommandV1, MessageTargetV1, RunStepStatusV1, TeamSettingsV1,
    TerminalModeV1, team_settings_from_domain, team_settings_to_domain,
};

pub enum BootstrapAdapterOutcome {
    Ready(Box<PreparedSession>),
    NeedsTeamSetup {
        pending: Box<PendingTeamSetup>,
        draft: Box<TeamSettingsV1>,
    },
}

pub struct PendingTeamSetup {
    setup: TeamSetup,
}

impl PendingTeamSetup {
    pub fn start(self, settings: &TeamSettingsV1) -> io::Result<PreparedSession> {
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
    pub team: TeamSettingsV1,
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

pub fn bootstrap_workspace(workspace: PathBuf) -> io::Result<BootstrapAdapterOutcome> {
    let options = SessionOptions {
        workspace: Some(workspace.clone()),
        restore: true,
        approvals: true,
        fake: cfg!(debug_assertions)
            && std::env::var_os("ASTERLINE_DESKTOP_FAKE").as_deref()
                == Some(std::ffi::OsStr::new("1")),
        auto_update: false,
        ..SessionOptions::default()
    };
    match app::bootstrap(options, &workspace)? {
        BootstrapOutcome::Ready(session) => Ok(BootstrapAdapterOutcome::Ready(Box::new(
            PreparedSession::new(session)?,
        ))),
        BootstrapOutcome::NeedsTeamSetup(setup) => {
            let workspace_text = setup.workspace.to_string_lossy().into_owned();
            let draft = match default_team(&setup.workspace, setup.detected) {
                Some(team) => team_settings_from_domain(&team, None).map_err(io::Error::other)?,
                None => TeamSettingsV1 {
                    name: "new-team".to_string(),
                    workspace: workspace_text,
                    members: Vec::new(),
                    default_target: None,
                    max_auto_relays: asterline::domain::team::DEFAULT_MAX_AUTO_RELAYS,
                    modes: Default::default(),
                    approvals: Default::default(),
                },
            };
            Ok(BootstrapAdapterOutcome::NeedsTeamSetup {
                pending: Box::new(PendingTeamSetup { setup }),
                draft: Box::new(draft),
            })
        }
    }
}

pub fn command_to_runtime(
    command: DesktopCommandV1,
    workspace: &Path,
) -> Result<UiCommand, String> {
    let run_id = |id: Option<u64>| id.map(RunId);
    match command {
        DesktopCommandV1::RequestLogs { .. }
        | DesktopCommandV1::RequestDiff { .. }
        | DesktopCommandV1::RequestSkills { .. } => {
            Err("utility queries are handled by the desktop host".to_string())
        }
        DesktopCommandV1::SetMode { mode } => Ok(UiCommand::SetMode {
            mode: terminal_mode(mode),
        }),
        DesktopCommandV1::UserMessage { target, body } => {
            require_text(&body, "message")?;
            Ok(UiCommand::UserMessage {
                target: message_target(target),
                body,
            })
        }
        DesktopCommandV1::Cancel { member } => Ok(UiCommand::Cancel {
            member: member.map(MemberId::new),
        }),
        DesktopCommandV1::Retry => Ok(UiCommand::Retry),
        DesktopCommandV1::Approve { id, decision } => Ok(UiCommand::Approve {
            id: ApprovalId(id),
            decision: match decision {
                ApprovalChoiceV1::Approve => ApprovalDecision::Approve,
                ApprovalChoiceV1::Reject => ApprovalDecision::Reject,
            },
        }),
        DesktopCommandV1::SetRelayPaused { paused } => Ok(UiCommand::SetRelayPaused(paused)),
        DesktopCommandV1::ResolvePausedRoute { resume } => {
            Ok(UiCommand::ResolvePausedRoute { resume })
        }
        DesktopCommandV1::SetEffort { .. } => {
            Err("effort changes must be normalized through team settings".to_string())
        }
        DesktopCommandV1::ReplaceTeamSettings { settings } => {
            ensure_workspace_matches(&settings, workspace).map_err(|error| error.to_string())?;
            let settings: TeamSettings = team_settings_to_domain(&settings)?;
            validate_team_settings(&settings, workspace)?;
            Ok(UiCommand::ReplaceTeamSettings {
                settings: Box::new(settings),
            })
        }
        DesktopCommandV1::NewSession => Ok(UiCommand::NewSession),
        DesktopCommandV1::RequestResume => Ok(UiCommand::RequestResume),
        DesktopCommandV1::ResumeConversation { conversation } => {
            Ok(UiCommand::ResumeConversation { conversation })
        }
        DesktopCommandV1::ContinueRun { run_id: id, note } => Ok(UiCommand::ContinueRun {
            run_id: run_id(id),
            note,
        }),
        DesktopCommandV1::NoteRun { run_id: id, note } => {
            require_text(&note, "run note")?;
            Ok(UiCommand::NoteRun {
                run_id: run_id(id),
                note,
            })
        }
        DesktopCommandV1::BlockRun { run_id: id, reason } => {
            require_text(&reason, "block reason")?;
            Ok(UiCommand::BlockRun {
                run_id: run_id(id),
                reason,
            })
        }
        DesktopCommandV1::VerifyRun {
            run_id: id,
            command,
        } => Ok(UiCommand::VerifyRun {
            run_id: run_id(id),
            command,
        }),
        DesktopCommandV1::AddRunStep {
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
        DesktopCommandV1::UpdateRunStep {
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
        DesktopCommandV1::RenameRunStep {
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
        DesktopCommandV1::RemoveRunStep { run_id: id, step } => {
            positive_step(step)?;
            Ok(UiCommand::RemoveRunStep {
                run_id: run_id(id),
                step,
            })
        }
        DesktopCommandV1::AssignRunStep {
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
        DesktopCommandV1::RunMode { mode, task } => {
            require_text(&task, "mode task")?;
            let mode = collab_mode(mode)
                .ok_or_else(|| "normal is not a collaboration run mode".to_string())?;
            Ok(UiCommand::RunMode { mode, task })
        }
        DesktopCommandV1::Shutdown => Ok(UiCommand::Shutdown),
    }
}

fn message_target(target: MessageTargetV1) -> MessageTarget {
    match target {
        MessageTargetV1::Default => MessageTarget::Default,
        MessageTargetV1::All => MessageTarget::All,
        MessageTargetV1::Member { member } => MessageTarget::Member(MemberId::new(member)),
        MessageTargetV1::Members { members } => {
            MessageTarget::Members(members.into_iter().map(MemberId::new).collect())
        }
    }
}

fn terminal_mode(mode: TerminalModeV1) -> TerminalMode {
    match mode {
        TerminalModeV1::Normal => TerminalMode::Normal,
        TerminalModeV1::Review => TerminalMode::Review,
        TerminalModeV1::Plan => TerminalMode::Plan,
        TerminalModeV1::Brainstorm => TerminalMode::Brainstorm,
        TerminalModeV1::Team => TerminalMode::Team,
    }
}

fn collab_mode(mode: TerminalModeV1) -> Option<CollabMode> {
    match mode {
        TerminalModeV1::Normal => None,
        TerminalModeV1::Review => Some(CollabMode::Review),
        TerminalModeV1::Plan => Some(CollabMode::Plan),
        TerminalModeV1::Brainstorm => Some(CollabMode::Brainstorm),
        TerminalModeV1::Team => Some(CollabMode::Team),
    }
}

fn run_step_status(status: RunStepStatusV1) -> RunStepStatus {
    match status {
        RunStepStatusV1::Todo => RunStepStatus::Todo,
        RunStepStatusV1::Doing => RunStepStatus::Doing,
        RunStepStatusV1::Done => RunStepStatus::Done,
        RunStepStatusV1::Blocked => RunStepStatus::Blocked,
    }
}

fn ensure_workspace_matches(settings: &TeamSettingsV1, workspace: &Path) -> io::Result<()> {
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

    #[test]
    fn normal_cannot_be_started_as_a_mode_run() {
        let command = DesktopCommandV1::RunMode {
            mode: TerminalModeV1::Normal,
            task: "task".to_string(),
        };
        assert!(command_to_runtime(command, Path::new("/workspace")).is_err());
    }

    #[test]
    fn zero_step_is_rejected_at_bridge_boundary() {
        let command = DesktopCommandV1::RemoveRunStep {
            run_id: None,
            step: 0,
        };
        assert!(command_to_runtime(command, Path::new("/workspace")).is_err());
    }

    #[test]
    fn unsafe_attached_session_ids_are_rejected() {
        assert!(validate_session_id("thread-123").is_ok());
        assert!(validate_session_id("thread\n123").is_err());
    }

    #[test]
    fn invalid_settings_are_rejected_before_runtime_enqueue() {
        let workspace = std::env::temp_dir();
        let command = DesktopCommandV1::ReplaceTeamSettings {
            settings: Box::new(TeamSettingsV1 {
                name: "empty".to_string(),
                workspace: workspace.to_string_lossy().into_owned(),
                members: Vec::new(),
                default_target: None,
                max_auto_relays: 1,
                modes: Default::default(),
                approvals: Default::default(),
            }),
        };
        let error = command_to_runtime(command, &workspace).unwrap_err();
        assert!(error.contains("team has no members"));
    }
}
