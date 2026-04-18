//! v2 command registry. Mirrors v1's `crate::tui::commands::COMMANDS` at
//! the discovery layer (all 29 commands appear in the palette with the
//! same names + descriptions) but dispatches through a v2-native enum so
//! action implementations live on `AppData` instead of v1's `App`.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    // ── Already implemented in v2 ──
    Help,
    Scenario,
    Quit,
    Find,

    // ── Phase 2c-3 overlays (status message until then) ──
    Theme,
    Deliver,
    EditPrompt,
    ToggleViewer,
    AddSelection,

    // ── Delivery + bundle/profile/template actions (all functional) ──
    Copy,
    CopyXml,
    CopyJson,
    Export,
    ExportXml,
    ExportJson,
    Pipe,
    SaveProfile,
    LoadProfile,
    Narrow,
    Model,
    Memory,
    Note,
    FindFn,
    FindType,
    FindDiff,
    Template,
    TemplateNew,
    TemplateRm,
    TemplateStarters,
    TemplateList,

    // ── P5/P1 parity: real handlers, no input required ──
    DocsDetect,
    DocsDetectAll,
    DocsRefresh,
    DocsList,
    UrlRefresh,
    CacheList,
    CacheClear,
    CacheVerify,

    // ── Input-required actions (text prompt overlay) ──
    DocsAdd,
    DocsRm,
    AddUrl,
    AddGh,

    // ── P4: auto-suggest ──
    Suggest,
    SuggestApplyAll,

    // ── Bundle CRUD parity with CLI (`rm`, `clear`, `list sources`) ──
    BundleRm,
    BundleClear,
    ListSources,

    // ── Prompt override management ──
    ClearPromptOverride,
}

