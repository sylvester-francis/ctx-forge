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

        /// Output format. Defaults to `markdown`. Accepts `markdown`, `md`,
        /// `xml`, `json`.
        #[arg(long)]
        format: Option<String>,

        /// Shortcut for `--format xml`.
        #[arg(long, conflicts_with = "format")]
        xml: bool,

        /// Shortcut for `--format json`.
        #[arg(long, conflicts_with_all = ["format", "xml"])]
        json: bool,

        /// Do not auto-attach memory notes to the export.
        #[arg(long)]
        no_memory: bool,

        /// Only include notes with this tag.
        #[arg(long)]
        memory_tag: Option<String>,

        /// Maximum number of notes to attach.
        #[arg(long, default_value_t = 10)]
        memory_limit: usize,
    },

    /// Copy the current bundle to the system clipboard.
    Copy {
        /// Output format. Defaults to `markdown`.
        #[arg(long)]
        format: Option<String>,

        /// Shortcut for `--format xml`.
        #[arg(long, conflicts_with = "format")]
        xml: bool,

        /// Shortcut for `--format json`.
        #[arg(long, conflicts_with_all = ["format", "xml"])]
        json: bool,

        /// Do not auto-attach memory notes.
        #[arg(long)]
        no_memory: bool,

        /// Only include notes with this tag.
        #[arg(long)]
        memory_tag: Option<String>,

        /// Maximum number of notes to attach.
        #[arg(long, default_value_t = 10)]
        memory_limit: usize,
    },

    /// Save the current bundle as a named profile.
    Save { name: String },

    /// Load a named profile as the current bundle.
    Load { name: String },

    /// List or remove profiles.
    Profiles {
        #[command(subcommand)]
        action: Option<ProfilesAction>,
    },

    /// Write a memory note. Accumulates across sessions.
    Note {
        /// Tag for the note (e.g. "auth", "tls"). Untagged notes go to
        /// `decisions.md`.
        #[arg(long)]
        tag: Option<String>,

        /// The note body. Remaining args are joined with spaces.
        body: Vec<String>,
    },

    /// Show memory notes, filtered.
    Recall {
        /// Only notes with this tag.
        #[arg(long)]
        tag: Option<String>,

        /// Only notes whose body or tag contains this string (case-insensitive).
        #[arg(long)]
        search: Option<String>,

        /// Only notes newer than this duration (e.g. `1w`, `3d`, `12h`, `30m`).
        #[arg(long)]
        since: Option<String>,

        /// Maximum number of notes to show.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Show the current bundle plus recent memory notes — "pick up where
    /// you left off".
    Resume {
        /// Maximum number of recent notes to show.
        #[arg(long, default_value_t = 5)]
        memory_limit: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfilesAction {
    /// Remove a profile by name.
    Rm { name: String },
}
