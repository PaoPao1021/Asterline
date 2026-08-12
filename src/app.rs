//! Application bootstrap: parse CLI args, resolve a team (config file or a
//! default roster from detected backends), open the store, spawn the runtime,
//! and run the chat-first TUI. Exiting shuts the runtime down gracefully.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread::JoinHandle;

use crate::adapter::{FakeRunner, MemberRunner, runner_for};
use crate::domain::config::{
    DetectedBackends, detect_backends, ensure_brainstorm_skill, ensure_team_skill,
    inject_team_protocol, load_team_config, strip_team_protocols,
};
use crate::domain::event::{ChatItem, LogEntry, RuntimeEvent, UiCommand};
use crate::domain::team::{TeamConfig, TeamSettings};
use crate::runtime::{self, Runners, RuntimeHandle};
use crate::store::sqlite::SqliteStore;
use crate::tui;
use crate::tui::app_state::AppState;

/// Entry point invoked from `main`.
pub fn run() -> io::Result<()> {
    run_with_args(std::env::args().skip(1), std::env::current_dir()?)
}

pub fn run_with_args<I, S>(args: I, cwd: impl AsRef<Path>) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let config = AppConfig::parse(args)?;
    if config.show_help {
        println!("{}", AppConfig::help());
        return Ok(());
    }
    if config.update {
        #[cfg(windows)]
        println!("{}", crate::update::update_now().map_err(io::Error::other)?);
        #[cfg(not(windows))]
        println!("automatic updates are currently available for the Windows Setup installation");
        return Ok(());
    }

    let options = SessionOptions::from(&config);
    let mut session = match bootstrap(options, cwd.as_ref())? {
        BootstrapOutcome::Ready(session) => session,
        BootstrapOutcome::NeedsTeamSetup(setup) => {
            if !setup.detected.any() {
                eprintln!(
                    "Asterline: no team config and no supported backend CLI was found on PATH.\n\
                     Install a backend CLI, or pass --team <config.json>."
                );
                return Ok(());
            }
            let Some(team) = crate::tui::team_builder::run(setup.detected, &setup.workspace)?
            else {
                return Ok(());
            };
            setup.start(team)?
        }
    };

    let mut state = AppState::new(session.take_initial_chat());
    state.seed_logs(session.take_initial_logs());
    let handle = session.handle();
    let events = session.take_events();

    if config.banner {
        print_startup_banner();
    }

    // Keep an independent shutdown handle so terminal-initialization errors
    // inside `tui::run` cannot leave the runtime waiting forever. The TUI also
    // sends Shutdown on its normal cleanup path; a duplicate send is harmless.
    let shutdown_handle = handle.clone();
    let tui_result = tui::run(handle, events, state);
    shutdown_handle.send(UiCommand::Shutdown);
    let runtime_result = session.shutdown();
    tui_result.and(runtime_result)
}

fn join_runtime(join: JoinHandle<()>) -> io::Result<()> {
    join.join()
        .map_err(|_| io::Error::other("Asterline runtime thread panicked"))
}

fn print_startup_banner() {
    println!("\x1b[1;36mAsterline\x1b[0m · Multi-Agent Coding Console");
}

/// UI-independent launch options shared by the terminal and desktop shells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionOptions {
    pub team_path: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
    pub db_path: Option<PathBuf>,
    pub restore: bool,
    pub approvals: bool,
    pub fake: bool,
    pub pick_team: bool,
    pub auto_update: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            team_path: None,
            workspace: None,
            db_path: None,
            restore: true,
            approvals: true,
            fake: false,
            pick_team: false,
            auto_update: true,
        }
    }
}

impl From<&AppConfig> for SessionOptions {
    fn from(config: &AppConfig) -> Self {
        Self {
            team_path: config.team_path.clone(),
            workspace: config.workspace.clone(),
            db_path: config.db_path.clone(),
            restore: !config.no_restore,
            approvals: !config.debug,
            fake: config.fake,
            pick_team: config.pick_team,
            auto_update: !config.no_auto_update,
        }
    }
}

/// Result of resolving a workspace. UI shells decide how to collect a roster
/// when setup is needed; the shared bootstrap never opens terminal UI.
#[allow(clippy::large_enum_variant)]
pub enum BootstrapOutcome {
    Ready(AppSession),
    NeedsTeamSetup(TeamSetup),
}

