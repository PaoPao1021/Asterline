import type {
  DesktopEventV2,
  DesktopSnapshotV2,
  DiffResultV2,
  LogEntryV2,
  ModesConfigV2,
  SkillSummaryV2,
  TimelineItemV2,
} from "./bridge/types";

/** Timeline caps mirrored from the host so live streaming truncates the same
 * way a recovered snapshot would (5 000 items; per-item bytes clamp host-side). */
export const MAX_TIMELINE_ITEMS = 5_000;

export type UtilityStatus = "idle" | "loading" | "ready" | "error";
export interface UtilityResource<T> {
  status: UtilityStatus;
  requestId: number;
  value: T;
  truncated: boolean;
  error: string | null;
}
export interface UtilityState {
  logs: UtilityResource<LogEntryV2[]>;
  diff: UtilityResource<DiffResultV2 | null>;
  skills: UtilityResource<SkillSummaryV2[]>;
}

export interface DesktopState {
  snapshot: DesktopSnapshotV2 | null;
  loading: boolean;
  error: string | null;
  utility: UtilityState;
}

const emptyResource = <T,>(value: T): UtilityResource<T> => ({ status: "idle", requestId: 0, value, truncated: false, error: null });

export const initialUtilityState: UtilityState = {
  logs: emptyResource<LogEntryV2[]>([]),
  diff: emptyResource<DiffResultV2 | null>(null),
  skills: emptyResource<SkillSummaryV2[]>([]),
};

export type DesktopAction =
  | { type: "loading" }
  | { type: "snapshot"; snapshot: DesktopSnapshotV2 }
  | { type: "event"; packet: DesktopEventV2 }
  | { type: "utility_request"; utility: keyof UtilityState; requestId: number }
  | { type: "utility_error"; utility: keyof UtilityState; requestId: number; error: string }
  | { type: "error"; error: string };

export const initialDesktopState: DesktopState = { snapshot: null, loading: true, error: null, utility: initialUtilityState };

function upsertTimeline(items: TimelineItemV2[], item: TimelineItemV2): TimelineItemV2[] {
  const index = items.findIndex(({ id }) => id === item.id);
  let next: TimelineItemV2[];
  if (index < 0) next = [...items, item];
  else {
    next = [...items];
    next[index] = { ...next[index], ...item };
  }
  // Mirror the host's timeline cap while streaming.
  if (next.length > MAX_TIMELINE_ITEMS) next = next.slice(next.length - MAX_TIMELINE_ITEMS);
  return next;
}

/** Field-level merge of mode defaults and conversation overrides. */
export function mergeModes(
  defaults: ModesConfigV2 | undefined,
  overrides: ModesConfigV2 | undefined,
): ModesConfigV2 {
  const merge = <T extends Record<string, unknown>>(
    base: T | null | undefined,
    over: T | null | undefined,
  ): T | null | undefined => {
    if (!base && !over) return null;
    return { ...(base ?? {}), ...(over ?? {}) } as T;
  };
  return {
    review: merge(defaults?.review, overrides?.review),
    plan: merge(defaults?.plan, overrides?.plan),
    brainstorm: merge(defaults?.brainstorm, overrides?.brainstorm),
    team: merge(defaults?.team, overrides?.team),
  };
}

