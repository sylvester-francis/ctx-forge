//! Interactive TUI composer.
//!
//! Run with `ctxforge` (no subcommand) when stdin is a TTY.
//! Two-panel layout: file tree (left) + bundle list (right), with a live
//! token gauge and keybinding footer.

pub mod app;
pub mod events;
pub mod mode;
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

        // Drain any pending stdout export (e.g. `x` key). We restore the
        // terminal, print, wait for a keypress, then re-init a fresh terminal
        // and restart the draw loop.
        if let Some(content) = app.pending_stdout.take() {
            ratatui::restore();
            println!("{content}");
            eprintln!("\nPress any key to return to ctxforge...");
            let _ = crossterm::event::read();
            *terminal = ratatui::init();
            continue;
        }

        // Drain any pending pipe (e.g. `p` menu choice). Same dance, but
        // spawn the target binary and pipe content into its stdin.
        if let Some((target, content)) = app.pending_pipe.take() {
            ratatui::restore();
            match std::process::Command::new(&target)
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(content.as_bytes());
                    }
                    let _ = child.wait();
                    app.status_message = format!("Piped to {target}");
                }
                Err(e) => {
                    eprintln!("Failed to start `{target}`: {e}");
                    eprintln!("Press any key to return...");
                    let _ = crossterm::event::read();
                }
            }
            *terminal = ratatui::init();
            continue;
        }

        if let Some(key) = events::poll() {
            events::handle(&mut app, key);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
