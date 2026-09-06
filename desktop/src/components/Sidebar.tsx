import { useMemo, useState } from "react";
import type { ConversationSummaryV2, RecentWorkspace } from "../bridge/types";
import type { Locale, Translate } from "../i18n";
import { ChatIcon, DownloadIcon, FolderIcon, HistoryIcon, PanelLeftIcon, PlusIcon, SearchIcon, SettingsIcon } from "./Icons";

interface SidebarProps {
  open: boolean;
  locale: Locale;
  workspace?: string | null;
  teamName?: string | null;
  conversations: ConversationSummaryV2[];
  recents: RecentWorkspace[];
  activeConversation?: number | null;
  t: Translate;
  onToggle: () => void;
  onNew: () => void;
  onOpenProject: () => void;
  onOpenRecent: (workspace: string) => void;
  onResume: (id: number) => void;
  onSettings: () => void;
  onImportSession: () => void;
  onExportSession: () => void;
}

const projectName = (path?: string | null) => path?.split(/[\\/]/).filter(Boolean).at(-1) ?? "Workspace";

function relativeDate(value: string, locale: Locale, t: Translate): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const today = new Date();
  const day = 86_400_000;
  const startToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
  const startDate = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const delta = Math.round((startToday - startDate) / day);
  if (delta === 0) return t("today");
  if (delta === 1) return t("yesterday");
  return new Intl.DateTimeFormat(locale, { month: "short", day: "numeric" }).format(date);
}

export function Sidebar({
  open,
  locale,
  workspace,
  teamName,
  conversations,
  recents,
  activeConversation,
  t,
  onToggle,
  onNew,
  onOpenProject,
  onOpenRecent,
  onResume,
  onSettings,
  onImportSession,
  onExportSession,
}: SidebarProps) {
  const [search, setSearch] = useState("");
  const filteredConversations = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return conversations;
    return conversations.filter((conversation) =>
      conversation.preview.toLowerCase().includes(needle) || String(conversation.id).includes(needle),
    );
  }, [conversations, search]);

  return (
    <aside className={`sidebar ${open ? "is-open" : "is-closed"}`} aria-label={t("menu")}>
      <div className="brand-row">
        <img className="brand-mark" src="/asterline-mark.svg" alt="" />
        <div className="brand-copy">
          <span className="brand-overline" aria-hidden="true">COLLECTIVE / 01</span>
          <strong>{t("appName")}</strong>
          <small>{t("desktop")}</small>
        </div>
        <button className="icon-button sidebar-toggle" onClick={onToggle} title={t("collapse")} aria-label={t("collapse")}>
          <PanelLeftIcon />
        </button>
      </div>

      <button className="new-chat-button" onClick={onNew}>
        <PlusIcon size={17} />
        <span>{t("newChat")}</span>
        <kbd>CTRL N</kbd>
      </button>

      <div className="workspace-card">
        <div className="workspace-icon"><FolderIcon size={18} /></div>
        <div className="workspace-copy">
          <span>{teamName || projectName(workspace)}</span>
          <small title={workspace ?? ""}>{workspace || t("workspace")}</small>
        </div>
        <button className="subtle-icon-button" onClick={onOpenProject} aria-label={t("openProject")} title={t("openProject")}><PlusIcon size={15} /></button>
      </div>

      <nav className="sidebar-scroll">
        <section className="sidebar-section">
          <div className="section-heading"><span>{t("conversations")}</span><HistoryIcon size={14} /></div>
          <label className="sidebar-search">
            <SearchIcon size={13} />
            <input
              value={search}
              placeholder={t("sessionSearch")}
              aria-label={t("sessionSearch")}
              onChange={(event) => setSearch(event.target.value)}
            />
          </label>
          <div className="conversation-list">
            {conversations.length === 0 && <div className="sidebar-empty">{t("noConversations")}</div>}
            {filteredConversations.length === 0 && conversations.length > 0 && <div className="sidebar-empty">{t("noHistoryMatches")}</div>}
            {filteredConversations.map((conversation) => (
              <button
                key={conversation.id}
                className={`conversation-item ${activeConversation === conversation.id ? "active" : ""}`}
                onClick={() => onResume(conversation.id)}
              >
                <ChatIcon size={15} />
                <span className="conversation-copy">
                  <strong>{conversation.preview}</strong>
                  <small>{relativeDate(conversation.created_at, locale, t)} · {t("messages", { count: conversation.message_count })}</small>
                </span>
              </button>
            ))}
          </div>
          <div className="sidebar-session-tools">
            <button className="sidebar-tool" onClick={onImportSession}><PlusIcon size={13} /><span>{t("importSession")}</span></button>
            <button className="sidebar-tool" onClick={onExportSession}><DownloadIcon size={13} /><span>{t("exportToClaude")}</span></button>
          </div>
        </section>

        {recents.length > 1 && (
          <section className="sidebar-section recent-projects">
            <div className="section-heading"><span>{t("recent")}</span></div>
            {recents.filter((item) => item.workspace !== workspace).slice(0, 4).map((item) => (
              <button key={item.workspace} className="recent-item" onClick={() => onOpenRecent(item.workspace)} title={item.workspace}>
                <FolderIcon size={14} />
                <span>{item.name || projectName(item.workspace)}</span>
              </button>
            ))}
          </section>
        )}
      </nav>

      <div className="sidebar-footer">
        <button className="sidebar-settings" onClick={onSettings}>
          <SettingsIcon size={17} />
          <span>{t("settings")}</span>
        </button>
        <div className="profile-status"><span className="profile-dot">A</span><small>LOCAL / READY</small></div>
      </div>
    </aside>
  );
}
