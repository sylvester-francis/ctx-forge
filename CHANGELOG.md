# Changelog

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
