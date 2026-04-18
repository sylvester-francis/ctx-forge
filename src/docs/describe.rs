//! One-line description fetch from ecosystem registry APIs, cached via P5.
//!
//! Description cache key is sha256 of a synthetic URI
//! `description://<ecosystem>/<name>` — never surfaced to users; it just
//! keys the existing ContentCache for dedup across runs.

use crate::cache::{CacheRead, ContentCache};
use crate::source::Ecosystem;
use serde_json::Value;

const DESC_MAX_CHARS: usize = 120;
const DESC_TTL_SECS: u64 = 7 * 86_400;

/// Fetch and cache a one-line description. Returns None on any failure
/// (description is optional everywhere; we never fail the overall detect
/// pipeline because a registry API is flaky).
pub fn fetch_description(cache: &ContentCache, ecosystem: Ecosystem, name: &str) -> Option<String> {
    let url = super::resolve::description_url(ecosystem, name)?;
    let key = cache_key(ecosystem, name);

    // Hit the cache first; re-extract from the cached body.
    if let Ok(CacheRead::Fresh { body, .. }) = cache.get(&key) {
        return extract(ecosystem, &body);
    }

    // Miss or stale — try to fetch.
    #[cfg(feature = "fetch")]
    let fetched = {
        let cfg = crate::fetch::FetchConfig::default();
        crate::fetch::fetch(&url, &cfg).ok()
    };
    #[cfg(not(feature = "fetch"))]
    let fetched: Option<()> = None;

    #[cfg(feature = "fetch")]
    {
        let fetched = fetched?;
        // Persist for next time.
        let _ = cache.put(
            &key,
            &format!("description://{}/{}", ecosystem.as_str(), name),
            "description",
            &fetched.body,
            DESC_TTL_SECS,
            fetched.etag,
            fetched.content_type,
        );
        extract(ecosystem, &fetched.body)
    }
    #[cfg(not(feature = "fetch"))]
    {
        let _ = url;
        None
    }
}

fn cache_key(ecosystem: Ecosystem, name: &str) -> String {
    crate::cache::sha256_hex(format!("description://{}/{}", ecosystem.as_str(), name).as_bytes())
}

fn extract(ecosystem: Ecosystem, body: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let raw = match ecosystem {
        // crates.io — { "crate": { "description": "..." } }
        Ecosystem::Rust => v.get("crate")?.get("description")?.as_str()?,
        // npm registry — top-level { "description": "..." }
        Ecosystem::Js => v.get("description")?.as_str()?,
        // PyPI — { "info": { "summary": "..." } }
        Ecosystem::Python => v.get("info")?.get("summary")?.as_str()?,
        Ecosystem::Go => return None,
    };
    Some(truncate(raw))
}

fn truncate(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= DESC_MAX_CHARS {
        return trimmed.trim_end_matches('.').to_string();
    }
    let truncated: String = trimmed.chars().take(DESC_MAX_CHARS - 1).collect();
    format!("{}…", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_is_unchanged() {
        assert_eq!(truncate("A web framework"), "A web framework");
    }

    #[test]
    fn truncate_strips_trailing_period() {
        assert_eq!(truncate("A web framework."), "A web framework");
    }

    #[test]
    fn truncate_long_appends_ellipsis() {
        let long = "x".repeat(200);
        let t = truncate(&long);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() <= DESC_MAX_CHARS);
    }

    #[test]
    fn extract_crates_io_shape() {
        let body = br#"{"crate":{"description":"Ergonomic web framework."}}"#;
        assert_eq!(
            extract(Ecosystem::Rust, body),
            Some("Ergonomic web framework".to_string()),
        );
    }

    #[test]
    fn extract_npm_shape() {
        let body = br#"{"description":"React framework for production."}"#;
        assert_eq!(
            extract(Ecosystem::Js, body),
            Some("React framework for production".to_string()),
        );
    }

    #[test]
    fn extract_pypi_shape() {
        let body = br#"{"info":{"summary":"Modern web framework for APIs."}}"#;
        assert_eq!(
            extract(Ecosystem::Python, body),
            Some("Modern web framework for APIs".to_string()),
        );
    }

    #[test]
    fn extract_go_returns_none() {
        let body = br#"{"anything":"here"}"#;
        assert!(extract(Ecosystem::Go, body).is_none());
    }
}
