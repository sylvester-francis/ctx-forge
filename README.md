# ctxforge

> The missing TUI for AI coding. Assemble perfect context in 10 seconds.

`ctxforge` is a Rust CLI that helps developers assemble and preserve the perfect context for their AI coding agents. It is the missing interactive composer between your codebase and Claude Code / Cursor / Aider, and it gives those agents persistent memory across sessions via a built-in MCP server.

It implements all four strategies of context engineering — **Write, Select, Compress, Isolate** — as a single Unix tool.

---

## Why ctxforge

Every morning, developers open their AI coding agent and re-explain the same decisions: which crate versions, which architectural trade-offs, which abandoned refactors, which failed experiments. The agent has no memory. The developer is the memory.

`ctxforge` fixes two pains at once:

1. **Composing context is tedious.** You copy-paste files, guess at token budgets, and hope your agent isn't about to grab the wrong 200KB of noise. ctxforge gives you a visceral, interactive TUI with a live token gauge so you can *see* what you're spending.
2. **Agents forget everything between sessions.** ctxforge gives Claude Code, Cursor, and Zed persistent, durable memory in one command via an MCP server — no cloud, no API keys, no database.

---

## Features

### Interactive TUI composer

Run `ctxforge` with no args to open a fullscreen ratatui composer:

```
┌─ ctxforge ──── profile: watchdog-hub ──── model: claude-sonnet-4 ─────────┐
│  tokens  ████████████░░░░░░░░░░░░░░░░░░░░░░░   12,847 / 200,000  6.4%  │
├─ files ───────────────────────┬─ bundle (6 items, ordered) ────────────┤
│  /search▏                     │  1 ■ src/hub/server.go     1,204  9.4% │
│                               │  2 ■ src/hub/check.go      1,837 14.3% │
│  ▾ src/                       │    └─ lines 45-120                     │
│    ▾ hub/                     │  3 λ fn:ProcessCheck         187  1.5% │
│      ■ server.go        1,204 │  4 ■ README.md               892  6.9% │
│      ■ check.go         1,837 │  5 ■ docs/architecture.md  7,912 61.6% │
│        ▸ fn:ProcessCheck  187 │  6 ■ PROMPT.md                18  0.1% │
│        ▸ fn:Validate      203 │                                        │
│      ▫ server_test.go     445 │  ── Hotspot ────────────────────────── │
│    ▾ config/                  │  5 consumes 61% of the budget.         │
│      ▫ config.go          612 │  Narrow to :10-50? [press n]           │
│    ▫ main.go              301 │                                         │
│  ■ README.md              892 │                                         │
│  ▸ docs/                      │                                         │
│  ▫ Cargo.toml             128 │                                         │
│                               │                                         │
├───────────────────────────────┴─────────────────────────────────────────┤
│  space toggle  ↵ expand  / search  f fn  t type  d diff  n narrow       │
│  c copy     p pipe claude     x export xml     s save    q quit         │
└─────────────────────────────────────────────────────────────────────────┘
```

Highlights:

- **Live token gauge** — animates as you toggle items, color-grades green → yellow → orange → red as you approach the model's window.
- **Hotspot panel** — flags any single item consuming more than 25% of your budget and offers to narrow it with one keystroke.
- **Fuzzy search** — `/` searches paths *and* function names (when built with `--features=extract`).
- **Pipe menu** — `p` opens a submenu to hand your bundle straight to `claude`, `cursor-agent`, `gemini`, or OpenRouter.
- **Memory panel** — `j` shows notes filtered by profile; `J` adds a note without leaving the composer.
- **Model switcher** — `m` recalculates the gauge against a different model window, live.

### Persistent memory across sessions

ctxforge is the persistent scratchpad that lives *between* agent sessions.

```bash
ctxforge note "decided tokio::select! over join! because we need cancellation on timeout"
ctxforge note --tag auth "middleware validates JWT from Authorization header, not cookies"
ctxforge note --tag tls "abandoned rustls 0.22 upgrade — breaks tonic 0.10; revisit after 0.11"

ctxforge recall                      # show recent notes
ctxforge recall --search "TLS"       # filter by content
ctxforge recall --tag auth           # filter by tag
ctxforge recall --since 1w           # filter by time

ctxforge resume                      # restore last bundle + attach relevant notes
```

Notes are stored as human-readable markdown in `.ctxforge/memory/` — git-committable, grep-able, no database, no daemon, no corruption risk.

### MCP server — give Claude Code persistent memory in one command

```bash
claude mcp add ctxforge -- ctxforge mcp
```

That's it. Claude Code now has persistent memory across sessions. It can write decisions to itself at task-completion checkpoints and read them back at the start of the next session.

Exposed MCP tools:

| Tool | Description |
|---|---|
| `ctxforge_recall` | Search memory, filtered by tag / keyword / recency |
| `ctxforge_note` | Write a decision or learning to memory |
| `ctxforge_load_bundle` | Pull a saved profile's files into the agent's context |
| `ctxforge_status` | Check token budget against the current model window |

Works with any MCP client — Claude Code, Cursor, Zed.

---

## Install

```bash
cargo install ctxforge
```

Optional feature flags:

```bash
# Add --fn / --type extraction via tree-sitter (Go, Rust, Python, TS/JS)
cargo install ctxforge --features=extract

# CLI-only build for scripting / CI
cargo install ctxforge --no-default-features --features=minimal
```

---

## CLI surface

