export const DESKTOP_BRIDGE_VERSION = 1 as const;

export type DesktopPhase =
  | "uninitialized"
  | "starting"
  | "needs_team_setup"
  | "ready"
  | "locked"
  | "error"
  | "shutting_down";
export type TerminalMode = "normal" | "review" | "plan" | "brainstorm" | "team";
export type BackendKind = "codex" | "claude" | "grok" | "agy";
export type MemberStatus = "idle" | "queued" | "running" | "waiting" | "needs_approval" | "failed";
export type Effort = "low" | "medium" | "high" | "xhigh" | "max" | "ultra";
export type SandboxPolicy = "read-only" | "workspace-write" | "danger-full-access";
export type SessionPolicy = "resume" | "fresh";
export type PermissionMode = "default" | "acceptEdits" | "plan" | "auto" | "dontAsk" | "bypassPermissions";
export type LogLevel = "debug" | "info" | "warn" | "error";
export type RunStatus = "planned" | "running" | "verifying" | "done" | "failed" | "blocked";
export type RunStepStatus = "todo" | "doing" | "done" | "blocked";

export interface MemberSummaryV1 {
  id: string;
  display_name: string;
  backend: BackendKind;
  role: string;
  status: MemberStatus;
  session?: string | null;
  cwd: string;
  model?: string | null;
  effort?: Effort | null;
  sandbox: SandboxPolicy;
  permission_mode?: PermissionMode | null;
  session_policy: SessionPolicy;
}

export interface RunVerificationV1 {
  command: string;
  ok: boolean;
  summary: string;
}

export interface RunStepV1 {
  number: number;
  status: RunStepStatus;
  owner?: string | null;
  title: string;
  note?: string | null;
  updated_at: string;
}

export interface RunEventV1 {
  kind: string;
  title: string;
  detail?: string | null;
  created_at: string;
  attempt: number;
}

export interface RunSummaryV1 {
  id: number;
  goal: string;
  status: RunStatus;
  coordinator?: string | null;
  verification?: RunVerificationV1 | null;
  created_at: string;
  updated_at: string;
  attempt: number;
  steps: RunStepV1[];
  events: RunEventV1[];
  mode?: { mode: TerminalMode; state: string } | null;
  legacy_mode?: string | null;
}

export interface ApprovalSummaryV1 {
  id: number;
  member?: string | null;
  action: string;
  body: string;
  created_at?: string | null;
}

export interface ConversationSummaryV1 {
  id: number;
  created_at: string;
  preview: string;
  message_count: number;
  member_count: number;
}

export type TimelineKind =
  | "user"
  | "agent"
  | "reasoning"
  | "tool"
  | "diff"
  | "route"
  | "route_paused"
  | "notice"
  | "error"
  | "verdict";

export interface TimelineItemV1 {
  id: string;
  kind: TimelineKind;
  turn?: number | null;
  member?: string | null;
  display_name?: string | null;
  backend?: BackendKind | null;
  text?: string | null;
  title?: string | null;
  detail?: string | null;
  to?: string[] | null;
  files?: Array<[string, string]> | null;
  ok?: boolean | null;
  streaming?: boolean;
  timestamp?: string | null;
}

export type DefaultTargetV1 = { type: "all" } | { type: "member"; member: string };

export interface TeamMemberSettingsV1 {
  id: string;
  display_name: string;
  backend: BackendKind;
  role: string;
  cwd?: string | null;
  model?: string | null;
  system_prompt?: string | null;
  sandbox: SandboxPolicy;
  permission_mode?: PermissionMode | null;
  allowed_tools: string[];
  session_policy: SessionPolicy;
  session_id?: string | null;
  effort?: Effort | null;
}

export interface ApprovalPolicyV1 {
  gate?: string[] | null;
  keywords: Record<string, string[]>;
  apply_to?: Array<"user" | "relay" | "mode"> | null;
}

export interface ModesConfigV1 {
  review?: {
    builder?: string | null;
    reviewer?: string | null;
    max_iterations?: number | null;
    auto_verify?: boolean | null;
    verify_command?: string | null;
  } | null;
  plan?: {
    leader?: string | null;
    reviewer?: string | null;
    max_iterations?: number | null;
    auto_verify?: boolean | null;
    verify_command?: string | null;
  } | null;
  brainstorm?: {
    participants?: string[] | null;
    generation_rounds?: number | null;
    ideas_per_round?: number | null;
  } | null;
  team?: {
    coordinator?: string | null;
    max_iterations?: number | null;
    auto_verify?: boolean | null;
    verify_command?: string | null;
  } | null;
}

export interface TeamSettingsV1 {
  name: string;
  workspace: string;
  members: TeamMemberSettingsV1[];
  default_target?: DefaultTargetV1 | null;
  max_auto_relays: number;
  modes: ModesConfigV1;
  approvals: ApprovalPolicyV1;
}

export interface DesktopSnapshotV1 {
  version: 1;
  sequence: number;
  phase: DesktopPhase;
  workspace?: string | null;
  team?: TeamSettingsV1 | null;
  mode: TerminalMode;
  members: MemberSummaryV1[];
  runs: RunSummaryV1[];
  approvals: ApprovalSummaryV1[];
  timeline: TimelineItemV1[];
  conversations: ConversationSummaryV1[];
  last_error?: string | null;
}

export interface LogEntryV1 {
  level: LogLevel;
  source: string;
  message: string;
}

export interface DiffResultV1 {
  text: string;
  truncated: boolean;
  file_count: number;
}

export interface SkillSummaryV1 {
  name: string;
  description: string;
  backend: BackendKind;
  invocation: string;
}

