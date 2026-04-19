---
description: Render a ctxforge refactor prompt for the current bundle. Output IS the crafted prompt.
argument-hint: "<refactor goal>"
---

# /ctxforge:refactor

Render a refactor prompt wrapping the current bundle. Output is the finished prompt.

## Steps

1. Parse `{{args}}` as the task description. If empty, ask what to refactor and stop.
2. Call `ctxforge_status`. If empty bundle, tell the user to `/ctxforge` or `/ctxforge:add` first.
3. Invoke MCP prompt `ctxforge_refactor` with `task = <args>`.
4. Emit the rendered prompt verbatim.

## Examples

- `/ctxforge:refactor "split the 500-line persist.rs into smaller modules"` → rendered refactor prompt.
- `/ctxforge:refactor "extract the URL normalisation into its own crate"` → rendered refactor prompt.

## Related

- `/ctxforge:code-review` — for quality feedback before refactoring.
- `/ctxforge:migrate` — for moving to a different framework / version.
