//! Managed image attachments for the desktop composer.
//!
//! Images never travel as web paths: the host persists them into the same
//! managed paste directory the TUI uses (`%TEMP%/asterline-pasted/<pid>`) and
//! hands the WebView an opaque token. On send, `user_message` tokens are
//! resolved back to files inside that directory and become native multimodal
//! input for each backend. Removal rules mirror the TUI: user-removed
//! attachments are deleted immediately; everything left is wiped when the
//! desktop session ends (or sweeps itself as an orphan after a crash).

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use asterline::adapter::prompt_images::PromptImage;
use asterline::tui::clipboard_image;

/// One staged attachment as the WebView sees it: a token, a display label,
/// and the MIME type. No absolute paths ever leave the host.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StagedAttachmentV2 {
    pub token: String,
    pub label: String,
    pub mime: String,
}

#[derive(Debug, Default)]
pub struct AttachmentRegistry {
    staged: HashMap<String, PathBuf>,
}

impl AttachmentRegistry {
    pub fn stage(&mut self, image: PromptImage) -> Result<StagedAttachmentV2, String> {
        let file_name = image
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "staged image has no usable file name".to_string())?
            .to_string();
        if !clipboard_image::is_managed_paste(&image.path) {
            return Err("staged images must live in the managed paste directory".to_string());
        }
        let label = image.label();
        let token = format!("att-{}-{}", Uuid::new_v4(), file_name);
        self.staged.insert(token.clone(), image.path);
        Ok(StagedAttachmentV2 {
            token,
            label,
            mime: image.mime,
        })
    }

    /// Resolve a token to its managed file. A missing file is reported (the
    /// registry entry stays; the whole directory is wiped on discard).
    pub fn resolve(&self, token: &str) -> Result<PathBuf, String> {
        let path = self
            .staged
            .get(token)
            .ok_or_else(|| format!("unknown attachment: {token}"))?;
        if !clipboard_image::is_managed_paste(path) || !path.is_file() {
            return Err(format!("attachment is no longer available: {token}"));
        }
        Ok(path.clone())
    }

    /// Drop a staged attachment and delete its managed file.
    pub fn remove(&mut self, token: &str) -> Result<(), String> {
        let path = self
            .staged
            .get(token)
            .ok_or_else(|| format!("unknown attachment: {token}"))?;
        clipboard_image::remove_managed_paste(path);
        self.staged.remove(token);
        Ok(())
    }

    pub fn retain_only(&mut self, tokens: &[String]) {
        let keep = tokens
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        self.staged.retain(|token, _| keep.contains(token));
    }

    /// Wipe the whole session's managed paste directory (workspace switch,
    /// new session, or shutdown), matching the TUI's cleanup rules.
    pub fn discard_all(&mut self) {
        clipboard_image::clear_session_pastes();
        self.staged.clear();
    }
}

/// Decode standard base64 (with optional padding) for image bytes sent from
/// the WebView. The desktop crate deliberately keeps no extra dependencies.
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=')
        .collect();
    if cleaned.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3 + 3);
    for chunk in cleaned.chunks(4) {
        let mut group: u32 = 0;
        for (index, byte) in chunk.iter().enumerate() {
            group |= u32::from(value(*byte)?) << (18 - 6 * index);
        }
        // The group holds 24 bits in the low three bytes of a u32: the first
        // output byte always lives at bytes[1].
        let bytes = group.to_be_bytes();
        match chunk.len() {
            4 => out.extend_from_slice(&bytes[1..4]),
            3 => out.extend_from_slice(&bytes[1..3]),
            2 => out.push(bytes[1]),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use asterline::adapter::prompt_images::PromptImage;

    #[test]
    fn tokens_never_expose_paths() {
        let mut registry = AttachmentRegistry::default();
        let staged = registry
            .stage(fake_managed_image())
            .expect("managed paste rejected");
        assert!(staged.token.starts_with("att-"));
        assert!(
            !staged
                .token
                .contains(std::path::MAIN_SEPARATOR.to_string().as_str())
        );
        assert_eq!(staged.mime, "image/png");
    }

    /// A real file inside this test process's managed paste directory.
    fn fake_managed_image() -> PromptImage {
        use std::io::Write;
        let dir = clipboard_image::paste_dir();
        std::fs::create_dir_all(&dir).expect("create paste dir");
        let path = dir.join(format!("fake-{}.png", Uuid::new_v4()));
        let mut file = std::fs::File::create(&path).expect("create fake paste file");
        file.write_all(b"not really a png")
            .expect("write fake paste file");
        PromptImage {
            path,
            mime: "image/png".to_string(),
        }
    }

    #[test]
    fn unmanaged_paths_are_rejected() {
        let mut registry = AttachmentRegistry::default();
        let outside = PromptImage {
            path: std::env::temp_dir().join("definitely-not-managed.png"),
            mime: "image/png".to_string(),
        };
        assert!(registry.stage(outside).is_err());
    }

    #[test]
    fn unknown_tokens_are_rejected_before_runtime_traffic() {
        let registry = AttachmentRegistry::default();
        let error = registry.resolve("att-nope.png").unwrap_err();
        assert!(error.contains("unknown attachment"));
    }

    #[test]
    fn oversize_limit_matches_the_shared_budget() {
        assert_eq!(
            asterline::adapter::prompt_images::MAX_IMAGE_BYTES,
            10 * 1024 * 1024
        );
    }

    #[test]
    fn base64_decode_round_trips_shared_encoding() {
        let encoded = asterline::adapter::prompt_images::encode_base64(b"png-bytes-123");
        assert_eq!(
            decode_base64(&encoded).as_deref(),
            Some(b"png-bytes-123".as_slice())
        );
        assert_eq!(
            decode_base64(&format!("{encoded}\n")),
            decode_base64(&encoded)
        );
        assert!(decode_base64("!!!!").is_none());
        assert!(decode_base64("A").is_none());
        assert_eq!(decode_base64(""), Some(Vec::new()));
    }
}
