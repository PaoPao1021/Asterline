import { useEffect, useState } from "react";
import type { DesktopLaunchOptions, RecentWorkspace } from "../bridge/types";
import type { Translate } from "../i18n";
import { ChevronIcon, FolderIcon, ShieldIcon, XIcon } from "./Icons";
import { useDialogFocus } from "./useDialogFocus";

interface ProjectPickerProps {
  recents: RecentWorkspace[];
  canClose: boolean;
  initialPath?: string;
  busy: boolean;
  t: Translate;
  onClose: () => void;
  onOpen: (workspace: string, options: DesktopLaunchOptions) => Promise<void>;
}

const nameOf = (path: string) => path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;

export function ProjectPicker({ recents, canClose, initialPath = "", busy, t, onClose, onOpen }: ProjectPickerProps) {
  const dialog = useDialogFocus<HTMLElement>();
  const [path, setPath] = useState(initialPath);
  const [error, setError] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [advanced, setAdvanced] = useState<DesktopLaunchOptions>({});
  useEffect(() => setPath(initialPath), [initialPath]);
  const submit = async (workspace = path) => {
    const value = workspace.trim();
    if (!value) { setError(t("projectRequired")); return; }
    setError("");
    await onOpen(value, advanced);
  };
  return (
    <div className="modal-backdrop project-backdrop">
      <section ref={dialog} className="project-modal" role="dialog" aria-modal="true" aria-labelledby="project-title" tabIndex={-1}>
        <header><img src="/asterline-mark.svg" alt="" /><div><h2 id="project-title">{t("projectTitle")}</h2><p>{t("projectBody")}</p></div>{canClose && <button className="icon-button" onClick={onClose} aria-label={t("close")}><XIcon /></button>}</header>
        {recents.length > 0 && <div className="project-recents"><strong>{t("recent")}</strong><div>{recents.slice(0, 5).map((item) => <button key={item.workspace} onClick={() => void submit(item.workspace)} disabled={busy}><span className="recent-folder"><FolderIcon /></span><span><strong>{item.name || nameOf(item.workspace)}</strong><small>{item.workspace}</small></span></button>)}</div></div>}
        <form onSubmit={(event) => { event.preventDefault(); void submit(); }}>
              <label><span>{t("workspacePath")}</span><div className={error ? "input-error" : ""}><FolderIcon size={17} /><input data-dialog-autofocus autoFocus value={path} onChange={(event) => setPath(event.target.value)} placeholder={t("pathPlaceholder")} /></div>{error && <small>{error}</small>}</label>

          <button type="button" className="advanced-launch-toggle" aria-expanded={advancedOpen} onClick={() => setAdvancedOpen((value) => !value)}>
            <ChevronIcon size={14} className={advancedOpen ? "rotated" : undefined} />
            {t("advancedLaunch")}
          </button>
          {advancedOpen && (
            <div className="advanced-launch" role="group" aria-label={t("advancedLaunch")}>
              <label><span>{t("teamPath")}</span><input value={advanced.team_path ?? ""} placeholder={t("teamPathPlaceholder")} onChange={(event) => setAdvanced((value) => ({ ...value, team_path: event.target.value || null }))} /></label>
              <label className="check"><input type="checkbox" checked={advanced.pick_team ?? false} onChange={(event) => setAdvanced((value) => ({ ...value, pick_team: event.target.checked }))} />{t("pickTeam")}</label>
              <label><span>{t("dbPath")}</span><input value={advanced.db_path ?? ""} placeholder={t("dbPathPlaceholder")} onChange={(event) => setAdvanced((value) => ({ ...value, db_path: event.target.value || null }))} /></label>
              <label className="check"><input type="checkbox" checked={advanced.restore ?? true} onChange={(event) => setAdvanced((value) => ({ ...value, restore: event.target.checked }))} />{t("restoreLast")}</label>
              <label className="check"><input type="checkbox" checked={advanced.auto_update ?? false} onChange={(event) => setAdvanced((value) => ({ ...value, auto_update: event.target.checked }))} />{t("autoUpdate")}</label>
              <label className="check">
                <input type="checkbox" checked={advanced.manual_approvals ?? false} onChange={(event) => setAdvanced((value) => ({ ...value, manual_approvals: event.target.checked }))} />
                <span>{t("manualApprovals")}<small>{t("manualApprovalsHint")}</small></span>
              </label>
              <label className="check">
                <input type="checkbox" checked={advanced.fake ?? false} onChange={(event) => setAdvanced((value) => ({ ...value, fake: event.target.checked }))} />
                <span>{t("fakeMode")}<small>{t("fakeHint")}</small></span>
              </label>
              <label className="check risk">
                <input
                  type="checkbox"
                  checked={advanced.debug ?? false}
                  onChange={(event) => {
                    const debug = event.target.checked;
                    setAdvanced((value) => ({ ...value, debug }));
                  }}
                />
                <span><ShieldIcon size={13} />{t("debugMode")}<small>{t("debugRisk")}</small></span>
              </label>
            </div>
          )}

          <div className="project-actions">{canClose && <button type="button" className="secondary-button" onClick={onClose}>{t("cancel")}</button>}<button type="submit" className="primary-button" disabled={busy}>{busy ? t("starting") : t("open")}</button></div>
        </form>
      </section>
    </div>
  );
}
