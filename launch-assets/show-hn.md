# Show HN Post — Day 3

---

**Title:** Show HN: ctxforge – A TUI and MCP server that gives your AI coding agent persistent memory

---

**Body:**

Hey HN,

I built ctxforge because I was tired of re-explaining my codebase to Claude Code every morning. It's a Rust CLI that does two things:

1. An interactive TUI that lets you assemble context bundles with a live token gauge — you can finally see what you're spending before you send it.

2. An MCP server that gives Claude Code (and any MCP client) persistent memory across sessions. One command: `claude mcp add --transport stdio ctxforge -- ctxforge mcp`. Your agent writes decisions to itself and reads them back.

It implements the four strategies of context engineering (Write, Select, Compress, Isolate) as a single Unix tool. No cloud, no API keys, no database. Everything in `.ctxforge/` on disk.

Features:
- TUI with live token gauge (green → yellow → orange → red)
- Persistent memory notes with tags and search
- XML export (Claude-optimized semantic tags), JSON, markdown
- Pipe directly to agent CLIs (`ctxforge pipe claude`)
- Tree-sitter function/type extraction (Go, Rust, Python, TS/JS)
- Profiles for isolated work streams
- MCP server (protocol 2025-11-25, 4 tools)

Install: `cargo install ctxforge`

With tree-sitter: `cargo install ctxforge --features=extract`

Repo: https://github.com/sylvester-francis/ctx-forge

Written in Rust (edition 2024). AGPL-3.0 licensed. ~3500 lines of code, 173 tests.

Full article with the context engineering framework: [Medium link]

Would love feedback. Especially interested in:
- What workflows would you use it for?
- What languages should tree-sitter support next?
- Would you use the MCP integration with Claude Code?

---

**Posting notes:**
- Post on Day 3 (Thursday), 9-10 AM EST
- By Day 3, the story has been battle-tested on primary channels
- Reply to every comment within 2 hours
- HN title leads with MCP/memory because "persistent memory for AI agents" is the hotter HN frame
