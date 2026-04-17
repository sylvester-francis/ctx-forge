//! Context source abstraction.

pub mod file;
pub mod func;
pub mod range;
pub mod type_;
pub mod uri;
pub mod url;

pub use file::FileSource;
pub use func::FuncSource;
pub use range::RangeSource;
pub use type_::TypeSource;
pub use uri::{KNOWN_SCHEMES, Uri, UriParseError};
pub use url::UrlSource;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    File(FileSource),
    Range(RangeSource),
    Func(FuncSource),
    Type(TypeSource),
    Url(UrlSource),
}

impl Source {
    pub fn from_uri(uri: &Uri) -> Result<Self, UriParseError> {
        match uri.scheme.as_str() {
            "file" => Ok(Source::File(FileSource {
                path: PathBuf::from(&uri.path),
            })),
            "range" => {
                let (start, end) = parse_range_fragment(uri.fragment.as_deref())?;
                if start == 0 || end == 0 || start > end {
                    return Err(UriParseError::BadFragment(
                        "range requires 1-indexed start <= end",
                    ));
                }
                Ok(Source::Range(RangeSource {
                    path: PathBuf::from(&uri.path),
                    start,
                    end,
                }))
            }
            "func" => {
                let name = uri
                    .fragment
                    .clone()
                    .ok_or(UriParseError::MissingFragment("func URI requires #name"))?;
                Ok(Source::Func(FuncSource {
                    path: PathBuf::from(&uri.path),
                    name,
                }))
            }
            "type" => {
                let name = uri
                    .fragment
                    .clone()
                    .ok_or(UriParseError::MissingFragment("type URI requires #name"))?;
                Ok(Source::Type(TypeSource {
                    path: PathBuf::from(&uri.path),
                    name,
                }))
            }
            "url" => {
                let authority = uri.authority.as_deref().unwrap_or("");
                let mut url = format!("https://{authority}{}", uri.path);
                if !uri.query.is_empty() {
                    url.push('?');
                    for (i, (k, v)) in uri.query.iter().enumerate() {
                        if i > 0 {
                            url.push('&');
                        }
                        if v.is_empty() {
                            url.push_str(k);
                        } else {
                            url.push_str(&format!("{k}={v}"));
                        }
                    }
                }
                if let Some(frag) = &uri.fragment {
                    url.push('#');
                    url.push_str(frag);
                }
                Ok(Source::Url(UrlSource { url }))
            }
            _ => Err(UriParseError::UnknownScheme(uri.scheme.clone())),
        }
    }

    pub fn to_uri(&self) -> Uri {
        match self {
            Source::File(f) => Uri {
                scheme: "file".into(),
                authority: None,
                path: f.path.to_string_lossy().into_owned(),
                query: vec![],
                fragment: None,
            },
            Source::Range(r) => Uri {
                scheme: "range".into(),
                authority: None,
                path: r.path.to_string_lossy().into_owned(),
                query: vec![],
                fragment: Some(format!("L{}-L{}", r.start, r.end)),
            },
            Source::Func(f) => Uri {
                scheme: "func".into(),
                authority: None,
                path: f.path.to_string_lossy().into_owned(),
                query: vec![],
                fragment: Some(f.name.clone()),
            },
            Source::Type(t) => Uri {
                scheme: "type".into(),
                authority: None,
                path: t.path.to_string_lossy().into_owned(),
                query: vec![],
                fragment: Some(t.name.clone()),
            },
            Source::Url(u) => parse_url_into_uri_parts(&u.url),
        }
    }

    pub fn is_cacheable(&self) -> bool {
        matches!(self, Source::Url(_))
    }

    pub fn default_ttl(&self) -> Duration {
        match self {
            Source::Url(_) => Duration::from_secs(86_400),
            _ => Duration::from_secs(0),
        }
    }

    /// Returns sha256 hex of the canonical URI form. Empty for local schemes.
    pub fn cache_key(&self) -> String {
        if self.is_cacheable() {
            crate::cache::sha256_hex(self.to_uri().to_string().as_bytes())
        } else {
            String::new()
        }
    }

    pub fn display_path(&self) -> Option<&PathBuf> {
        match self {
            Source::File(f) => Some(&f.path),
            Source::Range(r) => Some(&r.path),
            Source::Func(f) => Some(&f.path),
            Source::Type(t) => Some(&t.path),
            Source::Url(_) => None,
        }
    }

    pub fn display_label(&self) -> String {
        match self {
            Source::File(f) => f.path.display().to_string(),
            Source::Range(r) => format!("{}:{}-{}", r.path.display(), r.start, r.end),
            Source::Func(f) => format!("fn:{} ({})", f.name, f.path.display()),
            Source::Type(t) => format!("type:{} ({})", t.name, t.path.display()),
            Source::Url(u) => u.url.clone(),
        }
    }

    pub fn scheme_name(&self) -> &'static str {
        match self {
            Source::File(_) => "file",
            Source::Range(_) => "range",
            Source::Func(_) => "func",
            Source::Type(_) => "type",
            Source::Url(_) => "url",
        }
    }
}

