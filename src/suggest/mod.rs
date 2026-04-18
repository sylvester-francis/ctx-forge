//! Auto-suggest context — deterministic detector for missing and stale
//! documentation entries in the bundle's Project stack.
//!
//! "Missing" = a bundle (or project) file imports a package with no
//! matching `DocsSource` entry. "Stale" = a `DocsSource` entry exists
//! but no bundle/project file imports the package.
//!
//! Output is for the user, not the LLM — suggestions never leak into
//! `ctxforge export` rendered output.

pub mod detect;
pub mod match_;
pub mod report;
pub mod stdlib;

use crate::bundle::Bundle;
use crate::error::Result;
use crate::lang;
use crate::source::{DocsSource, Ecosystem, Source};
use crate::walk;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use report::{MissingDep, StaleDep, StaleReason, SuggestReport};

pub struct SuggestOptions {
    /// Default: scan bundle items only. With --all: walk whole project.
    pub scan_all_project: bool,
    /// Skip stale check; only flag missing.
    pub missing_only: bool,
}

impl Default for SuggestOptions {
    fn default() -> Self {
        Self {
            scan_all_project: false,
            missing_only: false,
        }
    }
}

/// Scan, compare, produce a report. Pure function — no filesystem
/// mutation, no network I/O, no cache writes.
pub fn run_suggest(
    bundle: &Bundle,
    project_root: &Path,
    options: &SuggestOptions,
) -> Result<SuggestReport> {
    let files = collect_files(bundle, project_root, options.scan_all_project);
    let mut imports_by_eco: BTreeMap<(String, Ecosystem), Vec<PathBuf>> = BTreeMap::new();
    let mut warnings = Vec::new();

    for rel in &files {
        let abs = project_root.join(rel);
        let language = lang::detect(rel);
        if !is_supported_language(language) {
            continue;
        }
        let source = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(e) => {
                warnings.push(format!("skipped {}: {e}", rel.display()));
                continue;
            }
        };
        for (raw, eco) in detect::scan_imports(&source, language) {
            imports_by_eco
                .entry((raw, eco))
                .or_default()
                .push(rel.clone());
        }
    }

    let existing = collect_existing_docs(bundle);

    let missing = compute_missing(&imports_by_eco, &existing);
    let stale = if options.missing_only {
        Vec::new()
    } else {
        compute_stale(&existing, &imports_by_eco)
    };

    Ok(SuggestReport {
        missing,
        stale,
        warnings,
    })
}

fn is_supported_language(language: &str) -> bool {
    matches!(
        language,
        "rust" | "javascript" | "typescript" | "tsx" | "jsx" | "python" | "go"
    )
}

fn collect_files(bundle: &Bundle, project_root: &Path, scan_all: bool) -> Vec<PathBuf> {
    if scan_all {
        return walk::expand("**/*", project_root, &[]).unwrap_or_default();
    }
    let mut out = Vec::new();
    for item in &bundle.items {
        let Some(path) = item.source.display_path() else {
            continue;
        };
        out.push(path.clone());
    }
    out
}

fn collect_existing_docs(bundle: &Bundle) -> Vec<&DocsSource> {
    bundle
        .items
        .iter()
        .filter_map(|i| match &i.source {
            Source::Docs(d) => Some(d),
            _ => None,
        })
        .collect()
}

