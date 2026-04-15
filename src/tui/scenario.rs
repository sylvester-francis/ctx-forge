//! Scenario = named template alias that drives the prompt wrapper.
//!
//! A scenario is any template the user can pick from: one of the built-in
//! starters (bugfix, code-review, explain, refactor, migrate) shipped in
//! the binary, or a file in the project-local or user-global templates
//! directories.

use crate::paths::{self, CtxforgeRoot};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub name: String,
    pub source: Source,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    BuiltIn,
    Project,
    Global,
}

/// Built-in scenario names shipped with the binary. Kept in sync with
/// `src/commands/template.rs::STARTERS` — same names, deliberately not
/// imported to avoid a cross-module dependency (TUI code shouldn't reach
/// into the CLI command implementations).
const BUILT_IN: &[&str] = &["bugfix", "code-review", "explain", "refactor", "migrate"];

/// Every scenario the user can select, sorted: built-ins first, then
/// project-local alphabetical, then global alphabetical (global entries
/// shadowed by a project template of the same name are dropped).
pub fn available(root: &CtxforgeRoot) -> Vec<Scenario> {
    let mut out: Vec<Scenario> = BUILT_IN
        .iter()
        .map(|n| Scenario {
            name: (*n).to_string(),
            source: Source::BuiltIn,
            path: None,
        })
        .collect();

    // Project-local templates.
    let project_dir = root.templates_dir();
    let mut project: Vec<Scenario> = Vec::new();
    if project_dir.is_dir() {
        for entry in std::fs::read_dir(&project_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            if let Some(stem) = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
            {
                if !out.iter().any(|s| s.name == stem) {
                    project.push(Scenario {
                        name: stem,
                        source: Source::Project,
                        path: Some(entry.path()),
                    });
                }
            }
        }
    }
    project.sort_by(|a, b| a.name.cmp(&b.name));
    out.extend(project);

    // User-global templates (skipped if overridden by project).
    if let Some(global_dir) = paths::global_templates_dir() {
        if global_dir.is_dir() {
            let mut global: Vec<Scenario> = Vec::new();
            for entry in std::fs::read_dir(&global_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                if let Some(stem) = entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(String::from)
                {
                    if !out.iter().any(|s| s.name == stem) {
                        global.push(Scenario {
                            name: stem,
                            source: Source::Global,
                            path: Some(entry.path()),
                        });
                    }
                }
            }
            global.sort_by(|a, b| a.name.cmp(&b.name));
            out.extend(global);
        }
    }

    out
}
