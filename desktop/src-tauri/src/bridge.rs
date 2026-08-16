use std::collections::BTreeMap;

use asterline::domain::event::{
    ChatItem, ConversationSummary, MemberSummary, RunSummary, RuntimeEvent,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DESKTOP_BRIDGE_VERSION: u8 = 1;
pub const DESKTOP_EVENT_CHANNEL: &str = "asterline://runtime-event";

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
pub enum TerminalModeV1 {
    #[default]
    Normal,
    Review,
    Plan,
    Brainstorm,
    Team,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKindV1 {
    Codex,
    Claude,
    Grok,
    Agy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatusV1 {
    Idle,
    Queued,
    Running,
    Waiting,
    NeedsApproval,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortV1 {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Ultra,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxPolicyV1 {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPolicyV1 {
    Resume,
    Fresh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PermissionModeV1 {
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
pub enum RunStatusV1 {
    Planned,
    Running,
    Verifying,
    Done,
    Failed,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStepStatusV1 {
    Todo,
    Doing,
    Done,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberSummaryV1 {
    pub id: String,
    pub display_name: String,
    pub backend: BackendKindV1,
    pub role: String,
    pub status: MemberStatusV1,
    pub session: Option<String>,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<EffortV1>,
    pub sandbox: SandboxPolicyV1,
    pub permission_mode: Option<PermissionModeV1>,
    pub session_policy: SessionPolicyV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunVerificationV1 {
    pub command: String,
    pub ok: bool,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunStepV1 {
    pub number: u32,
    pub status: RunStepStatusV1,
    pub owner: Option<String>,
    pub title: String,
    pub note: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunEventV1 {
    pub kind: String,
    pub title: String,
    pub detail: Option<String>,
    pub created_at: String,
    pub attempt: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModeRunV1 {
    pub mode: TerminalModeV1,
    pub state: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunSummaryV1 {
    pub id: u64,
    pub goal: String,
    pub status: RunStatusV1,
    pub coordinator: Option<String>,
    pub verification: Option<RunVerificationV1>,
    pub created_at: String,
    pub updated_at: String,
    pub attempt: u32,
    pub steps: Vec<RunStepV1>,
    pub events: Vec<RunEventV1>,
    pub mode: Option<ModeRunV1>,
    pub legacy_mode: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalSummaryV1 {
    pub id: u64,
    pub member: Option<String>,
    pub action: String,
    pub body: String,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationSummaryV1 {
    pub id: i64,
    pub created_at: String,
    pub preview: String,
    pub message_count: usize,
    pub member_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKindV1 {
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
pub struct TimelineItemV1 {
    pub id: String,
    pub kind: TimelineKindV1,
    pub turn: Option<u64>,
    pub member: Option<String>,
    pub display_name: Option<String>,
    pub backend: Option<BackendKindV1>,
    pub text: Option<String>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub to: Option<Vec<String>>,
    pub files: Option<Vec<(String, String)>>,
    pub ok: Option<bool>,
    #[serde(default)]
    pub streaming: bool,
    pub timestamp: Option<String>,
}

impl TimelineItemV1 {
    fn new(id: String, kind: TimelineKindV1) -> Self {
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
            timestamp: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DefaultTargetV1 {
    All,
    Member { member: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamMemberSettingsV1 {
    pub id: String,
    pub display_name: String,
    pub backend: BackendKindV1,
    pub role: String,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub sandbox: SandboxPolicyV1,
    pub permission_mode: Option<PermissionModeV1>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub session_policy: SessionPolicyV1,
    pub session_id: Option<String>,
    pub effort: Option<EffortV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSurfaceV1 {
    User,
    Relay,
    Mode,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalPolicyV1 {
    pub gate: Option<Vec<String>>,
    #[serde(default)]
    pub keywords: BTreeMap<String, Vec<String>>,
    pub apply_to: Option<Vec<ApprovalSurfaceV1>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewModeSettingsV1 {
    pub builder: Option<String>,
    pub reviewer: Option<String>,
    pub max_iterations: Option<u32>,
    pub auto_verify: Option<bool>,
    pub verify_command: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanModeSettingsV1 {
    pub leader: Option<String>,
    pub reviewer: Option<String>,
    pub max_iterations: Option<u32>,
    pub auto_verify: Option<bool>,
    pub verify_command: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BrainstormModeSettingsV1 {
    pub participants: Option<Vec<String>>,
    pub generation_rounds: Option<u32>,
    pub ideas_per_round: Option<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamModeSettingsV1 {
    pub coordinator: Option<String>,
    pub max_iterations: Option<u32>,
    pub auto_verify: Option<bool>,
    pub verify_command: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModesConfigV1 {
    pub review: Option<ReviewModeSettingsV1>,
    pub plan: Option<PlanModeSettingsV1>,
    pub brainstorm: Option<BrainstormModeSettingsV1>,
    pub team: Option<TeamModeSettingsV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TeamSettingsV1 {
    pub name: String,
    pub workspace: String,
    pub members: Vec<TeamMemberSettingsV1>,
    pub default_target: Option<DefaultTargetV1>,
    pub max_auto_relays: u32,
    #[serde(default)]
    pub modes: ModesConfigV1,
    #[serde(default)]
    pub approvals: ApprovalPolicyV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesktopSnapshotV1 {
    pub version: u8,
    pub sequence: u64,
    pub phase: DesktopPhase,
    pub workspace: Option<String>,
    pub team: Option<TeamSettingsV1>,
    pub mode: TerminalModeV1,
    pub members: Vec<MemberSummaryV1>,
    pub runs: Vec<RunSummaryV1>,
    pub approvals: Vec<ApprovalSummaryV1>,
    pub timeline: Vec<TimelineItemV1>,
    pub conversations: Vec<ConversationSummaryV1>,
    pub last_error: Option<String>,
}

impl Default for DesktopSnapshotV1 {
    fn default() -> Self {
        Self {
            version: DESKTOP_BRIDGE_VERSION,
            sequence: 0,
            phase: DesktopPhase::Uninitialized,
            workspace: None,
            team: None,
            mode: TerminalModeV1::Normal,
            members: Vec::new(),
            runs: Vec::new(),
            approvals: Vec::new(),
            timeline: Vec::new(),
            conversations: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DesktopEventV1 {
    pub version: u8,
    pub sequence: u64,
    pub event: DesktopRuntimeEventV1,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopRuntimeEventV1 {
    SnapshotReplaced {
        snapshot: DesktopSnapshotV1,
    },
    PhaseChanged {
        phase: DesktopPhase,
        error: Option<String>,
    },
    ModeChanged {
        mode: TerminalModeV1,
    },
    MemberUpdated {
        member: MemberSummaryV1,
    },
    TimelineAdded {
        item: TimelineItemV1,
    },
    TimelineUpdated {
        item: TimelineItemV1,
    },
    ApprovalAdded {
        approval: ApprovalSummaryV1,
    },
    ApprovalRemoved {
        id: u64,
        decision: Option<ApprovalChoiceV1>,
    },
    RunUpdated {
        run: RunSummaryV1,
    },
    ConversationsReplaced {
        conversations: Vec<ConversationSummaryV1>,
    },
    TeamSettingsUpdated {
        team: TeamSettingsV1,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChoiceV1 {
    Approve,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageTargetV1 {
    Default,
    All,
    Member { member: String },
    Members { members: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopCommandV1 {
    SetMode {
        mode: TerminalModeV1,
    },
    UserMessage {
        target: MessageTargetV1,
        body: String,
    },
    Cancel {
        member: Option<String>,
    },
    Retry,
    Approve {
        id: u64,
        decision: ApprovalChoiceV1,
    },
    SetRelayPaused {
        paused: bool,
    },
    ResolvePausedRoute {
        resume: bool,
    },
    SetEffort {
        member: String,
        effort: EffortV1,
    },
    ReplaceTeamSettings {
        settings: Box<TeamSettingsV1>,
    },
    NewSession,
    RequestResume,
    ResumeConversation {
        conversation: i64,
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
        status: RunStepStatusV1,
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
        mode: TerminalModeV1,
        task: String,
    },
    Shutdown,
}

/// State mirrored in the WebView. Runtime events are reduced here before they
/// are emitted, so `get_desktop_snapshot` is always a coherent recovery point.
#[derive(Debug, Default)]
pub struct DesktopModel {
    pub snapshot: DesktopSnapshotV1,
    item_sequence: u64,
}

impl DesktopModel {
    pub fn snapshot(&self) -> DesktopSnapshotV1 {
        self.snapshot.clone()
    }

    pub fn set_phase(
        &mut self,
        phase: DesktopPhase,
        error: Option<String>,
    ) -> DesktopRuntimeEventV1 {
        self.snapshot.phase = phase;
        self.snapshot.last_error = error.clone();
        DesktopRuntimeEventV1::PhaseChanged { phase, error }
    }

    pub fn set_workspace(&mut self, workspace: String) {
        self.snapshot.workspace = Some(workspace);
    }

    pub fn set_team(&mut self, team: TeamSettingsV1) {
        self.snapshot.team = Some(team);
    }

    pub fn seed_chat(&mut self, chat: Vec<ChatItem>) {
        self.snapshot.timeline.clear();
        for item in chat {
            let id = self.fresh_id("history");
            self.snapshot.timeline.push(timeline_from_chat(id, item));
        }
    }

    pub fn apply_runtime(&mut self, event: RuntimeEvent) -> DesktopRuntimeEventV1 {
        use RuntimeEvent::*;
        match event {
            Ready {
                team: _,
                workspace,
                default_target: _,
                members,
                runs,
            } => {
                self.snapshot.phase = DesktopPhase::Ready;
                self.snapshot.workspace = Some(workspace);
                self.snapshot.members = members.into_iter().map(member_summary).collect();
                self.snapshot.runs = runs.into_iter().map(run_summary).collect();
                self.snapshot.last_error = None;
                DesktopRuntimeEventV1::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            ModeChanged { mode } => {
                let mode = terminal_mode(mode.as_str());
                self.snapshot.mode = mode;
                DesktopRuntimeEventV1::ModeChanged { mode }
            }
            TurnStarted { turn } => DesktopRuntimeEventV1::TurnStarted { turn: turn.0 },
            TurnFinished { turn } => DesktopRuntimeEventV1::TurnFinished { turn: turn.0 },
            UserMessage {
                turn,
                targets,
                body,
            } => {
                let mut item =
                    TimelineItemV1::new(format!("turn-{}-user", turn.0), TimelineKindV1::User);
                item.turn = Some(turn.0);
                item.to = Some(targets.into_iter().map(|id| id.to_string()).collect());
                item.text = Some(body);
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
                    DesktopRuntimeEventV1::MemberUpdated {
                        member: summary.clone(),
                    }
                } else {
                    DesktopRuntimeEventV1::Notice {
                        message: format!("status received for unknown member {id}"),
                    }
                }
            }
            MessageStarted { msg, turn, member } => {
                let member_id = member.to_string();
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item = TimelineItemV1::new(format!("msg-{}", msg.0), TimelineKindV1::Agent);
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
                    DesktopRuntimeEventV1::TimelineUpdated { item: item.clone() }
                } else {
                    DesktopRuntimeEventV1::Notice {
                        message: format!("completion received for unknown message {id}"),
                    }
                }
            }
            Reasoning { member, text } => {
                let member_id = member.to_string();
                let id = self.fresh_id("reasoning");
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item = TimelineItemV1::new(id, TimelineKindV1::Reasoning);
                item.member = Some(member_id);
                item.display_name = display_name;
                item.backend = backend;
                item.text = Some(text);
                self.add_timeline(item)
            }
            ToolStarted {
                member,
                tool_id,
                name,
                summary,
            } => {
                let member_id = member.to_string();
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item =
                    TimelineItemV1::new(tool_item_id(&member_id, &tool_id), TimelineKindV1::Tool);
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
                    DesktopRuntimeEventV1::TimelineUpdated { item: item.clone() }
                } else {
                    DesktopRuntimeEventV1::Notice {
                        message: format!("completion received for unknown tool {tool_id}"),
                    }
                }
            }
            FileChange { member, files, ok } => {
                let member_id = member.to_string();
                let id = self.fresh_id("diff");
                let (display_name, backend) = self.member_identity(&member_id);
                let mut item = TimelineItemV1::new(id, TimelineKindV1::Diff);
                item.member = Some(member_id);
                item.display_name = display_name;
                item.backend = backend;
                item.files = Some(files);
                item.ok = Some(ok);
                self.add_timeline(item)
            }
            Route {
                turn,
                from,
                to,
                body,
            } => {
                let mut item = TimelineItemV1::new(self.fresh_id("route"), TimelineKindV1::Route);
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
                    TimelineItemV1::new(self.fresh_id("route-error"), TimelineKindV1::Error);
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
                    TimelineItemV1::new(self.fresh_id("route-paused"), TimelineKindV1::RoutePaused);
                item.turn = Some(turn.0);
                item.member = Some(member.clone());
                item.to = Some(to.clone());
                item.text = Some(reason.clone());
                item.detail = Some(queued.to_string());
                self.snapshot.timeline.push(item);
                DesktopRuntimeEventV1::RoutePaused {
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
                DesktopRuntimeEventV1::SessionUpdated { member, session }
            }
            ApprovalRequested {
                id,
                member,
                action,
                body,
            } => {
                let approval = ApprovalSummaryV1 {
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
                DesktopRuntimeEventV1::ApprovalAdded { approval }
            }
            ApprovalResolved { id, decision } => {
                self.snapshot.approvals.retain(|value| value.id != id.0);
                let decision = match decision.as_str() {
                    "approved" => ApprovalChoiceV1::Approve,
                    _ => ApprovalChoiceV1::Reject,
                };
                DesktopRuntimeEventV1::ApprovalRemoved {
                    id: id.0,
                    decision: Some(decision),
                }
            }
            MemberError { member, message } => {
                let mut item = TimelineItemV1::new(self.fresh_id("error"), TimelineKindV1::Error);
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
                DesktopRuntimeEventV1::RunUpdated { run }
            }
            Verdict {
                run,
                member,
                approve,
                summary,
            } => {
                let mut item =
                    TimelineItemV1::new(self.fresh_id("verdict"), TimelineKindV1::Verdict);
                item.member = Some(member.to_string());
                item.title = Some(format!("run-{}", run.0));
                item.text = Some(summary);
                item.ok = Some(approve);
                self.add_timeline(item)
            }
            Log(entry) => DesktopRuntimeEventV1::RuntimeLog {
                level: entry.level.as_str().to_string(),
                source: entry.source,
                message: entry.message,
            },
            Notice(message) => {
                let mut item = TimelineItemV1::new(self.fresh_id("notice"), TimelineKindV1::Notice);
                item.text = Some(message.clone());
                self.snapshot.timeline.push(item);
                DesktopRuntimeEventV1::Notice { message }
            }
            SessionReset => {
                self.snapshot.timeline.clear();
                self.snapshot.approvals.clear();
                self.snapshot.conversations.clear();
                self.snapshot.runs.clear();
                for member in &mut self.snapshot.members {
                    member.session = None;
                }
                DesktopRuntimeEventV1::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            ResumeChoices { conversations } => {
                let conversations: Vec<ConversationSummaryV1> = conversations
                    .into_iter()
                    .map(conversation_summary)
                    .collect();
                self.snapshot.conversations = conversations.clone();
                DesktopRuntimeEventV1::ConversationsReplaced { conversations }
            }
            ConversationResumed {
                conversation: _,
                chat,
            } => {
                self.seed_chat(chat);
                DesktopRuntimeEventV1::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            TeamSettingsUpdated { settings } => {
                match team_settings_from_domain(&settings, self.snapshot.workspace.as_deref()) {
                    Ok(team) => {
                        self.snapshot.team = Some(team.clone());
                        DesktopRuntimeEventV1::TeamSettingsUpdated { team }
                    }
                    Err(message) => DesktopRuntimeEventV1::Notice { message },
                }
            }
            RouteQueueUpdated { queued } => {
                let current = self
                    .snapshot
                    .timeline
                    .iter()
                    .filter(|item| item.kind == TimelineKindV1::RoutePaused)
                    .count();
                for _ in queued..current {
                    self.clear_next_paused_route();
                }
                DesktopRuntimeEventV1::SnapshotReplaced {
                    snapshot: self.snapshot(),
                }
            }
            AttachGranted { member } => DesktopRuntimeEventV1::Notice {
                message: format!("native session reserved for {member}"),
            },
            AttachDenied { member, reason } => DesktopRuntimeEventV1::Notice {
                message: format!("could not attach to {member}: {reason}"),
            },
        }
    }

    fn member_identity(&self, member: &str) -> (Option<String>, Option<BackendKindV1>) {
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

    fn add_timeline(&mut self, item: TimelineItemV1) -> DesktopRuntimeEventV1 {
        self.snapshot.timeline.push(item.clone());
        DesktopRuntimeEventV1::TimelineAdded { item }
    }

    fn update_text(
        &mut self,
        id: &str,
        delta: &str,
        streaming: bool,
        ok: Option<bool>,
    ) -> DesktopRuntimeEventV1 {
        if let Some(item) = self.snapshot.timeline.iter_mut().find(|item| item.id == id) {
            item.text.get_or_insert_with(String::new).push_str(delta);
            item.streaming = streaming;
            item.ok = ok;
            DesktopRuntimeEventV1::TimelineUpdated { item: item.clone() }
        } else {
            DesktopRuntimeEventV1::Notice {
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
    ) -> DesktopRuntimeEventV1 {
        if let Some(item) = self.snapshot.timeline.iter_mut().find(|item| item.id == id) {
            item.detail.get_or_insert_with(String::new).push_str(delta);
            item.streaming = streaming;
            item.ok = ok;
            DesktopRuntimeEventV1::TimelineUpdated { item: item.clone() }
        } else {
            DesktopRuntimeEventV1::Notice {
                message: format!("update received for unknown timeline item {id}"),
            }
        }
    }

    pub fn clear_next_paused_route(&mut self) {
        if let Some(index) = self
            .snapshot
            .timeline
            .iter()
            .position(|item| item.kind == TimelineKindV1::RoutePaused)
        {
            self.snapshot.timeline.remove(index);
        }
    }
}

pub fn envelope(sequence: u64, mut event: DesktopRuntimeEventV1) -> DesktopEventV1 {
    if let DesktopRuntimeEventV1::SnapshotReplaced { snapshot } = &mut event {
        snapshot.sequence = sequence;
    }
    DesktopEventV1 {
        version: DESKTOP_BRIDGE_VERSION,
        sequence,
        event,
    }
}

pub fn team_settings_from_domain<T: Serialize>(
    settings: &T,
    workspace: Option<&str>,
) -> Result<TeamSettingsV1, String> {
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
    settings: &TeamSettingsV1,
) -> Result<T, String> {
    let mut value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    normalize_default_target_to_domain(&mut value)?;
    serde_json::from_value(value).map_err(|error| format!("invalid team settings: {error}"))
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

fn timeline_from_chat(id: String, item: ChatItem) -> TimelineItemV1 {
    match item {
        ChatItem::User { body, .. } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::User);
            item.text = Some(body);
            item
        }
        ChatItem::Agent {
            member,
            display_name,
            backend,
            text,
        } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Agent);
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
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Tool);
            item.member = Some(member.to_string());
            item.title = Some(name);
            item.text = Some(summary);
            item.detail = Some(detail);
            item.ok = ok;
            item
        }
        ChatItem::Diff { member, files, .. } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Diff);
            item.member = Some(member.to_string());
            item.files = Some(files);
            item
        }
        ChatItem::Route { from, to, body } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Route);
            item.member = Some(from.to_string());
            item.to = Some(to);
            item.text = Some(body);
            item
        }
        ChatItem::Notice { text } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Notice);
            item.text = Some(text);
            item
        }
        ChatItem::Error { member, message } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Error);
            item.member = member.map(|value| value.to_string());
            item.text = Some(message);
            item
        }
        ChatItem::Verdict {
            member,
            approve,
            summary,
        } => {
            let mut item = TimelineItemV1::new(id, TimelineKindV1::Verdict);
            item.member = Some(member.to_string());
            item.ok = Some(approve);
            item.text = Some(summary);
            item
        }
    }
}

fn member_summary(value: MemberSummary) -> MemberSummaryV1 {
    MemberSummaryV1 {
        id: value.id.to_string(),
        display_name: value.display_name,
        backend: backend_kind(value.backend.as_str()),
        role: value.role,
        status: member_status(value.status.as_str()),
        session: value.session,
        cwd: value.cwd,
        model: value.model,
        effort: value.effort.map(|effort| effort_v1(effort.as_str())),
        sandbox: sandbox_policy(value.sandbox.codex_arg()),
        permission_mode: value
            .permission_mode
            .map(|mode| permission_mode(mode.claude_arg())),
        session_policy: match value.session_policy {
            asterline::domain::team::SessionPolicy::Resume => SessionPolicyV1::Resume,
            asterline::domain::team::SessionPolicy::Fresh => SessionPolicyV1::Fresh,
        },
    }
}

fn run_summary(value: RunSummary) -> RunSummaryV1 {
    RunSummaryV1 {
        id: value.id.0,
        goal: value.goal,
        status: run_status(value.status.as_str()),
        coordinator: value.coordinator.map(|id| id.to_string()),
        verification: value.verification.map(|verification| RunVerificationV1 {
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
            .map(|step| RunStepV1 {
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
            .map(|event| RunEventV1 {
                kind: event.kind,
                title: event.title,
                detail: event.detail,
                created_at: event.created_at,
                attempt: event.attempt,
            })
            .collect(),
        mode: value.mode.map(|mode| ModeRunV1 {
            mode: terminal_mode(mode.mode.as_str()),
            state: serde_json::to_string(&mode.state).unwrap_or_else(|_| "{}".to_string()),
        }),
        legacy_mode: value.legacy_mode,
    }
}

fn conversation_summary(value: ConversationSummary) -> ConversationSummaryV1 {
    ConversationSummaryV1 {
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

fn terminal_mode(value: &str) -> TerminalModeV1 {
    match value {
        "review" => TerminalModeV1::Review,
        "plan" => TerminalModeV1::Plan,
        "brainstorm" => TerminalModeV1::Brainstorm,
        "team" => TerminalModeV1::Team,
        _ => TerminalModeV1::Normal,
    }
}

fn backend_kind(value: &str) -> BackendKindV1 {
    match value {
        "claude" => BackendKindV1::Claude,
        "grok" => BackendKindV1::Grok,
        "agy" => BackendKindV1::Agy,
        _ => BackendKindV1::Codex,
    }
}

fn member_status(value: &str) -> MemberStatusV1 {
    match value {
        "queued" => MemberStatusV1::Queued,
        "running" => MemberStatusV1::Running,
        "waiting" => MemberStatusV1::Waiting,
        "needs_approval" => MemberStatusV1::NeedsApproval,
        "failed" => MemberStatusV1::Failed,
        _ => MemberStatusV1::Idle,
    }
}

fn effort_v1(value: &str) -> EffortV1 {
    match value {
        "low" => EffortV1::Low,
        "high" => EffortV1::High,
        "xhigh" => EffortV1::Xhigh,
        "max" => EffortV1::Max,
        "ultra" => EffortV1::Ultra,
        _ => EffortV1::Medium,
    }
}

fn sandbox_policy(value: &str) -> SandboxPolicyV1 {
    match value {
        "workspace-write" => SandboxPolicyV1::WorkspaceWrite,
        "danger-full-access" => SandboxPolicyV1::DangerFullAccess,
        _ => SandboxPolicyV1::ReadOnly,
    }
}

fn permission_mode(value: &str) -> PermissionModeV1 {
    match value {
        "acceptEdits" => PermissionModeV1::AcceptEdits,
        "plan" => PermissionModeV1::Plan,
        "auto" => PermissionModeV1::Auto,
        "dontAsk" => PermissionModeV1::DontAsk,
        "bypassPermissions" => PermissionModeV1::BypassPermissions,
        _ => PermissionModeV1::Default,
    }
}

fn run_status(value: &str) -> RunStatusV1 {
    match value {
        "running" => RunStatusV1::Running,
        "verifying" => RunStatusV1::Verifying,
        "done" => RunStatusV1::Done,
        "failed" => RunStatusV1::Failed,
        "blocked" => RunStatusV1::Blocked,
        _ => RunStatusV1::Planned,
    }
}

fn run_step_status(value: &str) -> RunStepStatusV1 {
    match value {
        "doing" => RunStepStatusV1::Doing,
        "done" => RunStepStatusV1::Done,
        "blocked" => RunStepStatusV1::Blocked,
        _ => RunStepStatusV1::Todo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asterline::domain::event::{MessageId, TurnId};
    use asterline::domain::team::{
        BackendKind, DefaultTarget, MemberId, TeamConfig, TeamMember, TeamSettings,
    };

    #[test]
    fn streaming_message_is_reduced_into_recoverable_snapshot() {
        let mut model = DesktopModel::default();
        model.snapshot.members.push(MemberSummaryV1 {
            id: "builder".to_string(),
            display_name: "Builder".to_string(),
            backend: BackendKindV1::Codex,
            role: "implementation".to_string(),
            status: MemberStatusV1::Idle,
            session: None,
            cwd: "/workspace".to_string(),
            model: None,
            effort: None,
            sandbox: SandboxPolicyV1::ReadOnly,
            permission_mode: None,
            session_policy: SessionPolicyV1::Resume,
        });

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
            Some(DefaultTargetV1::Member {
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
            DesktopRuntimeEventV1::SnapshotReplaced {
                snapshot: DesktopSnapshotV1::default(),
            },
        );
        assert_eq!(event.sequence, 42);
        let DesktopRuntimeEventV1::SnapshotReplaced { snapshot } = event.event else {
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
        assert_eq!(dto.members[0].sandbox, SandboxPolicyV1::ReadOnly);
        assert_eq!(dto.members[0].session_policy, SessionPolicyV1::Resume);
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
        assert!(matches!(event, DesktopRuntimeEventV1::RoutePaused { .. }));
        assert_eq!(model.snapshot.timeline[0].kind, TimelineKindV1::RoutePaused);
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
            .filter(|item| item.kind == TimelineKindV1::RoutePaused)
            .collect::<Vec<_>>();
        assert_eq!(paused.len(), 1);
        assert_eq!(paused[0].turn, Some(10));
    }

    #[test]
    fn session_reset_clears_member_sessions_from_recovery_snapshot() {
        let mut model = DesktopModel::default();
        model.snapshot.members.push(MemberSummaryV1 {
            id: "builder".to_string(),
            display_name: "Builder".to_string(),
            backend: BackendKindV1::Codex,
            role: "implementation".to_string(),
            status: MemberStatusV1::Idle,
            session: Some("old-session".to_string()),
            cwd: "/workspace".to_string(),
            model: None,
            effort: None,
            sandbox: SandboxPolicyV1::ReadOnly,
            permission_mode: None,
            session_policy: SessionPolicyV1::Resume,
        });
        model.apply_runtime(RuntimeEvent::SessionReset);
        assert_eq!(model.snapshot.members[0].session, None);
    }
}
