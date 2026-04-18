```
   _____________  __ __________  ____  ____________
  / ____/_  __/ |/ // ____/ __ \/ __ \/ ____/ ____/
 / /     / /  |   // /_  / / / / /_/ / / __/ __/
/ /___  / /  /   |/ __/ / /_/ / _, _/ /_/ / /___
\____/ /_/  /_/|_/_/    \____/_/ |_|\____/_____/
```

### The prompt engineer for AI coding agents.

[![crates.io](https://img.shields.io/crates/v/ctxforge.svg)](https://crates.io/crates/ctxforge)
[![GitHub release](https://img.shields.io/github/v/release/sylvester-francis/ctx-forge?include_prereleases)](https://github.com/sylvester-francis/ctx-forge/releases)
[![Rust](https://img.shields.io/badge/Rust-2024-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)

<p align="center">
  <img src="https://github.com/sylvester-francis/ctx-forge/releases/download/v1.1.1/hero-tui.gif" alt="ctxforge TUI — file tree, token gauge, slash command palette" width="720" />
</p>

ctxforge assembles, enriches, and exports token-disciplined prompts for Claude Code, Cursor, Aider, and any LLM. The output IS the crafted prompt. **ctxforge never calls an LLM** — it does everything a human prompt engineer does *except* the reasoning step.

---

## Installation

```bash
# Cargo (recommended — latest features)
cargo install ctxforge

# With tree-sitter function/type extraction
cargo install ctxforge --features=extract

# Minimal build (CLI-only, no TUI, no MCP, no network)
cargo install ctxforge --no-default-features
```

**Requirements:** Rust 1.85+ (edition 2024). Single static binary, ~6 MB release. No runtime dependencies. No API keys (optional `GITHUB_TOKEN` for higher `gh://` rate limits).

<details>
<summary>Feature flags</summary>

| Flag | Default | What it enables |
|---|---|---|
| `tui-v2` | ✓ | iocraft TUI composer with reactive rendering, flexbox layout, fluid animations |
| `mcp` | ✓ | Model Context Protocol server (`ctxforge mcp`; 31 tools) |
| `fetch` | ✓ | HTTP fetcher for registry metadata, GitHub API, and URL sources |
| `extract` | — | Tree-sitter function/type extraction for `ctxforge add --fn` / `--type` |
| `minimal` | — | No-op marker — pass `--no-default-features` for a lean CLI-only build |

</details>

### Claude Code plugin

```bash
claude plugin marketplace add anthropics/claude-plugins-community
claude plugin install ctxforge@claude-community
```

Ships the `/ctxforge` slash command, a context-engineering skill, and the MCP server pre-wired — no separate `claude mcp add` needed.

---

## Quick Start

```bash
# 1. Build a bundle from source
ctxforge add src/**/*.rs --exclude '*_test.rs'
ctxforge add src/main.rs:10-50                    # line range
ctxforge add --fn ProcessCheck src/hub/check.go   # single function
ctxforge add --diff main                          # files changed vs. branch

# 2. Enrich with library docs from your manifest
ctxforge docs detect                              # crawls Cargo.toml / package.json / pyproject.toml / go.mod

# 3. Attach a specific GitHub resource
ctxforge add gh:///tokio-rs/tokio/issues/1234
ctxforge add https://github.com/vercel/next.js/pull/12345   # auto-canonicalises

# 4. Check what's missing or stale in your prompt
ctxforge suggest
ctxforge suggest --apply                          # bulk add missing, remove stale

# 5. Count tokens + deliver
ctxforge status
ctxforge copy                                     # markdown → clipboard
ctxforge pipe claude                              # XML → Claude Code stdin
ctxforge save feature-auth                        # snapshot as a profile
```

Or run `ctxforge` with no arguments to open the interactive TUI.

---

## Core capabilities

ctxforge implements five strategies of context engineering as a single Unix tool with matching CLI / MCP / TUI / plugin surfaces — every feature is available on every surface.

### Library docs / Project stack

`ctxforge docs detect` scans manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`), classifies each dep via the built-in registry, and attaches canonical doc URLs + registry descriptions. Framework-tier deps by default; `--all` for library tier.

```markdown
### `Cargo.toml` (Rust)

- **Framework**: axum 0.7.5 — HTTP routing and request handling library
  - docs: https://docs.rs/axum/0.7.5/
  - releases: https://github.com/tokio-rs/axum/releases
  - open issues: https://github.com/tokio-rs/axum/issues
- **Database**: sqlx 0.8.2 — The Rust SQL Toolkit
  - docs: https://docs.rs/sqlx/0.8.2/
  - releases: https://github.com/launchbadge/sqlx/releases
  - open issues: https://github.com/launchbadge/sqlx/issues
```

Monorepo-aware. Nested manifests produce subsections keyed by manifest path.

### GitHub context miner

Attach a specific GitHub issue / PR / release / file body. Pasted `https://github.com/...` URLs auto-canonicalise.

```bash
ctxforge add gh:///tokio-rs/tokio/issues/1234
ctxforge add gh:///rust-lang/rust/pull/100000
ctxforge add gh:///tokio-rs/axum/releases/tag/v0.7.5
ctxforge add gh:///owner/repo/blob/main/CHANGELOG.md
```

`GITHUB_TOKEN` env var lifts the anonymous rate limit (60/hr → 5000/hr). Per-resource TTLs: issues/PRs 24h, releases 7d, SHA-pinned blobs 30d, branch-pinned blobs 24h. Bodies capped at 2 KB (issue/PR/release) or 10 KB (blob) with UTF-8-safe truncation.

### Auto-suggest

`ctxforge suggest` scans bundle imports (Rust `use`, JS/TS `import`, Python `from`/`import`, Go `import`) against the Project stack and flags gaps:

```
⚠ 2 missing, 1 stale

MISSING — imported but not in Project stack:
  rust/sqlx — imported in src/db.rs
    fix: ctxforge docs add sqlx --ecosystem rust

STALE — in Project stack but no file imports it:
  rust/async-std — ctxforge docs rm async-std
```

Deterministic: regex-based per language with stdlib blocklists, hyphen/underscore canonicalisation, scoped-npm handling, dotted-path truncation. `--apply` bulk-runs every fix (with Y/n confirm or `--yes` for CI). Zero prompt leakage — `ctxforge export` output is byte-identical with or without suggest loaded.

### Interactive TUI

Run `ctxforge` with no subcommand. Built on [iocraft](https://github.com/ccbrown/iocraft) — reactive components with taffy flexbox layout and fluid animations.

- **Scenario-aware** — pick a scenario (bugfix / code-review / explain / refactor / migrate / custom) on launch. Live prompt preview on the right updates as you type the task.
- **Slash command palette** — every feature is a `/command`. 48 entries. Fuzzy matching.
- **Prompt input + file picker** — `i` focuses a multi-line prompt input; `@` inside it opens a fuzzy file picker that inserts `@path/to/file` and adds the file to the bundle.
- **Delivery** — `Ctrl-Enter` opens a picker (copy / pipe / export). `Ctrl-E` opens the task in `$EDITOR`. `/edit-prompt` lets you hand-edit the full composed prompt; override persists to `.ctxforge/prompt-override.md`.
- **Code viewer + drag-to-add** — press `v` for a syntect-highlighted pane. Click-and-drag across lines, press `a` to append as a `Range` item.
- **Themes** — four built-in (`ctxforge`, `zinc`, `tokyo-night`, `gruvbox`). Persisted in `~/.config/ctxforge/config.toml` or set via `CTXFORGE_THEME=<name>`.

<details>
<summary>Keybinding reference</summary>

Only navigation keys and three shortcuts remain. Everything else is accessed via `/`.

| Key | Action |
|---|---|
| `j`/`k` or ↓/↑ | Move cursor (or scroll viewer when focused) |
| `g` / `G` | Jump to first / last (or top/bottom of viewer) |
| `Tab` | Cycle focus: tree → (viewer) → bundle |
| `space` | Toggle file selection (file tree) |
| `Enter` | Expand/collapse directory |
| `v` | Toggle the code viewer pane |
| `a` | Add drag-selected lines to the bundle |
| `i` | Focus the prompt input |
| `/` | Open slash command palette |
| `@` | (in prompt input) Open fuzzy file picker |
| `P` | Show full composed prompt preview |
| `Ctrl+F` | Fuzzy file search |
| `Ctrl+Enter` | Deliver (copy / pipe / export picker) |
| `Ctrl+E` | Edit task in `$EDITOR` |
| `?` | Toggle help overlay |
| `Esc` | Cancel overlay |
| `q` / `Ctrl+C` | Quit |

</details>

### Cross-session memory

Persistent notes that survive across agent sessions.

```bash
ctxforge note --tag auth "JWT validated from Authorization header, not cookies"
ctxforge recall --tag auth --since 1w
ctxforge resume      # bundle + 5 most recent notes
```

Storage: `.ctxforge/memory/_index.jsonl` (canonical append-only) + per-tag `.md` files (git-committable). Auto-attached to `ctxforge export` output; control with `--no-memory` / `--memory-tag` / `--memory-limit`.

### MCP server (31 tools)

```bash
claude mcp add --transport stdio ctxforge -- ctxforge mcp
```

Stdio JSON-RPC, protocol `2025-03-26`. No network, no daemon. Full tool list in [docs/mcp.md](docs/mcp.md) or via `tools/list`.

---

## CLI Reference

<details>
<summary>Adding context</summary>

```bash
ctxforge add src/**/*.rs                          # globs
ctxforge add src/ docs/                           # directories
ctxforge add src/main.rs:10-50                    # line range
ctxforge add --exclude '*_test.rs' src/           # exclude patterns
ctxforge add --diff main                          # files changed vs. branch
ctxforge add --fn ProcessCheck src/hub.go         # one function (--features=extract)
ctxforge add --type Config src/config.rs          # one type
ctxforge add https://example.com/spec.md          # URL source (cached)
ctxforge add gh:///tokio-rs/tokio/issues/1234     # GitHub resource
```

</details>

<details>
<summary>Library docs</summary>

```bash
ctxforge docs detect                              # auto-scan manifests
ctxforge docs detect --all                        # include library tier
ctxforge docs detect path/to/manifest             # explicit
ctxforge docs add tokio --ecosystem rust          # manual add
ctxforge docs rm serde                            # remove
ctxforge docs list                                # show attached
ctxforge docs refresh                             # re-read lock files
```

</details>

<details>
<summary>Auto-suggest</summary>

```bash
ctxforge suggest                                  # bundle scan, human output
ctxforge suggest --all                            # walk whole project
ctxforge suggest --missing-only                   # skip stale check
ctxforge suggest --json                           # machine-readable
ctxforge suggest --apply                          # apply all (confirm prompt)
ctxforge suggest --apply --yes                    # skip confirm (CI)
```

Exit codes: `0` = no suggestions / all applied, `1` = error, `2` = suggestions exist (useful for CI gate).

</details>

<details>
<summary>Bundle management</summary>

```bash
ctxforge status                                   # token counts + percentages
ctxforge status --model gpt-4o                    # recount for a different model
ctxforge rm src/main.rs                           # remove by path
ctxforge rm 3                                     # remove by 1-based index
ctxforge clear                                    # remove all
```

</details>

<details>
<summary>Exporting and piping</summary>

```bash
ctxforge export                                   # stdout (markdown)
ctxforge export --xml                             # stdout (XML, Claude-optimized)
ctxforge export --json                            # stdout (JSON)
ctxforge export -o prompt.md                      # to file
ctxforge export --with-provenance                 # include uri/sha/fetched_at comments

ctxforge copy                                     # clipboard (markdown)
ctxforge copy --xml                               # clipboard (XML)

ctxforge pipe claude                              # auto-selects XML
ctxforge pipe agent                               # markdown (Cursor CLI)
ctxforge pipe gemini                              # markdown
ctxforge pipe cat -- -n                           # any binary + args
```

</details>

<details>
<summary>Cache management</summary>

```bash
ctxforge cache list                               # show cached entries
ctxforge cache list --scheme url                  # filter by scheme
ctxforge cache clear --stale                      # clear stale only
ctxforge cache clear --all                        # nuke cache
ctxforge cache verify                             # SHA + HMAC walk
ctxforge refresh                                  # force-refresh stale URLs
ctxforge refresh --all                            # all cached URLs
```

</details>

<details>
<summary>Profiles, templates, memory</summary>

```bash
# Profiles
ctxforge save feature-auth                        # snapshot
ctxforge load feature-auth                        # restore
ctxforge profiles                                 # list
ctxforge profiles rm old-one                      # delete

# Templates
ctxforge templates                                # list
ctxforge templates new bugfix                     # scaffold
ctxforge templates new my-fix --from bugfix       # from starter
ctxforge templates starters                       # list starters
ctxforge templates rm bugfix                      # delete
ctxforge copy --template bugfix --task "..."      # apply with copy
ctxforge export --template explain --task "-"     # task from stdin

# Memory
ctxforge note "..."                               # untagged
ctxforge note --tag auth "..."                    # tagged
ctxforge recall                                   # all notes
ctxforge recall --tag auth --since 1w             # filtered
ctxforge resume                                   # bundle + recent notes
```

</details>

---

## Supported models

Exact token counts for OpenAI models via `tiktoken`. Character-based estimates (`chars / 4`) for all others.

| Model | Window | Counting |
|-------|--------|----------|
| `claude-opus-4-7` | 1,000,000 | ~estimate |
| `claude-sonnet-4-6` / `claude-haiku-4-5` | 200,000 | ~estimate |
| `gpt-4.1` / `gpt-4.1-mini` / `gpt-4.1-nano` | 1,047,576 | exact (o200k) |
| `o4-mini` / `o3` / `o3-mini` / `o1` | 200,000 | exact (o200k) |
| `gpt-4o` / `gpt-4o-mini` | 128,000 | exact (o200k) |
| `gemini-2.5-pro` / `gemini-2.5-flash` | 1,048,576 | ~estimate |
| `gemini-1.5-pro` | 2,000,000 | ~estimate |

Unknown model names fall back to a 200k-window estimate. Override with `--model <name>` on any command.

---

## Design principles

- **ctxforge never calls an LLM.** The LLM calls ctxforge (via MCP) or consumes its output. Period.
- **Deterministic pipeline.** Every stage is a pure function. No randomness except fresh HMAC keys (from OS entropy).
- **Links > content bodies wherever possible.** Doc URLs, forge URLs, registry descriptions — all cheaper than pasting full pages.
- **No retrieval engines inside ctxforge.** The LLM has `web_search`; we don't duplicate it. Token discipline wins.
- **Network only for metadata.** `docs detect` fetches tiny registry JSON. `gh://` fetches a single issue body. All cached with TTLs.
- **CLI / MCP / TUI / plugin parity.** Every user action is available on every surface.
- **No summarization.** Compression decisions are yours, guided by the live token gauge.
- **Local-first.** Bundles, memory, profiles, templates all in `.ctxforge/`. Nothing leaves your machine except explicit network fetches you initiated.

---

## FAQ

<details>
<summary>How is this different from Claude Code?</summary>

ctxforge and Claude Code are complementary, not competitive. Claude Code is the *agent* that drives your coding session. ctxforge is the *prompt engineer* that builds the context Claude Code (or any agent) runs on.

- Claude Code runs an LLM. ctxforge never does.
- Claude Code's MCP server model means ctxforge can register as a tool provider — Claude Code calls `ctxforge_docs_detect`, `ctxforge_suggest`, `ctxforge_add_files`, etc., and gets back assembled context.
- ctxforge works equally well handing off to Cursor, Aider, Gemini CLI, or pasting into a web UI. It's not coupled to any agent.

</details>

<details>
<summary>How is this different from Cursor / Windsurf / Aider?</summary>

Those are editors or agents that manage context implicitly (usually by auto-including open files and running retrieval over the repo). ctxforge makes context **explicit**: you decide what goes into the prompt, you see the token cost, you pin specific GitHub issues or doc links.

Use ctxforge when:
- The automatic retrieval isn't including the right files
- You're hitting context window limits and need to see what's eating your budget
- You want the exact same prompt for repeated workflows (save it as a profile)
- You need the prompt to include docs for libraries the agent doesn't know about

</details>

<details>
<summary>How is this different from RepoMix / files-to-prompt / gitingest?</summary>

Those tools dump files into a prompt. ctxforge does that too, but also:

- **Project stack awareness** — runs `docs detect` to attach the canonical doc URL + GitHub links for every dep in your manifest
- **GitHub inlining** — attach a specific issue / PR / release body with `gh://`
- **Auto-suggest** — flags when your files import a library but the docs aren't attached
- **Memory** — persistent notes across sessions so the agent remembers architectural decisions
- **MCP server** — the agent can call ctxforge directly instead of you copy-pasting
- **Interactive TUI** — scenario-aware prompt engineer with live preview, not just a CLI dumper
- **Token discipline** — per-item token accounting, hotspot highlighting, model-aware budget
- **Deterministic** — same inputs always produce the same prompt bytes; no LLM randomness

</details>

<details>
<summary>Does ctxforge send my code anywhere?</summary>

**No.** ctxforge runs entirely on your local machine. Network calls are explicit and bounded:

- `ctxforge docs detect` → registry JSON metadata (crates.io, npm, PyPI) — cached 7 days
- `ctxforge add https://...` → the URL you specified — cached per the URI
- `ctxforge add gh://...` → GitHub REST API / raw.githubusercontent.com — cached with per-resource TTLs
- `--offline` on export skips all network; serves from cache or placeholder

No telemetry. No analytics. No API keys required (`GITHUB_TOKEN` is optional). The MCP server communicates exclusively via stdio with your local AI agent.

</details>

<details>
<summary>What languages / ecosystems are supported?</summary>

| Feature | Rust | JS/TS | Python | Go |
|---|---|---|---|---|
| `docs detect` (manifests) | ✓ Cargo.toml + Cargo.lock | ✓ package.json | ✓ pyproject.toml + requirements.txt | ✓ go.mod |
| `suggest` (import scan) | ✓ `use` + `extern crate` | ✓ `import` / `require` | ✓ `import` / `from` | ✓ `import` blocks |
| `add --fn` / `--type` (tree-sitter) | ✓ | ✓ `.js` / `.ts` / `.jsx` / `.tsx` | ✓ | ✓ |
| `gh://` (forge) | n/a — works for any repo | n/a | n/a | n/a |

</details>

---

## Project status

ctxforge has shipped the full P-series roadmap (P1 through P5):

- **P5** — Context source abstraction (URI, HMAC cache, fetcher, provenance)
- **P1** — Library docs gatherer (`ctxforge docs detect/add/rm/list/refresh`)
- **P3** — GitHub enrichment (forge URLs + `gh://` source variant)
- **P4** — Auto-suggest (`ctxforge suggest [--apply]`)
- **Parity** — CLI / MCP (31 tools) / TUI (48 palette entries) / Claude plugin all in sync

See [CHANGELOG.md](CHANGELOG.md) for the full version history.

---

## Contributing

Contributions welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.

All contributions are subject to the [Contributor License Agreement](CLA.md), which assigns copyright to Sylvester Francis (same model as Qt / MongoDB / Canonical).

---

## License

ctxforge is licensed under [GNU Affero General Public License v3.0 or later](LICENSE) (AGPL-3.0-or-later).

Use it freely for any purpose. Modify and distribute freely. If you distribute a modified version or run it as a network service, your modifications must also be AGPL-3.0-or-later.

Copyright © 2026 Sylvester Francis. All rights reserved. See [NOTICE](NOTICE).

---

<p align="center">
  <a href="https://github.com/sylvester-francis/ctx-forge/issues">Issues</a> ·
  <a href="https://github.com/sylvester-francis/ctx-forge/discussions">Discussions</a> ·
  <a href="https://crates.io/crates/ctxforge">crates.io</a>
</p>
