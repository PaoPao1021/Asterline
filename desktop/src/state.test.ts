import { describe, expect, it } from "vitest";
import { demoSnapshot } from "./bridge/mock";
import { desktopReducer } from "./state";

describe("desktopReducer", () => {
  it("ignores duplicate and older events", () => {
    const state = { snapshot: structuredClone(demoSnapshot), loading: false, error: null };
    const next = desktopReducer(state, {
      type: "event",
      packet: { version: 1, sequence: demoSnapshot.sequence, event: { type: "mode_changed", mode: "review" } },
    });
    expect(next).toBe(state);
  });

  it("merges streaming timeline updates by stable id", () => {
    const snapshot = structuredClone(demoSnapshot);
    const state = { snapshot, loading: false, error: null };
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
    const next = desktopReducer({ snapshot, loading: false, error: null }, {
      type: "event",
      packet: {
        version: 1,
        sequence: snapshot.sequence + 1,
        event: { type: "route_paused", turn: 14, member: "builder", to: ["reviewer"], reason: "relay limit reached", queued: 2 },
      },
    });
    expect(next.snapshot?.timeline.at(-1)).toMatchObject({ kind: "route_paused", member: "builder", text: "relay limit reached", detail: "2" });
  });
});
