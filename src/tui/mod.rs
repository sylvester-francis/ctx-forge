//! Interactive TUI composer.
//!
//! Run with `ctxforge` (no subcommand) when stdin is a TTY.
//! Two-panel layout: file tree (left) + bundle list (right), with a live
//! token gauge and keybinding footer.

pub mod app;
pub mod events;
pub mod theme;
pub mod tree;
pub mod ui;

use crate::error::Result;
use crate::paths::CtxforgeRoot;
use app::App;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

/// Entry point: set up the terminal, run the event loop, restore terminal.
pub fn run(root: CtxforgeRoot) -> Result<()> {
    // Set up terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run.
    let result = run_loop(&mut terminal, root);

    // Restore terminal no matter what.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    root: CtxforgeRoot,
) -> Result<()> {
    let mut app = App::new(root);

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Some(key) = events::poll() {
            events::handle(&mut app, key);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
