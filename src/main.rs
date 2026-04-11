mod bundle;
mod cli;
mod clipboard;
mod commands;
mod error;
#[cfg(feature = "extract")]
mod extract;
mod format;
mod git;
mod lang;
#[cfg(feature = "mcp")]
mod mcp;
mod memory;
mod models;
mod output;
mod paths;
mod profile;
mod resolve;
mod tokens;
#[cfg(feature = "tui")]
mod tui;
mod walk;

#[cfg(feature = "tui")]
use std::io::IsTerminal;

use clap::Parser;
use cli::Cli;
use error::Result;

fn main() {
    if let Err(err) = run() {
        match &err {
            error::CtxforgeError::NotFound { path, suggestions } => {
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

    // No subcommand + TTY → launch TUI (if feature enabled).
    #[cfg(feature = "tui")]
    if cli.command.is_none() && std::io::stdin().is_terminal() {
        let cwd = std::env::current_dir()?;
        let root = paths::CtxforgeRoot::find_or_create(&cwd)?;
        return tui::run(root);
    }

    commands::dispatch(cli)
}
