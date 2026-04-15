//! Interactive TUI composer.
//!
//! Run with `ctxforge` (no subcommand) when stdin is a TTY.
//! Two-panel layout: file tree (left) + bundle list (right), with a live
//! token gauge and keybinding footer.

pub mod app;
pub mod commands;
pub mod events;
pub mod mode;
pub mod motion;
pub mod preview;
pub mod prompt_input;
pub mod scenario;
pub mod theme;
pub mod tree;
pub mod ui;
pub mod viewer;

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
    app.auto_open_scenario_picker_if_needed();

    // Opt into bracketed paste so pasted content arrives as a single
    // Event::Paste(String) rather than as a burst of key events. The
    // prompt input inserts the whole string atomically without, for
    // example, triggering slash-command or @-picker handlers on
    // characters that happen to appear inside the paste.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste);

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;
        app.cleanup_finished_animations();
        app.tick_status_fade();

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
                    app.set_status(format!("Piped to {target}"));
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

        // Animation-aware timeout: ~60fps while animating, block on input
        // when idle, or the time until the next scheduled event (e.g.
        // status fade-out trigger) otherwise.
        let timeout = app.next_wake_delay();

        if let Some(ev) = events::poll_event_with_timeout(timeout) {
            use crossterm::event::Event;
            match ev {
                Event::Key(k) => events::handle(&mut app, k),
                Event::Mouse(m) => events::handle_mouse(&mut app, m),
                Event::Paste(s) => events::handle_paste(&mut app, s),
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Persist any in-flight task text / scenario changes before returning.
    // Mutations to the bundle already save on their own; this is the
    // last-chance save for task text (which only syncs in memory during
    // typing) so a Ctrl-C quit doesn't drop the user's work.
    let _ = app.bundle.save(&app.root);

    // Before ratatui::run()'s drop-time restore runs, explicitly disable
    // anything we enabled outside ratatui's knowledge:
    //
    // - SGR mouse capture (toggled on by the code viewer). If left on,
    //   terminals keep emitting `0;96;38M` style bytes to the shell
    //   after the TUI exits.
    // - Bracketed paste. Leaving it on would cause the shell to receive
    //   `[200~...[201~` framing around pastes, which most shells don't
    //   interpret.
    //
    // Both calls are idempotent; safe regardless of what was enabled
    // during the session.
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
    );

    Ok(())
}
