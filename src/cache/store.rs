//! On-disk content-addressable cache store.
//!
//! ```text
//! $XDG_CACHE_HOME/ctxforge/
//! ├── cache.key                       (0600, 32 random bytes)
//! └── objects/<hex2>/<sha256(uri)>/
//!     ├── meta.json                   { uri, scheme, fetched_at, ttl_secs, etag,
//!     │                                 content_type, body_size, body_sha256 }
//!     ├── meta.hmac                   hex-encoded HMAC-SHA256 of meta.json bytes
//!     └── body                        raw bytes
//! ```
//! HMAC is stored in a sidecar so the signed bytes are the literal
//! `meta.json` contents — no field-zeroing gymnastics during verify.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

type HmacSha256 = Hmac<Sha256>;

/// Metadata for a cached object. Serialised *exactly* as stored on disk;
/// the sidecar `meta.hmac` is computed over the bytes of that serialisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub uri: String,
    pub scheme: String,
    pub fetched_at: DateTime<Utc>,
    pub ttl_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub body_size: u64,
    pub body_sha256: String,
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

pub fn cache_key_for_uri(uri: &str) -> String {
    sha256_hex(uri.as_bytes())
}

pub fn object_dir(cache_root: &Path, key: &str) -> PathBuf {
    let shard = &key[..2.min(key.len())];
    cache_root.join("objects").join(shard).join(key)
}

pub fn compute_hmac(key: &[u8; 32], data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key is always 32 bytes");
    mac.update(data);
    format!("{:x}", mac.finalize().into_bytes())
}

pub fn verify_hmac(key: &[u8; 32], data: &[u8], expected_hex: &str) -> bool {
    let Ok(bytes) = hex_decode(expected_hex) else {
        return false;
    };
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key is always 32 bytes");
    mac.update(data);
    mac.verify_slice(&bytes).is_ok()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha_deterministic() {
        assert_eq!(sha256_hex(b"hello"), sha256_hex(b"hello"));
        assert_ne!(sha256_hex(b"hello"), sha256_hex(b"world"));
        assert_eq!(sha256_hex(b"").len(), 64);
    }

    #[test]
    fn object_dir_shards_by_first_two_chars() {
        assert_eq!(
            object_dir(Path::new("/c"), "abcdef1234"),
            PathBuf::from("/c/objects/ab/abcdef1234"),
        );
    }

    #[test]
    fn hmac_roundtrip() {
        let key = [42u8; 32];
        let m = compute_hmac(&key, b"payload");
        assert!(verify_hmac(&key, b"payload", &m));
        assert!(!verify_hmac(&key, b"tampered", &m));
        assert!(!verify_hmac(&key, b"payload", "not-hex"));
    }
}
