//! Native session discovery for import pickers and session search.
//!
//! The desktop import/team-settings surfaces need to browse existing backend
//! sessions the same way `/import` accepts them. Summaries are display-only:
//! id, a short first-user-message preview, and the last-modified time.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;

use crate::domain::team::BackendKind;
use crate::tui::import_io;

const MAX_LISTED_SESSIONS: usize = 500;
const MAX_PREVIEW_CHARS: usize = 200;
const MAX_SESSION_SCAN_DEPTH: usize = 6;
const MAX_SESSION_SCAN_ENTRIES: usize = 20_000;

/// One browsable native session of a backend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NativeSessionSummary {
    pub backend: BackendKind,
    pub session_id: String,
    pub preview: String,
    /// Last modified time, in seconds since the Unix epoch.
    pub modified_secs: u64,
}

/// List existing native sessions of `backend` for the workspace `cwd`.
pub fn list_sessions(backend: BackendKind, cwd: &str) -> Vec<NativeSessionSummary> {
    let home = crate::domain::config::user_home_dir();
    list_sessions_with_homes(
        backend,
        cwd,
        home.as_deref()
            .map(|home| home.join(".claude").join("projects"))
            .as_deref(),
        home.as_deref()
            .map(|home| home.join(".codex").join("sessions"))
            .as_deref(),
        home.as_deref()
            .map(|home| home.join(".grok").join("sessions"))
            .as_deref(),
    )
}

/// Testable core: roots default to `$HOME`-derived paths in [`list_sessions`].
pub(crate) fn list_sessions_with_homes(
    backend: BackendKind,
    cwd: &str,
    claude_projects_root: Option<&Path>,
    codex_sessions_root: Option<&Path>,
    grok_sessions_root: Option<&Path>,
) -> Vec<NativeSessionSummary> {
    let mut sessions = match backend {
        BackendKind::Claude => claude_projects_root
            .filter(|root| root.is_dir())
            .map(|root| list_claude_sessions(root, cwd))
            .unwrap_or_default(),
        BackendKind::Codex => codex_sessions_root
            .filter(|root| root.is_dir())
            .map(|root| list_codex_sessions(root, cwd))
            .unwrap_or_default(),
        BackendKind::Grok => grok_sessions_root
            .filter(|root| root.is_dir())
            .map(list_grok_sessions)
            .unwrap_or_default(),
        BackendKind::Agy => Vec::new(),
    };
    sessions.sort_by_key(|session| std::cmp::Reverse(session.modified_secs));
    sessions.truncate(MAX_LISTED_SESSIONS);
    sessions
}

fn summary(
    backend: BackendKind,
    session_id: String,
    preview: Option<String>,
    modified: Option<SystemTime>,
) -> Option<NativeSessionSummary> {
    let session_id = session_id.trim().to_string();
    if !valid_session_id(&session_id) {
        return None;
    }
    Some(NativeSessionSummary {
        backend,
        session_id,
        preview: preview
            .map(|text| bounded_preview(&text))
            .unwrap_or_default(),
        modified_secs: modified
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    })
}

/// Session ids travel as CLI arguments and file names: keep them short and
/// free of separators or control characters.
fn valid_session_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id.len() <= 256
        && session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn bounded_preview(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut end = collapsed.len().min(MAX_PREVIEW_CHARS * 4);
    while end > 0 && !collapsed.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = &collapsed[..end];
    let mut chars = truncated.chars();
    let mut out = String::new();
    for _ in 0..MAX_PREVIEW_CHARS {
        match chars.next() {
            Some(character) => out.push(character),
            None => return out,
        }
    }
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

// --- Claude ---------------------------------------------------------------

fn list_claude_sessions(projects_root: &Path, cwd: &str) -> Vec<NativeSessionSummary> {
    let project = projects_root.join(munge_cwd(cwd));
    list_claude_in_project(&project)
}

pub(crate) fn list_claude_in_project(project: &Path) -> Vec<NativeSessionSummary> {
    let Ok(entries) = fs::read_dir(project) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(session_id) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if !valid_session_id(session_id) {
            continue;
        }
        sessions.push(
            summary(
                BackendKind::Claude,
                session_id.to_string(),
                claude_preview(&path),
                fs::metadata(&path).and_then(|meta| meta.modified()).ok(),
            )
            .filter(|candidate| !candidate.preview.is_empty()),
        );
        if sessions.len() >= MAX_LISTED_SESSIONS {
            return sessions.into_iter().flatten().collect();
        }
    }
    sessions.into_iter().flatten().collect()
}

