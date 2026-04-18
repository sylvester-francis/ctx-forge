//! gh:// URI path → GhResource dispatcher.

use crate::source::{GhResource, UriParseError};

/// Parse a `gh://` URI path (e.g. "/tokio-rs/axum/issues/1234") into a
/// `GhResource`. Returns a `UriParseError::BadFragment` with a specific
/// message on malformed input.
pub fn to_resource(path: &str) -> Result<GhResource, UriParseError> {
    let trimmed = path.trim_start_matches('/');
    let mut parts = trimmed.split('/');

    let owner = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(UriParseError::BadFragment("gh:// missing owner"))?
        .to_string();
    let repo = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(UriParseError::BadFragment("gh:// missing repo"))?
        .to_string();
    let kind = parts
        .next()
        .ok_or(UriParseError::BadFragment("gh:// missing resource kind"))?;

    match kind {
        "issues" => {
            let n = parts
                .next()
                .ok_or(UriParseError::BadFragment("gh:// issues requires a number"))?;
            let number: u32 = n
                .parse()
                .map_err(|_| UriParseError::BadFragment("gh:// issues number not numeric"))?;
            if parts.next().is_some() {
                return Err(UriParseError::BadFragment(
                    "gh:// issues/N has trailing path segments",
                ));
            }
            Ok(GhResource::Issue {
                owner,
                repo,
                number,
            })
        }
        "pull" => {
            let n = parts
                .next()
                .ok_or(UriParseError::BadFragment("gh:// pull requires a number"))?;
            let number: u32 = n
                .parse()
                .map_err(|_| UriParseError::BadFragment("gh:// pull number not numeric"))?;
            if parts.next().is_some() {
                return Err(UriParseError::BadFragment(
                    "gh:// pull/N has trailing path segments",
                ));
            }
            Ok(GhResource::Pull {
                owner,
                repo,
                number,
            })
        }
        "releases" => {
            let tag_kw = parts
                .next()
                .ok_or(UriParseError::BadFragment("gh:// releases requires /tag/X"))?;
            if tag_kw != "tag" {
                return Err(UriParseError::BadFragment(
                    "gh:// releases path must be releases/tag/<tag>",
                ));
            }
            let tag = parts
                .next()
                .ok_or(UriParseError::BadFragment("gh:// releases/tag missing tag"))?
                .to_string();
            if parts.next().is_some() {
                return Err(UriParseError::BadFragment(
                    "gh:// releases/tag/X has trailing path segments",
                ));
            }
            Ok(GhResource::Release { owner, repo, tag })
        }
        "blob" => {
            let reference = parts
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(UriParseError::BadFragment("gh:// blob requires a ref"))?
                .to_string();
            let rest: Vec<&str> = parts.collect();
            if rest.is_empty() {
                return Err(UriParseError::BadFragment("gh:// blob requires a file path"));
            }
            let path = rest.join("/");
            Ok(GhResource::Blob {
                owner,
                repo,
                reference,
                path,
            })
        }
        _ => Err(UriParseError::BadFragment(
            "gh:// unknown resource kind (expected issues/pull/releases/blob)",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_issue() {
        let r = to_resource("/tokio-rs/axum/issues/1234").unwrap();
        match r {
            GhResource::Issue {
                owner,
                repo,
                number,
            } => {
                assert_eq!(owner, "tokio-rs");
                assert_eq!(repo, "axum");
                assert_eq!(number, 1234);
            }
            _ => panic!("expected Issue"),
        }
    }

    #[test]
    fn parses_pull() {
        let r = to_resource("/foo/bar/pull/7").unwrap();
        assert!(matches!(r, GhResource::Pull { number: 7, .. }));
    }

    #[test]
    fn parses_release_tag() {
        let r = to_resource("/foo/bar/releases/tag/v1.0.0").unwrap();
        match r {
            GhResource::Release { tag, .. } => assert_eq!(tag, "v1.0.0"),
            _ => panic!("expected Release"),
        }
    }

    #[test]
    fn parses_blob_with_nested_path() {
        let r = to_resource("/foo/bar/blob/main/docs/CHANGELOG.md").unwrap();
        match r {
            GhResource::Blob {
                reference, path, ..
            } => {
                assert_eq!(reference, "main");
                assert_eq!(path, "docs/CHANGELOG.md");
            }
            _ => panic!("expected Blob"),
        }
    }

    #[test]
    fn rejects_unknown_kind() {
        assert!(to_resource("/foo/bar/wiki/page").is_err());
    }

    #[test]
    fn rejects_issues_with_non_numeric() {
        assert!(to_resource("/foo/bar/issues/abc").is_err());
    }

    #[test]
    fn rejects_missing_owner() {
        assert!(to_resource("/").is_err());
    }

    #[test]
    fn rejects_releases_without_tag_keyword() {
        assert!(to_resource("/foo/bar/releases/v1.0.0").is_err());
    }
}
