# I'm Tired of Explaining My Codebase to Claude Code Every Morning. So I Built ctxforge.

Six months ago I wrote about context engineering as the operating system for AI agents. I described four strategies — Write, Select, Compress, Isolate — for managing what an LLM sees. Today I'm releasing the tool that implements all four.

---

Every morning for the last three weeks, I opened Claude Code and re-explained my codebase to it. The TLS decision. The auth middleware. Why we're still on tonic 0.10. The module boundaries. The abandoned refactor.

The agent has no memory. I am the memory. And I'm tired.

If you use Claude Code, Cursor, Aider, or even paste code into ChatGPT, you know exactly what I mean. You do the same five things dozens of times a day:

1. Open files, scroll to the relevant section
2. Copy code into a prompt
3. Hope you haven't blown the token budget
4. Realize you forgot a key file and start over
5. Re-explain the same decisions you made last week

I built **ctxforge** to fix all five.

## What ctxforge actually does

ctxforge is a Rust CLI that assembles context bundles for your AI coding agent. It's the missing tool between your codebase and Claude Code.

Run it with no arguments and you get a fullscreen TUI:

```
┌─ ctxforge │ claude-sonnet-4 │ ~4,365 / 200,000 (2.2%) ──────────┐
│ tokens ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░                  │
├─ files ──────────────────┬─ bundle (6 items) ────────────────────┤
│ ▸ src/                   │  1  server.go            1,204  27.6% │
│   ■ server.go       1204 │  2  check.go:45-120        423   9.7% │
│   ■ check.go        1837 │  3  fn:ProcessCheck        187   4.3% │
│ ■ README.md          892 │  4  README.md              892  20.4% │
│ ▸ docs/                  │  5  architecture.md      1,641  37.6% │
│                          │  6  PROMPT.md               18   0.4% │
├──────────────────────────┴───────────────────────────────────────┤
│ ␣ toggle  j/k move  ↹ panel  c copy  q quit                     │
└──────────────────────────────────────────────────────────────────┘
```

That bar at the top? That's a live token gauge. It fills and color-grades — green, yellow, orange, red — as you toggle files. For the first time, you can **see** what you're spending.

And that file at 37.6%? The hotspot panel highlights it in orange. One file is eating a third of your budget. You didn't know until now.

## The four pillars

ctxforge is designed around the four strategies of context engineering. Every feature maps to one:

**Write** — Agents forget everything between sessions. ctxforge gives them persistent memory:

```bash
ctxforge note --tag auth "JWT validated from Authorization header, not cookies"
ctxforge recall --tag auth
ctxforge resume  # pick up where you left off
```

Notes are stored as human-readable markdown in `.ctxforge/memory/`. Git-committable. Grep-able. No database.

**Select** — You decide what the agent sees, but the tools make it fast:

```bash
ctxforge add src/**/*.go --exclude '*_test.go'
ctxforge add src/hub/check.go:45-120
ctxforge add --fn ProcessCheck src/hub/check.go  # tree-sitter extraction
ctxforge add --diff main  # only changed files
```

**Compress** — The live token gauge and percentage display make the invisible visible. When architecture.md is 37% of your context, you know to narrow it.

**Isolate** — Profiles keep work streams separate:

```bash
ctxforge save watchdog-hub
ctxforge load dkl-core  # completely separate context
```

## The scary part: persistent memory for Claude Code in one command

```bash
claude mcp add --transport stdio ctxforge -- ctxforge mcp
```

That's it. Claude Code now has persistent memory across sessions. It can write decisions to itself and read them back next time you open a conversation.

ctxforge exposes four MCP tools:

| Tool | What it does |
|------|-------------|
| `ctxforge_recall` | Agent searches its own memory |
| `ctxforge_note` | Agent writes a decision to memory |
| `ctxforge_load_bundle` | Agent pulls a saved profile |
| `ctxforge_status` | Agent checks its token budget |

The agent writes notes. The agent reads them back. You stop being the memory.

## Three export formats, one pipe

ctxforge doesn't just copy to clipboard. It exports in three formats:

- **Markdown** (default) — fenced code blocks, universal
- **XML** — Claude-optimized with semantic `<source>` / `<documentation>` / `<memory>` tags
- **JSON** — typed schema for API consumers and pipeline integration

And you can pipe directly to your agent:

```bash
ctxforge pipe claude        # auto-selects XML
ctxforge pipe cursor-agent  # auto-selects markdown
ctxforge pipe gemini
```

## Built in Rust. No cloud. No API keys.

ctxforge is a single static binary. It runs entirely on your machine. It never makes a network call. It never sends your code anywhere. It doesn't need an API key. It doesn't need a database.

Everything lives in `.ctxforge/` on disk. Human-readable. Git-committable. Yours.

## Install it now

```bash
cargo install ctxforge
```

For function/type extraction with tree-sitter (Go, Rust, Python, TypeScript, JavaScript):

```bash
cargo install ctxforge --features=extract
```

The repo: https://github.com/sylvester-francis/ctx-forge

Licensed under AGPL-3.0-or-later. Open source. Built on nights and weekends because I was tired of being my AI's short-term memory.

If you use Claude Code, Cursor, or Aider — try it. And if it saves you time, a star on GitHub would mean a lot.

---

*Sylvester Francis is a software engineer building tools for AI-assisted development. ctxforge is his first open-source Rust project.*
