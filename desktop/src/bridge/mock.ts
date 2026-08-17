import type {
  DesktopClient,
  DesktopCommandV1,
  DesktopEventV1,
  DesktopRuntimeEventV1,
  DesktopSnapshotV1,
  RecentWorkspaceV1,
  RunSummaryV1,
  TeamSettingsV1,
  TimelineItemV1,
  Unlisten,
} from "./types";

const now = () => new Date().toISOString();

const demoTeam: TeamSettingsV1 = {
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
    gate: ["git", "shell", "file"],
    keywords: { deploy: ["publish", "release", "deploy"] },
    apply_to: ["user", "relay", "mode"],
  },
  modes: {
    review: { builder: "builder", reviewer: "reviewer", max_iterations: 3, auto_verify: true, verify_command: "cargo test --locked" },
    plan: { leader: "builder", reviewer: "reviewer", max_iterations: 2, auto_verify: false, verify_command: null },
    brainstorm: { participants: ["builder", "reviewer", "researcher"], generation_rounds: 3, ideas_per_round: 4 },
    team: { coordinator: "builder", max_iterations: 4, auto_verify: true, verify_command: "cargo test --locked" },
  },
};

const demoRun: RunSummaryV1 = {
  id: 18,
  goal: "Ship the desktop event bridge",
  status: "running",
  coordinator: "builder",
  created_at: "2026-08-12T08:21:00Z",
  updated_at: "2026-08-12T08:34:00Z",
  attempt: 1,
  verification: null,
  legacy_mode: null,
  mode: { mode: "team", state: "implementing" },
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

export const demoSnapshot: DesktopSnapshotV1 = {
  version: 1,
  sequence: 12,
  phase: "ready",
  workspace: demoTeam.workspace,
  team: demoTeam,
  mode: "team",
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

function clone<T>(value: T): T {
  return structuredClone(value);
}

class MockDesktopClient implements DesktopClient {
  readonly kind = "mock" as const;
  private snapshot = clone(demoSnapshot);
  private listeners = new Set<(event: DesktopEventV1) => void>();
  private sequence = this.snapshot.sequence;
  private messageCounter = 100;

  async listenRuntimeEvents(listener: (event: DesktopEventV1) => void): Promise<Unlisten> {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private emit(event: DesktopRuntimeEventV1): void {
    const packet: DesktopEventV1 = { version: 1, sequence: ++this.sequence, event };
    this.snapshot.sequence = packet.sequence;
    this.listeners.forEach((listener) => listener(clone(packet)));
  }

  async bootstrapDesktop(workspace?: string): Promise<DesktopSnapshotV1> {
    if (workspace) {
      this.snapshot.workspace = workspace;
      if (this.snapshot.team) {
        this.snapshot.team.workspace = workspace;
      }
      this.snapshot.phase = "ready";
    }
    return clone(this.snapshot);
  }

  async getDesktopSnapshot(): Promise<DesktopSnapshotV1> {
    return clone(this.snapshot);
  }

  async dispatchDesktopCommand(command: DesktopCommandV1): Promise<void> {
    switch (command.type) {
      case "set_mode":
        this.snapshot.mode = command.mode;
        this.emit({ type: "mode_changed", mode: command.mode });
        return;
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
        const user: TimelineItemV1 = { id: `mock-user-${++this.messageCounter}`, kind: "user", turn, text: command.body, timestamp: now() };
        this.snapshot.timeline.push(user);
        this.emit({ type: "timeline_added", item: user });
        const member = this.snapshot.members[0];
        const running = { ...member, status: "running" as const };
        this.snapshot.members[0] = running;
        this.emit({ type: "member_updated", member: running });
        const answer: TimelineItemV1 = {
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
              const idle = { ...member, status: "idle" as const };
              this.snapshot.members[0] = idle;
              this.emit({ type: "member_updated", member: idle });
            }
          }, 180 * (index + 1));
        });
        return;
      }
      case "approve":
        this.snapshot.approvals = this.snapshot.approvals.filter(({ id }) => id !== command.id);
        this.emit({ type: "approval_removed", id: command.id, decision: command.decision });
        this.addNotice(command.decision === "approve" ? "Approval granted. The run may continue." : "Approval rejected. The member was notified.");
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
      case "new_session":
        this.snapshot.timeline = [];
        this.snapshot.approvals = [];
        this.emit({ type: "timeline_cleared" });
        return;
      case "request_resume":
        this.emit({ type: "conversations_replaced", conversations: clone(this.snapshot.conversations) });
        return;
      case "resume_conversation":
        this.snapshot.timeline = [
          { id: `restored-${command.conversation}-1`, kind: "user", text: "Review the restored workspace state.", timestamp: now() },
          { id: `restored-${command.conversation}-2`, kind: "agent", member: "reviewer", display_name: "Reviewer", backend: "claude", text: "Conversation restored from the local Asterline store.", timestamp: now() },
        ];
        this.emit({ type: "snapshot_replaced", snapshot: clone(this.snapshot) });
        return;
      case "continue_run":
      case "verify_run":
      case "block_run": {
        const run = this.snapshot.runs.find(({ id }) => id === command.run_id) ?? this.snapshot.runs[0];
        if (!run) return;
        if (command.type === "continue_run") run.status = "running";
        if (command.type === "verify_run") run.status = "verifying";
        if (command.type === "block_run") run.status = "blocked";
        run.updated_at = now();
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
        this.addNotice(`Demo command accepted: ${command.type}`);
    }
  }

  private addNotice(message: string): void {
    const item: TimelineItemV1 = { id: `mock-notice-${++this.messageCounter}`, kind: "notice", text: message, timestamp: now() };
    this.snapshot.timeline.push(item);
    this.emit({ type: "timeline_added", item });
  }

  async shutdownDesktop(): Promise<void> {
    await this.dispatchDesktopCommand({ type: "shutdown" });
  }

  async listRecentWorkspaces(): Promise<RecentWorkspaceV1[]> {
    return [
      { workspace: "D:\\workspace\\asterline", name: "Asterline", last_opened_at: now() },
      { workspace: "D:\\workspace\\research-lab", name: "Research Lab", last_opened_at: "2026-08-10T09:00:00Z" },
    ];
  }

  async forgetRecentWorkspace(_workspace: string): Promise<void> {}

  async checkDesktopUpdate() {
    return { current_version: "0.1.0", available_version: null, release_url: null, update_available: false };
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

export function createMockDesktopClient(): DesktopClient {
  return new MockDesktopClient();
}
