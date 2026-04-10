//! Key event handling for the TUI.

use crate::tui::app::{App, Focus};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Poll for a key event with a 100ms timeout.
pub fn poll() -> Option<KeyEvent> {
    if event::poll(Duration::from_millis(100)).ok()? {
        if let Event::Key(key) = event::read().ok()? {
            return Some(key);
        }
    }
    None
}

/// Handle a key event, mutating app state.
pub fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Char('c') => {
            app.copy_to_clipboard();
        }
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::FileTree => app.move_tree_cursor(1),
            Focus::BundleList => app.move_bundle_cursor(1),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::FileTree => app.move_tree_cursor(-1),
            Focus::BundleList => app.move_bundle_cursor(-1),
        },
        KeyCode::Char(' ') | KeyCode::Enter => {
            if app.focus == Focus::FileTree {
                app.toggle_current();
            }
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::FileTree => Focus::BundleList,
                Focus::BundleList => Focus::FileTree,
            };
        }
        KeyCode::Char('G') => match app.focus {
            Focus::FileTree => {
                if !app.tree_entries.is_empty() {
                    app.tree_cursor = app.tree_entries.len() - 1;
                }
            }
            Focus::BundleList => {
                if !app.bundle.is_empty() {
                    app.bundle_cursor = app.bundle.len() - 1;
                }
            }
        },
        KeyCode::Char('g') => match app.focus {
            Focus::FileTree => app.tree_cursor = 0,
            Focus::BundleList => app.bundle_cursor = 0,
        },
        _ => {}
    }
}
