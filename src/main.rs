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
mod walk;

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
    commands::dispatch(cli)
}