export interface DesktopEventV1 {
  version: 1;
  sequence: number;
  event: DesktopRuntimeEventV1;
}

export type DesktopRuntimeEventV1 =
  | { type: "snapshot_replaced"; snapshot: DesktopSnapshotV1 }
  | { type: "phase_changed"; phase: DesktopPhase; error?: string | null }
  | { type: "mode_changed"; mode: TerminalMode }
  | { type: "member_updated"; member: MemberSummaryV1 }
  | { type: "members_replaced"; members: MemberSummaryV1[] }
  | { type: "timeline_added"; item: TimelineItemV1 }
  | { type: "timeline_updated"; item: TimelineItemV1 }
  | { type: "timeline_cleared" }
  | { type: "approval_added"; approval: ApprovalSummaryV1 }
  | { type: "approval_removed"; id: number; decision?: "approve" | "reject" }
  | { type: "run_updated"; run: RunSummaryV1 }
  | { type: "conversations_replaced"; conversations: ConversationSummaryV1[] }
  | { type: "team_settings_updated"; team: TeamSettingsV1 }
  | { type: "notice"; message: string }
  | { type: "turn_started"; turn: number }
  | { type: "turn_finished"; turn: number }
  | { type: "route_paused"; turn: number; member: string; to: string[]; reason: string; queued: number }
  | { type: "session_updated"; member: string; session: string }
  | { type: "runtime_log"; level: string; source: string; message: string }
  | { type: "logs_replaced"; request_id: number; entries: LogEntryV1[]; truncated: boolean }
  | { type: "diff_replaced"; request_id: number; result: DiffResultV1 }
  | { type: "skills_replaced"; request_id: number; skills: SkillSummaryV1[]; truncated: boolean };

export type MessageTargetV1 =
  | { type: "default" }
  | { type: "all" }
  | { type: "member"; member: string }
  | { type: "members"; members: string[] };

export type DesktopCommandV1 =
  | { type: "request_logs"; request_id: number; member?: string | null; level?: LogLevel | null; query?: string | null; limit?: number | null }
  | { type: "request_diff"; request_id: number }
  | { type: "request_skills"; request_id: number; backend?: BackendKind | null; query?: string | null; limit?: number | null }
  | { type: "set_mode"; mode: TerminalMode }
  | { type: "user_message"; target: MessageTargetV1; body: string }
  | { type: "cancel"; member?: string | null }
  | { type: "retry" }
  | { type: "approve"; id: number; decision: "approve" | "reject" }
  | { type: "set_relay_paused"; paused: boolean }
  | { type: "resolve_paused_route"; resume: boolean }
  | { type: "set_effort"; member: string; effort: Effort }
  | { type: "replace_team_settings"; settings: TeamSettingsV1 }
  | { type: "new_session" }
  | { type: "request_resume" }
  | { type: "resume_conversation"; conversation: number }
  | { type: "continue_run"; run_id?: number | null; note?: string | null }
  | { type: "note_run"; run_id?: number | null; note: string }
  | { type: "block_run"; run_id?: number | null; reason: string }
  | { type: "verify_run"; run_id?: number | null; command?: string | null }
  | { type: "add_run_step"; run_id?: number | null; owner?: string | null; title: string }
  | { type: "update_run_step"; run_id?: number | null; step: number; status: RunStepStatus; note?: string | null }
  | { type: "rename_run_step"; run_id?: number | null; step: number; title: string }
  | { type: "remove_run_step"; run_id?: number | null; step: number }
  | { type: "assign_run_step"; run_id?: number | null; step: number; owner?: string | null }
  | { type: "run_mode"; mode: Exclude<TerminalMode, "normal">; task: string }
  | { type: "shutdown" };

export interface RecentWorkspaceV1 {
  workspace: string;
  name?: string | null;
  last_opened_at?: string | null;
}

export interface DesktopUpdateV1 {
  current_version: string;
  available_version?: string | null;
  release_url?: string | null;
  update_available: boolean;
  error?: string | null;
}

export interface DiagnosticsStatusV1 {
  previous_unclean_exit: boolean;
  log_path: string;
}

export interface AttachCapabilitiesV1 {
  supported: boolean;
  terminal?: string | null;
  backends: Partial<Record<BackendKind, { can_open: boolean; can_import: boolean }>>;
}

export interface ExternalAttachLaunchV1 {
  member: string;
  launched: boolean;
  terminal?: string | null;
  message?: string | null;
}

export type Unlisten = () => void;

export interface DesktopClient {
  readonly kind: "tauri" | "mock";
  listenRuntimeEvents(listener: (event: DesktopEventV1) => void): Promise<Unlisten>;
  bootstrapDesktop(workspace?: string): Promise<DesktopSnapshotV1>;
  getDesktopSnapshot(): Promise<DesktopSnapshotV1>;
  dispatchDesktopCommand(command: DesktopCommandV1): Promise<void>;
  shutdownDesktop(): Promise<void>;
  listRecentWorkspaces(): Promise<RecentWorkspaceV1[]>;
  forgetRecentWorkspace(workspace: string): Promise<void>;
  checkDesktopUpdate(): Promise<DesktopUpdateV1>;
  openDesktopUpdate(url: string): Promise<void>;
  getDesktopDiagnosticsStatus(): Promise<DiagnosticsStatusV1>;
  exportDesktopDiagnostics(): Promise<string>;
  getAttachCapabilities(): Promise<AttachCapabilitiesV1>;
  openNativeSession(member: string): Promise<ExternalAttachLaunchV1>;
}
