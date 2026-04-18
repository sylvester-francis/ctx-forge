```
   _____________  __ __________  ____  ____________
  / ____/_  __/ |/ // ____/ __ \/ __ \/ ____/ ____/
 / /     / /  |   // /_  / / / / /_/ / / __/ __/
/ /___  / /  /   |/ __/ / /_/ / _, _/ /_/ / /___
\____/ /_/  /_/|_/_/    \____/_/ |_|\____/_____/
```

**The prompt-engineering CLI for AI coding agents.**

Assemble, count, enrich, suggest, remember, and export token-disciplined prompts for Claude Code, Cursor, Aider, and any LLM. The output IS the crafted prompt; ctxforge does everything a human prompt engineer does — *except* call an LLM.

[![crates.io](https://img.shields.io/crates/v/ctxforge.svg)](https://crates.io/crates/ctxforge)
[![GitHub stars](https://img.shields.io/github/stars/sylvester-francis/ctx-forge?style=flat)](https://github.com/sylvester-francis/ctx-forge/stargazers)
[![GitHub release](https://img.shields.io/github/v/release/sylvester-francis/ctx-forge?include_prereleases)](https://github.com/sylvester-francis/ctx-forge/releases)
![Rust](https://img.shields.io/badge/Rust-2024_Edition-DEA584?logo=rust&logoColor=white)
![License](https://img.shields.io/badge/License-AGPL--3.0-blue)

[Install](#install) · [Quick Start](#quick-start) · [TUI](#interactive-tui) · [Docs & Stack](#library-docs--project-stack) · [GitHub Miner](#github-context-miner) · [Auto-Suggest](#auto-suggest) · [Memory](#cross-session-memory) · [MCP Server](#mcp-server) · [CLI Reference](#cli-reference)

<p align="center">
  <img src="https://github.com/sylvester-francis/ctx-forge/releases/download/v1.1.1/hero-tui.gif" alt="ctxforge TUI demo — file tree, token gauge, slash command palette" width="720" />
</p>

---

## Why ctxforge?

Every developer using an AI coding agent does the same manual work dozens of times a day:

1. Open files, scroll to relevant sections
2. Copy code into a prompt
3. Remember which libraries the project uses and attach their docs
4. Realize the LLM doesn't know about this specific issue — paste the GitHub link
5. Re-explain the same decisions every session
6. Guess whether you've blown the token budget

**ctxforge solves all six.** It implements the five strategies of context engineering — **Select**, **Enrich**, **Suggest**, **Compress**, **Isolate** — as a single Unix tool with matching CLI, MCP, and TUI surfaces:

| Strategy | Problem | How ctxforge solves it |
|----------|---------|----------------------|
| **Select** | Agents grab the wrong files or miss the right ones | `ctxforge add` with globs, line ranges, `--diff`, `--fn`, `--type`, an interactive TUI |
| **Enrich** | LLM doesn't know what stack you're using or the exact issue you hit | `ctxforge docs detect` attaches framework docs + GitHub URLs; `ctxforge add gh:///owner/repo/issues/N` inlines a specific issue/PR/release/file |
| **Suggest** | Your prompt has gaps (using `sqlx` but no sqlx docs attached) | `ctxforge suggest` scans bundle imports vs. Project stack; applies fixes with `--apply` |
| **Compress** | Can't see what's eating the context window | Live token gauge with per-item percentages and hotspot highlighting; links > content bodies |
| **Isolate** | Context from one task contaminates another | Profiles (`ctxforge save` / `ctxforge load`) and memory (`ctxforge note` / `recall`) keep work streams separate |

**North star:** best possible prompt, fewest tokens. No LLM-in-the-loop inside ctxforge's own pipeline. The LLM does retrieval and reasoning on the output — ctxforge stays deterministic.

---

## Install

```bash
cargo install ctxforge                              # default: TUI + MCP server + fetch
cargo install ctxforge --features=extract           # + tree-sitter fn/type extraction
cargo install ctxforge --no-default-features        # minimal: CLI-only, no TUI, no MCP, no network
```

**Requirements:** Rust 1.85+ (edition 2024). Single static binary, no cloud, no API keys (optional `GITHUB_TOKEN` for higher gh:// rate limits).

**Feature flags:**

| Flag | Default | What it enables |
|---|---|---|
| `tui-v2` | yes | iocraft-based TUI composer with reactive rendering, flexbox layout, fluid animations |
| `mcp` | yes | Model Context Protocol server (`ctxforge mcp`; 23 tools) |
| `fetch` | yes | HTTP fetcher for registry descriptions + GitHub API + URL sources |
| `extract` | — | Tree-sitter function/type extraction for `ctxforge add --fn` / `--type` |
| `minimal` | — | No-op marker — pass `--no-default-features` for a lean CLI-only build |

### Claude Code plugin

ctxforge is published in Anthropic's community plugin marketplace:

```bash
claude plugin marketplace add anthropics/claude-plugins-community
claude plugin install ctxforge@claude-community
```

Ships the `/ctxforge` slash command, context-engineering skill, and MCP server pre-wired — no separate `claude mcp add` needed.

---

## Quick Start

```bash
# 1. Build a bundle from source files
ctxforge add src/**/*.rs --exclude '*_test.rs'
ctxforge add src/main.rs:10-50                      # line range
ctxforge add --fn ProcessCheck src/hub/check.go     # single function (--features=extract)
ctxforge add --diff main                            # files changed vs. a branch

# 2. Enrich with library docs from your manifest
ctxforge docs detect                                # crawls Cargo.toml / package.json / pyproject.toml / go.mod
                                                    # attaches per-dep doc URLs + GitHub releases / issues links

# 3. Attach a specific GitHub resource
ctxforge add gh:///tokio-rs/tokio/issues/1234       # inline issue body + metadata
ctxforge add https://github.com/vercel/next.js/pull/12345   # pasted URLs auto-canonicalise

# 4. Check what's missing or stale in your prompt
ctxforge suggest                                    # "your files use sqlx but it's not in Project stack"
ctxforge suggest --apply                            # add missing, remove stale — bulk with confirm

# 5. Count tokens + deliver
ctxforge status                                     # live token gauge per item
ctxforge copy                                       # markdown → clipboard
ctxforge pipe claude                                # XML → Claude Code stdin
ctxforge save feature-auth                          # snapshot as a profile
```

Or just run `ctxforge` with no arguments to open the interactive TUI.

---

## Interactive TUI

Run `ctxforge` with no subcommand to launch the fullscreen composer. The TUI is rooted at the current working directory — it creates or reuses `./.ctxforge/`, never walks up.

**v1.3 — iocraft rewrite.** The TUI was rebuilt from ratatui (immediate-mode) to [iocraft](https://github.com/ccbrown/iocraft) — a React-like reactive framework with taffy flexbox layout. Every surface is redrawn from components; state flows declaratively; layout is responsive. ratatui is gone; iocraft is the only engine.

**Scenario-aware prompt engineer.** The TUI foregrounds the *artifact* (a crafted prompt for a specific scenario), not the bundle. Pick a scenario on launch (bugfix / code-review / explain / refactor / migrate / custom) via the picker or `/scenario`. A permanent multi-line **prompt input** at the bottom captures the task text (press `i` to focus, Shift-Enter for newlines, `@` opens a fuzzy file picker that adds the file + inserts `@path/to/file` at the cursor, `/` at line start opens the command palette). The right column is a **live prompt preview** — template prefix + Task + Context + template suffix — updating on every keystroke. `Ctrl-Enter` (or `/deliver`) opens a delivery picker (copy / pipe / export). `Ctrl-E` opens the task in `$EDITOR`. `/edit-prompt` opens the full composed prompt for a one-shot hand-edit persisted to `.ctxforge/prompt-override.md`. `P` shows the exact bytes that would be delivered.

**CLI/MCP/TUI parity.** Every CLI subcommand and MCP tool has a matching TUI palette entry — 60+ actions, all accessible via `/<name>`:

- `/docs detect`, `/docs detect all`, `/docs refresh`, `/docs list`, `/docs add`, `/docs rm`
- `/github attach` (for `gh://` resources)
- `/suggest`, `/suggest apply all`
- `/url refresh`, `/cache list`, `/cache clear`, `/cache verify`
- `/find fn`, `/find type`, `/find diff`, `/find`
- `/template new`, `/template rm`, `/template list`, `/template starters`, `/template`
- `/save`, `/load`, `/narrow`, `/model`, `/memory`, `/note`, `/theme`
- `/deliver`, `/copy`, `/copy-xml`, `/copy-json`, `/export`, `/pipe`, `/export-xml`, `/export-json`, `/export-stdout`

Themes: four built-in (`ctxforge`, `zinc`, `tokyo-night`, `gruvbox`), persisted in `~/.config/ctxforge/config.toml` or overridden via `CTXFORGE_THEME=<name>`.

**Fluid TUI.** The whole TUI animates: token gauge fills smoothly, modal overlays cross-fade with a dimmed backdrop, status messages fade, focus borders transition on Tab. Event-driven render loop idles at 0% CPU, ticks at 60fps only while animating. Auto-disabled on non-truecolor terminals and via `NO_ANIMATIONS=1`.

**Code viewer + drag-to-add.** Press `v` to open a syntect-highlighted preview pane. Click-and-drag to select a line range, then `a` to append as a `Range` item. Line numbers on every line. Binary and >2 MB files are labeled and skipped.

**Responsive layout.** Wide terminals (≥140 cols) with the viewer on get a 25/45/30 three-column split (tree / viewer / bundle); ≥120 cols get 40/60 two-column. Narrower terminals stack vertically.

### Keybinding reference

Only navigation keys and three shortcuts remain as direct keybindings. Everything else is accessed via the `/` command palette.

| Key | Mode | Action |
|---|---|---|
| `j`/`k` or ↓/↑ | any list | Move cursor (or scroll viewer when focused) |
| `g` / `G` | any list | Jump to first / last (or top/bottom of viewer) |
| `Tab` | normal | Cycle focus: tree → (viewer) → bundle → tree |
| `space` | normal | Toggle file selection (file tree) |
| `Enter` | normal | Expand/collapse directory (file tree) |
| `v` | normal | Toggle the code viewer pane |
| `a` | viewer focused | Add drag-selected lines to the bundle |
| `i` | normal | Focus the prompt input |
| `PgDn`/`PgUp` / `Ctrl+D`/`Ctrl+U` | viewer focused | Half-page scroll |
| mouse drag | viewer visible | Select a line range |
| mouse wheel | viewer visible | Scroll the viewer (3 lines per tick) |
| `/` | normal | Open the slash command palette |
| `@` | prompt input | Open fuzzy file picker + insert `@path` |
| `P` | normal | Show full composed prompt preview |
| `Ctrl+F` | normal | Fuzzy file search (shortcut for `/find`) |
| `Ctrl+Enter` | normal | Deliver (copy / pipe / export picker) |
| `Ctrl+E` | normal | Edit task in `$EDITOR` |
| `?` | normal | Toggle help overlay |
| `Esc` | any overlay | Cancel |
| `q` / `Ctrl-C` | normal | Quit |

---

## Library docs / Project stack

`ctxforge docs detect` scans manifest + lock files (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`) in the project root or recursively in monorepo mode, classifies each dep via the built-in registry, resolves canonical doc URLs, and fetches a one-line description from the ecosystem's registry API (crates.io, npm, PyPI). Framework-tier deps are attached by default; `--all` includes Library tier. All network traffic is minimal (registry metadata only) and cached via the P5 `ContentCache` with HMAC sidecar integrity.

```bash
ctxforge docs detect               # scan + attach framework-tier doc links
ctxforge docs detect --all         # include Library tier
ctxforge docs detect path/to/pkg   # explicit manifest path
ctxforge docs list                 # show attached docs items
ctxforge docs add tokio --ecosystem rust        # manually add
ctxforge docs rm serde                          # remove
ctxforge docs refresh              # re-read lock files, update versions
```

**Per-dep rendering** in the exported prompt:

```markdown
## Project stack

### `Cargo.toml` (Rust)

- **Framework**: axum 0.7.5 — HTTP routing and request handling library
  - docs: https://docs.rs/axum/0.7.5/
  - releases: https://github.com/tokio-rs/axum/releases
  - open issues: https://github.com/tokio-rs/axum/issues
- **Database**: sqlx 0.8.2 — The Rust SQL Toolkit
  - docs: https://docs.rs/sqlx/0.8.2/
  - releases: https://github.com/launchbadge/sqlx/releases
  - open issues: https://github.com/launchbadge/sqlx/issues
- **Async runtime**: tokio 1.38.0 — event-driven, non-blocking I/O
  - docs: https://docs.rs/tokio/1.38.0/
  - releases: https://github.com/tokio-rs/tokio/releases
  - open issues: https://github.com/tokio-rs/tokio/issues
```

**Supported ecosystems:** Rust (Cargo.toml + Cargo.lock with workspace inheritance), JS/TS (package.json), Python (pyproject.toml + requirements.txt), Go (go.mod).

**Monorepos:** Recursively detects manifests; each subsection in the output is keyed by manifest path.

---

## GitHub context miner

Attach a specific GitHub issue / PR / release / file body to the bundle. Pasted `https://github.com/...` URLs are canonicalised to the `gh://` URI form; both work in `ctxforge add`:

```bash
ctxforge add gh:///tokio-rs/tokio/issues/1234          # issue
ctxforge add gh:///rust-lang/rust/pull/100000          # PR (title + state + merged flag)
ctxforge add gh:///tokio-rs/axum/releases/tag/v0.7.5   # release
ctxforge add gh:///owner/repo/blob/main/CHANGELOG.md   # file at a ref
ctxforge add https://github.com/vercel/next.js/issues/1234   # pasted URLs auto-canonicalise
```

**Rendered section:**

```markdown
## `gh:///tokio-rs/tokio/issues/1` — Fix spelling mistake

*by Ported · closed · 2016-09-17T15:36:31Z*

Removed an extra 'a'.
```

**Auth:** `GITHUB_TOKEN` env var lifts the anonymous rate limit (60/hr → 5000/hr). No token = still works, just throttled.

**Caching:** Per-resource TTLs via the P5 ContentCache:
- Issues / PRs: 24h
- Releases: 7d
- Blob pinned to a SHA: 30d
- Blob pinned to a branch: 24h

**Body caps:** Issue / PR / release bodies capped at 2KB, blob contents at 10KB, with `…[N more chars]` marker at UTF-8 boundaries.

**TUI:** `/github attach` opens a prompt, paste the URL, done.

---

## Auto-suggest

`ctxforge suggest` scans bundle file imports (Rust `use`, JS/TS `import`/`require`, Python `import`/`from`, Go `import "..."`) and compares against the existing Project stack. Flags two kinds of gaps:

- **Missing**: a file imports a package with no corresponding DocsSource entry
- **Stale**: a DocsSource entry exists but no bundle file imports the package

```bash
ctxforge suggest                         # bundle-only scan, human output
ctxforge suggest --all                   # walk whole project, not just bundle
ctxforge suggest --missing-only          # skip stale check
ctxforge suggest --json                  # machine-readable (MCP / editor integrations)
ctxforge suggest --apply                 # apply all after Y/n confirm
ctxforge suggest --apply --yes           # scripts / CI
```

**Example output:**

```
⚠ 2 missing, 1 stale

MISSING — imported but not in Project stack:
  rust/sqlx — imported in src/db.rs, src/models/user.rs
    fix: ctxforge docs add sqlx --ecosystem rust

  rust/regex — imported in src/parse.rs
    fix: ctxforge docs add regex --ecosystem rust

STALE — in Project stack but no file imports it:
  rust/async-std — ctxforge docs rm async-std
    (manually added — confirm before removing)

Run `ctxforge suggest --apply` to apply all, or cherry-pick commands above.
```

**Detection is deterministic.** Regex-based per language with stdlib blocklists (skips `std`, `core`, Node builtins like `fs`/`path`, Python stdlib, Go stdlib paths). Hyphen/underscore canonicalisation (`use serde_json::` matches `serde-json` in Cargo.toml). Scoped npm packages preserved (`@next/core`). Deep imports truncated to package root (`lodash/debounce` → `lodash`). Python dotted paths truncated to top module.

**Exit codes** (for CI):
- `0` — no suggestions / all applied
- `1` — user-facing error
- `2` — suggestions exist and were not applied (use in CI: "fail if prompt has gaps")

**Zero prompt leakage.** Suggestions are for the *user*, not the LLM. `ctxforge export` output is byte-identical with or without the suggest module loaded — no `<suggestions>` block, no tokens wasted.

**TUI:** `/suggest` opens a multi-select picker (space to toggle, Enter to apply selected). `/suggest apply all` runs everything non-interactively.

---

## Cross-Session Memory

Persistent notes that survive across agent sessions.

```bash
ctxforge note --tag auth "JWT validated from Authorization header, not cookies"
ctxforge note --tag tls "abandoned rustls 0.22 — breaks tonic 0.10"
ctxforge note "module boundaries: memory/ owns persistence, commands/ stays thin"

ctxforge recall                      # all notes, newest first
ctxforge recall --tag auth           # filter by tag
ctxforge recall --search "rustls"    # filter by content
ctxforge recall --since 1w           # last week (1w, 3d, 12h, 30m)
ctxforge recall --limit 5            # N most recent

ctxforge resume                      # bundle + 5 most recent notes
```

**Storage:** `.ctxforge/memory/`:
- `_index.jsonl` — append-only JSONL index (canonical, used for recall)
- `<tag>.md` / `decisions.md` — human-readable markdown (git-committable)

**Auto-attach:** `ctxforge export` and `ctxforge copy` prepend a `## Memory` section with recent notes. Control with `--no-memory`, `--memory-tag`, `--memory-limit`.

---

## Prompt Templates

Templates wrap your bundle in author-written prose with `{{bundle}}` and `{{task}}` placeholders. Single-pass substitution.

```bash
ctxforge templates                               # list
ctxforge templates new bugfix                    # blank scaffold
ctxforge templates new my-review --from code-review   # from built-in starter
ctxforge templates starters                      # list built-in starters
ctxforge copy --template bugfix --task "null pointer in auth middleware"
ctxforge export --template explain --task "how does the token counting work"
ctxforge pipe claude --template code-review --task "review the new API endpoint"
ctxforge templates rm bugfix
```

**Resolution order:** Project-local (`.ctxforge/templates/`) overrides user-global (`~/.config/ctxforge/templates/`).

**Built-in starters:** `bugfix`, `code-review`, `explain`, `refactor`, `migrate`.

---

## MCP Server

```bash
claude mcp add --transport stdio ctxforge -- ctxforge mcp
```

Stdio JSON-RPC, 23 tools, protocol `2025-03-26`. No network, no daemon.

<p align="center">
  <img src="https://github.com/sylvester-francis/ctx-forge/releases/download/v1.1.1/mcp-demo.gif" alt="ctxforge MCP demo — agent writes and recalls memory" width="600" />
</p>

### Exposed tools (23)

| Tool | Description |
|------|-------------|
| `ctxforge_add_files` | Add files, globs, line ranges, URLs, or `gh://` resources |
| `ctxforge_add_function` | Add a function by name (tree-sitter) |
| `ctxforge_add_type` | Add a type/struct by name (tree-sitter) |
| `ctxforge_add_url` | Attach an arbitrary URL as a cached fetch |
| `ctxforge_remove` | Remove items by path or index |
| `ctxforge_clear` | Clear the entire bundle |
| `ctxforge_refresh` | Force-refresh cached URL sources |
| `ctxforge_list_sources` | Inspect cached URL sources |
| `ctxforge_list_items` | List items with paths, types, token counts |
| `ctxforge_status` | Token budget check vs. model window |
| `ctxforge_export` | Export bundle (markdown / xml / json) |
| `ctxforge_save_bundle` | Save current bundle as a named profile |
| `ctxforge_load_bundle` | Load a saved profile |
| `ctxforge_list_profiles` | List all profiles |
| `ctxforge_list_templates` | List templates (project + user-global) |
| `ctxforge_apply_template` | Render a template with bundle + task |
| `ctxforge_recall` | Search memory notes |
| `ctxforge_note` | Write a memory note |
| `ctxforge_docs_detect` | Detect manifests and attach per-dep doc URLs |
| `ctxforge_docs_add` | Manually add one dep's docs |
| `ctxforge_docs_list` | List docs items in the bundle |
| `ctxforge_suggest` | Report missing / stale documentation entries |
| `ctxforge_suggest_apply` | Apply suggestions (subset via `names` or all) |

### Resources & prompts

- **Resources:** `ctxforge://bundle`, `ctxforge://bundle/items`, `ctxforge://memory`, `ctxforge://memory/{tag}`
- **Prompts:** `bugfix`, `code-review`, `explain`, `refactor`, `migrate`

---

## CLI Reference

### Adding context

```bash
ctxforge add src/**/*.rs                          # globs
ctxforge add src/ docs/                           # dirs (recursive)
ctxforge add src/main.rs:10-50                    # line range
ctxforge add --exclude '*_test.rs' src/           # exclude patterns
ctxforge add --diff main                          # files changed vs. branch
ctxforge add --fn ProcessCheck src/hub.go         # one function (--features=extract)
ctxforge add --type Config src/config.rs          # one type (--features=extract)
ctxforge add https://example.com/spec.md          # URL source (cached)
ctxforge add gh:///tokio-rs/tokio/issues/1234     # GitHub resource
```

### Library docs

```bash
ctxforge docs detect                              # auto-scan manifests
ctxforge docs detect --all                        # include Library tier
ctxforge docs detect path/to/manifest             # explicit
ctxforge docs add tokio --ecosystem rust          # manual add
ctxforge docs rm serde                            # remove
ctxforge docs list                                # list attached
ctxforge docs refresh                             # re-read lock files
```

### Auto-suggest

```bash
ctxforge suggest                                  # bundle scan, human output
ctxforge suggest --all                            # walk whole project
ctxforge suggest --missing-only                   # skip stale check
ctxforge suggest --json                           # machine-readable
ctxforge suggest --apply                          # apply all (confirm prompt)
ctxforge suggest --apply --yes                    # skip confirm (CI)
```

### Inspecting

```bash
ctxforge status                                   # token counts + percentages
ctxforge status --model gpt-4o                    # recount for a different model
```

### Managing

```bash
ctxforge rm src/main.rs                           # remove by path
ctxforge rm 3                                     # remove by 1-based index
ctxforge clear                                    # remove all
```

### Exporting

```bash
ctxforge export                                   # stdout (markdown, no provenance)
ctxforge export --xml                             # stdout (XML, Claude-optimized)
ctxforge export --json                            # stdout (JSON)
ctxforge export -o prompt.md                      # to file
ctxforge export --with-provenance                 # include uri/sha/fetched_at comments
ctxforge copy                                     # clipboard (markdown)
```

### Piping

```bash
ctxforge pipe claude                              # auto-selects XML
ctxforge pipe agent                               # markdown (Cursor CLI)
ctxforge pipe gemini                              # markdown
ctxforge pipe cat -- -n                           # any binary + args
ctxforge pipe claude --format json                # override format
```

### Cache management

```bash
ctxforge cache list                               # show cached entries
ctxforge cache list --scheme url                  # filter by scheme
ctxforge cache clear --stale                      # clear stale only
ctxforge cache clear --all                        # nuke cache
ctxforge cache verify                             # SHA + HMAC walk
ctxforge refresh                                  # force-refresh stale URL sources
ctxforge refresh https://example.com/x.md         # refresh one URI
ctxforge refresh --all                            # all cached URLs
```

### Profiles & templates

```bash
ctxforge save feature-auth                        # snapshot
ctxforge load feature-auth                        # restore
ctxforge profiles                                 # list
ctxforge profiles rm old-one                      # delete

ctxforge templates                                # list
ctxforge templates new bugfix                     # scaffold
ctxforge templates new my-fix --from bugfix       # from starter
ctxforge templates starters                       # list starters
ctxforge templates rm bugfix                      # delete
```

### Memory

```bash
ctxforge note "..."                               # untagged
ctxforge note --tag auth "..."                    # tagged
ctxforge recall                                   # all notes
ctxforge recall --tag auth --since 1w             # filtered
ctxforge resume                                   # bundle + recent notes
```

---

## Export Formats

### Markdown (default)

```markdown
## Project stack
### `Cargo.toml` (Rust)
- **Framework**: axum 0.7.5 — Web framework
  - docs: https://docs.rs/axum/0.7.5/
  - releases: https://github.com/tokio-rs/axum/releases
  - open issues: https://github.com/tokio-rs/axum/issues

## `src/main.rs`
​```rust
fn main() { ... }
​```

## `gh:///tokio-rs/tokio/issues/1` — Fix spelling mistake
*by Ported · closed · 2016-09-17T15:36:31Z*
Removed an extra 'a'.
```

### XML (Claude-optimized)

```xml
<context items="3">
  <project-stack>
    <dep tier="framework" ecosystem="rust" name="axum" version="0.7.5" url="https://docs.rs/axum/0.7.5/" description="Web framework">
      <forge host="github" path="tokio-rs/axum" url="https://github.com/tokio-rs/axum" releases="..." issues="..."/>
    </dep>
  </project-stack>
  <memory count="2">
    <note timestamp="2026-04-09T14:30:00Z" tag="auth">JWT in header</note>
  </memory>
  <source path="src/main.rs" language="rust"><![CDATA[fn main() { ... }]]></source>
</context>
```

### JSON (API-friendly)

```json
{
  "schema_version": 1,
  "items_count": 2,
  "project_stack": [
    {
      "name": "axum", "version": "0.7.5", "ecosystem": "rust", "tier": "framework",
      "url": "https://docs.rs/axum/0.7.5/",
      "forge": { "host": "github", "path": "tokio-rs/axum", "raw_url": "https://github.com/tokio-rs/axum" }
    }
  ],
  "memory": [...],
  "items": [...]
}
```

---

## Supported Models

Exact token counts for OpenAI models via `tiktoken`. Character-based estimates (`chars / 4`) for all others.

| Model | Window | Counting |
|-------|--------|----------|
| `claude-opus-4-7` | 1,000,000 | ~estimate |
| `claude-sonnet-4-6` | 200,000 | ~estimate |
| `claude-haiku-4-5` | 200,000 | ~estimate |
| `claude-sonnet-4` | 200,000 | ~estimate |
| `claude-opus-4` | 200,000 | ~estimate |
| `gpt-4.1` | 1,047,576 | exact (o200k) |
| `gpt-4.1-mini` | 1,047,576 | exact (o200k) |
| `gpt-4.1-nano` | 1,047,576 | exact (o200k) |
| `o4-mini` | 200,000 | exact (o200k) |
| `o3` | 200,000 | exact (o200k) |
| `o3-mini` | 200,000 | exact (o200k) |
| `o1` | 200,000 | exact (o200k) |
| `gpt-4o` | 128,000 | exact (o200k) |
| `gpt-4o-mini` | 128,000 | exact (o200k) |
| `gemini-2.5-pro` | 1,048,576 | ~estimate |
| `gemini-2.5-flash` | 1,048,576 | ~estimate |
| `gemini-2-flash` | 1,000,000 | ~estimate |
| `gemini-1.5-pro` | 2,000,000 | ~estimate |

Unknown model names fall back to a 200k-window estimate. Override with `--model <name>` on any command.

---

## File Layout

```
project/
├── .ctxforge/
│   ├── bundle.json                   # current working bundle (gitignored)
│   ├── prompt-override.md            # TUI /edit-prompt result (gitignored)
│   ├── profiles/
│   │   ├── feature-auth.json         # saved profiles (committable)
│   │   └── onboarding.json
│   ├── templates/
│   │   ├── bugfix.md                 # project-local prompt templates
│   │   └── explain.md
│   └── memory/
│       ├── _index.jsonl              # canonical note store (append-only)
│       ├── decisions.md              # untagged notes
│       ├── auth.md                   # per-tag files (committable)
│       └── tls.md
~/.config/ctxforge/
├── config.toml                       # user-global theme + default send target
└── templates/                        # user-global templates (fallback)
~/.cache/ctxforge/                    # (or $XDG_CACHE_HOME)
└── v1/                               # P5 ContentCache: URL / gh:// / registry responses
    ├── <sha>.body
    └── <sha>.meta + <sha>.hmac
```

---

## Architecture

### The pipeline

ctxforge is strictly deterministic. Every pipeline stage is a pure function of the bundle + filesystem state, cached idempotently. No LLM ever sees your data *inside* ctxforge — the LLM is the consumer of the *output*.

```
 ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
 │  Bundle      │──▶│  Resolve     │──▶│  Render      │──▶│  Deliver     │
 │  (JSON)      │   │  (files +    │   │  (markdown / │   │  (stdout /   │
 │              │   │   cache +    │   │   xml /      │   │   clipboard/ │
 │  Source enum │   │   network)   │   │   json)      │   │   pipe)      │
 └──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘
       ▲                  ▲
       │                  │
       │  ┌───────────────┴────────────────┐
       │  │ P5 ContentCache (HMAC sidecar) │
       │  └────────────────────────────────┘
       │
 ┌─────┴──────────────────────────────────────────┐
 │  Inputs                                        │
 │  - `add` / TUI (files, ranges, functions)      │
 │  - `docs detect` (Cargo.toml, package.json…)   │
 │  - `add gh://` (GitHub issues/PRs/releases)    │
 │  - `suggest` (emits add/rm commands)           │
 └────────────────────────────────────────────────┘
```

### Source variants

```rust
pub enum Source {
    File(FileSource),        // local file
    Range(RangeSource),      // file + start..end
    Func(FuncSource),        // tree-sitter function extraction
    Type(TypeSource),        // tree-sitter type extraction
    Url(UrlSource),          // cached HTTPS fetch
    Docs(DocsSource),        // library-doc link + optional forge enrichment
    Gh(GhSource),            // specific GitHub resource
}
```

Each variant has a canonical URI (`file:///...`, `range:///path#L10-L20`, `docs:///rust/axum@0.7.5`, `gh:///owner/repo/issues/N`), used as the cache key and provenance record.

### Roadmap milestones (shipped)

- **P5** — Context source abstraction: URI + ContentCache + fetcher + provenance. PR #8.
- **P1** — Library docs gatherer: manifest scan + registry metadata + canonical doc URLs. PR #10.
- **P3** — GitHub enrichment: per-dep forge URLs + `gh://` specific-resource attachment. PR #11.
- **P4** — Auto-suggest context: deterministic import-vs-stack mismatch detector with `--apply`.

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (edition 2024, MSRV 1.85) |
| CLI parsing | clap 4.6 (derive) |
| TUI framework | iocraft 0.8 + crossterm 0.29 + smol |
| Syntax highlighting | syntect 5 (bundled syntaxes + `base16-ocean.dark`) |
| Token counting | tiktoken-rs 0.11 (OpenAI exact) + chars/4 fallback |
| Git integration | git2 0.20 (vendored libgit2) |
| File walking | ignore 0.4 (.gitignore-aware) |
| Regex | regex 1 (import detection, fetch pipeline) |
| HTTP | ureq 3 (blocking, sync) |
| Cache integrity | sha2 + hmac 0.12 + getrandom 0.3 (OS entropy) |
| Clipboard | arboard 3.6 |
| Serialization | serde + serde_json + toml 0.8 + serde_yaml |
| Colored output | owo-colors 4 (TTY-aware, honors `NO_COLOR`) |
| CLI tables | comfy-table 7 |
| Spinners | indicatif 0.17 (auto-hidden on non-TTY) |
| Interactive input | dialoguer 0.11 (Input, Select, MultiSelect, editor fallback) |
| Fuzzy matching | fuzzy-matcher 0.3 (TUI tree search) + strsim 0.11 ("did you mean") |
| MCP protocol | Hand-written stdio JSON-RPC (no external MCP crate) |
| Binary size | Single static binary, ~6 MB release |

---

## Design Principles

- **ctxforge never calls an LLM.** Period. The LLM calls ctxforge (via MCP) or consumes its output.
- **Deterministic pipeline.** Every stage is a pure function. No randomness except fresh HMAC keys (from OS entropy).
- **Links > content bodies wherever possible.** Registry descriptions, forge URLs, doc links — all cheaper than pasting full pages.
- **No retrieval engines inside ctxforge.** The LLM has `web_search`; we don't duplicate it. Token discipline wins.
- **No API keys required.** `GITHUB_TOKEN` is optional (higher gh:// rate limits). Everything works anonymously.
- **Network only for metadata.** `docs detect` fetches tiny registry JSON responses; `gh://` fetches a single issue/PR body. All cached with TTLs.
- **CLI / MCP / TUI parity.** Every user action is available on all three surfaces. No CLI-only or MCP-only features.
- **No summarization.** Compression decisions are yours, guided by the live token gauge.
- **Local-first.** Bundles, memory, profiles, templates all in `.ctxforge/`. Cache in `$XDG_CACHE_HOME`. Nothing leaves your machine except explicit network fetches you initiated.

---

## Project Status

- **v0.1–v0.4** — Core CLI, memory, XML/JSON export, pipe-to-agent.
- **v0.5** — First interactive TUI (ratatui composer, live token gauge, hotspot highlighting).
- **v0.6** — MCP server (stdio JSON-RPC, 4 tools).
- **v0.7** — Tree-sitter `--fn` / `--type` extraction (Rust, Go, Python, TypeScript, JavaScript).
- **v1.0** — Full TUI: collapsible tree, fuzzy search, narrow to range, save/load profiles, pipe menu, XML export, model switch, memory panel, inline note, function/type/diff pickers with `λ`/`τ` icons, hotspot warning panel. Feature flags.
- **v1.0.1 – v1.0.3** — Fixes: strict project-root resolution; skip dotfiles in glob walks; non-UTF-8 file placeholder instead of crash.
- **v1.1** — Slash command palette (`/`), responsive layout, help overlay, `Ctrl+F` shortcut. Prompt templates (`{{bundle}}` / `{{task}}`) with 5 built-in starters. CLI polish: `owo-colors`, `comfy-table`, `indicatif`, `dialoguer`, `strsim`.
- **v1.1.3** — MCP expanded to 15 tools. Resources (`ctxforge://bundle`, `ctxforge://memory`) and prompts. Protocol upgraded to `2025-03-26`. Claude Code plugin.
- **v1.2** — Fluid TUI: typed animation layer (`Animated<T>` + Fade/Gauge/Highlight/Slide), event-driven render loop, modal cross-fade with backdrop dim. Persistent `ListState` for long trees/bundles. Code viewer (`v` toggles syntect-highlighted preview; drag-select + `a` appends as Range). Three-way focus cycle. Mouse capture scoped to viewer-on.
- **v1.3** — iocraft TUI rewrite. ratatui is fully removed; iocraft is the only rendering engine. Reactive components, taffy flexbox layout, scenario-aware prompt engineer (prompt input + live preview + delivery picker + `/edit-prompt` with override file). Full CLI/MCP/TUI parity (60+ palette entries).
- **v1.3+** (unreleased, current branch) —
  - **P5: Context source abstraction.** `Source` enum with `File`/`Range`/`Func`/`Type`/`Url`/`Docs`/`Gh` variants. URI-keyed HMAC-integrity cache (`ContentCache` with sidecar). Provenance records (uri, sha256, fetched_at, etag, stale, failed). SSRF resolver checks.
  - **P1: Library docs gatherer.** `ctxforge docs detect/add/rm/list/refresh`. Parses Cargo.toml / package.json / pyproject.toml / go.mod. Per-dep canonical doc URL + registry description. Monorepo support (nested manifests). Tier classification via built-in registry.
  - **P3: GitHub enrichment.** Project stack per-dep releases + open issues URLs (for GitHub / GitLab / Codeberg forges). `gh:///owner/repo/issues|pull|releases/tag|blob/...` source variant. `GITHUB_TOKEN` Bearer auth. Pasted github.com URLs auto-canonicalise.
  - **P4: Auto-suggest context.** `ctxforge suggest` with `--all`, `--missing-only`, `--json`, `--apply`, `--yes`. Import scanners for Rust / JS / TS / Python / Go with stdlib blocklists and hyphen/underscore canonicalisation. MCP tool count 15 → 23. TUI `/suggest` multi-select picker + `/suggest apply all` one-shot.

---

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.

**CLA:** All contributions are subject to the [Contributor License Agreement](CLA.md), which assigns copyright to Sylvester Francis (same model as Qt / MongoDB / Canonical).

---

## License

ctxforge is licensed under [GNU Affero General Public License v3.0 or later](LICENSE) (AGPL-3.0-or-later).

**What this means:**
- Use it freely for any purpose
- Modify and distribute freely
- If you distribute a modified version or run it as a network service, your modifications must also be AGPL-3.0-or-later

**Copyright:** © 2026 Sylvester Francis. All rights reserved. See [NOTICE](NOTICE).

---

## Privacy

ctxforge runs entirely on your local machine. It collects **no telemetry**, requires **no API keys** (GITHUB_TOKEN is optional), and sends no data to any cloud or analytics service.

**Network calls are explicit and bounded:**
- `ctxforge docs detect` → registry JSON metadata (crates.io, npm, PyPI) — cached 7 days
- `ctxforge add https://...` → the URL you specified — cached per the URI
- `ctxforge add gh://...` → GitHub REST API / raw.githubusercontent.com — cached with per-resource TTLs
- `--offline` on export skips all network; serves from cache or placeholder

All state (bundles, profiles, memory notes, templates) lives in `.ctxforge/` in your project. The cache lives in `$XDG_CACHE_HOME/ctxforge` (or `~/.cache/ctxforge`). The MCP server communicates exclusively via stdio with your local AI agent — nothing leaves your machine beyond the explicit fetches above.