pub struct CommandSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub action: CommandAction,
}

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "copy",
        description: "copy bundle to clipboard (markdown)",
        action: CommandAction::Copy,
    },
    CommandSpec {
        name: "copy-xml",
        description: "copy bundle as XML to clipboard",
        action: CommandAction::CopyXml,
    },
    CommandSpec {
        name: "copy-json",
        description: "copy bundle as JSON to clipboard",
        action: CommandAction::CopyJson,
    },
    CommandSpec {
        name: "export",
        description: "export bundle to stdout (markdown)",
        action: CommandAction::Export,
    },
    CommandSpec {
        name: "export-xml",
        description: "export bundle as XML to stdout",
        action: CommandAction::ExportXml,
    },
    CommandSpec {
        name: "export-json",
        description: "export bundle as JSON to stdout",
        action: CommandAction::ExportJson,
    },
    CommandSpec {
        name: "pipe",
        description: "pipe to agent (claude / agent / gemini)",
        action: CommandAction::Pipe,
    },
    CommandSpec {
        name: "save",
        description: "save current bundle as named profile",
        action: CommandAction::SaveProfile,
    },
    CommandSpec {
        name: "load",
        description: "load a profile",
        action: CommandAction::LoadProfile,
    },
    CommandSpec {
        name: "narrow",
        description: "narrow current item to a line range",
        action: CommandAction::Narrow,
    },
    CommandSpec {
        name: "model",
        description: "switch target model",
        action: CommandAction::Model,
    },
    CommandSpec {
        name: "memory",
        description: "toggle memory recall panel",
        action: CommandAction::Memory,
    },
    CommandSpec {
        name: "note",
        description: "add inline memory note",
        action: CommandAction::Note,
    },
    CommandSpec {
        name: "find",
        description: "fuzzy file tree search",
        action: CommandAction::Find,
    },
    CommandSpec {
        name: "find-fn",
        description: "function picker (requires --features=extract)",
        action: CommandAction::FindFn,
    },
    CommandSpec {
        name: "find-type",
        description: "type picker (requires --features=extract)",
        action: CommandAction::FindType,
    },
    CommandSpec {
        name: "find-diff",
        description: "diff picker against branch",
        action: CommandAction::FindDiff,
    },
    CommandSpec {
        name: "template",
        description: "pick template + task -> copy",
        action: CommandAction::Template,
    },
    CommandSpec {
        name: "template-new",
        description: "scaffold a new project template",
        action: CommandAction::TemplateNew,
    },
    CommandSpec {
        name: "template-rm",
        description: "delete a project template",
        action: CommandAction::TemplateRm,
    },
    CommandSpec {
        name: "template-starters",
        description: "list built-in starter templates",
        action: CommandAction::TemplateStarters,
    },
    CommandSpec {
        name: "template-list",
        description: "show available templates",
        action: CommandAction::TemplateList,
    },
    CommandSpec {
        name: "help",
        description: "show navigation help overlay",
        action: CommandAction::Help,
    },
    CommandSpec {
        name: "view",
        description: "toggle code viewer pane",
        action: CommandAction::ToggleViewer,
    },
    CommandSpec {
        name: "add-selection",
        description: "add the viewer's selected lines to the bundle",
        action: CommandAction::AddSelection,
    },
    CommandSpec {
        name: "theme",
        description: "switch the active theme",
        action: CommandAction::Theme,
    },
    CommandSpec {
        name: "scenario",
        description: "switch the active scenario",
        action: CommandAction::Scenario,
    },
    CommandSpec {
        name: "deliver",
        description: "deliver the crafted prompt (pipe / copy / export)",
        action: CommandAction::Deliver,
    },
    CommandSpec {
        name: "edit-prompt",
        description: "edit the full composed prompt in $EDITOR",
        action: CommandAction::EditPrompt,
    },
    CommandSpec {
        name: "quit",
        description: "quit ctxforge",
        action: CommandAction::Quit,
    },
    CommandSpec {
        name: "docs detect",
        description: "detect project deps and attach doc URLs",
        action: CommandAction::DocsDetect,
    },
    CommandSpec {
        name: "docs detect --all",
        description: "detect all direct deps including Library tier",
        action: CommandAction::DocsDetectAll,
    },
    CommandSpec {
        name: "docs refresh",
        description: "re-read lock files; update versions on existing docs items",
        action: CommandAction::DocsRefresh,
    },
    CommandSpec {
        name: "docs list",
        description: "list docs items currently attached to the bundle",
        action: CommandAction::DocsList,
    },
    CommandSpec {
        name: "url refresh",
        description: "force-refresh all stale cached URL sources",
        action: CommandAction::UrlRefresh,
    },
    CommandSpec {
        name: "cache list",
        description: "list cached URL responses",
        action: CommandAction::CacheList,
    },
    CommandSpec {
        name: "cache clear",
        description: "clear all cached URL responses",
        action: CommandAction::CacheClear,
    },
    CommandSpec {
        name: "cache verify",
        description: "verify cache integrity (SHA + HMAC)",
        action: CommandAction::CacheVerify,
    },
    CommandSpec {
        name: "docs add",
        description: "manually attach a dep's doc URL by name",
        action: CommandAction::DocsAdd,
    },
    CommandSpec {
        name: "docs rm",
        description: "remove a docs item from the bundle by name",
        action: CommandAction::DocsRm,
    },
    CommandSpec {
        name: "url add",
        description: "attach an arbitrary URL to the bundle",
        action: CommandAction::AddUrl,
    },
    CommandSpec {
        name: "github attach",
        description: "attach a GitHub issue / PR / release / file by gh:// URI",
        action: CommandAction::AddGh,
    },
    CommandSpec {
        name: "suggest",
        description: "list missing + stale deps; pick which to apply",
        action: CommandAction::Suggest,
    },
    CommandSpec {
        name: "suggest apply all",
        description: "apply every missing/stale suggestion without asking",
        action: CommandAction::SuggestApplyAll,
    },
    CommandSpec {
        name: "rm",
        description: "remove a bundle item by path or 1-based index",
        action: CommandAction::BundleRm,
    },
    CommandSpec {
        name: "clear",
        description: "remove every item from the bundle",
        action: CommandAction::BundleClear,
    },
    CommandSpec {
        name: "list sources",
        description: "inspect cached URL / gh:// entries with freshness",
        action: CommandAction::ListSources,
    },
    CommandSpec {
        name: "clear prompt override",
        description: "drop the hand-edited prompt and resume rendering from the bundle",
        action: CommandAction::ClearPromptOverride,
    },
];

/// Fuzzy-filter commands by query. Empty query returns all commands in
/// declaration order. Non-empty query ranks by SkimMatcherV2 score.
pub fn filter(query: &str) -> Vec<&'static CommandSpec> {
    if query.is_empty() {
        return COMMANDS.iter().collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &'static CommandSpec)> = COMMANDS
        .iter()
        .filter_map(|cmd| matcher.fuzzy_match(cmd.name, query).map(|s| (s, cmd)))
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    scored.into_iter().map(|(_, c)| c).collect()
}
