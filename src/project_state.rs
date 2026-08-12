//! Workspace-level project-state compatibility marker.
//!
//! This format version covers the files Asterline owns as a set. SQLite keeps
//! its own, more granular `PRAGMA user_version` migration sequence.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

pub const PROJECT_STATE_FORMAT: u32 = 1;
pub const PROJECT_STATE_PATH: &str = ".asterline/project.json";

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ProjectStateManifest {
    project_state_format: u32,
}

/// Validate an existing workspace marker or create the V1 marker atomically.
/// Callers must hold the workspace instance lock before invoking this function.
pub fn ensure_project_state(workspace: &Path) -> io::Result<()> {
    let path = workspace.join(PROJECT_STATE_PATH);
    match read_manifest(&path) {
        Ok(manifest) => validate_manifest(&path, manifest),
        Err(error) if error.kind() == io::ErrorKind::NotFound => write_new_manifest(&path),
        Err(error) => Err(error),
    }
}

fn read_manifest(path: &Path) -> io::Result<ProjectStateManifest> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid project state marker {}: {error}", path.display()),
        )
    })
}

fn validate_manifest(path: &Path, manifest: ProjectStateManifest) -> io::Result<()> {
    if manifest.project_state_format == PROJECT_STATE_FORMAT {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "project state format {} in {} is unsupported; this build supports format {}",
            manifest.project_state_format,
            path.display(),
            PROJECT_STATE_FORMAT
        ),
    ))
}

fn write_new_manifest(path: &Path) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "project state marker must have a parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec_pretty(&ProjectStateManifest {
        project_state_format: PROJECT_STATE_FORMAT,
    })
    .map_err(io::Error::other)?;

    let temp_path = unique_temp_path(parent);
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);

        // The workspace lock excludes another Asterline writer. Recheck to
        // avoid replacing a marker created by an unrelated process.
        if path.exists() {
            return validate_manifest(path, read_manifest(path)?);
        }
        fs::rename(&temp_path, path)?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn unique_temp_path(parent: &Path) -> PathBuf {
    parent.join(format!(
        ".project.json.{}-{}.tmp",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "asterline-project-state-{name}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn creates_and_reopens_v1_marker() {
        let workspace = temp_workspace("create");
        ensure_project_state(&workspace).unwrap();
        ensure_project_state(&workspace).unwrap();

        let marker = read_manifest(&workspace.join(PROJECT_STATE_PATH)).unwrap();
        assert_eq!(marker.project_state_format, PROJECT_STATE_FORMAT);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn rejects_future_marker_without_replacing_it() {
        let workspace = temp_workspace("future");
        let path = workspace.join(PROJECT_STATE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = format!(
            "{{\"project_state_format\":{},\"future_field\":true}}",
            PROJECT_STATE_FORMAT + 1
        );
        fs::write(&path, &original).unwrap();

        let error = ensure_project_state(&workspace).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("unsupported"));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn rejects_malformed_marker() {
        let workspace = temp_workspace("malformed");
        let path = workspace.join(PROJECT_STATE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();

        let error = ensure_project_state(&workspace).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("invalid project state marker"));
        let _ = fs::remove_dir_all(workspace);
    }
}
