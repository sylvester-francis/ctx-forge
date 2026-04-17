//! URI type — the canonical identifier for every context source.
//!
//! Minimal RFC 3986 subset: `scheme://authority/path?query#fragment`.
//! Only registered schemes are accepted (file, range, func, type, url).
//! The URI form preserves query and fragment so `to_string` round-trips
//! and cache keys distinguish `?q=a` from `?q=b`.

use std::fmt;
use std::str::FromStr;

pub const KNOWN_SCHEMES: &[&str] = &["file", "range", "func", "type", "url"];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri {
    pub scheme: String,
    pub authority: Option<String>,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub fragment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriParseError {
    NoScheme,
    UnknownScheme(String),
    InvalidChar(char),
    DotSegment,
    NullByte,
    UserInfo,
    MissingFragment(&'static str),
    BadFragment(&'static str),
}

impl fmt::Display for UriParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoScheme => write!(f, "missing scheme (expected scheme://...)"),
            Self::UnknownScheme(s) => write!(
                f,
                "unknown scheme `{s}` (expected one of: {})",
                KNOWN_SCHEMES.join(", "),
            ),
            Self::InvalidChar(c) => write!(f, "invalid character `{c}` in URI"),
            Self::DotSegment => write!(f, "URI contains `..` segment (path traversal rejected)"),
            Self::NullByte => write!(f, "URI contains null byte"),
            Self::UserInfo => write!(f, "userinfo (user:pass@host) is rejected"),
            Self::MissingFragment(s) => write!(f, "missing fragment: {s}"),
            Self::BadFragment(s) => write!(f, "bad fragment: {s}"),
        }
    }
}

impl std::error::Error for UriParseError {}

impl FromStr for Uri {
    type Err = UriParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for c in s.chars() {
            if c == '\0' {
                return Err(UriParseError::NullByte);
            }
            if c.is_control() && c != '\t' {
                return Err(UriParseError::InvalidChar(c));
            }
        }

        let (scheme, rest) = s.split_once("://").ok_or(UriParseError::NoScheme)?;
        let scheme = scheme.to_ascii_lowercase();
        if !KNOWN_SCHEMES.contains(&scheme.as_str()) {
            return Err(UriParseError::UnknownScheme(scheme));
        }

        let (rest, fragment) = match rest.split_once('#') {
            Some((r, f)) => (r, Some(f.to_string())),
            None => (rest, None),
        };

        let (rest, query) = match rest.split_once('?') {
            Some((r, q)) => {
                let pairs = q
                    .split('&')
                    .filter(|p| !p.is_empty())
                    .map(|p| match p.split_once('=') {
                        Some((k, v)) => (k.to_string(), v.to_string()),
                        None => (p.to_string(), String::new()),
                    })
                    .collect();
                (r, pairs)
            }
            None => (rest, Vec::new()),
        };

        let (authority, path) = if scheme == "url" {
            match rest.split_once('/') {
                Some((auth, p)) => {
                    if auth.contains('@') {
                        return Err(UriParseError::UserInfo);
                    }
                    (Some(auth.to_string()), format!("/{p}"))
                }
                None => {
                    if rest.contains('@') {
                        return Err(UriParseError::UserInfo);
                    }
                    (Some(rest.to_string()), String::new())
                }
            }
        } else {
            (None, rest.to_string())
        };

        if path.split('/').any(|seg| seg == "..") {
            return Err(UriParseError::DotSegment);
        }

        Ok(Uri {
            scheme,
            authority,
            path,
            query,
            fragment,
        })
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://", self.scheme)?;
        if let Some(auth) = &self.authority {
            write!(f, "{auth}")?;
        }
        write!(f, "{}", self.path)?;
        if !self.query.is_empty() {
            write!(f, "?")?;
            for (i, (k, v)) in self.query.iter().enumerate() {
                if i > 0 {
                    write!(f, "&")?;
                }
                if v.is_empty() {
                    write!(f, "{k}")?;
                } else {
                    write!(f, "{k}={v}")?;
                }
            }
        }
        if let Some(frag) = &self.fragment {
            write!(f, "#{frag}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_uri() {
        let uri: Uri = "file:///abs/path/foo.rs".parse().unwrap();
        assert_eq!(uri.scheme, "file");
        assert_eq!(uri.authority, None);
        assert_eq!(uri.path, "/abs/path/foo.rs");
    }

    #[test]
    fn parse_url_preserves_query_and_fragment() {
        let uri: Uri = "url://example.com/search?q=react&lang=en#hits"
            .parse()
            .unwrap();
        assert_eq!(uri.authority, Some("example.com".into()));
        assert_eq!(uri.path, "/search");
        assert_eq!(
            uri.query,
            vec![
                ("q".into(), "react".into()),
                ("lang".into(), "en".into()),
            ]
        );
        assert_eq!(uri.fragment, Some("hits".into()));
    }

    #[test]
    fn roundtrip_every_known_form() {
        let inputs = [
            "file:///abs/path/foo.rs",
            "range:///abs/path/foo.rs#L10-L20",
            "func:///src/parser.rs#parse_expr",
            "type:///src/models.rs#Config",
            "url://example.com/docs?q=react&lang=en",
            "url://docs.rs/tokio/latest/tokio/sync/oneshot/fn.channel.html",
        ];
        for input in inputs {
            let uri: Uri = input.parse().unwrap();
            assert_eq!(uri.to_string(), input, "roundtrip failed for: {input}");
        }
    }

    #[test]
    fn reject_unknown_scheme() {
        assert!(matches!(
            "ftp://example.com".parse::<Uri>().unwrap_err(),
            UriParseError::UnknownScheme(_)
        ));
        assert!(matches!(
            "javascript://alert(1)".parse::<Uri>().unwrap_err(),
            UriParseError::UnknownScheme(_)
        ));
    }

    #[test]
    fn reject_dot_segments() {
        assert!(matches!(
            "file:///abs/../etc/passwd".parse::<Uri>().unwrap_err(),
            UriParseError::DotSegment
        ));
    }

    #[test]
    fn reject_null_and_control() {
        assert!(matches!(
            "file:///abs/\0path".parse::<Uri>().unwrap_err(),
            UriParseError::NullByte
        ));
        assert!(matches!(
            "file:///abs/\x01bad".parse::<Uri>().unwrap_err(),
            UriParseError::InvalidChar(_)
        ));
    }

    #[test]
    fn reject_userinfo() {
        assert!(matches!(
            "url://u:p@example.com/foo".parse::<Uri>().unwrap_err(),
            UriParseError::UserInfo
        ));
    }

    #[test]
    fn reject_no_scheme() {
        assert!(matches!(
            "just-a-path".parse::<Uri>().unwrap_err(),
            UriParseError::NoScheme
        ));
    }
}
