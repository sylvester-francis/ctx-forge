---
name: ctxforge
description: Use when the user invokes /ctxforge or any /ctxforge:* slash command, or asks to build, manage, or export a context bundle for AI coding. On-demand only.
---

# ctxforge — Context Engineering Skill

**ctxforge v2.0+ — 31 MCP tools, 25 slash commands.** Tool count drift signal: if these numbers don't match `plugin.json`, bump both files together.

You have access to 31 ctxforge MCP tools for assembling token-disciplined prompts with per-dep documentation links, pinned GitHub resources, and gap detection.

## When to Use

- User invokes `/ctxforge` (guided workflow) or any `/ctxforge:*` slash command.
- User says one of: "build context", "assemble a bundle", "add files to context", "attach docs for this dep", "inline this GitHub issue", "check token budget", "save this as a profile", "recall my notes on X", "render a bugfix prompt".

**Do NOT use proactively.** Only when the user asks.

## Capability index — intent → tool → slash

| User says | MCP tool | Matching slash |
|-----------|----------|----------------|
| "add the auth middleware file" | `ctxforge_add_files` | `/ctxforge:add <path>` |
| "add files matching src/auth/*" | `ctxforge_add_files` (glob) | `/ctxforge:add src/auth/*` |
| "add lines 10–40 of main.rs" | `ctxforge_add_files` (range) | `/ctxforge:add main.rs:10-40` |
| "pull in just the handleLogin function" | `ctxforge_add_function` | `/ctxforge:add --fn handleLogin <file>` |
| "add just the Config type" | `ctxforge_add_type` | `/ctxforge:add --type Config <file>` |
| "add files changed vs main" | `ctxforge_add_files` (`--diff`) | `/ctxforge:add --diff main` |
| "attach this URL / GitHub issue" | `ctxforge_add_files` (URL auto-normalises) | `/ctxforge:add <url>` |
| "attach this gh:// resource" | `ctxforge_add_files` | `/ctxforge:add gh:///owner/repo/pulls/N` |
| "remove item 3 / auth.rs" | `ctxforge_remove` | `/ctxforge:rm 3` |
| "clear the bundle" | `ctxforge_clear` | `/ctxforge:clear` |
| "how much of my context am I using" | `ctxforge_status` | `/ctxforge:status` |
| "list what's in the bundle" | `ctxforge_list_items` | `/ctxforge:list` |
| "show me stale URL items" | `ctxforge_list_sources` | `/ctxforge:list --sources --stale` |
| "what was I working on last time" | `ctxforge_status` + `ctxforge_recall` | `/ctxforge:resume` |
| "export as markdown / XML / JSON" | `ctxforge_export` | `/ctxforge:export --xml` |
| "copy to clipboard" | `ctxforge_export` + pbcopy/wl-copy/xclip | `/ctxforge:copy` |
| "pipe to claude / gemini / agent" | (shells out to `ctxforge pipe`) | `/ctxforge:pipe <target>` |
| "save as backend / pr-123" | `ctxforge_save_bundle` | `/ctxforge:save <name>` |
| "load the backend profile" | `ctxforge_load_bundle` | `/ctxforge:load <name>` |
| "list / delete profiles" | `ctxforge_list_profiles`, `ctxforge_profiles_rm` | `/ctxforge:profiles [rm <name>]` |
| "list templates" | `ctxforge_list_templates` | `/ctxforge:templates` |
| "render the bugfix template" | `ctxforge_apply_template` | `/ctxforge:templates apply bugfix "<task>"` |
| "scaffold a new template from explain" | `ctxforge_templates_new` | `/ctxforge:templates new <name> --from explain` |
| "remember this: X" | `ctxforge_note` | `/ctxforge:note [--tag <t>] <body>` |
| "what notes do I have on auth" | `ctxforge_recall` | `/ctxforge:recall --tag auth` |
| "attach docs for my deps" | `ctxforge_docs_detect` | `/ctxforge:docs` |
| "add docs for tokio" | `ctxforge_docs_add` | `/ctxforge:docs add tokio --ecosystem rust` |
| "list attached docs" | `ctxforge_docs_list` | `/ctxforge:docs list` |
| "remove clap from docs" | `ctxforge_docs_rm` | `/ctxforge:docs rm clap` |
| "re-read lock files, update versions" | `ctxforge_docs_refresh` | `/ctxforge:docs refresh` |
| "find missing docs / unused docs" | `ctxforge_suggest` | `/ctxforge:suggest` |
| "apply the suggestions" | `ctxforge_suggest_apply` | `/ctxforge:suggest --apply` |
| "list cache entries" | `ctxforge_cache_list` | `/ctxforge:cache list` |
| "clear stale cache" | `ctxforge_cache_clear` | `/ctxforge:cache clear --stale` |
| "verify cache integrity" | `ctxforge_cache_verify` | `/ctxforge:cache verify` |
| "refresh stale URLs" | `ctxforge_refresh` | `/ctxforge:refresh` |
| "render a bugfix prompt" | MCP prompt `ctxforge_bugfix` | `/ctxforge:bugfix "<task>"` |
| "render a code-review prompt" | MCP prompt `ctxforge_code_review` | `/ctxforge:code-review "<task>"` |
| "render an explain prompt" | MCP prompt `ctxforge_explain` | `/ctxforge:explain "<task>"` |
| "render a refactor prompt" | MCP prompt `ctxforge_refactor` | `/ctxforge:refactor "<task>"` |
| "render a migration prompt" | MCP prompt `ctxforge_migrate` | `/ctxforge:migrate "<task>"` |

## Workflow recipes

### Debugging a specific bug

1. `/ctxforge:status` — see current state.
2. `/ctxforge:recall --tag <area>` — pull prior context for the affected area.
3. `/ctxforge:add <path>` or `/ctxforge:add --fn <name> <file>` — add offending code.
4. `/ctxforge:add https://github.com/.../issues/N` — attach the bug report if there is one.
5. `/ctxforge:bugfix "<one-line description>"` — render the prompt.
6. Take the rendered prompt to your agent; once fixed, `/ctxforge:note --tag <area> "resolved: <summary>"`.

### Reviewing a PR

1. `/ctxforge:add --diff <base-branch>` — add changed files.
2. `/ctxforge:add https://github.com/.../pull/N` — attach the PR description.
3. `/ctxforge:docs detect` — attach docs for deps touched.
4. `/ctxforge:suggest` — check for missing / stale docs.
5. `/ctxforge:code-review "<focus>"` — render the review prompt.

### Cross-session resumption

1. `/ctxforge:resume` — bundle + last few notes in one view.
2. If nothing recent: `/ctxforge` — guided workflow to build fresh context.

## What NOT to do

- **Don't guess file paths.** Before claiming a file is or isn't in the bundle, call `ctxforge_list_items`.
- **Don't paraphrase tool output.** If a tool returns "added 12 items, 4,200 tokens", say that — don't collapse it to "added some files".
- **Don't call `ctxforge_clear` to "recover" from a schema error.** If the tool errors with `missing field 'path'`, the on-disk bundle is corrupt and `ctxforge_clear` will fail the same way. Tell the user to rename `.ctxforge/bundle.json` out of the way via Bash.
- **Don't invoke scenario prompts (`ctxforge_bugfix`, etc.) on an empty bundle.** Check `ctxforge_status` first; if empty, direct the user to `/ctxforge` or `/ctxforge:add`.
- **Don't use `ctxforge_copy` or `ctxforge_pipe` tools** — these don't exist in the MCP surface. Use `/ctxforge:copy` (which wraps `export` + clipboard) or `/ctxforge:pipe` (which shells out to the `ctxforge` binary).
