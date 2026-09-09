import { useEffect, useState } from "react";
import type { DesktopCommandV2, MemberSummaryV2, RunStatus, RunSummaryV2, RunStepStatus } from "../bridge/types";
import type { Translate } from "../i18n";
import { Dropdown } from "./Dropdown";
import { CheckIcon, PlusIcon, XIcon } from "./Icons";
import { useDialogFocus } from "./useDialogFocus";

interface RunsPanelProps {
  runs: RunSummaryV2[];
  members: MemberSummaryV2[];
  busy: boolean;
  t: Translate;
  onClose: () => void;
  dispatch: (command: DesktopCommandV2) => Promise<void>;
}

const STEP_STATUSES: RunStepStatus[] = ["todo", "doing", "done", "blocked"];

export function RunsPanel({ runs, members, busy, t, onClose, dispatch }: RunsPanelProps) {
  const dialog = useDialogFocus<HTMLElement>();
  const [openRun, setOpenRun] = useState<number | null>(runs[0]?.id ?? null);
  // Keyboard-dismissible dialog (WCAG): Escape closes the panel.
  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      // Closing this view does not stop or mutate the active run.
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [onClose]);
  const [stepTitle, setStepTitle] = useState("");
  const [noteText, setNoteText] = useState("");
  const [blockText, setBlockText] = useState("");

  const active = runs.find((run) => run.id === openRun) ?? null;

  const withRun = (command: (runId: number | null) => DesktopCommandV2) => {
    if (!active) return;
    void dispatch(command(active.id));
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section ref={dialog} className="runs-modal" role="dialog" aria-modal="true" aria-label={t("runs")} tabIndex={-1}>
        <header>
          <h2 id="runs-title">{t("runs")}</h2>
          <button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon size={15} /></button>
        </header>
        {runs.length === 0 && <p className="runs-empty">{t("noRuns")}</p>}
        <ul className="runs-list">
          {runs.map((run) => (
            <li key={run.id} className={run.id === openRun ? "active" : ""}>
              <button data-dialog-autofocus={run.id === openRun ? "true" : undefined} onClick={() => setOpenRun(run.id === openRun ? null : run.id)}>
                <span className={`run-pill status-${run.status}`}>{t(run.status)}</span>
                <strong>run-{run.number || run.id}</strong>
                <span className="runs-goal">{run.goal}</span>
                {run.mode && <span className="run-pill mode">{t(run.mode.mode)} · {run.mode.state.phase || t("running")} {run.mode.state.iteration > 0 ? `${run.mode.state.iteration}/${run.mode.state.max_iterations}` : ""}</span>}
              </button>
            </li>
          ))}
        </ul>
        {active && (
          <div className="runs-detail">
            <p className="runs-meta">
              {t("attemptLabel")} {active.attempt}
              {active.coordinator ? ` · ${t("coordinatorLabel")}: ${active.coordinator}` : ""}
              {active.mode ? ` · ${t(active.mode.mode)}` : ""}
            </p>
            <ol className="run-steps">
              {active.steps.map((step) => (
                <li key={step.number} className={`step-${step.status}`}>
                  <span className="step-number">{step.status === "done" ? <CheckIcon size={12} /> : step.number}</span>
                  <span className="step-title">
                    {step.title}
                    {step.note ? <small> — {step.note}</small> : null}
                    <small className="step-owner">{step.owner ? `@${members.find((member) => member.id === step.owner)?.display_name ?? step.owner}` : t("unassigned")}</small>
                  </span>
                  <Dropdown
                    variant="compact"
                    align="end"
                    label={`${t("runs")} ${step.number}`}
                    value={step.status}
                    disabled={busy}
                    onChange={(next) => withRun((runId) => ({ type: "update_run_step", run_id: runId, step: step.number, status: next as RunStepStatus }))}
                    options={STEP_STATUSES.map((status) => ({ value: status, label: t(status) }))}
                  />
                  <Dropdown
                    variant="compact"
                    align="end"
                    label={`${t("assignStep")} ${step.number}`}
                    value={step.owner ?? ""}
                    disabled={busy}
                    onChange={(next) => withRun((runId) => ({ type: "assign_run_step", run_id: runId, step: step.number, owner: next || null }))}
                    options={[
                      { value: "", label: t("unassigned") },
                      ...members.map((member) => ({ value: member.id, label: member.display_name, hint: member.backend })),
                    ]}
                  />
                  <button
                    className="subtle-icon-button"
                    aria-label={`${t("removeStep")} ${step.number}`}
                    disabled={busy}
                    onClick={() => withRun((runId) => ({ type: "remove_run_step", run_id: runId, step: step.number }))}
                  >
                    <XIcon size={12} />
                  </button>
                </li>
              ))}
            </ol>
            <div className="run-step-add">
              <input
                value={stepTitle}
                placeholder={t("stepTitlePlaceholder")}
                aria-label={t("addStep")}
                onChange={(event) => setStepTitle(event.target.value)}
              />
              <button
                className="secondary-button"
                disabled={busy || !stepTitle.trim()}
                onClick={() => {
                  withRun((runId) => ({ type: "add_run_step", run_id: runId, owner: null, title: stepTitle.trim() }));
                  setStepTitle("");
                }}
              >
                <PlusIcon size={13} />{t("addStep")}
              </button>
            </div>
            <div className="run-actions">
              <button className="secondary-button" disabled={busy} onClick={() => withRun((runId) => ({ type: "continue_run", run_id: runId, note: noteText.trim() || null }))}>{t("continue")}</button>
            </div>
            <div className="run-actions">
              <input
                value={noteText}
                placeholder={t("notePlaceholder")}
                aria-label={t("noteAction")}
                onChange={(event) => setNoteText(event.target.value)}
              />
              <button
                className="secondary-button"
                disabled={busy || !noteText.trim()}
                onClick={() => { withRun((runId) => ({ type: "note_run", run_id: runId, note: noteText.trim() })); setNoteText(""); }}
              >
                {t("noteAction")}
              </button>
              <input
                value={blockText}
                placeholder={t("blockPlaceholder")}
                aria-label={t("block")}
                onChange={(event) => setBlockText(event.target.value)}
              />
              <button
                className="secondary-button"
                disabled={busy || !blockText.trim()}
                onClick={() => { withRun((runId) => ({ type: "block_run", run_id: runId, reason: blockText.trim() })); setBlockText(""); }}
              >
                {t("block")}
              </button>
            </div>
            {active.events.length > 0 && (
              <>
                <h3>{t("runEvents")}</h3>
                <ul className="run-events">
                  {[...active.events].reverse().map((event, index) => (
                    <li key={`${event.created_at}-${index}`}>
                      <strong>{event.title}</strong>
                      {event.detail ? <span> — {event.detail}</span> : null}
                      <time>{event.created_at}</time>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </div>
        )}
      </section>
    </div>
  );
}

export type { RunStatus };
