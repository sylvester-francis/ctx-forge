//! Deliver picker — orchestrates final prompt emission from the TUI.
//!
//! The picker appears on Ctrl-Enter / Alt-Enter from the prompt input (or
//! `/deliver` from any mode) and lists every available destination:
//!
//!   - pipe claude / agent / gemini (XML / markdown)
//!   - copy markdown / XML / JSON to clipboard
//!   - export to stdout
//!
//! Selection runs through `run_choice`, which builds the payload (raw bundle
//! markdown/XML/JSON, wrapped by the active scenario's template if any)
//! and hands off to the existing clipboard / pending_pipe / pending_stdout
//! machinery. Last choice is remembered per-session so the next Ctrl-Enter
//! starts with the cursor on it.

#![allow(dead_code)]

use crate::tui::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverChoice {
    PipeClaude,
    PipeAgent,
    PipeGemini,
    CopyMarkdown,
    CopyXml,
    CopyJson,
    Export,
}

impl DeliverChoice {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PipeClaude => "pipe claude (XML)",
            Self::PipeAgent => "pipe agent (markdown)",
            Self::PipeGemini => "pipe gemini (markdown)",
            Self::CopyMarkdown => "copy markdown",
            Self::CopyXml => "copy XML",
            Self::CopyJson => "copy JSON",
            Self::Export => "export to stdout",
        }
    }

    pub fn all() -> &'static [DeliverChoice] {
        &[
            Self::PipeClaude,
            Self::PipeAgent,
            Self::PipeGemini,
            Self::CopyMarkdown,
            Self::CopyXml,
            Self::CopyJson,
            Self::Export,
        ]
    }
}

/// Execute a deliver choice. Returns the payload (useful for tests).
/// Side effects:
///   - Copy choices write to the system clipboard.
///   - Pipe choices stash the payload + target in `app.pending_pipe`;
///     the run loop picks it up, spawns the target binary, and writes
///     to its stdin.
///   - Export stashes the payload in `app.pending_stdout`.
/// `app.deliver_last` is updated to the chosen variant.
/// `app.prompt_override`, if set, replaces the template-rendered payload
/// (used by Task 27's full-prompt editor).
pub fn run_choice(app: &mut App, choice: DeliverChoice) -> Result<String, String> {
    let content = render_payload(app, choice)?;

    match choice {
        DeliverChoice::CopyMarkdown | DeliverChoice::CopyXml | DeliverChoice::CopyJson => {
            crate::clipboard::set(&content).map_err(|e| format!("clipboard: {e}"))?;
            app.set_status(format!(
                "copied as {} · {} chars",
                format_label(choice),
                content.chars().count()
            ));
        }
        DeliverChoice::PipeClaude => {
            app.pending_pipe = Some(("claude".to_string(), content.clone()));
        }
        DeliverChoice::PipeAgent => {
            app.pending_pipe = Some(("agent".to_string(), content.clone()));
        }
        DeliverChoice::PipeGemini => {
            app.pending_pipe = Some(("gemini".to_string(), content.clone()));
        }
        DeliverChoice::Export => {
            app.pending_stdout = Some(content.clone());
        }
    }

    app.deliver_last = Some(choice);
    // A one-shot prompt override (Task 27) applies to the next deliver
    // and is cleared afterwards so the user returns to the template-driven
    // default without an extra keystroke.
    app.prompt_override = None;
    Ok(content)
}

fn format_label(choice: DeliverChoice) -> &'static str {
    match choice {
        DeliverChoice::CopyMarkdown => "markdown",
        DeliverChoice::CopyXml => "XML",
        DeliverChoice::CopyJson => "JSON",
        _ => "",
    }
}

fn render_payload(app: &App, choice: DeliverChoice) -> Result<String, String> {
    // Task 27 will let a user open the composed prompt in $EDITOR and
    // stash a hand-edited version in prompt_override for one deliver.
    if let Some(override_content) = &app.prompt_override {
        return Ok(override_content.clone());
    }

    let format = match choice {
        DeliverChoice::PipeClaude | DeliverChoice::CopyXml => crate::format::Format::Xml,
        DeliverChoice::CopyJson => crate::format::Format::Json,
        _ => crate::format::Format::Markdown,
    };

    let resolved = crate::resolve::resolve_all(&app.bundle.items, &app.project_root)
        .map_err(|e| format!("resolve bundle: {e}"))?;
    let notes = crate::memory::index::read_all(&app.root).unwrap_or_default();
    let bundle_rendered = crate::format::render(format, &resolved, &notes);

    // If a scenario is active, wrap the raw bundle with its template.
    // Uses scenario::load_body so built-in starters (bugfix,
    // code-review, etc.) work alongside project- and user-global
    // template files. Plain substitute skips the task-flag validation
    // that apply_template does — appropriate here because a TUI
    // deliver can legitimately send a task-less prompt.
    if let Some(scenario) = &app.bundle.scenario {
        let body = crate::tui::scenario::load_body(&app.root, scenario)
            .map_err(|e| format!("load scenario body: {e}"))?;
        crate::template::substitute(scenario, &body, &bundle_rendered, &app.bundle.task_text)
            .map_err(|e| format!("render template: {e}"))
    } else {
        Ok(bundle_rendered)
    }
}
