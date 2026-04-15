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
        Mode::ScenarioPick { .. } => handle_scenario_pick(app, key),
        Mode::FullPromptPreview { .. } => handle_full_preview(app, key),
        Mode::AtPicker { .. } => handle_at_picker(app, key),
    }
}

/// Handle keys while the `@` file picker is open. Typing extends the
/// query; Up/Down moves the cursor; Backspace either shortens the query
/// or — when the query is empty — closes the picker (leaving the `@`
/// literally in the prompt as a degenerate no-op). Enter is handled in
/// Task 21; Esc always closes cleanly.
fn handle_at_picker(app: &mut App, key: KeyEvent) {
    // Plan what to do without holding a mutable borrow across set_mode /
    // prompt mutations. The borrow-split keeps the logic linear.
    enum Action {
        None,
        Close,
        Extend(char),
        MoveCursor(i32),
        Confirm(std::path::PathBuf),
    }

    let action = {
        let Mode::AtPicker {
            query,
            results,
            cursor,
            ..
        } = app.mode_mut()
        else {
            return;
        };
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => Action::Close,
            (KeyCode::Enter, _) => results
                .get(*cursor)
                .cloned()
                .map(Action::Confirm)
                .unwrap_or(Action::Close),
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => Action::MoveCursor(1),
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => Action::MoveCursor(-1),
            (KeyCode::Backspace, _) => {
                if query.is_empty() {
                    Action::Close
                } else {
                    query.pop();
                    Action::Extend('\0') // sentinel: rerank-only, no typing
                }
            }
            (KeyCode::Char(c), mods)
                if !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT) =>
            {
                query.push(c);
                Action::Extend(c)
            }
            _ => Action::None,
        }
    };

    match action {
        Action::None => {}
        Action::Close => app.set_mode(Mode::Normal),
        Action::MoveCursor(delta) => {
            if let Mode::AtPicker {
                results, cursor, ..
            } = app.mode_mut()
            {
                let len = results.len();
                if len == 0 {
                    *cursor = 0;
                } else if delta > 0 {
                    *cursor = (*cursor + 1).min(len - 1);
                } else {
                    *cursor = cursor.saturating_sub(1);
                }
            }
        }
        Action::Extend(c) => {
            // Rerank with the now-updated query.
            if let Mode::AtPicker {
                all,
                query,
                results,
                cursor,
            } = app.mode_mut()
            {
                *results = crate::tui::prompt_input::at_picker::rank(
                    all,
                    query,
                    crate::tui::prompt_input::at_picker::RESULT_LIMIT,
                );
                *cursor = 0;
            }
            // Real char: also type it into the prompt buffer. '\0'
            // sentinel means this came from Backspace and we already
            // handled the prompt side below.
            if c != '\0' {
                app.prompt_input.insert_char(c);
                sync_task_text(app);
            } else {
                // Backspace path: remove the matching char from the prompt
                // buffer too so the in-place @query stays in sync.
                app.prompt_input.backspace();
                sync_task_text(app);
            }
        }
        Action::Confirm(path) => {
            confirm_at_mention(app, path);
        }
    }
}

/// Replace the `@<query>` token at the cursor with `@<full path>` and
/// auto-add the file to the bundle if it's not already there. Closes the
/// picker on the way out.
fn confirm_at_mention(app: &mut App, path: std::path::PathBuf) {
    // Find the '@' preceding the cursor. Everything between that '@' and
    // the cursor is the query we typed, which we replace with the full
    // path.
    let cursor = app.prompt_input.cursor();
    let text = app.prompt_input.text().to_string();
    let at_idx = text[..cursor].rfind('@').unwrap_or(cursor);
    let path_str = path.display().to_string();

    let mut new_text = text;
    new_text.replace_range(at_idx..cursor, &format!("@{path_str}"));
    app.prompt_input.set_text(new_text);
    let new_cursor = at_idx + 1 + path_str.len();
    app.prompt_input.set_cursor(new_cursor);
    sync_task_text(app);

    // Add to bundle if missing.
    if !app.bundled_paths.contains(&path) {
        app.bundle.items.push(crate::bundle::Item {
            path: path.clone(),
            kind: crate::bundle::ItemKind::File,
            label: None,
        });
        app.bundled_paths.insert(path.clone());
        app.recalculate_tokens();
        let _ = app.bundle.save(&app.root);
        app.set_status(format!("@{path_str} added to bundle"));
    } else {
        app.set_status(format!("@{path_str} (already in bundle)"));
    }

    app.set_mode(Mode::Normal);
}

