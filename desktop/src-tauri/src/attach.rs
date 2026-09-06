use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::bridge::{BackendKindV2, MemberStatusV2, MemberSummaryV2};

const DESCRIPTOR_LIMIT: u64 = 64 * 1024;
const RESULT_LIMIT: u64 = 16 * 1024 * 1024;
const IMPORT_TEXT_LIMIT: usize = 8 * 1024 * 1024;
const RESULT_WAIT_LIMIT: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachBackendCapabilityV2 {
    pub can_open: bool,
    pub can_import: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachBackendsV2 {
    pub codex: AttachBackendCapabilityV2,
    pub claude: AttachBackendCapabilityV2,
    pub grok: AttachBackendCapabilityV2,
    pub agy: AttachBackendCapabilityV2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachCapabilitiesV2 {
    pub supported: bool,
    pub terminal: Option<String>,
    pub backends: AttachBackendsV2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExternalAttachLaunchV2 {
    pub member: String,
    pub launched: bool,
    pub terminal: Option<String>,
    pub message: Option<String>,
}

pub struct AttachWatch {
    pub member: String,
    result_path: PathBuf,
    descriptor_path: PathBuf,
}

pub struct AttachImport {
    pub session: Option<String>,
    pub items: Vec<(bool, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AttachDescriptor {
    version: u8,
    token: String,
    member: String,
    display_name: String,
    backend: BackendKindV2,
    session: Option<String>,
    cwd: String,
    result_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct AttachResult {
    version: u8,
    member: String,
    ok: bool,
    message: String,
    session: Option<String>,
    imported: Vec<AttachImported>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AttachImported {
    from_user: bool,
    text: String,
}

pub fn capabilities() -> AttachCapabilitiesV2 {
    let terminal = terminal_name();
    let capability = |backend, can_import| AttachBackendCapabilityV2 {
        can_open: resolve_program(backend).is_some(),
        can_import,
    };
    AttachCapabilitiesV2 {
        supported: terminal.is_some(),
        terminal,
        backends: AttachBackendsV2 {
            codex: capability("codex", true),
            claude: capability("claude", true),
            grok: capability("grok", false),
            agy: capability("agy", false),
        },
    }
}

pub fn launch(
    app: &AppHandle,
    member: &MemberSummaryV2,
) -> Result<(ExternalAttachLaunchV2, AttachWatch), String> {
    if !matches!(member.status, MemberStatusV2::Idle | MemberStatusV2::Failed) {
        return Err(format!(
            "{} is {}; cancel or wait for its run before attaching",
            member.display_name,
            member_status_name(member.status)
        ));
    }
    let backend_name = backend_program(member.backend);
    if resolve_program(backend_name).is_none() {
        return Err(format!("{backend_name} is not available on PATH"));
    }
    let terminal = terminal_name().ok_or_else(|| {
        "no supported external terminal is available on this platform".to_string()
    })?;
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("could not resolve attach directory: {error}"))?
        .join("attach");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create attach directory: {error}"))?;
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    let descriptor_path = directory.join(format!("{id}.json"));
    let result_path = directory.join(format!("{id}.result.json"));
    let descriptor = AttachDescriptor {
        version: 1,
        token: token.clone(),
        member: member.id.clone(),
        display_name: member.display_name.clone(),
        backend: member.backend,
        session: member.session.clone(),
        cwd: member.cwd.clone(),
        result_path: result_path.clone(),
    };
    write_descriptor(&descriptor_path, &descriptor)
        .map_err(|error| format!("could not write attach descriptor: {error}"))?;
    if let Err(error) = spawn_terminal(&descriptor_path, &token) {
        let _ = fs::remove_file(&descriptor_path);
        return Err(format!("could not open external terminal: {error}"));
    }
    let message = if matches!(member.backend, BackendKindV2::Grok | BackendKindV2::Agy) {
        Some("This backend can be opened, but v1 cannot import its native transcript.".to_string())
    } else {
        None
    };
    Ok((
        ExternalAttachLaunchV2 {
            member: member.id.clone(),
            launched: true,
            terminal: Some(terminal),
            message,
        },
        AttachWatch {
            member: member.id.clone(),
            result_path,
            descriptor_path,
        },
    ))
}

impl AttachWatch {
    pub fn wait(self) -> Option<Result<AttachImport, String>> {
        let deadline = Instant::now() + RESULT_WAIT_LIMIT;
        while Instant::now() < deadline {
            if self.result_path.is_file() {
                let result = read_bounded_json::<AttachResult>(&self.result_path, RESULT_LIMIT)
                    .map_err(|error| format!("could not read attach result: {error}"));
                let _ = fs::remove_file(&self.result_path);
                let _ = fs::remove_file(&self.descriptor_path);
                return Some(result.and_then(|result| {
                    if result.version != 1 || result.member != self.member {
                        return Err("attach result did not match its request".to_string());
                    }
                    if !result.ok {
                        return Err(result.message);
                    }
                    Ok(AttachImport {
                        session: result.session,
                        items: result
                            .imported
                            .into_iter()
                            .map(|item| (item.from_user, item.text))
                            .collect(),
                    })
                }));
            }
            thread::sleep(Duration::from_millis(500));
        }
        let _ = fs::remove_file(&self.descriptor_path);
        None
    }
}

pub fn run_helper_from_args() -> Option<Result<(), String>> {
    let mut args = env::args_os();
    let _program = args.next();
    if args.next().as_deref() != Some(OsStr::new("--attach-task")) {
        return None;
    }
    let descriptor = match args.next() {
        Some(path) => PathBuf::from(path),
        None => return Some(Err("--attach-task needs a descriptor path".to_string())),
    };
    let token = match args.next().and_then(|value| value.into_string().ok()) {
        Some(token) => token,
        None => return Some(Err("--attach-task needs a token".to_string())),
    };
    if args.next().is_some() {
        return Some(Err("unexpected attach helper argument".to_string()));
    }
    Some(run_helper(&descriptor, &token))
}

fn run_helper(path: &Path, token: &str) -> Result<(), String> {
    #[cfg(windows)]
    ensure_windows_console()?;
    let descriptor = read_bounded_json::<AttachDescriptor>(path, DESCRIPTOR_LIMIT)
        .map_err(|error| format!("could not read attach descriptor: {error}"))?;
    if descriptor.version != 1 || descriptor.token != token {
        return Err("attach descriptor authentication failed".to_string());
    }
    // Consume the descriptor before launching anything; replaying the helper
    // invocation cannot start the backend a second time.
    fs::remove_file(path)
        .map_err(|error| format!("could not consume attach descriptor: {error}"))?;
    let cwd = PathBuf::from(&descriptor.cwd);
    if !cwd.is_dir() {
        return write_failed_result(&descriptor, "member working directory no longer exists");
    }
    let program_name = backend_program(descriptor.backend);
    let program = match resolve_program(program_name) {
        Some(program) => program,
        None => return write_failed_result(&descriptor, &format!("{program_name} is not on PATH")),
    };
    let fresh_session = (descriptor.backend == BackendKindV2::Claude
        && descriptor.session.is_none())
    .then(|| uuid::Uuid::new_v4().to_string());
    let transcript_session = descriptor.session.as_deref().or(fresh_session.as_deref());
    let arguments = backend_arguments(
        descriptor.backend,
        descriptor.session.as_deref(),
        fresh_session.as_deref(),
    );

    enum Snapshot {
        Codex(asterline::tui::rollout_import::RolloutSnapshot),
        Claude(asterline::tui::claude_import::ClaudeSnapshot),
    }
    let snapshot =
        match descriptor.backend {
            BackendKindV2::Codex => Some(Snapshot::Codex(
                asterline::tui::rollout_import::snapshot(transcript_session, &descriptor.cwd),
            )),
            BackendKindV2::Claude => Some(Snapshot::Claude(
                asterline::tui::claude_import::snapshot(transcript_session, &descriptor.cwd),
            )),
            BackendKindV2::Grok | BackendKindV2::Agy => None,
        };

    println!(
        "\n── {} · {} {} ──\n  Exit the backend CLI to return to Asterline Desktop.\n",
        descriptor.display_name,
        program_name,
        arguments.join(" ")
    );
    let status = Command::new(program)
        .args(&arguments)
        .current_dir(&cwd)
        .status();
    let (ok, message) = match status {
        Ok(status) => (status.success(), format!("backend exited with {status}")),
        Err(error) => (false, format!("could not launch {program_name}: {error}")),
    };
    let (session, imported) = snapshot
        .map(|snapshot| match snapshot {
            Snapshot::Codex(snapshot) => {
                asterline::tui::rollout_import::imported_since_with_session(snapshot)
            }
            Snapshot::Claude(snapshot) => {
                asterline::tui::claude_import::imported_since_with_session(snapshot)
            }
        })
        .unwrap_or_else(|| (descriptor.session.clone(), Vec::new()));
    let session = session.or(fresh_session);
    let mut imported = imported
        .into_iter()
        .map(|item| AttachImported {
            from_user: item.from_user,
            text: item.text,
        })
        .collect();
    bound_imported(&mut imported);
    write_result(
        &descriptor.result_path,
        &AttachResult {
            version: 1,
            member: descriptor.member,
            ok,
            message,
            session,
            imported,
        },
    )
    .map_err(|error| format!("could not write attach result: {error}"))
}

fn write_failed_result(descriptor: &AttachDescriptor, message: &str) -> Result<(), String> {
    write_result(
        &descriptor.result_path,
        &AttachResult {
            version: 1,
            member: descriptor.member.clone(),
            ok: false,
            message: message.to_string(),
            session: descriptor.session.clone(),
            imported: Vec::new(),
        },
    )
    .map_err(|error| format!("could not write attach failure: {error}"))?;
    Err(message.to_string())
}

fn write_descriptor(path: &Path, descriptor: &AttachDescriptor) -> io::Result<()> {
    let bytes = serde_json::to_vec(descriptor)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&bytes)?;
    file.flush()?;
    file.sync_all()
}

fn write_result(path: &Path, result: &AttachResult) -> io::Result<()> {
    let bytes = serde_json::to_vec(result)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    AtomicFile::new(path, AllowOverwrite)
        .write(|file| {
            file.write_all(&bytes)?;
            file.flush()?;
            file.sync_all()
        })
        .map_err(|error| match error {
            atomicwrites::Error::Internal(error) | atomicwrites::Error::User(error) => error,
        })
}

fn read_bounded_json<T: for<'de> Deserialize<'de>>(path: &Path, limit: u64) -> io::Result<T> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "attach file is too large",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn bound_imported(items: &mut Vec<AttachImported>) {
    let mut remaining = IMPORT_TEXT_LIMIT;
    items.retain_mut(|item| {
        if remaining == 0 {
            return false;
        }
        if item.text.len() > remaining {
            let mut boundary = remaining;
            while !item.text.is_char_boundary(boundary) {
                boundary = boundary.saturating_sub(1);
            }
            item.text.truncate(boundary);
        }
        remaining = remaining.saturating_sub(item.text.len());
        !item.text.is_empty()
    });
}

#[cfg(windows)]
fn ensure_windows_console() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AllocConsole, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };

    // CREATE_NEW_CONSOLE normally gives the helper a console. AllocConsole is
    // a safe fallback for GUI-subsystem builds where the flag is ignored.
    unsafe {
        let _ = AllocConsole();
    }
    for (name, access, standard) in [
        ("CONIN$", GENERIC_READ | GENERIC_WRITE, STD_INPUT_HANDLE),
        ("CONOUT$", GENERIC_READ | GENERIC_WRITE, STD_OUTPUT_HANDLE),
        ("CONOUT$", GENERIC_READ | GENERIC_WRITE, STD_ERROR_HANDLE),
    ] {
        let wide = OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(format!("could not open {name} for attach helper"));
        }
        if unsafe { SetStdHandle(standard, handle) } == 0 {
            return Err(format!("could not connect {name} to attach helper"));
        }
    }
    Ok(())
}

fn backend_program(backend: BackendKindV2) -> &'static str {
    match backend {
        BackendKindV2::Codex => "codex",
        BackendKindV2::Claude => "claude",
        BackendKindV2::Grok => "grok",
        BackendKindV2::Agy => "agy",
    }
}

fn backend_arguments(
    backend: BackendKindV2,
    session: Option<&str>,
    fresh_session: Option<&str>,
) -> Vec<String> {
    match (backend, session, fresh_session) {
        (BackendKindV2::Codex, Some(session), _) => {
            vec!["resume".to_string(), session.to_string()]
        }
        (BackendKindV2::Claude, Some(session), _) => {
            vec!["--resume".to_string(), session.to_string()]
        }
        (BackendKindV2::Claude, None, Some(session)) => {
            vec!["--session-id".to_string(), session.to_string()]
        }
        (BackendKindV2::Grok, Some(session), _) => {
            vec!["--resume".to_string(), session.to_string()]
        }
        (BackendKindV2::Agy, Some(session), _) => {
            vec!["--conversation".to_string(), session.to_string()]
        }
        (_, None, _) => Vec::new(),
    }
}

fn member_status_name(status: MemberStatusV2) -> &'static str {
    match status {
        MemberStatusV2::Idle => "idle",
        MemberStatusV2::Queued => "queued",
        MemberStatusV2::Running => "running",
        MemberStatusV2::Waiting => "waiting",
        MemberStatusV2::NeedsApproval => "waiting for approval",
        MemberStatusV2::Failed => "failed",
    }
}