fn compute_missing(
    imports: &BTreeMap<(String, Ecosystem), Vec<PathBuf>>,
    existing: &[&DocsSource],
) -> Vec<MissingDep> {
    let mut out = Vec::new();
    for ((raw, eco), paths) in imports {
        let matched = existing
            .iter()
            .any(|d| d.ecosystem == *eco && match_::matches_source(raw, &d.name, *eco));
        if matched {
            continue;
        }
        out.push(MissingDep {
            name: raw.clone(),
            ecosystem: *eco,
            imported_in: paths.clone(),
            suggested_command: format!("ctxforge docs add {raw} --ecosystem {}", eco.as_str()),
        });
    }
    out.sort_by(|a, b| {
        a.ecosystem
            .as_str()
            .cmp(b.ecosystem.as_str())
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

fn compute_stale(
    existing: &[&DocsSource],
    imports: &BTreeMap<(String, Ecosystem), Vec<PathBuf>>,
) -> Vec<StaleDep> {
    let mut out = Vec::new();
    for d in existing {
        let imported = imports
            .keys()
            .any(|(raw, eco)| *eco == d.ecosystem && match_::matches_source(raw, &d.name, *eco));
        if imported {
            continue;
        }
        let reason = if d.manifest_path.is_none() {
            StaleReason::NotImportedAndManuallyAdded
        } else {
            StaleReason::NotImported
        };
        out.push(StaleDep {
            name: d.name.clone(),
            ecosystem: d.ecosystem,
            reason,
            suggested_command: format!("ctxforge docs rm {}", d.name),
            source: (*d).clone(),
        });
    }
    out.sort_by(|a, b| {
        a.ecosystem
            .as_str()
            .cmp(b.ecosystem.as_str())
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// A single suggestion the user can act on via `apply_suggestion`.
pub enum Suggestion<'a> {
    Missing(&'a MissingDep),
    Stale(&'a StaleDep),
}

/// Apply one suggestion. For a `Missing` dep, delegates to
/// `docs_cmd::add` (network round-trip for description / forge). For a
/// `Stale` dep, delegates to `docs_cmd::rm`. Caller is responsible for
/// reloading the bundle afterwards.
pub fn apply_suggestion(
    suggestion: &Suggestion<'_>,
    root: &crate::paths::CtxforgeRoot,
) -> Result<()> {
    match suggestion {
        Suggestion::Missing(m) => crate::commands::docs_cmd::add(
            root,
            m.name.clone(),
            Some(m.ecosystem.as_str().to_string()),
        ),
        Suggestion::Stale(s) => crate::commands::docs_cmd::rm(root, s.name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Item;
    use crate::source::{DocsSource, DocsTier, FileSource};
    use tempfile::TempDir;

    fn make_file_item(path: &str) -> Item {
        Item {
            source: Source::File(FileSource {
                path: PathBuf::from(path),
            }),
            label: None,
        }
    }

    fn make_docs_item(name: &str, eco: Ecosystem, manifest: Option<&str>) -> Item {
        Item {
            source: Source::Docs(DocsSource {
                name: name.into(),
                version: "1.0".into(),
                ecosystem: eco,
                tier: DocsTier::Library,
                url: String::new(),
                description: None,
                manifest_path: manifest.map(PathBuf::from),
                forge: None,
            }),
            label: None,
        }
    }

    #[test]
    fn missing_detected_when_import_has_no_docs_source() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("main.rs"), "use tokio::fs;\n").unwrap();

        let mut bundle = Bundle::new();
        bundle.items.push(make_file_item("main.rs"));

        let report = run_suggest(&bundle, td.path(), &SuggestOptions::default()).unwrap();
        assert_eq!(report.missing.len(), 1);
        assert_eq!(report.missing[0].name, "tokio");
        assert!(report.missing[0].suggested_command.contains("tokio"));
    }

    #[test]
    fn stale_detected_when_docs_has_no_import() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("main.rs"), "use tokio::fs;\n").unwrap();

        let mut bundle = Bundle::new();
        bundle.items.push(make_file_item("main.rs"));
        bundle
            .items
            .push(make_docs_item("tokio", Ecosystem::Rust, Some("Cargo.toml")));
        bundle.items.push(make_docs_item(
            "async-std",
            Ecosystem::Rust,
            Some("Cargo.toml"),
        ));

        let report = run_suggest(&bundle, td.path(), &SuggestOptions::default()).unwrap();
        assert!(report.missing.is_empty());
        assert_eq!(report.stale.len(), 1);
        assert_eq!(report.stale[0].name, "async-std");
        assert!(matches!(report.stale[0].reason, StaleReason::NotImported));
    }

    #[test]
    fn manually_added_stale_flagged_with_distinct_reason() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("main.rs"), "use tokio;\n").unwrap();

        let mut bundle = Bundle::new();
        bundle.items.push(make_file_item("main.rs"));
        bundle
            .items
            .push(make_docs_item("tokio", Ecosystem::Rust, Some("Cargo.toml")));
        bundle
            .items
            .push(make_docs_item("forgotten-lib", Ecosystem::Rust, None));

        let report = run_suggest(&bundle, td.path(), &SuggestOptions::default()).unwrap();
        assert_eq!(report.stale.len(), 1);
        assert!(matches!(
            report.stale[0].reason,
            StaleReason::NotImportedAndManuallyAdded
        ));
    }

    #[test]
    fn missing_only_option_skips_stale() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("main.rs"), "use tokio;\n").unwrap();

        let mut bundle = Bundle::new();
        bundle.items.push(make_file_item("main.rs"));
        bundle.items.push(make_docs_item(
            "unused-crate",
            Ecosystem::Rust,
            Some("Cargo.toml"),
        ));

        let report = run_suggest(
            &bundle,
            td.path(),
            &SuggestOptions {
                scan_all_project: false,
                missing_only: true,
            },
        )
        .unwrap();
        assert!(report.stale.is_empty());
        assert_eq!(report.missing.len(), 1);
    }

    #[test]
    fn hyphen_underscore_match_prevents_false_missing() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("main.rs"), "use serde_json;\n").unwrap();

        let mut bundle = Bundle::new();
        bundle.items.push(make_file_item("main.rs"));
        bundle.items.push(make_docs_item(
            "serde-json",
            Ecosystem::Rust,
            Some("Cargo.toml"),
        ));

        let report = run_suggest(&bundle, td.path(), &SuggestOptions::default()).unwrap();
        assert!(report.missing.is_empty());
        assert!(report.stale.is_empty());
    }

    #[test]
    fn empty_bundle_empty_report() {
        let td = TempDir::new().unwrap();
        let bundle = Bundle::new();
        let report = run_suggest(&bundle, td.path(), &SuggestOptions::default()).unwrap();
        assert!(report.is_empty());
    }

    #[test]
    fn unreadable_file_produces_warning_not_error() {
        let td = TempDir::new().unwrap();
        let mut bundle = Bundle::new();
        bundle.items.push(make_file_item("main.rs"));

        let report = run_suggest(&bundle, td.path(), &SuggestOptions::default()).unwrap();
        assert!(report.is_empty());
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("main.rs"));
    }

    #[test]
    fn apply_stale_removes_from_bundle() {
        use crate::paths::CtxforgeRoot;
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();

        let mut bundle = Bundle::new();
        bundle.items.push(make_docs_item(
            "dead-lib",
            Ecosystem::Rust,
            Some("Cargo.toml"),
        ));
        bundle.save(&root).unwrap();

        let stale = StaleDep {
            name: "dead-lib".into(),
            ecosystem: Ecosystem::Rust,
            reason: StaleReason::NotImported,
            suggested_command: "ctxforge docs rm dead-lib".into(),
            source: match &bundle.items[0].source {
                Source::Docs(d) => d.clone(),
                _ => unreachable!(),
            },
        };
        apply_suggestion(&Suggestion::Stale(&stale), &root).unwrap();

        let reloaded = Bundle::load_or_default(&root).unwrap();
        assert!(reloaded.items.is_empty());
    }
}