/// Information required by a UI-specific first-run team builder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeamSetup {
    pub workspace: PathBuf,
    pub save_path: PathBuf,
    pub detected: DetectedBackends,
    options: SessionOptions,
}

impl TeamSetup {
    /// Validate and save a builder result, then start the shared runtime. The
    /// builder cannot redirect the already-selected workspace.
    pub fn start(self, mut team: TeamConfig) -> io::Result<AppSession> {
        team.workspace = self.workspace.clone();
        let team = strip_team_protocols(team);
        team.validate()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        start_session(self.options, team, self.save_path, true)
    }
}

/// A running Asterline core session. It owns the store lock and runtime thread
/// and is safe to move to a desktop bridge worker thread.
pub struct AppSession {
    handle: RuntimeHandle,
    join: Option<JoinHandle<()>>,
    events: Option<mpsc::Receiver<RuntimeEvent>>,
    initial_chat: Vec<ChatItem>,
    initial_logs: Vec<LogEntry>,
    workspace: PathBuf,
    team_settings: TeamSettings,
    _workspace_lock: InstanceLock,
    _store_lock: InstanceLock,
}

impl AppSession {
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    pub fn events(&self) -> &mpsc::Receiver<RuntimeEvent> {
        self.events
            .as_ref()
            .expect("runtime event receiver has already been taken")
    }

    /// Transfer the sole event receiver to a UI event loop.
    pub fn take_events(&mut self) -> mpsc::Receiver<RuntimeEvent> {
        self.events
            .take()
            .expect("runtime event receiver has already been taken")
    }

    pub fn initial_chat(&self) -> &[ChatItem] {
        &self.initial_chat
    }

    pub fn take_initial_chat(&mut self) -> Vec<ChatItem> {
        std::mem::take(&mut self.initial_chat)
    }

    pub fn initial_logs(&self) -> &[LogEntry] {
        &self.initial_logs
    }

    pub fn take_initial_logs(&mut self) -> Vec<LogEntry> {
        std::mem::take(&mut self.initial_logs)
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// Initial settings contain user prompts with Asterline's injected
    /// protocol removed. Subsequent changes are delivered as runtime events.
    pub fn team_settings(&self) -> TeamSettings {
        self.team_settings.clone()
    }

    pub fn shutdown(mut self) -> io::Result<()> {
        self.shutdown_inner()
    }

    fn shutdown_inner(&mut self) -> io::Result<()> {
        self.handle.send(UiCommand::Shutdown);
        // A bounded event queue may currently have the runtime blocked in a
        // send. Dropping our receiver unblocks that send before joining. UIs
        // that took the receiver own its drain/drop lifecycle themselves.
        drop(self.events.take());
        match self.join.take() {
            Some(join) => join_runtime(join),
            None => Ok(()),
        }
    }
}

impl Drop for AppSession {
    fn drop(&mut self) {
        let _ = self.shutdown_inner();
    }
}

/// An OS-backed exclusive lock. The marker file remains after exit, but the
/// lock itself is released automatically with the handle.
struct InstanceLock {
    _file: std::fs::File,
}

impl InstanceLock {
    fn acquire_store(db_path: &Path) -> io::Result<Self> {
        let mut lock_name = db_path.as_os_str().to_os_string();
        lock_name.push(".lock");
        Self::acquire_path(
            &PathBuf::from(lock_name),
            &format!("store {}", db_path.display()),
        )
    }

