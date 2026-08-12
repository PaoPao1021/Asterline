import { useEffect, useState } from "react";
import type { RecentWorkspaceV1 } from "../bridge/types";
import type { Translate } from "../i18n";
import { FolderIcon, XIcon } from "./Icons";

interface ProjectPickerProps {
  recents: RecentWorkspaceV1[];
  canClose: boolean;
  initialPath?: string;
  busy: boolean;
  t: Translate;
  onClose: () => void;
  onOpen: (workspace: string) => Promise<void>;
}

const nameOf = (path: string) => path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;

export function ProjectPicker({ recents, canClose, initialPath = "", busy, t, onClose, onOpen }: ProjectPickerProps) {
  const [path, setPath] = useState(initialPath);
  const [error, setError] = useState("");
  useEffect(() => setPath(initialPath), [initialPath]);
  const submit = async (workspace = path) => {
    const value = workspace.trim();
    if (!value) { setError(t("projectRequired")); return; }
    setError("");
    await onOpen(value);
  };
  return (
    <div className="modal-backdrop project-backdrop">
      <section className="project-modal" role="dialog" aria-modal="true" aria-labelledby="project-title">
        <header><img src="/asterline-mark.svg" alt="" /><div><h2 id="project-title">{t("projectTitle")}</h2><p>{t("projectBody")}</p></div>{canClose && <button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon /></button>}</header>
        {recents.length > 0 && <div className="project-recents"><strong>{t("recent")}</strong><div>{recents.slice(0, 5).map((item) => <button key={item.workspace} onClick={() => void submit(item.workspace)} disabled={busy}><span className="recent-folder"><FolderIcon /></span><span><strong>{item.name || nameOf(item.workspace)}</strong><small>{item.workspace}</small></span></button>)}</div></div>}
        <form onSubmit={(event) => { event.preventDefault(); void submit(); }}>
          <label><span>{t("workspacePath")}</span><div className={error ? "input-error" : ""}><FolderIcon size={17} /><input autoFocus value={path} onChange={(event) => setPath(event.target.value)} placeholder={t("pathPlaceholder")} /></div>{error && <small>{error}</small>}</label>
          <div className="project-actions">{canClose && <button type="button" className="secondary-button" onClick={onClose}>{t("cancel")}</button>}<button type="submit" className="primary-button" disabled={busy}>{busy ? t("starting") : t("open")}</button></div>
        </form>
      </section>
    </div>
  );
}
