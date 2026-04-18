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
        "javascript" | "typescript" | "tsx" | "jsx" => (scan_js(source), Ecosystem::Js),
        "python" => (scan_python(source), Ecosystem::Python),
        "go" => (scan_go(source), Ecosystem::Go),
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

fn scan_js(source: &str) -> Vec<String> {
    static IMPORT_FROM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*import\s+.+?\s+from\s+['"]([^'"]+)['"]"#).unwrap()
    });
    static IMPORT_BARE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?m)^\s*import\s+['"]([^'"]+)['"]"#).unwrap());
    static REQUIRE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"require\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap());

    let mut out = Vec::new();
    for re in [&*IMPORT_FROM, &*IMPORT_BARE, &*REQUIRE] {
        for cap in re.captures_iter(source) {
            if let Some(pkg) = normalize_js_package(&cap[1]) {
                out.push(pkg);
            }
        }
    }
    out
}

/// JS package normalisation:
/// - relative / absolute paths (start with `.` or `/`) → None
/// - URL imports (`http://`, `https://`) → None
/// - scoped package (`@scope/name`) → keep as `@scope/name`
/// - deep scoped (`@scope/name/sub`) → `@scope/name`
/// - deep unscoped (`lodash/debounce`) → `lodash`
/// - bare name (`react`) → `react`
fn scan_python(source: &str) -> Vec<String> {
    static IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*import\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)")
            .unwrap()
    });
    static FROM_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^\s*from\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s+import")
            .unwrap()
    });

    let mut out = Vec::new();
    for cap in IMPORT_RE.captures_iter(source) {
        if let Some(first) = cap[1].split('.').next() {
            out.push(first.to_string());
        }
    }
    for cap in FROM_RE.captures_iter(source) {
        if let Some(first) = cap[1].split('.').next() {
            out.push(first.to_string());
        }
    }
    out
}

fn scan_go(source: &str) -> Vec<String> {
    static SINGLE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?m)^\s*import\s+"([^"]+)""#).unwrap());
    static BLOCK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)import\s*\(([^)]*)\)"#).unwrap());
    static QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""([^"]+)""#).unwrap());

    let mut out = Vec::new();
    for cap in SINGLE.captures_iter(source) {
        out.push(cap[1].to_string());
    }
    for cap in BLOCK.captures_iter(source) {
        for inner in QUOTED.captures_iter(&cap[1]) {
            out.push(inner[1].to_string());
        }
    }
    out
}

fn normalize_js_package(raw: &str) -> Option<String> {
    if raw.starts_with('.') || raw.starts_with('/') {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return None;
    }
    if raw.starts_with('@') {
        let mut parts = raw.splitn(3, '/');
        let scope = parts.next()?;
        let name = parts.next()?;
        return Some(format!("{scope}/{name}"));
    }
    let first = raw.split('/').next()?;
    if first.is_empty() {
        return None;
    }
    Some(first.to_string())
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

    #[test]
    fn js_import_from_named() {
        let src = "import { foo } from 'react';\n";
        let imports = scan_imports(src, "javascript");
        assert_eq!(imports, vec![("react".to_string(), Ecosystem::Js)]);
    }

    #[test]
    fn js_import_default() {
        let src = "import axios from 'axios';\n";
        let imports = scan_imports(src, "typescript");
        assert_eq!(imports, vec![("axios".to_string(), Ecosystem::Js)]);
    }

    #[test]
    fn js_scoped_package_preserved() {
        let src = "import { Button } from '@next/core';\n";
        let imports = scan_imports(src, "tsx");
        assert_eq!(imports, vec![("@next/core".to_string(), Ecosystem::Js)]);
    }

    #[test]
    fn js_deep_import_truncated_to_package_root() {
        let src = "import debounce from 'lodash/debounce';\n";
        let imports = scan_imports(src, "javascript");
        assert_eq!(imports, vec![("lodash".to_string(), Ecosystem::Js)]);
    }

    #[test]
    fn js_deep_scoped_truncated_to_scope_slash_name() {
        let src = "import { use } from '@remix-run/router/hooks';\n";
        let imports = scan_imports(src, "javascript");
        assert_eq!(
            imports,
            vec![("@remix-run/router".to_string(), Ecosystem::Js)]
        );
    }

    #[test]
    fn js_skips_relative_and_url_imports() {
        let src = r#"
import a from './foo';
import b from '../bar';
import c from '/abs/path';
import d from 'https://esm.sh/react';
"#;
        assert!(scan_imports(src, "javascript").is_empty());
    }

    #[test]
    fn js_skips_node_builtins_and_node_prefix() {
        let src = "import fs from 'fs';\nimport path from 'node:path';\n";
        assert!(scan_imports(src, "javascript").is_empty());
    }

    #[test]
    fn js_require_syntax() {
        let src = "const express = require('express');\n";
        let imports = scan_imports(src, "javascript");
        assert_eq!(imports, vec![("express".to_string(), Ecosystem::Js)]);
    }

    #[test]
    fn js_bare_import_side_effect() {
        let src = "import 'zone.js';\n";
        let imports = scan_imports(src, "javascript");
        assert_eq!(imports, vec![("zone.js".to_string(), Ecosystem::Js)]);
    }

    #[test]
    fn python_plain_import() {
        let src = "import requests\n";
        let imports = scan_imports(src, "python");
        assert_eq!(imports, vec![("requests".to_string(), Ecosystem::Python)]);
    }

    #[test]
    fn python_from_import() {
        let src = "from fastapi import FastAPI\n";
        let imports = scan_imports(src, "python");
        assert_eq!(imports, vec![("fastapi".to_string(), Ecosystem::Python)]);
    }

    #[test]
    fn python_dotted_path_truncated_to_root() {
        let src = "from google.cloud.storage import Client\n";
        let imports = scan_imports(src, "python");
        assert_eq!(imports, vec![("google".to_string(), Ecosystem::Python)]);
    }

    #[test]
    fn python_skips_stdlib() {
        let src = "import os\nimport sys\nfrom typing import Optional\nimport json\n";
        assert!(scan_imports(src, "python").is_empty());
    }

    #[test]
    fn python_skips_relative_imports() {
        let src = "from .local import Helper\nfrom ..parent import Base\n";
        assert!(scan_imports(src, "python").is_empty());
    }

    #[test]
    fn python_import_as_alias() {
        let src = "import numpy as np\n";
        let imports = scan_imports(src, "python");
        assert_eq!(imports, vec![("numpy".to_string(), Ecosystem::Python)]);
    }

    #[test]
    fn go_single_line_import() {
        let src = r#"package main

import "github.com/gin-gonic/gin"
"#;
        let imports = scan_imports(src, "go");
        assert_eq!(
            imports,
            vec![("github.com/gin-gonic/gin".to_string(), Ecosystem::Go)]
        );
    }

    #[test]
    fn go_block_import() {
        let src = r#"package main

import (
    "fmt"
    "net/http"
    "github.com/gin-gonic/gin"
    "golang.org/x/sync/errgroup"
)
"#;
        let imports = scan_imports(src, "go");
        assert_eq!(
            imports,
            vec![
                ("github.com/gin-gonic/gin".to_string(), Ecosystem::Go),
                ("golang.org/x/sync/errgroup".to_string(), Ecosystem::Go),
            ]
        );
    }

    #[test]
    fn go_skips_stdlib() {
        let src = r#"import (
    "context"
    "encoding/json"
    "net/http"
)"#;
        assert!(scan_imports(src, "go").is_empty());
    }
}