fn claude_preview(path: &Path) -> Option<String> {
    let mut preview = None;
    import_io::for_each_json_value_limit(path, 200, |value| {
        if let Some(text) = claude_user_text(&value) {
            preview = Some(text);
            return false;
        }
        true
    });
    preview
}

fn claude_user_text(value: &Value) -> Option<String> {
    if value.get("isSidechain").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    if value.get("type").and_then(Value::as_str) != Some("user") {
        return None;
    }
    let content = value.get("message")?.get("content")?;
    let text = match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            if blocks
                .iter()
                .any(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
            {
                return None;
            }
            let mut text = String::new();
            for block in blocks {
                if let Some(part) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(part);
                }
            }
            Some(text)
        }
        _ => None,
    }?;
    let text = import_io::imported_text(text)?;
    if text.trim_start().starts_with("<command-name>")
        || text.trim_start().starts_with("<local-command-stdout")
        || text.trim_start().starts_with("Caveat:")
    {
        return None;
    }
    Some(text)
}

/// Replace every character that is not `[A-Za-z0-9]` with `-`, matching
/// Claude's own project directory encoding.
pub(crate) fn munge_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

// --- Codex ----------------------------------------------------------------

fn list_codex_sessions(sessions_root: &Path, cwd: &str) -> Vec<NativeSessionSummary> {
    let mut sessions = Vec::new();
    let mut remaining = MAX_SESSION_SCAN_ENTRIES;
    scan_rollouts(sessions_root, cwd, 0, &mut sessions, &mut remaining);
    sessions
}

fn scan_rollouts(
    dir: &Path,
    cwd: &str,
    depth: usize,
    sessions: &mut Vec<NativeSessionSummary>,
    remaining: &mut usize,
) {
    if depth > MAX_SESSION_SCAN_DEPTH || *remaining == 0 || sessions.len() >= MAX_LISTED_SESSIONS {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        *remaining = remaining.saturating_sub(1);
        let path = entry.path();
        if path.is_dir() {
            scan_rollouts(&path, cwd, depth + 1, sessions, remaining);
            if sessions.len() >= MAX_LISTED_SESSIONS || *remaining == 0 {
                return;
            }
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl")
            || !path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("rollout-"))
        {
            continue;
        }
        let Some((session_id, session_cwd, preview)) = codex_intro(&path) else {
            continue;
        };
        if !paths_equal(&session_cwd, cwd) {
            continue;
        }
        if let Some(candidate) = summary(
            BackendKind::Codex,
            session_id,
            preview,
            fs::metadata(&path).and_then(|meta| meta.modified()).ok(),
        ) {
            sessions.push(candidate);
        }
        if sessions.len() >= MAX_LISTED_SESSIONS || *remaining == 0 {
            return;
        }
    }
}