/// Route a keystroke into the multi-line prompt input. Only reached when
/// `Focus::Prompt` is active. Esc defocuses back to the last panel; text
/// keys insert; Backspace / arrows / Home / End / Ctrl-W / Ctrl-A / Ctrl-E
/// have their standard meanings.
fn handle_prompt_key(app: &mut App, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) => {
            app.defocus_prompt();
            return;
        }
        (KeyCode::Tab, _) => {
            app.toggle_focus();
            return;
        }
        (KeyCode::Enter, mods) if mods.contains(KeyModifiers::SHIFT) => {
            app.prompt_input.insert_newline();
        }
        (KeyCode::Enter, _) => {
            // Plain Enter inside the prompt inserts a newline too — this
            // is a multi-line surface. Ctrl-Enter / Alt-Enter (Task 23)
            // will trigger delivery.
            app.prompt_input.insert_newline();
        }
        (KeyCode::Backspace, _) => app.prompt_input.backspace(),
        (KeyCode::Left, _) => {
            app.prompt_input.move_left();
            return;
        }
        (KeyCode::Right, _) => {
            app.prompt_input.move_right();
            return;
        }
        (KeyCode::Home, _) => {
            app.prompt_input.move_home();
            return;
        }
        (KeyCode::End, _) => {
            app.prompt_input.move_end();
            return;
        }
        (KeyCode::Char('w'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.prompt_input.delete_word_back();
        }
        (KeyCode::Char('a'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.prompt_input.move_home();
            return;
        }
        (KeyCode::Char('e'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.prompt_input.move_end();
            return;
        }
        (KeyCode::Char('@'), mods)
            if !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT) =>
        {
            // Insert the literal '@' at the cursor (so the picker can
            // later replace it with the selected path) and open the
            // fuzzy popover.
            app.prompt_input.insert_char('@');
            sync_task_text(app);
            let all = crate::tui::prompt_input::at_picker::walk_files(&app.project_root);
            let results = crate::tui::prompt_input::at_picker::rank(
                &all,
                "",
                crate::tui::prompt_input::at_picker::RESULT_LIMIT,
            );
            app.set_mode(Mode::AtPicker {
                all,
                query: String::new(),
                results,
                cursor: 0,
            });
            return;
        }
        (KeyCode::Char('/'), mods)
            if !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT) =>
        {
            // Opens the slash-command palette when the cursor is at the
            // start of a line (empty buffer or sitting right after a
            // newline). Anywhere else, `/` inserts literally — this
            // keeps paths like `src/main.rs` type-able inside the task.
            let text = app.prompt_input.text();
            let cursor = app.prompt_input.cursor();
            let at_line_start = cursor == 0 || text[..cursor].ends_with('\n');
            if at_line_start {
                app.set_mode(Mode::CommandPalette {
                    query: String::new(),
                    cursor: 0,
                });
                return;
            }
            app.prompt_input.insert_char('/');
        }
        (KeyCode::Char(c), mods)
            if !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT) =>
        {
            app.prompt_input.insert_char(c);
        }
        _ => return,
    }
    // Reached only for mutation branches — keep bundle.task_text in lockstep
    // with the prompt buffer so the preview updates on the next frame.
    sync_task_text(app);
}

/// Copy the current prompt input text into `bundle.task_text`. Called
/// after any mutation so the preview's `## Task` section reflects the
/// latest edit without a manual save.
pub fn sync_task_text(app: &mut App) {
    app.bundle.task_text = app.prompt_input.text().to_string();
}

/// Handle a bracketed-paste event. Inserts the pasted string verbatim
/// into the prompt when `Focus::Prompt` is active; ignored otherwise.
/// The atomic insert avoids per-char side effects (e.g. a `@` in the
/// paste triggering the file picker once that lands in Phase 5).
pub fn handle_paste(app: &mut App, content: String) {
    if !matches!(app.focus, Focus::Prompt) {
        return;
    }
    app.prompt_input.insert_str(&content);
    sync_task_text(app);
}

fn handle_full_preview(app: &mut App, key: KeyEvent) {
    let Mode::FullPromptPreview { scroll, .. } = app.mode_mut() else {
        return;
    };
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('P') => {
            app.set_mode(Mode::Normal);
        }
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *scroll = scroll.saturating_add(10);
        }
        KeyCode::PageDown => {
            *scroll = scroll.saturating_add(10);
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *scroll = scroll.saturating_sub(10);
        }
        KeyCode::PageUp => {
            *scroll = scroll.saturating_sub(10);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            *scroll = scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            *scroll = scroll.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            *scroll = 0;
        }
        _ => {}
    }
}