The TUI is the hero, but every action has a CLI equivalent so your workflows stay scriptable.

### Adding context

```bash
ctxforge add src/hub/**/*.go --exclude '*_test.go'
ctxforge add src/hub/server.go:45-120
ctxforge add --fn ProcessCheck src/hub/check.go
ctxforge add --fn handleRequest --fn validateConfig src/hub/server.go
ctxforge add --type Config src/config/config.go
ctxforge add --diff main
```

### Inspecting

```bash
ctxforge status                  # bundle summary with token counts + percentages
ctxforge status --model gpt-4o   # recount for a different model
```

### Managing

```bash
ctxforge rm src/hub/server.go    # remove by path
ctxforge rm 3                    # remove by index
ctxforge clear                   # remove all items
```

### Exporting

```bash
ctxforge copy                    # clipboard (markdown)
ctxforge copy --xml              # clipboard (XML)
ctxforge export -o prompt.md     # write to file
ctxforge export --xml            # stdout XML
ctxforge export --json           # stdout JSON
```

### Pipe to your agent

```bash
ctxforge pipe claude             # pipe to claude CLI (XML format)
ctxforge pipe cursor-agent       # pipe to cursor-agent
ctxforge pipe gemini             # pipe to gemini CLI (markdown)
ctxforge pipe -- <any-command>   # escape hatch: pipe to any stdin-accepting command
```

`pipe` is a real subcommand, not a shell afterthought. It:

- Spawns the target CLI as a subprocess and writes the formatted bundle to its stdin
- Auto-detects whether the target is installed and errors clearly if not
- Picks the optimal format per agent (XML for Claude, markdown for others)
- Never makes HTTP calls of its own — the target CLI holds credentials

### Profiles

```bash
ctxforge save watchdog-hub
ctxforge load watchdog-hub
ctxforge profiles                # list
ctxforge profiles rm old-one
```

### Continuity

```bash
ctxforge note "..."
ctxforge note --tag auth "..."
ctxforge recall
ctxforge recall --search "TLS"
ctxforge resume
ctxforge mcp                     # start MCP server (used by MCP clients)
```

---

## The four pillars of context engineering

ctxforge is designed around four strategies for managing an agent's context window. Every feature maps to one of them:

| Pillar | What it means | How ctxforge does it |
|---|---|---|
| **Write** | Persist decisions across sessions | `ctxforge note`, `ctxforge recall`, `ctxforge resume`, MCP memory tools |
| **Select** | Choose what the agent sees | `ctxforge add` with files, line ranges, `--fn`, `--type`, `--diff`; the TUI composer |
| **Compress** | Stay within the token budget | Live token gauge + hotspot panel (user-driven, never LLM summarization) |
| **Isolate** | Keep contexts separate per task | Profiles (save/load, git-committable, scoped memory tags) |

---

## File layout on disk

```
project/
├── .ctxforge/
│   ├── bundle.json             # current working bundle (gitignored)
│   ├── profiles/
│   │   ├── watchdog-hub.json   # save/load targets, optionally committed
│   │   └── dkl-core.json
│   └── memory/
│       ├── decisions.md        # untagged notes
│       ├── auth.md             # per-tag files
│       ├── tls.md
│       └── _index.jsonl        # timestamped index
└── ...
```

**Git treatment:**

- `.ctxforge/bundle.json` — gitignored (session-specific)
- `.ctxforge/profiles/*.json` — optionally committed for team-shared contexts
- `.ctxforge/memory/*.md` — committable for team-shared decisions; user decides per-file

---

## Design principles

- **ctxforge never calls an LLM itself.** MCP flips the direction: LLMs call ctxforge.
- **No network calls.** State, memory, and bundles all live in `.ctxforge/` on disk.
- **No API keys.** `ctxforge pipe` hands off to whatever LLM CLI you already have installed — it holds its own credentials.
- **No auto-selection heuristics.** The user decides what is relevant; ctxforge makes the selection visceral and fast.
- **No summarization.** Compression decisions are yours, guided by the live token gauge.
- **ctxforge does not replace the agent.** It is a tool, not a framework.

---

## Project status

ctxforge is under active development. The v1.0 surface is ruthlessly scoped around two hero features — the TUI composer and cross-session continuity via MCP — plus a complete CLI for every action the TUI exposes.

Supported languages for `--fn` / `--type` extraction at launch: Go, Rust, Python, TypeScript/JavaScript.

---

## License

ctxforge is licensed under **[GNU Affero General Public License v3.0 or later](LICENSE)** (AGPL-3.0-or-later).

### What this means for you

- **Use it freely** — run ctxforge for any purpose, personal or commercial.
- **Modify it freely** — fork, patch, experiment, go wild.
- **Distribute it freely** — share your fork, publish your modifications.
- **But**: if you distribute a modified version or run it as a hosted network service, your modifications must also be released under AGPL-3.0-or-later. No SaaS wrappers that hide the source.

### Contributing

All contributions are subject to the [Contributor License Agreement](CLA.md), which assigns copyright in your contribution to Sylvester Francis. This is the same model used by Qt, MongoDB, and Canonical — it lets the project be maintained and relicensed over its lifetime without requiring approval from every past contributor. See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Copyright

Copyright © 2026 Sylvester Francis. All rights reserved.

ctxforge is created and maintained by [Sylvester Francis](https://github.com/sylvester-francis). Contributions from the community are welcome under the terms of the CLA.
