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

const BUILT_IN_BODIES: &[(&str, &str)] = &[
    ("bugfix", include_str!("../templates/starters/bugfix.md")),
    (
        "code-review",
        include_str!("../templates/starters/code-review.md"),
    ),
    ("explain", include_str!("../templates/starters/explain.md")),
    (
        "refactor",
        include_str!("../templates/starters/refactor.md"),
    ),
    ("migrate", include_str!("../templates/starters/migrate.md")),
];

/// Load the template body for a scenario. Handles built-in starters (baked
/// into the binary), project-local templates, and user-global templates.
/// Returns an error if the name is not recognised or the file cannot be
/// read.
pub fn load_body(root: &CtxforgeRoot, name: &str) -> Result<String, String> {
    if let Some((_, body)) = BUILT_IN_BODIES.iter().find(|(n, _)| *n == name) {
        return Ok((*body).to_string());
    }
    let path = crate::template::resolve_template_path(root, name)
        .map_err(|e| format!("resolve template: {e}"))?;
    std::fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))
}

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
