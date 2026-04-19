---
description: Render a ctxforge explain prompt for the current bundle. Output IS the crafted prompt.
argument-hint: "<what you want explained>"
disable-model-invocation: true
---

# /ctxforge:explain

Render an explain prompt wrapping the current bundle. Output is the finished prompt.

## Steps

1. Parse `$ARGUMENTS` as the task description. If empty, ask what to explain and stop.
2. Call `ctxforge_status`. If empty bundle, tell the user to `/ctxforge:ctxforge` or `/ctxforge:add` first.
3. Invoke MCP prompt `ctxforge_explain` with `task = <args>`.
4. Emit the rendered prompt verbatim.

## Examples

- `/ctxforge:explain "how does bundle migration from v1 to v2 work?"` → rendered explain prompt.
- `/ctxforge:explain "walk me through the docs auto-detect pipeline"` → rendered explain prompt.

## Related

- `/ctxforge:bugfix` — for fixing a specific defect.
- `/ctxforge:code-review` — for quality feedback.
