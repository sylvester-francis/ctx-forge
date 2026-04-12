//! Static command table and fuzzy-matched filter for the TUI slash palette.
//!
//! Each command has a name, description, takes_arg flag, and an action
//! function that's invoked when the user selects it from the palette.
//! The action function receives a `&mut App` and an `Option<String>`
//! containing the inline argument (if any).

#![allow(dead_code)]

use crate::tui::app::App;
use crate::tui::mode;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// Static spec for a slash command.
pub struct CommandSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub takes_arg: bool,
    pub action: fn(&mut App, Option<String>),
}

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "copy",
        description: "copy bundle to clipboard (markdown)",
        takes_arg: false,
        action: |app, _| app.copy_to_clipboard(),
    },
    CommandSpec {
        name: "copy-xml",
        description: "copy bundle as XML to clipboard",
        takes_arg: false,
        action: |app, _| app.set_status("copy as XML — use /export-xml"),
    },
    CommandSpec {
        name: "copy-json",
        description: "copy bundle as JSON to clipboard",
        takes_arg: false,
        action: |app, _| app.set_status("copy as JSON — use CLI: ctxforge copy --json"),
    },
    CommandSpec {
        name: "export",
        description: "export bundle to stdout (markdown)",
        takes_arg: false,
        action: |app, _| app.set_status("export markdown — use /export-xml for XML"),
    },
    CommandSpec {
        name: "export-xml",
        description: "export bundle as XML to stdout",
        takes_arg: false,
        action: |app, _| app.export_xml_to_stdout(),
    },
    CommandSpec {
        name: "export-json",
        description: "export bundle as JSON to stdout",
        takes_arg: false,
        action: |app, _| app.set_status("export as JSON — use CLI: ctxforge export --json"),
    },
    CommandSpec {
        name: "pipe",
        description: "pipe to agent (claude / agent / gemini)",
        takes_arg: false,
        action: |app, _| {
            app.mode = mode::Mode::PipeMenu;
        },
    },
    CommandSpec {
        name: "save",
        description: "save current bundle as named profile <name?>",
        takes_arg: true,
        action: |app, arg| match arg {
            Some(n) if !n.is_empty() => app.save_profile(&n),
            _ => {
                app.mode = mode::Mode::SaveProfile {
                    name: String::new(),
                };
            }
        },
    },
    CommandSpec {
        name: "load",
        description: "load a profile <name?>",
        takes_arg: true,
        action: |app, _| app.start_load_profile(),
    },
    CommandSpec {
        name: "narrow",
        description: "narrow current item to a line range",
        takes_arg: false,
        action: |app, _| app.start_narrow(),
    },
    CommandSpec {
        name: "model",
        description: "switch target model <name?>",
        takes_arg: true,
        action: |app, _| {
            app.mode = mode::Mode::ModelSwitch { cursor: 0 };
        },
    },
    CommandSpec {
        name: "memory",
        description: "toggle memory recall panel",
        takes_arg: false,
        action: |app, _| app.toggle_memory_panel(),
    },
    CommandSpec {
        name: "note",
        description: "add inline memory note",
        takes_arg: false,
        action: |app, _| {
            app.mode = mode::Mode::AddNote {
                tag: String::new(),
                body: String::new(),
                field: mode::InputField::First,
            };
        },
    },
    CommandSpec {
        name: "find",
        description: "fuzzy file tree search <query?>",
        takes_arg: true,
        action: |app, arg| {
            let query = arg.unwrap_or_default();
            app.mode = mode::Mode::Search {
                query: query.clone(),
            };
            app.run_search(&query);
        },
    },
    CommandSpec {
        name: "find-fn",
        description: "function picker (requires --features=extract)",
        takes_arg: false,
        action: |app, _| {
            #[cfg(feature = "extract")]
            app.start_function_pick();
            #[cfg(not(feature = "extract"))]
            app.set_status("function picker requires --features=extract");
        },
    },
    CommandSpec {
        name: "find-type",
        description: "type picker (requires --features=extract)",
        takes_arg: false,
        action: |app, _| {
            #[cfg(feature = "extract")]
            app.start_type_pick();
            #[cfg(not(feature = "extract"))]
            app.set_status("type picker requires --features=extract");
        },
    },
    CommandSpec {
        name: "find-diff",
        description: "diff picker against branch <branch?>",
        takes_arg: true,
        action: |app, _| app.start_diff_pick(),
    },
    CommandSpec {
        name: "template",
        description: "pick template + task -> copy <name?>",
        takes_arg: true,
        action: |app, arg| app.start_template_flow(arg),
    },
    CommandSpec {
        name: "template-new",
        description: "scaffold a new project template <name>",
        takes_arg: true,
        action: |app, arg| app.run_template_new(arg),
    },
    CommandSpec {
        name: "template-rm",
        description: "delete a project template <name>",
        takes_arg: true,
        action: |app, arg| app.run_template_rm(arg),
    },
    CommandSpec {
        name: "template-starters",
        description: "list built-in starter templates",
        takes_arg: false,
        action: |app, _| app.run_template_starters(),
    },
    CommandSpec {
        name: "template-list",
        description: "show available templates",
        takes_arg: false,
        action: |app, _| app.show_template_list(),
    },
    CommandSpec {
        name: "help",
        description: "show navigation help overlay",
        takes_arg: false,
        action: |app, _| {
            app.show_help = !app.show_help;
        },
    },
    CommandSpec {
        name: "quit",
        description: "quit ctxforge",
        takes_arg: false,
        action: |app, _| app.should_quit = true,
    },
];

