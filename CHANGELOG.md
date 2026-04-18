# Changelog

## [2.0.0](https://github.com/sylvester-francis/ctx-forge/compare/v1.3.0...v2.0.0) - 2026-04-18

### Added

- *(tui)* /rm + /clear + /list-sources palette entries for CLI parity
- *(mcp)* add 8 tools for full CLI parity; tool count 23 → 31
- *(tui)* suggest + suggest-apply-all palette + multi-select picker
- *(mcp)* suggest + suggest_apply tools; tool count 21 → 23
- *(cli)* ctxforge suggest subcommand with --all/--apply/--json/--yes
- *(suggest)* apply_suggestion wraps docs_cmd add/rm
- *(suggest)* run_suggest orchestration + comparison logic
- *(suggest)* report types + human/JSON renderers
- *(suggest)* name canonicalisation — hyphen/underscore + case
- *(suggest)* go import scanner handling single + block forms
- *(suggest)* python import scanner + dotted path + relative-skip
- *(suggest)* JS/TS import scanner + scoped/deep normalisation
- *(suggest)* rust import scanner + tests
- *(suggest)* per-ecosystem stdlib blocklists
- *(suggest)* module skeleton + regex dep
- *(tui)* github attach palette entry + AddGh text prompt
- *(cli+mcp)* accept gh:// URIs in add pipeline
- *(format/json+xml)* forge sub-object/element; Source::Gh path rendering
- *(format/md)* Project stack emits forge URLs per dep; Source::Gh renders verbatim
- *(resolve)* dispatch Source::Gh through gh::fetch + cache
- *(gh)* markdown section renderer with utf8-safe body cap
- *(gh)* GitHub REST + raw content fetcher with GITHUB_TOKEN auth
- *(fetch)* FetchConfig::auth_token; Authorization: Bearer header when set
- *(bundle)* canonicalise pasted github.com URLs to gh:/// form
- *(gh)* gh:// URI parser + Source::from_uri dispatch
- *(source)* Source::Gh variant with 4 resource types + per-variant TTLs
- *(docs)* run_detect stores forge info on DocsSource items
- *(docs)* extract repository URL alongside description — no extra HTTP
- *(source)* DocsSource gains optional forge field (additive)
- *(gh)* ForgeHost + ForgeRef types + registry-URL parser
- *(tui)* persist edit-prompt output as delivery override; wipe all remaining TODOs
- *(tui)* full CLI/MCP parity via suspend-and-prompt pattern
- *(tui)* command palette entries for docs, url refresh, cache ops
- *(mcp)* ctxforge_docs_{detect,add,list} tools
- *(docs)* workspace-aware full-scan + attention-weight subsection ordering
- *(docs)* Go parser — go.mod direct deps + go.work workspace
- *(docs)* Python parser — pyproject.toml + uv/poetry/requirements.txt
- *(docs)* JS/TS parser — package.json + package-lock / pnpm lock support
- *(format)* Project stack section in markdown/json/xml
- *(cli)* ctxforge docs {detect,add,rm,list,refresh} subcommands
- *(docs)* run_detect orchestrator ties parsers/classifier/URL/cache + fix docs scheme
- *(docs)* detection modes (scoped/full-scan/explicit) + walk-up
- *(docs)* Cargo.toml + Cargo.lock parser with workspace inheritance
- *(docs)* registry description fetcher with truncate + cache
- *(docs)* canonical URL templates for Rust/JS/Python/Go
- *(docs)* classification registry with ~80 MVP entries across 4 ecosystems
- *(source)* Source::Docs variant with docs:/// URI + resolve arm
- *(source)* DocsSource + Ecosystem + DocsTier types
- *(tui)* freshness dots on bundle summary for cacheable sources
- *(mcp)* add_url, refresh, list_sources tools; URI-aware add_files
- *(cli)* ctxforge cache list/clear/verify and ctxforge refresh
- *(cli)* --strict/--offline/--no-provenance and --allow-http/--allow-private-net
- *(format)* render Provenance in md/json/xml with --no-provenance strip
- *(resolve)* ResolveCtx with mode-aware fetch pipeline and cache integration
- *(source)* URL validation with literal+resolved-IP SSRF guard and content-type allowlist
- *(source)* Provenance type for local + network audit trail
- *(cache)* content-addressable cache with sidecar HMAC and OS entropy
- *(source)* Source enum with File/Range/Func/Type/Url variants and URI conversion
- *(source)* Uri parser with RFC 3986 subset and security hardening