fn resolve_program(name: &str) -> Option<PathBuf> {
    let path_ext = env::var_os("PATHEXT");
    let extensions = if cfg!(windows) {
        path_ext
            .as_deref()
            .unwrap_or_else(|| OsStr::new(".COM;.EXE;.BAT;.CMD"))
            .to_string_lossy()
            .split(';')
            .filter(|value| !value.is_empty())
            .map(OsString::from)
            .collect::<Vec<_>>()
    } else {
        vec![OsString::new()]
    };
    env::var_os("PATH")
        .into_iter()
        .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .find_map(|directory| {
            extensions.iter().find_map(|extension| {
                let mut file = OsString::from(name);
                file.push(extension);
                let candidate = directory.join(file);
                candidate.is_file().then_some(candidate)
            })
        })
}

fn terminal_name() -> Option<String> {
    #[cfg(windows)]
    return Some("Windows Console".to_string());
    #[cfg(target_os = "macos")]
    return Path::new("/usr/bin/osascript")
        .is_file()
        .then(|| "Terminal".to_string());
    #[cfg(all(unix, not(target_os = "macos")))]
    return [
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
    ]
    .into_iter()
    .find(|name| resolve_program(name).is_some())
    .map(str::to_string);
    #[allow(unreachable_code)]
    None
}

