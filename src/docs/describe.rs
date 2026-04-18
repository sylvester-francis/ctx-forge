//! One-line description + forge URL fetch from ecosystem registry APIs,
//! cached via P5.
//!
//! Description cache key is sha256 of a synthetic URI
//! `description://<ecosystem>/<name>` — never surfaced to users; it just
//! keys the existing ContentCache for dedup across runs.

use crate::cache::{CacheRead, ContentCache};
use crate::gh::forge::{self, ForgeRef};
use crate::source::Ecosystem;
use serde_json::Value;

const DESC_MAX_CHARS: usize = 120;
const DESC_TTL_SECS: u64 = 7 * 86_400;

/// Metadata extracted from a single registry-API response — used by
/// `run_detect` to populate both `description` and `forge` on DocsSource
/// items without a second HTTP round-trip.
#[derive(Debug, Default, Clone)]
pub struct Metadata {
    pub description: Option<String>,
    pub forge: Option<ForgeRef>,
}

pub fn fetch_metadata(cache: &ContentCache, ecosystem: Ecosystem, name: &str) -> Metadata {
    let Some(url) = super::resolve::description_url(ecosystem, name) else {
        return Metadata::default();
    };
    let key = cache_key(ecosystem, name);

    if let Ok(CacheRead::Fresh { body, .. }) = cache.get(&key) {
        return extract(ecosystem, &body);
    }

    #[cfg(feature = "fetch")]
    let fetched = {
        let cfg = crate::fetch::FetchConfig::default();
        crate::fetch::fetch(&url, &cfg).ok()
    };
    #[cfg(not(feature = "fetch"))]
    let fetched: Option<()> = None;

    #[cfg(feature = "fetch")]
    {
        let Some(fetched) = fetched else {
            return Metadata::default();
        };
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
        Metadata::default()
    }
}

/// Back-compat shim — existing call sites that only need the description.
pub fn fetch_description(cache: &ContentCache, ecosystem: Ecosystem, name: &str) -> Option<String> {
    fetch_metadata(cache, ecosystem, name).description
}

fn cache_key(ecosystem: Ecosystem, name: &str) -> String {
    crate::cache::sha256_hex(format!("description://{}/{}", ecosystem.as_str(), name).as_bytes())
}

fn extract(ecosystem: Ecosystem, body: &[u8]) -> Metadata {
    let Ok(v) = serde_json::from_slice::<Value>(body) else {
        return Metadata::default();
    };
    match ecosystem {
        Ecosystem::Rust => extract_rust(&v),
        Ecosystem::Js => extract_js(&v),
        Ecosystem::Python => extract_python(&v),
        Ecosystem::Go => Metadata::default(),
    }
}

fn extract_rust(v: &Value) -> Metadata {
    let crate_obj = v.get("crate");
    let description = crate_obj
        .and_then(|c| c.get("description"))
        .and_then(|d| d.as_str())
        .map(truncate);
    let forge = crate_obj
        .and_then(|c| c.get("repository"))
        .and_then(|r| r.as_str())
        .and_then(forge::parse_forge_url);
    Metadata { description, forge }
}

fn extract_js(v: &Value) -> Metadata {
    let description = v.get("description").and_then(|d| d.as_str()).map(truncate);
    // npm `repository` is either a string or { "url": "..." }.
    let repo_url = v.get("repository").and_then(|r| {
        if let Some(s) = r.as_str() {
            Some(s.to_string())
        } else {
            r.get("url").and_then(|u| u.as_str()).map(String::from)
        }
    });
    let forge = repo_url.as_deref().and_then(forge::parse_forge_url);
    Metadata { description, forge }
}

fn extract_python(v: &Value) -> Metadata {
    let info = v.get("info");
    let description = info
        .and_then(|i| i.get("summary"))
        .and_then(|s| s.as_str())
        .map(truncate);

    // info.project_urls.Source preferred; fall back to info.home_page.
    let repo_url = info
        .and_then(|i| i.get("project_urls"))
        .and_then(|p| p.get("Source").or_else(|| p.get("Repository")))
        .and_then(|u| u.as_str())
        .or_else(|| {
            info.and_then(|i| i.get("home_page"))
                .and_then(|h| h.as_str())
        })
        .map(String::from);
    let forge = repo_url.as_deref().and_then(forge::parse_forge_url);
    Metadata { description, forge }
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
    use crate::gh::forge::ForgeHost;

    #[test]
    fn extract_rust_with_repository() {
        let body = br#"{"crate":{"description":"Web framework.","repository":"https://github.com/tokio-rs/axum"}}"#;
        let md = extract(Ecosystem::Rust, body);
        assert_eq!(md.description.as_deref(), Some("Web framework"));
        assert_eq!(md.forge.as_ref().map(|f| f.host), Some(ForgeHost::GitHub));
        assert_eq!(
            md.forge.as_ref().map(|f| f.path.as_str()),
            Some("tokio-rs/axum"),
        );
    }

    #[test]
    fn extract_rust_without_repository() {
        let body = br#"{"crate":{"description":"Foo"}}"#;
        let md = extract(Ecosystem::Rust, body);
        assert_eq!(md.description.as_deref(), Some("Foo"));
        assert!(md.forge.is_none());
    }

    #[test]
    fn extract_js_object_form_repository() {
        let body = br#"{"description":"React framework.","repository":{"url":"git+https://github.com/vercel/next.js.git"}}"#;
        let md = extract(Ecosystem::Js, body);
        assert_eq!(
            md.forge.as_ref().map(|f| f.path.as_str()),
            Some("vercel/next.js"),
        );
    }

    #[test]
    fn extract_js_string_form_repository() {
        let body = br#"{"description":"Lib.","repository":"https://github.com/foo/bar"}"#;
        let md = extract(Ecosystem::Js, body);
        assert_eq!(md.forge.as_ref().map(|f| f.path.as_str()), Some("foo/bar"),);
    }

    #[test]
    fn extract_python_project_urls_source() {
        let body = br#"{"info":{"summary":"Modern framework.","project_urls":{"Source":"https://github.com/tiangolo/fastapi"}}}"#;
        let md = extract(Ecosystem::Python, body);
        assert_eq!(
            md.forge.as_ref().map(|f| f.path.as_str()),
            Some("tiangolo/fastapi"),
        );
    }

    #[test]
    fn extract_python_home_page_fallback() {
        let body = br#"{"info":{"summary":"Thing.","home_page":"https://github.com/foo/bar"}}"#;
        let md = extract(Ecosystem::Python, body);
        assert_eq!(md.forge.as_ref().map(|f| f.path.as_str()), Some("foo/bar"),);
    }

    #[test]
    fn extract_go_is_always_empty() {
        let body = br#"{"anything":"here"}"#;
        let md = extract(Ecosystem::Go, body);
        assert!(md.description.is_none());
        assert!(md.forge.is_none());
    }

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
}