    fn acquire_workspace(workspace: &Path) -> io::Result<Self> {
        let canonical_workspace = workspace.canonicalize()?;
        let state_dir = workspace.join(".asterline");
        if std::fs::symlink_metadata(&state_dir).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "refusing symbolic-link state directory for workspace {}",
                    workspace.display()
                ),
            ));
        }
        std::fs::create_dir_all(&state_dir)?;
        let canonical_state_dir = state_dir.canonicalize()?;
        if canonical_state_dir.parent() != Some(canonical_workspace.as_path()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "workspace state directory resolves outside {}",
                    workspace.display()
                ),
            ));
        }
        Self::acquire_path(
            &canonical_state_dir.join("instance.lock"),
            &format!("workspace {}", workspace.display()),
        )
    }

    fn acquire_path(lock_path: &Path, resource: &str) -> io::Result<Self> {
        if std::fs::symlink_metadata(lock_path).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("refusing symbolic-link lock file for {resource}"),
            ));
        }
        let mut options = std::fs::OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let file = options.open(lock_path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "could not lock {resource} (another Asterline instance may be using it): \
                     {err}"
                ),
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                let err = io::Error::last_os_error();
                return Err(io::Error::new(
                    err.kind(),
                    format!("another Asterline instance is already using {resource}: {err}"),
                ));
            }
        }
        Ok(Self { _file: file })
    }
}

/// Resolve configuration without opening a UI and start the shared runtime
/// when a saved or explicit team is available.
pub fn bootstrap(options: SessionOptions, cwd: impl AsRef<Path>) -> io::Result<BootstrapOutcome> {
    let requested_workspace = options
        .workspace
        .clone()
        .unwrap_or_else(|| cwd.as_ref().to_path_buf());

    let saved_team = requested_workspace.join(".asterline").join("team.json");
    let mut team = match &options.team_path {
        Some(path) => load_team_config(path)?,
        None if !options.pick_team && saved_team.is_file() => load_team_config(&saved_team)?,
        None => {
            return Ok(BootstrapOutcome::NeedsTeamSetup(TeamSetup {
                workspace: requested_workspace,
                save_path: saved_team,
                detected: detect_backends(),
                options,
            }));
        }
    };
    // A CLI workspace is an explicit launch-time override. Without one, the
    // team file's workspace is canonical for runners, skills, and the default
    // database location.
    if let Some(workspace) = &options.workspace {
        team.workspace = workspace.clone();
    }
    let team_save_path = options.team_path.clone().unwrap_or(saved_team);
    start_session(options, team, team_save_path, false).map(BootstrapOutcome::Ready)
}

fn start_session(
    options: SessionOptions,
    team: TeamConfig,
    team_save_path: PathBuf,
    save_initial_team: bool,
) -> io::Result<AppSession> {
    let raw_team = strip_team_protocols(team);
    raw_team
        .validate()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let workspace = raw_team.workspace.clone();
    let team_settings = TeamSettings::from_config(&raw_team);
    // A workspace identity is locked independently from its configurable DB.
    // This prevents `--db` from opening two runtimes over the same team, while
    // the store sidecar also prevents different workspaces sharing one DB.
    let workspace_lock = InstanceLock::acquire_workspace(&workspace)?;
    crate::project_state::ensure_project_state(&workspace)?;
    if save_initial_team {
        runtime::save_team_config(&team_save_path, &raw_team)?;
    }
    ensure_team_skill(&workspace)?;
    ensure_brainstorm_skill(&workspace)?;
    let mut team = raw_team;
    inject_team_protocol(&mut team);

    let db_path = options
        .db_path
        .clone()
        .unwrap_or_else(|| workspace.join(".asterline").join("asterline.sqlite3"));
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store_lock = InstanceLock::acquire_store(&db_path)?;
    let store = SqliteStore::open(&db_path).map_err(|err| io::Error::other(err.to_string()))?;

    let runners = build_runners(&team, options.fake);
    let (chat, logs) = if !options.restore {
        (Vec::new(), Vec::new())
    } else {
        // Replay only the current conversation (the latest, or a fresh one).
        if let Ok(conversation) = store.current_conversation() {
            store
                .set_conversation(conversation)
                .map_err(|err| io::Error::other(err.to_string()))?;
        }
        // A replay failure must be visible, not a silently-blank transcript:
        // surface it as the first chat item so a schema/store problem is
        // obvious in-app instead of looking like "history was lost".
        let chat = match store.replay_chat() {
            Ok(chat) => chat,
            Err(err) => vec![ChatItem::Notice {
                text: format!("could not replay history: {err}"),
            }],
        };
        // Logs are persisted too; replay the recent tail so the logs drawer
        // isn't empty after a restart.
        let logs = store.recent_logs(4000).unwrap_or_default();
        (chat, logs)
    };
    // Bound the runtime-to-TUI stream so a fast or malformed backend cannot
    // turn a slow terminal renderer into an unbounded in-memory queue.
    let (events_tx, events_rx) = mpsc::sync_channel(2_048);
    #[cfg(windows)]
    if options.auto_update {
        crate::update::spawn_auto_update(events_tx.clone());
    }
    let (handle, join) = runtime::spawn_bounded(
        team,
        store,
        runners,
        events_tx,
        options.approvals,
        options.fake,
        Some(team_save_path),
    );

    Ok(AppSession {
        handle,
        join: Some(join),
        events: Some(events_rx),
        initial_chat: chat,
        initial_logs: logs,
        workspace,
        team_settings,
        _workspace_lock: workspace_lock,
        _store_lock: store_lock,
    })
}

