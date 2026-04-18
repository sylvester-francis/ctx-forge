//! GitHub REST + raw content fetcher. Reads `GITHUB_TOKEN` once per
//! process and reuses the existing fetch pipeline + cache.

#![cfg(feature = "fetch")]

use crate::error::{CtxforgeError, Result};
use crate::fetch::FetchConfig;
use crate::source::GhResource;
use serde_json::Value;
use std::sync::OnceLock;

static AUTH_TOKEN: OnceLock<Option<String>> = OnceLock::new();

fn auth_token() -> Option<String> {
    AUTH_TOKEN
        .get_or_init(|| std::env::var("GITHUB_TOKEN").ok())
        .clone()
}

/// Response from fetching a GhResource — structured so the renderer
/// can display title + metadata + body without re-parsing.
#[derive(Debug, Clone)]
pub struct GhBody {
    pub title: String,
    pub state: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub body: String,
}

/// Fetch a GhResource. Returns `GhBody` on success.
/// Issue / PR / Release use the GitHub REST API; Blob uses raw.githubusercontent.com.
pub fn fetch_resource(resource: &GhResource) -> Result<GhBody> {
    let cfg = FetchConfig {
        auth_token: auth_token(),
        max_bytes: 1_024 * 1_024,
        ..Default::default()
    };

    match resource {
        GhResource::Issue {
            owner,
            repo,
            number,
        } => fetch_issue(&cfg, owner, repo, *number),
        GhResource::Pull {
            owner,
            repo,
            number,
        } => fetch_pull(&cfg, owner, repo, *number),
        GhResource::Release { owner, repo, tag } => fetch_release(&cfg, owner, repo, tag),
        GhResource::Blob {
            owner,
            repo,
            reference,
            path,
        } => fetch_blob(&cfg, owner, repo, reference, path),
    }
}

fn fetch_issue(cfg: &FetchConfig, owner: &str, repo: &str, number: u32) -> Result<GhBody> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{number}");
    let json = github_api_get(cfg, &url)?;
    Ok(GhBody {
        title: json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(untitled)")
            .to_string(),
        state: json
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        author: json
            .get("user")
            .and_then(|u| u.get("login"))
            .and_then(|l| l.as_str())
            .map(String::from),
        published_at: json
            .get("created_at")
            .and_then(|d| d.as_str())
            .map(String::from),
        body: json
            .get("body")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn fetch_pull(cfg: &FetchConfig, owner: &str, repo: &str, number: u32) -> Result<GhBody> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{number}");
    let json = github_api_get(cfg, &url)?;
    let state = match (
        json.get("merged").and_then(|m| m.as_bool()),
        json.get("state").and_then(|s| s.as_str()),
    ) {
        (Some(true), _) => "merged".to_string(),
        (_, Some(s)) => s.to_string(),
        _ => "unknown".to_string(),
    };
    Ok(GhBody {
        title: json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(untitled)")
            .to_string(),
        state,
        author: json
            .get("user")
            .and_then(|u| u.get("login"))
            .and_then(|l| l.as_str())
            .map(String::from),
        published_at: json
            .get("created_at")
            .and_then(|d| d.as_str())
            .map(String::from),
        body: json
            .get("body")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn fetch_release(cfg: &FetchConfig, owner: &str, repo: &str, tag: &str) -> Result<GhBody> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
    let json = github_api_get(cfg, &url)?;
    Ok(GhBody {
        title: json
            .get("name")
            .and_then(|n| n.as_str())
            .or_else(|| json.get("tag_name").and_then(|t| t.as_str()))
            .unwrap_or(tag)
            .to_string(),
        state: if json.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
            "draft".into()
        } else if json
            .get("prerelease")
            .and_then(|p| p.as_bool())
            .unwrap_or(false)
        {
            "prerelease".into()
        } else {
            "released".into()
        },
        author: json
            .get("author")
            .and_then(|u| u.get("login"))
            .and_then(|l| l.as_str())
            .map(String::from),
        published_at: json
            .get("published_at")
            .and_then(|d| d.as_str())
            .map(String::from),
        body: json
            .get("body")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn fetch_blob(
    cfg: &FetchConfig,
    owner: &str,
    repo: &str,
    reference: &str,
    path: &str,
) -> Result<GhBody> {
    let url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{reference}/{path}");
    let result = crate::fetch::fetch(&url, cfg).map_err(CtxforgeError::Fetch)?;
    let body = String::from_utf8_lossy(&result.body).into_owned();
    Ok(GhBody {
        title: path.to_string(),
        state: format!("at ref {reference}"),
        author: None,
        published_at: None,
        body,
    })
}

fn github_api_get(cfg: &FetchConfig, url: &str) -> Result<Value> {
    let result = crate::fetch::fetch(url, cfg).map_err(|e| map_github_error(&e, cfg))?;
    serde_json::from_slice(&result.body)
        .map_err(|e| CtxforgeError::Fetch(format!("GitHub API response was not valid JSON: {e}")))
}

/// Convert a raw fetch error string into a more actionable message for
/// common GitHub failure modes (rate limit, bad token).
fn map_github_error(e: &str, cfg: &FetchConfig) -> CtxforgeError {
    if e.contains("HTTP 403") {
        if cfg.auth_token.is_none() {
            return CtxforgeError::Fetch(
                "GitHub anonymous rate limit hit (60/hr); set GITHUB_TOKEN for 5000/hr".to_string(),
            );
        }
        return CtxforgeError::Fetch(format!("GitHub access denied: {e}"));
    }
    if e.contains("HTTP 401") {
        return CtxforgeError::Fetch("GITHUB_TOKEN rejected — regenerate or unset".to_string());
    }
    CtxforgeError::Fetch(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_403_without_token_to_rate_limit_message() {
        let cfg = FetchConfig::default();
        let err = map_github_error("HTTP 403", &cfg);
        assert!(err.to_string().contains("GITHUB_TOKEN"));
        assert!(err.to_string().contains("60/hr"));
    }

    #[test]
    fn maps_403_with_token_to_access_denied() {
        let cfg = FetchConfig {
            auth_token: Some("x".into()),
            ..Default::default()
        };
        let err = map_github_error("HTTP 403", &cfg);
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn maps_401_to_bad_token() {
        let cfg = FetchConfig::default();
        let err = map_github_error("HTTP 401", &cfg);
        assert!(err.to_string().contains("rejected"));
    }
}
