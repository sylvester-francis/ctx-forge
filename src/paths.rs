//! Resolution of `.ctxforge/` directory paths.
//!
//! ctxforge is always rooted at the current working directory: it uses
//! `./.ctxforge/` if one exists, otherwise it creates one there. It does
//! NOT walk up to find an ancestor `.ctxforge/` — that behavior was removed
//! in v1.0.1 because a stray `$HOME/.ctxforge/` would silently capture
//! every invocation run from anywhere under the home directory.
#![allow(dead_code)]

use crate::error::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CtxforgeRoot {
    pub root: PathBuf,
}

impl CtxforgeRoot {
    /// Returns `Some` if `start/.ctxforge/` already exists, else `None`.
    /// Does NOT walk up — strictly checks `start` itself.
    pub fn find(start: &Path) -> Option<CtxforgeRoot> {
        let candidate = start.join(".ctxforge");
        if candidate.is_dir() {
            Some(CtxforgeRoot { root: candidate })
        } else {
            None
        }
    }

    /// Returns the root at `start/.ctxforge`, creating it if it does not
    /// already exist. Ensures `profiles/` and `memory/` subdirs exist in
    /// either case so older `.ctxforge/` layouts are transparently upgraded.
    /// Never walks up from `start`.
    pub fn find_or_create(start: &Path) -> Result<CtxforgeRoot> {
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

    pub fn templates_dir(&self) -> PathBuf {
        self.root.join("templates")
    }

    pub fn template_path(&self, name: &str) -> PathBuf {
        self.templates_dir().join(format!("{name}.md"))
    }
}

/// Returns the user-global templates directory: `~/.config/ctxforge/templates`.
/// On systems where the home directory cannot be determined, returns `None`.
pub fn global_templates_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    Some(home.join(".config").join("ctxforge").join("templates"))
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
    fn find_returns_some_when_ctxforge_dir_exists_in_start() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".ctxforge")).unwrap();
        let found = CtxforgeRoot::find(td.path()).expect("should find");
        assert_eq!(found.root, td.path().join(".ctxforge"));
    }

    #[test]
    fn find_does_not_walk_up_to_ancestor() {
        // A .ctxforge/ in an ancestor directory must NOT be found when
        // starting from a nested subdirectory. Regression test for v1.0.1:
        // previously, a stray ~/.ctxforge/ captured every invocation run
        // from anywhere under $HOME.
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".ctxforge")).unwrap();
        let nested = td.path().join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();

        assert!(
            CtxforgeRoot::find(&nested).is_none(),
            "find must not walk up past its start directory"
        );
    }

    #[test]
    fn find_or_create_does_not_walk_up_to_ancestor() {
        // Ensure find_or_create creates a NEW .ctxforge at the nested
        // location instead of reusing the ancestor's.
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".ctxforge")).unwrap();
        let nested = td.path().join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();

        let root = CtxforgeRoot::find_or_create(&nested).unwrap();
        assert_eq!(root.root, nested.join(".ctxforge"));
        assert!(root.root.is_dir());
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