fn spawn_terminal(descriptor: &Path, token: &str) -> io::Result<()> {
    let executable = env::current_exe()?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        Command::new(executable)
            .arg("--attach-task")
            .arg(descriptor)
            .arg(token)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        const SCRIPT: &str = r#"on run argv
set commandText to quoted form of item 1 of argv
repeat with i from 2 to count argv
set commandText to commandText & " " & quoted form of item i of argv
end repeat
tell application "Terminal" to do script commandText
end run"#;
        Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(SCRIPT)
            .arg(&executable)
            .arg("--attach-task")
            .arg(descriptor)
            .arg(token)
            .spawn()?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let terminal = terminal_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no supported terminal"))?;
        let separator = terminal_separator(&terminal);
        let program = resolve_program(&terminal).unwrap_or_else(|| terminal.clone().into());
        let mut command = Command::new(program);
        command.arg(separator);
        command
            .arg(executable)
            .arg("--attach-task")
            .arg(descriptor)
            .arg(token)
            .spawn()?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "external terminal attach is unsupported",
    ))
}

#[cfg(any(all(unix, not(target_os = "macos")), test))]
fn terminal_separator(terminal: &str) -> &'static str {
    if terminal == "gnome-terminal" {
        "--"
    } else {
        "-e"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_arguments_do_not_pass_through_a_shell() {
        assert_eq!(
            backend_arguments(BackendKindV2::Codex, Some("thread;echo bad"), None),
            vec!["resume", "thread;echo bad"]
        );
        assert_eq!(
            backend_arguments(BackendKindV2::Claude, None, Some("fresh-id")),
            vec!["--session-id", "fresh-id"]
        );
    }

    #[test]
    fn active_members_cannot_be_attached() {
        assert_ne!(member_status_name(MemberStatusV2::Running), "idle");
        assert!(!matches!(
            MemberStatusV2::Running,
            MemberStatusV2::Idle | MemberStatusV2::Failed
        ));
    }

    #[test]
    fn gnome_terminal_uses_double_dash_separator() {
        assert_eq!(terminal_separator("gnome-terminal"), "--");
        assert_eq!(terminal_separator("x-terminal-emulator"), "-e");
    }
}
