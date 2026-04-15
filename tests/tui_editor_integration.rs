//! Integration tests for $EDITOR round-tripping.

#![cfg(feature = "tui")]
#![cfg(unix)] // fake-editor script relies on POSIX exec bit + shell

use std::os::unix::fs::PermissionsExt;

/// Write a POSIX shell script to `path` that appends `_edited` to the
/// content of its first argument. Returns the script path.
fn write_fake_editor(path: &std::path::Path) {
    std::fs::write(
        path,
        "#!/usr/bin/env bash\nset -e\ncontent=$(cat \"$1\")\nprintf '%s_edited' \"$content\" > \"$1\"\n",
    )
    .unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn spawn_editor_round_trips_content() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("fake_editor.sh");
    write_fake_editor(&script);

    let edited = ctxforge::tui::editor::spawn_editor_with("original", script.to_str().unwrap())
        .expect("fake editor should succeed");
    assert_eq!(edited, "original_edited");
}

#[test]
fn spawn_editor_errors_on_missing_editor() {
    let result = ctxforge::tui::editor::spawn_editor_with("x", "/definitely/does/not/exist/editor");
    assert!(result.is_err());
}

#[test]
fn spawn_editor_errors_when_editor_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("failing_editor.sh");
    std::fs::write(&script, "#!/usr/bin/env bash\nexit 1\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let result = ctxforge::tui::editor::spawn_editor_with("x", script.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn ctrl_e_marks_pending_editor_with_current_task() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ctxforge::paths::CtxforgeRoot;
    use ctxforge::tui::app::{App, PendingEditor};

    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    app.bundle.scenario = Some("bugfix".to_string());
    app.focus_prompt();
    app.prompt_input.set_text("work in progress".to_string());
    app.bundle.task_text = "work in progress".to_string();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
    );

    match app.pending_editor.as_ref() {
        Some(PendingEditor::TaskText(s)) => assert_eq!(s, "work in progress"),
        _ => panic!(
            "expected PendingEditor::TaskText, got {:?}",
            app.pending_editor
        ),
    }
}

#[test]
fn slash_edit_prompt_command_marks_pending_full_prompt() {
    use ctxforge::paths::CtxforgeRoot;
    use ctxforge::tui::app::{App, PendingEditor};

    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    app.bundle.scenario = Some("bugfix".to_string());

    let spec = ctxforge::tui::commands::COMMANDS
        .iter()
        .find(|c| c.name == "edit-prompt")
        .expect("/edit-prompt command should exist");
    (spec.action)(&mut app, None);

    assert!(matches!(
        app.pending_editor,
        Some(PendingEditor::FullPrompt(_))
    ));
}
