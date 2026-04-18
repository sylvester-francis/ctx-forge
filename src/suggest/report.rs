//! Report types + rendering for `ctxforge suggest`.

use crate::source::{DocsSource, Ecosystem};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MissingDep {
    /// Canonicalised name (matches the form a `DocsSource.name` would take).
    pub name: String,
    pub ecosystem: Ecosystem,
    /// Project-relative paths of files that import this package.
    pub imported_in: Vec<PathBuf>,
    /// The command the user would run to fix this.
    pub suggested_command: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StaleDep {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub reason: StaleReason,
    pub suggested_command: String,
    #[serde(skip)]
    pub source: DocsSource,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    NotImported,
    NotImportedAndManuallyAdded,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestReport {
    pub missing: Vec<MissingDep>,
    pub stale: Vec<StaleDep>,
    pub warnings: Vec<String>,
}

impl SuggestReport {
    pub fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.stale.is_empty()
    }

    pub fn total(&self) -> usize {
        self.missing.len() + self.stale.len()
    }
}

/// Render the report as human-readable text.
pub fn render_human(report: &SuggestReport) -> String {
    if report.is_empty() {
        return String::from("✓ no suggestions — bundle matches imports\n");
    }

    let mut out = String::new();
    out.push_str(&format!(
        "⚠ {} missing, {} stale\n\n",
        report.missing.len(),
        report.stale.len()
    ));

    if !report.missing.is_empty() {
        out.push_str("MISSING — imported but not in Project stack:\n");
        for dep in &report.missing {
            let paths_str = format_paths(&dep.imported_in, 5);
            out.push_str(&format!(
                "  {}/{} — imported in {}\n    fix: {}\n\n",
                dep.ecosystem.as_str(),
                dep.name,
                paths_str,
                dep.suggested_command,
            ));
        }
    }

    if !report.stale.is_empty() {
        out.push_str("STALE — in Project stack but no file imports it:\n");
        for dep in &report.stale {
            out.push_str(&format!(
                "  {}/{} — {}\n",
                dep.ecosystem.as_str(),
                dep.name,
                dep.suggested_command,
            ));
            if matches!(dep.reason, StaleReason::NotImportedAndManuallyAdded) {
                out.push_str("    (manually added — confirm before removing)\n");
            }
            out.push('\n');
        }
    }

    out.push_str("Run `ctxforge suggest --apply` to apply all, or cherry-pick commands above.\n");

    for w in &report.warnings {
        out.push_str(&format!("⚠ warning: {w}\n"));
    }

    out
}

fn format_paths(paths: &[PathBuf], cap: usize) -> String {
    let display: Vec<String> = paths
        .iter()
        .take(cap)
        .map(|p| p.display().to_string())
        .collect();
    if paths.len() > cap {
        format!("{}, +{} more", display.join(", "), paths.len() - cap)
    } else {
        display.join(", ")
    }
}

/// Render as JSON (for `--json` and MCP).
pub fn render_json(report: &SuggestReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{DocsSource, DocsTier, Ecosystem};

    fn sample_missing() -> MissingDep {
        MissingDep {
            name: "sqlx".into(),
            ecosystem: Ecosystem::Rust,
            imported_in: vec![PathBuf::from("src/db.rs")],
            suggested_command: "ctxforge docs add sqlx".into(),
        }
    }

    fn sample_docs_source() -> DocsSource {
        DocsSource {
            name: "async-std".into(),
            version: "1.0".into(),
            ecosystem: Ecosystem::Rust,
            tier: DocsTier::AsyncRuntime,
            url: "https://docs.rs/async-std/".into(),
            description: None,
            manifest_path: None,
            forge: None,
        }
    }

    #[test]
    fn empty_report_renders_success_marker() {
        let r = SuggestReport {
            missing: vec![],
            stale: vec![],
            warnings: vec![],
        };
        let out = render_human(&r);
        assert!(out.contains("no suggestions"));
    }

    #[test]
    fn missing_render_shows_fix_command() {
        let r = SuggestReport {
            missing: vec![sample_missing()],
            stale: vec![],
            warnings: vec![],
        };
        let out = render_human(&r);
        assert!(out.contains("⚠ 1 missing"));
        assert!(out.contains("rust/sqlx"));
        assert!(out.contains("imported in src/db.rs"));
        assert!(out.contains("ctxforge docs add sqlx"));
    }

    #[test]
    fn stale_manually_added_shows_warning_line() {
        let stale = StaleDep {
            name: "async-std".into(),
            ecosystem: Ecosystem::Rust,
            reason: StaleReason::NotImportedAndManuallyAdded,
            suggested_command: "ctxforge docs rm async-std".into(),
            source: sample_docs_source(),
        };
        let r = SuggestReport {
            missing: vec![],
            stale: vec![stale],
            warnings: vec![],
        };
        let out = render_human(&r);
        assert!(out.contains("manually added — confirm before removing"));
    }

    #[test]
    fn path_list_caps_and_shows_remainder() {
        let many: Vec<PathBuf> = (0..8).map(|i| PathBuf::from(format!("f{i}.rs"))).collect();
        let s = format_paths(&many, 5);
        assert!(s.starts_with("f0.rs, f1.rs"));
        assert!(s.contains("+3 more"));
    }

    #[test]
    fn json_roundtrip_preserves_fields() {
        let r = SuggestReport {
            missing: vec![sample_missing()],
            stale: vec![],
            warnings: vec!["skipped bad.rs: read error".into()],
        };
        let json = render_json(&r);
        assert!(json.contains("\"sqlx\""));
        assert!(json.contains("\"ctxforge docs add sqlx\""));
        assert!(json.contains("\"skipped bad.rs"));
    }
}