/// Read the rollout header: `(session id, cwd, first user message)`.
fn codex_intro(path: &Path) -> Option<(String, String, Option<String>)> {
    let mut session_id = None;
    let mut session_cwd = None;
    let mut preview = None;
    import_io::for_each_json_value_limit(path, 40, |value| {
        if session_id.is_none() {
            let id = value
                .get("payload")
                .and_then(|payload| payload.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    value
                        .get("session_id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
            if let Some(id) = id {
                session_id = Some(id);
            }
            if session_cwd.is_none() {
                session_cwd = value
                    .get("payload")
                    .and_then(|payload| payload.get("cwd"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
        }
        if preview.is_none()
            && value.get("type").and_then(Value::as_str) == Some("response_item")
            && value
                .get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str)
                == Some("message")
            && value
                .get("payload")
                .and_then(|payload| payload.get("role"))
                .and_then(Value::as_str)
                == Some("user")
            && let Some(text) = codex_input_text(
                value
                    .get("payload")
                    .and_then(|payload| payload.get("content")),
            )
            && !text.trim().is_empty()
        {
            preview = Some(text);
        }
        !(session_id.is_some() && preview.is_some())
    });
    let session_id = session_id?;
    Some((session_id, session_cwd.unwrap_or_default(), preview))
}

fn codex_input_text(content: Option<&Value>) -> Option<String> {
    let content = content?.as_array()?;
    let mut text = String::new();
    for block in content {
        if let Some(part) = block.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(part);
        }
    }
    Some(text)
}

// --- Grok -----------------------------------------------------------------

fn list_grok_sessions(sessions_root: &Path) -> Vec<NativeSessionSummary> {
    let mut sessions = Vec::new();
    let mut remaining = MAX_SESSION_SCAN_ENTRIES;
    scan_grok_histories(sessions_root, 0, &mut sessions, &mut remaining);
    sessions
}

fn scan_grok_histories(
    dir: &Path,
    depth: usize,
    sessions: &mut Vec<NativeSessionSummary>,
    remaining: &mut usize,
) {
    if depth > MAX_SESSION_SCAN_DEPTH || *remaining == 0 || sessions.len() >= MAX_LISTED_SESSIONS {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        *remaining = remaining.saturating_sub(1);
        let path = entry.path();
        if path.is_dir() {
            scan_grok_histories(&path, depth + 1, sessions, remaining);
            if sessions.len() >= MAX_LISTED_SESSIONS || *remaining == 0 {
                return;
            }
            continue;
        }
        if path.file_name().and_then(|value| value.to_str()) != Some("chat_history.jsonl") {
            continue;
        }
        let Some(session_id) = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let preview = grok_preview(&path);
        let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok();
        if let Some(candidate) = summary(BackendKind::Grok, session_id, preview, modified) {
            sessions.push(candidate);
        }
        if sessions.len() >= MAX_LISTED_SESSIONS || *remaining == 0 {
            return;
        }
    }
}

fn grok_preview(path: &Path) -> Option<String> {
    let mut preview = None;
    import_io::for_each_json_value_limit(path, 200, |value| {
        let content = value.get("content");
        if value.get("type").and_then(Value::as_str) == Some("user")
            && value.get("synthetic_reason").is_none()
            && let Some(text) = content.and_then(grok_user_text)
        {
            preview = Some(text);
            return false;
        }
        true
    });
    preview
}

fn grok_user_text(content: &Value) -> Option<String> {
    let text = match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let mut text = String::new();
            for block in blocks {
                if let Some(part) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(part);
                }
            }
            Some(text)
        }
        _ => None,
    }?;
    let text = import_io::imported_text(text)?;
    if text.trim_start().starts_with('<') {
        return None;
    }
    Some(text)
}

