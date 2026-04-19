//! Cross-session memory: notes, tag files, JSONL index, recall.
//!
//! Storage under `.ctxforge/memory/`:
//! - `_index.jsonl` — append-only, one note per line. Canonical source for recall.
//! - `<tag>.md` / `decisions.md` — markdown mirror for git review; write-only.

#![allow(dead_code)]

pub mod index;
pub mod note;
pub mod recall;

pub use note::Note;
pub use recall::{RecallFilter, recall as run_recall};

use crate::error::Result;
use crate::paths::CtxforgeRoot;
use std::io::Write;

pub fn append_to_tag_file(root: &CtxforgeRoot, note: &Note) -> Result<()> {
    std::fs::create_dir_all(root.memory_dir())?;
    let path = root.memory_tag_path(note.tag.as_deref());

    let fresh = !path.exists();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    if fresh {
        let title = note.tag.as_deref().unwrap_or("decisions");
        writeln!(file, "# Memory — {title}\n")?;
    }

    write!(file, "{}", note.to_markdown_block())?;
    Ok(())
}

/// Write a note to both the JSONL index and the per-tag markdown file.
pub fn write_note(
    root: &CtxforgeRoot,
    body: impl Into<String>,
    tag: Option<String>,
) -> Result<Note> {
    let body = body.into();
    if body.trim().is_empty() {
        return Err(crate::error::CtxforgeError::Msg(
            "note body cannot be empty".into(),
        ));
    }
    let note = Note::new(body, tag);
    index::append(root, &note)?;
    append_to_tag_file(root, &note)?;
    Ok(note)
}

/// Collect notes to auto-attach to an export (newest-first, tag-filtered).
pub fn collect_for_attach(
    root: &CtxforgeRoot,
    no_memory: bool,
    tag: Option<&str>,
    limit: usize,
) -> Result<Vec<Note>> {
    if no_memory {
        return Ok(Vec::new());
    }
    let all = index::read_all(root)?;
    let filter = RecallFilter {
        tag: tag.map(String::from),
        limit: Some(limit),
        ..Default::default()
    };
    Ok(run_recall(&all, &filter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn append_creates_tag_file_with_heading() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let note = Note::new("use JWT", Some("auth".into()));
        append_to_tag_file(&root, &note).unwrap();

        let contents = std::fs::read_to_string(root.memory_tag_path(Some("auth"))).unwrap();
        assert!(contents.starts_with("# Memory — auth"));
        assert!(contents.contains("use JWT"));
    }

    #[test]
    fn append_untagged_goes_to_decisions() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let note = Note::new("general decision", None);
        append_to_tag_file(&root, &note).unwrap();

        let contents = std::fs::read_to_string(root.memory_tag_path(None)).unwrap();
        assert!(contents.contains("# Memory — decisions"));
        assert!(contents.contains("general decision"));
    }

    #[test]
    fn append_twice_does_not_duplicate_heading() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        append_to_tag_file(&root, &Note::new("first", Some("auth".into()))).unwrap();
        append_to_tag_file(&root, &Note::new("second", Some("auth".into()))).unwrap();

        let contents = std::fs::read_to_string(root.memory_tag_path(Some("auth"))).unwrap();
        assert_eq!(contents.matches("# Memory — auth").count(), 1);
        assert!(contents.contains("first"));
        assert!(contents.contains("second"));
    }

    #[test]
    fn write_note_appends_to_index_and_tag_file() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let note = write_note(&root, "JWT in header", Some("auth".into())).unwrap();

        let indexed = index::read_all(&root).unwrap();
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].body, "JWT in header");

        let md = std::fs::read_to_string(root.memory_tag_path(Some("auth"))).unwrap();
        assert!(md.contains("JWT in header"));

        assert!(note.timestamp <= chrono::Utc::now());
    }

    #[test]
    fn write_note_rejects_empty_body() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        assert!(write_note(&root, "", None).is_err());
        assert!(write_note(&root, "   ", None).is_err());
    }

    #[test]
    fn collect_for_attach_returns_recent_notes() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        write_note(&root, "one", None).unwrap();
        write_note(&root, "two", Some("auth".into())).unwrap();
        write_note(&root, "three", Some("tls".into())).unwrap();

        let notes = collect_for_attach(&root, false, None, 10).unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].body, "three");
    }

    #[test]
    fn collect_for_attach_respects_no_memory() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        write_note(&root, "one", None).unwrap();
        let notes = collect_for_attach(&root, true, None, 10).unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn collect_for_attach_filters_by_tag() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        write_note(&root, "auth thing", Some("auth".into())).unwrap();
        write_note(&root, "tls thing", Some("tls".into())).unwrap();

        let notes = collect_for_attach(&root, false, Some("auth"), 10).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "auth thing");
    }

    #[test]
    fn collect_for_attach_respects_limit() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        for i in 0..10 {
            write_note(&root, format!("note {i}"), None).unwrap();
        }
        let notes = collect_for_attach(&root, false, None, 3).unwrap();
        assert_eq!(notes.len(), 3);
    }
}
