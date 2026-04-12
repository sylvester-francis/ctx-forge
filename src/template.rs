//! Prompt template parser, substitution, and file resolution.
//!
//! Templates are plain-text files with `{{bundle}}` and `{{task}}`
//! placeholders. Substitution is single-pass and does not re-scan
//! substituted content — file content inside `{{bundle}}` containing
//! the literal string `{{task}}` stays literal in the output.

#![allow(dead_code)]

use crate::error::{CtxforgeError, Result};

/// Returns spans of every `{{name}}` placeholder in the template.
/// Each entry is `(start_byte, end_byte, name_str)`. The end_byte is
/// the position immediately after the closing `}}`.
///
/// Placeholder names must match `[a-z_]+` (lowercase ASCII letters and
/// underscores, length ≥ 1). Anything else is treated as literal text
/// and not returned.
pub fn parse_placeholders(template: &str) -> Vec<(usize, usize, &str)> {
    let bytes = template.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len() && (bytes[j].is_ascii_lowercase() || bytes[j] == b'_') {
                j += 1;
            }
            if j + 1 < bytes.len() && bytes[j] == b'}' && bytes[j + 1] == b'}' && j > name_start {
                result.push((i, j + 2, &template[name_start..j]));
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    result
}

/// Single-pass substitution. Replacements happen only at positions
/// parsed from the *original* template — content inside a substituted
/// `{{bundle}}` is never re-scanned for placeholders. Returns an error
/// if any placeholder is not in the v1.1 allow-list `{bundle, task}`.
pub fn substitute(
    template_name: &str,
    template: &str,
    bundle: &str,
    task: &str,
) -> Result<String> {
    let placeholders = parse_placeholders(template);

    // Phase 1: validate all placeholders are known.
    for (_, _, name) in &placeholders {
        if !matches!(*name, "bundle" | "task") {
            return Err(CtxforgeError::Msg(format!(
                "template '{template_name}' references unknown placeholder {{{{{name}}}}}\n  valid in v1.1: {{{{bundle}}}}, {{{{task}}}}"
            )));
        }
    }

    // Phase 2: substitute in a single pass.
    let mut out = String::with_capacity(template.len() + bundle.len() + task.len());
    let mut last_end = 0;
    for (start, end, name) in placeholders {
        out.push_str(&template[last_end..start]);
        out.push_str(match name {
            "bundle" => bundle,
            "task" => task,
            _ => unreachable!(),
        });
        last_end = end;
    }
    out.push_str(&template[last_end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_placeholders_finds_bundle_and_task() {
        let t = "before {{task}} middle {{bundle}} after";
        let p = parse_placeholders(t);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].2, "task");
        assert_eq!(p[1].2, "bundle");
    }

    #[test]
    fn parse_placeholders_ignores_malformed() {
        // Empty {{}}, uppercase, hyphens — none of these are valid placeholder names.
        let t = "x {{}} y {{Foo}} z {{bun-dle}} w";
        let p = parse_placeholders(t);
        assert!(p.is_empty(), "expected empty, got {p:?}");
    }

    #[test]
    fn parse_placeholders_handles_adjacent() {
        let t = "{{task}}{{bundle}}";
        let p = parse_placeholders(t);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].2, "task");
        assert_eq!(p[1].2, "bundle");
    }

    #[test]
    fn parse_placeholders_handles_underscores() {
        let t = "{{some_name}}";
        let p = parse_placeholders(t);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].2, "some_name");
    }

    #[test]
    fn substitute_inserts_bundle_and_task() {
        let t = "Task: {{task}}\nCode:\n{{bundle}}";
        let r = substitute("test", t, "FILES", "fix the bug").unwrap();
        assert_eq!(r, "Task: fix the bug\nCode:\nFILES");
    }

    #[test]
    fn substitute_handles_repeated_placeholders() {
        let t = "{{task}} ... {{task}}";
        let r = substitute("test", t, "B", "T").unwrap();
        assert_eq!(r, "T ... T");
    }

    #[test]
    fn substitute_preserves_surrounding_text() {
        let t = "before\n{{bundle}}\nafter";
        let r = substitute("test", t, "MIDDLE", "").unwrap();
        assert_eq!(r, "before\nMIDDLE\nafter");
    }

    #[test]
    fn substitute_does_not_rescan_substituted_content() {
        // The bundle content contains the literal string {{task}}.
        // The substitute should NOT replace it again.
        let t = "{{bundle}}";
        let bundle = "this contains {{task}} as text";
        let r = substitute("test", t, bundle, "MY_TASK").unwrap();
        assert_eq!(r, "this contains {{task}} as text");
    }

    #[test]
    fn substitute_errors_on_unknown_placeholder() {
        let t = "{{unknown}}";
        let r = substitute("test", t, "B", "T");
        assert!(r.is_err());
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("unknown placeholder"));
        assert!(msg.contains("unknown"));
    }
}
