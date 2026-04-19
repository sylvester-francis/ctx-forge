//! Cargo parser — reads Cargo.toml and Cargo.lock for a single crate.

use super::DetectedDep;
use crate::error::{CtxforgeError, Result};
use crate::source::Ecosystem;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
struct CargoToml {
    #[serde(default)]
    #[allow(dead_code)]
    package: Option<CargoPackage>,
    #[serde(default)]
    workspace: Option<CargoWorkspace>,
    #[serde(default)]
    dependencies: HashMap<String, CargoDepSpec>,
}

#[derive(Deserialize, Debug)]
struct CargoPackage {
    #[allow(dead_code)]
    name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CargoWorkspace {
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    dependencies: HashMap<String, CargoDepSpec>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum CargoDepSpec {
    Version(String),
    Detailed {
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        git: Option<String>,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        workspace: Option<bool>,
    },
}

#[derive(Deserialize, Debug)]
struct CargoLock {
    #[serde(default)]
    package: Vec<CargoLockPackage>,
}

#[derive(Deserialize, Debug)]
struct CargoLockPackage {
    name: String,
    version: String,
}

/// Parse a `Cargo.toml` plus its nearest `Cargo.lock` ancestor. Returns
/// direct deps only; git/path deps are skipped.
pub fn parse(manifest_path: &Path) -> Result<Vec<DetectedDep>> {
    let raw = std::fs::read_to_string(manifest_path)
        .map_err(|e| CtxforgeError::Msg(format!("read {}: {e}", manifest_path.display())))?;
    let parsed: CargoToml = toml::from_str(&raw)
        .map_err(|e| CtxforgeError::Msg(format!("parse {}: {e}", manifest_path.display())))?;

    let lock = find_cargo_lock(manifest_path)
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| toml::from_str::<CargoLock>(&s).ok());

    let workspace_deps = find_workspace_root_deps(manifest_path).unwrap_or_default();

    let mut deps = Vec::new();
    for (name, spec) in &parsed.dependencies {
        let version_spec = resolve_version_spec(spec, &workspace_deps, name);
        let Some(version) = resolved_version(&lock, name, version_spec.as_deref()) else {
            continue;
        };
        deps.push(DetectedDep {
            name: name.clone(),
            version,
            ecosystem: Ecosystem::Rust,
            manifest_path: manifest_path.to_path_buf(),
        });
    }

    Ok(deps)
}

pub fn workspace_members(manifest_path: &Path) -> Option<Vec<PathBuf>> {
    let raw = std::fs::read_to_string(manifest_path).ok()?;
    let parsed: CargoToml = toml::from_str(&raw).ok()?;
    let workspace = parsed.workspace?;
    let root_dir = manifest_path.parent()?;
    let mut out = Vec::new();
    for member_glob in &workspace.members {
        if !member_glob.contains('*') {
            let p = root_dir.join(member_glob).join("Cargo.toml");
            if p.is_file() {
                out.push(p);
            }
        } else {
            let prefix = member_glob.trim_end_matches("/*");
            let dir = root_dir.join(prefix);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let p = entry.path().join("Cargo.toml");
                    if p.is_file() {
                        out.push(p);
                    }
                }
            }
        }
    }
    Some(out)
}

fn resolve_version_spec(
    spec: &CargoDepSpec,
    workspace_deps: &HashMap<String, CargoDepSpec>,
    name: &str,
) -> Option<String> {
    match spec {
        CargoDepSpec::Version(v) => Some(v.clone()),
        CargoDepSpec::Detailed {
            version,
            git,
            path,
            workspace,
        } => {
            if git.is_some() || path.is_some() {
                return None;
            }
            if workspace == &Some(true) {
                if let Some(ws) = workspace_deps.get(name) {
                    return resolve_version_spec(ws, &HashMap::new(), name);
                }
            }
            version.clone()
        }
    }
}

fn find_cargo_lock(manifest_path: &Path) -> Option<PathBuf> {
    manifest_path.parent()?.ancestors().find_map(|dir| {
        let p = dir.join("Cargo.lock");
        p.is_file().then_some(p)
    })
}

