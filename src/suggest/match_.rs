//! Canonicalise raw import tokens to candidate `DocsSource.name` values,
//! handling hyphen/underscore differences between source and registries.

use crate::source::Ecosystem;

pub fn candidates(raw: &str, ecosystem: Ecosystem) -> Vec<String> {
    let mut out = vec![raw.to_string()];
    match ecosystem {
        Ecosystem::Rust | Ecosystem::Python => {
            if raw.contains('_') {
                out.push(raw.replace('_', "-"));
            } else if raw.contains('-') {
                out.push(raw.replace('-', "_"));
            }
        }
        Ecosystem::Js => {
            let lower = raw.to_lowercase();
            if lower != raw {
                out.push(lower);
            }
        }
        Ecosystem::Go => {}
    }
    out
}

pub fn matches_source(raw: &str, name: &str, ecosystem: Ecosystem) -> bool {
    let raw_candidates = candidates(raw, ecosystem);
    let name_candidates = candidates(name, ecosystem);
    raw_candidates.iter().any(|r| name_candidates.contains(r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_underscore_matches_hyphen_source() {
        assert!(matches_source("serde_json", "serde-json", Ecosystem::Rust));
        assert!(matches_source("serde_json", "serde_json", Ecosystem::Rust));
    }

    #[test]
    fn rust_hyphen_matches_underscore_source() {
        assert!(matches_source("my-crate", "my_crate", Ecosystem::Rust));
    }

    #[test]
    fn rust_no_match_for_unrelated_names() {
        assert!(!matches_source("tokio", "sqlx", Ecosystem::Rust));
    }

    #[test]
    fn js_case_insensitive_match() {
        assert!(matches_source("React", "react", Ecosystem::Js));
    }

    #[test]
    fn js_exact_match_for_scoped() {
        assert!(matches_source("@next/core", "@next/core", Ecosystem::Js));
    }

    #[test]
    fn python_hyphen_underscore_bidirectional() {
        assert!(matches_source(
            "typing_extensions",
            "typing-extensions",
            Ecosystem::Python,
        ));
    }

    #[test]
    fn go_literal_match_only() {
        assert!(matches_source(
            "github.com/gin-gonic/gin",
            "github.com/gin-gonic/gin",
            Ecosystem::Go,
        ));
        assert!(!matches_source(
            "gin",
            "github.com/gin-gonic/gin",
            Ecosystem::Go,
        ));
    }
}