fn parse_range_fragment(frag: Option<&str>) -> Result<(usize, usize), UriParseError> {
    let frag = frag.ok_or(UriParseError::MissingFragment(
        "range URI requires #Lstart-Lend",
    ))?;
    let body = frag.trim_start_matches('L');
    let (start_s, end_s) = body
        .split_once("-L")
        .or_else(|| body.split_once('-'))
        .ok_or(UriParseError::BadFragment(
            "range fragment must be Lstart-Lend",
        ))?;
    let start: usize = start_s
        .parse()
        .map_err(|_| UriParseError::BadFragment("range start not numeric"))?;
    let end: usize = end_s
        .parse()
        .map_err(|_| UriParseError::BadFragment("range end not numeric"))?;
    Ok((start, end))
}

fn parse_url_into_uri_parts(url: &str) -> Uri {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let (stripped, fragment) = match stripped.split_once('#') {
        Some((r, f)) => (r, Some(f.to_string())),
        None => (stripped, None),
    };
    let (stripped, query) = match stripped.split_once('?') {
        Some((r, q)) => {
            let pairs: Vec<(String, String)> = q
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once('=') {
                    Some((k, v)) => (k.to_string(), v.to_string()),
                    None => (p.to_string(), String::new()),
                })
                .collect();
            (r, pairs)
        }
        None => (stripped, Vec::new()),
    };
    let (authority, path) = match stripped.split_once('/') {
        Some((a, p)) => (Some(a.to_string()), format!("/{p}")),
        None => (Some(stripped.to_string()), String::new()),
    };
    Uri {
        scheme: "url".into(),
        authority,
        path,
        query,
        fragment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_source_roundtrip() {
        let uri: Uri = "file:///src/main.rs".parse().unwrap();
        let source = Source::from_uri(&uri).unwrap();
        assert!(matches!(&source, Source::File(f) if f.path == PathBuf::from("/src/main.rs")));
        assert_eq!(source.to_uri().to_string(), "file:///src/main.rs");
    }

    #[test]
    fn range_source_roundtrip() {
        let uri: Uri = "range:///src/main.rs#L10-L20".parse().unwrap();
        let source = Source::from_uri(&uri).unwrap();
        assert!(matches!(&source, Source::Range(r) if r.start == 10 && r.end == 20));
        assert_eq!(source.to_uri().to_string(), "range:///src/main.rs#L10-L20");
    }

    #[test]
    fn range_rejects_zero_and_reversed() {
        let bad1: Uri = "range:///f.rs#L0-L5".parse().unwrap();
        assert!(Source::from_uri(&bad1).is_err());
        let bad2: Uri = "range:///f.rs#L10-L5".parse().unwrap();
        assert!(Source::from_uri(&bad2).is_err());
    }

    #[test]
    fn func_source_roundtrip() {
        let uri: Uri = "func:///src/p.rs#parse_expr".parse().unwrap();
        let s = Source::from_uri(&uri).unwrap();
        assert!(matches!(&s, Source::Func(f) if f.name == "parse_expr"));
        assert_eq!(s.to_uri().to_string(), "func:///src/p.rs#parse_expr");
    }

    #[test]
    fn url_roundtrip_preserves_query_and_fragment() {
        let uri: Uri = "url://example.com/search?q=react&l=en#hits".parse().unwrap();
        let s = Source::from_uri(&uri).unwrap();
        if let Source::Url(u) = &s {
            assert_eq!(u.url, "https://example.com/search?q=react&l=en#hits");
        } else {
            panic!("expected Url");
        }
        assert_eq!(
            s.to_uri().to_string(),
            "url://example.com/search?q=react&l=en#hits"
        );
    }

    #[test]
    fn cache_key_differs_for_different_queries() {
        let a = Source::from_uri(&"url://example.com/s?q=a".parse().unwrap()).unwrap();
        let b = Source::from_uri(&"url://example.com/s?q=b".parse().unwrap()).unwrap();
        assert_ne!(a.cache_key(), b.cache_key());
    }

    #[test]
    fn serde_tag_file() {
        let s = Source::File(FileSource {
            path: "a.rs".into(),
        });
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains(r#""kind":"file""#));
        let back: Source = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn fs_sources_not_cacheable() {
        let f = Source::File(FileSource {
            path: "a.rs".into(),
        });
        assert!(!f.is_cacheable());
        assert!(f.cache_key().is_empty());
    }

    #[test]
    fn url_sources_cacheable() {
        let u = Source::Url(UrlSource {
            url: "https://example.com/foo".into(),
        });
        assert!(u.is_cacheable());
        assert_eq!(u.cache_key().len(), 64);
    }

    #[test]
    fn scheme_names() {
        assert_eq!(
            Source::File(FileSource { path: "a".into() }).scheme_name(),
            "file"
        );
        assert_eq!(
            Source::Url(UrlSource { url: "".into() }).scheme_name(),
            "url"
        );
    }
}