fn find_workspace_root_deps(manifest_path: &Path) -> Option<HashMap<String, CargoDepSpec>> {
    manifest_path.parent()?.ancestors().find_map(|dir| {
        let p = dir.join("Cargo.toml");
        if !p.is_file() || p == manifest_path {
            return None;
        }
        let raw = std::fs::read_to_string(&p).ok()?;
        let parsed: CargoToml = toml::from_str(&raw).ok()?;
        parsed.workspace.map(|ws| ws.dependencies)
    })
}

fn resolved_version(
    lock: &Option<CargoLock>,
    name: &str,
    manifest_spec: Option<&str>,
) -> Option<String> {
    if let Some(lock) = lock {
        if let Some(pkg) = lock.package.iter().find(|p| p.name == name) {
            return Some(pkg.version.clone());
        }
    }
    manifest_spec.map(strip_version_prefix)
}

fn strip_version_prefix(spec: &str) -> String {
    spec.trim_start_matches(|c: char| "^~=><".contains(c))
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(td: &TempDir, rel: &str, content: &str) -> PathBuf {
        let p = td.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn parses_simple_cargo_toml_no_lock() {
        let td = TempDir::new().unwrap();
        let manifest = write(
            &td,
            "Cargo.toml",
            r#"
[package]
name = "app"

[dependencies]
axum = "0.7"
tokio = { version = "1.38", features = ["full"] }
"#,
        );
        let deps = parse(&manifest).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "axum" && d.version == "0.7"));
        assert!(
            deps.iter()
                .any(|d| d.name == "tokio" && d.version == "1.38")
        );
    }

    #[test]
    fn lock_file_wins_over_manifest_spec() {
        let td = TempDir::new().unwrap();
        let manifest = write(
            &td,
            "Cargo.toml",
            r#"
[package]
name = "app"

[dependencies]
axum = "^0.7"
"#,
        );
        write(
            &td,
            "Cargo.lock",
            r#"
[[package]]
name = "axum"
version = "0.7.5"
"#,
        );
        let deps = parse(&manifest).unwrap();
        assert_eq!(deps[0].version, "0.7.5");
    }

    #[test]
    fn skips_git_and_path_deps() {
        let td = TempDir::new().unwrap();
        let manifest = write(
            &td,
            "Cargo.toml",
            r#"
[package]
name = "app"

[dependencies]
axum = "0.7"
myfork = { git = "https://github.com/me/fork" }
local = { path = "../local" }
"#,
        );
        let deps = parse(&manifest).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "axum");
    }

    #[test]
    fn workspace_inheritance_resolves_version() {
        let td = TempDir::new().unwrap();
        write(
            &td,
            "Cargo.toml",
            r#"
[workspace]
members = ["crates/app"]

[workspace.dependencies]
axum = "0.7.5"
"#,
        );
        let member = write(
            &td,
            "crates/app/Cargo.toml",
            r#"
[package]
name = "app"

[dependencies]
axum = { workspace = true }
"#,
        );
        let deps = parse(&member).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "0.7.5");
    }

    #[test]
    fn workspace_members_enumerate_literal_paths() {
        let td = TempDir::new().unwrap();
        let root = write(
            &td,
            "Cargo.toml",
            r#"
[workspace]
members = ["crates/api", "crates/worker"]
"#,
        );
        write(
            &td,
            "crates/api/Cargo.toml",
            r#"
[package]
name = "api"
"#,
        );
        write(
            &td,
            "crates/worker/Cargo.toml",
            r#"
[package]
name = "worker"
"#,
        );
        let members = workspace_members(&root).unwrap();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn workspace_members_enumerate_glob() {
        let td = TempDir::new().unwrap();
        let root = write(
            &td,
            "Cargo.toml",
            r#"
[workspace]
members = ["crates/*"]
"#,
        );
        write(&td, "crates/api/Cargo.toml", "[package]\nname = \"api\"\n");
        write(
            &td,
            "crates/worker/Cargo.toml",
            "[package]\nname = \"worker\"\n",
        );
        let members = workspace_members(&root).unwrap();
        assert_eq!(members.len(), 2);
    }
}
