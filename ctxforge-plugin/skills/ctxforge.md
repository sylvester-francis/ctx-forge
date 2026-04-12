---
name: ctxforge
description: Use when the user invokes /ctxforge or asks to build, manage, or export a context bundle for AI coding. On-demand only.
---

# ctxforge — Context Engineering Skill

You have access to ctxforge MCP tools for assembling context bundles with live token counting.

## When to Use

- User invokes `/ctxforge`
- User asks to "build context", "assemble a bundle", "add files to context", or similar

Do NOT use proactively — only when the user asks.

## Workflow

### 1. Assess Current State

Call `ctxforge_status` to check if a bundle already exists.
Call `ctxforge_recall` to retrieve any relevant prior memory notes.

Report what you find: "You have X items (Y tokens, Z% of context window)" or "Bundle is empty, let's build one."

### 2. Understand the Task

Ask the user what they're working on if not clear. This determines which files, functions, and types to include.

### 3. Build the Bundle

Use these tools to assemble context:

- **`ctxforge_add_files`** — Add files by path, glob pattern, or line range (`path:start-end`)
- **`ctxforge_add_function`** — Add a specific function by name (requires file path)
- **`ctxforge_add_type`** — Add a specific type/struct by name (requires file path)

Start narrow (specific files relevant to the task), then expand if needed. Prefer adding functions and types over whole files when only part of a file is relevant.

### 4. Monitor Token Budget

Call `ctxforge_status` after adding items. Watch the budget:

- **Under 50%**: Plenty of room, add more if helpful
- **50-75%**: Good range for most tasks
- **Over 75%**: Consider trimming — use `ctxforge_remove` to drop low-value items
- **Over 90%**: Too full — the model needs room for its response

Use `ctxforge_list_items` to see per-item token counts and identify what to trim.

### 5. Persist (Optional)

- **`ctxforge_save_bundle`** — Save as a named profile if this bundle is reusable (e.g., "frontend", "api-layer")
- **`ctxforge_note`** — Write memory notes for decisions, gotchas, or context that's useful across sessions

### 6. Report

Tell the user what was assembled:
- Number of items
- Total tokens and percentage of context window
- Key files/functions included

## Tool Reference

| Tool | Purpose | Modifies Bundle? |
|------|---------|-----------------|
| `ctxforge_status` | Check token count and budget | No |
| `ctxforge_recall` | Search memory notes | No |
| `ctxforge_note` | Write a memory note | No |
| `ctxforge_add_files` | Add files/globs/ranges | Yes |
| `ctxforge_add_function` | Add function by name | Yes |
| `ctxforge_add_type` | Add type by name | Yes |
| `ctxforge_remove` | Remove items | Yes |
| `ctxforge_clear` | Clear entire bundle | Yes |
| `ctxforge_export` | Export bundle content | No |
| `ctxforge_list_items` | List items with token counts | No |
| `ctxforge_save_bundle` | Save bundle as profile | No |
| `ctxforge_load_bundle` | Load a saved profile | Yes |
| `ctxforge_list_profiles` | List saved profiles | No |
| `ctxforge_list_templates` | List prompt templates | No |
| `ctxforge_apply_template` | Render a template with bundle | No |

## Best Practices

- **Start narrow, expand as needed** — Don't add `**/*`. Add the specific files for the task.
- **Functions over files** — If only one function matters, add it instead of the whole file.
- **Watch the gauge** — The token budget is your constraint. Respect it.
- **Use profiles** — Save useful bundles for repeated workflows.
- **Write notes** — Record architectural decisions and gotchas for future sessions.
- **Load before switching** — When changing work areas, save the current profile first, then load the relevant one.
