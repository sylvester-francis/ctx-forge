//! `_index.jsonl` append-only store — one `Note` per line as JSON.

#![allow(dead_code)]

use crate::error::Result;
use crate::memory::note::Note;
use crate::paths::CtxforgeRoot;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

pub fn append(root: &CtxforgeRoot, note: &Note) -> Result<()> {
    std::fs::create_dir_all(root.memory_dir())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.memory_index_path())?;
    let line = note.to_jsonl()?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Read notes in insertion order. Returns empty if the index does not exist.
pub fn read_all(root: &CtxforgeRoot) -> Result<Vec<Note>> {
    let path = root.memory_index_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut notes = Vec::new();
    for (lineno, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let note = Note::from_jsonl(&line).map_err(|e| {
            crate::error::CtxforgeError::Msg(format!(
                "failed to parse note at line {} of {}: {}",
                lineno + 1,
                path.display(),
                e
            ))
        })?;
        notes.push(note);
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn append_creates_index_file() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let note = Note::new("first", Some("auth".into()));
        append(&root, &note).unwrap();
        assert!(root.memory_index_path().is_file());
    }

    #[test]
    fn read_all_returns_empty_when_no_index() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let notes = read_all(&root).unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn append_then_read_roundtrip() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();

        let n1 = Note::new("first", Some("auth".into()));
        let n2 = Note::new("second", None);
        append(&root, &n1).unwrap();
        append(&root, &n2).unwrap();

        let notes = read_all(&root).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].body, "first");
        assert_eq!(notes[1].body, "second");
    }

    #[test]
    fn read_all_skips_blank_lines() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let n1 = Note::new("solo", None);
        append(&root, &n1).unwrap();

        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(root.memory_index_path())
            .unwrap();
        writeln!(f).unwrap();
        writeln!(f).unwrap();

        let notes = read_all(&root).unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn read_all_errors_on_corrupt_line() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        std::fs::write(root.memory_index_path(), "not valid json\n").unwrap();
        assert!(read_all(&root).is_err());
    }
}
