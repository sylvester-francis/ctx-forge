//! Key event handling for the TUI.
//!
//! Dispatches events based on `app.mode` — the Normal mode handles browsing,
//! while overlay/input modes (search, narrow, etc.) are added by later tasks.

use crate::tui::app::{App, Focus};
use crate::tui::mode::Mode;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Poll for a key event with a 100ms timeout (legacy entry point, kept for
/// any callers that don't need the animation-aware timeout).
pub fn poll() -> Option<KeyEvent> {
    poll_with_timeout(Duration::from_millis(100))
}

/// Poll for a key event with a caller-supplied timeout. The render loop
/// passes 16ms while animating and an effectively-infinite timeout when idle.
pub fn poll_with_timeout(timeout: Duration) -> Option<KeyEvent> {
    match poll_event_with_timeout(timeout) {
        Some(Event::Key(k)) => Some(k),
        _ => None,
    }
}

/// Poll for any supported event (Key or Mouse). The render loop dispatches
/// mouse events to `handle_mouse` and key events to `handle`.
pub fn poll_event_with_timeout(timeout: Duration) -> Option<Event> {
    if event::poll(timeout).ok()? {
        return event::read().ok();
    }
    None
}

/// Handle a mouse event. Only meaningful when the viewer is enabled and
/// the mouse lands inside the viewer pane.
pub fn handle_mouse(app: &mut App, ev: crossterm::event::MouseEvent) {
    use crossterm::event::{MouseButton, MouseEventKind};
    if !app.viewer.enabled {
        return;
    }
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.viewer_mouse_down(ev.column, ev.row);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            app.viewer_mouse_drag(ev.column, ev.row);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.viewer_mouse_up(ev.column, ev.row);
        }
        MouseEventKind::ScrollDown => {
            // Scroll only when the cursor is over the viewer pane.
            if app.viewer_line_at(ev.column, ev.row).is_some() {
                app.move_viewer_scroll(3);
            }
        }
        MouseEventKind::ScrollUp => {
            if app.viewer_line_at(ev.column, ev.row).is_some() {
                app.move_viewer_scroll(-3);
            }
        }
        _ => {}
    }
}

