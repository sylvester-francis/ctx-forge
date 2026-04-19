---
description: Add files, globs, line ranges, functions, types, URLs, or gh:// resources to the current ctxforge bundle.
argument-hint: <paths|globs|url|gh://...> [--fn name file] [--type name file] [--diff branch]
---

# /ctxforge:add

Add sources to the current ctxforge bundle. Dispatches to the right MCP tool based on argument shape.

## Steps

1. Parse `{{args}}`. Classify each token:
   - Starts with `http://` or `https://` → URL source (pass to `ctxforge_add_files`; the tool auto-normalises `https://github.com/...` to `gh://`).
   - Starts with `gh://` or `url://` or `file://` or `range://` → URI source (pass to `ctxforge_add_files`).
   - Matches `<path>:<start>-<end>` → line range (pass to `ctxforge_add_files`).
   - Flag `--fn <name> <file>` → call `ctxforge_add_function` with `{name, file}`.
   - Flag `--type <name> <file>` → call `ctxforge_add_type` with `{name, file}`.
   - Flag `--diff <branch>` → call `ctxforge_add_files` with `{patterns: ["--diff", "<branch>"]}` (the tool understands the `--diff` token).
   - Otherwise → file path or glob (pass to `ctxforge_add_files`).
2. If the user gave no arguments, tell them the expected shapes (see Examples) and stop — do NOT call any tool.
3. After each tool call, report the tool's response verbatim — don't paraphrase. Include the added path, item index, and token count that the tool returned.
4. If the tool returns an error mentioning `missing field 'path'` or similar schema errors, tell the user the bundle file is corrupt and suggest `/ctxforge:clear`.

## Examples

- `/ctxforge:add src/auth/*.rs` → add all Rust files in `src/auth/`.
- `/ctxforge:add src/main.rs:10-40` → add lines 10–40 of `src/main.rs`.
- `/ctxforge:add --fn handleLogin src/auth/session.rs` → add just the `handleLogin` function.
- `/ctxforge:add --type Config src/config.rs` → add just the `Config` type.
- `/ctxforge:add https://github.com/owner/repo/issues/42` → attach GitHub issue #42.
- `/ctxforge:add gh:///owner/repo/pulls/123` → attach PR #123 via gh:// scheme.
- `/ctxforge:add --diff main` → add files changed vs. `main`.

## Related

- `/ctxforge:rm` — remove items from the bundle.
- `/ctxforge:status` — check how much budget is used after adding.
- `/ctxforge:suggest` — find missing / stale docs entries.
