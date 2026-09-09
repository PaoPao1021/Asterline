import { Fragment, useMemo, useState } from "react";
import type { MemberSummaryV2, TimelineItemV2 } from "../bridge/types";
import { renderMarkdown } from "../markdown";
import type { Locale, Translate } from "../i18n";
import { AlertIcon, CheckIcon, ChevronIcon, CopyIcon, FileIcon, ImageIcon, RouteIcon, SparkIcon, ToolIcon, XIcon } from "./Icons";

interface TimelineProps {
  items: TimelineItemV2[];
  members: MemberSummaryV2[];
  locale: Locale;
  t: Translate;
  resolvingRoute?: string | null;
  onResolvePausedRoute: (itemId: string, resume: boolean) => Promise<void>;
}

const initials = (name: string) => name.split(/\s+/).map((part) => part[0]).join("").slice(0, 2).toUpperCase();

function formatTime(value: string | null | undefined, locale: Locale): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }).format(date);
}

function DetailToggle({ detail, t }: { detail: string; t: Translate }) {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  const copy = async (event: React.MouseEvent) => {
    event.stopPropagation();
    try {
      await navigator.clipboard.writeText(detail);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      /* clipboard write may fail gracefully */
    }
  };

  return (
    <div className="detail-toggle">
      <div className="detail-toggle-bar">
        <button onClick={() => setOpen((value) => !value)} aria-expanded={open}>
          <ChevronIcon size={14} className={open ? "rotate-90" : ""} /> {open ? t("hideDetail") : t("showDetail")}
        </button>
        {open && (
          <button className="detail-copy-btn" onClick={copy} title={t("copyDetail")} aria-label={t("copyDetail")}>
            {copied ? <CheckIcon size={12} /> : <CopyIcon size={12} />}
            <span>{copied ? t("copied") : t("copyDetail")}</span>
          </button>
        )}
      </div>
      {open && <pre className="detail-code-block">{detail}</pre>}
    </div>
  );
}

