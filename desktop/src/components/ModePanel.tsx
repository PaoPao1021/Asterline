import { useEffect, useMemo, useState } from "react";
import type { DesktopCommandV2, MemberSummaryV2, ModesConfigV2, TerminalMode } from "../bridge/types";
import type { Translate } from "../i18n";
import { Dropdown } from "./Dropdown";
import { XIcon } from "./Icons";
import { useDialogFocus } from "./useDialogFocus";

const COLLAB_MODES = ["review", "plan", "brainstorm", "team"] as const;
type CollabMode = (typeof COLLAB_MODES)[number];

interface ModePanelProps {
  mode: TerminalMode;
  defaults: ModesConfigV2;
  overrides: ModesConfigV2;
  members: MemberSummaryV2[];
  busy: boolean;
  t: Translate;
  onClose: () => void;
  dispatch: (command: DesktopCommandV2) => Promise<void>;
}

/** Where the effective value of one knob currently comes from. */
type ValueSource = "conversation" | "team_json" | "default";

interface Knob {
  key: string;
  label: string;
  value: string;
  source: ValueSource;
  kind: "member" | "number" | "flag" | "text" | "members";
  options?: string[];
}

function blockFor(modes: ModesConfigV2, mode: CollabMode): Record<string, unknown> | null {
  return (modes[mode] as Record<string, unknown> | null | undefined) ?? null;
}

function sourceOf(overridden: boolean, inTeamJson: boolean): ValueSource {
  if (overridden) return "conversation";
  if (inTeamJson) return "team_json";
  return "default";
}

function sourceLabel(source: ValueSource, t: Translate): string {
  return source === "conversation" ? t("sourceThisChat")
    : source === "team_json" ? t("sourceTeamJson")
    : t("sourceDefault");
}