### Fixed

- *(gh)* use ? operator for scheme strip (clippy::question_mark)
- *(bundle)* accept bare gh:// URIs in parse_add_argument (CLI + MCP smoke gap)
- *(bundle)* migrate v1-shape items even when version header says 2
- *(clippy)* satisfy Rust 1.95 cmp_owned and unnecessary_sort_by lints

### Other

- rewrite README — installation-first, FAQ-driven, collapsible reference sections
- *(plugin)* plugin.json tool count 15 → 31
- *(plugin)* refresh plugin.json + skill ref table for 31 tools
- rewrite README covering P5/P1/P3/P4 + iocraft TUI rewrite
- P4 fmt polish + derive Default on SuggestOptions
- *(suggest)* integration across rust/js/python/go fixtures
- *(suggest)* fixture projects across all four ecosystems
- cargo fmt polish
- *(gh)* canned API fixtures + integration coverage + token-budget guard
- *(docs)* skip .worktrees/.claude/vendor in full_scan walk
- *(docs)* fixture projects + integration coverage + token-budget guard
- gitignore .ctxforge/bundle.json.v1.bak
- provenance off by default; mark HMAC + fetch pipeline as P5-scoped
- cargo fmt (wrap long matches! in assert! macros)
- cargo fmt
- security and migration regression suites
- migrate Bundle to v2 Source model across all consumers
- *(deps)* add ureq, hmac, getrandom, bytes for source abstraction
- gitignore spikes/ directory

## 1.3.0 — iocraft TUI rewrite

Complete TUI rewrite from ratatui (immediate-mode) to **iocraft** — a React-like reactive framework with taffy flexbox layout. The ratatui v1 TUI has been removed; iocraft is now the sole rendering engine. Every surface has been rebuilt with a distinctive design language, fluid interactions, and responsive layouts.

### Highlights

- **Reactive rendering.** Components re-render only when their state changes — no per-frame full-screen redraws. Drag-select, scroll, and focus transitions feel fluid.
- **Flexbox layout.** Panels scale with the terminal via taffy flexbox. Width-adaptive: two-column (≥100 cols), single-focused-panel (<100 cols), three-column when the code viewer is enabled.
- **Welcome splash.** A full-screen branded splash on every launch with a quick-start guide. Any key enters the app, `q` quits.
- **Distinctive design language.** `▍` left-bar section markers, `●/○` status dots, `▸/▾` directory markers, gradient token gauge (`█▓▒░`), nesting rails (`│`), block-letter wordmark.

### Added

- **iocraft reactive TUI.** Complete rewrite on iocraft 0.8 — React-like components with `use_state`, `use_future`, `use_animated` hooks. Replaces the ratatui immediate-mode renderer.
- **Welcome splash screen.** Full-screen branded splash with block-letter wordmark, quick-start guide, and file count. Shows every launch, dismisses on any keypress.
- **Command palette (`/`).** All 29 v1 commands discoverable with fuzzy search. `Ctrl-F` for file search.
- **Scenario picker (`S`).** Lists built-in + project + global scenarios with source badges.
- **Theme picker (`/theme`).** Live theme swap with 4-cell palette swatch preview. Persists to `config.toml`.
- **Delivery picker (`d`).** 7 choices — pipe claude/agent/gemini, copy md/xml/json, export. Terminal suspend/resume for pipe and export actions.
- **`P` — full composed prompt preview overlay.** Scrollable with j/k/g/G.
- **`@` file picker.** Type `@` in the prompt for fuzzy file search. Enter inserts path + adds to bundle.
- **`x` — export to stdout.** Suspends TUI, prints, "press any key to return."
- **`/edit-prompt` — $EDITOR handoff.** Spawns editor via suspend/resume loop.
- **Code viewer (`v`).** Syntect highlighting, line numbers, keyboard scroll (j/k/g/G/Ctrl-U/D), mouse wheel scroll, drag-to-select with highlight, `a` adds selected range to bundle.
- **Animated focus borders.** `use_animated<u8>` tweens RGB between panel colors on Tab.
- **Gradient token gauge.** `█▓▒░` sub-cell resolution with animated fill.
- **Dynamic viewports.** Tree and viewer row counts scale with terminal height.
- **Width-adaptive layout.** Two-column (≥100 cols), single-panel (<100 cols), three-column when viewer enabled.
- **Smart path truncation.** `…/filename.rs` preserves extension, never breaks readability.
- **Auto-clearing status messages.** 3-second timeout via background timer.
- **Incremental tokenization.** Bundle add/remove tokenizes only the affected item, not all.
- **Shared `LazyLock<Highlighter>`.** Syntect SyntaxSet loaded once, reused forever.