function TimelineCard({ item, members, locale, t, resolvingRoute, actionablePausedRoute, onResolvePausedRoute }: { item: TimelineItemV2; members: MemberSummaryV2[]; locale: Locale; t: Translate; resolvingRoute?: string | null; actionablePausedRoute?: string; onResolvePausedRoute: (itemId: string, resume: boolean) => Promise<void> }) {
  const member = members.find(({ id }) => id === item.member);
  const name = item.display_name || member?.display_name || item.member || "Asterline";
  const backend = item.backend || member?.backend;

  if (item.kind === "notice") {
    return <div className="timeline-notice"><SparkIcon size={15} /><span>{item.text}</span></div>;
  }
  if (item.kind === "error") {
    return <div className="timeline-notice error"><AlertIcon size={15} /><span>{item.text || item.detail}</span></div>;
  }
  if (item.kind === "route") {
    return (
      <div className="route-card">
        <RouteIcon size={16} />
        <div><strong>{name}</strong> <span>{t("routedTo", { names: item.to?.join(", ") || "—" })}</span><p>{item.text}</p></div>
        <time>{formatTime(item.timestamp, locale)}</time>
      </div>
    );
  }
  if (item.kind === "route_paused") {
    const canResolve = actionablePausedRoute === item.id;
    return (
      <div className="paused-route-card">
        <span className="paused-route-icon"><AlertIcon size={17} /></span>
        <div className="paused-route-copy">
          <strong>{t("routePaused")}</strong>
          <p>{item.text}</p>
          <small>{name} · {t("routedTo", { names: item.to?.join(", ") || "—" })} · {t("queuedRoutes", { count: Number(item.detail) || 1 })}</small>
        </div>
        <div className="paused-route-actions">
          <button disabled={!canResolve || resolvingRoute === item.id} onClick={() => void onResolvePausedRoute(item.id, false)}>{t("dropRelay")}</button>
          <button className="primary" disabled={!canResolve || resolvingRoute === item.id} onClick={() => void onResolvePausedRoute(item.id, true)}>{t("resumeRelay")}</button>
        </div>
      </div>
    );
  }
  if (item.kind === "tool") {
    return (
      <div className={`tool-card ${item.ok === false ? "failed" : ""}`}>
        <div className="tool-card-head">
          <span className="tool-icon"><ToolIcon size={15} /></span>
          <div><strong>{item.title || t("tool")}</strong><small>{name}</small></div>
          {item.truncated && <span className="truncated-pill">{t("truncated")}</span>}
          <span className={`result-icon ${item.ok === false ? "bad" : "good"}`}>{item.ok === false ? <XIcon size={14} /> : <CheckIcon size={14} />}</span>
        </div>
        {item.detail && <DetailToggle detail={item.detail} t={t} />}
      </div>
    );
  }
  if (item.kind === "diff") {
    return (
      <div className="tool-card diff-card">
        <div className="tool-card-head"><span className="tool-icon"><FileIcon size={15} /></span><div><strong>{t("filesChanged", { count: item.files?.length ?? 0 })}</strong><small>{name}</small></div></div>
        <div className="file-pills">{item.files?.map(([path, action]) => <span key={path}><b>{action}</b>{path}</span>)}</div>
      </div>
    );
  }
  if (item.kind === "reasoning") {
    return <div className="reasoning-card"><SparkIcon size={15} /><div><strong>{t("reasoning")} · {name}</strong><p>{item.text}</p></div></div>;
  }
  if (item.kind === "verdict") {
    return <div className={`verdict-card ${item.ok ? "approved" : "changes"}`}>{item.ok ? <CheckIcon /> : <AlertIcon />}<div><strong>{item.ok ? t("approve") : t("review")}</strong><p>{item.text}</p></div></div>;
  }

  const isUser = item.kind === "user";
  const plainLines = useMemo(() => (item.text ?? "").split("\n"), [item.text]);
  const html = useMemo(
    () => (!isUser && item.text ? renderMarkdown(item.text) : ""),
    [isUser, item.text],
  );
  return (
    <article className={`message-row ${isUser ? "is-user" : "is-agent"}`}>
      {!isUser && <div className={`member-avatar backend-${backend ?? "system"}`}>{initials(name)}</div>}
      <div className="message-content">
        <div className="message-meta">
          <strong>{isUser ? "You" : name}</strong>
          {!isUser && backend && <span className={`backend-label backend-${backend}`}>{backend}</span>}
          <time>{formatTime(item.timestamp, locale)}</time>
          {item.truncated && <span className="truncated-pill" title={t("truncated")}>{t("truncated")}</span>}
        </div>
        {isUser && item.attachments && item.attachments.length > 0 && (
          <div className="message-attachments" aria-label={t("attachments")}>
            {item.attachments.map((label) => <span key={label} className="attachment-chip"><ImageIcon size={11} />{label}</span>)}
          </div>
        )}
        <div className={`message-bubble ${item.streaming ? "is-streaming" : ""}`}>
          {isUser
            ? plainLines.map((line, index) => <Fragment key={index}>{line}{index < plainLines.length - 1 && <br />}</Fragment>)
            : <div className="markdown-body" dangerouslySetInnerHTML={{ __html: html }} />}
          {item.streaming && <span className="streaming-caret" />}
        </div>
      </div>
    </article>
  );
}

export function Timeline({ items, members, locale, t, resolvingRoute, onResolvePausedRoute }: TimelineProps) {
  if (items.length === 0) {
    return (
      <div className="timeline-empty">
        <div className="empty-content-box">
          <img src="/asterline-mark.svg" alt="" />
          <h2>{t("timelineEmptyTitle")}</h2>
          <p>{t("timelineEmptyBody")}</p>
        </div>
      </div>
    );
  }

  const actionablePausedRoute = items.find(({ kind }) => kind === "route_paused")?.id;
  return (
    <div className="timeline-list" aria-live="polite">
      {items.map((item) => <TimelineCard key={item.id} item={item} members={members} locale={locale} t={t} resolvingRoute={resolvingRoute} actionablePausedRoute={actionablePausedRoute} onResolvePausedRoute={onResolvePausedRoute} />)}
      <div className="timeline-end" />
    </div>
  );
}
