use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

const RELEASES_API: &str = "https://api.github.com/repos/song0705/Asterline/releases?per_page=30";
const RELEASE_URL_PREFIX: &str = "https://github.com/song0705/Asterline/releases/";
const DESKTOP_TAG_PREFIX: &str = "desktop-v";
const RESPONSE_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DesktopUpdateV1 {
    pub current_version: String,
    pub available_version: Option<String>,
    pub release_url: Option<String>,
    pub update_available: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

pub fn check() -> DesktopUpdateV1 {
    let current_text = env!("CARGO_PKG_VERSION").to_string();
    match check_inner() {
        Ok(Some((available, url))) => {
            let current = Version::parse(&current_text).unwrap_or_else(|_| Version::new(0, 0, 0));
            DesktopUpdateV1 {
                current_version: current_text,
                available_version: Some(available.to_string()),
                release_url: Some(url),
                update_available: available > current,
                error: None,
            }
        }
        Ok(None) => DesktopUpdateV1 {
            current_version: current_text,
            available_version: None,
            release_url: None,
            update_available: false,
            error: None,
        },
        Err(error) => DesktopUpdateV1 {
            current_version: current_text,
            available_version: None,
            release_url: None,
            update_available: false,
            error: Some(error),
        },
    }
}

fn check_inner() -> Result<Option<(Version, String)>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        )
        .build()
        .new_agent();
    let text = agent
        .get(RELEASES_API)
        .header(
            "User-Agent",
            concat!("Asterline-Desktop/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|error| format!("release request failed: {error}"))?
        .body_mut()
        .with_config()
        .limit(RESPONSE_LIMIT)
        .read_to_string()
        .map_err(|error| format!("could not read release response: {error}"))?;
    let releases: Vec<Release> = serde_json::from_str(&text)
        .map_err(|error| format!("could not parse release response: {error}"))?;
    Ok(latest_desktop_release(releases))
}

fn latest_desktop_release(releases: Vec<Release>) -> Option<(Version, String)> {
    releases
        .into_iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            let version = release.tag_name.strip_prefix(DESKTOP_TAG_PREFIX)?;
            let version = Version::parse(version).ok()?;
            if !release.html_url.starts_with(RELEASE_URL_PREFIX) {
                return None;
            }
            Some((version, release.html_url))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_filter_ignores_cli_drafts_and_untrusted_urls() {
        let releases = vec![
            Release {
                tag_name: "v9.0.0".to_string(),
                html_url: format!("{RELEASE_URL_PREFIX}tag/v9.0.0"),
                draft: false,
                prerelease: false,
            },
            Release {
                tag_name: "desktop-v3.0.0".to_string(),
                html_url: "https://example.com/malicious".to_string(),
                draft: false,
                prerelease: false,
            },
            Release {
                tag_name: "desktop-v2.0.0".to_string(),
                html_url: format!("{RELEASE_URL_PREFIX}tag/desktop-v2.0.0"),
                draft: false,
                prerelease: false,
            },
        ];
        let (version, _) = latest_desktop_release(releases).unwrap();
        assert_eq!(version, Version::new(2, 0, 0));
    }
}