/// Fuzzy-matched filter over the command table. Returns commands ranked
/// by skim-style fuzzy match score (best first). Empty query returns
/// all commands in their declaration order.
pub fn fuzzy_filter(query: &str) -> Vec<&'static CommandSpec> {
    if query.is_empty() {
        return COMMANDS.iter().collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &'static CommandSpec)> = COMMANDS
        .iter()
        .filter_map(|cmd| matcher.fuzzy_match(cmd.name, query).map(|s| (s, cmd)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, c)| c).collect()
}

/// Parses a palette query string of the form `command-name [arg]` into
/// the command and an optional argument.
pub fn parse_palette_query(query: &str) -> (String, Option<String>) {
    let trimmed = query.trim_start_matches('/').trim_start();
    match trimmed.split_once(' ') {
        Some((cmd, arg)) => (cmd.to_string(), Some(arg.trim().to_string())),
        None => (trimmed.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_filter_matches_prefix() {
        let results = fuzzy_filter("cop");
        assert!(results.iter().any(|c| c.name == "copy"));
    }

    #[test]
    fn fuzzy_filter_matches_substring() {
        let results = fuzzy_filter("template");
        assert!(results.iter().any(|c| c.name == "template"));
    }

    #[test]
    fn fuzzy_filter_empty_query_returns_all() {
        let results = fuzzy_filter("");
        assert_eq!(results.len(), COMMANDS.len());
    }

    #[test]
    fn fuzzy_filter_no_match_returns_empty() {
        let results = fuzzy_filter("xyzqwertyabcdef");
        assert!(results.is_empty());
    }

    #[test]
    fn command_table_has_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for cmd in COMMANDS {
            assert!(seen.insert(cmd.name), "duplicate command: {}", cmd.name);
        }
    }

    #[test]
    fn command_table_has_at_least_21_commands() {
        assert!(
            COMMANDS.len() >= 21,
            "expected >= 21 commands, got {}",
            COMMANDS.len()
        );
    }

    #[test]
    fn command_table_takes_arg_consistency() {
        // Commands marked takes_arg should have <name?> or similar in description.
        for cmd in COMMANDS {
            if cmd.takes_arg {
                assert!(
                    cmd.description.contains('<') || cmd.description.contains('('),
                    "command {} marked takes_arg but description doesn't hint at it: {}",
                    cmd.name,
                    cmd.description
                );
            }
        }
    }
}
