use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const RECENT_FILE: &str = "recent-workspaces.json";
const RECENT_LIMIT: usize = 12;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecentWorkspaceV2 {
    pub workspace: String,
    pub name: Option<String>,
    pub last_opened_at: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RecentFile {
    #[serde(default = "recent_version")]
    version: u8,
    #[serde(default)]
    workspaces: Vec<RecentWorkspaceV2>,
}

fn recent_version() -> u8 {
    1
}

pub fn list(app: &AppHandle) -> Result<Vec<RecentWorkspaceV2>, String> {
    let path = config_path(app)?;
    load(&path)
        .map(|file| file.workspaces)
        .map_err(|error| format!("could not read recent workspaces: {error}"))
}

pub fn record(app: &AppHandle, workspace: &Path) -> Result<(), String> {
    let path = config_path(app)?;
    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let workspace_text = canonical.to_string_lossy().into_owned();
    let key = path_key(&canonical);
    let mut file =
        load(&path).map_err(|error| format!("could not read recent workspaces: {error}"))?;
    file.workspaces
        .retain(|item| path_key(Path::new(&item.workspace)) != key);
    file.workspaces.insert(
        0,
        RecentWorkspaceV2 {
            workspace: workspace_text,
            name: canonical
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            last_opened_at: Some(now_rfc3339()),
        },
    );
    file.workspaces.truncate(RECENT_LIMIT);
    save(&path, &file).map_err(|error| format!("could not save recent workspaces: {error}"))
}

pub fn forget(app: &AppHandle, workspace: &str) -> Result<(), String> {
    let path = config_path(app)?;
    let key = path_key(Path::new(workspace));
    let mut file =
        load(&path).map_err(|error| format!("could not read recent workspaces: {error}"))?;
    file.workspaces
        .retain(|item| path_key(Path::new(&item.workspace)) != key);
    save(&path, &file).map_err(|error| format!("could not save recent workspaces: {error}"))
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(RECENT_FILE))
        .map_err(|error| format!("could not resolve application config directory: {error}"))
}

fn load(path: &Path) -> io::Result<RecentFile> {
    if !path.is_file() {
        return Ok(RecentFile {
            version: recent_version(),
            workspaces: Vec::new(),
        });
    }
    let text = fs::read_to_string(path)?;
    let file = serde_json::from_str::<RecentFile>(&text)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if file.version != recent_version() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported recent-workspace format {}", file.version),
        ));
    }
    Ok(file)
}

fn save(path: &Path, file: &RecentFile) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(file)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    AtomicFile::new(path, AllowOverwrite)
        .write(|output| {
            use std::io::Write;
            output.write_all(&bytes)?;
            output.flush()?;
            output.sync_all()
        })
        .map_err(|error| match error {
            atomicwrites::Error::Internal(error) | atomicwrites::Error::User(error) => error,
        })
}

fn path_key(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let value = canonical.to_string_lossy().into_owned();
    if cfg!(windows) {
        value.replace('/', "\\").to_lowercase()
    } else {
        value
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_file_round_trips_without_project_content() {
        let path = std::env::temp_dir().join(format!(
            "asterline-desktop-recent-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = RecentFile {
            version: 1,
            workspaces: vec![RecentWorkspaceV2 {
                workspace: "C:\\workspace".to_string(),
                name: Some("workspace".to_string()),
                last_opened_at: Some("2026-08-12T00:00:00Z".to_string()),
            }],
        };
        save(&path, &file).unwrap();
        assert_eq!(load(&path).unwrap().workspaces, file.workspaces);
        let _ = fs::remove_file(path);
    }
}
