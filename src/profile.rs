//! Profile save/load/list/rm. A profile is a named snapshot of a Bundle
//! persisted to `.ctxforge/profiles/<name>.json`. Profiles can be committed
//! to git for team sharing.

#![allow(dead_code)]

use crate::bundle::Bundle;
use crate::error::{CtxforgeError, Result};
use crate::paths::CtxforgeRoot;

pub fn save(root: &CtxforgeRoot, name: &str, bundle: &Bundle) -> Result<()> {
    validate_name(name)?;
    std::fs::create_dir_all(root.profiles_dir())?;
    let path = root.profile_path(name);
    let raw = serde_json::to_string_pretty(bundle)?;
    std::fs::write(path, raw)?;
    Ok(())
}

pub fn load(root: &CtxforgeRoot, name: &str) -> Result<Bundle> {
    use crate::bundle::migrate;
    let path = root.profile_path(name);
    if !path.exists() {
        return Err(CtxforgeError::ProfileNotFound(name.to_string()));
    }
    let raw = std::fs::read_to_string(&path)?;
    let (bundle, outcome) = migrate::migrate_json(&raw)?;
    if outcome == migrate::MigrationOutcome::MigratedFromV1 {
        eprintln!(
            "ctxforge: profile `{name}` migrated to v2 in memory (will rewrite on next save)"
        );
    }
    Ok(bundle)
}

pub fn list(root: &CtxforgeRoot) -> Result<Vec<String>> {
    let dir = root.profiles_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn remove(root: &CtxforgeRoot, name: &str) -> Result<()> {
    let path = root.profile_path(name);
    if !path.exists() {
        return Err(CtxforgeError::ProfileNotFound(name.to_string()));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err("profile name cannot be empty".into());
    }
    if name.contains(['/', '\\', '.', '\0']) {
        return Err("profile name cannot contain /, \\, ., or null bytes".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Item;
    use crate::source::{FileSource, Source};
    use tempfile::TempDir;

    fn sample_bundle() -> Bundle {
        let mut b = Bundle::new();
        b.add(Item {
            source: Source::File(FileSource {
                path: "src/main.rs".into(),
            }),
            label: None,
        });
        b
    }

    #[test]
    fn save_and_load_roundtrip() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        save(&root, "feature-a", &sample_bundle()).unwrap();

        let loaded = load(&root, "feature-a").unwrap();
        assert_eq!(loaded.items.len(), 1);
    }

    #[test]
    fn list_returns_profile_names_sorted() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        save(&root, "zeta", &sample_bundle()).unwrap();
        save(&root, "alpha", &sample_bundle()).unwrap();

        let list = list(&root).unwrap();
        assert_eq!(list, vec!["alpha", "zeta"]);
    }

    #[test]
    fn load_missing_profile_errors() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert!(load(&root, "nope").is_err());
    }

    #[test]
    fn remove_deletes_file() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        save(&root, "doomed", &sample_bundle()).unwrap();
        remove(&root, "doomed").unwrap();
        assert!(load(&root, "doomed").is_err());
    }

    #[test]
    fn invalid_names_rejected() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert!(save(&root, "", &sample_bundle()).is_err());
        assert!(save(&root, "has/slash", &sample_bundle()).is_err());
        assert!(save(&root, "has.dot", &sample_bundle()).is_err());
    }
}
