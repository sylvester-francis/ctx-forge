//! File discovery: expand glob patterns, apply exclude patterns,
//! and honor .gitignore.
//!
//! Returns a deterministic (sorted) list of paths relative to the current
//! working directory, suitable for inserting into a `Bundle` as `Item`s.

#![allow(dead_code)]

use crate::error::Result;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Expand a glob pattern relative to `cwd`, walking the filesystem
/// with .gitignore awareness. `excludes` are glob patterns whose matches
/// are filtered out after the initial walk.
pub fn expand(pattern: &str, cwd: &Path, excludes: &[String]) -> Result<Vec<PathBuf>> {
    let exclude_set = {
        let mut b = GlobSetBuilder::new();
        for e in excludes {
            b.add(Glob::new(e)?);
        }
        b.build()?
    };

    // Two-mode behavior:
    //   1. If `pattern` contains a glob metacharacter, expand via globset
    //      against a gitignore-aware walk.
    //   2. Otherwise treat `pattern` as a literal path. If it's a directory,
    //      walk it recursively.
    let is_glob = pattern.contains('*')
        || pattern.contains('?')
        || pattern.contains('[')
        || pattern.contains('{');

    let mut results: Vec<PathBuf> = Vec::new();

    // `hidden(true)` skips dotfiles (`.git/`, `.ctxforge/`, `.DS_Store`, etc.)
    // during directory traversal. Users can still add hidden files explicitly
    // by naming them on the command line — the literal-file branch below
    // bypasses the walker entirely.
    if is_glob {
        let matcher = Glob::new(pattern)?.compile_matcher();
        for entry in WalkBuilder::new(cwd).hidden(true).build().flatten() {
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            // Match against path relative to cwd.
            let rel = entry.path().strip_prefix(cwd).unwrap_or(entry.path());
            if matcher.is_match(rel) && !exclude_set.is_match(rel) {
                results.push(rel.to_path_buf());
            }
        }
    } else {
        let target = cwd.join(pattern);
        if target.is_file() {
            let rel = target.strip_prefix(cwd).unwrap_or(&target).to_path_buf();
            if !exclude_set.is_match(&rel) {
                results.push(rel);
            }
        } else if target.is_dir() {
            for entry in WalkBuilder::new(&target).hidden(true).build().flatten() {
                if !entry.file_type().is_some_and(|t| t.is_file()) {
                    continue;
                }
                let rel = entry
                    .path()
                    .strip_prefix(cwd)
                    .unwrap_or(entry.path())
                    .to_path_buf();
                if !exclude_set.is_match(&rel) {
                    results.push(rel);
                }
            }
        }
        // If neither file nor dir, silently return empty (caller prints warning).
    }

    results.sort();
    results.dedup();
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &str) {
        let full = root.join(rel);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, content).unwrap();
    }

    #[test]
    fn literal_file_path_returns_just_that_file() {
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "fn main() {}");
        write(td.path(), "src/lib.rs", "");

        let r = expand("src/main.rs", td.path(), &[]).unwrap();
        assert_eq!(r, vec![PathBuf::from("src/main.rs")]);
    }

    #[test]
    fn directory_walks_recursively() {
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");
        write(td.path(), "src/lib/inner.rs", "");
        write(td.path(), "README.md", "");

        let r = expand("src", td.path(), &[]).unwrap();
        assert_eq!(
            r,
            vec![
                PathBuf::from("src/lib/inner.rs"),
                PathBuf::from("src/main.rs"),
            ]
        );
    }

    #[test]
    fn glob_expands_extension() {
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");
        write(td.path(), "src/lib.rs", "");
        write(td.path(), "src/main.go", "");

        let r = expand("src/*.rs", td.path(), &[]).unwrap();
        assert_eq!(
            r,
            vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")]
        );
    }

    #[test]
    fn exclude_filters_results() {
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");
        write(td.path(), "src/main_test.rs", "");

        let r = expand("src/*.rs", td.path(), &["*_test.rs".to_string()]).unwrap();
        assert_eq!(r, vec![PathBuf::from("src/main.rs")]);
    }

    #[test]
    fn ignored_files_are_skipped() {
        // The `ignore` crate honors `.ignore` files unconditionally and
        // `.gitignore` files only inside a git repo. Tests use `.ignore` so
        // no `git init` is required.
        let td = TempDir::new().unwrap();
        write(td.path(), ".ignore", "target/\n");
        write(td.path(), "src/main.rs", "");
        write(td.path(), "target/debug/ctxforge", "");

        let r = expand("**/*", td.path(), &[]).unwrap();
        assert!(r.contains(&PathBuf::from("src/main.rs")));
        assert!(!r.iter().any(|p| p.starts_with("target")));
    }

    #[test]
    fn dotfiles_are_skipped_during_glob_walk() {
        // Regression test for v1.0.3: previously, `ctxforge add '**/*'` (or
        // `ctxforge add <dir>`) walked with `hidden(false)`, which dragged
        // `.git/HEAD`, `.git/config`, `.git/hooks/*.sample`, `.DS_Store`,
        // and other dotfiles into the bundle. Now dotfiles are skipped.
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");
        write(td.path(), "README.md", "");
        write(td.path(), ".git/HEAD", "ref: refs/heads/main");
        write(td.path(), ".git/config", "[core]");
        write(td.path(), ".git/hooks/pre-commit.sample", "#!/bin/sh");
        write(td.path(), ".DS_Store", "");
        write(td.path(), ".env.example", "KEY=");

        let r = expand("**/*", td.path(), &[]).unwrap();
        assert!(r.contains(&PathBuf::from("src/main.rs")));
        assert!(r.contains(&PathBuf::from("README.md")));
        assert!(
            !r.iter().any(|p| p.starts_with(".git")),
            "dotfile paths under .git/ must be skipped during walk, got: {r:?}"
        );
        assert!(
            !r.iter().any(|p| p == &PathBuf::from(".DS_Store")),
            ".DS_Store must be skipped"
        );
        assert!(
            !r.iter().any(|p| p == &PathBuf::from(".env.example")),
            ".env.example must be skipped"
        );
    }

    #[test]
    fn dotfiles_are_skipped_when_walking_a_directory() {
        // Same bug as above but via `ctxforge add <dir>` (literal path,
        // not glob).
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");
        write(td.path(), ".git/HEAD", "ref: refs/heads/main");

        let r = expand(".", td.path(), &[]).unwrap();
        assert!(r.contains(&PathBuf::from("src/main.rs")));
        assert!(!r.iter().any(|p| p.starts_with(".git")));
    }

    #[test]
    fn explicit_dotfile_literal_still_works() {
        // Users should still be able to add a dotfile by naming it
        // explicitly — the literal-file branch bypasses the walker.
        let td = TempDir::new().unwrap();
        write(td.path(), ".gitignore", "target/\n");
        write(td.path(), "src/main.rs", "");

        let r = expand(".gitignore", td.path(), &[]).unwrap();
        assert_eq!(r, vec![PathBuf::from(".gitignore")]);
    }
}
