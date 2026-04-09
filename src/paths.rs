//! Resolution of `.ctxforge/` directory paths.
//!
//! ctxforge looks for `.ctxforge/` in the current directory then walks up to
//! find an existing one. If none exists, `find_or_create` creates one in the
//! current working directory.
#![allow(dead_code)]

use crate::error::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CtxforgeRoot {
    pub root: PathBuf,
}

impl CtxforgeRoot {
    /// Walks up from `start` looking for an existing `.ctxforge/` directory.
    /// Returns `None` if none is found.
    pub fn find(start: &Path) -> Option<CtxforgeRoot> {
        let mut current = Some(start);
        while let Some(dir) = current {
            let candidate = dir.join(".ctxforge");
            if candidate.is_dir() {
                return Some(CtxforgeRoot { root: candidate });
            }
            current = dir.parent();
        }
        None
    }

    /// Finds an existing root or creates one at `start/.ctxforge`.
    /// Ensures `profiles/` and `memory/` subdirs exist in either case
    /// so older `.ctxforge/` layouts are transparently upgraded.
    pub fn find_or_create(start: &Path) -> Result<CtxforgeRoot> {
        if let Some(existing) = Self::find(start) {
            std::fs::create_dir_all(existing.profiles_dir())?;
            std::fs::create_dir_all(existing.memory_dir())?;
            return Ok(existing);
        }
        let root = start.join(".ctxforge");
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(root.join("profiles"))?;
        std::fs::create_dir_all(root.join("memory"))?;
        Ok(CtxforgeRoot { root })
    }

    pub fn bundle_path(&self) -> PathBuf {
        self.root.join("bundle.json")
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    pub fn profile_path(&self, name: &str) -> PathBuf {
        self.profiles_dir().join(format!("{name}.json"))
    }

    pub fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }

    pub fn memory_index_path(&self) -> PathBuf {
        self.memory_dir().join("_index.jsonl")
    }

    /// Resolves the markdown file path for a tagged note. Untagged notes
    /// (`tag = None`) go to `decisions.md`.
    pub fn memory_tag_path(&self, tag: Option<&str>) -> PathBuf {
        let filename = match tag {
            Some(t) => format!("{t}.md"),
            None => "decisions.md".to_string(),
        };
        self.memory_dir().join(filename)
    }

    /// Resolves project root = parent of `.ctxforge`.
    pub fn project_root(&self) -> &Path {
        self.root.parent().unwrap_or(&self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_returns_none_when_no_ctxforge_dir() {
        let td = TempDir::new().unwrap();
        assert!(CtxforgeRoot::find(td.path()).is_none());
    }

    #[test]
    fn find_walks_up_to_parent() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".ctxforge")).unwrap();
        let nested = td.path().join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();

        let found = CtxforgeRoot::find(&nested).expect("should find");
        assert_eq!(found.root, td.path().join(".ctxforge"));
    }

    #[test]
    fn find_or_create_creates_profiles_dir() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert!(root.root.is_dir());
        assert!(root.profiles_dir().is_dir());
    }

    #[test]
    fn bundle_path_is_inside_root() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert_eq!(root.bundle_path(), root.root.join("bundle.json"));
    }

    #[test]
    fn memory_dir_is_inside_root() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert_eq!(root.memory_dir(), root.root.join("memory"));
    }

    #[test]
    fn memory_index_path_is_inside_memory_dir() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert_eq!(
            root.memory_index_path(),
            root.root.join("memory").join("_index.jsonl")
        );
    }

    #[test]
    fn memory_tag_path_untagged_goes_to_decisions() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert_eq!(
            root.memory_tag_path(None),
            root.root.join("memory").join("decisions.md")
        );
    }

    #[test]
    fn memory_tag_path_named_tag() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert_eq!(
            root.memory_tag_path(Some("auth")),
            root.root.join("memory").join("auth.md")
        );
    }

    #[test]
    fn find_or_create_creates_memory_dir() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert!(root.memory_dir().is_dir());
    }
}
