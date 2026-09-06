import { useMemo, useState } from "react";
import type { DesktopCommandV2, MemberSummaryV2, RunSummaryV2 } from "../bridge/types";
import type { Locale, Translate } from "../i18n";
import { CheckIcon, ChevronIcon, MoreIcon, PanelRightIcon, RefreshIcon, RunIcon, UsersIcon } from "./Icons";

interface InspectorProps {
  open: boolean;
  members: MemberSummaryV2[];
  runs: RunSummaryV2[];
  locale: Locale;
  t: Translate;
  onToggle: () => void;
  dispatch: (command: DesktopCommandV2) => Promise<void>;
  onAttach: (member: string) => Promise<void>;
}

const initials = (name: string) => name.split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase();

function MemberCard({ member, t, onAttach }: { member: MemberSummaryV2; t: Translate; onAttach: (member: string) => Promise<void> }) {
  return (
    <div className="member-card">
      <div className={`member-avatar backend-${member.backend}`}>{initials(member.display_name)}</div>
      <div className="member-card-copy">
        <div><strong>{member.display_name}</strong><span className={`status-dot status-${member.status}`} /></div>
        <p>{member.role}</p>
        <small><span className={`backend-label backend-${member.backend}`}>{member.backend}</span>{member.model || member.effort || "default"}</small>
      </div>
      <button className="subtle-icon-button member-menu" title={t("attach")} aria-label={`${t("attach")}: ${member.display_name}`} onClick={() => void onAttach(member.id)}><MoreIcon size={16} /></button>
      <div className="member-state"><span className={`state-pill status-${member.status}`}>{t(member.status)}</span></div>
    </div>
  );
}

function RunCard({ run, t, dispatch }: { run: RunSummaryV2; t: Translate; dispatch: (command: DesktopCommandV2) => Promise<void> }) {
  const [open, setOpen] = useState(true);
  const done = run.steps.filter(({ status }) => status === "done").length;
  const progress = run.steps.length ? Math.round((done / run.steps.length) * 100) : 0;
  return (
    <div className="run-card">
      <button className="run-card-heading" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <span className={`run-status-icon run-${run.status}`}><RunIcon size={15} /></span>
        <span><strong>{run.goal}</strong><small>#{run.id} · {t(run.status)}</small></span>
        <ChevronIcon className={open ? "rotate-90" : ""} size={16} />
      </button>
      <div className="run-progress"><span style={{ width: `${progress}%` }} /></div>
      <div className="run-progress-label"><span>{t("runSteps", { done, total: run.steps.length })}</span><b>{progress}%</b></div>
      {open && (
        <>
          <ol className="run-step-list">
            {run.steps.map((step) => (
              <li key={step.number} className={`step-${step.status}`}>
                <span>{step.status === "done" ? <CheckIcon size={12} /> : step.number}</span>
                <div><strong>{step.title}</strong>{step.owner && <small>@{step.owner}</small>}</div>
              </li>
            ))}
          </ol>
          <div className="run-actions">
            <button onClick={() => void dispatch({ type: "continue_run", run_id: run.id, note: null })}>{t("continue")}</button>
            <button onClick={() => void dispatch({ type: "verify_run", run_id: run.id, command: null })}><RefreshIcon size={13} />{t("verify")}</button>
          </div>
        </>
      )}
    </div>
  );
}

export function Inspector({ open, members, runs, t, onToggle, dispatch, onAttach }: InspectorProps) {
  const [tab, setTab] = useState<"members" | "runs">("members");
  const running = useMemo(() => members.filter(({ status }) => status === "running" || status === "queued").length, [members]);
  return (
    <aside className={`inspector ${open ? "is-open" : "is-closed"}`} aria-label={t("inspector")}>
      <div className="inspector-head">
        <div className="inspector-tabs" role="tablist">
          <button role="tab" aria-selected={tab === "members"} className={tab === "members" ? "active" : ""} onClick={() => setTab("members")}><UsersIcon size={16} />{t("members")}<span>{members.length}</span></button>
          <button role="tab" aria-selected={tab === "runs"} className={tab === "runs" ? "active" : ""} onClick={() => setTab("runs")}><RunIcon size={16} />{t("runs")}<span>{runs.length}</span></button>
        </div>
        <button className="icon-button inspector-toggle" onClick={onToggle} aria-label={t("details")}><PanelRightIcon /></button>
      </div>
      <div className="inspector-summary">
        <span><i className={running ? "pulse" : ""} />{running ? t("running") : t("connected")}</span>
        <small>{members.length} {t("members").toLowerCase()}</small>
      </div>
      <div className="inspector-scroll">
        {tab === "members" && <div className="member-list">{members.map((member) => <MemberCard key={member.id} member={member} t={t} onAttach={onAttach} />)}</div>}
        {tab === "runs" && <div className="run-list">{runs.length ? runs.map((run) => <RunCard key={run.id} run={run} t={t} dispatch={dispatch} />) : <div className="panel-empty"><RunIcon /><p>{t("noRuns")}</p></div>}</div>}
      </div>
    </aside>
  );
}
