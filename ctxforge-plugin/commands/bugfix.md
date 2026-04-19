---
description: Render a ctxforge bugfix prompt for the current bundle. Output IS the crafted prompt.
argument-hint: "<task description>"
---

# /ctxforge:bugfix

Render a bugfix prompt that wraps the current bundle with instructions for investigating and fixing a bug. The output is the finished prompt, ready to feed to any agent (Claude, Cursor, Gemini, etc.).

## Steps

1. Parse `{{args}}` as the task description. Trim whitespace. If empty, ask the user what bug they want fixed and stop.
2. Call `ctxforge_status`. If the bundle has 0 items, tell the user to run `/ctxforge` (guided build) or `/ctxforge:add` first, and stop.
3. Invoke the MCP prompt `ctxforge_bugfix` with the task string as the `task` argument. (In Claude Code this surfaces as `/plugin:ctxforge:ctxforge:ctxforge_bugfix` — a convenience invocation below the slash menu.)
4. Emit the rendered prompt verbatim as the response. Do NOT summarise, do NOT paraphrase, do NOT execute it — the prompt IS the deliverable.

## Examples

- `/ctxforge:bugfix "null pointer in auth middleware when session is nil"` → rendered bugfix prompt.
- `/ctxforge:bugfix "tests pass locally but fail in CI on macOS"` → rendered prompt with bundle.

## Related

- `/ctxforge:explain` — for understanding rather than fixing.
- `/ctxforge:code-review` — for general quality review.
- `/ctxforge:templates apply bugfix "<task>"` — equivalent path via the templates system.
