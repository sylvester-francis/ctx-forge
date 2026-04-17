//! `$EDITOR` integration for in-TUI editing.
//!
//! Writes content to a temp file, spawns the editor with stdin/stdout/
//! stderr inherited (so the editor takes over the terminal), waits for
//! it to exit, reads the (potentially edited) content back, and returns
//! it. Callers drain the returned string into wherever they need.
//!
//! The run loop is responsible for leaving ratatui's alternate screen
//! before calling in here and re-entering afterwards — this module only
//! handles the editor spawn itself.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::process::{Command, Stdio};

/// Spawn the user's `$EDITOR` (or `$CTXFORGE_EDITOR` override) on a
/// temporary file pre-seeded with `content`. Blocks until the editor
/// exits, then reads the file back and returns its contents.
pub fn spawn_editor(content: &str) -> Result<String, String> {
    let editor = std::env::var("CTXFORGE_EDITOR")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| "no $EDITOR set — configure your shell or set CTXFORGE_EDITOR".to_string())?;
    spawn_editor_with(content, &editor)
}

/// Variant that accepts an explicit editor command (useful for tests
/// that point at a fake editor script instead of `$EDITOR`).
pub fn spawn_editor_with(content: &str, editor: &str) -> Result<String, String> {
    let mut tmp = tempfile::NamedTempFile::new().map_err(|e| format!("create temp file: {e}"))?;
    tmp.write_all(content.as_bytes())
        .map_err(|e| format!("write temp file: {e}"))?;
    tmp.flush().map_err(|e| format!("flush temp file: {e}"))?;
    let path = tmp.into_temp_path();

    // Allow `EDITOR="vim -p"` style commands with arguments. The first
    // whitespace-separated token is the binary; the rest are argv
    // prefixed before the temp file path.
    let mut parts = editor.split_whitespace();
    let cmd = parts
        .next()
        .ok_or_else(|| "empty editor command".to_string())?;
    let prefix_args: Vec<&str> = parts.collect();

    let status = Command::new(cmd)
        .args(&prefix_args)
        .arg(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("spawn {cmd}: {e}"))?;
    if !status.success() {
        return Err(format!("{cmd} exited with {status}"));
    }

    let mut body = String::new();
    std::fs::File::open(&path)
        .and_then(|mut f| f.read_to_string(&mut body))
        .map_err(|e| format!("read back: {e}"))?;
    Ok(body)
}
