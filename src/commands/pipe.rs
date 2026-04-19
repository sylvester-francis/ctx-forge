//! `ctxforge pipe <target>` — pipe the rendered bundle to a local agent CLI.
//!
//! Spawns the target CLI as a subprocess and writes the formatted bundle to
//! its stdin. Never makes HTTP calls — the target CLI holds its own
//! credentials and handles the network.
//!
//! Known targets auto-select a format (XML for claude, markdown for others).
//! The user can override with `--format`.

use crate::bundle::Bundle;
use crate::error::{CtxforgeError, Result};
use crate::format::{self, Format};
use crate::memory;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use std::io::Write;
use std::process::{Command, Stdio};

fn default_format_for(target: &str) -> Format {
    match target {
        "claude" => Format::Xml,
        _ => Format::Markdown,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    root: &CtxforgeRoot,
    target: &str,
    format_override: Option<&str>,
    no_memory: bool,
    memory_tag: Option<String>,
    memory_limit: usize,
    template_name: Option<&str>,
    task: Option<String>,
    extra_args: &[String],
) -> Result<()> {
    let fmt = match format_override {
        Some(name) => Format::parse(name).ok_or_else(|| {
            CtxforgeError::Msg(format!(
                "invalid --format `{name}` (expected one of: markdown, md, xml, json)"
            ))
        })?,
        None => default_format_for(target),
    };

    let bundle = Bundle::load_or_default(root)?;
    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;
    let memory_notes =
        memory::collect_for_attach(root, no_memory, memory_tag.as_deref(), memory_limit)?;
    // Piping to an LLM CLI — audit-trail provenance wastes prompt tokens.
    let rendered = format::render(fmt, &resolved, &memory_notes, true);

    let rendered = match template_name {
        Some(name) => crate::template::apply_template(root, name, &rendered, task.as_deref())?,
        None => rendered,
    };

    let mut child = Command::new(target)
        .args(extra_args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| {
            CtxforgeError::Msg(format!(
                "failed to start `{target}`: {e}\n\
                 (Is `{target}` installed and in your PATH?)"
            ))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(rendered.as_bytes())
            .map_err(|e| CtxforgeError::Msg(format!("failed to write to `{target}` stdin: {e}")))?;
    }

    let status = child.wait()?;

    let note_info = if memory_notes.is_empty() {
        String::new()
    } else {
        format!(" + {} memory note(s)", memory_notes.len())
    };

    if status.success() {
        eprintln!(
            "piped {} item(s){} ({}) to `{target}`",
            bundle.len(),
            note_info,
            fmt.name(),
        );
    } else {
        return Err(CtxforgeError::Msg(format!(
            "`{target}` exited with {status}"
        )));
    }

    Ok(())
}
