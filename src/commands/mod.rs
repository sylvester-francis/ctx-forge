//! Subcommand handlers. Each handler is a thin function that reads the
//! current bundle, modifies it, saves, and prints user-facing output.

pub mod add;
pub mod clear;
pub mod copy;
pub mod export;
pub mod load;
pub mod note;
pub mod profiles;
pub mod recall;
pub mod resume;
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
        Some(Command::Export {
            output,
            no_memory,
            memory_tag,
            memory_limit,
        }) => export::run(&root, output, no_memory, memory_tag, memory_limit),
        Some(Command::Copy {
            no_memory,
            memory_tag,
            memory_limit,
        }) => copy::run(&root, no_memory, memory_tag, memory_limit),
        Some(Command::Save { name }) => save::run(&root, &name),
        Some(Command::Load { name }) => load::run(&root, &name),
        Some(Command::Profiles { action }) => profiles::run(&root, action),
        Some(Command::Note { tag, body }) => note::run(&root, body, tag),
        Some(Command::Recall {
            tag,
            search,
            since,
            limit,
        }) => recall::run(&root, tag, search, since, limit),
        Some(Command::Resume { memory_limit }) => resume::run(&root, memory_limit),
    }
}
