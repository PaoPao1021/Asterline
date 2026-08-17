import { useMemo, useState } from "react";
import type { BackendKind, DiffResultV1, LogEntryV1, SkillSummaryV1 } from "../bridge/types";
import type { Locale, Translate } from "../i18n";
import type { UtilityResource } from "../state";
import { CheckIcon, CopyIcon, FileIcon, RefreshIcon, SearchIcon, XIcon } from "./Icons";

export type UtilityKind = "logs" | "diff" | "skills" | "find";

interface UtilityDrawerProps {
  kind: UtilityKind;
  locale: Locale;
  t: Translate;
  logs: UtilityResource<LogEntryV1[]>;
  diff: UtilityResource<DiffResultV1 | null>;
  skills: UtilityResource<SkillSummaryV1[]>;
  timelineText: Array<{ id: string; title: string; text: string }>;
  onClose: () => void;
  onOpen: (kind: UtilityKind, query?: string) => void;
  onRefresh: (kind: Exclude<UtilityKind, "find">, filters?: { query?: string; level?: LogEntryV1["level"]; backend?: BackendKind }) => void;
  initialQuery?: string;
}

const backendOptions: Array<BackendKind | "all"> = ["all", "codex", "claude", "grok", "agy"];

function CopyButton({ value, label, t }: { value: string; label: string; t: Translate }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch { /* Clipboard permissions are optional. */ }
  };
  return <button className="utility-copy" onClick={() => void copy()} aria-label={label} title={label}>{copied ? <CheckIcon size={14} /> : <CopyIcon size={14} />}<span>{copied ? t("copied") : label}</span></button>;
}

export function UtilityDrawer({ kind, locale, t, logs, diff, skills, timelineText, onClose, onOpen, onRefresh, initialQuery }: UtilityDrawerProps) {
  const [query, setQuery] = useState(initialQuery ?? "");
  const [level, setLevel] = useState<LogEntryV1["level"] | "all">("all");
  const [backend, setBackend] = useState<BackendKind | "all">("all");
  const [findIndex, setFindIndex] = useState(0);
  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return [];
    return timelineText.filter((item) => `${item.title} ${item.text}`.toLowerCase().includes(needle));
  }, [query, timelineText]);

  const refresh = () => onRefresh(kind === "find" ? "logs" : kind, {
    query: query || undefined,
    level: kind === "logs" && level !== "all" ? level : undefined,
    backend: kind === "skills" && backend !== "all" ? backend : undefined,
  });

  return <aside className="utility-drawer" aria-label={t("utilities")}>
    <div className="utility-head"><div><small>{t("utilities")}</small><strong>{t(kind)}</strong></div><button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon /></button></div>
    <div className="utility-tabs" role="tablist">
      {(["logs", "diff", "skills", "find"] as UtilityKind[]).map((tab) => <button key={tab} role="tab" aria-selected={kind === tab} className={kind === tab ? "active" : ""} onClick={() => onOpen(tab)}>{t(tab)}</button>)}
    </div>
    {kind === "logs" && <>
      <div className="utility-filters"><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("filterLogs")} aria-label={t("filterLogs")} /><select value={level} onChange={(event) => setLevel(event.target.value as typeof level)} aria-label={t("logLevel")}><option value="all">{t("allLevels")}</option>{["debug", "info", "warn", "error"].map((item) => <option key={item} value={item}>{item}</option>)}</select><button onClick={refresh} aria-label={t("refreshUtility")}><RefreshIcon size={15} /></button></div>
      <div className="utility-meta">{logs.value.length} · {logs.truncated ? t("truncated") : t("complete")}</div>
      <div className="utility-scroll">{logs.value.length ? logs.value.map((entry, index) => <article className={`log-row log-${entry.level}`} key={`${entry.source}-${index}`}><span>{entry.level}</span><strong>{entry.source}</strong><p>{entry.message}</p></article>) : <div className="utility-empty">{logs.status === "loading" ? t("loading") : t("noLogs")}</div>}</div>
    </>}
    {kind === "diff" && <>
      <div className="utility-toolbar"><span>{diff.value?.file_count ?? 0} {t("files")}</span><button onClick={refresh}><RefreshIcon size={15} />{t("refreshUtility")}</button>{diff.value && <CopyButton value={diff.value.text} label={t("copyDiff")} t={t} />}</div>
      <div className="utility-meta">{diff.value?.truncated ? t("truncated") : t("complete")}</div>
      <pre className="utility-diff">{diff.value?.text || (diff.status === "loading" ? t("loading") : t("noDiff"))}</pre>
    </>}
    {kind === "skills" && <>
      <div className="utility-filters"><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("filterSkills")} aria-label={t("filterSkills")} /><select value={backend} onChange={(event) => setBackend(event.target.value as typeof backend)} aria-label={t("backend")}><option value="all">{t("allBackends")}</option>{backendOptions.slice(1).map((item) => <option key={item} value={item}>{item}</option>)}</select><button onClick={refresh} aria-label={t("refreshUtility")}><RefreshIcon size={15} /></button></div>
      <div className="utility-meta">{skills.value.length} · {skills.truncated ? t("truncated") : t("complete")}</div>
      <div className="utility-scroll">{skills.value.length ? skills.value.map((skill) => <article className="skill-row" key={`${skill.backend}-${skill.invocation}`}><div><strong>{skill.name}</strong><span>{skill.backend}</span><p>{skill.description || t("noDescription")}</p></div><CopyButton value={skill.invocation} label={t("copyInvocation")} t={t} /></article>) : <div className="utility-empty">{skills.status === "loading" ? t("loading") : t("noSkills")}</div>}</div>
    </>}
    {kind === "find" && <>
      <div className="utility-filters"><SearchIcon size={15} /><input autoFocus value={query} onChange={(event) => { setQuery(event.target.value); setFindIndex(0); }} placeholder={t("findPlaceholder")} aria-label={t("findPlaceholder")} /><button onClick={onClose} aria-label={t("close")}><XIcon size={15} /></button></div>
      <div className="utility-meta">{matches.length} {t("matches")}</div>
      <div className="utility-scroll">{matches.length ? matches.map((item, index) => <button className={`find-row ${index === findIndex ? "active" : ""}`} key={item.id} onClick={() => setFindIndex(index)}><FileIcon size={14} /><span><strong>{item.title}</strong><small>{item.text}</small></span></button>) : <div className="utility-empty">{query ? t("noMatches") : t("findHint")}</div>}</div>
      {matches.length > 1 && <div className="utility-toolbar"><button onClick={() => setFindIndex((findIndex + matches.length - 1) % matches.length)}>{t("previous")}</button><button onClick={() => setFindIndex((findIndex + 1) % matches.length)}>{t("next")}</button></div>}
    </>}
  </aside>;
}
