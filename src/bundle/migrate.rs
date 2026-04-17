//! v1 → v2 migration for anything that serialises a Bundle: the live
//! bundle (`.ctxforge/bundle.json`) and saved profiles
//! (`.ctxforge/profiles/<name>.json`).
//!
//! Migration is in-memory only. Callers decide when (and whether) to
//! persist the migrated form. Persisting during `load` is deliberately
//! avoided — it makes the loader fail on read-only mounts and violates the
//! spec's "rewritten on next save" language.

use crate::bundle::persist::{BundleV1, ItemKindV1, ItemV1};
use crate::bundle::{Bundle, Item};
use crate::error::{CtxforgeError, Result};
use crate::source::{FileSource, FuncSource, RangeSource, Source, TypeSource};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum MigrationOutcome {
    AlreadyV2,
    MigratedFromV1,
}

/// Parse a JSON string containing either a v1 or v2 Bundle (or a profile
/// sharing that schema) and return the v2 form. Returns an error for
/// future schema versions.
pub fn migrate_json(raw: &str) -> Result<(Bundle, MigrationOutcome)> {
    let peek: Value = serde_json::from_str(raw)?;
    let version = peek
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    match version {
        2 => {
            let b: Bundle = serde_json::from_str(raw)?;
            Ok((b, MigrationOutcome::AlreadyV2))
        }
        0 | 1 => {
            let v1: BundleV1 = serde_json::from_str(raw)?;
            let items = v1
                .items
                .iter()
                .map(item_from_v1)
                .collect::<Result<Vec<_>>>()?;
            Ok((
                Bundle {
                    version: 2,
                    items,
                    model: v1.model,
                    task_text: v1.task_text,
                    scenario: v1.scenario,
                },
                MigrationOutcome::MigratedFromV1,
            ))
        }
        other => Err(CtxforgeError::Msg(format!(
            "bundle version {other} is newer than this ctxforge — please upgrade"
        ))),
    }
}

fn item_from_v1(i: &ItemV1) -> Result<Item> {
    let source = match &i.kind {
        ItemKindV1::File => Source::File(FileSource {
            path: i.path.clone(),
        }),
        ItemKindV1::Range { start, end } => {
            Source::Range(RangeSource::new(i.path.clone(), *start, *end)?)
        }
        ItemKindV1::Function { name } => Source::Func(FuncSource {
            path: i.path.clone(),
            name: name.clone(),
        }),
        ItemKindV1::Type { name } => Source::Type(TypeSource {
            path: i.path.clone(),
            name: name.clone(),
        }),
    };
    Ok(Item {
        source,
        label: i.label.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_v2_passes_through() {
        let raw = r#"{"version":2,"items":[]}"#;
        let (b, o) = migrate_json(raw).unwrap();
        assert_eq!(o, MigrationOutcome::AlreadyV2);
        assert_eq!(b.version, 2);
    }

    #[test]
    fn v1_migrates_all_kinds() {
        let raw = r#"{
          "version": 1,
          "items": [
            {"path": "a.rs", "kind": {"kind": "file"}},
            {"path": "b.rs", "kind": {"kind": "range", "start": 1, "end": 5}},
            {"path": "c.rs", "kind": {"kind": "function", "name": "foo"}},
            {"path": "d.rs", "kind": {"kind": "type", "name": "Bar"}}
          ]
        }"#;
        let (b, o) = migrate_json(raw).unwrap();
        assert_eq!(o, MigrationOutcome::MigratedFromV1);
        assert_eq!(b.version, 2);
        assert_eq!(b.items.len(), 4);
        assert!(matches!(b.items[0].source, Source::File(_)));
        assert!(matches!(b.items[1].source, Source::Range(_)));
        assert!(matches!(b.items[2].source, Source::Func(_)));
        assert!(matches!(b.items[3].source, Source::Type(_)));
    }

    #[test]
    fn future_version_errors() {
        let err = migrate_json(r#"{"version":99,"items":[]}"#).unwrap_err();
        assert!(err.to_string().contains("upgrade"));
    }
}
