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

        /// Extract a specific function by name (requires --features=extract).
        /// Usage: ctxforge add --fn ProcessCheck src/hub/check.go
        #[arg(long = "fn")]
        function: Vec<String>,

        /// Extract a specific type/struct/interface by name (requires --features=extract).
        /// Usage: ctxforge add --type Config src/config/config.go
        #[arg(long = "type")]
        type_name: Vec<String>,
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

        /// Wrap the rendered bundle inside a named template.
        #[arg(long)]
        template: Option<String>,

        /// Task description for `{{task}}` substitution.
        #[arg(long)]
        task: Option<String>,
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

        /// Wrap the rendered bundle inside a named template.
        #[arg(long)]
        template: Option<String>,

        /// Task description for `{{task}}` substitution.
        #[arg(long)]
        task: Option<String>,
    },

    /// Save the current bundle as a named profile.
    ///
    /// If no name is provided and stdin is a TTY, you'll be prompted.
    Save {
        /// Profile name. Omit to prompt interactively (requires a TTY).
        name: Option<String>,
    },

    /// Load a named profile as the current bundle.
    Load { name: String },

    /// List or remove profiles.
    Profiles {
        #[command(subcommand)]
        action: Option<ProfilesAction>,
    },

    /// Manage prompt templates (`.ctxforge/templates/*.md`).
    ///
    /// Templates wrap a rendered bundle with author-written prose containing
    /// `{{bundle}}` and `{{task}}` placeholders. Use them with
    /// `ctxforge copy --template <name> --task "..."`.
    Templates {
        #[command(subcommand)]
        action: Option<TemplatesAction>,
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

    /// Pipe the current bundle to a local agent CLI via stdin.
    ///
    /// Known targets auto-select the best format:
    ///   claude → XML (Claude-optimized semantic tags)
    ///   agent  → markdown (Cursor CLI)
    ///   gemini → markdown
    ///
    /// Any other name is treated as a binary in your PATH (markdown default).
    /// Pass extra args to the target CLI after `--`.
    Pipe {
        /// Target agent CLI name (e.g. `claude`, `agent`, `gemini`).
        target: String,

        /// Override the auto-selected format.
        #[arg(long)]
        format: Option<String>,

        /// Do not auto-attach memory notes.
        #[arg(long)]
        no_memory: bool,

        /// Only include notes with this tag.
        #[arg(long)]
        memory_tag: Option<String>,

        /// Maximum number of notes to attach.
        #[arg(long, default_value_t = 10)]
        memory_limit: usize,

        /// Wrap the rendered bundle inside a named template.
        #[arg(long)]
        template: Option<String>,

        /// Task description for `{{task}}` substitution.
        #[arg(long)]
        task: Option<String>,

        /// Extra arguments passed to the target CLI after `--`.
        #[arg(last = true)]
        extra_args: Vec<String>,
    },

    /// Start the MCP (Model Context Protocol) server (protocol 2025-03-26).
    ///
    /// Reads JSON-RPC requests from stdin, writes responses to stdout.
    /// Install for Claude Code:
    ///   claude mcp add --transport stdio ctxforge -- ctxforge mcp
    #[cfg(feature = "mcp")]
    Mcp,
}

#[derive(Subcommand, Debug)]
pub enum ProfilesAction {
    /// Remove a profile by name.
    Rm { name: String },
}

#[derive(Subcommand, Debug)]
pub enum TemplatesAction {
    /// Scaffold a new project-local template. With `--from <starter>`,
    /// copy from a built-in starter; otherwise create a blank scaffold.
    New {
        name: String,
        /// Copy from a built-in starter library template.
        /// Available: bugfix, code-review, explain, refactor, migrate.
        #[arg(long)]
        from: Option<String>,
    },
    /// Delete a project-local template by name.
    Rm { name: String },
    /// List the built-in starter templates shipped with ctxforge.
    Starters,
}
