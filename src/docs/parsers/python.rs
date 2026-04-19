//! Python parser: pyproject.toml (PEP 621 or Poetry) plus uv.lock /
//! poetry.lock / requirements.txt for resolved versions.

use super::DetectedDep;
use crate::error::{CtxforgeError, Result};
use crate::source::Ecosystem;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct PyProject {
    #[serde(default)]
    project: Option<PyProjectMeta>,
    #[serde(default)]
    tool: Option<ToolSection>,
}

#[derive(Deserialize, Debug)]
struct PyProjectMeta {
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct ToolSection {
    #[serde(default)]
    poetry: Option<PoetryConfig>,
}

#[derive(Deserialize, Debug)]
struct PoetryConfig {
    #[serde(default)]
    dependencies: HashMap<String, toml::Value>,
}

#[derive(Deserialize, Debug)]
struct UvLock {
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Deserialize, Debug)]
struct LockPackage {
    name: String,
    version: String,
}

pub fn parse(manifest_path: &Path) -> Result<Vec<DetectedDep>> {
    let raw = std::fs::read_to_string(manifest_path)
        .map_err(|e| CtxforgeError::Msg(format!("read {}: {e}", manifest_path.display())))?;
    let parsed: PyProject = toml::from_str(&raw)
        .map_err(|e| CtxforgeError::Msg(format!("parse {}: {e}", manifest_path.display())))?;

    let mut specs: Vec<(String, Option<String>)> = Vec::new();
    if let Some(meta) = &parsed.project {
        for dep_spec in &meta.dependencies {
            if let Some((name, ver)) = parse_pep508(dep_spec) {
                specs.push((name, ver));
            }
        }
    }
    if let Some(tool) = &parsed.tool {
        if let Some(poetry) = &tool.poetry {
            for (name, spec) in &poetry.dependencies {
                if name == "python" {
                    continue;
                }
                let ver = match spec {
                    toml::Value::String(s) => Some(strip_version_prefix(s)),
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(strip_version_prefix),
                    _ => None,
                };
                specs.push((name.clone(), ver));
            }
        }
    }

    let dir = manifest_path.parent().unwrap_or(Path::new("."));
    let locked = load_lock_versions(dir);

    let mut deps = Vec::new();
    for (name, spec_ver) in specs {
        let normalized = name.to_lowercase().replace('_', "-");
        let version = locked
            .get(&normalized)
            .cloned()
            .or(spec_ver)
            .unwrap_or_else(|| "latest".into());
        deps.push(DetectedDep {
            name: normalized,
            version,
            ecosystem: Ecosystem::Python,
            manifest_path: manifest_path.to_path_buf(),
        });
    }

    Ok(deps)
}

fn parse_pep508(spec: &str) -> Option<(String, Option<String>)> {
    let before_semi = spec.split(';').next().unwrap_or(spec);
    let name_part = before_semi.split_whitespace().next()?;
    let end = name_part
        .find(|c: char| "[=><~!".contains(c))
        .unwrap_or(name_part.len());
    let name = name_part[..end].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let version = name_part[end..]
        .split(|c: char| "[],".contains(c))
        .find(|piece| piece.starts_with("==") || piece.starts_with(">="))
        .map(strip_version_prefix);
    Some((name, version))
}

fn load_lock_versions(dir: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for name in ["uv.lock", "poetry.lock"] {
        if let Ok(raw) = std::fs::read_to_string(dir.join(name)) {
            if let Ok(lock) = toml::from_str::<UvLock>(&raw) {
                for pkg in lock.package {
                    let key = pkg.name.to_lowercase().replace('_', "-");
                    out.insert(key, pkg.version);
                }
                return out;
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string(dir.join("requirements.txt")) {
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((name, ver)) = line.split_once("==") {
                let key = name.trim().to_lowercase().replace('_', "-");
                out.insert(key, ver.trim().to_string());
            }
        }
    }
    out
}

fn strip_version_prefix(spec: &str) -> String {
    spec.trim()
        .trim_start_matches(|c: char| "^~=><!".contains(c))
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(td: &TempDir, rel: &str, content: &str) -> PathBuf {
        let p = td.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn parses_pep621_project_dependencies() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "pyproject.toml",
            r#"
[project]
name = "app"
dependencies = [
  "fastapi>=0.115",
  "sqlalchemy[asyncio]==2.0.30",
  "httpx ; python_version >= '3.8'"
]
"#,
        );
        let deps = parse(&p).unwrap();
        assert!(deps.iter().any(|d| d.name == "fastapi"));
        assert!(
            deps.iter()
                .any(|d| d.name == "sqlalchemy" && d.version == "2.0.30")
        );
        assert!(deps.iter().any(|d| d.name == "httpx"));
    }

    #[test]
    fn uv_lock_wins_over_spec() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "pyproject.toml",
            r#"
[project]
name = "app"
dependencies = ["fastapi>=0.100"]
"#,
        );
        write(
            &td,
            "uv.lock",
            r#"
[[package]]
name = "fastapi"
version = "0.115.0"
"#,
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps[0].version, "0.115.0");
    }

    #[test]
    fn parses_poetry_dependencies() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "pyproject.toml",
            r#"
[tool.poetry]
name = "app"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.115"
sqlalchemy = { version = "^2.0" }
"#,
        );
        let deps = parse(&p).unwrap();
        assert!(!deps.iter().any(|d| d.name == "python"));
        assert!(deps.iter().any(|d| d.name == "fastapi"));
        assert!(deps.iter().any(|d| d.name == "sqlalchemy"));
    }

    #[test]
    fn parses_requirements_txt_fallback() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "pyproject.toml",
            r#"
[project]
name = "app"
dependencies = ["fastapi"]
"#,
        );
        write(&td, "requirements.txt", "fastapi==0.115.0\n# comment\n");
        let deps = parse(&p).unwrap();
        assert_eq!(deps[0].version, "0.115.0");
    }
}
