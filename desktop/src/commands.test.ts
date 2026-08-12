import { describe, expect, it } from "vitest";
import { parseComposerInput } from "./commands";

describe("parseComposerInput", () => {
  it("routes @member messages independently of the selected target", () => {
    expect(parseComposerInput("@reviewer check the bridge", "builder")).toEqual({
      kind: "message",
      target: { type: "member", member: "reviewer" },
      body: "check the bridge",
    });
  });

  it("parses mode and lifecycle commands", () => {
    expect(parseComposerInput("/mode brainstorm", "default")).toEqual({ kind: "command", command: { type: "set_mode", mode: "brainstorm" } });
    expect(parseComposerInput("/new", "default")).toEqual({ kind: "command", command: { type: "new_session" } });
    expect(parseComposerInput("/verify cargo test", "default")).toEqual({ kind: "command", command: { type: "verify_run", command: "cargo test" } });
  });

  it("marks deferred drawers explicitly", () => {
    expect(parseComposerInput("/diff", "default")).toEqual({ kind: "unsupported", command: "diff" });
  });
});
