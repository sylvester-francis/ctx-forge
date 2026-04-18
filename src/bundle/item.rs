//! Bundle items — v2 schema.
//!
//! An `Item` wraps a `Source` and an optional human-readable label.
//! Content is resolved lazily at export time by `resolve.rs`.

#![allow(dead_code)]

use crate::error::{CtxforgeError, Result};
use crate::source::{FileSource, RangeSource, Source};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Keep `Range` as the CLI-side parser helper — still used for `foo.rs:10-20`
/// input. Distinct from `source::RangeSource`, which is the persisted form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl Range {
    pub fn parse(input: &str) -> Result<Self> {
        let (s, e) = input
            .split_once('-')
            .ok_or_else(|| CtxforgeError::InvalidRange {
                input: input.into(),
                reason: "expected `start-end`".into(),
            })?;
        let start: usize = s.parse().map_err(|_| CtxforgeError::InvalidRange {
            input: input.into(),
            reason: format!("start `{s}` is not a number"),
        })?;
        let end: usize = e.parse().map_err(|_| CtxforgeError::InvalidRange {
            input: input.into(),
            reason: format!("end `{e}` is not a number"),
        })?;
        if start == 0 || end == 0 {
            return Err(CtxforgeError::InvalidRange {
                input: input.into(),
                reason: "lines are 1-indexed; 0 is invalid".into(),
            });
        }
        if start > end {
            return Err(CtxforgeError::InvalidRange {
                input: input.into(),
                reason: format!("start {start} > end {end}"),
            });
        }
        Ok(Range { start, end })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Item {
    /// Parse a user-supplied argument:
    /// - `https://…` / `http://…` → `Source::Url`
    /// - `url:…` / `file://…` / `range://…` / `func:…` / `type:…` → canonicalise then `Source::from_uri`
    /// - `path:start-end` → `Source::Range`
    /// - bare path → `Source::File`
    pub fn parse_add_argument(input: &str) -> Result<Self> {
        if input.starts_with("https://") || input.starts_with("http://") {
            // Canonicalise pasted GitHub URLs to gh:// form so they route to
            // the GitHub fetcher instead of the generic URL fetcher.
            if let Some(canonical) = canonicalise_github_url(input) {
                let uri: crate::source::Uri = canonical.parse()?;
                let source = Source::from_uri(&uri)?;
                return Ok(Item {
                    source,
                    label: None,
                });
            }
            return Ok(Item {
                source: Source::Url(crate::source::UrlSource { url: input.into() }),
                label: None,
            });
        }
        if input.starts_with("url:")
            || input.starts_with("file://")
            || input.starts_with("range://")
            || input.starts_with("func:")
            || input.starts_with("type:")
            || input.starts_with("url://")
            || input.starts_with("gh://")
        {
            let canonical = canonicalize_uri_prefix(input);
            let uri: crate::source::Uri = canonical.parse()?;
            let source = Source::from_uri(&uri)?;
            return Ok(Item {
                source,
                label: None,
            });
        }
        if let Some((path, range_str)) = input.rsplit_once(':') {
            if range_str.contains('-') && !range_str.contains('/') && !range_str.contains('\\') {
                let r = Range::parse(range_str)?;
                let src = RangeSource::new(PathBuf::from(path), r.start, r.end)?;
                return Ok(Item {
                    source: Source::Range(src),
                    label: None,
                });
            }
        }
        Ok(Item {
            source: Source::File(FileSource {
                path: PathBuf::from(input),
            }),
            label: None,
        })
    }

    pub fn display(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| self.source.display_label())
    }
}

/// Convert `https://github.com/owner/repo/<resource>/...` to the `gh://`
/// form. Returns `None` if the URL is not a github.com URL or doesn't
/// match a known `gh://` resource shape.
fn canonicalise_github_url(url: &str) -> Option<String> {
    let stripped = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    let stripped = stripped.trim_end_matches('/');
    let parts: Vec<&str> = stripped.splitn(4, '/').collect();
    if parts.len() < 3 {
        return None;
    }
    let kind = parts[2];
    if !matches!(kind, "issues" | "pull" | "releases" | "blob") {
        return None;
    }
    Some(format!("gh:///{stripped}"))
}

