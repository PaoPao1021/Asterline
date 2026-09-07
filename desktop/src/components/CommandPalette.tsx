import { useEffect, useMemo, useRef, useState } from "react";
import type { CommandSpec, DesktopCommandV2 } from "../bridge/types";
import type { Translate } from "../i18n";
import { XIcon } from "./Icons";

interface CommandPaletteProps {
  catalog: CommandSpec[];
  /** When false the palette is read-only help (/help). */
  executable: boolean;
  title: string;
  t: Translate;
  onClose: () => void;
  onRun: (text: string) => void;
  /** Members offered for commands that take a member argument. */
  members?: Array<{ id: string; display_name: string }>;
}

export function CommandPalette({ catalog, executable, title, t, onClose, onRun, members = [] }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    input.current?.focus();
  }, []);

  const memberNames = useMemo(() => new Map(members.map((member) => [member.id, member.display_name])), [members]);

  const entries = useMemo(() => {
    const lower = query.trim().replace(/^\//, "").toLowerCase();
    return catalog
      .filter((spec) => !lower || spec.name.includes(lower) || spec.hint.toLowerCase().includes(lower))
      .map((spec) => ({ spec, text: spec.takes_argument ? exampleFor(spec, memberNames) : `/${spec.name}` }));
  }, [catalog, query, memberNames]);

  useEffect(() => {
    setIndex(0);
  }, [query]);

  const run = (text: string) => {
    onClose();
    onRun(text);
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="palette-modal" role="dialog" aria-modal="true" aria-label={title}>
        <header>
          <h2 id="palette-title">{title}</h2>
          <button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon size={15} /></button>
        </header>
        <p className="palette-intro">{t("helpIntro")}</p>
        <input
          ref={input}
          value={query}
          placeholder={t("paletteHint")}
          aria-label={t("commandPalette")}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") { event.preventDefault(); setIndex((value) => Math.min(value + 1, entries.length - 1)); }
            if (event.key === "ArrowUp") { event.preventDefault(); setIndex((value) => Math.max(value - 1, 0)); }
            if (event.key === "Enter" && entries[index]) run(entries[index].text);
            if (event.key === "Escape") onClose();
          }}
        />
        <ul className="palette-list">
          {entries.map(({ spec, text }, position) => (
            <li key={spec.name} className={position === index ? "active" : ""}>
              <button
                onMouseEnter={() => setIndex(position)}
                onClick={() => (executable ? run(text) : undefined)}
                disabled={!executable}
              >
                <code>{spec.takes_argument ? `/${spec.name} …` : `/${spec.name}`}</code>
                <span>{spec.hint}</span>
                <small>{spec.takes_argument ? t("takesArgument") : t("noArgument")}</small>
              </button>
            </li>
          ))}
          {entries.length === 0 && <li className="palette-empty">{t("noMatches")}</li>}
        </ul>
      </section>
    </div>
  );
}

function exampleFor(spec: CommandSpec, members: Map<string, string>): string {
  const first = members.keys().next().value;
  switch (spec.name) {
    case "ask": return `/ask ${first ?? "member"} …`;
    case "all": return "/all …";
    case "attach": return `/attach ${first ?? "member"}`;
    case "block": return "/block run-1 reason";
    case "continue": return "/continue run-1 note";
    case "export": return "/export claude";
    case "find": return "/find needle";
    case "focus": return `/focus ${first ?? "member"}`;
    case "import": return `/import ${first ?? "member"} <session_id>`;
    case "mode": return "/mode plan";
    case "note": return "/note run-1 checkpoint";
    case "step": return "/step add title";
    default: return `/${spec.name} …`;
  }
}

export type { DesktopCommandV2 };
