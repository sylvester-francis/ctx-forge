//! Subcommand handlers. Each handler is a thin function that reads the
//! current bundle, modifies it, saves, and prints user-facing output.

pub mod add;
pub mod clear;
pub mod copy;
pub mod export;
pub mod load;
pub mod profiles;
pub mod rm;
pub mod save;
pub mod status;

use crate::cli::{Cli, Command};
use crate::error::Result;
use crate::paths::CtxforgeRoot;

pub fn dispatch(cli: Cli) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = CtxforgeRoot::find_or_create(&cwd)?;
    let model_override = cli.model.clone();

    match cli.command {
        None => {
            // v0.1: no TUI — print help.
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
        Some(Command::Add {
            patterns,
            exclude,
            diff,
        }) => add::run(&root, &cwd, patterns, exclude, diff),
        Some(Command::Rm { target }) => rm::run(&root, target),
        Some(Command::Clear) => clear::run(&root),
        Some(Command::Status) => status::run(&root, model_override.as_deref()),
        Some(Command::Export { output }) => export::run(&root, output),
        Some(Command::Copy) => copy::run(&root),
        Some(Command::Save { name }) => save::run(&root, &name),
        Some(Command::Load { name }) => load::run(&root, &name),
        Some(Command::Profiles { action }) => profiles::run(&root, action),
    }
}
