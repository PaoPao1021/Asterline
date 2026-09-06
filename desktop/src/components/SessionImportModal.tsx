import { useEffect, useMemo, useState } from "react";
import type { BackendKind, DesktopCommandV2, MemberSummaryV2, NativeSessionSummary } from "../bridge/types";
import type { Translate } from "../i18n";
import { Dropdown } from "./Dropdown";
import { XIcon } from "./Icons";

const BACKENDS: BackendKind[] = ["codex", "claude", "grok", "agy"];

interface SessionImportProps {
  workspace: string;
  members: MemberSummaryV2[];
  busy: boolean;
  t: Translate;
  onClose: () => void;
  dispatch: (command: DesktopCommandV2) => Promise<void>;
  listSessions: (backend: BackendKind, cwd: string) => Promise<NativeSessionSummary[]>;
}

/** Browse a backend's native sessions and import one into the chat. */
export function SessionImportModal({ workspace, members, busy, t, onClose, dispatch, listSessions }: SessionImportProps) {
  const [backend, setBackend] = useState<BackendKind>("codex");
  const [query, setQuery] = useState("");
  const [sessions, setSessions] = useState<NativeSessionSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [memberId, setMemberId] = useState<string>("");

  const load = async () => {
    setSessions(null);
    setError(null);
    try {
      setSessions(await listSessions(backend, workspace));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  useEffect(() => {
    void load();
  }, [backend, workspace]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return sessions ?? [];
    return (sessions ?? []).filter((session) =>
      session.session_id.toLowerCase().includes(needle) || session.preview.toLowerCase().includes(needle),
    );
  }, [sessions, query]);

  const importSession = (session: NativeSessionSummary) => {
    void dispatch({
      type: "import_session",
      member: memberId || null,
      session_id: session.session_id,
    });
    onClose();
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="session-import-modal" role="dialog" aria-modal="true" aria-label={t("importSession")}>
        <header>
          <h2 id="session-import-title">{t("importSession")}</h2>
          <button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon size={15} /></button>
        </header>
        <div className="session-import-controls">
          <Dropdown
            label={t("backend")}
            value={backend}
            onChange={(next) => setBackend(next as BackendKind)}
            options={BACKENDS.map((candidate) => ({ value: candidate, label: candidate }))}
          />
          <Dropdown
            label={t("target")}
            value={memberId}
            onChange={setMemberId}
            options={[
              { value: "", label: t("defaultTarget") },
              ...members.map((member) => ({ value: member.id, label: member.display_name, hint: member.backend })),
            ]}
          />
          <input
            value={query}
            placeholder={t("nativeSessionSearch")}
            aria-label={t("nativeSessionSearch")}
            onChange={(event) => setQuery(event.target.value)}
          />
          <button className="icon-button" aria-label={t("refreshUtility")} onClick={() => void load()} disabled={busy}>⟳</button>
        </div>
        {error && <p className="session-import-error">{error}</p>}
        <ul className="session-import-list">
          {filtered.map((session) => (
            <li key={`${session.backend}-${session.session_id}`}>
              <div>
                <code>{session.session_id}</code>
                <p>{session.preview || "—"}</p>
              </div>
              <button className="secondary-button" disabled={busy} onClick={() => importSession(session)}>{t("importIntoChat")}</button>
            </li>
          ))}
        </ul>
        {sessions !== null && sessions.length === 0 && <p className="session-import-error">{t("noNativeSessions")}</p>}
      </section>
    </div>
  );
}