export function ModePanel({ mode, defaults, overrides, members, busy, t, onClose, dispatch }: ModePanelProps) {
  const dialog = useDialogFocus<HTMLElement>();
  // Keyboard-dismissible dialog (WCAG): Escape closes the panel.
  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [busy, onClose]);
  const [task, setTask] = useState("");
  const memberIds = useMemo(() => members.map((member) => member.id), [members]);

  const knobs = useMemo((): Knob[] => {
    if (mode === "normal") return [];
    const collab = mode as CollabMode;
    const base = blockFor(defaults, collab);
    const over = blockFor(overrides, collab);
    const read = (key: string): { value: unknown; source: ValueSource } => {
      if (over && over[key] !== undefined && over[key] !== null) return { value: over[key], source: "conversation" };
      if (base && base[key] !== undefined && base[key] !== null) return { value: base[key], source: "team_json" };
      return { value: undefined, source: "default" };
    };
    const text = (key: string, label: string): Knob => {
      const { value, source } = read(key);
      return { key, label, value: value === undefined || value === null ? "" : String(value), source, kind: "member", options: memberIds };
    };
    if (collab === "review") {
      return [text("builder", t("builder")), text("reviewer", t("reviewer")),
        { ...readNumber("max_iterations"), label: t("maxIterationsLabel"), key: "max_iterations" },
        { ...readText("reviewer_hint"), label: t("reviewerHintLabel"), key: "reviewer_hint" },
      ];
    }
    if (collab === "plan") {
      return [text("leader", t("leader")), text("builder", t("planBuilder")), text("reviewer", t("reviewer")),
        { ...readNumber("max_iterations"), label: t("maxIterationsLabel"), key: "max_iterations" },
        { ...readFlag("auto_execute"), label: t("planAutoExecute"), key: "auto_execute" },
      ];
    }
    if (collab === "brainstorm") {
      const { value, source } = read("participants");
      return [
        { key: "participants", label: t("participantsLabel"), value: Array.isArray(value) ? value.join(", ") : "", source, kind: "members", options: memberIds },
        { ...readNumber("generation_rounds"), label: t("roundsLabel"), key: "generation_rounds" },
        { ...readNumber("ideas_per_round"), label: t("ideasLabel"), key: "ideas_per_round" },
      ];
    }
    return [text("coordinator", t("coordinatorLabel")),
      { ...readNumber("max_iterations"), label: t("maxIterationsLabel"), key: "max_iterations" },
      { ...readFlag("allow_add_members"), label: t("allowAddMembersLabel"), key: "allow_add_members" },
    ];

    function readNumber(key: string) {
      const { value, source } = read(key);
      return { value: value === undefined || value === null ? "" : String(value), source, kind: "number" as const };
    }
    function readFlag(key: string) {
      const { value, source } = read(key);
      return { value: value === undefined || value === null ? "" : value ? "true" : "false", source, kind: "flag" as const };
    }
    function readText(key: string) {
      const { value, source } = read(key);
      return { value: value === undefined || value === null ? "" : String(value), source, kind: "text" as const };
    }
  }, [mode, defaults, overrides, memberIds, t]);

  const writeOverride = (collab: CollabMode, key: string, raw: string) => {
    const next: Record<string, unknown> = { ...(blockFor(overrides, collab) ?? {}) };
    if (raw === "") delete next[key];
    else if (key === "max_iterations" || key === "generation_rounds" || key === "ideas_per_round") next[key] = Number(raw);
    else if (key === "auto_execute" || key === "allow_add_members") next[key] = raw === "true";
    else if (key === "participants") next[key] = raw.split(",").map((entry) => entry.trim()).filter(Boolean);
    else next[key] = raw;
    const cleaned = Object.fromEntries(Object.entries(next).filter(([, value]) => value !== undefined));
    dispatch({
      type: "set_mode_overrides",
      overrides: { ...overrides, [collab]: Object.keys(cleaned).length > 0 ? cleaned : null } as ModesConfigV2,
    });
  };

  const startRun = async () => {
    if (!task.trim() || mode === "normal") return;
    await dispatch({ type: "run_mode", mode: mode as Exclude<TerminalMode, "normal">, task: task.trim() });
    onClose();
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section ref={dialog} className="mode-panel-modal" role="dialog" aria-modal="true" aria-label={t("modes")} tabIndex={-1}>
        <header>
          <h2 id="mode-panel-title">{t("modes")}</h2>
          <button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon size={15} /></button>
        </header>
        <div className="mode-panel-tabs" role="tablist">
          {(["normal", ...COLLAB_MODES] as TerminalMode[]).map((value) => (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={value === mode}
              className={value === mode ? "active" : ""}
              disabled={busy}
              data-dialog-autofocus={value === mode ? "true" : undefined}
              onClick={() => void dispatch({ type: "set_mode", mode: value })}
            >
              {t(value)}
            </button>
          ))}
        </div>

        {mode === "normal" ? (
          <p className="mode-panel-note">{t("commandHint")}</p>
        ) : (
          <>
            <h3>{t("modeBindings")}</h3>
            <table className="mode-knobs">
              <tbody>
                {knobs.map((knob) => (
                  <tr key={knob.key}>
                    <th scope="row">{knob.label}</th>
                    <td>
                      {knob.kind === "member" ? (
                        <Dropdown
                          label={knob.label}
                          value={String(knob.value)}
                          disabled={busy}
                          onChange={(next) => writeOverride(mode as CollabMode, knob.key, next)}
                          options={[
                            { value: "", label: t("unassigned") },
                            ...(knob.options ?? []).map((id) => ({ value: id, label: id })),
                          ]}
                        />
                      ) : knob.kind === "flag" ? (
                        <Dropdown
                          label={knob.label}
                          value={String(knob.value)}
                          disabled={busy}
                          onChange={(next) => writeOverride(mode as CollabMode, knob.key, next)}
                          options={[
                            { value: "", label: t("unassigned") },
                            { value: "true", label: "true" },
                            { value: "false", label: "false" },
                          ]}
                        />
                      ) : (
                        <input
                          value={String(knob.value)}
                          disabled={busy}
                          onChange={(event) => writeOverride(mode as CollabMode, knob.key, event.target.value)}
                        />
                      )}
                    </td>
                    <td>
                      <span className={`mode-source source-${knob.source}`}>{sourceLabel(knob.source, t)}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="mode-panel-actions">
              <button className="secondary-button" disabled={busy} onClick={() => dispatch({ type: "reset_mode_overrides", mode: mode as Exclude<TerminalMode, "normal"> })}>{t("resetOverrides")}</button>
              <button className="secondary-button" disabled={busy} onClick={() => dispatch({ type: "save_mode_defaults", mode: mode as Exclude<TerminalMode, "normal"> })}>{t("saveAsDefault")}</button>
            </div>
            <h3>{t("startRun")}</h3>
            <div className="mode-run-launcher">
              <textarea
                value={task}
                placeholder={t("runTaskPlaceholder")}
                aria-label={t("runTaskPlaceholder")}
                onChange={(event) => setTask(event.target.value)}
              />
              <button className="primary-button" disabled={busy || !task.trim()} onClick={() => void startRun()}>{t("startRun")}</button>
            </div>
          </>
        )}
      </section>
    </div>
  );
}
