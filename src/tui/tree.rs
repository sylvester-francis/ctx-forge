//! Build a flat, indented file tree from the project directory.
//!
//! Walks the project using the `ignore` crate (honors .gitignore) and
//! produces a sorted, indented list suitable for rendering in the TUI's
//! left panel. Directories appear as non-selectable separators; files
//! are selectable entries.

use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single entry in the flattened tree.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// Display name (just the file/dir name, not the full path).
    pub name: String,
    /// Relative path from project root.
    pub rel_path: PathBuf,
    /// Nesting depth (0 = root level).
    pub depth: usize,
    /// True if this entry is a directory header.
    pub is_dir: bool,
}

/// Build the tree entries from a project root directory.
pub fn build(project_root: &Path) -> Vec<TreeEntry> {
    // Collect all file paths, then build a tree from them.
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in WalkBuilder::new(project_root)
        .hidden(true)
        .build()
        .flatten()
    {
        if entry.file_type().is_some_and(|t| t.is_file()) {
            if let Ok(rel) = entry.path().strip_prefix(project_root) {
                files.push(rel.to_path_buf());
            }
        }
    }
    files.sort();

    // Build nested map: dir -> [entries].
    // We use BTreeMap so dirs are sorted.
    let mut entries: Vec<TreeEntry> = Vec::new();
    let mut seen_dirs: BTreeMap<PathBuf, bool> = BTreeMap::new();

    for file in &files {
        // Ensure all parent directories are emitted.
        let mut ancestors: Vec<PathBuf> = Vec::new();
        let mut current = file.parent();
        while let Some(p) = current {
            if p.as_os_str().is_empty() {
                break;
            }
            ancestors.push(p.to_path_buf());
            current = p.parent();
        }
        ancestors.reverse();

        for dir in &ancestors {
            if !seen_dirs.contains_key(dir) {
                seen_dirs.insert(dir.clone(), true);
                let depth = dir.components().count() - 1;
                let name = dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                entries.push(TreeEntry {
                    name: format!("{name}/"),
                    rel_path: dir.clone(),
                    depth,
                    is_dir: true,
                });
            }
        }

        // Emit the file.
        let depth = file.components().count() - 1;
        let name = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        entries.push(TreeEntry {
            name,
            rel_path: file.clone(),
            depth,
            is_dir: false,
        });
    }

    entries
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
    fn build_produces_sorted_entries_with_dirs() {
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");
        write(td.path(), "src/lib.rs", "");
        write(td.path(), "README.md", "");

        let entries = build(td.path());

        // Should have: README.md, src/, src/lib.rs, src/main.rs
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"README.md"));
        assert!(names.contains(&"src/"));
        assert!(names.contains(&"main.rs"));
        assert!(names.contains(&"lib.rs"));
    }

    #[test]
    fn dirs_are_non_selectable() {
        let td = TempDir::new().unwrap();
        write(td.path(), "src/main.rs", "");

        let entries = build(td.path());
        let src_dir = entries.iter().find(|e| e.name == "src/").unwrap();
        assert!(src_dir.is_dir);

        let main_file = entries.iter().find(|e| e.name == "main.rs").unwrap();
        assert!(!main_file.is_dir);
    }

    #[test]
    fn depth_reflects_nesting() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a/b/c.rs", "");

        let entries = build(td.path());
        let c_file = entries.iter().find(|e| e.name == "c.rs").unwrap();
        assert_eq!(c_file.depth, 2);
    }
}
