use std::collections::BTreeMap;

use asterline::domain::event::{
    ChatItem, ConversationSummary, MemberSummary, RunSummary, RuntimeEvent,
};
use asterline::domain::mode::ModeStatusSummary;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DESKTOP_BRIDGE_VERSION: u8 = 2;
pub const DESKTOP_EVENT_CHANNEL: &str = "asterline://runtime-event";

/// Timeline caps mirrored from the TUI (`app_state::MAX_CHAT_ITEMS` and
/// `MAX_CHAT_ITEM_BYTES`). When either cap is exceeded the oldest items are
/// dropped and `timeline_truncated` is raised so the UI can say so.
pub const MAX_TIMELINE_ITEMS: usize = 5_000;
pub const MAX_TIMELINE_ITEM_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopPhase {
    #[default]
    Uninitialized,
    Starting,
    NeedsTeamSetup,
    Ready,
    Locked,
    Error,
    ShuttingDown,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalModeV2 {
    #[default]
    Normal,
    Review,
    Plan,
    Brainstorm,
    Team,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKindV2 {
    Codex,
    Claude,
    Grok,
    Agy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatusV2 {
    Idle,
    Queued,
    Running,
    Waiting,
    NeedsApproval,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortV2 {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Ultra,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxPolicyV2 {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPolicyV2 {
    Resume,
    Fresh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PermissionModeV2 {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    #[serde(rename = "plan")]
    Plan,
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "dontAsk")]
    DontAsk,
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatusV2 {
    Planned,
    Running,
    Verifying,
    Done,
    Failed,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStepStatusV2 {
    Todo,
    Doing,
    Done,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberSummaryV2 {
    pub id: String,
    pub display_name: String,
    pub backend: BackendKindV2,
    pub role: String,
    pub status: MemberStatusV2,
    pub session: Option<String>,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<EffortV2>,
    pub sandbox: SandboxPolicyV2,
    pub permission_mode: Option<PermissionModeV2>,
    pub session_policy: SessionPolicyV2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunVerificationV2 {
    pub command: String,
    pub ok: bool,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunStepV2 {
    pub number: u32,
    pub status: RunStepStatusV2,
    pub owner: Option<String>,
    pub title: String,
    pub note: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunEventV2 {
    pub kind: String,
    pub title: String,
    pub detail: Option<String>,
    pub created_at: String,
    pub attempt: u32,
}

/// Structured collaboration-mode state for a run. Mirrors the persisted
/// `mode_state` summary (`ModeStatusSummary`) instead of a free-form string.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModeStateV2 {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub iteration: u32,
    #[serde(default)]
    pub max_iterations: u32,
    #[serde(default)]
    pub round: u32,
    #[serde(default)]
    pub rounds: u32,
    #[serde(default)]
    pub idea_count: u32,
    #[serde(default)]
    pub vote_count: u32,
}

impl From<&ModeStatusSummary> for ModeStateV2 {
    fn from(value: &ModeStatusSummary) -> Self {
        Self {
            phase: value.phase.clone(),
            iteration: value.iteration,
            max_iterations: value.max_iterations,
            round: value.round,
            rounds: value.rounds,
            idea_count: value.idea_count,
            vote_count: value.vote_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModeRunV2 {
    pub mode: TerminalModeV2,
    pub state: ModeStateV2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSummaryV2 {
    pub id: u64,
    pub goal: String,
    pub status: RunStatusV2,
    pub coordinator: Option<String>,
    pub verification: Option<RunVerificationV2>,
    pub created_at: String,
    pub updated_at: String,
    pub attempt: u32,
    pub steps: Vec<RunStepV2>,
    pub events: Vec<RunEventV2>,
    pub mode: Option<ModeRunV2>,
    pub legacy_mode: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalSummaryV2 {
    pub id: u64,
    pub member: Option<String>,
    pub action: String,
    pub body: String,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationSummaryV2 {
    pub id: i64,
    pub created_at: String,
    pub preview: String,
    pub message_count: usize,
    pub member_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKindV2 {
    User,
    Agent,
    Reasoning,
    Tool,
    Diff,
    Route,
    RoutePaused,
    Notice,
    Error,
    Verdict,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimelineItemV2 {
    pub id: String,
    pub kind: TimelineKindV2,
    pub turn: Option<u64>,
    pub member: Option<String>,
    pub display_name: Option<String>,
    pub backend: Option<BackendKindV2>,
    pub text: Option<String>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub to: Option<Vec<String>>,
    pub files: Option<Vec<(String, String)>>,
    pub ok: Option<bool>,
    #[serde(default)]
    pub streaming: bool,
    /// Set when the item text or tool output hit [`MAX_TIMELINE_ITEM_BYTES`].
    #[serde(default)]
    pub truncated: bool,
    /// Managed attachment labels carried by a user message (image placeholders
    /// only; the WebView never sees absolute paths).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<String>,
    pub timestamp: Option<String>,
}

impl TimelineItemV2 {
    fn new(id: String, kind: TimelineKindV2) -> Self {
        Self {
            id,
            kind,
            turn: None,
            member: None,
            display_name: None,
            backend: None,
            text: None,
            title: None,
            detail: None,
            to: None,
            files: None,
            ok: None,
            streaming: false,
            truncated: false,
            attachments: Vec::new(),
            timestamp: None,
        }
    }
}

/// One member's not-yet-started prompt queue, mirrored from the runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberQueueV2 {
    pub member: String,
    pub prompts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DefaultTargetV2 {
    All,
    Member { member: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamMemberSettingsV2 {
    pub id: String,
    pub display_name: String,
    pub backend: BackendKindV2,
    pub role: String,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub sandbox: SandboxPolicyV2,
    pub permission_mode: Option<PermissionModeV2>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub session_policy: SessionPolicyV2,
    pub session_id: Option<String>,
    pub effort: Option<EffortV2>,
    /// Settings fields the desktop does not model yet survive edits unchanged.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSurfaceV2 {
    User,
    Relay,
    Mode,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalPolicyV2 {
    pub gate: Option<Vec<String>>,
    #[serde(default)]
    pub keywords: BTreeMap<String, Vec<String>>,
    pub apply_to: Option<Vec<ApprovalSurfaceV2>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewModeSettingsV2 {
    pub builder: Option<String>,
    pub reviewer: Option<String>,
    pub max_iterations: Option<u32>,
    pub auto_verify: Option<bool>,
    pub verify_command: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Plan mode carries `builder` and `auto_execute` upstream; both are modelled
/// so conversation overrides and team.json edits no longer lose them.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanModeSettingsV2 {
    pub leader: Option<String>,
    pub builder: Option<String>,
    pub reviewer: Option<String>,
    pub max_iterations: Option<u32>,
    pub auto_execute: Option<bool>,
    pub auto_verify: Option<bool>,
    pub verify_command: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BrainstormModeSettingsV2 {
    pub participants: Option<Vec<String>>,
    pub generation_rounds: Option<u32>,
    pub ideas_per_round: Option<u32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamModeSettingsV2 {
    pub coordinator: Option<String>,
    pub max_iterations: Option<u32>,
    pub auto_verify: Option<bool>,
    pub verify_command: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModesConfigV2 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<ReviewModeSettingsV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanModeSettingsV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brainstorm: Option<BrainstormModeSettingsV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamModeSettingsV2>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamSettingsV2 {
    pub name: String,
    pub workspace: String,
    pub members: Vec<TeamMemberSettingsV2>,
    pub default_target: Option<DefaultTargetV2>,
    pub max_auto_relays: u32,
    #[serde(default)]
    pub modes: ModesConfigV2,
    #[serde(default)]
    pub approvals: ApprovalPolicyV2,
    /// Settings fields the desktop does not model yet survive edits unchanged.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesktopSnapshotV2 {
    pub version: u8,
    pub sequence: u64,
    pub phase: DesktopPhase,
    pub workspace: Option<String>,
    pub team: Option<TeamSettingsV2>,
    pub mode: TerminalModeV2,
    /// Conversation-scoped mode knob overrides (field-level).
    #[serde(default)]
    pub mode_overrides: ModesConfigV2,
    /// Per-member prompt queues that have not started yet.
    #[serde(default)]
    pub queues: Vec<MemberQueueV2>,
    /// Whether automatic agent-to-agent relay is paused by the user.
    #[serde(default)]
    pub relay_paused: bool,
    /// Verify command suggested by the workspace (used as the default in
    /// `/verify` and the runs panel).
    #[serde(default)]
    pub suggested_verify: Option<String>,
    /// Conversation currently restored in the workbench, if any.
    #[serde(default)]
    pub active_conversation: Option<i64>,
    /// Set once the timeline had to drop items because a cap was hit.
    #[serde(default)]
    pub timeline_truncated: bool,
    pub members: Vec<MemberSummaryV2>,
    pub runs: Vec<RunSummaryV2>,
    pub approvals: Vec<ApprovalSummaryV2>,
    pub timeline: Vec<TimelineItemV2>,
    pub conversations: Vec<ConversationSummaryV2>,
    pub last_error: Option<String>,
}

impl Default for DesktopSnapshotV2 {
    fn default() -> Self {
        Self {
            version: DESKTOP_BRIDGE_VERSION,
            sequence: 0,
            phase: DesktopPhase::Uninitialized,
            workspace: None,
            team: None,
            mode: TerminalModeV2::Normal,
            mode_overrides: ModesConfigV2::default(),
            queues: Vec::new(),
            relay_paused: false,
            suggested_verify: None,
            active_conversation: None,
            timeline_truncated: false,
            members: Vec::new(),
            runs: Vec::new(),
            approvals: Vec::new(),
            timeline: Vec::new(),
            conversations: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevelV2 {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LogEntryV2 {
    pub level: LogLevelV2,
    pub source: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiffResultV2 {
    pub text: String,
    pub truncated: bool,
    pub file_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillSummaryV2 {
    pub name: String,
    pub description: String,
    pub backend: BackendKindV2,
    pub invocation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DesktopEventV2 {
    pub version: u8,
    pub sequence: u64,
    pub event: DesktopRuntimeEventV2,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[allow(clippy::large_enum_variant)] // Snapshot replacement intentionally carries the full state.
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopRuntimeEventV2 {
    SnapshotReplaced {
        snapshot: DesktopSnapshotV2,
    },
    PhaseChanged {
        phase: DesktopPhase,
        error: Option<String>,
    },
    ModeChanged {
        mode: TerminalModeV2,
    },
    /// Mode bindings changed (`SaveModeDefaults`, `SetModeOverrides`, or a
    /// restore). `defaults` is the team.json/default merge base; `overrides`
    /// are the conversation-scoped knobs.
    ModesUpdated {
        defaults: ModesConfigV2,
        overrides: ModesConfigV2,
    },
    MemberUpdated {
        member: MemberSummaryV2,
    },
    TimelineAdded {
        item: TimelineItemV2,
    },
    TimelineUpdated {
        item: TimelineItemV2,
    },
    ApprovalAdded {
        approval: ApprovalSummaryV2,
    },
    ApprovalRemoved {
        id: u64,
        decision: Option<ApprovalChoiceV2>,
    },
    RunUpdated {
        run: RunSummaryV2,
    },
    ConversationsReplaced {
        conversations: Vec<ConversationSummaryV2>,
    },
    TeamSettingsUpdated {
        team: TeamSettingsV2,
    },
    Notice {
        message: String,
    },
    TurnStarted {
        turn: u64,
    },
    TurnFinished {
        turn: u64,
    },
    /// Structured queue contents for one member after a queue change.
    QueueUpdated {
        member: String,
        prompts: Vec<String>,
    },
    /// The last queued prompt was returned for editing.
    QueuedPromptReturned {
        member: String,
        body: String,
    },
    RoutePaused {
        turn: u64,
        member: String,
        to: Vec<String>,
        reason: String,
        queued: usize,
    },
    SessionUpdated {
        member: String,
        session: String,
    },
    RuntimeLog {
        level: String,
        source: String,
        message: String,
    },
    LogsReplaced {
        request_id: u64,
        entries: Vec<LogEntryV2>,
        truncated: bool,
    },
    DiffReplaced {
        request_id: u64,
        result: DiffResultV2,
    },
    SkillsReplaced {
        request_id: u64,
        skills: Vec<SkillSummaryV2>,
        truncated: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChoiceV2 {
    Approve,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageTargetV2 {
    Default,
    All,
    Member { member: String },
    Members { members: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Team settings replacement carries the full document.
pub enum DesktopCommandV2 {
    RequestLogs {
        request_id: u64,
        member: Option<String>,
        level: Option<LogLevelV2>,
        query: Option<String>,
        limit: Option<usize>,
    },
    RequestDiff {
        request_id: u64,
    },
    RequestSkills {
        request_id: u64,
        backend: Option<BackendKindV2>,
        query: Option<String>,
        limit: Option<usize>,
    },
    SetMode {
        mode: TerminalModeV2,
    },
    /// Replace conversation-scoped mode knob overrides (field-level).
    SetModeOverrides {
        overrides: ModesConfigV2,
    },
    /// Persist the selected mode's current overrides into team.json.
    SaveModeDefaults {
        mode: TerminalModeV2,
    },
    /// Clear the conversation-scoped overrides of one mode.
    ResetModeOverrides {
        mode: TerminalModeV2,
    },
    UserMessage {
        target: MessageTargetV2,
        body: String,
        /// Managed attachment tokens staged via `stage_*` IPC commands. The
        /// Rust host resolves them inside the managed paste directory and
        /// converts them into each backend's native image input.
        #[serde(default)]
        attachments: Vec<String>,
    },
    Cancel {
        member: Option<String>,
    },
    /// Pull the last not-yet-started queued prompt back into the composer.
    EditQueuedPrompt {
        member: Option<String>,
    },
    Retry,
    Approve {
        id: u64,
        decision: ApprovalChoiceV2,
    },
    SetRelayPaused {
        paused: bool,
    },
    ResolvePausedRoute {
        resume: bool,
    },
    ReplaceTeamSettings {
        settings: Box<TeamSettingsV2>,
    },
    /// Ask the runtime to re-emit the full team settings document.
    RequestTeamSettings,
    NewSession,
    RequestResume,
    ResumeConversation {
        conversation: i64,
    },
    /// Import an existing native session by session ID into the chat.
    ImportSession {
        member: Option<String>,
        session_id: String,
    },
    /// Export the current session to Claude Code native format.
    ExportSession {
        format: Option<String>,
    },
    ContinueRun {
        run_id: Option<u64>,
        note: Option<String>,
    },
    NoteRun {
        run_id: Option<u64>,
        note: String,
    },
    BlockRun {
        run_id: Option<u64>,
        reason: String,
    },
    VerifyRun {
        run_id: Option<u64>,
        command: Option<String>,
    },
    AddRunStep {
        run_id: Option<u64>,
        owner: Option<String>,
        title: String,
    },
    UpdateRunStep {
        run_id: Option<u64>,
        step: u32,
        status: RunStepStatusV2,
        note: Option<String>,
    },
    RenameRunStep {
        run_id: Option<u64>,
        step: u32,
        title: String,
    },
    RemoveRunStep {
        run_id: Option<u64>,
        step: u32,
    },
    AssignRunStep {
        run_id: Option<u64>,
        step: u32,
        owner: Option<String>,
    },
    RunMode {
        mode: TerminalModeV2,
        task: String,
    },
    Shutdown,
}

/// State mirrored in the WebView. Runtime events are reduced here before they
/// are emitted, so `get_desktop_snapshot` is always a coherent recovery point.
#[derive(Debug, Default)]
pub struct DesktopModel {
    pub snapshot: DesktopSnapshotV2,
    item_sequence: u64,
    logs: Vec<LogEntryV2>,
}

impl DesktopModel {
    pub fn snapshot(&self) -> DesktopSnapshotV2 {
        self.snapshot.clone()
    }

    pub fn set_phase(
        &mut self,
        phase: DesktopPhase,
        error: Option<String>,
    ) -> DesktopRuntimeEventV2 {
        self.snapshot.phase = phase;
        self.snapshot.last_error = error.clone();
        DesktopRuntimeEventV2::PhaseChanged { phase, error }
    }

    pub fn set_workspace(&mut self, workspace: String) {
        self.snapshot.workspace = Some(workspace);
    }

    pub fn set_team(&mut self, team: TeamSettingsV2) {
        self.snapshot.team = Some(team);
    }

    pub fn seed_logs(&mut self, logs: Vec<LogEntryV2>) {
        self.logs = logs;
        self.trim_logs();
    }

    pub fn logs(&self) -> &[LogEntryV2] {
        &self.logs
    }

    pub fn push_log(&mut self, entry: LogEntryV2) {
        self.logs.push(entry);
        self.trim_logs();
    }

    fn trim_logs(&mut self) {
        const MAX_LOGS: usize = 4_000;
        const MAX_LOG_BYTES: usize = 2 * 1024 * 1024;
        while self.logs.len() > MAX_LOGS
            || self
                .logs
                .iter()
                .map(|entry| entry.source.len().saturating_add(entry.message.len()))
                .sum::<usize>()
                > MAX_LOG_BYTES
        {
            if self.logs.is_empty() {
                break;
            }
            self.logs.remove(0);
        }
    }

    pub fn seed_chat(&mut self, chat: Vec<ChatItem>) {
        self.snapshot.timeline.clear();
        self.snapshot.timeline_truncated = false;
        for item in chat {
            let id = self.fresh_id("history");
            self.add_timeline(timeline_from_chat(id, item));
        }
    }

    pub fn apply_runtime(&mut self, event: RuntimeEvent) -> DesktopRuntimeEventV2 {
        use RuntimeEvent::*;
        match event {
            Ready {
                team: _,
                workspace,
                default_target: _,
                members,
                runs,
                mode_overrides,
                suggested_verify,
                ..
            } => {
                self.snapshot.phase = DesktopPhase::Ready;
                self.snapshot.workspace = Some(workspace);
                self.snapshot.members = members.into_iter().map(member_summary).collect();
                self.snapshot.runs = runs.into_iter().map(run_summary).collect();
                self.snapshot.mode_overrides =
                    modes_config_from_domain(&mode_overrides).unwrap_or_default();
                self.snapshot.suggested_verify = suggested_verify;
                self.snapshot.last_error = None;
                DesktopRuntimeEventV2::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            ModesUpdated {
                defaults,
                overrides,
            } => {
                self.snapshot.mode_overrides =
                    modes_config_from_domain(&overrides).unwrap_or_default();
                DesktopRuntimeEventV2::ModesUpdated {
                    defaults: modes_config_from_domain(&defaults).unwrap_or_default(),
                    overrides: self.snapshot.mode_overrides.clone(),
                }
            }
            ModeChanged { mode } => {
                let mode = terminal_mode(mode.as_str());
                self.snapshot.mode = mode;
                DesktopRuntimeEventV2::ModeChanged { mode }
            }
            TurnStarted { turn } => DesktopRuntimeEventV2::TurnStarted { turn: turn.0 },
            TurnFinished { turn } => DesktopRuntimeEventV2::TurnFinished { turn: turn.0 },
            UserMessage {
                turn,
                targets,
                body,
            } => {
                let (text, attachments) = split_attachment_labels(&body);
                let mut item =
                    TimelineItemV2::new(format!("turn-{}-user", turn.0), TimelineKindV2::User);
                item.turn = Some(turn.0);
                item.to = Some(targets.into_iter().map(|id| id.to_string()).collect());
                item.text = Some(text);
                item.attachments = attachments;
                self.add_timeline(item)
            }
            MemberStatus { member, status } => {
                let id = member.to_string();
                if let Some(summary) = self
                    .snapshot
                    .members
                    .iter_mut()
                    .find(|value| value.id == id)
                {
                    summary.status = member_status(status.as_str());
                    DesktopRuntimeEventV2::MemberUpdated {
                        member: summary.clone(),
                    }
                } else {
                    DesktopRuntimeEventV2::Notice {
                        message: format!("status received for unknown member {id}"),
                    }
                }
            }
            QueueUpdated { member, prompts } => {
                let member = member.to_string();
                self.snapshot.queues.retain(|queue| queue.member != member);
                if !prompts.is_empty() {
                    self.snapshot.queues.push(MemberQueueV2 {
                        member: member.clone(),
                        prompts: prompts.clone(),
                    });
                }
                DesktopRuntimeEventV2::QueueUpdated { member, prompts }
            }
            QueuedPromptReturned { member, body } => {
                let member = member.to_string();
                self.snapshot.queues.retain(|queue| queue.member != member);
                DesktopRuntimeEventV2::QueuedPromptReturned { member, body }
            }
            MessageStarted { msg, turn, member } => {
                let member_id = member.to_string();
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item = TimelineItemV2::new(format!("msg-{}", msg.0), TimelineKindV2::Agent);
                item.turn = Some(turn.0);
                item.member = Some(member_id);
                item.display_name = display_name;
                item.backend = backend;
                item.text = Some(String::new());
                item.streaming = true;
                self.add_timeline(item)
            }
            MessageDelta { msg, text } => {
                self.update_text(&format!("msg-{}", msg.0), &text, true, None)
            }
            MessageCompleted { msg, text } => {
                let id = format!("msg-{}", msg.0);
                if let Some(item) = self.snapshot.timeline.iter_mut().find(|item| item.id == id) {
                    item.text = Some(text);
                    item.streaming = false;
                    DesktopRuntimeEventV2::TimelineUpdated { item: item.clone() }
                } else {
                    DesktopRuntimeEventV2::Notice {
                        message: format!("completion received for unknown message {id}"),
                    }
                }
            }
            Reasoning { member, text } => {
                let member_id = member.to_string();
                let id = self.fresh_id("reasoning");
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item = TimelineItemV2::new(id, TimelineKindV2::Reasoning);
                item.member = Some(member_id);
                item.display_name = display_name;
                item.backend = backend;
                item.text = Some(text);
                self.add_timeline(item)
            }
            ReasoningCompleted { member } => DesktopRuntimeEventV2::RuntimeLog {
                level: "debug".to_string(),
                source: member.to_string(),
                message: "reasoning completed".to_string(),
            },
            ToolStarted {
                member,
                tool_id,
                name,
                summary,
            } => {
                let member_id = member.to_string();
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item =
                    TimelineItemV2::new(tool_item_id(&member_id, &tool_id), TimelineKindV2::Tool);
                item.member = Some(member_id);
                item.display_name = display_name;
                item.backend = backend;
                item.title = Some(name);
                item.text = Some(summary);
                item.detail = Some(String::new());
                item.streaming = true;
                self.add_timeline(item)
            }
            ToolProgress {
                member,
                tool_id,
                delta,
            } => self.update_detail(&tool_item_id(member.as_str(), &tool_id), &delta, true, None),
            ToolCompleted {
                member,
                tool_id,
                ok,
                output,
            } => {
                let id = tool_item_id(member.as_str(), &tool_id);
                if let Some(item) = self.snapshot.timeline.iter_mut().find(|item| item.id == id) {
                    item.detail = Some(output);
                    item.ok = Some(ok);
                    item.streaming = false;
                    DesktopRuntimeEventV2::TimelineUpdated { item: item.clone() }
                } else {
                    DesktopRuntimeEventV2::Notice {
                        message: format!("completion received for unknown tool {tool_id}"),
                    }
                }
            }
            FileChange { member, files, ok } => {
                let member_id = member.to_string();
                let id = self.fresh_id("diff");
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item = TimelineItemV2::new(id, TimelineKindV2::Diff);
                item.member = Some(member_id);
                item.display_name = display_name;
                item.backend = backend;
                item.files = Some(
                    files
                        .into_iter()
                        .map(|file| (file.path, file.kind))
                        .collect(),
                );
                item.ok = Some(ok);
                self.add_timeline(item)
            }
            Route {
                turn,
                from,
                to,
                body,
            } => {
                let mut item = TimelineItemV2::new(self.fresh_id("route"), TimelineKindV2::Route);
                item.turn = Some(turn.0);
                item.member = Some(from.to_string());
                item.to = Some(to);
                item.text = Some(body);
                self.add_timeline(item)
            }
            RouteError {
                turn,
                from,
                target,
                reason,
                body,
            } => {
                let mut item =
                    TimelineItemV2::new(self.fresh_id("route-error"), TimelineKindV2::Error);
                item.turn = Some(turn.0);
                item.member = Some(from.to_string());
                item.to = Some(vec![target]);
                item.title = Some(reason);
                item.text = Some(body);
                self.add_timeline(item)
            }
            RoutePaused {
                turn,
                from,
                to,
                reason,
                queued,
            } => {
                let member = from.to_string();
                let mut item =
                    TimelineItemV2::new(self.fresh_id("route-paused"), TimelineKindV2::RoutePaused);
                item.turn = Some(turn.0);
                item.member = Some(member.clone());
                item.to = Some(to.clone());
                item.text = Some(reason.clone());
                item.detail = Some(queued.to_string());
                self.add_timeline(item);
                DesktopRuntimeEventV2::RoutePaused {
                    turn: turn.0,
                    member,
                    to,
                    reason,
                    queued,
                }
            }
            SessionUpdated { member, session } => {
                let member = member.to_string();
                let session = session.to_string();
                if let Some(summary) = self
                    .snapshot
                    .members
                    .iter_mut()
                    .find(|value| value.id == member)
                {
                    summary.session = Some(session.clone());
                }
                DesktopRuntimeEventV2::SessionUpdated { member, session }
            }
            ApprovalRequested {
                id,
                member,
                action,
                body,
            } => {
                let approval = ApprovalSummaryV2 {
                    id: id.0,
                    member: member.map(|value| value.to_string()),
                    action,
                    body,
                    created_at: None,
                };
                self.snapshot
                    .approvals
                    .retain(|value| value.id != approval.id);
                self.snapshot.approvals.push(approval.clone());
                DesktopRuntimeEventV2::ApprovalAdded { approval }
            }
            ApprovalResolved { id, decision } => {
                self.snapshot.approvals.retain(|value| value.id != id.0);
                let decision = match decision {
                    asterline::domain::event::ApprovalDecision::Approve => {
                        ApprovalChoiceV2::Approve
                    }
                    asterline::domain::event::ApprovalDecision::Reject => ApprovalChoiceV2::Reject,
                };
                DesktopRuntimeEventV2::ApprovalRemoved {
                    id: id.0,
                    decision: Some(decision),
                }
            }
            MemberError { member, message } => {
                let mut item = TimelineItemV2::new(self.fresh_id("error"), TimelineKindV2::Error);
                item.member = Some(member.to_string());
                item.text = Some(message);
                self.add_timeline(item)
            }
            RunUpdated { run } => {
                let run = run_summary(run);
                if let Some(existing) = self
                    .snapshot
                    .runs
                    .iter_mut()
                    .find(|value| value.id == run.id)
                {
                    *existing = run.clone();
                } else {
                    self.snapshot.runs.insert(0, run.clone());
                }
                DesktopRuntimeEventV2::RunUpdated { run }
            }
            Verdict {
                run,
                member,
                approve,
                summary,
            } => {
                let mut item =
                    TimelineItemV2::new(self.fresh_id("verdict"), TimelineKindV2::Verdict);
                item.member = Some(member.to_string());
                item.title = Some(format!("run-{}", run.0));
                item.text = Some(summary);
                item.ok = Some(approve);
                self.add_timeline(item)
            }
            Log(entry) => {
                let level = entry.level.as_str().to_string();
                self.push_log(LogEntryV2 {
                    level: log_level(entry.level.as_str()),
                    source: entry.source.clone(),
                    message: entry.message.clone(),
                });
                DesktopRuntimeEventV2::RuntimeLog {
                    level,
                    source: entry.source,
                    message: entry.message,
                }
            }
            Notice(message) => {
                let mut item = TimelineItemV2::new(self.fresh_id("notice"), TimelineKindV2::Notice);
                item.text = Some(message.clone());
                self.add_timeline(item);
                DesktopRuntimeEventV2::Notice { message }
            }
            SessionReset => {
                self.snapshot.timeline.clear();
                self.snapshot.timeline_truncated = false;
                self.snapshot.approvals.clear();
                self.snapshot.conversations.clear();
                self.snapshot.runs.clear();
                self.snapshot.queues.clear();
                self.snapshot.active_conversation = None;
                for member in &mut self.snapshot.members {
                    member.session = None;
                }
                DesktopRuntimeEventV2::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            ResumeChoices { conversations } => {
                let conversations: Vec<ConversationSummaryV2> = conversations
                    .into_iter()
                    .map(conversation_summary)
                    .collect();
                self.snapshot.conversations = conversations.clone();
                DesktopRuntimeEventV2::ConversationsReplaced { conversations }
            }
            ConversationResumed { conversation, chat } => {
                self.snapshot.active_conversation = Some(conversation);
                self.seed_chat(chat);
                DesktopRuntimeEventV2::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            TeamSettingsUpdated { settings } => {
                match team_settings_from_domain(&settings, self.snapshot.workspace.as_deref()) {
                    Ok(team) => {
                        self.snapshot.team = Some(team.clone());
                        DesktopRuntimeEventV2::TeamSettingsUpdated { team }
                    }
                    Err(message) => DesktopRuntimeEventV2::Notice { message },
                }
            }
            RouteQueueUpdated { queued } => {
                let current = self
                    .snapshot
                    .timeline
                    .iter()
                    .filter(|item| item.kind == TimelineKindV2::RoutePaused)
                    .count();
                for _ in queued..current {
                    self.clear_next_paused_route();
                }
                DesktopRuntimeEventV2::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            AttachGranted { member } => DesktopRuntimeEventV2::Notice {
                message: format!("native session reserved for {member}"),
            },
            AttachDenied { member, reason } => DesktopRuntimeEventV2::Notice {
                message: format!("could not attach to {member}: {reason}"),
            },
        }
    }

    fn member_identity(&self, member: &str) -> (Option<String>, Option<BackendKindV2>) {
        self.snapshot
            .members
            .iter()
            .find(|value| value.id == member)
            .map(|value| (Some(value.display_name.clone()), Some(value.backend)))
            .unwrap_or((None, None))
    }

    fn fresh_id(&mut self, prefix: &str) -> String {
        self.item_sequence = self.item_sequence.saturating_add(1);
        format!("{prefix}-{}", self.item_sequence)
    }

    fn add_timeline(&mut self, item: TimelineItemV2) -> DesktopRuntimeEventV2 {
        self.push_timeline(item);
        match self.snapshot.timeline.last() {
            Some(latest) => DesktopRuntimeEventV2::TimelineAdded {
                item: latest.clone(),
            },
            None => DesktopRuntimeEventV2::Notice {
                message: "timeline item was dropped".to_string(),
            },
        }
    }

    /// Append with the TUI's caps: 5 000 items and 256 KiB per item. Dropping
    /// is reported once through `timeline_truncated` so the UI can show an
    /// explicit truncation state instead of silently losing history.
    fn push_timeline(&mut self, item: TimelineItemV2) {
        self.snapshot.timeline.push(item);
        let mut dropped = 0usize;
        while self.snapshot.timeline.len() > MAX_TIMELINE_ITEMS {
            self.snapshot.timeline.remove(0);
            dropped += 1;
        }
        if dropped > 0 {
            self.snapshot.timeline_truncated = true;
        }
    }

    fn update_text(
        &mut self,
        id: &str,
        delta: &str,
        streaming: bool,
        ok: Option<bool>,
    ) -> DesktopRuntimeEventV2 {
        if let Some(item) = self.snapshot.timeline.iter_mut().find(|item| item.id == id) {
            clamp_bounded_text(&mut item.text, delta, &mut item.truncated);
            item.streaming = streaming;
            item.ok = ok;
            DesktopRuntimeEventV2::TimelineUpdated { item: item.clone() }
        } else {
            DesktopRuntimeEventV2::Notice {
                message: format!("update received for unknown timeline item {id}"),
            }
        }
    }

    fn update_detail(
        &mut self,
        id: &str,
        delta: &str,
        streaming: bool,
        ok: Option<bool>,
    ) -> DesktopRuntimeEventV2 {
        if let Some(item) = self.snapshot.timeline.iter_mut().find(|item| item.id == id) {
            clamp_bounded_text(&mut item.detail, delta, &mut item.truncated);
            item.streaming = streaming;
            item.ok = ok;
            DesktopRuntimeEventV2::TimelineUpdated { item: item.clone() }
        } else {
            DesktopRuntimeEventV2::Notice {
                message: format!("update received for unknown timeline item {id}"),
            }
        }
    }

    pub fn clear_next_paused_route(&mut self) {
        if let Some(index) = self
            .snapshot
            .timeline
            .iter()
            .position(|item| item.kind == TimelineKindV2::RoutePaused)
        {
            self.snapshot.timeline.remove(index);
        }
    }
}

/// User prompts carry image placeholders like `[Image #1]`; those labels move
/// to the structured `attachments` field so the WebView can render them
/// without parsing message text.
fn split_attachment_labels(body: &str) -> (String, Vec<String>) {
    const PLACEHOLDER_PREFIX: &str = "[Image #";
    let mut text = String::new();
    let mut attachments = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(PLACEHOLDER_PREFIX) && trimmed.ends_with(']') {
            attachments.push(trimmed.to_string());
            continue;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(line);
    }
    (text, attachments)
}

fn clamp_bounded_text(buffer: &mut Option<String>, delta: &str, truncated: &mut bool) {
    let text = buffer.get_or_insert_with(String::new);
    let remaining = MAX_TIMELINE_ITEM_BYTES.saturating_sub(text.len());
    if remaining == 0 {
        *truncated = true;
        return;
    }
    let mut end = delta.len().min(remaining);
    while end > 0 && !delta.is_char_boundary(end) {
        end -= 1;
    }
    text.push_str(&delta[..end]);
    if end < delta.len() {
        *truncated = true;
    }
}

pub fn envelope(sequence: u64, mut event: DesktopRuntimeEventV2) -> DesktopEventV2 {
    if let DesktopRuntimeEventV2::SnapshotReplaced { snapshot } = &mut event {
        snapshot.sequence = sequence;
    }
    DesktopEventV2 {
        version: DESKTOP_BRIDGE_VERSION,
        sequence,
        event,
    }
}

pub fn team_settings_from_domain<T: Serialize>(
    settings: &T,
    workspace: Option<&str>,
) -> Result<TeamSettingsV2, String> {
    let mut value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    if let Some(workspace) = workspace {
        value
            .as_object_mut()
            .ok_or_else(|| "team settings were not an object".to_string())?
            .entry("workspace")
            .or_insert_with(|| Value::String(workspace.to_string()));
    }
    normalize_team_members_from_domain(&mut value)?;
    normalize_default_target_from_domain(&mut value)?;
    serde_json::from_value(value)
        .map_err(|error| format!("could not encode team settings: {error}"))
}

fn normalize_team_members_from_domain(value: &mut Value) -> Result<(), String> {
    let members = value
        .get_mut("members")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "team settings were missing members".to_string())?;
    for member in members {
        let member = member
            .as_object_mut()
            .ok_or_else(|| "team member was not an object".to_string())?;
        let backend = member
            .get("backend")
            .and_then(Value::as_str)
            .ok_or_else(|| "team member was missing backend".to_string())?;
        if !member.contains_key("id") {
            let display_name = member
                .get("display_name")
                .and_then(Value::as_str)
                .ok_or_else(|| "team member was missing display_name".to_string())?;
            let fallback = backend;
            let id = asterline::domain::team::derived_member_id(display_name, fallback).to_string();
            member.insert("id".to_string(), Value::String(id));
        }
        member
            .entry("sandbox")
            .or_insert_with(|| Value::String("read-only".to_string()));
        member
            .entry("session_policy")
            .or_insert_with(|| Value::String("resume".to_string()));
        member
            .entry("allowed_tools")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    Ok(())
}

pub fn team_settings_to_domain<T: DeserializeOwned>(
    settings: &TeamSettingsV2,
) -> Result<T, String> {
    let mut value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    normalize_default_target_to_domain(&mut value)?;
    serde_json::from_value(value).map_err(|error| format!("invalid team settings: {error}"))
}

/// Convert runtime mode bindings into the DTO shape. Unknown fields survive
/// through each settings block's `extra` map (data-loss fix for settings the
/// desktop does not model yet, e.g. plan `builder`/`auto_execute` before V2).
pub fn modes_config_from_domain(
    modes: &asterline::domain::mode::ModesConfig,
) -> Result<ModesConfigV2, String> {
    let value = serde_json::to_value(modes).map_err(|error| error.to_string())?;
    serde_json::from_value(value)
        .map_err(|error| format!("could not encode mode settings: {error}"))
}

/// Convert DTO mode overrides back into the runtime vocabulary.
pub fn modes_config_to_domain(
    modes: &ModesConfigV2,
) -> Result<asterline::domain::mode::ModesConfig, String> {
    let value = serde_json::to_value(modes).map_err(|error| error.to_string())?;
    serde_json::from_value(value).map_err(|error| format!("invalid mode settings: {error}"))
}

/// Clear one mode's conversation override block in the DTO vocabulary
/// (`ResetModeOverrides`).
pub fn clear_mode_overrides(overrides: &mut ModesConfigV2, mode: TerminalModeV2) {
    match mode {
        TerminalModeV2::Normal => {}
        TerminalModeV2::Review => overrides.review = None,
        TerminalModeV2::Plan => overrides.plan = None,
        TerminalModeV2::Brainstorm => overrides.brainstorm = None,
        TerminalModeV2::Team => overrides.team = None,
    }
}

fn normalize_default_target_from_domain(value: &mut Value) -> Result<(), String> {
    let Some(root) = value.as_object_mut() else {
        return Err("team settings were not an object".to_string());
    };
    let Some(target) = root.get_mut("default_target") else {
        return Ok(());
    };
    if target.is_null() {
        return Ok(());
    }
    if target.as_str() == Some("all") {
        *target = serde_json::json!({ "type": "all" });
        return Ok(());
    }
    if let Some(member) = target.get("member").and_then(Value::as_str) {
        *target = serde_json::json!({ "type": "member", "member": member });
        return Ok(());
    }
    Err("team settings contained an unknown default target".to_string())
}

fn normalize_default_target_to_domain(value: &mut Value) -> Result<(), String> {
    let Some(root) = value.as_object_mut() else {
        return Err("team settings were not an object".to_string());
    };
    let Some(target) = root.get_mut("default_target") else {
        return Ok(());
    };
    if target.is_null() {
        return Ok(());
    }
    match target.get("type").and_then(Value::as_str) {
        Some("all") => *target = Value::String("all".to_string()),
        Some("member") => {
            let member = target
                .get("member")
                .and_then(Value::as_str)
                .ok_or_else(|| "member default target needs a member".to_string())?;
            *target = serde_json::json!({ "member": member });
        }
        _ => return Err("team settings contained an unknown default target".to_string()),
    }
    Ok(())
}

fn timeline_from_chat(id: String, item: ChatItem) -> TimelineItemV2 {
    match item {
        ChatItem::User { body, .. } => {
            let (text, attachments) = split_attachment_labels(&body);
            let mut item = TimelineItemV2::new(id, TimelineKindV2::User);
            item.text = Some(text);
            item.attachments = attachments;
            item
        }
        ChatItem::Agent {
            member,
            display_name,
            backend,
            text,
        } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Agent);
            item.member = Some(member.to_string());
            item.display_name = Some(display_name);
            item.backend = Some(backend_kind(backend.as_str()));
            item.text = Some(text);
            item
        }
        ChatItem::Thinking {
            member,
            display_name,
            backend,
            text,
            ..
        } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Reasoning);
            item.member = Some(member.to_string());
            item.display_name = Some(display_name);
            item.backend = Some(backend_kind(backend.as_str()));
            item.text = Some(text);
            item
        }
        ChatItem::Tool {
            member,
            name,
            summary,
            detail,
            ok,
        } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Tool);
            item.member = Some(member.to_string());
            item.title = Some(name);
            item.text = Some(summary);
            item.detail = Some(detail);
            item.ok = ok;
            item
        }
        ChatItem::Diff { member, files, .. } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Diff);
            item.member = Some(member.to_string());
            item.files = Some(
                files
                    .into_iter()
                    .map(|file| (file.path, file.kind))
                    .collect(),
            );
            item
        }
        ChatItem::Route { from, to, body } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Route);
            item.member = Some(from.to_string());
            item.to = Some(to);
            item.text = Some(body);
            item
        }
        ChatItem::Notice { text } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Notice);
            item.text = Some(text);
            item
        }
        ChatItem::Error { member, message } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Error);
            item.member = member.map(|value| value.to_string());
            item.text = Some(message);
            item
        }
        ChatItem::Verdict {
            member,
            approve,
            summary,
        } => {
            let mut item = TimelineItemV2::new(id, TimelineKindV2::Verdict);
            item.member = Some(member.to_string());
            item.ok = Some(approve);
            item.text = Some(summary);
            item
        }
    }
}

fn member_summary(value: MemberSummary) -> MemberSummaryV2 {
    MemberSummaryV2 {
        id: value.id.to_string(),
        display_name: value.display_name,
        backend: backend_kind(value.backend.as_str()),
        role: value.role,
        status: member_status(value.status.as_str()),
        session: value.session,
        cwd: value.cwd,
        model: value.model,
        effort: value.effort.map(|effort| effort_v2(effort.as_str())),
        sandbox: sandbox_policy(value.sandbox.codex_arg()),
        permission_mode: value
            .permission_mode
            .map(|mode| permission_mode(mode.claude_arg())),
        session_policy: match value.session_policy {
            asterline::domain::team::SessionPolicy::Resume => SessionPolicyV2::Resume,
            asterline::domain::team::SessionPolicy::Fresh => SessionPolicyV2::Fresh,
        },
    }
}

fn run_summary(value: RunSummary) -> RunSummaryV2 {
    RunSummaryV2 {
        id: value.id.0,
        goal: value.goal,
        status: run_status(value.status.as_str()),
        coordinator: value.coordinator.map(|id| id.to_string()),
        verification: value.verification.map(|verification| RunVerificationV2 {
            command: verification.command,
            ok: verification.ok,
            summary: verification.summary,
        }),
        created_at: value.created_at,
        updated_at: value.updated_at,
        attempt: value.attempt,
        steps: value
            .steps
            .into_iter()
            .map(|step| RunStepV2 {
                number: step.number,
                status: run_step_status(step.status.as_str()),
                owner: step.owner.map(|id| id.to_string()),
                title: step.title,
                note: step.note,
                updated_at: step.updated_at,
            })
            .collect(),
        events: value
            .events
            .into_iter()
            .map(|event| RunEventV2 {
                kind: event.kind,
                title: event.title,
                detail: event.detail,
                created_at: event.created_at,
                attempt: event.attempt,
            })
            .collect(),
        mode: value.mode.map(|mode| ModeRunV2 {
            mode: terminal_mode(mode.mode.as_str()),
            state: ModeStateV2::from(&mode.state),
        }),
        legacy_mode: value.legacy_mode,
    }
}

fn conversation_summary(value: ConversationSummary) -> ConversationSummaryV2 {
    ConversationSummaryV2 {
        id: value.id,
        created_at: value.created_at,
        preview: value.preview,
        message_count: value.message_count,
        member_count: value.member_count,
    }
}

fn tool_item_id(member: &str, tool_id: &str) -> String {
    format!("tool-{member}-{tool_id}")
}

fn terminal_mode(value: &str) -> TerminalModeV2 {
    match value {
        "review" => TerminalModeV2::Review,
        "plan" => TerminalModeV2::Plan,
        "brainstorm" => TerminalModeV2::Brainstorm,
        "team" => TerminalModeV2::Team,
        _ => TerminalModeV2::Normal,
    }
}

fn log_level(value: &str) -> LogLevelV2 {
    match value {
        "debug" => LogLevelV2::Debug,
        "warn" => LogLevelV2::Warn,
        "error" => LogLevelV2::Error,
        _ => LogLevelV2::Info,
    }
}

pub(crate) fn backend_kind(value: &str) -> BackendKindV2 {
    match value {
        "claude" => BackendKindV2::Claude,
        "grok" => BackendKindV2::Grok,
        "agy" => BackendKindV2::Agy,
        _ => BackendKindV2::Codex,
    }
}

fn member_status(value: &str) -> MemberStatusV2 {
    match value {
        "queued" => MemberStatusV2::Queued,
        "running" => MemberStatusV2::Running,
        "waiting" => MemberStatusV2::Waiting,
        "needs_approval" => MemberStatusV2::NeedsApproval,
        "failed" => MemberStatusV2::Failed,
        _ => MemberStatusV2::Idle,
    }
}

fn effort_v2(value: &str) -> EffortV2 {
    match value {
        "low" => EffortV2::Low,
        "high" => EffortV2::High,
        "xhigh" => EffortV2::Xhigh,
        "max" => EffortV2::Max,
        "ultra" => EffortV2::Ultra,
        _ => EffortV2::Medium,
    }
}

fn sandbox_policy(value: &str) -> SandboxPolicyV2 {
    match value {
        "workspace-write" => SandboxPolicyV2::WorkspaceWrite,
        "danger-full-access" => SandboxPolicyV2::DangerFullAccess,
        _ => SandboxPolicyV2::ReadOnly,
    }
}

fn permission_mode(value: &str) -> PermissionModeV2 {
    match value {
        "acceptEdits" => PermissionModeV2::AcceptEdits,
        "plan" => PermissionModeV2::Plan,
        "auto" => PermissionModeV2::Auto,
        "dontAsk" => PermissionModeV2::DontAsk,
        "bypassPermissions" => PermissionModeV2::BypassPermissions,
        _ => PermissionModeV2::Default,
    }
}

fn run_status(value: &str) -> RunStatusV2 {
    match value {
        "running" => RunStatusV2::Running,
        "verifying" => RunStatusV2::Verifying,
        "done" => RunStatusV2::Done,
        "failed" => RunStatusV2::Failed,
        "blocked" => RunStatusV2::Blocked,
        _ => RunStatusV2::Planned,
    }
}

fn run_step_status(value: &str) -> RunStepStatusV2 {
    match value {
        "doing" => RunStepStatusV2::Doing,
        "done" => RunStepStatusV2::Done,
        "blocked" => RunStepStatusV2::Blocked,
        _ => RunStepStatusV2::Todo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asterline::domain::event::{MessageId, TurnId};
    use asterline::domain::mode::PlanModeConfig;
    use asterline::domain::team::{
        BackendKind, DefaultTarget, MemberId, TeamConfig, TeamMember, TeamSettings,
    };

    fn member(id: &str) -> MemberSummaryV2 {
        MemberSummaryV2 {
            id: id.to_string(),
            display_name: id.to_string(),
            backend: BackendKindV2::Codex,
            role: "implementation".to_string(),
            status: MemberStatusV2::Idle,
            session: None,
            cwd: "/workspace".to_string(),
            model: None,
            effort: None,
            sandbox: SandboxPolicyV2::ReadOnly,
            permission_mode: None,
            session_policy: SessionPolicyV2::Resume,
        }
    }

    #[test]
    fn streaming_message_is_reduced_into_recoverable_snapshot() {
        let mut model = DesktopModel::default();
        model.snapshot.members.push(member("builder"));

        model.apply_runtime(RuntimeEvent::MessageStarted {
            msg: MessageId(7),
            turn: TurnId(3),
            member: MemberId::new("builder"),
        });
        model.apply_runtime(RuntimeEvent::MessageDelta {
            msg: MessageId(7),
            text: "hello".to_string(),
        });
        model.apply_runtime(RuntimeEvent::MessageCompleted {
            msg: MessageId(7),
            text: "hello world".to_string(),
        });

        let item = &model.snapshot.timeline[0];
        assert_eq!(item.id, "msg-7");
        assert_eq!(item.text.as_deref(), Some("hello world"));
        assert!(!item.streaming);
    }

    #[test]
    fn team_default_target_round_trips_through_bridge_shape() {
        let mut team = TeamConfig::new("test", "/workspace").with_member(TeamMember::new(
            "builder",
            "Builder",
            BackendKind::Codex,
            "implementation",
        ));
        team.default_target = Some(DefaultTarget::Member(MemberId::new("builder")));

        let dto = team_settings_from_domain(&team, None).unwrap();
        assert_eq!(
            dto.default_target,
            Some(DefaultTargetV2::Member {
                member: "builder".to_string()
            })
        );
        let decoded: TeamConfig = team_settings_to_domain(&dto).unwrap();
        assert_eq!(decoded.default_target, team.default_target);
    }

    #[test]
    fn snapshot_replacement_carries_envelope_sequence() {
        let event = envelope(
            42,
            DesktopRuntimeEventV2::SnapshotReplaced {
                snapshot: DesktopSnapshotV2::default(),
            },
        );
        assert_eq!(event.sequence, 42);
        let DesktopRuntimeEventV2::SnapshotReplaced { snapshot } = event.event else {
            panic!("wrong event");
        };
        assert_eq!(snapshot.sequence, 42);
    }

    #[test]
    fn ordinary_team_settings_fill_wire_defaults_and_round_trip() {
        let config = TeamConfig::new("test", "/workspace").with_member(TeamMember::new(
            "codex",
            "Codex",
            BackendKind::Codex,
            "general",
        ));
        let settings = TeamSettings::from_config(&config);
        let dto = team_settings_from_domain(&settings, Some("/workspace")).unwrap();
        assert_eq!(dto.members[0].id, "codex");
        assert_eq!(dto.members[0].sandbox, SandboxPolicyV2::ReadOnly);
        assert_eq!(dto.members[0].session_policy, SessionPolicyV2::Resume);
        assert!(dto.members[0].allowed_tools.is_empty());

        let decoded: TeamSettings = team_settings_to_domain(&dto).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn paused_route_is_retained_in_recovery_snapshot_until_resolved() {
        let mut model = DesktopModel::default();
        let event = model.apply_runtime(RuntimeEvent::RoutePaused {
            turn: TurnId(9),
            from: MemberId::new("builder"),
            to: vec!["reviewer".to_string()],
            reason: "relay limit reached".to_string(),
            queued: 1,
        });
        assert!(matches!(event, DesktopRuntimeEventV2::RoutePaused { .. }));
        assert_eq!(model.snapshot.timeline[0].kind, TimelineKindV2::RoutePaused);
        model.clear_next_paused_route();
        assert!(model.snapshot.timeline.is_empty());
    }

    #[test]
    fn resolving_a_route_preserves_later_paused_routes() {
        let mut model = DesktopModel::default();
        for turn in [9, 10] {
            model.apply_runtime(RuntimeEvent::RoutePaused {
                turn: TurnId(turn),
                from: MemberId::new("builder"),
                to: vec!["reviewer".to_string()],
                reason: "relay limit reached".to_string(),
                queued: 1,
            });
        }
        model.clear_next_paused_route();
        let paused = model
            .snapshot
            .timeline
            .iter()
            .filter(|item| item.kind == TimelineKindV2::RoutePaused)
            .collect::<Vec<_>>();
        assert_eq!(paused.len(), 1);
        assert_eq!(paused[0].turn, Some(10));
    }

    #[test]
    fn session_reset_clears_member_sessions_from_recovery_snapshot() {
        let mut model = DesktopModel::default();
        let mut builder = member("builder");
        builder.session = Some("old-session".to_string());
        model.snapshot.members.push(builder);
        model.snapshot.active_conversation = Some(3);
        model.apply_runtime(RuntimeEvent::SessionReset);
        assert_eq!(model.snapshot.members[0].session, None);
        assert_eq!(model.snapshot.active_conversation, None);
        assert!(model.snapshot.queues.is_empty());
    }

    #[test]
    fn runtime_logs_are_retained_outside_the_chat_timeline() {
        let mut model = DesktopModel::default();
        let event = model.apply_runtime(RuntimeEvent::Log(
            asterline::domain::event::LogEntry::warn("builder", "approval pending"),
        ));
        assert!(matches!(event, DesktopRuntimeEventV2::RuntimeLog { .. }));
        assert_eq!(model.logs()[0].source, "builder");
        assert_eq!(model.logs()[0].level, LogLevelV2::Warn);
        assert!(model.snapshot.timeline.is_empty());
    }

    #[test]
    fn utility_commands_use_versioned_request_ids() {
        let command = DesktopCommandV2::RequestSkills {
            request_id: 17,
            backend: Some(BackendKindV2::Claude),
            query: Some("audit".to_string()),
            limit: Some(8),
        };
        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["type"], "request_skills");
        assert_eq!(value["request_id"], 17);
        assert_eq!(value["backend"], "claude");
    }

    #[test]
    fn queue_updates_are_structured_not_notices() {
        let mut model = DesktopModel::default();
        let event = model.apply_runtime(RuntimeEvent::QueueUpdated {
            member: MemberId::new("builder"),
            prompts: vec!["first".to_string(), "second".to_string()],
        });
        let DesktopRuntimeEventV2::QueueUpdated { member, prompts } = event else {
            panic!("queue update must stay structured");
        };
        assert_eq!(member, "builder");
        assert_eq!(prompts.len(), 2);
        assert_eq!(model.snapshot.queues[0].prompts.len(), 2);

        // An empty queue removes the member entry entirely.
        model.apply_runtime(RuntimeEvent::QueueUpdated {
            member: MemberId::new("builder"),
            prompts: Vec::new(),
        });
        assert!(model.snapshot.queues.is_empty());
    }

    #[test]
    fn queued_prompt_return_is_structured_and_clears_the_queue() {
        let mut model = DesktopModel::default();
        model.apply_runtime(RuntimeEvent::QueueUpdated {
            member: MemberId::new("builder"),
            prompts: vec!["come back".to_string()],
        });
        let event = model.apply_runtime(RuntimeEvent::QueuedPromptReturned {
            member: MemberId::new("builder"),
            body: "come back".to_string(),
        });
        let DesktopRuntimeEventV2::QueuedPromptReturned { member, body } = event else {
            panic!("queued prompt return must stay structured");
        };
        assert_eq!(member, "builder");
        assert_eq!(body, "come back");
        assert!(model.snapshot.queues.is_empty());
    }

    #[test]
    fn user_message_placeholders_become_structured_attachments() {
        let mut model = DesktopModel::default();
        model.apply_runtime(RuntimeEvent::UserMessage {
            turn: TurnId(1),
            targets: vec![MemberId::new("builder")],
            body: "check this\n[Image #1]\nand this".to_string(),
        });
        let item = &model.snapshot.timeline[0];
        assert_eq!(item.kind, TimelineKindV2::User);
        assert_eq!(item.text.as_deref(), Some("check this\nand this"));
        assert_eq!(item.attachments, vec!["[Image #1]".to_string()]);
    }

    #[test]
    fn modes_updated_is_structured_with_defaults_and_overrides() {
        let mut model = DesktopModel::default();
        let overrides = asterline::domain::mode::ModesConfig {
            team: Some(asterline::domain::mode::TeamModeConfig {
                max_iterations: Some(9),
                ..Default::default()
            }),
            ..Default::default()
        };
        let event = model.apply_runtime(RuntimeEvent::ModesUpdated {
            defaults: asterline::domain::mode::ModesConfig::default(),
            overrides: overrides.clone(),
        });
        let DesktopRuntimeEventV2::ModesUpdated {
            defaults,
            overrides: dto_overrides,
        } = event
        else {
            panic!("ModesUpdated must stay structured");
        };
        assert!(defaults.team.is_none());
        assert_eq!(dto_overrides.team.unwrap().max_iterations, Some(9));
        assert_eq!(
            model.snapshot.mode_overrides.team.unwrap().max_iterations,
            Some(9)
        );
    }

    #[test]
    fn plan_mode_builder_and_auto_execute_round_trip() {
        let modes = asterline::domain::mode::ModesConfig {
            plan: Some(PlanModeConfig {
                leader: Some(MemberId::new("lead")),
                builder: Some(MemberId::new("builder")),
                reviewer: Some(MemberId::new("reviewer")),
                max_iterations: Some(2),
                auto_execute: Some(true),
                auto_verify: Some(false),
                verify_command: Some("cargo test".to_string()),
            }),
            ..Default::default()
        };
        let dto = modes_config_from_domain(&modes).unwrap();
        let plan = dto.plan.as_ref().expect("plan settings");
        assert_eq!(plan.builder.as_deref(), Some("builder"));
        assert_eq!(plan.auto_execute, Some(true));
        assert_eq!(plan.leader.as_deref(), Some("lead"));
        let decoded = modes_config_to_domain(&dto).unwrap();
        assert_eq!(decoded, modes);
    }

    #[test]
    fn conversation_restore_tracks_active_conversation() {
        let mut model = DesktopModel::default();
        model.apply_runtime(RuntimeEvent::ConversationResumed {
            conversation: 41,
            chat: vec![ChatItem::Notice {
                text: "restored".to_string(),
            }],
        });
        assert_eq!(model.snapshot.active_conversation, Some(41));
    }

    #[test]
    fn timeline_caps_mirror_the_tui_and_report_truncation() {
        let mut model = DesktopModel::default();
        for index in 0..(MAX_TIMELINE_ITEMS + 25) {
            let mut item = TimelineItemV2::new(format!("notice-{index}"), TimelineKindV2::Notice);
            item.text = Some(index.to_string());
            model.push_timeline(item);
        }
        assert_eq!(model.snapshot.timeline.len(), MAX_TIMELINE_ITEMS);
        assert!(model.snapshot.timeline_truncated);
        assert_eq!(model.snapshot.timeline[0].text.as_deref(), Some("25"));
    }

    #[test]
    fn oversized_streaming_text_is_clamped_with_an_explicit_flag() {
        let mut model = DesktopModel::default();
        model.apply_runtime(RuntimeEvent::MessageStarted {
            msg: MessageId(1),
            turn: TurnId(1),
            member: MemberId::new("builder"),
        });
        let chunk = "x".repeat(64 * 1024);
        for _ in 0..8 {
            model.apply_runtime(RuntimeEvent::MessageDelta {
                msg: MessageId(1),
                text: chunk.clone(),
            });
        }
        let item = &model.snapshot.timeline[0];
        assert!(item.text.as_ref().unwrap().len() <= MAX_TIMELINE_ITEM_BYTES);
        assert!(item.truncated);
    }

    #[test]
    fn ready_event_carries_overrides_and_suggested_verify() {
        let mut model = DesktopModel::default();
        let overrides = asterline::domain::mode::ModesConfig {
            review: Some(asterline::domain::mode::ReviewModeConfig {
                max_iterations: Some(4),
                ..Default::default()
            }),
            ..Default::default()
        };
        model.apply_runtime(RuntimeEvent::Ready {
            team: "test".to_string(),
            workspace: "/workspace".to_string(),
            default_target: None,
            members: Vec::new(),
            runs: Vec::new(),
            modes: asterline::domain::mode::ModesConfig::default(),
            mode_overrides: overrides,
            suggested_verify: Some("cargo test --locked".to_string()),
        });
        assert_eq!(model.snapshot.phase, DesktopPhase::Ready);
        assert_eq!(
            model.snapshot.mode_overrides.review.unwrap().max_iterations,
            Some(4)
        );
        assert_eq!(
            model.snapshot.suggested_verify.as_deref(),
            Some("cargo test --locked")
        );
    }

    #[test]
    fn run_mode_state_is_structured_not_a_json_string() {
        let run = RunSummary {
            id: asterline::domain::event::RunId(3),
            goal: "ship".to_string(),
            status: asterline::domain::event::RunStatus::Running,
            coordinator: None,
            verification: None,
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
            attempt: 1,
            events: Vec::new(),
            steps: Vec::new(),
            mode: Some(asterline::domain::event::ModeRunStatus {
                mode: asterline::domain::mode::CollabMode::Brainstorm,
                state: ModeStatusSummary {
                    phase: "voting".to_string(),
                    round: 2,
                    rounds: 3,
                    idea_count: 12,
                    vote_count: 3,
                    ..Default::default()
                },
            }),
            legacy_mode: None,
        };
        let dto = run_summary(run);
        let mode = dto.mode.expect("mode run");
        assert_eq!(mode.state.phase, "voting");
        assert_eq!(mode.state.round, 2);
        assert_eq!(mode.state.idea_count, 12);
        assert_eq!(mode.mode, TerminalModeV2::Brainstorm);
    }
}
