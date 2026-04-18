//! Forge host detection and URL extraction.
//!
//! Registry APIs return `repository` URLs in inconsistent shapes (HTTPS,
//! git+ prefix, .git suffix, object wrapper). This module normalises them
//! into a `ForgeRef` that downstream rendering can consume uniformly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForgeHost {
    GitHub,
    GitLab,
    Codeberg,
    /// Known to be a repo URL, but not a forge ctxforge emits structured
    /// URLs for. The raw URL is still retained on `ForgeRef::raw_url` so
    /// the Project stack can fall back to a single-line display.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeRef {
    pub host: ForgeHost,
    /// "owner/repo" for known forges; raw URL path for `Other`.
    pub path: String,
    /// Canonical https URL of the repo root.
    pub raw_url: String,
}

impl ForgeRef {
    /// URL for the releases page. None for `Other` — caller falls back to
    /// `raw_url`.
    pub fn releases_url(&self) -> Option<String> {
        match self.host {
            ForgeHost::GitHub => {
                Some(format!("https://github.com/{}/releases", self.path))
            }
            ForgeHost::GitLab => {
                Some(format!("https://gitlab.com/{}/-/releases", self.path))
            }
            ForgeHost::Codeberg => {
                Some(format!("https://codeberg.org/{}/releases", self.path))
            }
            ForgeHost::Other => None,
        }
    }

    /// URL for open issues. None for `Other`.
    pub fn issues_url(&self) -> Option<String> {
        match self.host {
            ForgeHost::GitHub => {
                Some(format!("https://github.com/{}/issues", self.path))
            }
            ForgeHost::GitLab => {
                Some(format!("https://gitlab.com/{}/-/issues", self.path))
            }
            ForgeHost::Codeberg => {
                Some(format!("https://codeberg.org/{}/issues", self.path))
            }
            ForgeHost::Other => None,
        }
    }
}

/// Parse a registry `repository` URL into a `ForgeRef`. Handles:
/// - `git+` prefix (npm convention)
/// - `.git` suffix
/// - `github.com`, `gitlab.com`, `codeberg.org` hosts → known forges
/// - anything else with a recognizable `https://…/owner/repo` shape → `Other`
/// - malformed / non-HTTP / None → return `None`
pub fn parse_forge_url(input: &str) -> Option<ForgeRef> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip "git+" prefix used by npm.
    let stripped = trimmed.strip_prefix("git+").unwrap_or(trimmed);

    // Only HTTPS or HTTP (strip ssh git@ — can't render those as browser URLs).
    let after_scheme = if let Some(s) = stripped.strip_prefix("https://") {
        s
    } else if let Some(s) = stripped.strip_prefix("http://") {
        s
    } else {
        return None;
    };

    // Split host from path.
    let (host, rest) = after_scheme.split_once('/')?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");

    let forge = match host {
        "github.com" | "www.github.com" => ForgeHost::GitHub,
        "gitlab.com" | "www.gitlab.com" => ForgeHost::GitLab,
        "codeberg.org" | "www.codeberg.org" => ForgeHost::Codeberg,
        _ => ForgeHost::Other,
    };

    let raw_url = format!("https://{host}/{rest}");

    // For known forges, extract owner/repo (first two path segments).
    let path = match forge {
        ForgeHost::GitHub | ForgeHost::GitLab | ForgeHost::Codeberg => {
            let mut segs = rest.splitn(3, '/');
            let owner = segs.next()?;
            let repo = segs.next()?;
            if owner.is_empty() || repo.is_empty() {
                return None;
            }
            format!("{owner}/{repo}")
        }
        ForgeHost::Other => rest.to_string(),
    };

    Some(ForgeRef {
        host: forge,
        path,
        raw_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_plain_url() {
        let f = parse_forge_url("https://github.com/tokio-rs/axum").unwrap();
        assert_eq!(f.host, ForgeHost::GitHub);
        assert_eq!(f.path, "tokio-rs/axum");
        assert_eq!(f.raw_url, "https://github.com/tokio-rs/axum");
    }

    #[test]
    fn strips_git_plus_prefix_and_dot_git_suffix() {
        let f = parse_forge_url("git+https://github.com/vercel/next.js.git").unwrap();
        assert_eq!(f.path, "vercel/next.js");
    }

    #[test]
    fn handles_trailing_slash() {
        let f = parse_forge_url("https://github.com/tokio-rs/axum/").unwrap();
        assert_eq!(f.path, "tokio-rs/axum");
    }

    #[test]
    fn detects_gitlab() {
        let f = parse_forge_url("https://gitlab.com/gitlab-org/gitlab").unwrap();
        assert_eq!(f.host, ForgeHost::GitLab);
    }

    #[test]
    fn detects_codeberg() {
        let f = parse_forge_url("https://codeberg.org/forgejo/forgejo").unwrap();
        assert_eq!(f.host, ForgeHost::Codeberg);
    }

    #[test]
    fn unknown_host_becomes_other() {
        let f = parse_forge_url("https://git.sr.ht/~sircmpwn/aerc").unwrap();
        assert_eq!(f.host, ForgeHost::Other);
    }

    #[test]
    fn rejects_ssh_url() {
        assert!(parse_forge_url("git@github.com:tokio-rs/axum.git").is_none());
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_forge_url("").is_none());
        assert!(parse_forge_url("   ").is_none());
    }

    #[test]
    fn rejects_non_http() {
        assert!(parse_forge_url("ftp://example.com/foo").is_none());
    }

    #[test]
    fn github_releases_url() {
        let f = parse_forge_url("https://github.com/tokio-rs/axum").unwrap();
        assert_eq!(
            f.releases_url().unwrap(),
            "https://github.com/tokio-rs/axum/releases",
        );
    }

    #[test]
    fn gitlab_uses_dash_in_path() {
        let f = parse_forge_url("https://gitlab.com/gitlab-org/gitlab").unwrap();
        assert_eq!(
            f.issues_url().unwrap(),
            "https://gitlab.com/gitlab-org/gitlab/-/issues",
        );
    }

    #[test]
    fn other_has_no_forge_urls() {
        let f = parse_forge_url("https://example.com/foo/bar").unwrap();
        assert!(f.releases_url().is_none());
        assert!(f.issues_url().is_none());
    }
}
