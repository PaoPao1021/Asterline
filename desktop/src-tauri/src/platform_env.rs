#[cfg(any(unix, test))]
use std::collections::HashSet;
#[cfg(any(unix, test))]
use std::ffi::{OsStr, OsString};
#[cfg(any(unix, test))]
use std::path::PathBuf;

#[cfg(any(unix, test))]
const PATH_BEGIN: &str = "__ASTERLINE_PATH_BEGIN__";
#[cfg(any(unix, test))]
const PATH_END: &str = "__ASTERLINE_PATH_END__";

/// Restore paths normally added by a login shell when a macOS or Linux GUI
/// app is launched outside a terminal. Only PATH is merged; profile stdout and
/// all other environment changes are ignored.
pub fn restore_desktop_path() {
    #[cfg(unix)]
    if let Ok(login_path) = login_shell_path() {
        let current = std::env::var_os("PATH").unwrap_or_default();
        if let Ok(merged) = merge_paths(&login_path, &current) {
            // SAFETY: this runs at the start of `run`, before Tauri or the
            // desktop bridge creates any threads that can read the process
            // environment.
            unsafe { std::env::set_var("PATH", merged) };
        }
    }
}

#[cfg(unix)]
fn login_shell_path() -> Result<OsString, String> {
    use std::fs::{self, OpenOptions};
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(3);
    const OUTPUT_LIMIT: u64 = 64 * 1024;
    const COMMAND: &str =
        r#"printf '\n__ASTERLINE_PATH_BEGIN__%s__ASTERLINE_PATH_END__\n' "$PATH""#;

    let shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| is_supported_shell(path))
        .or_else(|| {
            ["/bin/zsh", "/bin/bash"]
                .into_iter()
                .map(PathBuf::from)
                .find(|p| p.is_file())
        })
        .ok_or_else(|| "no login shell was found".to_string())?;
    let output_path = std::env::temp_dir().join(format!(
        "asterline-path-{}-{}.txt",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let mut output_options = OpenOptions::new();
    output_options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        output_options.mode(0o600);
    }
    let output_file = output_options
        .open(&output_path)
        .map_err(|error| format!("could not create PATH output: {error}"))?;
    let mut child = Command::new(shell)
        // Interactive login mode covers both profile files and the shell rc
        // files where npm/nvm/Volta commonly add Codex or Claude to PATH.
        .args(["-l", "-i", "-c", COMMAND])
        .stdin(Stdio::null())
        // A regular file is deliberate. A reader waiting for pipe EOF can
        // hang forever when a shell profile starts a background child that
        // inherits stdout, even after the login shell itself exits.
        .stdout(Stdio::from(output_file))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            let _ = fs::remove_file(&output_path);
            format!("could not start login shell: {error}")
        })?;
    let status = wait_with_timeout(&mut child, TIMEOUT);
    let result = status.and_then(|status| {
        if !status.success() {
            return Err(format!("login shell exited with {status}"));
        }
        let mut output = Vec::new();
        fs::File::open(&output_path)
            .and_then(|file| file.take(OUTPUT_LIMIT + 1).read_to_end(&mut output))
            .map_err(|error| format!("could not read login shell PATH: {error}"))?;
        if output.len() > OUTPUT_LIMIT as usize {
            return Err("login shell PATH output exceeded 64 KiB".to_string());
        }
        parse_marked_path(&output).map(OsString::from)
    });
    let _ = fs::remove_file(output_path);
    result
}

#[cfg(unix)]
fn is_supported_shell(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path.is_file()
        && matches!(
            path.file_name().and_then(OsStr::to_str),
            Some("zsh" | "bash")
        )
}

#[cfg(unix)]
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::ExitStatus, String> {
    use std::time::Instant;

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("login shell PATH lookup timed out".to_string());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("could not wait for login shell: {error}"));
            }
        }
    }
}

#[cfg(any(unix, test))]
fn parse_marked_path(output: &[u8]) -> Result<String, String> {
    let output = String::from_utf8_lossy(output);
    let begin = output
        .rfind(PATH_BEGIN)
        .ok_or_else(|| "login shell PATH marker was missing".to_string())?
        + PATH_BEGIN.len();
    let rest = &output[begin..];
    let end = rest
        .find(PATH_END)
        .ok_or_else(|| "login shell PATH end marker was missing".to_string())?;
    let value = &rest[..end];
    if value.is_empty() || value.chars().any(|ch| matches!(ch, '\r' | '\n' | '\0')) {
        return Err("login shell returned an invalid PATH".to_string());
    }
    Ok(value.to_string())
}

#[cfg(any(unix, test))]
fn merge_paths(login: &OsStr, current: &OsStr) -> Result<OsString, String> {
    let mut seen = HashSet::new();
    let mut paths = Vec::<PathBuf>::new();
    for path in std::env::split_paths(login).chain(std::env::split_paths(current)) {
        if path.as_os_str().is_empty() {
            continue;
        }
        let key = path.to_string_lossy().into_owned();
        if seen.insert(key) {
            paths.push(path);
        }
    }
    std::env::join_paths(paths).map_err(|error| format!("could not merge login PATH: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_ignores_profile_chatter_and_uses_last_marker() {
        let output = format!(
            "profile text\n{PATH_BEGIN}/bad{PATH_END}\nmore\n{PATH_BEGIN}/usr/local/bin:/usr/bin{PATH_END}\n"
        );
        assert_eq!(
            parse_marked_path(output.as_bytes()).unwrap(),
            "/usr/local/bin:/usr/bin"
        );
    }

    #[test]
    fn path_merge_prefers_login_entries_and_deduplicates() {
        let root = std::env::temp_dir();
        let first = root.join("asterline-login-bin");
        let second = root.join("asterline-current-bin");
        let login = std::env::join_paths([&first, &second]).unwrap();
        let current = std::env::join_paths([&second]).unwrap();
        let merged = merge_paths(&login, &current).unwrap();
        assert_eq!(
            std::env::split_paths(&merged).collect::<Vec<_>>(),
            vec![first, second]
        );
    }

    #[cfg(unix)]
    #[test]
    fn child_wait_has_a_real_timeout() {
        use std::process::Command;
        use std::time::{Duration, Instant};

        let mut child = Command::new("/bin/sh")
            .args(["-c", "sleep 2"])
            .spawn()
            .unwrap();
        let started = Instant::now();
        assert!(wait_with_timeout(&mut child, Duration::from_millis(50)).is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
