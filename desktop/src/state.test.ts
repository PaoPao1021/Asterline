import { describe, expect, it } from "vitest";
import { demoSnapshot } from "./bridge/mock";
import { desktopReducer, initialUtilityState } from "./state";

describe("desktopReducer", () => {
  it("ignores duplicate and older events", () => {
    const state = { snapshot: structuredClone(demoSnapshot), loading: false, error: null, utility: structuredClone(initialUtilityState) };
    const next = desktopReducer(state, {
      type: "event",
      packet: { version: 1, sequence: demoSnapshot.sequence, event: { type: "mode_changed", mode: "review" } },
    });
    expect(next).toBe(state);
  });

  it("merges streaming timeline updates by stable id", () => {
    const snapshot = structuredClone(demoSnapshot);
    const state = { snapshot, loading: false, error: null, utility: structuredClone(initialUtilityState) };
    const next = desktopReducer(state, {
      type: "event",
      packet: {
        version: 1,
        sequence: snapshot.sequence + 1,
        event: { type: "timeline_updated", item: { id: "agent-1", kind: "agent", text: "complete", streaming: false } },
      },
    });
    expect(next.snapshot?.timeline.find(({ id }) => id === "agent-1")?.text).toBe("complete");
    expect(next.snapshot?.timeline.filter(({ id }) => id === "agent-1")).toHaveLength(1);
  });

  it("surfaces paused routes as actionable timeline items", () => {
    const snapshot = structuredClone(demoSnapshot);
    const next = desktopReducer({ snapshot, loading: false, error: null, utility: structuredClone(initialUtilityState) }, {
      type: "event",
      packet: {
        version: 1,
        sequence: snapshot.sequence + 1,
        event: { type: "route_paused", turn: 14, member: "builder", to: ["reviewer"], reason: "relay limit reached", queued: 2 },
      },
    });
    expect(next.snapshot?.timeline.at(-1)).toMatchObject({ kind: "route_paused", member: "builder", text: "relay limit reached", detail: "2" });
  });

  it("keeps member and team session IDs in sync", () => {
    const snapshot = structuredClone(demoSnapshot);
    const state = { snapshot, loading: false, error: null, utility: structuredClone(initialUtilityState) };
    const next = desktopReducer(state, {
      type: "event",
      packet: {
        version: 1,
        sequence: snapshot.sequence + 1,
        event: { type: "session_updated", member: "researcher", session: "session-refreshed" },
      },
    });

    expect(next.snapshot?.members.find(({ id }) => id === "researcher")?.session).toBe("session-refreshed");
    expect(next.snapshot?.team?.members.find(({ id }) => id === "researcher")?.session_id).toBe("session-refreshed");
  });

  it("ignores stale utility responses", () => {
    const snapshot = structuredClone(demoSnapshot);
    const requested = desktopReducer({ snapshot, loading: false, error: null, utility: structuredClone(initialUtilityState) }, {
      type: "utility_request", utility: "logs", requestId: 9,
    });
    const stale = desktopReducer(requested, {
      type: "event",
      packet: { version: 1, sequence: snapshot.sequence + 1, event: { type: "logs_replaced", request_id: 8, entries: [{ level: "info", source: "old", message: "stale" }], truncated: false } },
    });
    expect(stale.utility.logs.value).toEqual([]);
  });
});