export function desktopReducer(state: DesktopState, action: DesktopAction): DesktopState {
  if (action.type === "loading") return { ...state, loading: true, error: null };
  if (action.type === "error") return { ...state, loading: false, error: action.error };
  if (action.type === "utility_request") {
    return {
      ...state,
      utility: {
        ...state.utility,
        [action.utility]: { ...state.utility[action.utility], status: "loading", requestId: action.requestId, error: null },
      },
    };
  }
  if (action.type === "utility_error") {
    const resource = state.utility[action.utility];
    if (action.requestId !== resource.requestId) return state;
    return { ...state, utility: { ...state.utility, [action.utility]: { ...resource, status: "error", error: action.error } } };
  }
  if (action.type === "snapshot") {
    if (state.snapshot && action.snapshot.sequence < state.snapshot.sequence) return state;
    return { snapshot: action.snapshot, loading: false, error: action.snapshot.last_error ?? null, utility: initialUtilityState };
  }

  const { packet } = action;
  if (packet.version !== 2 || !state.snapshot || packet.sequence <= state.snapshot.sequence) return state;
  const snapshot = { ...state.snapshot, sequence: packet.sequence };
  const event = packet.event;

  switch (event.type) {
    case "snapshot_replaced":
      return { snapshot: event.snapshot, loading: false, error: event.snapshot.last_error ?? null, utility: initialUtilityState };
    case "phase_changed":
      snapshot.phase = event.phase;
      snapshot.last_error = event.error ?? null;
      break;
    case "mode_changed":
      snapshot.mode = event.mode;
      break;
    case "modes_updated":
      snapshot.mode_overrides = event.overrides;
      if (snapshot.team) snapshot.team = { ...snapshot.team, modes: event.defaults };
      break;
    case "member_updated":
      snapshot.members = snapshot.members.some(({ id }) => id === event.member.id)
        ? snapshot.members.map((member) => member.id === event.member.id ? event.member : member)
        : [...snapshot.members, event.member];
      break;
    case "members_replaced":
      snapshot.members = event.members;
      break;
    case "session_updated":
      // Session IDs are emitted independently from the member summary when a
      // native backend establishes (or refreshes) its conversation. Keep the
      // roster in sync so attach/resume actions immediately use the new ID.
      snapshot.members = snapshot.members.map((member) =>
        member.id === event.member ? { ...member, session: event.session } : member,
      );
      if (snapshot.team) {
        snapshot.team = {
          ...snapshot.team,
          members: snapshot.team.members.map((member) =>
            member.id === event.member ? { ...member, session_id: event.session } : member,
          ),
        };
      }
      break;
    case "logs_replaced": {
      const resource = state.utility.logs;
      if (event.request_id !== resource.requestId) return state;
      return { ...state, snapshot, utility: { ...state.utility, logs: { ...resource, status: "ready", value: event.entries, truncated: event.truncated, error: null } } };
    }
    case "diff_replaced": {
      const resource = state.utility.diff;
      if (event.request_id !== resource.requestId) return state;
      return { ...state, snapshot, utility: { ...state.utility, diff: { ...resource, status: "ready", value: event.result, truncated: event.result.truncated, error: null } } };
    }
    case "skills_replaced": {
      const resource = state.utility.skills;
      if (event.request_id !== resource.requestId) return state;
      return { ...state, snapshot, utility: { ...state.utility, skills: { ...resource, status: "ready", value: event.skills, truncated: event.truncated, error: null } } };
    }
    case "timeline_added":
    case "timeline_updated":
      snapshot.timeline = upsertTimeline(snapshot.timeline, event.item);
      break;
    case "timeline_cleared":
      snapshot.timeline = [];
      snapshot.timeline_truncated = false;
      break;
    case "approval_added":
      snapshot.approvals = [...snapshot.approvals.filter(({ id }) => id !== event.approval.id), event.approval];
      break;
    case "approval_removed":
      snapshot.approvals = snapshot.approvals.filter(({ id }) => id !== event.id);
      break;
    case "run_updated":
      snapshot.runs = snapshot.runs.some(({ id }) => id === event.run.id)
        ? snapshot.runs.map((run) => run.id === event.run.id ? event.run : run)
        : [event.run, ...snapshot.runs];
      break;
    case "conversations_replaced":
      snapshot.conversations = event.conversations;
      break;
    case "team_settings_updated":
      snapshot.team = event.team;
      snapshot.workspace = event.team.workspace;
      break;
    case "queue_updated": {
      const queues = snapshot.queues.filter((queue) => queue.member !== event.member);
      if (event.prompts.length > 0) queues.push({ member: event.member, prompts: event.prompts });
      snapshot.queues = queues;
      break;
    }
    case "queued_prompt_returned":
      snapshot.queues = snapshot.queues.filter((queue) => queue.member !== event.member);
      break;
    case "notice":
      snapshot.timeline = upsertTimeline(snapshot.timeline, {
        id: `notice-${packet.sequence}`,
        kind: "notice",
        text: event.message,
        timestamp: new Date().toISOString(),
      });
      break;
    case "route_paused":
      snapshot.timeline = upsertTimeline(snapshot.timeline, {
        id: `route-paused-${event.turn}-${packet.sequence}`,
        kind: "route_paused",
        turn: event.turn,
        member: event.member,
        to: event.to,
        text: event.reason,
        detail: String(event.queued),
        timestamp: new Date().toISOString(),
      });
      break;
    default:
      // Unknown events are intentionally ignored so a newer backend does not
      // break an older desktop shell.
      break;
  }

  return { snapshot, loading: false, error: snapshot.last_error ?? null, utility: state.utility };
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : typeof error === "string" ? error : "Unknown desktop bridge error";
}