/// Handle a key event, mutating app state. Routes to the appropriate
/// per-mode handler. Ctrl-C is a global quit shortcut in every mode.
pub fn handle(app: &mut App, key: KeyEvent) {
    // Global quit keys work in any mode.
    if let KeyCode::Char('c') = key.code {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            app.should_quit = true;
            return;
        }
    }

    match app.mode() {
        Mode::Normal => handle_normal(app, key),
        Mode::Search { .. } => handle_search(app, key),
        Mode::Narrow { .. } => handle_narrow(app, key),
        Mode::SaveProfile { .. } => handle_save_profile(app, key),
        Mode::LoadProfile { .. } => handle_load_profile(app, key),
        Mode::PipeMenu => handle_pipe_menu(app, key),
        Mode::ModelSwitch { .. } => handle_model_switch(app, key),
        Mode::MemoryPanel { .. } => handle_memory_panel(app, key),
        Mode::AddNote { .. } => handle_add_note(app, key),
        #[cfg(feature = "extract")]
        Mode::FunctionPick { .. } => handle_function_pick(app, key),
        #[cfg(feature = "extract")]
        Mode::TypePick { .. } => handle_type_pick(app, key),
        Mode::DiffPick { .. } => handle_diff_pick(app, key),
        Mode::CommandPalette { .. } => handle_command_palette(app, key),
        Mode::Help => handle_help(app, key),
        Mode::TemplatePick { .. } => handle_template_pick(app, key),
        Mode::TemplateTask { .. } => handle_template_task(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        // Quit
        KeyCode::Char('q') => app.should_quit = true,

        // Navigation (vim-style)
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::FileTree => app.move_tree_cursor(1),
            Focus::Viewer => app.move_viewer_scroll(1),
            Focus::BundleList => app.move_bundle_cursor(1),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::FileTree => app.move_tree_cursor(-1),
            Focus::Viewer => app.move_viewer_scroll(-1),
            Focus::BundleList => app.move_bundle_cursor(-1),
        },
        KeyCode::Char(' ') => {
            if app.focus == Focus::FileTree {
                app.toggle_current();
            }
        }
        KeyCode::Enter => {
            if app.focus == Focus::FileTree {
                app.toggle_expand();
            }
        }
        KeyCode::Tab => {
            app.toggle_focus();
        }
        KeyCode::Char('G') => match app.focus {
            Focus::FileTree => {
                let len = app.visible_tree_len();
                if len > 0 {
                    app.tree_cursor = len - 1;
                    app.reload_viewer_for_cursor();
                }
            }
            Focus::Viewer => app.scroll_viewer_to_bottom(),
            Focus::BundleList => {
                if !app.bundle.is_empty() {
                    app.bundle_cursor = app.bundle.len() - 1;
                }
            }
        },
        KeyCode::Char('g') => match app.focus {
            Focus::FileTree => {
                app.tree_cursor = 0;
                app.reload_viewer_for_cursor();
            }
            Focus::Viewer => app.scroll_viewer_to_top(),
            Focus::BundleList => app.bundle_cursor = 0,
        },

        // Viewer toggle
        KeyCode::Char('v') => {
            app.toggle_viewer();
        }

        // Viewer scroll (only effective when Focus::Viewer)
        KeyCode::PageDown => {
            if app.focus == Focus::Viewer {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(half);
            }
        }
        KeyCode::PageUp => {
            if app.focus == Focus::Viewer {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(-half);
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.focus == Focus::Viewer {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(half);
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.focus == Focus::Viewer {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(-half);
            }
        }

        // Slash command palette
        KeyCode::Char('/') => {
            app.set_mode(Mode::CommandPalette {
                query: String::new(),
                cursor: 0,
            });
        }

        // Help overlay
        KeyCode::Char('?') => {
            app.set_mode(Mode::Help);
            app.show_help = true;
        }

        // Ctrl+F = direct search shortcut
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.set_mode(Mode::Search {
                query: String::new(),
            });
            app.run_search("");
        }

        // Esc closes help if open
        KeyCode::Esc => {
            if app.show_help {
                app.show_help = false;
            }
        }

        _ => {}
    }
}

fn handle_command_palette(app: &mut App, key: KeyEvent) {
    use crate::tui::commands::{fuzzy_filter, parse_palette_query};

    let (query, cursor) = match app.mode() {
        Mode::CommandPalette { query, cursor } => (query.clone(), *cursor),
        _ => return,
    };

    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            let results = fuzzy_filter(&query);
            if let Some(cmd) = results.get(cursor) {
                let (_name_typed, arg) = parse_palette_query(&query);
                let action = cmd.action;
                app.set_mode(Mode::Normal);
                action(app, arg);
            } else {
                app.set_mode(Mode::Normal);
            }
        }
        KeyCode::Char(ch) => {
            let new_query = format!("{query}{ch}");
            app.set_mode(Mode::CommandPalette {
                query: new_query,
                cursor: 0,
            });
        }
        KeyCode::Backspace => {
            if query.is_empty() {
                app.set_mode(Mode::Normal);
            } else {
                let mut new_query = query;
                new_query.pop();
                app.set_mode(Mode::CommandPalette {
                    query: new_query,
                    cursor: 0,
                });
            }
        }
        KeyCode::Down => {
            let results = fuzzy_filter(&query);
            if cursor + 1 < results.len() {
                app.set_mode(Mode::CommandPalette {
                    query,
                    cursor: cursor + 1,
                });
            }
        }
        KeyCode::Up => {
            if cursor > 0 {
                app.set_mode(Mode::CommandPalette {
                    query,
                    cursor: cursor - 1,
                });
            }
        }
        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.show_help = false;
            app.set_mode(Mode::Normal);
        }
        _ => {}
    }
}

