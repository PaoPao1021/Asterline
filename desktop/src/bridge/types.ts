export const DESKTOP_BRIDGE_VERSION = 2 as const;

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

export interface MemberSummaryV2 {
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

export interface RunVerificationV2 {
  command: string;
  ok: boolean;
  summary: string;
}

export interface RunStepV2 {
  number: number;
  status: RunStepStatus;
  owner?: string | null;
  title: string;
  note?: string | null;
  updated_at: string;
}

export interface RunEventV2 {
  kind: string;
  title: string;
  detail?: string | null;
  created_at: string;
  attempt: number;
}

/** Structured collaboration-mode state of a run (mirrors the TUI summary). */
export interface ModeStateV2 {
  phase: string;
  iteration: number;
  max_iterations: number;
  round: number;
  rounds: number;
  idea_count: number;
  vote_count: number;
}

export interface RunSummaryV2 {
  id: number;
  goal: string;
  status: RunStatus;
  coordinator?: string | null;
  verification?: RunVerificationV2 | null;
  created_at: string;
  updated_at: string;
  attempt: number;
  steps: RunStepV2[];
  events: RunEventV2[];
  mode?: { mode: TerminalMode; state: ModeStateV2 } | null;
  legacy_mode?: string | null;
}

export interface ApprovalSummaryV2 {
  id: number;
  member?: string | null;
  action: string;
  body: string;
  created_at?: string | null;
}

export interface ConversationSummaryV2 {
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

export interface TimelineItemV2 {
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
  /** Set when the host clamped this item's text or tool output. */
  truncated?: boolean;
  /** Image placeholder labels carried by a user message (`[Image #1]`). */
  attachments?: string[];
  timestamp?: string | null;
}

/** One member's not-yet-started prompt queue. */
export interface MemberQueueV2 {
  member: string;
  prompts: string[];
}

export type DefaultTargetV2 = { type: "all" } | { type: "member"; member: string };

export interface TeamMemberSettingsV2 {
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
  /** Settings fields the desktop does not model yet; preserved on save. */
  [key: string]: unknown;
}

export interface ApprovalPolicyV2 {
  gate?: string[] | null;
  keywords: Record<string, string[]>;
  apply_to?: Array<"user" | "relay" | "mode"> | null;
  [key: string]: unknown;
}

export interface ReviewModeSettingsV2 {
  builder?: string | null;
  reviewer?: string | null;
  max_iterations?: number | null;
  auto_verify?: boolean | null;
  verify_command?: string | null;
  [key: string]: unknown;
}

export interface PlanModeSettingsV2 {
  leader?: string | null;
  builder?: string | null;
  reviewer?: string | null;
  max_iterations?: number | null;
  auto_execute?: boolean | null;
  auto_verify?: boolean | null;
  verify_command?: string | null;
  [key: string]: unknown;
}

export interface BrainstormModeSettingsV2 {
  participants?: string[] | null;
  generation_rounds?: number | null;
  ideas_per_round?: number | null;
  [key: string]: unknown;
}

export interface TeamModeSettingsV2 {
  coordinator?: string | null;
  max_iterations?: number | null;
  auto_verify?: boolean | null;
  verify_command?: string | null;
  [key: string]: unknown;
}

export interface ModesConfigV2 {
  review?: ReviewModeSettingsV2 | null;
  plan?: PlanModeSettingsV2 | null;
  brainstorm?: BrainstormModeSettingsV2 | null;
  team?: TeamModeSettingsV2 | null;
}

export interface TeamSettingsV2 {
  name: string;
  workspace: string;
  members: TeamMemberSettingsV2[];
  default_target?: DefaultTargetV2 | null;
  max_auto_relays: number;
  modes: ModesConfigV2;
  approvals: ApprovalPolicyV2;
  [key: string]: unknown;
}

export interface DesktopSnapshotV2 {
  version: 2;
  sequence: number;
  phase: DesktopPhase;
  workspace?: string | null;
  team?: TeamSettingsV2 | null;
  mode: TerminalMode;
  mode_overrides: ModesConfigV2;
  queues: MemberQueueV2[];
  relay_paused: boolean;
  suggested_verify?: string | null;
  active_conversation?: number | null;
  timeline_truncated?: boolean;
  members: MemberSummaryV2[];
  runs: RunSummaryV2[];
  approvals: ApprovalSummaryV2[];
  timeline: TimelineItemV2[];
  conversations: ConversationSummaryV2[];
  last_error?: string | null;
}

export interface LogEntryV2 {
  level: LogLevel;
  source: string;
  message: string;
}

export interface DiffResultV2 {
  text: string;
  truncated: boolean;
  file_count: number;
}

export interface SkillSummaryV2 {
  name: string;
  description: string;
  backend: BackendKind;
  invocation: string;
}

export interface DesktopEventV2 {
  version: 2;
  sequence: number;
  event: DesktopRuntimeEventV2;
}

export type DesktopRuntimeEventV2 =
  | { type: "snapshot_replaced"; snapshot: DesktopSnapshotV2 }
  | { type: "phase_changed"; phase: DesktopPhase; error?: string | null }
  | { type: "mode_changed"; mode: TerminalMode }
  | { type: "modes_updated"; defaults: ModesConfigV2; overrides: ModesConfigV2 }
  | { type: "member_updated"; member: MemberSummaryV2 }
  | { type: "members_replaced"; members: MemberSummaryV2[] }
  | { type: "timeline_added"; item: TimelineItemV2 }
  | { type: "timeline_updated"; item: TimelineItemV2 }
  | { type: "timeline_cleared" }
  | { type: "approval_added"; approval: ApprovalSummaryV2 }
  | { type: "approval_removed"; id: number; decision?: "approve" | "reject" }
  | { type: "run_updated"; run: RunSummaryV2 }
  | { type: "conversations_replaced"; conversations: ConversationSummaryV2[] }
  | { type: "team_settings_updated"; team: TeamSettingsV2 }
  | { type: "notice"; message: string }
  | { type: "turn_started"; turn: number }
  | { type: "turn_finished"; turn: number }
  | { type: "queue_updated"; member: string; prompts: string[] }
  | { type: "queued_prompt_returned"; member: string; body: string }
  | { type: "route_paused"; turn: number; member: string; to: string[]; reason: string; queued: number }
  | { type: "session_updated"; member: string; session: string }
  | { type: "runtime_log"; level: string; source: string; message: string }
  | { type: "logs_replaced"; request_id: number; entries: LogEntryV2[]; truncated: boolean }
  | { type: "diff_replaced"; request_id: number; result: DiffResultV2 }
  | { type: "skills_replaced"; request_id: number; skills: SkillSummaryV2[]; truncated: boolean };

export type MessageTargetV2 =
  | { type: "default" }
  | { type: "all" }
  | { type: "member"; member: string }
  | { type: "members"; members: string[] };

export type DesktopCommandV2 =
  | { type: "request_logs"; request_id: number; member?: string | null; level?: LogLevel | null; query?: string | null; limit?: number | null }
  | { type: "request_diff"; request_id: number }
  | { type: "request_skills"; request_id: number; backend?: BackendKind | null; query?: string | null; limit?: number | null }
  | { type: "set_mode"; mode: TerminalMode }
  | { type: "set_mode_overrides"; overrides: ModesConfigV2 }
  | { type: "save_mode_defaults"; mode: Exclude<TerminalMode, "normal"> }
  | { type: "reset_mode_overrides"; mode: Exclude<TerminalMode, "normal"> }
  | { type: "user_message"; target: MessageTargetV2; body: string; attachments?: string[] }
  | { type: "cancel"; member?: string | null }
  | { type: "edit_queued_prompt"; member?: string | null }
  | { type: "retry" }
  | { type: "approve"; id: number; decision: "approve" | "reject" }
  | { type: "set_relay_paused"; paused: boolean }
  | { type: "resolve_paused_route"; resume: boolean }
  | { type: "replace_team_settings"; settings: TeamSettingsV2 }
  | { type: "request_team_settings" }
  | { type: "new_session" }
  | { type: "request_resume" }
  | { type: "resume_conversation"; conversation: number }
  | { type: "import_session"; member?: string | null; session_id: string }
  | { type: "export_session"; format?: string | null }
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

// --- composer contract (parsed by the shared Rust parser) -------------------

export type ComposerSurface = "team" | "runs" | "logs" | "diff" | "mode" | "member_logs";

export type ComposerAction =
  | { type: "empty" }
  | { type: "exit" }
  | { type: "attach"; member: string }
  | { type: "command"; command: DesktopCommandV2 }
  | { type: "surface"; surface: ComposerSurface; member?: string | null }
  | { type: "approve_first"; decision: "approve" | "reject" }
  | { type: "find"; query: string }
  | { type: "help" }
  | { type: "invalid"; message: string }
  | { type: "needs_target" };

export interface CommandSpec {
  name: string;
  hint: string;
  takes_argument: boolean;
}

export interface CompletionItem {
  label: string;
  insert: string;
}

export interface Completion {
  title: string;
  token_start: number;
  items: CompletionItem[];
}

// --- read-only catalog queries ----------------------------------------------

export interface BackendAvailability {
  codex: boolean;
  claude: boolean;
  grok: boolean;
  agy: boolean;
}

export interface ModelSummary {
  id: string;
  name: string;
  description?: string | null;
  default_effort?: string | null;
  supported_efforts: string[];
  is_default: boolean;
}

export interface NativeSessionSummary {
  backend: BackendKind;
  session_id: string;
  preview: string;
  modified_secs: number;
}

export interface StagedAttachment {
  token: string;
  label: string;
  mime: string;
}

// --- launch & host queries ---------------------------------------------------

export interface DesktopLaunchOptions {
  team_path?: string | null;
  pick_team?: boolean;
  db_path?: string | null;
  restore?: boolean | null;
  approvals?: boolean | null;
  debug?: boolean;
  fake?: boolean;
  auto_update?: boolean | null;
  /** Frontend-only: user accepted the approval-gate risk confirmation. */
  riskAck?: boolean;
}

export interface RecentWorkspace {
  workspace: string;
  name?: string | null;
  last_opened_at?: string | null;
}

export interface DesktopUpdate {
  current_version: string;
  available_version?: string | null;
  release_url?: string | null;
  update_available: boolean;
  error?: string | null;
}

export interface DiagnosticsStatus {
  previous_unclean_exit: boolean;
  log_path: string;
}

export interface AttachCapabilities {
  supported: boolean;
  terminal?: string | null;
  backends: Partial<Record<BackendKind, { can_open: boolean; can_import: boolean }>>;
}

export interface ExternalAttachLaunch {
  member: string;
  launched: boolean;
  terminal?: string | null;
  message?: string | null;
}

export type Unlisten = () => void;

export interface DesktopClient {
  readonly kind: "tauri" | "mock";
  listenRuntimeEvents(listener: (event: DesktopEventV2) => void): Promise<Unlisten>;
  bootstrapDesktop(workspace?: string, options?: DesktopLaunchOptions): Promise<DesktopSnapshotV2>;
  getDesktopSnapshot(): Promise<DesktopSnapshotV2>;
  dispatchDesktopCommand(command: DesktopCommandV2): Promise<void>;
  shutdownDesktop(): Promise<void>;
  exitDesktop(): Promise<void>;
  parseComposerText(text: string): Promise<ComposerAction>;
  completeComposer(head: string): Promise<Completion | null>;
  commandCatalog(): Promise<CommandSpec[]>;
  getBackendAvailability(): Promise<BackendAvailability>;
  listModels(backend: BackendKind, cwd: string): Promise<ModelSummary[]>;
  listNativeSessions(backend: BackendKind, cwd: string): Promise<NativeSessionSummary[]>;
  stageClipboardImage(): Promise<StagedAttachment>;
  stageImagePath(path: string): Promise<StagedAttachment>;
  stageImageBytes(base64: string): Promise<StagedAttachment>;
  removeStagedAttachment(token: string): Promise<void>;
  discardStagedAttachments(): Promise<void>;
  listRecentWorkspaces(): Promise<RecentWorkspace[]>;
  forgetRecentWorkspace(workspace: string): Promise<void>;
  checkDesktopUpdate(): Promise<DesktopUpdate>;
  openDesktopUpdate(url: string): Promise<void>;
  getDesktopDiagnosticsStatus(): Promise<DiagnosticsStatus>;
  exportDesktopDiagnostics(): Promise<string>;
  getAttachCapabilities(): Promise<AttachCapabilities>;
  openNativeSession(member: string): Promise<ExternalAttachLaunch>;
}
