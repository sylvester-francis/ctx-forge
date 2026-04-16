use ctxforge::cli::Cli;
use ctxforge::error::{CtxforgeError, Result};
use ctxforge::{commands, output, paths};

#[cfg(feature = "tui")]
use ctxforge::tui;
#[cfg(feature = "tui-v2")]
use ctxforge::tui2;
#[cfg(any(feature = "tui", feature = "tui-v2"))]
use std::io::IsTerminal;

use clap::Parser;

fn main() {
    if let Err(err) = run() {
        match &err {
            CtxforgeError::NotFound { path, suggestions } => {
                let cwd = std::env::current_dir().unwrap_or_default();
                let resolved = cwd.join(path);
                output::error_with_suggestion(
                    "file not found",
                    path,
                    &resolved,
                    suggestions,
                    Some("Use `ctxforge status` to list files already in the bundle."),
                );
            }
            _ => {
                output::error(&format!("{err}"));
            }
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // No subcommand + TTY → launch TUI (if any feature enabled).
    #[cfg(any(feature = "tui", feature = "tui-v2"))]
    if cli.command.is_none() && std::io::stdin().is_terminal() {
        let cwd = std::env::current_dir()?;
        let root = paths::CtxforgeRoot::find_or_create(&cwd)?;

        // Dispatch precedence: CLI flag → env var → default (v1).
        let env_v2 = std::env::var("CTXFORGE_TUI").ok().as_deref() == Some("v2");
        let env_v1 = std::env::var("CTXFORGE_TUI").ok().as_deref() == Some("v1");
        let want_v2 = cli.tui_v2 || (env_v2 && !cli.tui_v1);
        let want_v1 = cli.tui_v1 || env_v1;

        if want_v2 {
            #[cfg(feature = "tui-v2")]
            return tui2::run(root);
            #[cfg(not(feature = "tui-v2"))]
            {
                output::error("tui-v2 feature not enabled in this build");
                std::process::exit(1);
            }
        }

        // Default: v1. Falls through to v1 if no flag set.
        let _ = want_v1;
        #[cfg(feature = "tui")]
        return tui::run(root);
        #[cfg(not(feature = "tui"))]
        {
            output::error("tui feature not enabled in this build");
            std::process::exit(1);
        }
    }

    commands::dispatch(cli)
}