fn handle_template_pick(app: &mut App, key: KeyEvent) {
    let (cursor, templates) = match app.mode() {
        Mode::TemplatePick { cursor, templates } => (*cursor, templates.clone()),
        _ => return,
    };
    match key.code {
        KeyCode::Esc => app.set_mode(Mode::Normal),
        KeyCode::Char('j') | KeyCode::Down => {
            if cursor + 1 < templates.len() {
                app.set_mode(Mode::TemplatePick {
                    cursor: cursor + 1,
                    templates,
                });
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if cursor > 0 {
                app.set_mode(Mode::TemplatePick {
                    cursor: cursor - 1,
                    templates,
                });
            }
        }
        KeyCode::Enter => {
            if let Some((name, _)) = templates.get(cursor) {
                let template_name = name.clone();
                app.set_mode(Mode::TemplateTask {
                    template_name,
                    task: String::new(),
                });
            }
        }
        _ => {}
    }
}

fn handle_template_task(app: &mut App, key: KeyEvent) {
    let (template_name, task) = match app.mode() {
        Mode::TemplateTask {
            template_name,
            task,
        } => (template_name.clone(), task.clone()),
        _ => return,
    };
    match key.code {
        KeyCode::Esc => app.set_mode(Mode::Normal),
        KeyCode::Enter => {
            if task.trim().is_empty() {
                app.set_status("task cannot be empty");
            } else {
                app.confirm_template_task();
            }
        }
        KeyCode::Backspace => {
            let mut new_task = task;
            new_task.pop();
            app.set_mode(Mode::TemplateTask {
                template_name,
                task: new_task,
            });
        }
        KeyCode::Char(ch) => {
            let new_task = format!("{task}{ch}");
            app.set_mode(Mode::TemplateTask {
                template_name,
                task: new_task,
            });
        }
        _ => {}
    }
}

#[cfg(feature = "extract")]
fn handle_function_pick(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            app.add_picked_function();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::FunctionPick { cursor, items } = app.mode_mut() {
                if *cursor + 1 < items.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::FunctionPick { cursor, .. } = app.mode_mut() {
                *cursor = cursor.saturating_sub(1);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "extract")]
fn handle_type_pick(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            app.add_picked_type();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::TypePick { cursor, items } = app.mode_mut() {
                if *cursor + 1 < items.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::TypePick { cursor, .. } = app.mode_mut() {
                *cursor = cursor.saturating_sub(1);
            }
        }
        _ => {}
    }
}

fn handle_diff_pick(app: &mut App, key: KeyEvent) {
    let entering = matches!(
        app.mode(),
        Mode::DiffPick {
            entering_branch: true,
            ..
        }
    );

    if entering {
        match key.code {
            KeyCode::Esc => {
                app.set_mode(Mode::Normal);
            }
            KeyCode::Enter => {
                app.load_diff_files();
            }
            KeyCode::Backspace => {
                if let Mode::DiffPick { branch, .. } = app.mode_mut() {
                    branch.pop();
                }
            }
            KeyCode::Char(ch) => {
                if let Mode::DiffPick { branch, .. } = app.mode_mut() {
                    branch.push(ch);
                }
            }
            _ => {}
        }
        return;
    }

    // File selection phase.
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Char(' ') => {
            if let Mode::DiffPick {
                selected, cursor, ..
            } = app.mode_mut()
            {
                if selected.contains(cursor) {
                    selected.remove(cursor);
                } else {
                    selected.insert(*cursor);
                }
            }
        }
        KeyCode::Enter => {
            app.add_selected_diff_files();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::DiffPick { cursor, files, .. } = app.mode_mut() {
                if *cursor + 1 < files.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::DiffPick { cursor, .. } = app.mode_mut() {
                *cursor = cursor.saturating_sub(1);
            }
        }
        _ => {}
    }
}

fn handle_memory_panel(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('r') => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::MemoryPanel { cursor, count } = app.mode_mut() {
                if *cursor + 1 < *count {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::MemoryPanel { cursor, .. } = app.mode_mut() {
                *cursor = cursor.saturating_sub(1);
            }
        }
        _ => {}
    }
}

fn handle_add_note(app: &mut App, key: KeyEvent) {
    use crate::tui::mode::InputField;
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Tab => {
            if let Mode::AddNote { field, .. } = app.mode_mut() {
                *field = match *field {
                    InputField::First => InputField::Second,
                    InputField::Second => InputField::First,
                };
            }
        }
        KeyCode::Enter => {
            app.write_note_inline();
        }
        KeyCode::Backspace => {
            if let Mode::AddNote { tag, body, field } = app.mode_mut() {
                match field {
                    InputField::First => {
                        tag.pop();
                    }
                    InputField::Second => {
                        body.pop();
                    }
                }
            }
        }
        KeyCode::Char(ch) => {
            if let Mode::AddNote { tag, body, field } = app.mode_mut() {
                match field {
                    InputField::First => tag.push(ch),
                    InputField::Second => body.push(ch),
                }
            }
        }
        _ => {}
    }
}

