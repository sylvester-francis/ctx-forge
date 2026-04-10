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

/// Entry point: uses ratatui 0.30's `run()` convenience function which
/// handles terminal init, alternate screen, raw mode, and guaranteed
/// restore (even on panic) automatically.
pub fn run(root: CtxforgeRoot) -> Result<()> {
    ratatui::run(|terminal| run_loop(terminal, root))?;
    Ok(())
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, root: CtxforgeRoot) -> Result<()> {
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
