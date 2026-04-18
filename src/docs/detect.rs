//! Three detection modes: scoped (default, follows bundle items), all
//! (full project scan), explicit (user-provided path).

use crate::bundle::Bundle;
use crate::docs::parsers::cargo;
use crate::source::Ecosystem;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedManifest {
    pub path: PathBuf,
    pub ecosystem: Ecosystem,
}

const MANIFESTS: &[(&str, Ecosystem)] = &[
    ("Cargo.toml", Ecosystem::Rust),
    ("package.json", Ecosystem::Js),
    ("pyproject.toml", Ecosystem::Python),
    ("go.mod", Ecosystem::Go),
];

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".hg",
    ".svn",
    "dist",
    "build",
    "out",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".worktrees",
    ".claude",
    "vendor",
];

/// Walk upward from `start` until we find a recognized manifest.
pub fn manifest_for(start: &Path) -> Option<DetectedManifest> {
    start.ancestors().find_map(|dir| {
        MANIFESTS.iter().find_map(|(name, eco)| {
            let p = dir.join(name);
            p.is_file().then_some(DetectedManifest {
                path: p,
                ecosystem: *eco,
            })
        })
    })
}

/// Scoped detection — walk up from each bundle file to its nearest manifest.
pub fn scoped(bundle: &Bundle, project_root: &Path) -> Vec<DetectedManifest> {
    let mut seen: Vec<DetectedManifest> = Vec::new();
    for item in &bundle.items {
        let Some(rel) = item.source.display_path() else {
            continue;
        };
        let full = project_root.join(rel);
        if let Some(m) = manifest_for(&full) {
            if !seen.contains(&m) {
                seen.push(m);
            }
        }
    }
    seen
}

/// Full-scan mode — enumerate workspace members if the root is workspace-aware,
/// else shallow walk (cwd + 2 levels) for manifest files.
pub fn full_scan(project_root: &Path) -> Vec<DetectedManifest> {
    let mut workspace_members: Vec<DetectedManifest> = Vec::new();

    // Cargo workspace.
    let root_cargo = project_root.join("Cargo.toml");
    if root_cargo.is_file() {
        if let Some(members) = cargo::workspace_members(&root_cargo) {
            for p in members {
                workspace_members.push(DetectedManifest {
                    path: p,
                    ecosystem: Ecosystem::Rust,
                });
            }
        }
    }

    // npm / yarn / pnpm workspace via package.json `workspaces` field.
    let root_pkg = project_root.join("package.json");
    if root_pkg.is_file() {
        if let Some(members) = crate::docs::parsers::npm::workspace_members(&root_pkg) {
            for p in members {
                workspace_members.push(DetectedManifest {
                    path: p,
                    ecosystem: Ecosystem::Js,
                });
            }
        }
    }

    // Standalone pnpm-workspace.yaml.
    let pnpm_ws = project_root.join("pnpm-workspace.yaml");
    if pnpm_ws.is_file() {
        if let Ok(raw) = std::fs::read_to_string(&pnpm_ws) {
            if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&raw) {
                if let Some(patterns) = value.get("packages").and_then(|v| v.as_sequence()) {
                    for p in patterns {
                        if let Some(pat) = p.as_str() {
                            expand_pnpm_pattern(project_root, pat, &mut workspace_members);
                        }
                    }
                }
            }
        }
    }

    // go.work.
    let go_work = project_root.join("go.work");
    if go_work.is_file() {
        if let Some(members) = crate::docs::parsers::go::workspace_members(&go_work) {
            for p in members {
                workspace_members.push(DetectedManifest {
                    path: p,
                    ecosystem: Ecosystem::Go,
                });
            }
        }
    }

    if !workspace_members.is_empty() {
        return dedup_manifests(workspace_members);
    }

    // No workspace — shallow walk cwd + 2 levels.
    let mut out = Vec::new();
    scan_directory(project_root, &mut out);
    walk_subdirs(project_root, 2, &mut out);
    dedup_manifests(out)
}

fn expand_pnpm_pattern(root: &Path, pat: &str, out: &mut Vec<DetectedManifest>) {
    if !pat.contains('*') {
        let p = root.join(pat).join("package.json");
        if p.is_file() {
            out.push(DetectedManifest {
                path: p,
                ecosystem: Ecosystem::Js,
            });
        }
        return;
    }
    let prefix = pat.trim_end_matches("/*");
    let dir = root.join(prefix);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path().join("package.json");
        if p.is_file() {
            out.push(DetectedManifest {
                path: p,
                ecosystem: Ecosystem::Js,
            });
        }
    }
}

fn dedup_manifests(mut xs: Vec<DetectedManifest>) -> Vec<DetectedManifest> {
    xs.sort_by(|a, b| a.path.cmp(&b.path));
    xs.dedup();
    xs
}

fn walk_subdirs(dir: &Path, max_depth: usize, out: &mut Vec<DetectedManifest>) {
    if max_depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if SKIP_DIRS.iter().any(|s| name == std::ffi::OsStr::new(s)) {
            continue;
        }
        scan_directory(&path, out);
        walk_subdirs(&path, max_depth - 1, out);
    }
}

fn scan_directory(dir: &Path, out: &mut Vec<DetectedManifest>) {
    for (name, eco) in MANIFESTS {
        let p = dir.join(name);
        if p.is_file() {
            out.push(DetectedManifest {
                path: p,
                ecosystem: *eco,
            });
        }
    }
}

/// Explicit mode — detect the single manifest at or nearest to `path`.
pub fn explicit(path: &Path) -> Option<DetectedManifest> {
    manifest_for(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn touch(td: &TempDir, rel: &str) -> PathBuf {
        let p = td.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "").unwrap();
        p
    }

    #[test]
    fn walk_up_finds_nearest_manifest() {
        let td = TempDir::new().unwrap();
        touch(&td, "Cargo.toml");
        let file = touch(&td, "src/main.rs");
        let m = manifest_for(&file).unwrap();
        assert!(m.path.ends_with("Cargo.toml"));
        assert_eq!(m.ecosystem, Ecosystem::Rust);
    }

    #[test]
    fn walk_up_stops_at_first_manifest() {
        let td = TempDir::new().unwrap();
        touch(&td, "Cargo.toml");
        let inner = touch(&td, "crates/api/Cargo.toml");
        let file = touch(&td, "crates/api/src/main.rs");
        let m = manifest_for(&file).unwrap();
        assert_eq!(m.path, inner);
    }

    #[test]
    fn full_scan_skips_node_modules() {
        let td = TempDir::new().unwrap();
        touch(&td, "package.json");
        touch(&td, "node_modules/foo/package.json");
        let manifests = full_scan(td.path());
        assert_eq!(manifests.len(), 1);
    }

    #[test]
    fn full_scan_finds_polyglot_flat_layout() {
        let td = TempDir::new().unwrap();
        touch(&td, "Cargo.toml");
        touch(&td, "package.json");
        touch(&td, "pyproject.toml");
        touch(&td, "go.mod");
        let manifests = full_scan(td.path());
        assert_eq!(manifests.len(), 4);
    }

    #[test]
    fn full_scan_finds_nested_packages_one_level_deep() {
        let td = TempDir::new().unwrap();
        touch(&td, "services/api/go.mod");
        touch(&td, "apps/web/package.json");
        let manifests = full_scan(td.path());
        assert_eq!(manifests.len(), 2);
    }
}
