//! Read-only query DTOs for the GUI: the shared command catalog is exposed by
//! `composer`, this module covers backend CLI availability, per-backend model
//! catalogs, and native session summaries for import and session search.

use std::path::Path;

use serde::Serialize;

use asterline::domain::config::detect_backends;
use asterline::domain::team::{BackendKind, Effort};
use asterline::tui::native_sessions::{self, NativeSessionSummary};

use crate::bridge::BackendKindV2;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BackendAvailabilityV2 {
    pub codex: bool,
    pub claude: bool,
    pub grok: bool,
    pub agy: bool,
}

/// Which backend CLIs are currently on `PATH` (drives setup hints and the
/// team settings install checks).
pub fn backend_availability() -> BackendAvailabilityV2 {
    let detected = detect_backends();
    BackendAvailabilityV2 {
        codex: detected.codex,
        claude: detected.claude,
        grok: detected.grok,
        agy: detected.agy,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModelSummaryV2 {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_effort: Option<String>,
    pub supported_efforts: Vec<String>,
    pub is_default: bool,
}

/// Discover the model catalog of one backend CLI (bounded by the shared
/// discovery timeout). Manual model entry stays available in the GUI.
pub fn list_models(backend: BackendKindV2, cwd: &str) -> Result<Vec<ModelSummaryV2>, String> {
    let models =
        asterline::adapter::models::discover_models(domain_backend(backend), Path::new(cwd))?;
    Ok(models
        .into_iter()
        .map(|model| ModelSummaryV2 {
            id: model.id,
            name: model.name,
            description: model.description,
            default_effort: model.default_effort.map(effort_str),
            supported_efforts: model
                .supported_efforts
                .into_iter()
                .map(effort_str)
                .collect(),
            is_default: model.is_default,
        })
        .collect())
}

/// List native sessions of one backend for import pickers and search.
pub fn list_native_sessions(backend: BackendKindV2, cwd: &str) -> Vec<NativeSessionSummary> {
    native_sessions::list_sessions(domain_backend(backend), cwd)
}

fn domain_backend(backend: BackendKindV2) -> BackendKind {
    match backend {
        BackendKindV2::Codex => BackendKind::Codex,
        BackendKindV2::Claude => BackendKind::Claude,
        BackendKindV2::Grok => BackendKind::Grok,
        BackendKindV2::Agy => BackendKind::Agy,
    }
}

fn effort_str(effort: Effort) -> String {
    effort.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_reports_four_backends() {
        let availability = backend_availability();
        // All fields must be present; on CI none of the CLIs may exist.
        let _ = (
            availability.codex,
            availability.claude,
            availability.grok,
            availability.agy,
        );
    }

    #[test]
    fn native_session_listing_never_panics_on_missing_homes() {
        let sessions = list_native_sessions(BackendKindV2::Claude, "/definitely/not/a/workspace");
        assert!(sessions.len() <= 500);
    }
}