fn paths_equal(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.replace('/', "\\")
            .eq_ignore_ascii_case(&b.replace('/', "\\"))
    } else {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn bounded_preview_is_short_and_single_line() {
        let preview = bounded_preview("hello\n  world   this is a longer preview\ttext");
        assert!(!preview.contains('\n'));
        assert!(preview.len() <= MAX_PREVIEW_CHARS * 4 + 3);
        let long = bounded_preview(&"x".repeat(5000));
        assert!(long.chars().count() <= MAX_PREVIEW_CHARS + 1);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn munging_matches_claude_project_dirs() {
        assert_eq!(munge_cwd(r"C:\Users\Ada\repo"), "C--Users-Ada-repo");
        assert_eq!(munge_cwd("/tmp/proj.name"), "-tmp-proj-name");
    }

    #[test]
    fn claude_sessions_list_with_preview_sorted_by_recency() {
        let root = std::env::temp_dir().join(format!("asterline-sessions-{}", std::process::id()));
        let project = root.join(munge_cwd("/work/demo"));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &project.join("sess-old.jsonl"),
            concat!(
                r#"{"type":"user","message":{"content":"fix the parser"}}"#,
                "\n",
                r#"{"type":"assistant","message":{"content":"on it"}}"#,
                "\n"
            ),
        );
        write(
            &project.join("sess-new.jsonl"),
            concat!(
                r#"{"type":"user","message":{"content":"second request"}}"#,
                "\n"
            ),
        );
        // Ensure distinct mtimes.
        std::thread::sleep(std::time::Duration::from_millis(20));
        write(
            &project.join("sess-new.jsonl"),
            concat!(
                r#"{"type":"user","message":{"content":"second request"}}"#,
                "\n"
            ),
        );

        let sessions =
            list_sessions_with_homes(BackendKind::Claude, "/work/demo", Some(&root), None, None);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "sess-new");
        assert_eq!(sessions[0].preview, "second request");
        assert_eq!(sessions[1].session_id, "sess-old");
        assert_eq!(sessions[1].preview, "fix the parser");
        assert!(sessions[0].modified_secs >= sessions[1].modified_secs);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn claude_sessions_ignore_sidechains_and_meta_rows() {
        let root =
            std::env::temp_dir().join(format!("asterline-sessions-meta-{}", std::process::id()));
        let project = root.join(munge_cwd("/work/demo"));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &project.join("sess-meta.jsonl"),
            concat!(
                r#"{"type":"user","message":{"content":"<command-name>/new</command-name>"}}"#,
                "\n",
                r#"{"isSidechain":true,"type":"user","message":{"content":"hidden"}}"#,
                "\n",
                r#"{"type":"user","message":{"content":[{"type":"text","text":"real ask"}]}}"#,
                "\n"
            ),
        );
        let sessions =
            list_sessions_with_homes(BackendKind::Claude, "/work/demo", Some(&root), None, None);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].preview, "real ask");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn codex_sessions_match_cwd_and_expose_ids() {
        let root =
            std::env::temp_dir().join(format!("asterline-sessions-codex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root.join("2026/09/05/rollout-20260905T101010-abc-123.jsonl"),
            concat!(
                r#"{"payload":{"id":"abc-123","cwd":"/work/demo"}}"#,
                "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"codex ask"}]}}"#,
                "\n"
            ),
        );
        write(
            &root.join("2026/09/05/rollout-20260905T090909-other-999.jsonl"),
            concat!(
                r#"{"payload":{"id":"other-999","cwd":"/other/place"}}"#,
                "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"elsewhere"}]}}"#,
                "\n"
            ),
        );

        let sessions =
            list_sessions_with_homes(BackendKind::Codex, "/work/demo", None, Some(&root), None);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "abc-123");
        assert_eq!(sessions[0].preview, "codex ask");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn grok_sessions_list_by_directory_id() {
        let root =
            std::env::temp_dir().join(format!("asterline-sessions-grok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root
                .join("%2Fwork%2Fdemo")
                .join("grok-sess-1")
                .join("chat_history.jsonl"),
            concat!(
                r#"{"type":"user","content":"grok ask"}"#,
                "\n",
                r#"{"type":"assistant","content":"reply"}"#,
                "\n"
            ),
        );
        let sessions =
            list_sessions_with_homes(BackendKind::Grok, "/work/demo", None, None, Some(&root));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "grok-sess-1");
        assert_eq!(sessions[0].preview, "grok ask");
        let _ = std::fs::remove_dir_all(&root);
    }
}
