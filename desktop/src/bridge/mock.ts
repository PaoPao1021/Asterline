import type {
  BackendAvailability,
  BackendKind,
  CommandSpec,
  ComposerAction,
  Completion,
  DesktopClient,
  DesktopCommandV2,
  DesktopEventV2,
  DesktopRuntimeEventV2,
  DesktopSnapshotV2,
  MemberQueueV2,
  ModelSummary,
  NativeSessionSummary,
  RecentWorkspace,
  RunSummaryV2,
  StagedAttachment,
  TeamSettingsV2,
  TimelineItemV2,
  Unlisten,
} from "./types";

const now = () => new Date().toISOString();

const demoTeam: TeamSettingsV2 = {
  name: "Asterline Core",
  workspace: "D:\\workspace\\asterline",
  default_target: { type: "member", member: "builder" },
  max_auto_relays: 6,
  members: [
    {
      id: "builder",
      display_name: "Builder",
      backend: "codex",
      role: "Implementation lead",
      cwd: null,
      model: "gpt-5.3-codex",
      system_prompt: "Build changes in small, verifiable steps.",
      sandbox: "workspace-write",
      permission_mode: null,
      allowed_tools: ["shell", "apply_patch"],
      session_policy: "resume",
      session_id: "019-demo-builder",
      effort: "high",
    },
    {
      id: "reviewer",
      display_name: "Reviewer",
      backend: "claude",
      role: "Architecture and quality review",
      cwd: null,
      model: "claude-sonnet-4-5",
      system_prompt: "Review correctness, regressions, and maintainability.",
      sandbox: "read-only",
      permission_mode: "plan",
      allowed_tools: [],
      session_policy: "resume",
      session_id: "demo-reviewer-thread",
      effort: "high",
    },
    {
      id: "researcher",
      display_name: "Researcher",
      backend: "grok",
      role: "Research and alternatives",
      cwd: null,
      model: null,
      system_prompt: null,
      sandbox: "read-only",
      permission_mode: "default",
      allowed_tools: [],
      session_policy: "fresh",
      session_id: null,
      effort: "medium",
    },
  ],
  approvals: {
    manual: true,
    gate: ["git", "shell", "file"],
    keywords: { deploy: ["publish", "release", "deploy"] },
    apply_to: ["user", "relay", "mode"],
  },
  modes: {
    review: { builder: "builder", reviewer: "reviewer", max_iterations: 3, reviewer_hint: "Check the desktop flow and accessibility before approving." },
    plan: { leader: "builder", builder: "researcher", reviewer: "reviewer", max_iterations: 2, auto_execute: false },
    brainstorm: { participants: ["builder", "reviewer", "researcher"], generation_rounds: 3, ideas_per_round: 4 },
    team: { coordinator: "builder", max_iterations: 4, allow_add_members: false },
  },
};

const demoRun: RunSummaryV2 = {
  id: 18,
  number: 1,
  goal: "Ship the desktop event bridge",
  status: "running",
  coordinator: "builder",
  created_at: "2026-08-12T08:21:00Z",
  updated_at: "2026-08-12T08:34:00Z",
  attempt: 1,
  verification: null,
  legacy_mode: null,
  mode: {
    mode: "team",
    state: { phase: "implementing", iteration: 1, max_iterations: 4, round: 0, rounds: 0, idea_count: 0, vote_count: 0 },
  },
  steps: [
    { number: 1, status: "done", owner: "builder", title: "Define versioned DTOs", updated_at: "2026-08-12T08:28:00Z" },
    { number: 2, status: "doing", owner: "builder", title: "Connect runtime events", note: "Snapshot ordering is covered", updated_at: "2026-08-12T08:34:00Z" },
    { number: 3, status: "todo", owner: "reviewer", title: "Review failure recovery", updated_at: "2026-08-12T08:21:00Z" },
  ],
  events: [
    { kind: "started", title: "Team run started", created_at: "2026-08-12T08:21:00Z", attempt: 1 },
    { kind: "step", title: "DTO contract completed", detail: "Builder completed step 1", created_at: "2026-08-12T08:28:00Z", attempt: 1 },
  ],
};

