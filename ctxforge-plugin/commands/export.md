---
description: Export the current bundle to chat or file. Supports markdown / xml / json and optional template wrapping.
argument-hint: [--format md|xml|json] [--template <name> --task "..."] [--no-memory] [--output <path>]
---

# /ctxforge:export

Render the current bundle as prompt text. Default format is markdown; use `--xml` / `--json` or `--format` to override. Optionally wrap the output in a named template via `--template <name> --task "..."`.

## Steps

1. Parse `{{args}}`:
   - `--format md|markdown|xml|json` — sets `format`.
   - `--xml`, `--json` — shortcuts.
   - `--template <name>` — wrap with template.
   - `--task <text>` — substitute `{{task}}` in the template (rest-of-line after `--task`).
   - `--no-memory` — pass `include_memory: false` to the tool.
   - `--output <path>` — write to file instead of chat.
2. Call `ctxforge_export` with the parsed args (omit args the user didn't pass).
3. If `--output` was set, write the returned content to the path via a Bash step (`Write` tool); then print `Wrote <N> bytes to <path>`.
4. If `--output` was not set, print the returned content verbatim as the response.
5. If the tool errors with a schema complaint, tell the user the bundle file is corrupt and suggest `/ctxforge:clear`.

## Examples

- `/ctxforge:export` → print the bundle as markdown.
- `/ctxforge:export --xml` → print as Claude-optimised XML.
- `/ctxforge:export --template bugfix --task "fix null deref in auth"` → wrap in bugfix template.
- `/ctxforge:export --output bundle.md` → write to `bundle.md`.

## Related

- `/ctxforge:copy` — same thing but to system clipboard.
- `/ctxforge:pipe` — pipe to a local agent CLI (`claude`, `agent`, `gemini`).
- `/ctxforge:templates` — list / manage templates.
