import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { getDesktopClient } from "./bridge/client";
import type {
  BackendKind,
  CommandSpec,
  ComposerAction,
  DesktopCommandV2,
  DesktopEventV2,
  DesktopLaunchOptions,
  DesktopUpdate,
  LogEntryV2,
  MessageTargetV2,
  StagedAttachment,
  TeamSettingsV2,
  TerminalMode,
} from "./bridge/types";
import { ApprovalQueue } from "./components/ApprovalQueue";
import { CommandPalette } from "./components/CommandPalette";
import { Composer } from "./components/Composer";
import { ChevronIcon, CommandIcon, FolderIcon, GlobeIcon, MoonIcon, PanelLeftIcon, PanelRightIcon, PauseIcon, PlayIcon, RefreshIcon, SettingsIcon, SunIcon, XIcon } from "./components/Icons";
import { Inspector } from "./components/Inspector";
import { ModePanel } from "./components/ModePanel";
import { ProjectPicker } from "./components/ProjectPicker";
import { RunsPanel } from "./components/RunsPanel";
import { SessionImportModal } from "./components/SessionImportModal";
import { SettingsModal } from "./components/SettingsModal";
import { Sidebar } from "./components/Sidebar";
import { Timeline } from "./components/Timeline";
import { UtilityDrawer, type UtilityKind } from "./components/UtilityDrawer";
import { createTranslator, detectLocale, type Locale } from "./i18n";
import { desktopReducer, errorMessage, initialDesktopState } from "./state";

type Theme = "light" | "dark";

const SIDEBAR_BREAKPOINT = 860;
const INSPECTOR_BREAKPOINT = 1200;

function panelStartsOpen(breakpoint: number): boolean {
  return typeof window === "undefined" || window.innerWidth > breakpoint;
}

