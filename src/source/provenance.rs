//! Provenance — the audit trail attached to every resolved bundle item.
//!
//! Local sources get `uri + sha256(content)`. Network sources additionally
//! carry `fetched_at`, `etag`, and a `stale` bit set when we served a
//! past-TTL cache entry because the refetch failed.

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub uri: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Provenance {
    pub fn local(uri: String, sha256: String) -> Self {
        Self {
            uri,
            sha256,
            fetched_at: None,
            etag: None,
            stale: false,
            failed: false,
            reason: None,
        }
    }

    pub fn network(
        uri: String,
        sha256: String,
        fetched_at: DateTime<Utc>,
        etag: Option<String>,
        stale: bool,
    ) -> Self {
        Self {
            uri,
            sha256,
            fetched_at: Some(fetched_at),
            etag,
            stale,
            failed: false,
            reason: None,
        }
    }

    pub fn failed(uri: String, reason: String) -> Self {
        Self {
            uri,
            sha256: String::new(),
            fetched_at: Some(Utc::now()),
            etag: None,
            stale: false,
            failed: true,
            reason: Some(reason),
        }
    }

    pub fn fetched_at_str(&self) -> Option<String> {
        self.fetched_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
    }
}
