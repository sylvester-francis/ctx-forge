# ctxforge — Claude Code Plugin

Deterministic prompt engineer for AI coding agents. Assemble, compress, and export context bundles with live token counting, per-dep doc attachment, and gap analysis. 25 slash commands, 31 MCP tools. **Never calls an LLM — the agent does.**

## Prerequisites

Install the ctxforge binary (required for MCP server and for `/ctxforge:pipe`):

```bash
cargo install ctxforge
```

## Installation

From the `claude-community` marketplace:

```bash
claude plugin install ctxforge
```

For local development:

```bash
claude --plugin-dir ./ctxforge-plugin
```

## Quickstart

```
You: /ctxforge:bugfix "null deref in auth middleware"
Claude: [calls ctxforge_status — bundle is empty]
        Your bundle is empty. Run /ctxforge (guided workflow) or /ctxforge:add <paths> first.

You: /ctxforge:add src/middleware/auth.rs src/middleware/session.rs
Claude: [calls ctxforge_add_files]
        Added 2 items, 2,000 tokens.

You: /ctxforge:bugfix "null deref in auth middleware"
Claude: [calls MCP prompt ctxforge_bugfix]
        <rendered bugfix prompt wrapping the bundle — ready to paste into any agent>
```

## Commands

### Workflow

| Slash | Description |
|-------|-------------|
| `/ctxforge` | Guided bundle-building workflow (status → recall → ask → build → report). |

### Bundle editing

| Slash | Description |
|-------|-------------|
| `/ctxforge:add` | Add files, globs, ranges, functions, types, URLs, or gh:// resources. |
| `/ctxforge:rm` | Remove items by path or index. |
| `/ctxforge:clear` | Empty the current bundle. |

### Inspection

| Slash | Description |
|-------|-------------|
| `/ctxforge:status` | Token count vs. model window. |
| `/ctxforge:list` | Enumerate items (or sources with `--sources`). |
| `/ctxforge:resume` | Bundle + most-recent memory notes. |

### Output

| Slash | Description |
|-------|-------------|
| `/ctxforge:export` | Render to chat or file. `--xml`, `--json`, `--template`, `--task`. |
| `/ctxforge:copy` | Render and copy to system clipboard. |
| `/ctxforge:pipe` | Pipe rendered bundle to another CLI (claude / agent / gemini / custom). Requires `ctxforge` on PATH. |

### Profiles

| Slash | Description |
|-------|-------------|
| `/ctxforge:save <name>` | Save current bundle as a named profile. |
| `/ctxforge:load <name>` | Restore a saved profile. |
| `/ctxforge:profiles` | List / remove profiles. |

### Templates

| Slash | Description |
|-------|-------------|
| `/ctxforge:templates` | List / apply / create / delete prompt templates. Built-in starters: bugfix, code-review, explain, refactor, migrate. |

### Memory

| Slash | Description |
|-------|-------------|
| `/ctxforge:note` | Write a memory note (optional `--tag`). |
| `/ctxforge:recall` | Search notes by tag / text / recency. |

### Docs & deps

| Slash | Description |
|-------|-------------|
| `/ctxforge:docs` | detect / add / list / rm / refresh dep-docs entries. |
| `/ctxforge:suggest` | Find missing / stale docs entries. `--apply` fixes them. |

### Cache

| Slash | Description |
|-------|-------------|
| `/ctxforge:cache` | list / clear (`--stale` / `--all`) / verify. |
| `/ctxforge:refresh` | Re-fetch stale (or all with `--all`) URL / gh:// sources. |

### Scenarios — output IS the crafted prompt

| Slash | Description |
|-------|-------------|
| `/ctxforge:bugfix "<task>"` | Render a bugfix prompt wrapping the bundle. |
| `/ctxforge:code-review "<task>"` | Render a code-review prompt. |
| `/ctxforge:explain "<task>"` | Render an explain prompt. |
| `/ctxforge:refactor "<task>"` | Render a refactor prompt. |
| `/ctxforge:migrate "<task>"` | Render a migration-steps prompt. |

## Cross-session memory example

```
You: /ctxforge:note --tag auth "Using JWT with RS256, not HS256"
Claude: Noted: [2026-04-18 14:30 UTC] #auth Using JWT with RS256, not HS256

--- next session ---

You: /ctxforge:recall --tag auth
Claude: Found 3 notes tagged "auth":
  - Using JWT with RS256, not HS256
  - Token refresh interval is 15 minutes
  - Refresh tokens stored in httpOnly cookies
```

## Profile-based workflow switching

```
You: /ctxforge:save frontend
Claude: Saved profile "frontend" (12 items).

You: /ctxforge:load backend
Claude: Loaded profile "backend" (8 items, 15,000 tokens).
```

## MCP tools (31)

The plugin launches the MCP server via `ctxforge mcp` (stdio). The model can call every tool directly; slash commands are convenience wrappers.

Tool list: `ctxforge_add_files`, `ctxforge_add_function`, `ctxforge_add_type`, `ctxforge_add_url`, `ctxforge_remove`, `ctxforge_clear`, `ctxforge_status`, `ctxforge_list_items`, `ctxforge_list_sources`, `ctxforge_export`, `ctxforge_save_bundle`, `ctxforge_load_bundle`, `ctxforge_list_profiles`, `ctxforge_profiles_rm`, `ctxforge_list_templates`, `ctxforge_apply_template`, `ctxforge_templates_new`, `ctxforge_templates_rm`, `ctxforge_note`, `ctxforge_recall`, `ctxforge_docs_detect`, `ctxforge_docs_add`, `ctxforge_docs_list`, `ctxforge_docs_rm`, `ctxforge_docs_refresh`, `ctxforge_suggest`, `ctxforge_suggest_apply`, `ctxforge_cache_list`, `ctxforge_cache_clear`, `ctxforge_cache_verify`, `ctxforge_refresh`.

MCP prompts: `ctxforge_bugfix`, `ctxforge_code_review`, `ctxforge_explain`, `ctxforge_refactor`, `ctxforge_migrate`.

## Links

- [ctxforge repository](https://github.com/sylvester-francis/ctx-forge)
- [claude-community marketplace](https://github.com/anthropics/claude-community)
- [MCP protocol specification](https://modelcontextprotocol.io)
