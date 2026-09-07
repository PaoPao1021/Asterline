import { beforeEach, describe, expect, it } from "vitest";
import { setDesktopClientForTests } from "./bridge/client";
import { createMockDesktopClient, demoSnapshot } from "./bridge/mock";
import { mergeModes } from "./state";

describe("composer contract via the bridge parser (mock mirror)", () => {
  let client: ReturnType<typeof createMockDesktopClient>;

  beforeEach(() => {
    client = createMockDesktopClient();
    setDesktopClientForTests(client);
    return () => setDesktopClientForTests(undefined);
  });

  it("accepts every documented no-argument command", async () => {
    for (const command of ["approve", "diff", "exit", "help", "logs", "resume", "retry", "runs", "team"]) {
      const action = await client.parseComposerText(`/${command}`);
      expect(action.type, `/${command}`).not.toBe("invalid");
    }
  });

  it("treats /clear as an alias of /new", async () => {
    expect(await client.parseComposerText("/new")).toEqual(await client.parseComposerText("/clear"));
    expect(await client.parseComposerText("/clear")).toEqual({ type: "command", command: { type: "new_session" } });
  });

  it("keeps unknown commands as help so the draft stays visible", async () => {
    expect(await client.parseComposerText("/definitely-not-real")).toEqual({ type: "help" });
    expect(await client.parseComposerText("/model builder gpt-9")).toEqual({ type: "help" });
  });

  it("routes plain text to the target selector", async () => {
    expect(await client.parseComposerText("no explicit target")).toEqual({ type: "needs_target" });
  });

  it("refuses targeted slash invocations that are not discovered skills", async () => {
    const action = await client.parseComposerText("@builder /model gpt-9");
    expect(action.type).toBe("invalid");
    expect((action as { message: string }).message).toContain("not a discovered prompt-invocable skill");
  });

  it("marks runs blocked and returns queued prompts for editing", async () => {
    await client.dispatchDesktopCommand({ type: "note_run", run_id: 18, note: "checkpoint" });
    await client.dispatchDesktopCommand({ type: "block_run", run_id: 18, reason: "missing credentials" });
    const snapshot = await client.getDesktopSnapshot();
    expect(snapshot.runs[0].status).toBe("blocked");
  });

  it("surfaces the suggested verify command and structured mode state", () => {
    expect(demoSnapshot.suggested_verify).toBeNull();
    expect(demoSnapshot.runs[0].mode?.state.phase).toBe("implementing");
    expect(demoSnapshot.runs[0].mode?.state.max_iterations).toBe(4);
  });
});

describe("mergeModes", () => {
  it("gives conversation overrides precedence over team defaults", () => {
    const merged = mergeModes(
      { team: { coordinator: "builder", max_iterations: 3 } },
      { team: { max_iterations: 7 } },
    );
    expect(merged.team).toEqual({ coordinator: "builder", max_iterations: 7 });
  });

  it("keeps modes untouched when no overrides exist", () => {
    const merged = mergeModes({ review: { reviewer: "reviewer" } }, {});
    expect(merged.review).toEqual({ reviewer: "reviewer" });
    expect(merged.plan).toBeNull();
  });
});