### Changed

- **`/` opens command palette** (was file search). File search moved to `Ctrl-F`.
- **v2 is the default.** No `--tui-v1`/`--tui-v2` flags. Just `cargo run`.
- **AppTheme uses `crossterm::style::Color`** instead of `ratatui::style::Color`.
- **Shared modules extracted.** `theme/`, `tree.rs`, `scenario.rs`, `preview/`, `prompt_input/`, `deliver.rs`, `editor.rs` are now top-level crate modules.
- **`tui2/` renamed to `tui/`.** The iocraft TUI is the canonical `src/tui/`.

### Removed

- **ratatui dependency.** Deleted entirely — not even optional.
- **v1 TUI.** Deleted `app.rs`, `ui.rs`, `events.rs`, `motion.rs`, `mode.rs`, `commands.rs`, `viewer/` (9,755 lines).
- **`tui` Cargo feature.** Replaced by `tui-v2`.
- **`--tui-v1`/`--tui-v2` CLI flags.** Single TUI, no switching.
- **12 v1 integration tests + 4 snapshot files.**

### Internal

- **`src/motion_core.rs`** — framework-agnostic Lerp, EasingFn, timing constants shared between animation systems.
- **`src/tui/motion.rs`** — `use_animated<T>` hook wrapping iocraft `use_state` + `use_future` with `Arc<AtomicBool>` wake signal.
- **`src/tui/viewer/`** — ViewerState with generation-tracked background loads, `use_component_rect` for dynamic viewport sizing, `use_local_terminal_events` for component-local mouse coordinates.
- **`src/tui/overlays/card.rs`** — reusable centered overlay card with `overflow: Hidden`.
- **`src/tui/command_registry.rs`** — 29-command enum with fuzzy filter via `SkimMatcherV2`.
- **Thread-local bridge pattern** — `STARTUP` / `PENDING` / `ROOT_STASH` for component↔run() state transfer across suspend/resume cycles.
- `iocraft 0.8`, `smol 2`, `crossterm 0.29` as runtime deps. `syntect 5` shared with viewer highlighter.

## 1.2.0 — 2026-04-14

