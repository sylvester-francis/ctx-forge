//! Prompt template parser, substitution, and file resolution.
//!
//! Templates are plain-text files with `{{bundle}}` and `{{task}}`
//! placeholders. Substitution is single-pass and does not re-scan
//! substituted content — file content inside `{{bundle}}` containing
//! the literal string `{{task}}` stays literal in the output.

#![allow(dead_code)]

use crate::error::{CtxforgeError, Result};
use crate::paths::CtxforgeRoot;
use std::path::PathBuf;

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
pub fn substitute(template_name: &str, template: &str, bundle: &str, task: &str) -> Result<String> {
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

/// Resolves a template name to a file path. Tries the project-local
/// templates dir first, then the user-global config dir. Returns the
/// path to the file if found, or a descriptive error.
pub fn resolve_template_path(root: &CtxforgeRoot, name: &str) -> Result<PathBuf> {
    let global = crate::paths::global_templates_dir();
    resolve_template_path_with_global(root, name, global.as_deref())
}

/// Resolution helper with explicit global directory (used by tests so
/// they can isolate from `$HOME`).
pub fn resolve_template_path_with_global(
    root: &CtxforgeRoot,
    name: &str,
    global_dir: Option<&std::path::Path>,
) -> Result<PathBuf> {
    if name.is_empty() {
        return Err(CtxforgeError::Msg("template name required".into()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(CtxforgeError::Msg(format!(
            "template name must not contain path separators (got '{name}')"
        )));
    }

    // Strip optional `.md` extension to normalize.
    let stem = name.strip_suffix(".md").unwrap_or(name);

    // 1. Project-local
    let project = root.template_path(stem);
    if project.is_file() {
        return Ok(project);
    }

    // 2. Global
    if let Some(g) = global_dir {
        let global_path = g.join(format!("{stem}.md"));
        if global_path.is_file() {
            return Ok(global_path);
        }
    }

    let mut msg = format!(
        "template '{stem}' not found\n  looked in:\n    {}",
        project.display()
    );
    if let Some(g) = global_dir {
        msg.push_str(&format!("\n    {}", g.join(format!("{stem}.md")).display()));
    }
    msg.push_str(&format!(
        "\n  Tip: create one with 'ctxforge templates new {stem}'"
    ));
    Err(CtxforgeError::Msg(msg))
}

/// Returns true if the template text contains a `{{task}}` placeholder.
pub fn template_has_task_placeholder(template: &str) -> bool {
    parse_placeholders(template)
        .iter()
        .any(|(_, _, n)| *n == "task")
}

/// Loads, validates, and renders a template wrapping the given bundle
/// content. Used by copy/export/pipe handlers.
pub fn apply_template(
    root: &CtxforgeRoot,
    template_name: &str,
    bundle_rendered: &str,
    task_flag: Option<&str>,
) -> Result<String> {
    let path = resolve_template_path(root, template_name)?;
    let template_text = std::fs::read_to_string(&path).map_err(|e| {
        CtxforgeError::Msg(format!(
            "cannot read template '{template_name}' at {}: {e}",
            path.display()
        ))
    })?;
    let requires_task = template_has_task_placeholder(&template_text);
    if requires_task && task_flag.is_none() {
        return Err(CtxforgeError::Msg(
            "template requires --task; provide --task \"<text>\" or --task - to read from stdin"
                .into(),
        ));
    }
    let task = task_flag.unwrap_or("");
    substitute(template_name, &template_text, bundle_rendered, task)
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

    #[test]
    fn resolve_project_template_wins_over_global() {
        use tempfile::TempDir;
        let td = TempDir::new().unwrap();
        let root_dir = td.path().join("project");
        std::fs::create_dir_all(&root_dir).unwrap();
        let project_templates = root_dir.join(".ctxforge").join("templates");
        std::fs::create_dir_all(&project_templates).unwrap();
        std::fs::write(project_templates.join("bugfix.md"), "PROJECT VERSION").unwrap();

        let global_dir = td.path().join("global-config");
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::write(global_dir.join("bugfix.md"), "GLOBAL VERSION").unwrap();

        let root = crate::paths::CtxforgeRoot::find_or_create(&root_dir).unwrap();
        let resolved = resolve_template_path_with_global(&root, "bugfix", Some(&global_dir));
        let path = resolved.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "PROJECT VERSION");
    }

    #[test]
    fn resolve_falls_back_to_global() {
        use tempfile::TempDir;
        let td = TempDir::new().unwrap();
        let root_dir = td.path().join("project");
        std::fs::create_dir_all(&root_dir).unwrap();

        let global_dir = td.path().join("global-config");
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::write(global_dir.join("bugfix.md"), "GLOBAL VERSION").unwrap();

        let root = crate::paths::CtxforgeRoot::find_or_create(&root_dir).unwrap();
        let resolved = resolve_template_path_with_global(&root, "bugfix", Some(&global_dir));
        let path = resolved.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "GLOBAL VERSION");
    }

    #[test]
    fn resolve_errors_if_missing_in_both() {
        use tempfile::TempDir;
        let td = TempDir::new().unwrap();
        let root_dir = td.path().join("project");
        std::fs::create_dir_all(&root_dir).unwrap();

        let global_dir = td.path().join("global-config");
        std::fs::create_dir_all(&global_dir).unwrap();

        let root = crate::paths::CtxforgeRoot::find_or_create(&root_dir).unwrap();
        let resolved = resolve_template_path_with_global(&root, "missing", Some(&global_dir));
        assert!(resolved.is_err());
        let msg = format!("{}", resolved.unwrap_err());
        assert!(msg.contains("not found"));
    }

    #[test]
    fn resolve_appends_md_extension_automatically() {
        use tempfile::TempDir;
        let td = TempDir::new().unwrap();
        let root_dir = td.path().join("project");
        let project_templates = root_dir.join(".ctxforge").join("templates");
        std::fs::create_dir_all(&project_templates).unwrap();
        std::fs::write(project_templates.join("bugfix.md"), "X").unwrap();

        let root = crate::paths::CtxforgeRoot::find_or_create(&root_dir).unwrap();
        // Pass "bugfix" without .md extension.
        let resolved = resolve_template_path_with_global(&root, "bugfix", None);
        assert!(resolved.is_ok());
    }

    #[test]
    fn resolve_rejects_path_separators() {
        use tempfile::TempDir;
        let td = TempDir::new().unwrap();
        let root_dir = td.path().join("project");
        std::fs::create_dir_all(&root_dir).unwrap();
        let root = crate::paths::CtxforgeRoot::find_or_create(&root_dir).unwrap();

        let r1 = resolve_template_path_with_global(&root, "foo/bar", None);
        assert!(r1.is_err());
        let r2 = resolve_template_path_with_global(&root, "foo\\bar", None);
        assert!(r2.is_err());
    }
}
