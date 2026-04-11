```
   _____________  __ __________  ____  ____________
  / ____/_  __/ |/ // ____/ __ \/ __ \/ ____/ ____/
 / /     / /  |   // /_  / / / / /_/ / / __/ __/
/ /___  / /  /   |/ __/ / /_/ / _, _/ /_/ / /___
\____/ /_/  /_/|_/_/    \____/_/ |_|\____/_____/
```

**The context engineering CLI for AI coding agents.**

Assemble, count, remember, and export perfect context bundles for Claude Code, Cursor, Aider, and any LLM.

[![crates.io](https://img.shields.io/crates/v/ctxforge.svg)](https://crates.io/crates/ctxforge)
[![GitHub stars](https://img.shields.io/github/stars/sylvester-francis/ctx-forge?style=flat)](https://github.com/sylvester-francis/ctx-forge/stargazers)
[![GitHub release](https://img.shields.io/github/v/release/sylvester-francis/ctx-forge?include_prereleases)](https://github.com/sylvester-francis/ctx-forge/releases)
![Rust](https://img.shields.io/badge/Rust-2024_Edition-DEA584?logo=rust&logoColor=white)
![License](https://img.shields.io/badge/License-AGPL--3.0-blue)

[Install](#install) · [Quick Start](#quick-start) · [TUI](#interactive-tui) · [Memory](#cross-session-memory) · [MCP Server](#mcp-server) · [CLI Reference](#cli-reference) · [Export Formats](#export-formats)

---

## Why ctxforge?

Every developer using an AI coding agent does the same manual work dozens of times a day:

1. Open files, scroll to relevant sections
2. Copy code into a prompt
3. Guess whether you've blown the token budget
4. Realize you forgot a key file, repeat
5. Re-explain the same decisions to the agent every session

**ctxforge solves all five.** It implements the four strategies of context engineering — **Write**, **Select**, **Compress**, **Isolate** — as a single Unix tool:

| Strategy | Problem | How ctxforge solves it |
|----------|---------|----------------------|
| **Write** | Agents forget everything between sessions | `ctxforge note` / `ctxforge recall` / `ctxforge resume` — persistent memory |
| **Select** | Agents grab the wrong files or miss the right ones | `ctxforge add` with globs, line ranges, `--diff`, and an interactive TUI |
| **Compress** | You can't see what's eating your context window | Live token gauge with per-item percentages and hotspot highlighting |
| **Isolate** | Context from one task contaminates another | Profiles (`ctxforge save` / `ctxforge load`) keep work streams separate |

---

## Install

```bash
cargo install ctxforge                              # default: TUI + MCP server
cargo install ctxforge --features=extract           # + tree-sitter fn/type extraction
cargo install ctxforge --no-default-features        # minimal: CLI-only, no TUI, no MCP
```

**Requirements:** Rust 1.85+ (edition 2024). Single static binary, no runtime dependencies, no API keys, no cloud.

**Feature flags** (all additive, all opt-out):

| Flag | Default | What it enables |
|---|---|---|
| `tui` | ✓ | Interactive ratatui composer (depends on `ratatui`, `crossterm`, `fuzzy-matcher`) |
| `mcp` | ✓ | Model Context Protocol server (`ctxforge mcp` subcommand) |
| `extract` | — | Tree-sitter function/type extraction for `ctxforge add --fn` / `--type` |
| `minimal` | — | No-op marker — pass `--no-default-features` for a lean CLI-only build |

---

## Quick Start

```bash
# Add files to a context bundle
ctxforge add src/**/*.rs --exclude '*_test.rs'
ctxforge add README.md docs/architecture.md

# Add a specific line range
ctxforge add src/main.rs:10-50

# Add only files changed vs. a branch
ctxforge add --diff main

# Extract a single function or type (requires --features=extract)
ctxforge add --fn ProcessCheck src/hub/check.go
ctxforge add --type Config src/config/config.rs

# See token counts per item
ctxforge status

# Copy to clipboard
ctxforge copy

# Pipe directly to Claude Code
ctxforge pipe claude

# Save this bundle as a reusable profile
ctxforge save feature-auth
```

Or just run `ctxforge` with no arguments to open the interactive TUI.

---

## Interactive TUI

Run `ctxforge` with no subcommand to launch the fullscreen composer. The TUI is always rooted at the current working directory — it creates or reuses `./.ctxforge/`, it never walks up to an ancestor.

```
┌─ ctxforge │ profile: feature-auth │ claude-sonnet-4 │ ~1,204 / 200,000 (0.6%) ─┐
│ tokens ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                                │
├─ files (42) ────────────────────┬─ bundle (4 items) ──────────────────────────┤
│ ▾ src/                          │  1 ■ src/main.rs                   423  27% │
│     ▫ main.rs                   │  2 ■ src/cli.rs                    612  39% │
│     ■ cli.rs                    │  3 λ fn:ProcessCheck (hub/check.go) 231  15% │
│   ▸ commands/                   │  4 τ type:Config (config.rs)       302  19% │
│   ▸ memory/                     ├─ Hotspot ───────────────────────────────────┤
│   ▫ Cargo.toml                  │   2 consumes 39% of the budget.             │
│ ■ README.md                     │   Narrow to a range? [press n]              │
├─────────────────────────────────┴─────────────────────────────────────────────┤
│  ␣ toggle  ↵ expand  / search  n narrow  s save  l load  c copy  q quit       │
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Live token gauge** — color grades green → yellow → orange → red as you approach the model's context window
- **Collapsible file tree** — `▾`/`▸` markers, `Enter` expands/collapses, top-level dirs start open
- **Fuzzy search** — press `/` to filter the tree by path (powered by `fuzzy-matcher`)
- **Hotspot panel** — when one bundle item exceeds 25% of the budget, a warning panel surfaces it by index and suggests `n` to narrow
- **Narrow to range** — `n` on a file item lets you restrict it to specific line numbers, dropping the token cost
- **Profiles** — `s` to save the current bundle, `l` to load one; the active profile name is shown in the header
- **Pipe to agent** — `p` opens a menu to pipe the bundle to `claude` (XML), `agent` (Cursor CLI, markdown), or `gemini`
- **Export XML** — `x` drops out of the alternate screen, prints XML to stdout, then returns
- **Switch model** — `m` opens the model registry and recomputes the gauge live
- **Memory recall** — `r` toggles the memory panel; `J` writes a note inline with tag + body inputs
- **Function / type picker** — `f` and `t` (with `--features=extract`) scan the project via tree-sitter and show pickable lists. Bundle items get `λ`/`τ` icons.
- **Diff picker** — `d` prompts for a branch name, then lets you multi-select changed files
- **.gitignore-aware** — the file tree respects your `.gitignore` automatically
- **TTY detection** — launches the TUI when interactive, falls back to help text when piped

### Keybinding reference

| Key | Mode | Action |
|---|---|---|
| `j`/`k` or ↓/↑ | any list | Move cursor |
| `g` / `G` | any list | Jump to first / last |
| `Tab` | normal | Switch focus between file tree and bundle list |
| `space` | normal | Toggle file selection (file tree) |
| `Enter` | normal | Expand/collapse directory (file tree) |
| `c` | normal | Copy bundle to clipboard (markdown) |
| `/` | normal | Open fuzzy search over file paths |
| `n` | normal | Narrow current bundle item to a line range (Tab between start/end, Enter to apply) |
| `s` | normal | Save current bundle as a named profile |
| `l` | normal | Load a profile from the list |
| `p` | normal | Pipe menu: `c`laude, `a`gent, `g`emini |
| `x` | normal | Export XML to stdout (terminal restores, prints, returns on keypress) |
| `m` | normal | Switch target model (gauge recomputes live) |
| `r` | normal | Toggle memory recall panel |
| `J` | normal | Add a note inline (tag + body, Tab to switch) |
| `f` | normal | Function picker (requires `--features=extract`) |
| `t` | normal | Type picker (requires `--features=extract`) |
| `d` | normal | Diff picker: enter a branch, multi-select changed files |
| `Esc` | any overlay | Cancel and return to normal mode |
| `q` | normal | Quit |
| `Ctrl-C` | any mode | Quit (global) |

---

## Cross-Session Memory

ctxforge maintains persistent notes that survive across agent sessions. Stop re-explaining decisions.

```bash
# Write notes with optional tags
ctxforge note --tag auth "JWT validated from Authorization header, not cookies"
ctxforge note --tag tls "abandoned rustls 0.22 — breaks tonic 0.10"
ctxforge note "module boundaries: memory/ owns persistence, commands/ stays thin"

# Search notes
ctxforge recall                      # all notes, newest first
ctxforge recall --tag auth           # filter by tag
ctxforge recall --search "rustls"    # filter by content (case-insensitive)
ctxforge recall --since 1w           # last week (supports 1w, 3d, 12h, 30m)
ctxforge recall --limit 5            # N most recent

# Pick up where you left off
ctxforge resume                      # shows bundle + 5 most recent notes
```

**Storage:** Notes live in `.ctxforge/memory/` as two parallel stores:
- `_index.jsonl` — append-only JSONL index (canonical, used for recall)
- `<tag>.md` / `decisions.md` — human-readable markdown files (git-committable)

**Auto-attach:** `ctxforge export` and `ctxforge copy` prepend a `## Memory` section with recent notes. Control with `--no-memory`, `--memory-tag`, `--memory-limit`.

---

## MCP Server

Give Claude Code persistent memory in one command:

```bash
claude mcp add --transport stdio ctxforge -- ctxforge mcp
```

That's it. Claude Code can now read and write memory notes, load saved profiles, and check token budgets — all without leaving the conversation.

### Exposed Tools

| Tool | Description |
|------|-------------|
| `ctxforge_recall` | Search memory notes by tag, keyword, or recency |
| `ctxforge_note` | Write a decision or learning that persists across sessions |
| `ctxforge_load_bundle` | Load a saved profile's files into context |
| `ctxforge_status` | Check the current bundle's token budget against the model window |

The MCP server runs as a stdio JSON-RPC process — no network, no daemon, no configuration beyond the one-liner above. Protocol version: `2025-11-25`.

---

## CLI Reference

### Adding Context

```bash
ctxforge add src/**/*.rs                      # glob patterns
ctxforge add src/ docs/                       # directories (recursive)
ctxforge add src/main.rs:10-50                # line range
ctxforge add --exclude '*_test.rs' src/       # exclude patterns
ctxforge add --diff main                      # files changed vs. branch
ctxforge add --fn ProcessCheck src/hub.go     # extract one function (--features=extract)
ctxforge add --type Config src/config.rs      # extract one type (--features=extract)
```

Supported languages for `--fn` / `--type`: Rust, Go, Python, TypeScript, JavaScript.

### Inspecting

```bash
ctxforge status                               # token counts + percentages
ctxforge status --model gpt-4o                # recount for a different model
```

### Managing

```bash
ctxforge rm src/main.rs                       # remove by path
ctxforge rm 3                                 # remove by 1-based index
ctxforge clear                                # remove all items
```

### Exporting

```bash
ctxforge export                               # stdout (markdown)
ctxforge export --xml                         # stdout (XML, Claude-optimized)
ctxforge export --json                        # stdout (JSON, API-friendly)
ctxforge export -o prompt.md                  # write to file
ctxforge copy                                 # clipboard (markdown)
ctxforge copy --xml                           # clipboard (XML)
```

### Piping to Agents

```bash
ctxforge pipe claude                          # auto-selects XML for Claude
ctxforge pipe agent                           # auto-selects markdown (Cursor CLI)
ctxforge pipe gemini                          # auto-selects markdown
ctxforge pipe cat -- -n                       # escape hatch: any binary + args
ctxforge pipe claude --format json            # override format
```

### Profiles

```bash
ctxforge save feature-auth                    # snapshot current bundle
ctxforge load feature-auth                    # restore a profile
ctxforge profiles                             # list saved profiles
ctxforge profiles rm old-one                  # delete a profile
```

### Memory

```bash
ctxforge note "..."                           # untagged note
ctxforge note --tag auth "..."                # tagged note
ctxforge recall                               # show all notes
ctxforge recall --tag auth --since 1w         # filtered
ctxforge resume                               # bundle + recent notes
```

---

## Export Formats

### Markdown (default)

```bash
ctxforge export
```

Each item becomes a fenced code block with a heading and language annotation:

```markdown
## `src/main.rs`

​```rust
fn main() { ... }
​```
```

### XML (Claude-optimized)

```bash
ctxforge export --xml
```

Semantic tags give the agent signals about how to treat each item:

```xml
<context items="3">
  <memory count="2">
    <note timestamp="2026-04-09T14:30:00Z" tag="auth">JWT in header</note>
    <note timestamp="2026-04-09T11:00:00Z">general decision</note>
  </memory>
  <source path="src/main.rs" language="rust"><![CDATA[fn main() { ... }]]></source>
  <documentation path="README.md" language="markdown"><![CDATA[# Project ...]]></documentation>
</context>
```

- `<source>` for code files, `<documentation>` for markdown/docs
- `<memory>` block with timestamps and tags
- CDATA wrapping handles special characters in code safely

### JSON (API-friendly)

```bash
ctxforge export --json
```

Typed, versioned schema for pipeline integration:

```json
{
  "schema_version": 1,
  "items_count": 2,
  "memory": [
    { "timestamp": "2026-04-09T14:30:00Z", "tag": "auth", "body": "JWT in header" }
  ],
  "items": [
    { "path": "src/main.rs", "language": "rust", "kind": "file", "content": "fn main() { ... }\n" },
    { "path": "src/hub.rs", "language": "rust", "kind": "range", "lines": { "start": 45, "end": 120 }, "content": "..." }
  ]
}
```

---

## Supported Models

Exact token counts for OpenAI models via `tiktoken`. Character-based estimates (`chars / 4`) for all others.

| Model | Window | Counting |
|-------|--------|----------|
| `claude-sonnet-4` | 200,000 | ~estimate |
| `claude-opus-4` | 200,000 | ~estimate |
| `claude-haiku-4` | 200,000 | ~estimate |
| `gpt-4o` | 128,000 | exact (o200k) |
| `gpt-4o-mini` | 128,000 | exact (o200k) |
| `gpt-4-turbo` | 128,000 | exact (cl100k) |
| `o1` | 200,000 | exact (o200k) |
| `gemini-1.5-pro` | 2,000,000 | ~estimate |
| `gemini-2-flash` | 1,000,000 | ~estimate |

Unknown model names fall back to a 200k-window estimate. Override the default model with `--model <name>` on any command.

---

## File Layout

```
project/
├── .ctxforge/
│   ├── bundle.json                   # current working bundle (gitignored)
│   ├── profiles/
│   │   ├── feature-auth.json         # saved profiles (committable)
│   │   └── onboarding.json
│   └── memory/
│       ├── _index.jsonl              # canonical note store (append-only)
│       ├── decisions.md              # untagged notes (human-readable)
│       ├── auth.md                   # per-tag files (committable)
│       └── tls.md
```

- `.ctxforge/bundle.json` — gitignored (session-specific working state)
- `.ctxforge/profiles/*.json` — optionally committed for team sharing
- `.ctxforge/memory/*.md` — committable for team-shared decisions

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (edition 2024, MSRV 1.85) |
| CLI parsing | clap 4.6 (derive) |
| TUI framework | ratatui 0.30 + crossterm 0.29 |
| Token counting | tiktoken-rs 0.11 (OpenAI exact) + chars/4 fallback |
| Git integration | git2 0.20 (vendored libgit2) |
| File walking | ignore 0.4 (.gitignore-aware) |
| Clipboard | arboard 3.6 |
| Serialization | serde + serde_json |
| MCP protocol | Hand-written stdio JSON-RPC (no external MCP dependency) |
| Binary size | Single static binary, ~5 MB release |

---

## Design Principles

- **ctxforge never calls an LLM.** The MCP server flips the direction: LLMs call ctxforge.
- **No network calls.** State, memory, and bundles all live in `.ctxforge/` on disk.
- **No API keys.** `ctxforge pipe` hands off to whatever agent CLI you already have installed.
- **No auto-selection heuristics.** You decide what's relevant; ctxforge makes the selection fast.
- **No summarization.** Compression decisions are yours, guided by the live token gauge.

---

## Project Status

- ✅ **v0.1** — Core CLI (add, rm, clear, status, export, copy, save, load, profiles, --diff, tiktoken counting)
- ✅ **v0.2** — Cross-session memory (note, recall, resume, auto-attach)
- ✅ **v0.3** — XML and JSON export formats (--xml, --json, --format)
- ✅ **v0.4** — Pipe to agent (ctxforge pipe claude / agent / gemini)
- ✅ **v0.5** — Interactive TUI (ratatui composer, live token gauge, hotspot highlighting)
- ✅ **v0.6** — MCP server (ctxforge mcp — stdio JSON-RPC, 4 tools)
- ✅ **v0.7** — Tree-sitter function/type extraction (`--fn`, `--type` behind `--features=extract`; Rust, Go, Python, TypeScript, JavaScript)
- ✅ **v1.0** — Full TUI: collapsible tree, fuzzy search (`/`), narrow to range (`n`), save/load profiles (`s`/`l`), pipe menu (`p`), XML export (`x`), model switch (`m`), memory panel (`r`), inline note (`J`), function/type/diff pickers (`f`/`t`/`d`) with `λ`/`τ` icons, hotspot warning panel. Feature flags (`tui` / `mcp` / `extract` / `minimal`) for conditional compilation. Asciinema recording pipeline.
- ✅ **v1.0.1** — Fix: project root resolution is strictly rooted at the current working directory; no longer walks up to find an ancestor `.ctxforge/`. Eliminates the "`$HOME/.ctxforge/` captures everything" footgun.

---

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.

**Important:** All contributions are subject to the [Contributor License Agreement](CLA.md), which assigns copyright to Sylvester Francis. This is the same model used by Qt, MongoDB, and Canonical. See [CLA.md](CLA.md) for the full text and [CONTRIBUTING.md](CONTRIBUTING.md) for how the signing process works.

---

## License

ctxforge is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE) (AGPL-3.0-or-later).

**What this means:**
- Use it freely for any purpose
- Modify and distribute freely
- If you distribute a modified version or run it as a network service, your modifications must also be AGPL-3.0-or-later

**Copyright:** © 2026 Sylvester Francis. All rights reserved. See [NOTICE](NOTICE).