fn handle_pipe_menu(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Char('c') => {
            app.pipe_to_agent("claude");
        }
        KeyCode::Char('a') => {
            app.pipe_to_agent("agent");
        }
        KeyCode::Char('g') => {
            app.pipe_to_agent("gemini");
        }
        _ => {}
    }
}

fn handle_model_switch(app: &mut App, key: KeyEvent) {
    let models = crate::models::all_models();
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            let chosen = if let Mode::ModelSwitch { cursor } = app.mode() {
                models.get(*cursor).map(|m| m.name.to_string())
            } else {
                None
            };
            if let Some(name) = chosen {
                app.switch_model(&name);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::ModelSwitch { cursor } = app.mode_mut() {
                if *cursor + 1 < models.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::ModelSwitch { cursor } = app.mode_mut() {
                *cursor = cursor.saturating_sub(1);
            }
        }
        _ => {}
    }
}

fn handle_save_profile(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            let name = if let Mode::SaveProfile { name } = app.mode() {
                name.clone()
            } else {
                return;
            };
            if name.is_empty() {
                app.set_status("Profile name cannot be empty");
                return;
            }
            app.save_profile(&name);
        }
        KeyCode::Backspace => {
            if let Mode::SaveProfile { name } = app.mode_mut() {
                name.pop();
            }
        }
        KeyCode::Char(ch) => {
            if let Mode::SaveProfile { name } = app.mode_mut() {
                name.push(ch);
            }
        }
        _ => {}
    }
}

fn handle_load_profile(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            app.load_selected_profile();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::LoadProfile { cursor, profiles } = app.mode_mut() {
                if *cursor + 1 < profiles.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::LoadProfile { cursor, .. } = app.mode_mut() {
                *cursor = cursor.saturating_sub(1);
            }
        }
        _ => {}
    }
}

fn handle_narrow(app: &mut App, key: KeyEvent) {
    use crate::tui::mode::InputField;
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Tab => {
            if let Mode::Narrow { field, .. } = app.mode_mut() {
                *field = match *field {
                    InputField::First => InputField::Second,
                    InputField::Second => InputField::First,
                };
            }
        }
        KeyCode::Enter => {
            app.confirm_narrow();
        }
        KeyCode::Backspace => {
            if let Mode::Narrow { start, end, field } = app.mode_mut() {
                match field {
                    InputField::First => {
                        start.pop();
                    }
                    InputField::Second => {
                        end.pop();
                    }
                }
            }
        }
        KeyCode::Char(ch) if ch.is_ascii_digit() => {
            if let Mode::Narrow { start, end, field } = app.mode_mut() {
                match field {
                    InputField::First => start.push(ch),
                    InputField::Second => end.push(ch),
                }
            }
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::Enter => {
            // Move cursor to top search result and exit search.
            if let Some(&idx) = app.search_results.first() {
                // Find this index in visible_tree to set tree_cursor.
                if let Some(pos) = app.visible_tree.iter().position(|&v| v == idx) {
                    app.tree_cursor = pos;
                }
            }
            app.set_mode(Mode::Normal);
        }
        KeyCode::Backspace => {
            let q = if let Mode::Search { query } = app.mode_mut() {
                query.pop();
                Some(query.clone())
            } else {
                None
            };
            if let Some(q) = q {
                app.run_search(&q);
            }
        }
        KeyCode::Char(ch) => {
            let q = if let Mode::Search { query } = app.mode_mut() {
                query.push(ch);
                Some(query.clone())
            } else {
                None
            };
            if let Some(q) = q {
                app.run_search(&q);
            }
        }
        _ => {}
    }
}
