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
    /// Whether this directory is expanded (always false / ignored for files).
    pub expanded: bool,
}

/// Build the tree entries from a project root directory.
pub fn build(project_root: &Path) -> Vec<TreeEntry> {
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

    let mut entries: Vec<TreeEntry> = Vec::new();
    let mut seen_dirs: BTreeMap<PathBuf, bool> = BTreeMap::new();

    for file in &files {
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
                    // Top-level directories start expanded; deeper ones collapsed.
                    expanded: depth == 0,
                });
            }
        }

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
            expanded: false,
        });
    }

    entries
}

/// Mark every directory entry as expanded. Files are untouched.
pub fn expand_all(entries: &mut [TreeEntry]) {
    for e in entries.iter_mut() {
        if e.is_dir {
            e.expanded = true;
        }
    }
}

/// Mark every directory entry as collapsed. Files are untouched.
pub fn collapse_all(entries: &mut [TreeEntry]) {
    for e in entries.iter_mut() {
        if e.is_dir {
            e.expanded = false;
        }
    }
}

/// Returns indices of entries visible given current expand/collapse state.
/// A file or dir is visible if all its ancestor directories are expanded.
pub fn visible_indices(entries: &[TreeEntry]) -> Vec<usize> {
    let mut result = Vec::new();
    let mut collapsed_depth: Option<usize> = None;

    for (i, entry) in entries.iter().enumerate() {
        if let Some(cd) = collapsed_depth {
            if entry.depth > cd {
                continue;
            }
            collapsed_depth = None;
        }

        result.push(i);

        if entry.is_dir && !entry.expanded {
            collapsed_depth = Some(entry.depth);
        }
    }

    result
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

    #[test]
    fn visible_indices_hides_collapsed_children() {
        let entries = vec![
            TreeEntry {
                name: "src/".into(),
                rel_path: "src".into(),
                depth: 0,
                is_dir: true,
                expanded: false,
            },
            TreeEntry {
                name: "main.rs".into(),
                rel_path: "src/main.rs".into(),
                depth: 1,
                is_dir: false,
                expanded: false,
            },
            TreeEntry {
                name: "lib.rs".into(),
                rel_path: "src/lib.rs".into(),
                depth: 1,
                is_dir: false,
                expanded: false,
            },
            TreeEntry {
                name: "README.md".into(),
                rel_path: "README.md".into(),
                depth: 0,
                is_dir: false,
                expanded: false,
            },
        ];
        let vis = visible_indices(&entries);
        assert_eq!(vis, vec![0, 3]);
    }

    #[test]
    fn visible_indices_shows_expanded_children() {
        let entries = vec![
            TreeEntry {
                name: "src/".into(),
                rel_path: "src".into(),
                depth: 0,
                is_dir: true,
                expanded: true,
            },
            TreeEntry {
                name: "main.rs".into(),
                rel_path: "src/main.rs".into(),
                depth: 1,
                is_dir: false,
                expanded: false,
            },
            TreeEntry {
                name: "README.md".into(),
                rel_path: "README.md".into(),
                depth: 0,
                is_dir: false,
                expanded: false,
            },
        ];
        let vis = visible_indices(&entries);
        assert_eq!(vis, vec![0, 1, 2]);
    }

    #[test]
    fn expand_all_marks_every_directory_expanded() {
        let mut entries = vec![
            TreeEntry {
                name: "src/".into(),
                rel_path: "src".into(),
                depth: 0,
                is_dir: true,
                expanded: true,
            },
            TreeEntry {
                name: "hub/".into(),
                rel_path: "src/hub".into(),
                depth: 1,
                is_dir: true,
                expanded: false,
            },
            TreeEntry {
                name: "deep/".into(),
                rel_path: "src/hub/deep".into(),
                depth: 2,
                is_dir: true,
                expanded: false,
            },
            TreeEntry {
                name: "main.rs".into(),
                rel_path: "src/main.rs".into(),
                depth: 1,
                is_dir: false,
                expanded: false,
            },
        ];
        expand_all(&mut entries);
        assert!(entries[0].expanded);
        assert!(entries[1].expanded);
        assert!(entries[2].expanded);
        assert!(!entries[3].expanded);
    }

    #[test]
    fn collapse_all_marks_every_directory_collapsed() {
        let mut entries = vec![
            TreeEntry {
                name: "src/".into(),
                rel_path: "src".into(),
                depth: 0,
                is_dir: true,
                expanded: true,
            },
            TreeEntry {
                name: "hub/".into(),
                rel_path: "src/hub".into(),
                depth: 1,
                is_dir: true,
                expanded: true,
            },
            TreeEntry {
                name: "main.rs".into(),
                rel_path: "src/main.rs".into(),
                depth: 1,
                is_dir: false,
                expanded: false,
            },
        ];
        collapse_all(&mut entries);
        assert!(!entries[0].expanded);
        assert!(!entries[1].expanded);
        assert!(!entries[2].expanded);
    }

    #[test]
    fn visible_indices_nested_collapse() {
        let entries = vec![
            TreeEntry {
                name: "src/".into(),
                rel_path: "src".into(),
                depth: 0,
                is_dir: true,
                expanded: true,
            },
            TreeEntry {
                name: "hub/".into(),
                rel_path: "src/hub".into(),
                depth: 1,
                is_dir: true,
                expanded: false,
            },
            TreeEntry {
                name: "server.go".into(),
                rel_path: "src/hub/server.go".into(),
                depth: 2,
                is_dir: false,
                expanded: false,
            },
            TreeEntry {
                name: "main.rs".into(),
                rel_path: "src/main.rs".into(),
                depth: 1,
                is_dir: false,
                expanded: false,
            },
        ];
        let vis = visible_indices(&entries);
        assert_eq!(vis, vec![0, 1, 3]);
    }
}