export const demoSnapshot: DesktopSnapshotV2 = {
  version: 2,
  sequence: 12,
  phase: "ready",
  workspace: demoTeam.workspace,
  team: demoTeam,
  mode: "team",
  mode_overrides: {},
  queues: [],
  relay_paused: false,
  suggested_verify: null,
  active_conversation: null,
  timeline_truncated: false,
  members: demoTeam.members.map((member, index) => ({
    id: member.id,
    display_name: member.display_name,
    backend: member.backend,
    role: member.role,
    status: index === 0 ? "running" : index === 1 ? "waiting" : "idle",
    session: member.session_id,
    cwd: member.cwd ?? demoTeam.workspace,
    model: member.model,
    effort: member.effort,
    sandbox: member.sandbox,
    permission_mode: member.permission_mode,
    session_policy: member.session_policy,
  })),
  runs: [demoRun],
  approvals: [
    {
      id: 7,
      member: "builder",
      action: "shell",
      body: "Run `cargo test --all-targets --locked` in the workspace",
      created_at: "2026-08-12T08:35:00Z",
    },
  ],
  conversations: [
    { id: 41, created_at: "2026-08-12T08:21:00Z", preview: "Build the desktop event bridge and verify…", message_count: 14, member_count: 3 },
    { id: 38, created_at: "2026-08-11T12:08:00Z", preview: "Audit persistence and recovery semantics…", message_count: 22, member_count: 2 },
    { id: 31, created_at: "2026-08-09T07:42:00Z", preview: "Brainstorm packaging strategies for v1…", message_count: 18, member_count: 3 },
  ],
  timeline: [
    { id: "user-1", kind: "user", turn: 9, text: "Build the versioned desktop bridge and keep the CLI behavior unchanged.", timestamp: "2026-08-12T08:21:00Z" },
    { id: "route-1", kind: "route", turn: 9, member: "builder", display_name: "Builder", to: ["reviewer"], text: "Please audit the event ordering and failure paths while I implement the bridge.", timestamp: "2026-08-12T08:23:00Z" },
    { id: "agent-1", kind: "agent", turn: 9, member: "builder", display_name: "Builder", backend: "codex", text: "I’ve separated bootstrap from the TUI and added a versioned snapshot boundary. The frontend can subscribe before requesting state, then discard events at or below the snapshot sequence.", timestamp: "2026-08-12T08:30:00Z" },
    { id: "tool-1", kind: "tool", turn: 9, member: "builder", display_name: "Builder", title: "cargo test --lib", detail: "638 passed; 1 platform-specific path assertion needs review", ok: true, timestamp: "2026-08-12T08:32:00Z" },
    { id: "agent-2", kind: "agent", turn: 9, member: "reviewer", display_name: "Reviewer", backend: "claude", text: "The boundary looks sound. Preserve unknown settings fields on every edit and make shutdown idempotent before packaging.", timestamp: "2026-08-12T08:34:00Z" },
    { id: "notice-1", kind: "notice", text: "Automatic relay paused while an approval is pending.", timestamp: "2026-08-12T08:35:00Z" },
  ],
  last_error: null,
};

