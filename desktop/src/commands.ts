import type { DesktopCommandV1, MessageTargetV1, TerminalMode } from "./bridge/types";

export type ParsedComposerAction =
  | { kind: "empty" }
  | { kind: "command"; command: DesktopCommandV1 }
  | { kind: "message"; target: MessageTargetV1; body: string }
  | { kind: "unsupported"; command: string };

const MODES = new Set<TerminalMode>(["normal", "review", "plan", "brainstorm", "team"]);
const DEFERRED = new Set(["logs", "diff", "skills", "find"]);

export function parseComposerInput(raw: string, selectedTarget: string): ParsedComposerAction {
  const text = raw.trim();
  if (!text) return { kind: "empty" };
  if (text.startsWith("/")) {
    const [head, ...rest] = text.slice(1).split(/\s+/);
    const name = head.toLowerCase();
    const tail = rest.join(" ").trim();
    if (name === "mode" && MODES.has(tail as TerminalMode)) {
      return { kind: "command", command: { type: "set_mode", mode: tail as TerminalMode } };
    }
    if (name === "new") return { kind: "command", command: { type: "new_session" } };
    if (name === "resume") return { kind: "command", command: { type: "request_resume" } };
    if (name === "retry") return { kind: "command", command: { type: "retry" } };
    if (name === "cancel") return { kind: "command", command: { type: "cancel", member: tail || null } };
    if (name === "continue") return { kind: "command", command: { type: "continue_run", note: tail || null } };
    if (name === "verify") return { kind: "command", command: { type: "verify_run", command: tail || null } };
    if (DEFERRED.has(name)) return { kind: "unsupported", command: name };
    return { kind: "unsupported", command: name || "/" };
  }

  const mention = text.match(/^@([A-Za-z0-9_-]+)\s+([\s\S]+)$/);
  if (mention) return { kind: "message", target: mention[1].toLowerCase() === "all" ? { type: "all" } : { type: "member", member: mention[1] }, body: mention[2].trim() };
  const target: MessageTargetV1 = selectedTarget === "all"
    ? { type: "all" }
    : selectedTarget === "default"
      ? { type: "default" }
      : { type: "member", member: selectedTarget };
  return { kind: "message", target, body: text };
}
