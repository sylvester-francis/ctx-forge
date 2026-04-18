//! npm / pnpm / yarn parser.
//!
//! Reads direct deps from package.json. Version precedence:
//!   package-lock.json > pnpm-lock.yaml > yarn.lock > manifest spec.

use super::DetectedDep;
use crate::error::{CtxforgeError, Result};
use crate::source::Ecosystem;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
struct PackageJson {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default)]
    #[serde(rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,
    #[serde(default)]
    workspaces: Option<WorkspacesField>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum WorkspacesField {
    List(Vec<String>),
    Config {
        #[serde(default)]
        packages: Vec<String>,
    },
}

#[derive(Deserialize, Debug)]
struct PackageLock {
    #[serde(default)]
    packages: HashMap<String, PackageLockEntry>,
}

#[derive(Deserialize, Debug)]
struct PackageLockEntry {
    #[serde(default)]
    version: Option<String>,
}

pub fn parse(manifest_path: &Path) -> Result<Vec<DetectedDep>> {
    let raw = std::fs::read_to_string(manifest_path)
        .map_err(|e| CtxforgeError::Msg(format!("read {}: {e}", manifest_path.display())))?;
    let parsed: PackageJson = serde_json::from_str(&raw)
        .map_err(|e| CtxforgeError::Msg(format!("parse {}: {e}", manifest_path.display())))?;

    let dir = manifest_path.parent().unwrap_or(Path::new("."));
    let lock_versions = load_lock_versions(dir);

    let mut deps = Vec::new();
    for (name, spec) in parsed
        .dependencies
        .iter()
        .chain(parsed.peer_dependencies.iter())
    {
        if is_local_ref(spec) {
            continue;
        }
        let version = lock_versions
            .get(name)
            .cloned()
            .unwrap_or_else(|| strip_version_prefix(spec));
        deps.push(DetectedDep {
            name: name.clone(),
            version,
            ecosystem: Ecosystem::Js,
            manifest_path: manifest_path.to_path_buf(),
        });
    }
    Ok(deps)
}

/// Enumerate workspace member manifests from a root package.json.
pub fn workspace_members(manifest_path: &Path) -> Option<Vec<PathBuf>> {
    let raw = std::fs::read_to_string(manifest_path).ok()?;
    let parsed: PackageJson = serde_json::from_str(&raw).ok()?;
    let patterns = match parsed.workspaces? {
        WorkspacesField::List(xs) => xs,
        WorkspacesField::Config { packages } => packages,
    };
    let root_dir = manifest_path.parent()?;
    let mut out = Vec::new();
    for pat in patterns {
        expand_workspace_pattern(root_dir, &pat, &mut out);
    }
    Some(out)
}

fn expand_workspace_pattern(root: &Path, pat: &str, out: &mut Vec<PathBuf>) {
    if !pat.contains('*') {
        let p = root.join(pat).join("package.json");
        if p.is_file() {
            out.push(p);
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
            out.push(p);
        }
    }
}

fn load_lock_versions(dir: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    // package-lock.json — packages["node_modules/<name>"].version
    if let Ok(raw) = std::fs::read_to_string(dir.join("package-lock.json")) {
        if let Ok(lock) = serde_json::from_str::<PackageLock>(&raw) {
            for (k, v) in lock.packages {
                if let Some(name) = k.strip_prefix("node_modules/") {
                    if let Some(ver) = v.version {
                        out.insert(name.to_string(), ver);
                    }
                }
            }
            return out;
        }
    }
    // pnpm-lock.yaml — importers["."].dependencies[<name>].version
    if let Ok(raw) = std::fs::read_to_string(dir.join("pnpm-lock.yaml")) {
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&raw) {
            if let Some(importer) = value.get("importers").and_then(|i| i.get(".")) {
                if let Some(deps) = importer.get("dependencies").and_then(|d| d.as_mapping()) {
                    for (k, v) in deps {
                        if let (Some(name), Some(ver)) =
                            (k.as_str(), v.get("version").and_then(|v| v.as_str()))
                        {
                            out.insert(name.to_string(), strip_version_prefix(ver));
                        }
                    }
                }
            }
            return out;
        }
    }
    // yarn.lock — skip for MVP (format is non-trivial).
    out
}

fn is_local_ref(spec: &str) -> bool {
    spec.starts_with("file:")
        || spec.starts_with("link:")
        || spec.starts_with("workspace:")
        || spec.starts_with("portal:")
        || spec.starts_with("git+")
        || spec.starts_with("http")
        || spec.starts_with("github:")
}

fn strip_version_prefix(spec: &str) -> String {
    let trimmed = spec.trim();
    let left = trimmed.split_whitespace().next().unwrap_or(trimmed);
    left.trim_start_matches(|c: char| "^~=><".contains(c))
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
    fn parses_manifest_without_lock() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "package.json",
            r#"{
              "name": "app",
              "dependencies": { "next": "^14.2.0", "zod": "~3.22.0" }
            }"#,
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "next" && d.version == "14.2.0"));
    }

    #[test]
    fn package_lock_wins() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "package.json",
            r#"{"name":"app","dependencies":{"next":"^14.0.0"}}"#,
        );
        write(
            &td,
            "package-lock.json",
            r#"{"packages":{"node_modules/next":{"version":"14.2.5"}}}"#,
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps[0].version, "14.2.5");
    }

    #[test]
    fn skips_workspace_and_local_refs() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "package.json",
            r#"{"name":"app","dependencies":{
              "next":"^14",
              "shared":"workspace:^1",
              "local":"file:./vendor/local"
            }}"#,
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "next");
    }

    #[test]
    fn workspace_members_list_form() {
        let td = TempDir::new().unwrap();
        let root = write(
            &td,
            "package.json",
            r#"{"name":"root","workspaces":["apps/*"]}"#,
        );
        write(&td, "apps/web/package.json", r#"{"name":"web"}"#);
        write(&td, "apps/docs/package.json", r#"{"name":"docs"}"#);
        let members = workspace_members(&root).unwrap();
        assert_eq!(members.len(), 2);
    }
}