fn handle_scenario_pick(app: &mut App, key: KeyEvent) {
    let Mode::ScenarioPick { cursor, scenarios } = app.mode_mut() else {
        return;
    };
    match key.code {
        KeyCode::Esc => app.set_mode(Mode::Normal),
        KeyCode::Char('j') | KeyCode::Down => {
            if !scenarios.is_empty() {
                *cursor = (*cursor + 1).min(scenarios.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            *cursor = cursor.saturating_sub(1);
        }
        KeyCode::Enter => {
            let picked = scenarios.get(*cursor).map(|s| s.name.clone());
            app.set_mode(Mode::Normal);
            if let Some(name) = picked {
                if let Err(e) = app.set_scenario(&name) {
                    app.set_status(e);
                }
            }
        }
        _ => {}
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Prompt focus short-circuits normal-mode handling: everything the user
    // types goes into the multi-line input. Dedicated keys (Esc to defocus,
    // Ctrl-C global quit above) remain available.
    if matches!(app.focus, Focus::Prompt) {
        handle_prompt_key(app, key);
        return;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => app.should_quit = true,

        // Focus the prompt input (text surface at the bottom).
        KeyCode::Char('i') => app.focus_prompt(),

        // Navigation (vim-style)
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::FileTree => app.move_tree_cursor(1),
            Focus::Viewer => app.move_viewer_scroll(1),
            Focus::BundleList => app.move_bundle_cursor(1),
            Focus::Prompt => {}
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::FileTree => app.move_tree_cursor(-1),
            Focus::Viewer => app.move_viewer_scroll(-1),
            Focus::BundleList => app.move_bundle_cursor(-1),
            Focus::Prompt => {}
        },

        // Page scrolling — PageDown / PageUp jumps a full viewport.
        KeyCode::PageDown => match app.focus {
            Focus::FileTree => app.page_tree_cursor(1),
            Focus::Viewer => {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(half);
            }
            Focus::BundleList => app.page_bundle_cursor(1),
            Focus::Prompt => {}
        },
        KeyCode::PageUp => match app.focus {
            Focus::FileTree => app.page_tree_cursor(-1),
            Focus::Viewer => {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(-half);
            }
            Focus::BundleList => app.page_bundle_cursor(-1),
            Focus::Prompt => {}
        },

        // Vim-style half-page: Ctrl-D / Ctrl-U.
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => match app.focus {
            Focus::FileTree => app.half_page_tree_cursor(1),
            Focus::Viewer => {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(half);
            }
            Focus::BundleList => app.half_page_bundle_cursor(1),
            Focus::Prompt => {}
        },
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => match app.focus {
            Focus::FileTree => app.half_page_tree_cursor(-1),
            Focus::Viewer => {
                let half = (app.viewer_last_viewport_height.get() / 2).max(1) as i32;
                app.move_viewer_scroll(-half);
            }
            Focus::BundleList => app.half_page_bundle_cursor(-1),
            Focus::Prompt => {}
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
            Focus::Prompt => {}
        },
        KeyCode::Char('g') => match app.focus {
            Focus::FileTree => {
                app.tree_cursor = 0;
                app.reload_viewer_for_cursor();
            }
            Focus::Viewer => app.scroll_viewer_to_top(),
            Focus::BundleList => app.bundle_cursor = 0,
            Focus::Prompt => {}
        },

        // Viewer toggle
        KeyCode::Char('v') => {
            app.toggle_viewer();
        }

        // Add the current viewer selection to the bundle as a Range item.
        KeyCode::Char('a') if app.focus == Focus::Viewer => {
            app.add_viewer_selection_to_bundle();
        }

        // Clear an active viewer selection without adding it.
        KeyCode::Esc if app.focus == Focus::Viewer && app.viewer.selection().is_some() => {
            app.viewer.clear_selection();
        }

        // Expand / collapse all directories in the tree.
        KeyCode::Char('E') => {
            if app.focus == Focus::FileTree {
                app.expand_all_dirs();
            }
        }
        KeyCode::Char('C') => {
            if app.focus == Focus::FileTree {
                app.collapse_all_dirs();
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

        // Full-text composed-prompt preview.
        KeyCode::Char('P') => {
            let content = crate::tui::preview::full::render_full(app);
            app.set_mode(Mode::FullPromptPreview { content, scroll: 0 });
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