### Added
- **Fluid TUI animations.** New `src/tui/motion/` module with `Animated<T>` + semantic wrappers (`Fade`, `Gauge`, `Highlight`, `Slide`), easings, truecolor blending, and a `Clock` abstraction. The render loop is event-driven while idle and ticks at 16ms only when animations are active. Applied to: smooth token gauge fill, modal overlay fade-in with cross-fade between overlays, animated backdrop dim, status message fade-in / hold / fade-out, animated focus border on Tab, bundle-row fade on add, startup fade.
- **Reduce-motion escape hatches.** Honors `NO_ANIMATIONS=1` / `PROMPT_NO_ANIMATIONS=1`; auto-disables on non-truecolor terminals.
- **Code viewer pane.** Press `v` (or `/view`) to toggle a syntect-highlighted preview of the file under the tree cursor. Width-adaptive: three-column (25/45/30) on ≥140 cols; vertical stack on 100-139 cols; suppressed on <100. Line numbers. Binary detection and 2 MB truncation. Three-way Tab cycle (tree → viewer → bundle). Syntect `base16-ocean.dark` theme.
- **Mouse drag-select → bundle range.** With the viewer on, click-and-drag across lines, press `a` (or `/add-selection`) to append that range as a `Range` bundle item. Mouse capture is scoped to viewer-on only so normal terminal text-selection still works when the viewer is off. Scroll-wheel scrolls the viewer (3 lines per tick).
- **Viewer keybindings.** `j/k`/Up/Down to scroll when focused, `g`/`G` to jump top/bottom, `PgUp`/`PgDn` and `Ctrl+U`/`Ctrl+D` for half-page, `Esc` to clear an active selection.
- **Readable directory color.** `Color::Blue` → `Rgb(121, 192, 255)` (GitHub-style link blue) so directories are actually visible on dark terminals.

### Changed
- **`App::mode` is now private.** All call sites migrated to `set_mode()`, which records a `ModeTransition` used by the cross-fade renderer. `mode_mut()` is available for in-place variant-field edits.
- **Three-pass render pipeline.** Normal layer → backdrop dim → outgoing overlay (at `1-t`) → incoming overlay (at `t`) → startup fade. Overlays render into a scratch buffer and blend per-cell so cross-fade is genuinely transparent.
- **New `Focus::Viewer` variant.** `toggle_focus` adapts the cycle based on `viewer.enabled`.

## 1.1.6 — 2026-04-14

### Fixed
- **TUI file tree and bundle list now scroll.** Both lists were rendered with ratatui's `List` widget stateless, so rendering always started from index 0 and the highlighted cursor scrolled off-screen as the user moved past the viewport — large projects looked truncated (e.g. only ~7 of 89 files visible). Both lists are now rendered as stateful widgets with `ListState::select(Some(cursor))`, so ratatui auto-scrolls to keep the cursor visible.

### Added
- **TUI navigation keys for long trees**:
  - `PgDn` / `PgUp` — full-viewport scroll.
  - `Ctrl-D` / `Ctrl-U` — half-page scroll (vim-style).
  - `E` / `C` — expand-all / collapse-all directories. The cursor is preserved on its current file (by relative path) when possible, and clamped to the last visible row otherwise.
- Footer hint row and `?` help overlay updated with the new keys.

## 1.1.4 — 2026-04-12

### Fixed
- Rustfmt formatting fixes for CI compliance.
- README and CHANGELOG updates now included in crates.io package.

## 1.1.3 — 2026-04-12

### Added
- **MCP server: 15 tools** (up from 4). New tools: `ctxforge_add_files`, `ctxforge_add_function`, `ctxforge_add_type`, `ctxforge_remove`, `ctxforge_clear`, `ctxforge_export`, `ctxforge_list_items`, `ctxforge_save_bundle`, `ctxforge_list_profiles`, `ctxforge_list_templates`, `ctxforge_apply_template`. Claude Code can now build context bundles, manage profiles, apply templates, and inspect token budgets autonomously via MCP.
- **MCP resources**: `ctxforge://bundle` (full bundle content), `ctxforge://bundle/items` (item list with token counts), `ctxforge://memory` (all notes), `ctxforge://memory/{tag}` (notes by tag).
- **MCP prompts**: 5 built-in prompts (bugfix, code-review, explain, refactor, migrate) rendered with the current bundle baked in.
- **Safety annotations** on all 15 MCP tools (`readOnlyHint` or `destructiveHint`) per marketplace requirements.
- **MCP protocol updated** from `2025-11-25` to `2025-03-26`.
- **Claude Code plugin** (`ctxforge-plugin/`): plugin manifest, `.mcp.json` config, `/ctxforge` slash command, context engineering skill, README with 3 usage examples.
- **Privacy section** added to README.

## 1.1.1 — 2026-04-12

