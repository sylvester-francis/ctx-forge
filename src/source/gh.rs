//! Source::Gh — a reference to a specific GitHub resource (issue, PR,
//! release, or file at ref). Fetched on demand via the GitHub REST / raw
//! content APIs; cached in the existing ContentCache with per-resource TTLs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhSource {
    pub resource: GhResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GhResource {
    Issue {
        owner: String,
        repo: String,
        number: u32,
    },
    Pull {
        owner: String,
        repo: String,
        number: u32,
    },
    Release {
        owner: String,
        repo: String,
        tag: String,
    },
    Blob {
        owner: String,
        repo: String,
        reference: String,
        path: String,
    },
}

impl GhResource {
    /// Cache TTL in seconds — mutable resources expire daily; tagged
    /// releases are immutable (only metadata corrections); SHA-pinned
    /// blobs are immutable; branch-pinned blobs expire daily.
    pub fn default_ttl_secs(&self) -> u64 {
        match self {
            Self::Issue { .. } | Self::Pull { .. } => 86_400,
            Self::Release { .. } => 7 * 86_400,
            Self::Blob { reference, .. } => {
                if is_sha_like(reference) {
                    30 * 86_400
                } else {
                    86_400
                }
            }
        }
    }

    /// URL of the resource on github.com (for rendering in section headings
    /// and as the raw `url` field).
    pub fn browser_url(&self) -> String {
        match self {
            Self::Issue {
                owner,
                repo,
                number,
            } => format!("https://github.com/{owner}/{repo}/issues/{number}"),
            Self::Pull {
                owner,
                repo,
                number,
            } => format!("https://github.com/{owner}/{repo}/pull/{number}"),
            Self::Release { owner, repo, tag } => {
                format!("https://github.com/{owner}/{repo}/releases/tag/{tag}")
            }
            Self::Blob {
                owner,
                repo,
                reference,
                path,
            } => format!("https://github.com/{owner}/{repo}/blob/{reference}/{path}"),
        }
    }

    /// Canonical URI path — the form we emit in `to_uri`. Matches GitHub URL
    /// shape so users can paste a github.com URL and prefix with `gh:`.
    pub fn canonical_path(&self) -> String {
        match self {
            Self::Issue {
                owner,
                repo,
                number,
            } => format!("/{owner}/{repo}/issues/{number}"),
            Self::Pull {
                owner,
                repo,
                number,
            } => format!("/{owner}/{repo}/pull/{number}"),
            Self::Release { owner, repo, tag } => {
                format!("/{owner}/{repo}/releases/tag/{tag}")
            }
            Self::Blob {
                owner,
                repo,
                reference,
                path,
            } => format!("/{owner}/{repo}/blob/{reference}/{path}"),
        }
    }
}

fn is_sha_like(reference: &str) -> bool {
    reference.len() == 40 && reference.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_browser_url() {
        let r = GhResource::Issue {
            owner: "tokio-rs".into(),
            repo: "axum".into(),
            number: 1234,
        };
        assert_eq!(
            r.browser_url(),
            "https://github.com/tokio-rs/axum/issues/1234",
        );
    }

    #[test]
    fn pull_canonical_path() {
        let r = GhResource::Pull {
            owner: "foo".into(),
            repo: "bar".into(),
            number: 7,
        };
        assert_eq!(r.canonical_path(), "/foo/bar/pull/7");
    }

    #[test]
    fn release_ttl_is_week() {
        let r = GhResource::Release {
            owner: "a".into(),
            repo: "b".into(),
            tag: "v1".into(),
        };
        assert_eq!(r.default_ttl_secs(), 7 * 86_400);
    }

    #[test]
    fn sha_blob_gets_long_ttl() {
        let r = GhResource::Blob {
            owner: "a".into(),
            repo: "b".into(),
            reference: "a".repeat(40),
            path: "CHANGELOG.md".into(),
        };
        assert_eq!(r.default_ttl_secs(), 30 * 86_400);
    }

    #[test]
    fn branch_blob_gets_daily_ttl() {
        let r = GhResource::Blob {
            owner: "a".into(),
            repo: "b".into(),
            reference: "main".into(),
            path: "CHANGELOG.md".into(),
        };
        assert_eq!(r.default_ttl_secs(), 86_400);
    }

    #[test]
    fn is_sha_like_matches_40_hex() {
        assert!(is_sha_like("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_sha_like("main"));
        assert!(!is_sha_like("0123456789abcdef0123456789abcdef0123456"));
    }
}