/** Shared-command mirror for the browser demo (the real host parses in Rust). */
const demoCatalog: CommandSpec[] = [
  { name: "ask", hint: "send to one member", takes_argument: true },
  { name: "all", hint: "send to everyone", takes_argument: true },
  { name: "attach", hint: "open a member's native CLI session", takes_argument: true },
  { name: "approve", hint: "approve first pending", takes_argument: false },
  { name: "block", hint: "mark a run blocked", takes_argument: true },
  { name: "continue", hint: "resume latest or selected run", takes_argument: true },
  { name: "diff", hint: "show working-tree git diff", takes_argument: false },
  { name: "exit", hint: "exit Asterline", takes_argument: false },
  { name: "export", hint: "export session to Claude Code format", takes_argument: true },
  { name: "find", hint: "search the transcript", takes_argument: true },
  { name: "focus", hint: "view a member's logs", takes_argument: true },
  { name: "help", hint: "show commands", takes_argument: false },
  { name: "import", hint: "import native session transcript into chat", takes_argument: true },
  { name: "logs", hint: "raw logs · stderr · warnings", takes_argument: false },
  { name: "mode", hint: "open mode panel or switch dispatch mode", takes_argument: true },
  { name: "new", hint: "start a fresh chat (new session, cleared transcript)", takes_argument: false },
  { name: "note", hint: "record a run checkpoint", takes_argument: true },
  { name: "reject", hint: "reject first pending", takes_argument: false },
  { name: "resume", hint: "choose and restore a saved chat", takes_argument: false },
  { name: "retry", hint: "re-send the latest user request", takes_argument: false },
  { name: "runs", hint: "run status · next action", takes_argument: false },
  { name: "step", hint: "manage run checklist", takes_argument: true },
  { name: "team", hint: "edit roster · sessions · approvals", takes_argument: false },
];

const demoModels: ModelSummary[] = [
  { id: "gpt-5.3-codex", name: "gpt-5.3-codex", description: "Latest codex model", default_effort: "high", supported_efforts: ["low", "medium", "high"], is_default: true },
  { id: "gpt-5.1-codex-mini", name: "gpt-5.1-codex-mini", description: "Fast small model", default_effort: "medium", supported_efforts: ["low", "medium"], is_default: false },
  { id: "claude-sonnet-4-5", name: "claude-sonnet-4-5", description: "Balanced Claude model", default_effort: null, supported_efforts: [], is_default: true },
  { id: "grok-4-fast", name: "grok-4-fast", description: "Fast Grok model", default_effort: null, supported_efforts: [], is_default: true },
];

function clone<T>(value: T): T {
  return structuredClone(value);
}

class MockDesktopClient implements DesktopClient {
  readonly kind = "mock" as const;
  private snapshot = clone(demoSnapshot);
  private listeners = new Set<(event: DesktopEventV2) => void>();
  private sequence = this.snapshot.sequence;
  private messageCounter = 100;
  private stagedAttachments = new Map<string, StagedAttachment>();
  private activeStream = false;