### Added
- **TUI: `/template-new`, `/template-rm`, `/template-starters`** slash commands for managing templates without leaving the TUI.
- **Model registry refresh** for 2026: Claude Opus 4.6 (1M), Claude Sonnet 4.6, Claude Haiku 4.5, GPT-4.1/mini/nano (1M, exact tiktoken), o3, o3-mini, o4-mini, Gemini 2.5 Pro/Flash (1M). Removed legacy GPT-4 and GPT-3.5-turbo.

## 1.1.0 — 2026-04-12

### Added
- **TUI: Slash command palette** — every feature is now invoked via `/command` (fuzzy-matched). Press `/`, type the first few chars of what you want, Enter to run. Replaces all 13+ single-letter v1.0 keybindings. Vim navigation (`j`/`k`/`Tab`/`Space`/`Enter`/`g`/`G`/`q`) and `Ctrl+F` (file search shortcut) are the only single-key bindings. `?` opens a help overlay showing navigation keys.
- **TUI: Responsive layout** — 40/60 horizontal split on terminals >= 120 cols, vertical stack on narrower terminals.
- **TUI: Status row + hint row** — context-relevant navigation hints plus the three global shortcuts (`/`, `Ctrl+F`, `?`, `q`).
- **CLI: Semantic colored output** via `owo-colors`. Status, recall, resume, error messages all colored. TTY-aware, honors `NO_COLOR=1`.
- **CLI: `ctxforge status` rewritten with `comfy-table`** — proper column layout, color-bucketed `%` column, hotspot warning + tip below the table.
- **CLI: "Did you mean?" suggestions** for `ctxforge add <missing-file>` via `strsim` Jaro-Winkler matching.
- **CLI: Progress spinners** for `ctxforge add --fn` / `--type` tree-sitter scans (auto-hidden when stdout isn't a TTY).
- **CLI: Interactive prompt** for `ctxforge save` when name is omitted on a TTY (`dialoguer::Input`).
- **Prompt templates** — author-written `.md` templates with `{{bundle}}` and `{{task}}` placeholders. Single-pass substitution, no re-scanning. Project-local templates (`.ctxforge/templates/`) shadow user-global templates (`~/.config/ctxforge/templates/`).
  - `ctxforge templates` — list templates from both sources with shadow indicators
  - `ctxforge templates new <name>` — scaffold a project-local template with a starter
  - `ctxforge templates new <name> --from <starter>` — copy from a built-in starter
  - `ctxforge templates starters` — list the 5 built-in starter templates
  - `ctxforge templates rm <name>` — delete a project-local template (refuses global)
  - `--template <name>` and `--task <text>` flags on `copy`, `export`, `pipe`
  - TUI `/template` slash command opens picker -> task input -> auto-copy

### Dependencies added
- `owo-colors 4` (color)
- `comfy-table 7` (status table)
- `indicatif 0.17` (progress bars and spinners)
- `dialoguer 0.11` (interactive input + editor fallback)
- `strsim 0.11` (Jaro-Winkler for "did you mean")

### Removed
- v1.0 single-letter feature keybindings (`c`, `n`, `s`, `l`, `p`, `x`, `m`, `r`, `J`, `f`, `t`, `d`). Use `/copy`, `/narrow`, `/save`, `/load`, `/pipe`, `/export`, `/model`, `/memory`, `/note`, `/find-fn`, `/find-type`, `/find-diff` respectively.

## 1.0.3 — 2026-04-11

### Fixed
- **`ctxforge add <dir>` no longer drags in `.git/`, `.ctxforge/`, and other dotfiles.** `walk::expand` now walks with `hidden(true)`, matching the TUI file tree's existing behavior. Previously, running `ctxforge add .` (or `ctxforge add /path/to/project`, or `ctxforge add '**/*'`) would recursively walk the target directory with `hidden(false)` and include every dotfile in its path — `.git/HEAD`, `.git/config`, `.git/hooks/*.sample`, `.DS_Store`, `.env.example`, `.claude/projects/*`, etc. One user ended up with a 528-item bundle full of git internals. Users who still want to add a specific dotfile can name it explicitly (`ctxforge add .gitignore`) — the literal-file branch of `walk::expand` bypasses the walker entirely.
- **Non-UTF-8 files in a bundle no longer zero out the entire token budget.** `resolve::resolve_one` now calls a new `read_to_utf8_or_placeholder` helper: non-UTF-8 files (e.g. `.git/index`, images, compiled binaries) resolve to `<non-UTF-8 file, N bytes — omitted>` instead of returning `io::Error`. Before this fix, a single binary file in the bundle would make `resolve_all` fail, which the TUI's `recalculate_tokens` swallowed via `.unwrap_or_default()`, causing the gauge to display `~0 / 200,000 (0.0%)` and the "Resolve error: stream did not contain valid UTF-8" status message. Now the gauge is accurate and each binary item shows up in the bundle list with its own small placeholder cost.

### Added
- 5 regression tests in `src/walk.rs` and `src/resolve.rs` covering: dotfiles skipped during glob walks, dotfiles skipped when walking a directory literally, explicit-dotfile-literal still works, non-UTF-8 files resolve to a placeholder, `resolve_all` succeeds when a bundle mixes text and binary files.

## 1.0.2 — 2026-04-11

### Documentation
- **README refresh** to reflect v1.0 reality: install section now lists the four feature flags (`tui`, `mcp`, `extract`, `minimal`) and the three install modes. The Interactive TUI section has an updated ASCII mockup (collapsible `▾`/`▸` markers, profile in header, `λ`/`τ` icons on bundle items, hotspot panel, new footer). Added a full keybinding reference table. `ctxforge add --fn` / `--type` examples added to Quick Start and CLI Reference. MCP protocol version corrected from the stale `2024-11-05` to the actual `2025-11-25`. Project Status rolled forward to include v1.0 and v1.0.1.

This is a **docs-only release** — no code changes, no behavior changes. Published so that the README displayed on crates.io matches the current README on GitHub.

## 1.0.1 — 2026-04-11

### Fixed
- **Project root resolution:** ctxforge no longer walks up the directory tree to find an ancestor `.ctxforge/`. It is now strictly rooted at the current working directory: if `./.ctxforge/` exists it is used, otherwise one is created there. The previous walk-up behavior meant a stray `$HOME/.ctxforge/` (which could be created accidentally by running `ctxforge` from your home shell once) would silently capture every invocation run from anywhere under `$HOME`, showing the home directory as the "project" instead of the project you were actually in. This affected both the TUI launch and every CLI subcommand.

## 1.0.0 — 2026-04-10

### Added
- **TUI:** Collapsible file tree with expand/collapse on Enter (▾/▸ markers, top-level dirs start expanded)
- **TUI:** Fuzzy search with `/` key across all file paths (powered by `fuzzy-matcher`)
- **TUI:** Hotspot panel warns when a single bundle item exceeds 25% of the token budget
- **TUI:** Narrow bundle items to a line range with `n` key (Tab between start/end, Enter to apply)
- **TUI:** Save/load profiles with `s`/`l` keys; current profile name shown in the header
- **TUI:** Pipe-to-agent submenu with `p` key (claude → XML, agent / gemini → markdown)
- **TUI:** Export XML to stdout with `x` key (alternate-screen-aware: restores, prints, returns)
- **TUI:** Switch target model with `m` key — gauge updates live as the bundle is recounted
- **TUI:** Memory recall panel with `r` key, inline note creation with `J` (tag + body inputs)
- **TUI:** Function pick (`f`), type pick (`t`), diff pick (`d`) — `λ`/`τ` icons in the bundle list
- Feature flags: `tui`, `mcp`, `extract`, `minimal` for conditional compilation. Defaults are `tui` + `mcp`. The `minimal` build is a CLI-only ctxforge with no TUI, no MCP, and no tree-sitter.
- Asciinema recording pipeline (`scripts/record-demo.sh`, `scripts/gif-convert.sh`) with consistent 120×35, JetBrains Mono, Monokai theme.

### Fixed
- Updated Cursor CLI target name from `cursor-agent` to `agent` to match the current Cursor CLI binary name.

## 0.7.0 — 2026-04-10

### Tree-sitter extraction (new!)
- **`ctxforge add --fn <name> <file>`** — extract a specific function by name using tree-sitter. Adds only that function's text to the bundle, not the whole file.
- **`ctxforge add --type <name> <file>`** — extract a specific type (struct, class, interface, enum) by name.
- Behind `--features=extract`: `cargo install ctxforge --features=extract`
- **Supported languages**: Rust, Go, Python, TypeScript, JavaScript.
- 9 unit tests covering all languages, not-found cases, and unsupported-language errors.
- Uses tree-sitter 0.26 with `StreamingIterator` API (2026 pattern).
- Default install (`cargo install ctxforge`) is unchanged — extract is opt-in.

## 0.6.1 — 2026-04-09

### Modernization
- **MCP protocol version** bumped from `2024-11-05` to `2025-11-25` (current spec). No breaking changes — the protocol version string in the `initialize` handshake now matches the latest MCP specification.
- **Claude Code install command** updated to explicit transport: `claude mcp add --transport stdio ctxforge -- ctxforge mcp` (follows 2026 `claude mcp add` conventions).
- **TUI initialization** refactored to use `ratatui::run()` convenience function (ratatui 0.30 idiom). Eliminates manual `enable_raw_mode` / `EnterAlternateScreen` / `Terminal::new` boilerplate and guarantees terminal restore even on panic.

## 0.6.0 — 2026-04-09

### MCP Server (new!)
- **`ctxforge mcp`** — stdio JSON-RPC server implementing the Model Context Protocol (2025-11-25). Install for Claude Code with `claude mcp add --transport stdio ctxforge -- ctxforge mcp`.
- **4 tools exposed**: `ctxforge_recall` (search memory), `ctxforge_note` (write memory), `ctxforge_load_bundle` (load a profile), `ctxforge_status` (check token budget).
- Hand-written JSON-RPC — no external MCP dependency. Single-threaded, synchronous, ~300 lines.

### README
- Complete professional rewrite with feature-accurate documentation, ASCII art banner, badge bar, comparison table, full CLI reference, export format examples, supported models table, and tech stack summary.

## 0.5.0 — 2026-04-09

### Interactive TUI (new!)
- **Run `ctxforge` with no subcommand** to launch the interactive TUI composer.
- **Two-panel layout**: file tree (left) with project files, bundle list (right) with selected items and per-item token counts + percentages.
- **Live token gauge** at the top — color grades green → yellow → orange → red as you approach the model's context window. The viral detail.
- **Hotspot highlighting** — items consuming >25% of the budget are highlighted in orange.
- **Vim-style keybindings**: `j`/`k` navigate, `space` toggles selection, `Tab` switches panels, `g`/`G` jump to top/bottom, `c` copies to clipboard, `q` quits.
- `.gitignore`-aware file tree built via the `ignore` crate — target/, .git/, etc. are hidden automatically.
- Auto-detects TTY: launches TUI when stdin is a terminal, falls back to help text when piped.
- New dependencies: `ratatui` 0.30, `crossterm` 0.29.

## 0.4.0 — 2026-04-09

### Pipe to agent (new!)
- **`ctxforge pipe <target> [-- <extra-args>]`** — pipe the rendered bundle to a local agent CLI via stdin. Known targets auto-select the best format: `claude` → XML, everything else → markdown. Override with `--format`.
- Spawns the target as a subprocess, writes to its stdin, waits for exit. Never makes HTTP calls — the target CLI holds its own credentials.
- Supports all memory flags: `--no-memory`, `--memory-tag`, `--memory-limit`.
- Extra arguments after `--` are passed through to the target CLI (e.g. `ctxforge pipe cat -- -n`).
- Clear error messages when the target binary is not found ("Is it installed and in your PATH?").

## 0.3.0 — 2026-04-09

### XML + JSON export formats (new!)
- **`ctxforge export --xml`** — Claude-optimized XML with semantic `<context>` / `<source>` / `<documentation>` / `<memory>` tags. Code content wrapped in `<![CDATA[...]]>`; markdown files become `<documentation>`, everything else becomes `<source>`.
- **`ctxforge export --json`** — structured JSON with `schema_version`, `items_count`, typed item kinds (`file`/`range`/`function`/`type`), and RFC3339 timestamps on memory notes. Built on serde for forward compatibility.
- Same flags work with **`ctxforge copy`** for clipboard output.
- New `--format <name>` flag accepts `markdown`, `md`, `xml`, or `json` (case-insensitive). `--xml` and `--json` are shortcuts.

## 0.2.0 — 2026-04-09

### Memory + continuity (new!)
- **`ctxforge note [--tag <tag>] "<body>"`** — write a memory note that persists across sessions. Stored to `.ctxforge/memory/_index.jsonl` (canonical, append-only JSONL) and `.ctxforge/memory/<tag>.md` (human-readable, git-committable).
- **`ctxforge recall [--tag / --search / --since / --limit]`** — search memory notes by tag, substring, time window (e.g. `1w`, `3d`, `12h`, `30m`), or limit.
- **`ctxforge resume`** — show the current bundle plus the N most-recent memory notes. "Pick up where you left off."
- **`ctxforge export`** and **`ctxforge copy`** now auto-attach the most recent memory notes under a `## Memory` heading. Flags: `--no-memory`, `--memory-tag <tag>`, `--memory-limit <n>`.

### Narrative
ctxforge now implements the full **Write** pillar of context engineering: persistent, human-readable, git-committable decisions that survive across AI agent sessions. Combined with the existing Select / Compress / Isolate features from v0.1, ctxforge now honestly covers all four strategies.

### Infrastructure
- New `memory/` module with `note`, `index`, `recall` submodules.
- `CtxforgeRoot` extended with `memory_dir`, `memory_index_path`, `memory_tag_path`.
- Format dispatcher signature accepts `&[Note]` alongside resolved items.
- **Rust edition 2024** (bumped from 2021). **MSRV 1.85** (from 1.75).
- **Dependency refresh to 2026 latest**: `thiserror` 2.0, `tiktoken-rs` 0.11, `git2` 0.20, `arboard` 3.6, `clap` 4.6, plus patch bumps across the board.

### Backwards compatibility
- Existing `.ctxforge/` directories without a `memory/` subdir are upgraded automatically on the next `ctxforge` run.
- `bundle.json` format is unchanged.
- No migration needed for existing profiles.

## 0.1.1 — 2026-04-09

### Licensing
- **Relicensed from MIT to AGPL-3.0-or-later.** The new license closes the SaaS loophole: anyone who distributes a modified version or runs ctxforge as a hosted network service must release their modifications under AGPL-3.0-or-later.
- **Added Contributor License Agreement (`CLA.md`).** All contributions to ctxforge are assigned to Sylvester Francis, the project's copyright holder. Contributors retain the right to use their code elsewhere but cannot claim ownership of it inside ctxforge. Same model as Qt, MongoDB, and Canonical.
- **Added `CONTRIBUTING.md`** explaining the CLA and contribution workflow.
- **Added `NOTICE`** with explicit copyright attribution.

## 0.1.0 — 2026-04-09

Initial release.

### Features
- `ctxforge add` with file globs, line ranges, `--exclude`, and `--diff <branch>`
- `ctxforge rm` by 1-based index or path
- `ctxforge clear`
- `ctxforge status` with per-item token counts and window percentages
- `ctxforge export` to stdout or file (markdown)
- `ctxforge copy` to the system clipboard
- `ctxforge save` / `ctxforge load` / `ctxforge profiles` / `ctxforge profiles rm`
- Token counting: exact `tiktoken` for OpenAI models, `chars/4` estimate for others
- Model registry with context-window sizes
- `.gitignore`-aware file walking

### Not yet
- Interactive TUI (v0.2)
- Cross-session memory (v0.2)
- MCP server (v0.4)
- XML/JSON exports (v0.2)
- Tree-sitter `--fn` / `--type` (v0.3 behind `--features=extract`)