function initialTheme(): Theme {
  const saved = localStorage.getItem("asterline.theme");
  if (saved === "light" || saved === "dark") return saved;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function fallbackTeam(workspace = ""): TeamSettingsV2 {
  return {
    name: workspace.split(/[\\/]/).filter(Boolean).at(-1) || "New team",
    workspace,
    default_target: { type: "member", member: "builder" },
    max_auto_relays: 6,
    members: [{
      id: "builder",
      display_name: "Builder",
      backend: "codex",
      role: "implementation",
      cwd: null,
      model: null,
      system_prompt: null,
      sandbox: "workspace-write",
      permission_mode: null,
      allowed_tools: [],
      session_policy: "resume",
      session_id: null,
      effort: "high",
    }],
    approvals: { manual: false, gate: null, keywords: {}, apply_to: null },
    modes: {},
  };
}

export function App() {
  const client = useMemo(() => getDesktopClient(), []);
  const [state, reduce] = useReducer(desktopReducer, initialDesktopState);
  const [recents, setRecents] = useState<Array<{ workspace: string; name?: string | null; last_opened_at?: string | null }>>([]);
  const [locale, setLocale] = useState<Locale>(detectLocale);
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [sidebarOpen, setSidebarOpen] = useState(() => panelStartsOpen(SIDEBAR_BREAKPOINT));
  const [inspectorOpen, setInspectorOpen] = useState(() => panelStartsOpen(INSPECTOR_BREAKPOINT));
  const [projectPicker, setProjectPicker] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [projectBusy, setProjectBusy] = useState(false);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [availableUpdate, setAvailableUpdate] = useState<DesktopUpdate | null>(null);
  const [approvalBusy, setApprovalBusy] = useState<number | null>(null);
  const [resolvingRoute, setResolvingRoute] = useState<string | null>(null);
  const [resolvedRoutes, setResolvedRoutes] = useState<Set<string>>(() => new Set());
  const [target, setTarget] = useState("default");
  const [activeConversation, setActiveConversation] = useState<number | null>(null);
  const [toast, setToast] = useState<{ id: number; text: string; tone?: "error" | "success" } | null>(null);
  const [utilityKind, setUtilityKind] = useState<UtilityKind | null>(null);
  const [utilityQuery, setUtilityQuery] = useState<string | undefined>(undefined);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteExecutable, setPaletteExecutable] = useState(true);
  const [catalog, setCatalog] = useState<CommandSpec[]>([]);
  const [modePanelOpen, setModePanelOpen] = useState(false);
  const [runsOpen, setRunsOpen] = useState(false);
  const [sessionImportOpen, setSessionImportOpen] = useState(false);
  const [launchOptions, setLaunchOptions] = useState<DesktopLaunchOptions>({});
  const [composerInjection, setComposerInjection] = useState<{ text: string; nonce: number } | null>(null);
  const utilityRequest = useRef(0);
  const pendingEvents = useRef<DesktopEventV2[]>([]);
  const hasSnapshot = useRef(false);
  const timelineScroll = useRef<HTMLDivElement>(null);
  const pendingTeamSettings = useRef<string | null>(null);
  const diagnosticsChecked = useRef(false);
  const t = useMemo(() => createTranslator(locale), [locale]);
  const snapshot = state.snapshot;

  const notify = useCallback((text: string, tone?: "error" | "success") => {
    const next = { id: Date.now(), text, tone };
    setToast(next);
    window.setTimeout(() => setToast((current) => current?.id === next.id ? null : current), 3200);
  }, []);

  const refreshRecents = useCallback(async () => {
    try { setRecents(await client.listRecentWorkspaces()); } catch { /* app config is non-critical */ }
  }, [client]);

  const bootstrap = useCallback(async (workspace?: string, options?: DesktopLaunchOptions) => {
    reduce({ type: "loading" });
    try {
      const next = await client.bootstrapDesktop(workspace, options);
      hasSnapshot.current = true;
      setResolvedRoutes(new Set());
      reduce({ type: "snapshot", snapshot: next });
      pendingEvents.current.sort((a, b) => a.sequence - b.sequence).forEach((packet) => reduce({ type: "event", packet }));
      pendingEvents.current = [];
      setProjectPicker(!next.workspace || next.phase === "uninitialized");
      if (next.phase === "needs_team_setup") setSettingsOpen(true);
      if (next.phase === "ready") {
        try { await client.dispatchDesktopCommand({ type: "request_resume" }); }
        catch (error) { notify(errorMessage(error), "error"); }
      }
      await refreshRecents();
      if (!diagnosticsChecked.current) {
        diagnosticsChecked.current = true;
        try {
          const diagnostics = await client.getDesktopDiagnosticsStatus();
          if (diagnostics.previous_unclean_exit) notify(t("uncleanExit"), "error");
        } catch { /* Diagnostics are best-effort and never block workspace startup. */ }
      }
    } catch (error) {
      reduce({ type: "error", error: errorMessage(error) });
    }
  }, [client, notify, refreshRecents, t]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void client.listenRuntimeEvents((packet) => {
      if (disposed) return;
      // A queued prompt pulled back for editing lands in the composer draft.
      if (packet.event.type === "queued_prompt_returned") {
        setComposerInjection({ text: packet.event.body, nonce: packet.sequence });
      }
      if (!hasSnapshot.current) pendingEvents.current.push(packet);
      else reduce({ type: "event", packet });
    }).then((stop) => {
      if (disposed) stop();
      else {
        unlisten = stop;
        void bootstrap();
      }
    }).catch((error) => reduce({ type: "error", error: errorMessage(error) }));
    return () => { disposed = true; unlisten?.(); };
  }, [bootstrap, client]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("asterline.theme", theme);
    const meta = document.querySelector('meta[name="theme-color"]');
    meta?.setAttribute("content", theme === "dark" ? "#0d1117" : "#ffffff");
  }, [theme]);

  useEffect(() => {
    document.documentElement.lang = locale;
    localStorage.setItem("asterline.locale", locale);
  }, [locale]);

  useEffect(() => {
    let sidebarIsCompact = window.innerWidth <= SIDEBAR_BREAKPOINT;
    let inspectorIsOverlay = window.innerWidth <= INSPECTOR_BREAKPOINT;
    const handleResize = () => {
      const nextSidebarIsCompact = window.innerWidth <= SIDEBAR_BREAKPOINT;
      const nextInspectorIsOverlay = window.innerWidth <= INSPECTOR_BREAKPOINT;
      if (nextSidebarIsCompact !== sidebarIsCompact) setSidebarOpen(!nextSidebarIsCompact);
      if (nextInspectorIsOverlay !== inspectorIsOverlay) setInspectorOpen(!nextInspectorIsOverlay);
      sidebarIsCompact = nextSidebarIsCompact;
      inspectorIsOverlay = nextInspectorIsOverlay;
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    const element = timelineScroll.current;
    if (!element) return;
    const nearBottom = element.scrollHeight - element.scrollTop - element.clientHeight < 240;
    if (nearBottom) requestAnimationFrame(() => element.scrollTo({ top: element.scrollHeight, behavior: "smooth" }));
  }, [snapshot?.timeline.length, snapshot?.timeline.at(-1)?.text]);

  useEffect(() => {
    if (pendingTeamSettings.current && snapshot?.team && JSON.stringify(snapshot.team) === pendingTeamSettings.current) {
      pendingTeamSettings.current = null;
      notify(t("settingsConfirmed"), "success");
    }
  }, [notify, snapshot?.team, t]);

  // Load the shared command catalog once for palettes and /help.
  useEffect(() => {
    let alive = true;
    client.commandCatalog().then((value) => { if (alive) setCatalog(value); }).catch(() => undefined);
    return () => { alive = false; };
  }, [client]);

  // Ctrl+K opens the executable command palette.
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteExecutable(true);
        setPaletteOpen(true);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const dispatch = useCallback(async (command: DesktopCommandV2) => {
    try { await client.dispatchDesktopCommand(command); }
    catch (error) { notify(errorMessage(error), "error"); throw error; }
  }, [client, notify]);

  const requestUtility = useCallback(async (kind: Exclude<UtilityKind, "find">, filters?: { query?: string; level?: LogEntryV2["level"]; backend?: BackendKind; member?: string | null }) => {
    const requestId = ++utilityRequest.current;
    reduce({ type: "utility_request", utility: kind, requestId });
    const command: DesktopCommandV2 = kind === "logs"
      ? { type: "request_logs", request_id: requestId, query: filters?.query ?? null, level: filters?.level ?? null, member: filters?.member ?? null, limit: 400 }
      : kind === "diff"
        ? { type: "request_diff", request_id: requestId }
        : { type: "request_skills", request_id: requestId, query: filters?.query ?? null, backend: filters?.backend ?? null, limit: 512 };
    try { await dispatch(command); }
    catch (error) { reduce({ type: "utility_error", utility: kind, requestId, error: errorMessage(error) }); }
  }, [dispatch]);

  const openUtility = useCallback((kind: UtilityKind, initialQuery?: string) => {
    setUtilityKind(kind);
    setUtilityQuery(initialQuery);
    if (kind !== "find") void requestUtility(kind);
  }, [requestUtility]);

  const openWorkspace = async (workspace: string, options: DesktopLaunchOptions = {}) => {
    setProjectBusy(true);
    try {
      const launch = options;
      setLaunchOptions(launch);
      setUtilityKind(null);
      setUtilityQuery(undefined);
      if (snapshot?.workspace && snapshot.workspace !== workspace) await client.shutdownDesktop();
      hasSnapshot.current = false;
      pendingEvents.current = [];
      await bootstrap(workspace, launch);
      setProjectPicker(false);
    } finally { setProjectBusy(false); }
  };

  const changeMode = async (mode: TerminalMode) => {
    if (mode === snapshot?.mode) return;
    await dispatch({ type: "set_mode", mode });
  };

  const selectedTarget = (value: string): MessageTargetV2 =>
    value === "all" ? { type: "all" } : value === "default" ? { type: "default" } : { type: "member", member: value };

  const decision = useCallback(async (id: number, decisionValue: "approve" | "reject") => {
    setApprovalBusy(id);
    try {
      await dispatch({ type: "approve", id, decision: decisionValue });
      notify(t(decisionValue === "approve" ? "approvalGranted" : "approvalRejected"), "success");
    } finally { setApprovalBusy(null); }
  }, [dispatch, notify, t]);

  const approveFirst = async (decisionValue: "approve" | "reject") => {
    const pending = snapshot?.approvals[0];
    if (!pending) {
      notify(t("noMatches"), "error");
      return;
    }
    await decision(pending.id, decisionValue);
  };

  const submit = async (text: string, attachments: StagedAttachment[] = []): Promise<boolean> => {
    let action: ComposerAction;
    try {
      action = await client.parseComposerText(text);
    } catch (error) {
      notify(errorMessage(error), "error");
      return false;
    }
    const send = async (body: string, messageTarget: MessageTargetV2, staged: StagedAttachment[]): Promise<boolean> => {
      try {
        await dispatch({ type: "user_message", target: messageTarget, body, attachments: staged.map(({ token }) => token) });
        return true;
      } catch {
        return false;
      }
    };
    switch (action.type) {
      case "empty":
        return attachments.length > 0 ? send("", { type: "default" }, attachments) : false;
      case "needs_target":
        // Bare text follows the GUI target selector — the GUI affordance for
        // the TUI's remembered last target.
        return send(text, selectedTarget(target), attachments);
      case "invalid":
        notify(action.message, "error");
        return false;
      case "help":
        setPaletteExecutable(false);
        setPaletteOpen(true);
        return true;
      case "exit":
        if (window.confirm(t("exitConfirm"))) {
          try { await client.exitDesktop(); } catch (error) { notify(errorMessage(error), "error"); }
        }
        return true;
      case "attach":
        try {
          const result = await client.openNativeSession(action.member);
          notify(result.message || (result.launched ? `${t("attach")}: ${result.terminal ?? "terminal"}` : t("errorTitle")), result.launched ? "success" : "error");
        } catch (error) { notify(errorMessage(error), "error"); }
        return true;
      case "find":
        openUtility("find", action.query);
        return true;
      case "surface":
        if (action.surface === "logs") {
          openUtility("logs");
        } else if (action.surface === "diff") {
          openUtility("diff");
        } else if (action.surface === "member_logs") {
          setUtilityKind("logs");
          setUtilityQuery(undefined);
          if (action.member) await requestUtility("logs", { member: action.member });
        } else if (action.surface === "mode") {
          setModePanelOpen(true);
        } else if (action.surface === "runs") {
          setRunsOpen(true);
        } else if (action.surface === "team") {
          setSettingsOpen(true);
        }
        return true;
      case "approve_first":
        await approveFirst(action.decision);
        return true;
      case "command": {
        // A target-only submission (`@member` with no body) sends nothing on
        // its own — it exists to carry staged images, like the TUI.
        if (
          action.command.type === "user_message" &&
          !action.command.body.trim() &&
          !(action.command.attachments?.length) &&
          attachments.length === 0
        ) {
          return false;
        }
        if (
          action.command.type === "user_message" &&
          attachments.length > 0 &&
          !(action.command.attachments?.length)
        ) {
          await dispatch({
            ...action.command,
            attachments: attachments.map(({ token }) => token),
          });
          return true;
        }
        await dispatch(action.command);
        if (action.command.type === "set_mode") notify(t("commandOnly"), "success");
        return true;
      }
    }
    return false;
  };

  const saveSettings = async (settings: TeamSettingsV2) => {
    setSettingsBusy(true);
    pendingTeamSettings.current = JSON.stringify(settings);
    notify(t("settingsSubmitted"));
    try {
      await dispatch({ type: "replace_team_settings", settings });
      setSettingsOpen(false);
    } catch (error) {
      pendingTeamSettings.current = null;
      throw error;
    } finally { setSettingsBusy(false); }
  };

  const checkUpdate = async () => {
    setUpdateBusy(true);
    try {
      const result = await client.checkDesktopUpdate();
      if (result.error) notify(result.error, "error");
      else if (result.update_available && result.release_url) setAvailableUpdate(result);
      else notify(t("noUpdate"), "success");
    } catch (error) { notify(errorMessage(error), "error"); }
    finally { setUpdateBusy(false); }
  };

  const openUpdate = async () => {
    const url = availableUpdate?.release_url;
    if (!url) return;
    try {
      await client.openDesktopUpdate(url);
      setAvailableUpdate(null);
    } catch (error) {
      notify(errorMessage(error), "error");
    }
  };

  const exportDiagnostics = async () => {
    try {
      const path = await client.exportDesktopDiagnostics();
      notify(t("diagnosticsExported", { path }), "success");
    } catch (error) {
      notify(errorMessage(error), "error");
    }
  };

  const resolvePausedRoute = async (itemId: string, resume: boolean) => {
    setResolvingRoute(itemId);
    try {
      await dispatch({ type: "resolve_paused_route", resume });
      setResolvedRoutes((current) => new Set(current).add(itemId));
    } finally { setResolvingRoute(null); }
  };

  const resume = async (conversation: number) => {
    setActiveConversation(conversation);
    setUtilityKind(null);
    setUtilityQuery(undefined);
    setResolvedRoutes(new Set());
    await dispatch({ type: "resume_conversation", conversation });
    if (window.innerWidth < 900) setSidebarOpen(false);
  };

  const newSession = async () => {
    setActiveConversation(null);
    setUtilityKind(null);
    setUtilityQuery(undefined);
    setResolvedRoutes(new Set());
    await dispatch({ type: "new_session" });
    await client.discardStagedAttachments().catch(() => undefined);
    if (window.innerWidth < 900) setSidebarOpen(false);
  };

  const attach = async (member: string) => {
    try {
      const result = await client.openNativeSession(member);
      notify(result.message || (result.launched ? `${t("attach")}: ${result.terminal ?? "terminal"}` : t("errorTitle")), result.launched ? "success" : "error");
    } catch (error) { notify(errorMessage(error), "error"); }
  };

  const runPaletteCommand = (text: string) => {
    void submit(text);
  };

  const toggleSidebar = () => {
    setSidebarOpen((current) => {
      const next = !current;
      if (next && window.innerWidth <= SIDEBAR_BREAKPOINT) setInspectorOpen(false);
      return next;
    });
  };

  const toggleInspector = () => {
    setInspectorOpen((current) => {
      const next = !current;
      if (next && window.innerWidth <= SIDEBAR_BREAKPOINT) setSidebarOpen(false);
      return next;
    });
  };

  const membersBusy = snapshot?.members.some(({ status }) => status === "running" || status === "queued") ?? false;

  if (state.loading && !snapshot) {
    return <div className="full-state"><img src="/asterline-mark.svg" alt="" /><div className="loading-ring" /><h1>{t("loadingTitle")}</h1><p>{t("loadingBody")}</p></div>;
  }

  if ((state.error || snapshot?.phase === "error" || snapshot?.phase === "locked") && !projectPicker) {
    const locked = snapshot?.phase === "locked";
    return <div className="full-state error-state"><img src="/asterline-mark.svg" alt="" /><h1>{t(locked ? "lockedTitle" : "errorTitle")}</h1><p>{locked ? t("lockedBody") : state.error || snapshot?.last_error}</p><div><button className="primary-button" onClick={() => void bootstrap(snapshot?.workspace ?? undefined, launchOptions)}>{t("retryOpen")}</button><button className="secondary-button" onClick={() => setProjectPicker(true)}>{t("chooseAnother")}</button></div>{projectPicker && <ProjectPicker recents={recents} canClose initialPath={snapshot?.workspace ?? ""} busy={projectBusy} t={t} onClose={() => setProjectPicker(false)} onOpen={openWorkspace} />}</div>;
  }

  const teamSettings = snapshot?.team ?? fallbackTeam(snapshot?.workspace ?? "");
  return (
    <div className="app-shell">
      <Sidebar
        open={sidebarOpen}
        locale={locale}
        workspace={snapshot?.workspace}
        teamName={snapshot?.team?.name}
        conversations={snapshot?.conversations ?? []}
        recents={recents}
        activeConversation={activeConversation ?? snapshot?.active_conversation ?? null}
        t={t}
        onToggle={toggleSidebar}
        onNew={() => void newSession()}
        onOpenProject={() => setProjectPicker(true)}
        onOpenRecent={(workspace) => void openWorkspace(workspace)}
        onResume={(id) => void resume(id)}
        onSettings={() => setSettingsOpen(true)}
        onImportSession={() => setSessionImportOpen(true)}
        onExportSession={() => void dispatch({ type: "export_session", format: "claude" })}
      />

      <main className="workspace-main">
        <header className="topbar">
          {!sidebarOpen && <button className="icon-button panel-restore-button" onClick={toggleSidebar} aria-label={t("expand")}><PanelLeftIcon /></button>}
          <div className="topbar-title"><span className="topbar-folder"><FolderIcon size={16} /></span><div><strong>{snapshot?.team?.name || snapshot?.workspace?.split(/[\\/]/).at(-1) || t("workspace")}</strong><small><i className={snapshot?.phase === "ready" ? "online" : ""} />{snapshot?.phase === "ready" ? t("connected") : t("starting")}</small></div></div>
          <div className="topbar-actions">
            {client.kind === "mock" && <span className="demo-pill">{t("demo")}</span>}
            <button className="icon-button" aria-label={snapshot?.relay_paused ? t("resumeRelayAuto") : t("pauseRelay")} title={snapshot?.relay_paused ? t("resumeRelayAuto") : t("pauseRelay")} onClick={() => void dispatch({ type: "set_relay_paused", paused: !snapshot?.relay_paused })}>{snapshot?.relay_paused ? <PlayIcon /> : <PauseIcon />}</button>
            <button className="icon-button" aria-label={t("commandPalette")} title={`${t("commandPalette")} (Ctrl+K)`} onClick={() => { setPaletteExecutable(true); setPaletteOpen(true); }}><CommandIcon /></button>
            <button className="icon-button" aria-label={t("language")} title={t("language")} onClick={() => setLocale((value) => value === "zh-CN" ? "en-US" : "zh-CN")}><GlobeIcon /><small>{locale === "zh-CN" ? "中" : "EN"}</small></button>
            <button className="icon-button" aria-label={t("theme")} title={t("theme")} onClick={() => setTheme((value) => value === "light" ? "dark" : "light")}>{theme === "light" ? <MoonIcon /> : <SunIcon />}</button>
            <button className={`icon-button ${updateBusy ? "is-spinning" : ""}`} aria-label={t("update")} title={t("update")} disabled={updateBusy} onClick={() => void checkUpdate()}><RefreshIcon /></button>
            <button className="icon-button" aria-label={t("settings")} title={t("settings")} onClick={() => setSettingsOpen(true)}><SettingsIcon /></button>
            <button className="utility-launcher-button" onClick={() => openUtility("find")} title={t("find")}><span>/</span>{t("find")}</button>
            {!inspectorOpen && <button className="icon-button panel-restore-button" onClick={toggleInspector} aria-label={t("details")}><PanelRightIcon /></button>}
          </div>
        </header>

        {client.kind === "mock" && <div className="demo-banner">{t("mockBanner")}</div>}
        <div className="utility-launcher" aria-label={t("utilities")}>
          {(["logs", "diff", "skills"] as const).map((kind) => <button key={kind} onClick={() => openUtility(kind)} className={utilityKind === kind ? "active" : ""}>{t(kind)}</button>)}
          <button onClick={() => setRunsOpen(true)} className={runsOpen ? "active" : ""}>{t("runs")}</button>
          <button onClick={() => setModePanelOpen(true)} className={modePanelOpen ? "active" : ""}>{t("modes")}</button>
        </div>
        <div className="conversation-column">
          <div className="timeline-scroll" ref={timelineScroll}>
            <div className="timeline-inner">
              {snapshot?.timeline_truncated && (
                <div className="timeline-truncated" role="status">{t("truncated")}</div>
              )}
              <Timeline items={(snapshot?.timeline ?? []).filter(({ id }) => !resolvedRoutes.has(id))} members={snapshot?.members ?? []} locale={locale} t={t} resolvingRoute={resolvingRoute} onResolvePausedRoute={resolvePausedRoute} />
              <ApprovalQueue approvals={snapshot?.approvals ?? []} members={snapshot?.members ?? []} busyId={approvalBusy} t={t} onDecision={decision} />
            </div>
          </div>
          <Composer
            members={snapshot?.members ?? []}
            mode={snapshot?.mode ?? "normal"}
            target={target}
            busy={membersBusy}
            disabled={snapshot?.phase !== "ready"}
            queues={snapshot?.queues ?? []}
            defaultMemberId={snapshot?.team?.default_target?.type === "member" ? snapshot.team.default_target.member : snapshot?.members[0]?.id ?? null}
            injection={composerInjection}
            t={t}
            onTarget={setTarget}
            onMode={(mode) => void changeMode(mode)}
            onEditQueued={() => void dispatch({ type: "edit_queued_prompt", member: target === "all" || target === "default" ? null : target })}
            onSubmit={submit}
            onCancel={() => dispatch({ type: "cancel", member: target === "all" || target === "default" ? null : target })}
          />
        </div>
      </main>

      <Inspector open={inspectorOpen} members={snapshot?.members ?? []} runs={snapshot?.runs ?? []} locale={locale} t={t} onToggle={toggleInspector} dispatch={dispatch} onAttach={attach} />

      {utilityKind && <UtilityDrawer kind={utilityKind} locale={locale} t={t} logs={state.utility.logs} diff={state.utility.diff} skills={state.utility.skills} timelineText={(snapshot?.timeline ?? []).map((item) => ({ id: item.id, title: item.title || item.kind, text: item.text || item.detail || "" }))} initialQuery={utilityQuery} onClose={() => setUtilityKind(null)} onOpen={openUtility} onRefresh={(kind, filters) => void requestUtility(kind, filters)} onExportDiagnostics={exportDiagnostics} />}

      {runsOpen && (
        <RunsPanel runs={snapshot?.runs ?? []} members={snapshot?.members ?? []} busy={membersBusy} t={t} onClose={() => setRunsOpen(false)} dispatch={dispatch} />
      )}
      {modePanelOpen && (
        <ModePanel mode={snapshot?.mode ?? "normal"} defaults={teamSettings.modes} overrides={snapshot?.mode_overrides ?? {}} members={snapshot?.members ?? []} busy={membersBusy} t={t} onClose={() => setModePanelOpen(false)} dispatch={dispatch} />
      )}
      {sessionImportOpen && (
        <SessionImportModal workspace={snapshot?.workspace ?? ""} members={snapshot?.members ?? []} busy={membersBusy} t={t} onClose={() => setSessionImportOpen(false)} dispatch={dispatch} listSessions={(backend, cwd) => client.listNativeSessions(backend, cwd)} />
      )}
      {paletteOpen && (
        <CommandPalette catalog={catalog} executable={paletteExecutable} title={paletteExecutable ? t("commandPalette") : t("helpTitle")} t={t} onClose={() => setPaletteOpen(false)} onRun={runPaletteCommand} members={(snapshot?.members ?? []).map(({ id, display_name }) => ({ id, display_name }))} />
      )}

      {projectPicker && <ProjectPicker recents={recents} canClose={Boolean(snapshot?.workspace)} initialPath={snapshot?.workspace ?? ""} busy={projectBusy} t={t} onClose={() => setProjectPicker(false)} onOpen={openWorkspace} />}
      {settingsOpen && <SettingsModal settings={teamSettings} busy={settingsBusy} t={t} onClose={() => setSettingsOpen(false)} onSave={saveSettings} />}
      {availableUpdate && (
        <div className="modal-backdrop update-backdrop" role="presentation">
          <section className="update-modal" role="dialog" aria-modal="true" aria-labelledby="update-title">
            <header>
              <span className="update-modal-icon"><RefreshIcon /></span>
              <div><h2 id="update-title">{t("updateTitle")}</h2><p>{t("updateBody")}</p></div>
              <button className="icon-button" onClick={() => setAvailableUpdate(null)} aria-label={t("close")}><XIcon /></button>
            </header>
            <div className="update-version-grid">
              <div><small>{t("currentVersion")}</small><strong>{availableUpdate.current_version}</strong></div>
              <span><ChevronIcon /></span>
              <div><small>{t("availableVersion")}</small><strong>{availableUpdate.available_version}</strong></div>
            </div>
            <footer>
              <button className="secondary-button" onClick={() => setAvailableUpdate(null)}>{t("later")}</button>
              <button className="primary-button" onClick={() => void openUpdate()}>{t("downloadUpdate")}</button>
            </footer>
          </section>
        </div>
      )}
      {toast && <div className={`toast ${toast.tone ?? ""}`} role="status"><span>{toast.text}</span><button onClick={() => setToast(null)} aria-label={t("close")}><XIcon size={14} /></button></div>}
    </div>
  );
}