  async listenRuntimeEvents(listener: (event: DesktopEventV2) => void): Promise<Unlisten> {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private emit(event: DesktopRuntimeEventV2): void {
    const packet: DesktopEventV2 = { version: 2, sequence: ++this.sequence, event };
    this.snapshot.sequence = packet.sequence;
    this.listeners.forEach((listener) => listener(clone(packet)));
  }

  async bootstrapDesktop(workspace?: string): Promise<DesktopSnapshotV2> {
    if (workspace) {
      this.snapshot.workspace = workspace;
      if (this.snapshot.team) {
        this.snapshot.team.workspace = workspace;
      }
      this.snapshot.phase = "ready";
    }
    return clone(this.snapshot);
  }

  async getDesktopSnapshot(): Promise<DesktopSnapshotV2> {
    return clone(this.snapshot);
  }

  async parseComposerText(text: string): Promise<ComposerAction> {
    const trimmed = text.trim();
    if (!trimmed) return { type: "empty" };
    if (trimmed.startsWith("/")) {
      const [rawCommand, ...rest] = trimmed.slice(1).split(/\s+/);
      const command = rawCommand.toLowerCase();
      const argument = rest.join(" ");
      switch (command) {
        case "ask": {
          const [member, ...body] = argument.split(/\s+/);
          if (!member || body.length === 0) return { type: "help" };
          return this.userMessageAction(member, body.join(" "));
        }
        case "all":
          if (!argument) return { type: "help" };
          return this.userMessageAction("all", argument);
        case "attach": {
          if (!argument) return { type: "help" };
          if (argument.split(/\s+/).length > 1) return { type: "invalid", message: "/attach does not accept arguments; draft kept" };
          return { type: "attach", member: argument };
        }
        case "approve": return argument ? invalidNoArgs("approve") : { type: "approve_first", decision: "approve" };
        case "reject": return argument ? invalidNoArgs("reject") : { type: "approve_first", decision: "reject" };
        case "diff": return argument ? invalidNoArgs("diff") : { type: "surface", surface: "diff" };
        case "exit": return argument ? invalidNoArgs("exit") : { type: "exit" };
        case "export":
          return { type: "command", command: { type: "export_session", format: argument || null } };
        case "find": return { type: "find", query: argument };
        case "focus": {
          const [member, extra] = argument.split(/\s+/);
          if (!member) return { type: "help" };
          if (extra) return { type: "invalid", message: "/focus accepts exactly one member; trailing arguments were not used; draft kept" };
          return { type: "surface", surface: "member_logs", member };
        }
        case "help": return argument ? invalidNoArgs("help") : { type: "help" };
        case "import": {
          const [first, second] = argument.split(/\s+/);
          if (!first) return { type: "help" };
          return {
            type: "command",
            command: { type: "import_session", member: second ? first : null, session_id: second ?? first },
          };
        }
        case "logs": return argument ? invalidNoArgs("logs") : { type: "surface", surface: "logs" };
        case "mode":
          if (!argument) return { type: "surface", surface: "mode" };
          if (["normal", "review", "plan", "brainstorm", "team"].includes(argument)) {
            return { type: "command", command: { type: "set_mode", mode: argument as "normal" | "review" | "plan" | "brainstorm" | "team" } };
          }
          return { type: "help" };
        case "new":
        case "clear": return argument ? invalidNoArgs("new") : { type: "command", command: { type: "new_session" } };
        case "note": {
          if (!argument) return { type: "help" };
          const match = argument.match(/^run-(\d+)\s+(.*)$/);
          return {
            type: "command",
            command: { type: "note_run", run_id: match ? Number(match[1]) : null, note: match ? match[2] : argument },
          };
        }
        case "resume": return argument ? invalidNoArgs("resume") : { type: "command", command: { type: "request_resume" } };
        case "retry": return argument ? invalidNoArgs("retry") : { type: "command", command: { type: "retry" } };
        case "runs": return argument ? invalidNoArgs("runs") : { type: "surface", surface: "runs" };
        case "team": return argument ? invalidNoArgs("team") : { type: "surface", surface: "team" };
        default: return { type: "help" };
      }
    }
    if (trimmed.startsWith("@")) {
      const [member, ...body] = trimmed.slice(1).split(/\s+/);
      if (!member || body.length === 0) return { type: "empty" };
      if (body[0]?.startsWith("/")) {
        return {
          type: "invalid",
          message: `${body.join(" ")} is not a discovered prompt-invocable skill for ${member}; use /attach <member> for that backend's native CLI`,
        };
      }
      return this.userMessageAction(member, trimmed);
    }
    return { type: "needs_target" };
  }

  private userMessageAction(member: string, body: string): ComposerAction {
    return {
      type: "command",
      command: {
        type: "user_message",
        target: member === "all" ? { type: "all" } : { type: "member", member },
        body: member === "all" ? `@all ${body}` : body.startsWith("@") ? body : `@${member} ${body}`,
        attachments: [],
      },
    };
  }

  async completeComposer(head: string): Promise<Completion | null> {
    if (head.startsWith("/") && !head.includes(" ")) {
      const lower = head.slice(1).toLowerCase();
      const items = demoCatalog
        .filter((spec) => spec.name.startsWith(lower) || (spec.name === "new" && "clear".startsWith(lower)))
        .map((spec) => ({
          label: `/${spec.name} — ${spec.hint}`,
          insert: spec.name === "mode" ? `/${spec.name}` : `/${spec.name} `,
        }));
      if (items.length === 0) return null;
      return { title: "commands", token_start: 0, items };
    }
    if (head.startsWith("@") && !head.includes(" ")) {
      const lower = head.slice(1).toLowerCase();
      const candidates = [
        { id: "all", label: "All members" },
        ...this.snapshot.members.map((member) => ({ id: member.id, label: member.display_name })),
      ];
      const items = candidates
        .filter((candidate) => candidate.id.toLowerCase().startsWith(lower))
        .map((candidate) => ({ label: `@${candidate.id}`, insert: `@${candidate.id} ` }));
      if (items.length === 0) return null;
      return { title: "mention a member", token_start: 0, items };
    }
    return null;
  }

  async commandCatalog(): Promise<CommandSpec[]> {
    return clone(demoCatalog);
  }

  async getBackendAvailability(): Promise<BackendAvailability> {
    return { codex: true, claude: true, grok: true, agy: false };
  }

  async listModels(backend: BackendKind): Promise<ModelSummary[]> {
    return demoModels.filter((model) =>
      backend === "codex" ? model.id.includes("codex")
      : backend === "claude" ? model.id.startsWith("claude")
      : backend === "grok" ? model.id.startsWith("grok")
      : false,
    );
  }

  async listNativeSessions(backend: BackendKind): Promise<NativeSessionSummary[]> {
    const sessions: NativeSessionSummary[] = [
      { backend: "claude", session_id: "demo-session-abc", preview: "Refactor the bridge parser", modified_secs: 1767000000 },
      { backend: "codex", session_id: "019-demo-rollout", preview: "Add regression tests for the reducer", modified_secs: 1766900000 },
      { backend: "grok", session_id: "grok-demo-thread", preview: "Research packaging strategies", modified_secs: 1766800000 },
    ];
    return sessions.filter((session) => session.backend === backend);
  }

  async stageClipboardImage(): Promise<StagedAttachment> {
    throw new Error("Clipboard images are only available in the installed desktop app.");
  }

  async stageImagePath(): Promise<StagedAttachment> {
    throw new Error("File staging is only available in the installed desktop app.");
  }

  async stageImageBytes(base64: string): Promise<StagedAttachment> {
    const token = `att-${++this.messageCounter}.png`;
    const staged: StagedAttachment = { token, label: `demo-${token}.png`, mime: "image/png" };
    this.stagedAttachments.set(token, { ...staged, label: `${staged.label} (${base64.length} b64 chars)` });
    return staged;
  }

  async removeStagedAttachment(token: string): Promise<void> {
    this.stagedAttachments.delete(token);
  }

  async discardStagedAttachments(): Promise<void> {
    this.stagedAttachments.clear();
  }

  async dispatchDesktopCommand(command: DesktopCommandV2): Promise<void> {
    switch (command.type) {
      case "set_mode":
        this.snapshot.mode = command.mode;
        this.emit({ type: "mode_changed", mode: command.mode });
        return;
      case "set_mode_overrides":
        this.snapshot.mode_overrides = clone(command.overrides);
        this.emit({
          type: "modes_updated",
          defaults: clone(this.snapshot.team?.modes ?? {}),
          overrides: clone(command.overrides),
        });
        return;
      case "save_mode_defaults": {
        const overrides = this.snapshot.mode_overrides;
        if (this.snapshot.team) {
          const block = command.mode === "review" ? overrides.review
            : command.mode === "plan" ? overrides.plan
            : command.mode === "brainstorm" ? overrides.brainstorm
            : overrides.team;
          if (block) {
            this.snapshot.team.modes = { ...this.snapshot.team.modes, [command.mode]: block };
          }
        }
        this.emit({ type: "team_settings_updated", team: clone(this.snapshot.team!) });
        return;
      }
      case "reset_mode_overrides": {
        const overrides = clone(this.snapshot.mode_overrides);
        delete (overrides as Record<string, unknown>)[command.mode];
        this.snapshot.mode_overrides = overrides;
        this.emit({
          type: "modes_updated",
          defaults: clone(this.snapshot.team?.modes ?? {}),
          overrides: clone(overrides),
        });
        return;
      }
      case "request_logs":
        this.emit({ type: "logs_replaced", request_id: command.request_id, entries: [
          { level: "info", source: "runtime", message: "Desktop demo log stream is ready." },
          { level: "warn", source: "builder", message: "A verification command is waiting for approval." },
        ], truncated: false });
        return;
      case "request_diff":
        this.emit({ type: "diff_replaced", request_id: command.request_id, result: { text: "diff --git a/desktop/src/App.tsx b/desktop/src/App.tsx\n@@\n+utility drawer enabled\n", truncated: false, file_count: 1 } });
        return;
      case "request_skills":
        this.emit({ type: "skills_replaced", request_id: command.request_id, skills: [
          { name: "review", description: "Review a change set with the team.", backend: "codex", invocation: "/review" },
          { name: "audit", description: "Audit a repository for risks.", backend: "claude", invocation: "/audit" },
        ], truncated: false });
        return;
      case "user_message": {
        const turn = Date.now();
        const user: TimelineItemV2 = {
          id: `mock-user-${++this.messageCounter}`,
          kind: "user",
          turn,
          text: command.body,
          attachments: command.attachments ?? [],
          timestamp: now(),
        };
        this.snapshot.timeline.push(user);
        this.emit({ type: "timeline_added", item: user });
        const target = command.target.type === "member" ? command.target.member
          : command.target.type === "members" ? command.target.members[0]
          : this.snapshot.members[0].id;
        const member = this.snapshot.members.find(({ id }) => id === target) ?? this.snapshot.members[0];
        // If the member is genuinely streaming a previous reply, the message
        // joins their queue (the demo snapshot's cosmetic "running" status
        // does not count as work in flight).
        if (member.status === "running" && this.activeStream) {
          const existing = this.snapshot.queues.find((queue) => queue.member === member.id);
          const prompts = existing ? [...existing.prompts, command.body] : [command.body];
          const queue: MemberQueueV2 = { member: member.id, prompts };
          this.snapshot.queues = [
            ...this.snapshot.queues.filter((entry) => entry.member !== member.id),
            queue,
          ];
          this.emit({ type: "queue_updated", member: member.id, prompts });
          return;
        }
        const running = { ...member, status: "running" as const };
        this.snapshot.members[this.snapshot.members.indexOf(member)] = running;
        this.emit({ type: "member_updated", member: running });
        const answer: TimelineItemV2 = {
          id: `mock-agent-${++this.messageCounter}`,
          kind: "agent",
          turn,
          member: member.id,
          display_name: member.display_name,
          backend: member.backend,
          text: "",
          streaming: true,
          timestamp: now(),
        };
        this.snapshot.timeline.push(answer);
        this.emit({ type: "timeline_added", item: answer });
        this.activeStream = true;
        const chunks = [
          "I’m mapping that request onto the current run. ",
          "The desktop bridge will keep runtime state authoritative, ",
          "and I’ll report verification evidence when the step finishes.",
        ];
        chunks.forEach((chunk, index) => {
          window.setTimeout(() => {
            answer.text = `${answer.text}${chunk}`;
            answer.streaming = index !== chunks.length - 1;
            this.emit({ type: "timeline_updated", item: { ...answer } });
            if (index === chunks.length - 1) {
              this.activeStream = false;
              const idle = { ...member, status: "idle" as const };
              this.snapshot.members[this.snapshot.members.indexOf(member)] = idle;
              this.emit({ type: "member_updated", member: idle });
            }
          // 300 ms cadence keeps the reply lively while giving the queue
          // smoke test a ~900 ms window to send a second message.
          }, 300 * (index + 1));
        });
        return;
      }
      case "edit_queued_prompt": {
        const queue = command.member
          ? this.snapshot.queues.find((entry) => entry.member === command.member)
          : this.snapshot.queues[0];
        if (!queue || queue.prompts.length === 0) {
          this.addNotice("No queued prompt is waiting to be edited.");
          return;
        }
        const body = queue.prompts[queue.prompts.length - 1];
        const prompts = queue.prompts.slice(0, -1);
        this.snapshot.queues = prompts.length
          ? this.snapshot.queues.map((entry) => entry.member === queue.member ? { ...entry, prompts } : entry)
          : this.snapshot.queues.filter((entry) => entry.member !== queue.member);
        this.emit({ type: "queued_prompt_returned", member: queue.member, body });
        return;
      }
      case "approve":
        this.snapshot.approvals = this.snapshot.approvals.filter(({ id }) => id !== command.id);
        this.emit({ type: "approval_removed", id: command.id, decision: command.decision });
        this.addNotice(command.decision === "approve" ? "Approval granted. The run may continue." : "Approval rejected. The member was notified.");
        return;
      case "set_relay_paused":
        this.snapshot.relay_paused = command.paused;
        this.addNotice(command.paused ? "Automatic relay paused by the user." : "Automatic relay resumed.");
        return;
      case "resolve_paused_route":
        this.addNotice(command.resume ? "Paused route resumed." : "Paused route dropped.");
        return;
      case "replace_team_settings":
        this.snapshot.team = clone(command.settings);
        this.snapshot.workspace = command.settings.workspace;
        this.snapshot.members = command.settings.members.map((member) => ({
          id: member.id,
          display_name: member.display_name,
          backend: member.backend,
          role: member.role,
          status: "idle",
          session: member.session_id,
          cwd: member.cwd ?? command.settings.workspace,
          model: member.model,
          effort: member.effort,
          sandbox: member.sandbox,
          permission_mode: member.permission_mode,
          session_policy: member.session_policy,
        }));
        this.emit({ type: "team_settings_updated", team: clone(command.settings) });
        this.emit({ type: "members_replaced", members: clone(this.snapshot.members) });
        return;
      case "request_team_settings":
        this.emit({ type: "team_settings_updated", team: clone(this.snapshot.team!) });
        return;
      case "new_session":
        this.snapshot.timeline = [];
        this.snapshot.approvals = [];
        this.snapshot.runs = [];
        this.snapshot.active_conversation = null;
        this.emit({ type: "timeline_cleared" });
        return;
      case "request_resume":
        this.emit({ type: "conversations_replaced", conversations: clone(this.snapshot.conversations) });
        return;
      case "resume_conversation":
        this.snapshot.active_conversation = command.conversation;
        this.snapshot.timeline = [
          { id: `restored-${command.conversation}-1`, kind: "user", text: "Review the restored workspace state.", timestamp: now() },
          { id: `restored-${command.conversation}-2`, kind: "agent", member: "reviewer", display_name: "Reviewer", backend: "claude", text: "Conversation restored from the local Asterline store.", timestamp: now() },
        ];
        this.emit({ type: "snapshot_replaced", snapshot: clone(this.snapshot) });
        return;
      case "import_session":
        this.addNotice(`Imported native session ${command.session_id} into the chat.`);
        return;
      case "export_session":
        this.addNotice("Successfully exported session to Claude format: ~/.claude/projects/demo/session.jsonl");
        return;
      case "continue_run":
      case "block_run": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        if (command.type === "continue_run") run.status = "running";
        if (command.type === "block_run") run.status = "blocked";
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "note_run": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        run.events = [
          ...run.events,
          { kind: "note", title: "Checkpoint note", detail: command.note, created_at: now(), attempt: run.attempt },
        ];
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "add_run_step": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        run.steps = [
          ...run.steps,
          { number: run.steps.length + 1, status: "todo", owner: command.owner ?? null, title: command.title, updated_at: now() },
        ];
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "update_run_step": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        run.steps = run.steps.map((step) =>
          step.number === command.step ? { ...step, status: command.status, note: command.note ?? step.note, updated_at: now() } : step,
        );
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "rename_run_step": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        run.steps = run.steps.map((step) =>
          step.number === command.step ? { ...step, title: command.title, updated_at: now() } : step,
        );
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "remove_run_step": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        run.steps = run.steps.filter((step) => step.number !== command.step).map((step, index) => ({ ...step, number: index + 1 }));
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "assign_run_step": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        run.steps = run.steps.map((step) =>
          step.number === command.step ? { ...step, owner: command.owner ?? null, updated_at: now() } : step,
        );
        run.updated_at = now();
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "run_mode": {
        const run: RunSummaryV2 = {
          id: Math.max(18, ...this.snapshot.runs.map(({ id }) => id)) + 1,
          number: Math.max(0, ...this.snapshot.runs.map(({ number }) => number)) + 1,
          goal: command.task,
          status: "running",
          coordinator: this.snapshot.members[0]?.id ?? null,
          created_at: now(),
          updated_at: now(),
          attempt: 1,
          verification: null,
          legacy_mode: null,
          mode: {
            mode: command.mode,
            state: { phase: "starting", iteration: 0, max_iterations: 3, round: 0, rounds: 0, idea_count: 0, vote_count: 0 },
          },
          steps: [],
          events: [{ kind: "started", title: "Mode run started", created_at: now(), attempt: 1 }],
        };
        this.snapshot.runs = [run, ...this.snapshot.runs];
        this.emit({ type: "run_updated", run: clone(run) });
        return;
      }
      case "cancel": {
        this.snapshot.members = this.snapshot.members.map((member) => ({ ...member, status: "idle" }));
        this.emit({ type: "members_replaced", members: clone(this.snapshot.members) });
        this.addNotice("Active work was cancelled.");
        return;
      }
      case "retry":
        this.addNotice("Retry requested for the latest turn.");
        return;
      case "shutdown":
        this.snapshot.phase = "shutting_down";
        this.emit({ type: "phase_changed", phase: "shutting_down" });
        return;
      default:
        this.addNotice(`Demo command accepted: ${(command as { type: string }).type}`);
    }
  }

  private addNotice(message: string): void {
    const item: TimelineItemV2 = { id: `mock-notice-${++this.messageCounter}`, kind: "notice", text: message, timestamp: now() };
    this.snapshot.timeline.push(item);
    this.emit({ type: "timeline_added", item });
  }

  async shutdownDesktop(): Promise<void> {
    await this.dispatchDesktopCommand({ type: "shutdown" });
  }

  async exitDesktop(): Promise<void> {
    await this.dispatchDesktopCommand({ type: "shutdown" });
  }

  async listRecentWorkspaces(): Promise<RecentWorkspace[]> {
    return [
      { workspace: "D:\\workspace\\asterline", name: "Asterline", last_opened_at: now() },
      { workspace: "D:\\workspace\\research-lab", name: "Research Lab", last_opened_at: "2026-08-10T09:00:00Z" },
    ];
  }

  async forgetRecentWorkspace(_workspace: string): Promise<void> {}

  async checkDesktopUpdate() {
    return { current_version: "0.3.0", available_version: null, release_url: null, update_available: false };
  }

  async openDesktopUpdate(_url: string): Promise<void> {}

  async getDesktopDiagnosticsStatus() {
    return { previous_unclean_exit: false, log_path: "mock/asterline-desktop.log" };
  }

  async exportDesktopDiagnostics(): Promise<string> {
    return "Downloads/asterline-desktop-diagnostics-demo.txt";
  }

  async getAttachCapabilities() {
    return {
      supported: true,
      terminal: "Windows Terminal",
      backends: {
        codex: { can_open: true, can_import: true },
        claude: { can_open: true, can_import: true },
        grok: { can_open: true, can_import: false },
        agy: { can_open: true, can_import: false },
      },
    };
  }

  async openNativeSession(member: string) {
    const known = this.snapshot.members.some(({ id }) => id === member);
    return {
      member,
      launched: known,
      terminal: known ? "Demo Terminal" : null,
      message: known ? "Native session launched by the browser demo." : "Unknown member",
    };
  }
}

function invalidNoArgs(command: string): ComposerAction {
  return { type: "invalid", message: `/${command} does not accept arguments; draft kept` };
}

export function createMockDesktopClient(): DesktopClient {
  return new MockDesktopClient();
}
