use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const LOG_FILE: &str = "asterline-desktop.log";
const PREVIOUS_LOG_FILE: &str = "asterline-desktop.previous.log";
const RUN_MARKER: &str = "running.marker";
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticsStatusV1 {
    pub previous_unclean_exit: bool,
    pub log_path: String,
}

struct DiagnosticsInner {
    log_path: PathBuf,
    marker_path: PathBuf,
    previous_unclean_exit: bool,
}

pub struct Diagnostics {
    inner: Mutex<DiagnosticsInner>,
}

impl Diagnostics {
    pub fn initialize(app: &AppHandle) -> Result<Self, String> {
        let directory = app
            .path()
            .app_log_dir()
            .map_err(|error| format!("could not resolve diagnostics directory: {error}"))?;
        Self::initialize_in_directory(&directory)
    }

    fn initialize_in_directory(directory: &Path) -> Result<Self, String> {
        fs::create_dir_all(directory)
            .map_err(|error| format!("could not create diagnostics directory: {error}"))?;

        let log_path = directory.join(LOG_FILE);
        rotate_if_needed(&log_path)
            .map_err(|error| format!("could not rotate diagnostics log: {error}"))?;
        let marker_path = directory.join(RUN_MARKER);
        let previous_unclean_exit = marker_path.exists();
        fs::write(
            &marker_path,
            format!(
                "pid={}\nversion={}\n",
                std::process::id(),
                env!("CARGO_PKG_VERSION")
            ),
        )
        .map_err(|error| format!("could not create run marker: {error}"))?;

        append_line(&log_path, "info", "desktop", "application starting")
            .map_err(|error| format!("could not initialize diagnostics log: {error}"))?;
        if previous_unclean_exit {
            let _ = append_line(
                &log_path,
                "warn",
                "desktop",
                "the previous process did not complete a clean shutdown",
            );
        }

        Ok(Self {
            inner: Mutex::new(DiagnosticsInner {
                log_path,
                marker_path,
                previous_unclean_exit,
            }),
        })
    }

    pub fn install_panic_hook(&self) {
        let Ok(inner) = self.inner.lock() else {
            return;
        };
        let log_path = inner.log_path.clone();
        drop(inner);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = append_line(&log_path, "error", "panic", &info.to_string());
            previous(info);
        }));
    }

    pub fn status(&self) -> Result<DiagnosticsStatusV1, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "diagnostics state is unavailable".to_string())?;
        Ok(DiagnosticsStatusV1 {
            previous_unclean_exit: inner.previous_unclean_exit,
            log_path: inner.log_path.to_string_lossy().into_owned(),
        })
    }

    pub fn write(&self, level: &str, source: &str, message: &str) {
        if let Ok(inner) = self.inner.lock() {
            let _ = append_line(&inner.log_path, level, source, message);
        }
    }

    pub fn mark_clean(&self) {
        if let Ok(inner) = self.inner.lock() {
            let _ = append_line(
                &inner.log_path,
                "info",
                "desktop",
                "clean shutdown completed",
            );
            match fs::remove_file(&inner.marker_path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    let _ = append_line(
                        &inner.log_path,
                        "warn",
                        "desktop",
                        &format!("could not remove run marker: {error}"),
                    );
                }
            }
        }
    }

    pub fn export(&self, app: &AppHandle) -> Result<String, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "diagnostics state is unavailable".to_string())?;
        let contents = fs::read_to_string(&inner.log_path)
            .unwrap_or_else(|error| format!("diagnostics log could not be read: {error}\n"));
        let previous_path = inner.log_path.with_file_name(PREVIOUS_LOG_FILE);
        let previous_contents = fs::read_to_string(&previous_path).ok();
        let destination = app.path().download_dir().unwrap_or_else(|_| {
            inner
                .log_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        });
        fs::create_dir_all(&destination)
            .map_err(|error| format!("could not create diagnostics export directory: {error}"))?;
        let stamp = OffsetDateTime::now_utc()
            .format(&time::macros::format_description!(
                "[year][month][day]-[hour][minute][second]"
            ))
            .unwrap_or_else(|_| "unknown-time".to_string());
        let path = destination.join(format!("asterline-desktop-diagnostics-{stamp}.txt"));
        let mut report = format!(
            "Asterline Desktop diagnostics\nversion={}\nos={}\narch={}\nprevious_unclean_exit={}\n\n{}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            inner.previous_unclean_exit,
            contents,
        );
        if let Some(previous) = previous_contents {
            report.push_str("\n\n--- previous log ---\n");
            report.push_str(&previous);
        }
        fs::write(&path, report)
            .map_err(|error| format!("could not export diagnostics: {error}"))?;
        Ok(path.to_string_lossy().into_owned())
    }
}

fn rotate_if_needed(path: &Path) -> io::Result<()> {
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES)
    {
        let previous = path.with_file_name(PREVIOUS_LOG_FILE);
        if previous.exists() {
            fs::remove_file(&previous)?;
        }
        fs::rename(path, previous)?;
    }
    Ok(())
}

fn append_line(path: &Path, level: &str, source: &str, message: &str) -> io::Result<()> {
    rotate_if_needed(path)?;
    let mut output = OpenOptions::new().create(true).append(true).open(path)?;
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown-time".to_string());
    let line = format!(
        "{} [{}] {}: {}\n",
        timestamp,
        single_line(level, 16),
        single_line(source, 96),
        single_line(message, 4096),
    );
    output.write_all(line.as_bytes())?;
    output.flush()
}

fn single_line(value: &str, limit: usize) -> String {
    value
        .chars()
        .map(|ch| if ch == '\r' || ch == '\n' { ' ' } else { ch })
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "asterline-desktop-diagnostics-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn log_fields_are_single_line_and_bounded() {
        assert_eq!(single_line("one\r\ntwo", 20), "one  two");
        assert_eq!(single_line("abcdef", 3), "abc");
    }

    #[test]
    fn a_stale_marker_is_reported_and_clean_shutdown_removes_it() {
        let directory = test_directory("marker");
        let first = Diagnostics::initialize_in_directory(&directory).unwrap();
        assert!(!first.status().unwrap().previous_unclean_exit);
        drop(first);

        let second = Diagnostics::initialize_in_directory(&directory).unwrap();
        assert!(second.status().unwrap().previous_unclean_exit);
        second.mark_clean();
        assert!(!directory.join(RUN_MARKER).exists());
        drop(second);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn append_rotates_a_full_log_during_the_running_process() {
        let directory = test_directory("rotation");
        fs::create_dir_all(&directory).unwrap();
        let log = directory.join(LOG_FILE);
        fs::write(&log, vec![b'x'; MAX_LOG_BYTES as usize]).unwrap();

        append_line(&log, "info", "test", "after rotation").unwrap();

        assert!(log.metadata().unwrap().len() < MAX_LOG_BYTES);
        assert_eq!(
            directory.join(PREVIOUS_LOG_FILE).metadata().unwrap().len(),
            MAX_LOG_BYTES
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
