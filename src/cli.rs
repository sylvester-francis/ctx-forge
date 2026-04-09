//! Clap command definitions.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ctxforge", version, about = "Assemble context bundles for AI coding.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Target model for token counting.
    #[arg(long, global = true)]
    pub model: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Add files, globs, or ranges to the current bundle.
    Add {
        /// One or more path patterns (globs supported).
        patterns: Vec<String>,

        /// Exclude patterns (applied after expansion).
        #[arg(long)]
        exclude: Vec<String>,

        /// Add files changed vs. this branch.
        #[arg(long)]
        diff: Option<String>,
    },

    /// Remove an item by index (1-based) or path.
    Rm {
        /// Index or path.
        target: String,
    },

    /// Remove all items from the current bundle.
    Clear,

    /// Show bundle contents with token counts and percentages.
    Status,

    /// Export the current bundle to stdout or a file.
    Export {
        /// Write to file instead of stdout.
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Copy the current bundle to the system clipboard.
    Copy,

    /// Save the current bundle as a named profile.
    Save { name: String },

    /// Load a named profile as the current bundle.
    Load { name: String },

    /// List or remove profiles.
    Profiles {
        #[command(subcommand)]
        action: Option<ProfilesAction>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfilesAction {
    /// Remove a profile by name.
    Rm { name: String },
}
