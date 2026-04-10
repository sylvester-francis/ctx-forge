mod bundle;
mod cli;
mod clipboard;
mod commands;
mod error;
mod format;
mod git;
mod lang;
mod memory;
mod models;
mod paths;
mod profile;
mod resolve;
mod tokens;
mod tui;
mod walk;

use std::io::IsTerminal;

use clap::Parser;
use cli::Cli;
use error::Result;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // No subcommand + TTY → launch TUI.
    if cli.command.is_none() && std::io::stdin().is_terminal() {
        let cwd = std::env::current_dir()?;
        let root = paths::CtxforgeRoot::find_or_create(&cwd)?;
        return tui::run(root);
    }

    commands::dispatch(cli)
}
