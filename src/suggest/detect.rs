//! Regex-based import scanner per language.
//!
//! Each `scan_*` function returns a `Vec<String>` of raw import names
//! (not canonicalised — that's match_'s job). Duplicate names within
//! a single file are deduped.

use crate::source::Ecosystem;
use crate::suggest::stdlib;
use regex::Regex;
use std::sync::LazyLock;

/// Scan a source file for imports; return list of (raw_name, ecosystem)
/// pairs with stdlib and pseudo-crate references already filtered out.
/// Non-matching languages return empty.
pub fn scan_imports(source: &str, language: &str) -> Vec<(String, Ecosystem)> {
    let (raws, eco) = match language {
        "rust" => (scan_rust(source), Ecosystem::Rust),
        _ => return Vec::new(),
    };
    let mut seen = std::collections::BTreeSet::new();
    raws.into_iter()
        .filter(|r| !stdlib::is_stdlib(r, eco))
        .filter(|r| seen.insert(r.clone()))
        .map(|r| (r, eco))
        .collect()
}

fn scan_rust(source: &str) -> Vec<String> {
    static USE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*use\s+([A-Za-z_][A-Za-z0-9_]*)(?:::|;|\s*\{)").unwrap()
    });
    static EXTERN_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*extern\s+crate\s+([A-Za-z_][A-Za-z0-9_]*)\s*;").unwrap()
    });

    let mut out = Vec::new();
    for cap in USE_RE.captures_iter(source) {
        out.push(cap[1].to_string());
    }
    for cap in EXTERN_RE.captures_iter(source) {
        out.push(cap[1].to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_use_simple() {
        let src = "use tokio::runtime::Runtime;\n";
        let imports = scan_imports(src, "rust");
        assert_eq!(imports, vec![("tokio".to_string(), Ecosystem::Rust)]);
    }

    #[test]
    fn rust_use_brace_group() {
        let src = "use serde::{Serialize, Deserialize};\n";
        let imports = scan_imports(src, "rust");
        assert_eq!(imports, vec![("serde".to_string(), Ecosystem::Rust)]);
    }

    #[test]
    fn rust_use_plain_then_semi() {
        let src = "use anyhow;\n";
        let imports = scan_imports(src, "rust");
        assert_eq!(imports, vec![("anyhow".to_string(), Ecosystem::Rust)]);
    }

    #[test]
    fn rust_extern_crate() {
        let src = "extern crate proc_macro;\nextern crate my_legacy;\n";
        let imports = scan_imports(src, "rust");
        assert_eq!(imports, vec![("my_legacy".to_string(), Ecosystem::Rust)]);
    }

    #[test]
    fn rust_skips_stdlib_and_scope_keywords() {
        let src = "use std::fs;\nuse crate::foo;\nuse self::bar;\nuse super::baz;\nuse core::mem;\n";
        let imports = scan_imports(src, "rust");
        assert!(imports.is_empty());
    }

    #[test]
    fn rust_dedupes_within_file() {
        let src = "use tokio::fs;\nuse tokio::time;\nuse tokio;\n";
        let imports = scan_imports(src, "rust");
        assert_eq!(imports, vec![("tokio".to_string(), Ecosystem::Rust)]);
    }

    #[test]
    fn rust_ignores_use_inside_string_literal() {
        let src = r#"let s = "use tokio::foo;";"#;
        let imports = scan_imports(src, "rust");
        assert!(imports.is_empty());
    }

    #[test]
    fn non_rust_language_returns_empty() {
        assert!(scan_imports("some content", "unknown").is_empty());
    }
}
