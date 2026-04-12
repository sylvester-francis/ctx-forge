# ctxforge — Claude Code Plugin

Context engineering for AI coding agents. Assemble, compress, and export perfect context bundles with live token counting.

## Prerequisites

Install ctxforge:

```bash
cargo install ctxforge
```

## Installation

```bash
claude plugin install ctxforge
```

Or for local development:

```bash
claude --plugin-dir ./ctxforge-plugin
```

## Usage

### Example 1: Debugging Session

Type `/ctxforge` in Claude Code to start building context.

```
You: /ctxforge
Claude: Your bundle is empty. What are you working on?
You: There's a null pointer in the auth middleware
Claude: Let me build context for that...
  - Added src/middleware/auth.rs (1,200 tokens)
  - Added src/middleware/session.rs (800 tokens)
  - Recalled 2 notes tagged "auth"
  Bundle: 3 items, 2,000 tokens (1.0% of 200k window)
```

### Example 2: Profile-Based Workflow Switching

Save bundles as profiles to switch between work areas quickly.

```
You: Save this as "frontend" and switch to the backend
Claude: [calls ctxforge_save_bundle with name "frontend"]
  Saved profile "frontend" (12 items)
  [calls ctxforge_load_bundle with profile "backend"]
  Loaded profile "backend" (8 items, 15,000 tokens)
```

### Example 3: Cross-Session Memory

Write notes during a session; recall them in the next one.

```
You: Remember that we're using JWT with RS256, not HS256
Claude: [calls ctxforge_note with tag "auth"]
  Noted: [2026-04-12 14:30 UTC] Using JWT with RS256, not HS256

--- next session ---

You: /ctxforge — I'm working on the auth refresh flow
Claude: [calls ctxforge_recall with tag "auth"]
  Found 3 notes tagged "auth":
  - Using JWT with RS256, not HS256
  - Token refresh interval is 15 minutes
  - Refresh tokens stored in httpOnly cookies
```

## Tools Reference

| Tool | Description |
|------|-------------|
| `ctxforge_add_files` | Add files, globs, or line ranges to the bundle |
| `ctxforge_add_function` | Add a function by name (tree-sitter) |
| `ctxforge_add_type` | Add a type/struct by name (tree-sitter) |
| `ctxforge_remove` | Remove items by path or index |
| `ctxforge_clear` | Clear entire bundle |
| `ctxforge_export` | Export bundle (markdown/xml/json) |
| `ctxforge_list_items` | List items with token counts |
| `ctxforge_status` | Check token budget |
| `ctxforge_save_bundle` | Save bundle as named profile |
| `ctxforge_load_bundle` | Load a saved profile |
| `ctxforge_list_profiles` | List available profiles |
| `ctxforge_recall` | Search memory notes |
| `ctxforge_note` | Write a persistent memory note |
| `ctxforge_list_templates` | List prompt templates |
| `ctxforge_apply_template` | Render a template with bundle content |

## Links

- [ctxforge repository](https://github.com/sylvester-francis/ctx-forge)
- [MCP protocol specification](https://modelcontextprotocol.io)