fn build_runners(team: &TeamConfig, fake: bool) -> Runners {
    let mut runners: Runners = HashMap::new();
    for member in &team.members {
        let runner: Arc<dyn MemberRunner> = if fake {
            Arc::new(FakeRunner::team(member.backend))
        } else {
            Arc::from(runner_for(member, &team.workspace))
        };
        runners.insert(member.id.clone(), runner);
    }
    runners
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppConfig {
    team_path: Option<PathBuf>,
    workspace: Option<PathBuf>,
    db_path: Option<PathBuf>,
    no_restore: bool,
    debug: bool,
    fake: bool,
    pick_team: bool,
    banner: bool,
    no_auto_update: bool,
    update: bool,
    show_help: bool,
}

impl AppConfig {
    pub fn parse<I, S>(args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut config = AppConfig::default();
        let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "--team" => {
                    config.team_path = Some(Self::value(&args, &mut index, "--team")?.into())
                }
                "--workspace" => {
                    config.workspace = Some(Self::value(&args, &mut index, "--workspace")?.into())
                }
                "--db" => config.db_path = Some(Self::value(&args, &mut index, "--db")?.into()),
                "--no-restore" => config.no_restore = true,
                "--debug" => config.debug = true,
                "--fake" => config.fake = true,
                "--pick-team" => config.pick_team = true,
                "--banner" => config.banner = true,
                "--no-auto-update" => config.no_auto_update = true,
                "--update" => config.update = true,
                "-h" | "--help" => config.show_help = true,
                _ if arg.starts_with("--team=") => {
                    config.team_path = Some(arg["--team=".len()..].into())
                }
                _ if arg.starts_with("--workspace=") => {
                    config.workspace = Some(arg["--workspace=".len()..].into())
                }
                _ if arg.starts_with("--db=") => config.db_path = Some(arg["--db=".len()..].into()),
                unknown => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument: {unknown}"),
                    ));
                }
            }
            index += 1;
        }
        Ok(config)
    }

    fn value(args: &[String], index: &mut usize, flag: &str) -> io::Result<String> {
        *index += 1;
        args.get(*index).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{flag} requires a value"),
            )
        })
    }

    pub fn help() -> &'static str {
        "Asterline — a chat-first multi-agent coding console.\n\
         \n\
         Usage: asterline [OPTIONS]\n\
         \n\
         Options:\n\
         \x20 --team <PATH>       Load a team config (JSON). Skips the team builder.\n\
         \x20 --pick-team         Re-open the interactive team builder (ignore the saved team).\n\
         \x20 --workspace <PATH>  Working directory for members. Default: current directory.\n\
         \x20 --db <PATH>         SQLite path. Default: <workspace>/.asterline/asterline.sqlite3.\n\
         \x20 --no-restore        Do not replay persisted chat history on startup.\n\
         \x20 --debug             Disable the approval gate (developer mode).\n\
         \x20 --fake              Use offline fake agents instead of real CLIs.\n\
         \x20 --banner            Print a compact startup banner before the TUI.\n\
         \x20 --update            Check now and schedule a Windows installer update.\n\
         \x20 --no-auto-update    Skip the Windows installer update check.\n\
         \x20 -h, --help          Show this help.\n\
         \n\
         With no --team, Asterline opens a team builder from the detected backends\n\
         and remembers your choice in <workspace>/.asterline/team.json."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::{MessageTarget, UiCommand};
    use std::time::Duration;

    fn prepare_session(config: &AppConfig, cwd: &Path) -> AppSession {
        match bootstrap(SessionOptions::from(config), cwd).unwrap() {
            BootstrapOutcome::Ready(session) => session,
            BootstrapOutcome::NeedsTeamSetup(_) => panic!("expected a configured team"),
        }
    }

    #[test]
    fn parses_flags() {
        let config = AppConfig::parse([
            "--team",
            "/tmp/t.json",
            "--workspace",
            "/tmp/ws",
            "--no-restore",
            "--fake",
            "--banner",
            "--no-auto-update",
        ])
        .unwrap();
        assert_eq!(config.team_path, Some(PathBuf::from("/tmp/t.json")));
        assert_eq!(config.workspace, Some(PathBuf::from("/tmp/ws")));
        assert!(config.no_restore);
        assert!(config.fake);
        assert!(config.banner);
        assert!(config.no_auto_update);
    }

    #[test]
    fn parses_equals_form_and_help() {
        let config = AppConfig::parse(["--db=/tmp/x.sqlite3", "--help"]).unwrap();
        assert_eq!(config.db_path, Some(PathBuf::from("/tmp/x.sqlite3")));
        assert!(config.show_help);
        assert!(!config.banner);
        assert!(!config.update);
    }

    #[test]
    fn help_mentions_compact_banner_flag() {
        assert!(AppConfig::help().contains("--banner"));
        assert!(AppConfig::help().contains("compact startup banner"));
        assert!(AppConfig::help().contains("--update"));
        assert!(AppConfig::help().contains("--no-auto-update"));
    }

    #[test]
    fn parses_manual_update_flag() {
        let config = AppConfig::parse(["--update"]).unwrap();
        assert!(config.update);
        assert!(!config.no_auto_update);
    }

    #[test]
    fn unknown_arg_rejected() {
        assert!(AppConfig::parse(["--nope"]).is_err());
    }

    #[test]
    fn missing_value_rejected() {
        assert!(AppConfig::parse(["--team"]).is_err());
    }

    #[test]
    fn app_session_is_send_and_static_for_desktop_worker_ownership() {
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<AppSession>();
    }

    #[test]
    fn bootstrap_returns_ui_neutral_team_setup_when_config_is_missing() {
        let dir =
            std::env::temp_dir().join(format!("asterline-app-needs-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = SessionOptions {
            workspace: Some(dir.clone()),
            auto_update: false,
            ..SessionOptions::default()
        };

        let outcome = bootstrap(options, &dir).unwrap();
        let BootstrapOutcome::NeedsTeamSetup(setup) = outcome else {
            panic!("missing team must not open a UI or invent a roster");
        };
        assert_eq!(setup.workspace, dir);
        assert_eq!(setup.save_path, dir.join(".asterline/team.json"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn team_setup_saves_raw_config_and_starts_a_session() {
        let dir =
            std::env::temp_dir().join(format!("asterline-app-setup-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = SessionOptions {
            workspace: Some(dir.clone()),
            fake: true,
            auto_update: false,
            ..SessionOptions::default()
        };
        let BootstrapOutcome::NeedsTeamSetup(setup) = bootstrap(options, &dir).unwrap() else {
            panic!("setup expected");
        };
        let mut team = TeamConfig::new("desktop", "/must/not/win").with_member(
            crate::domain::team::TeamMember::new(
                "builder",
                "Builder",
                crate::domain::team::BackendKind::Codex,
                "implementation",
            ),
        );
        team.members[0].system_prompt = Some("visible custom prompt".to_string());
        let session = setup.start(team).unwrap();

        assert_eq!(session.workspace(), dir.as_path());
        assert_eq!(session.team_settings().name, "desktop");
        assert_eq!(
            session.team_settings().members[0].system_prompt.as_deref(),
            Some("visible custom prompt")
        );
        let saved = std::fs::read_to_string(dir.join(".asterline/team.json")).unwrap();
        assert!(saved.contains("visible custom prompt"));
        assert!(!saved.contains("ASTERLINE_TEAM_PROTOCOL"));
        session.shutdown().unwrap();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn runtime_thread_panic_is_reported_as_an_io_error() {
        let join = std::thread::spawn(|| panic!("runtime failure"));
        let result = join_runtime(join);

        assert_eq!(
            result.expect_err("panic must be visible").to_string(),
            "Asterline runtime thread panicked"
        );
    }

    #[test]
    fn store_instance_lock_is_exclusive_and_released_on_drop() {
        let dir =
            std::env::temp_dir().join(format!("asterline-instance-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("state.sqlite3");

        let first = InstanceLock::acquire_store(&db).unwrap();
        let error = InstanceLock::acquire_store(&db)
            .err()
            .expect("a second instance must be rejected");
        assert!(error.to_string().contains("another Asterline instance"));

        drop(first);
        InstanceLock::acquire_store(&db).expect("lock must be released when the owner exits");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn instance_lock_rejects_symlinks_without_touching_the_target() {
        use std::os::unix::fs::symlink;

        let dir = std::env::temp_dir().join(format!(
            "asterline-instance-lock-symlink-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("keep.txt");
        std::fs::write(&target, "do not truncate").unwrap();
        let lock = dir.join("state.sqlite3.lock");
        symlink(&target, &lock).unwrap();

        let error = InstanceLock::acquire_store(&dir.join("state.sqlite3"))
            .err()
            .expect("symbolic-link lock must be rejected");
        assert!(error.to_string().contains("symbolic-link"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "do not truncate");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn workspace_lock_uses_canonical_identity_and_releases_on_drop() {
        let dir =
            std::env::temp_dir().join(format!("asterline-workspace-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let first = InstanceLock::acquire_workspace(&dir).unwrap();
        let alias = dir.join(".");
        let error = InstanceLock::acquire_workspace(&alias)
            .err()
            .expect("a canonical workspace alias must share the same lock");
        assert!(error.to_string().contains("another Asterline instance"));

        drop(first);
        InstanceLock::acquire_workspace(&alias)
            .expect("workspace lock must be released when the owner exits");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn custom_database_cannot_bypass_the_workspace_lock() {
        let dir = std::env::temp_dir().join(format!(
            "asterline-workspace-session-lock-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let team =
            TeamConfig::new("locked", &dir).with_member(crate::domain::team::TeamMember::new(
                "builder",
                "Builder",
                crate::domain::team::BackendKind::Codex,
                "implementation",
            ));
        let options = |db_path: PathBuf| SessionOptions {
            workspace: Some(dir.clone()),
            db_path: Some(db_path),
            fake: true,
            auto_update: false,
            ..SessionOptions::default()
        };
        let save_path = dir.join(".asterline/team.json");
        let first = start_session(
            options(dir.join("first.sqlite3")),
            team.clone(),
            save_path.clone(),
            false,
        )
        .unwrap();
        let error = start_session(
            options(dir.join("second.sqlite3")),
            team.clone(),
            save_path.clone(),
            false,
        )
        .err()
        .expect("a second DB path must not bypass the workspace lock");
        assert!(error.to_string().contains("another Asterline instance"));

        first.shutdown().unwrap();
        let reopened = start_session(options(dir.join("second.sqlite3")), team, save_path, false)
            .expect("shutdown must release both workspace and store locks");
        reopened.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn inject_protocol_lists_teammates() {
        let mut team = crate::domain::config::default_team(
            "/tmp/ws",
            crate::domain::config::DetectedBackends {
                codex: true,
                claude: true,
                grok: false,
                agy: false,
            },
        )
        .unwrap();
        inject_team_protocol(&mut team);
        let builder = team
            .member(&crate::domain::team::MemberId::new("builder"))
            .unwrap();
        let prompt = builder.system_prompt.as_ref().unwrap();
        assert!(prompt.contains("$asterline-team"));
        assert!(prompt.contains("reviewer"));
    }

    #[test]
    fn prepare_with_fake_backend_runs_a_turn() {
        let dir = std::env::temp_dir().join(format!("asterline-app-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Force a single-codex default team via a written config so the test is
        // independent of what is installed on PATH.
        let team = crate::domain::config::default_team(
            &dir,
            crate::domain::config::DetectedBackends {
                codex: true,
                claude: false,
                grok: false,
                agy: false,
            },
        )
        .unwrap();
        let team_path = dir.join("team.json");
        std::fs::write(&team_path, serde_json::to_string(&team).unwrap()).unwrap();

        let config = AppConfig::parse([
            "--team",
            team_path.to_str().unwrap(),
            "--db",
            dir.join("db.sqlite3").to_str().unwrap(),
            "--fake",
        ])
        .unwrap();

        let prepared = prepare_session(&config, &dir);
        assert!(
            dir.join(crate::domain::config::ASTERLINE_TEAM_SKILL_PATH)
                .is_file()
        );
        let handle = prepared.handle();
        let events = prepared.events();

        // Drain the Ready event.
        let ready = events.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(ready, RuntimeEvent::Ready { .. }));

        handle.send(UiCommand::UserMessage {
            target: MessageTarget::Default,
            body: "hello".to_string(),
        });

        let mut saw_completed = false;
        while let Ok(event) = events.recv_timeout(Duration::from_secs(2)) {
            if let RuntimeEvent::MessageCompleted { text, .. } = &event
                && text.contains("hello")
            {
                saw_completed = true;
            }
            if matches!(event, RuntimeEvent::TurnFinished { .. }) {
                break;
            }
        }
        assert!(saw_completed);

        handle.send(UiCommand::Shutdown);
        prepared.shutdown().unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn team_workspace_is_canonical_for_the_default_database() {
        let root = std::env::temp_dir().join(format!(
            "asterline-app-team-workspace-{}",
            std::process::id()
        ));
        let invocation = root.join("invocation");
        let team_workspace = root.join("team-workspace");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&invocation).unwrap();
        std::fs::create_dir_all(&team_workspace).unwrap();

        let team = crate::domain::config::default_team(
            &team_workspace,
            crate::domain::config::DetectedBackends {
                codex: true,
                claude: false,
                grok: false,
                agy: false,
            },
        )
        .unwrap();
        let team_path = root.join("team.json");
        std::fs::write(&team_path, serde_json::to_string(&team).unwrap()).unwrap();
        let config = AppConfig::parse([
            "--team",
            team_path.to_str().unwrap(),
            "--fake",
            "--no-restore",
        ])
        .unwrap();

        let prepared = prepare_session(&config, &invocation);
        assert!(
            team_workspace
                .join(".asterline/asterline.sqlite3")
                .is_file()
        );
        assert!(!invocation.join(".asterline/asterline.sqlite3").exists());
        let ready = prepared
            .events()
            .recv_timeout(Duration::from_secs(2))
            .expect("ready");
        assert!(matches!(
            ready,
            RuntimeEvent::Ready { workspace, .. }
                if workspace == team_workspace.display().to_string()
        ));

        prepared.handle().send(UiCommand::Shutdown);
        prepared.shutdown().unwrap();
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn cli_workspace_overrides_the_team_file_workspace() {
        let root = std::env::temp_dir().join(format!(
            "asterline-app-cli-workspace-{}",
            std::process::id()
        ));
        let declared_workspace = root.join("declared-workspace");
        let cli_workspace = root.join("cli-workspace");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&declared_workspace).unwrap();
        std::fs::create_dir_all(&cli_workspace).unwrap();

        let team = crate::domain::config::default_team(
            &declared_workspace,
            crate::domain::config::DetectedBackends {
                codex: true,
                claude: false,
                grok: false,
                agy: false,
            },
        )
        .unwrap();
        let team_path = root.join("team.json");
        std::fs::write(&team_path, serde_json::to_string(&team).unwrap()).unwrap();
        let config = AppConfig::parse([
            "--team",
            team_path.to_str().unwrap(),
            "--workspace",
            cli_workspace.to_str().unwrap(),
            "--fake",
            "--no-restore",
        ])
        .unwrap();

        let prepared = prepare_session(&config, &root);
        assert!(cli_workspace.join(".asterline/asterline.sqlite3").is_file());
        assert!(!declared_workspace.join(".asterline").exists());
        let ready = prepared
            .events()
            .recv_timeout(Duration::from_secs(2))
            .expect("ready");
        assert!(matches!(
            ready,
            RuntimeEvent::Ready { workspace, .. }
                if workspace == cli_workspace.display().to_string()
        ));

        prepared.handle().send(UiCommand::Shutdown);
        prepared.shutdown().unwrap();
        std::fs::remove_dir_all(&root).ok();
    }
}
