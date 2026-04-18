//! DocsSource — a reference to a library's canonical documentation.
//!
//! Links-only: the content we emit is a single rendered line, not the doc
//! body. Network I/O at `docs detect` time only (to fetch the one-line
//! description); resolve-time is purely local.

use crate::gh::forge::ForgeRef;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    Rust,
    Js,
    Python,
    Go,
}

impl Ecosystem {
    pub fn as_str(&self) -> &'static str {
        match self {
            Ecosystem::Rust => "rust",
            Ecosystem::Js => "js",
            Ecosystem::Python => "python",
            Ecosystem::Go => "go",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "rust" => Some(Ecosystem::Rust),
            "js" | "javascript" | "typescript" | "ts" => Some(Ecosystem::Js),
            "python" | "py" => Some(Ecosystem::Python),
            "go" | "golang" => Some(Ecosystem::Go),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Ecosystem::Rust => "Rust",
            Ecosystem::Js => "JS/TS",
            Ecosystem::Python => "Python",
            Ecosystem::Go => "Go",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsTier {
    Framework,
    Database,
    AsyncRuntime,
    LanguageCore,
    Library,
}

impl DocsTier {
    pub fn label(&self) -> &'static str {
        match self {
            DocsTier::Framework => "Framework",
            DocsTier::Database => "Database",
            DocsTier::AsyncRuntime => "Async runtime",
            DocsTier::LanguageCore => "Language core",
            DocsTier::Library => "Library",
        }
    }

    /// Tiers that render in the default (non-`--all`) emission.
    pub fn is_default_emit(&self) -> bool {
        !matches!(self, DocsTier::Library)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsSource {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    pub tier: DocsTier,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forge: Option<ForgeRef>,
}

impl DocsSource {
    /// The single line rendered as this source's "content" at resolve time.
    pub fn render_line(&self) -> String {
        let desc = self
            .description
            .as_deref()
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        format!(
            "- **{}**: {} {} — {}{}",
            self.tier.label(),
            self.name,
            self.version,
            self.url,
            desc,
        )
    }

    /// Deterministic sort order for tiers within a subsection.
    pub fn tier_order(&self) -> u8 {
        match self.tier {
            DocsTier::LanguageCore => 0,
            DocsTier::Framework => 1,
            DocsTier::Database => 2,
            DocsTier::AsyncRuntime => 3,
            DocsTier::Library => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_roundtrip() {
        for eco in [
            Ecosystem::Rust,
            Ecosystem::Js,
            Ecosystem::Python,
            Ecosystem::Go,
        ] {
            assert_eq!(Ecosystem::from_str(eco.as_str()), Some(eco));
        }
    }

    #[test]
    fn tier_default_emit() {
        assert!(DocsTier::Framework.is_default_emit());
        assert!(DocsTier::Database.is_default_emit());
        assert!(DocsTier::AsyncRuntime.is_default_emit());
        assert!(DocsTier::LanguageCore.is_default_emit());
        assert!(!DocsTier::Library.is_default_emit());
    }

    #[test]
    fn render_line_with_description() {
        let d = DocsSource {
            name: "axum".into(),
            version: "0.7.5".into(),
            ecosystem: Ecosystem::Rust,
            tier: DocsTier::Framework,
            url: "https://docs.rs/axum/0.7.5/".into(),
            description: Some("Ergonomic web framework".into()),
            manifest_path: None,
            forge: None,
        };
        assert_eq!(
            d.render_line(),
            "- **Framework**: axum 0.7.5 — https://docs.rs/axum/0.7.5/ (Ergonomic web framework)",
        );
    }

    #[test]
    fn render_line_without_description() {
        let d = DocsSource {
            name: "gin".into(),
            version: "v1.10.0".into(),
            ecosystem: Ecosystem::Go,
            tier: DocsTier::Framework,
            url: "https://pkg.go.dev/github.com/gin-gonic/gin@v1.10.0".into(),
            description: None,
            manifest_path: None,
            forge: None,
        };
        assert!(d.render_line().ends_with("@v1.10.0"));
    }

    #[test]
    fn forge_roundtrips_on_docs_source() {
        use crate::gh::forge::{ForgeHost, ForgeRef};
        let d = DocsSource {
            name: "axum".into(),
            version: "0.7.5".into(),
            ecosystem: Ecosystem::Rust,
            tier: DocsTier::Framework,
            url: "https://docs.rs/axum/0.7.5/".into(),
            description: None,
            manifest_path: None,
            forge: Some(ForgeRef {
                host: ForgeHost::GitHub,
                path: "tokio-rs/axum".into(),
                raw_url: "https://github.com/tokio-rs/axum".into(),
            }),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains(r#""host":"github""#));
        let back: DocsSource = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn forge_absent_deserialises_as_none() {
        // Existing v2 bundles (pre-P3) don't have a `forge` field.
        let raw = r#"{
            "name": "axum",
            "version": "0.7.5",
            "ecosystem": "rust",
            "tier": "framework",
            "url": "https://docs.rs/axum/0.7.5/"
        }"#;
        let d: DocsSource = serde_json::from_str(raw).unwrap();
        assert!(d.forge.is_none());
    }
}
