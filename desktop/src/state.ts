import type { DesktopEventV1, DesktopSnapshotV1, TimelineItemV1 } from "./bridge/types";

export interface DesktopState {
  snapshot: DesktopSnapshotV1 | null;
  loading: boolean;
  error: string | null;
}

export type DesktopAction =
  | { type: "loading" }
  | { type: "snapshot"; snapshot: DesktopSnapshotV1 }
  | { type: "event"; packet: DesktopEventV1 }
  | { type: "error"; error: string };

export const initialDesktopState: DesktopState = { snapshot: null, loading: true, error: null };

function upsertTimeline(items: TimelineItemV1[], item: TimelineItemV1): TimelineItemV1[] {
  const index = items.findIndex(({ id }) => id === item.id);
  if (index < 0) return [...items, item];
  const next = [...items];
  next[index] = { ...next[index], ...item };
  return next;
}

export function desktopReducer(state: DesktopState, action: DesktopAction): DesktopState {
  if (action.type === "loading") return { ...state, loading: true, error: null };
  if (action.type === "error") return { ...state, loading: false, error: action.error };
  if (action.type === "snapshot") {
    if (state.snapshot && action.snapshot.sequence < state.snapshot.sequence) return state;
    return { snapshot: action.snapshot, loading: false, error: action.snapshot.last_error ?? null };
  }

  const { packet } = action;
  if (packet.version !== 1 || !state.snapshot || packet.sequence <= state.snapshot.sequence) return state;
  const snapshot = { ...state.snapshot, sequence: packet.sequence };
  const event = packet.event;

  switch (event.type) {
    case "snapshot_replaced":
      return { snapshot: event.snapshot, loading: false, error: event.snapshot.last_error ?? null };
    case "phase_changed":
      snapshot.phase = event.phase;
      snapshot.last_error = event.error ?? null;
      break;
    case "mode_changed":
      snapshot.mode = event.mode;
      break;
    case "member_updated":
      snapshot.members = snapshot.members.some(({ id }) => id === event.member.id)
        ? snapshot.members.map((member) => member.id === event.member.id ? event.member : member)
        : [...snapshot.members, event.member];
      break;
    case "members_replaced":
      snapshot.members = event.members;
      break;
    case "timeline_added":
    case "timeline_updated":
      snapshot.timeline = upsertTimeline(snapshot.timeline, event.item);
      break;
    case "timeline_cleared":
      snapshot.timeline = [];
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
    case "notice":
      snapshot.timeline = [...snapshot.timeline, {
        id: `notice-${packet.sequence}`,
        kind: "notice",
        text: event.message,
        timestamp: new Date().toISOString(),
      }];
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

  return { snapshot, loading: false, error: snapshot.last_error ?? null };
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : typeof error === "string" ? error : "Unknown desktop bridge error";
}
