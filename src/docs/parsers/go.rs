//! Go parser — reads go.mod for direct deps (version pinned inline).
//! Transitive deps are marked `// indirect` and filtered out.

use super::DetectedDep;
use crate::error::{CtxforgeError, Result};
use crate::source::Ecosystem;
use std::path::{Path, PathBuf};

pub fn parse(manifest_path: &Path) -> Result<Vec<DetectedDep>> {
    let raw = std::fs::read_to_string(manifest_path)
        .map_err(|e| CtxforgeError::Msg(format!("read {}: {e}", manifest_path.display())))?;

    let mut deps = Vec::new();
    let mut in_require_block = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("require ") {
            if rest.starts_with('(') {
                in_require_block = true;
                continue;
            }
            if let Some(dep) = parse_require_line(rest, manifest_path) {
                deps.push(dep);
            }
            continue;
        }

        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }

        if in_require_block {
            if trimmed == ")" {
                in_require_block = false;
                continue;
            }
            if let Some(dep) = parse_require_line(trimmed, manifest_path) {
                deps.push(dep);
            }
        }
    }

    Ok(deps)
}

fn parse_require_line(line: &str, manifest_path: &Path) -> Option<DetectedDep> {
    let without_comment = line.split("//").next()?.trim();
    let is_indirect = line.contains("// indirect");
    if is_indirect {
        return None;
    }
    let mut parts = without_comment.split_whitespace();
    let name = parts.next()?.to_string();
    let version = parts.next()?.to_string();
    if name.is_empty() || version.is_empty() {
        return None;
    }
    Some(DetectedDep {
        name,
        version,
        ecosystem: Ecosystem::Go,
        manifest_path: manifest_path.to_path_buf(),
    })
}

/// Enumerate Go workspace members from a `go.work` file.
pub fn workspace_members(go_work_path: &Path) -> Option<Vec<PathBuf>> {
    let raw = std::fs::read_to_string(go_work_path).ok()?;
    let root = go_work_path.parent()?;
    let mut out = Vec::new();
    let mut in_use_block = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("use ") {
            if rest.starts_with('(') {
                in_use_block = true;
                continue;
            }
            out.push(root.join(rest.trim_matches('"')).join("go.mod"));
            continue;
        }
        if trimmed.starts_with("use (") {
            in_use_block = true;
            continue;
        }
        if in_use_block {
            if trimmed == ")" {
                in_use_block = false;
                continue;
            }
            out.push(root.join(trimmed.trim_matches('"')).join("go.mod"));
        }
    }
    Some(out)
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
    fn parses_single_line_require() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "go.mod",
            "module example.com/app\n\ngo 1.22\n\nrequire github.com/gin-gonic/gin v1.10.0\n",
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "github.com/gin-gonic/gin");
        assert_eq!(deps[0].version, "v1.10.0");
    }

    #[test]
    fn parses_require_block_and_filters_indirect() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "go.mod",
            r#"module example.com/app

go 1.22

require (
    github.com/gin-gonic/gin v1.10.0
    github.com/jackc/pgx/v5 v5.5.0
    github.com/bytedance/sonic v1.11.6 // indirect
)
"#,
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(!deps.iter().any(|d| d.name.contains("sonic")));
    }

    #[test]
    fn handles_pseudo_versions() {
        let td = TempDir::new().unwrap();
        let p = write(
            &td,
            "go.mod",
            "module example.com/app\n\nrequire example.com/foo v0.0.0-20230601125847-abc123def456\n",
        );
        let deps = parse(&p).unwrap();
        assert_eq!(deps[0].version, "v0.0.0-20230601125847-abc123def456");
    }

    #[test]
    fn parses_go_work_use_block() {
        let td = TempDir::new().unwrap();
        let work = write(
            &td,
            "go.work",
            "go 1.22\n\nuse (\n    ./services/api\n    ./services/worker\n)\n",
        );
        write(&td, "services/api/go.mod", "module api\n");
        write(&td, "services/worker/go.mod", "module worker\n");
        let members = workspace_members(&work).unwrap();
        assert_eq!(members.len(), 2);
    }
}
