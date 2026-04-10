//! Subcommand handlers. Each handler is a thin function that reads the
//! current bundle, modifies it, saves, and prints user-facing output.

pub mod add;
pub mod clear;
pub mod copy;
pub mod export;
pub mod load;
pub mod note;
pub mod pipe;
pub mod profiles;
pub mod recall;
pub mod resume;
pub mod rm;
pub mod save;
pub mod status;

use crate::cli::{Cli, Command};
use crate::error::{CtxforgeError, Result};
use crate::format::Format;
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
            format,
            xml,
            json,
            no_memory,
            memory_tag,
            memory_limit,
        }) => {
            let fmt = resolve_format(format.as_deref(), xml, json)?;
            export::run(&root, output, fmt, no_memory, memory_tag, memory_limit)
        }
        Some(Command::Copy {
            format,
            xml,
            json,
            no_memory,
            memory_tag,
            memory_limit,
        }) => {
            let fmt = resolve_format(format.as_deref(), xml, json)?;
            copy::run(&root, fmt, no_memory, memory_tag, memory_limit)
        }
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
        Some(Command::Pipe {
            target,
            format,
            no_memory,
            memory_tag,
            memory_limit,
            extra_args,
        }) => pipe::run(
            &root,
            &target,
            format.as_deref(),
            no_memory,
            memory_tag,
            memory_limit,
            &extra_args,
        ),
        Some(Command::Mcp) => crate::mcp::run(root),
    }
}

/// Resolves the effective format from the `--format` string and the
/// `--xml` / `--json` shortcut flags. The clap `conflicts_with` rules
/// ensure at most one of the three is set.
fn resolve_format(format: Option<&str>, xml: bool, json: bool) -> Result<Format> {
    if xml {
        return Ok(Format::Xml);
    }
    if json {
        return Ok(Format::Json);
    }
    match format {
        Some(name) => Format::parse(name).ok_or_else(|| {
            CtxforgeError::Msg(format!(
                "invalid --format `{name}` (expected one of: markdown, md, xml, json)"
            ))
        }),
        None => Ok(Format::default()),
    }
}