fn canonicalize_uri_prefix(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("url:") {
        if rest.starts_with("//") {
            return input.to_string();
        }
        if let Some(tail) = rest.strip_prefix("https://") {
            return format!("url://{tail}");
        }
        if let Some(tail) = rest.strip_prefix("http://") {
            return format!("url://{tail}");
        }
        return format!("url://{rest}");
    }
    for prefix in ["func:", "type:"] {
        if let Some(rest) = input.strip_prefix(prefix) {
            if rest.starts_with("//") {
                return input.to_string();
            }
            let sep = if rest.starts_with('/') { "" } else { "/" };
            return format!("{prefix}//{sep}{rest}");
        }
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_parses_valid() {
        assert_eq!(Range::parse("1-5").unwrap(), Range { start: 1, end: 5 });
        assert_eq!(
            Range::parse("45-120").unwrap(),
            Range {
                start: 45,
                end: 120,
            }
        );
    }

    #[test]
    fn range_rejects_zero_and_reversed() {
        assert!(Range::parse("0-5").is_err());
        assert!(Range::parse("10-5").is_err());
    }

    #[test]
    fn parses_plain_path() {
        let i = Item::parse_add_argument("src/main.rs").unwrap();
        assert!(
            matches!(i.source, Source::File(ref f) if f.path == std::path::Path::new("src/main.rs"))
        );
    }

    #[test]
    fn parses_ranged_path() {
        let i = Item::parse_add_argument("src/main.rs:10-50").unwrap();
        assert!(matches!(i.source, Source::Range(ref r) if r.start == 10 && r.end == 50));
    }

    #[test]
    fn parses_bare_https_as_url() {
        let i = Item::parse_add_argument("https://example.com/x.md").unwrap();
        assert!(matches!(i.source, Source::Url(_)));
    }

    #[test]
    fn parses_url_prefix() {
        let i = Item::parse_add_argument("url:https://example.com/x.md").unwrap();
        assert!(matches!(i.source, Source::Url(ref u) if u.url == "https://example.com/x.md"));
    }

    #[test]
    fn parses_canonical_url_uri() {
        let i = Item::parse_add_argument("url://example.com/x.md?q=a").unwrap();
        assert!(matches!(&i.source, Source::Url(u) if u.url == "https://example.com/x.md?q=a"));
    }

    #[test]
    fn display_falls_back_to_source_label() {
        let i = Item {
            source: Source::File(FileSource {
                path: "a.rs".into(),
            }),
            label: None,
        };
        assert_eq!(i.display(), "a.rs");
    }

    #[test]
    fn display_uses_label_override() {
        let i = Item {
            source: Source::File(FileSource {
                path: "a.rs".into(),
            }),
            label: Some("entry".into()),
        };
        assert_eq!(i.display(), "entry");
    }

    #[test]
    fn does_not_eat_colon_in_non_range() {
        let i = Item::parse_add_argument("https://example.com/file").unwrap();
        assert!(matches!(i.source, Source::Url(_)));
    }

    #[test]
    fn parses_pasted_github_issue_url() {
        let i = Item::parse_add_argument("https://github.com/tokio-rs/axum/issues/1234").unwrap();
        assert!(matches!(&i.source, Source::Gh(_)));
        assert_eq!(
            i.source.to_uri().to_string(),
            "gh:///tokio-rs/axum/issues/1234",
        );
    }

    #[test]
    fn parses_pasted_github_pr_url() {
        let i = Item::parse_add_argument("https://github.com/foo/bar/pull/7").unwrap();
        assert!(matches!(&i.source, Source::Gh(_)));
    }

    #[test]
    fn pasted_github_wiki_falls_back_to_url_source() {
        let i = Item::parse_add_argument("https://github.com/foo/bar/wiki/Home").unwrap();
        assert!(matches!(&i.source, Source::Url(_)));
    }
}
