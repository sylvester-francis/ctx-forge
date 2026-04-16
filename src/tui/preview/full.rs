//! Full-text preview — the exact bytes that would be delivered right now.
//!
//! Unlike the structural preview (right column), this renders the full
//! composed prompt, including any template wrapper, task text, and bundle
//! content. It's what the user will actually send.

use crate::tui::app::App;

/// Build the full composed prompt for the current bundle + scenario.
/// Returns a user-visible string — errors are captured inline rather than
/// propagated, since this is a read-only preview and must never panic.
pub fn render_full(app: &App) -> String {
    let Some(scenario) = app.bundle.scenario.as_deref() else {
        return "(no scenario selected — use /scenario to pick one)\n".to_string();
    };

    // Resolve the bundle items (read their content from disk).
    let resolved = match crate::resolve::resolve_all(&app.bundle.items, &app.project_root) {
        Ok(r) => r,
        Err(e) => return format!("⚠ failed to resolve bundle: {e}\n"),
    };

    // Memory notes are attached to copy/export by default; mirror that
    // behaviour so the preview matches the delivered output.
    let notes = crate::memory::index::read_all(&app.root).unwrap_or_default();
    let bundle_rendered = crate::format::render(crate::format::Format::Markdown, &resolved, &notes);

    // Use scenario::load_body + template::substitute so built-in starters
    // (bugfix, code-review, explain, refactor, migrate) work alongside
    // file-backed templates. `apply_template` would only search for files
    // and reject the embedded starters.
    let body = match crate::tui::scenario::load_body(&app.root, scenario) {
        Ok(s) => s,
        Err(e) => return format!("⚠ load scenario body: {e}\n"),
    };
    match crate::template::substitute(scenario, &body, &bundle_rendered, &app.bundle.task_text) {
        Ok(s) => s,
        Err(e) => format!("⚠ template render failed: {e}\n"),
    }
}
