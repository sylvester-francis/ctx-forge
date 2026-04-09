//! Git integration for `ctxforge add --diff <branch>`.
//!
//! Returns the list of files changed between the current HEAD and the named
//! branch. Used to populate bundles from PR-scoped changes.

#![allow(dead_code)]

use crate::error::{CtxforgeError, Result};
use git2::{DiffOptions, Repository};
use std::path::{Path, PathBuf};

/// Returns paths changed between `branch` and HEAD in the repo at `cwd`.
/// Paths are relative to the repo root.
pub fn changed_files(cwd: &Path, branch: &str) -> Result<Vec<PathBuf>> {
    let repo = Repository::discover(cwd)?;

    let head_tree = repo.head()?.peel_to_tree()?;

    let branch_ref = repo
        .find_branch(branch, git2::BranchType::Local)
        .or_else(|_| repo.find_branch(branch, git2::BranchType::Remote))
        .map_err(|e| CtxforgeError::Msg(format!("branch `{branch}` not found: {e}")))?;
    let branch_tree = branch_ref.get().peel_to_tree()?;

    let mut opts = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(Some(&branch_tree), Some(&head_tree), Some(&mut opts))?;

    let mut files: Vec<PathBuf> = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(p) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                files.push(p.to_path_buf());
            }
            true
        },
        None,
        None,
        None,
    )?;

    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn run(dir: &Path, cmd: &[&str]) {
        let status = Command::new(cmd[0])
            .args(&cmd[1..])
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "command failed: {cmd:?}");
    }

    #[test]
    fn changed_files_between_branches() {
        let td = TempDir::new().unwrap();
        let p = td.path();

        // Init a fresh repo and commit a file on main.
        run(p, &["git", "init", "-q", "-b", "main"]);
        run(p, &["git", "config", "user.email", "t@t"]);
        run(p, &["git", "config", "user.name", "t"]);
        std::fs::write(p.join("a.rs"), "fn a() {}\n").unwrap();
        run(p, &["git", "add", "a.rs"]);
        run(p, &["git", "commit", "-q", "-m", "initial"]);

        // Create a feature branch and add a file.
        run(p, &["git", "checkout", "-q", "-b", "feature"]);
        std::fs::write(p.join("b.rs"), "fn b() {}\n").unwrap();
        run(p, &["git", "add", "b.rs"]);
        run(p, &["git", "commit", "-q", "-m", "add b"]);

        // Diff feature vs main: should show b.rs (changed from main to HEAD=feature).
        let changed = changed_files(p, "main").unwrap();
        assert_eq!(changed, vec![PathBuf::from("b.rs")]);
    }

    #[test]
    fn unknown_branch_errors() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        run(p, &["git", "init", "-q", "-b", "main"]);
        run(p, &["git", "config", "user.email", "t@t"]);
        run(p, &["git", "config", "user.name", "t"]);
        std::fs::write(p.join("a.rs"), "").unwrap();
        run(p, &["git", "add", "a.rs"]);
        run(p, &["git", "commit", "-q", "-m", "init"]);

        assert!(changed_files(p, "no-such-branch").is_err());
    }
}
