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
    pub fn find_or_create(start: &Path) -> Result<CtxforgeRoot> {
        if let Some(existing) = Self::find(start) {
            return Ok(existing);
        }
        let root = start.join(".ctxforge");
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(root.join("profiles"))?;
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
}
