---
description: Render a ctxforge migration prompt for the current bundle. Output IS the crafted prompt.
argument-hint: "<migration goal>"
disable-model-invocation: true
---

# /ctxforge:migrate

Render a step-by-step migration prompt wrapping the current bundle. Output is the finished prompt.

## Steps

1. Parse `$ARGUMENTS` as the task description. If empty, ask what migration is planned and stop.
2. Call `ctxforge_status`. If empty bundle, tell the user to `/ctxforge:ctxforge` or `/ctxforge:add` first.
3. Invoke MCP prompt `ctxforge_migrate` with `task = <args>`.
4. Emit the rendered prompt verbatim.

## Examples

- `/ctxforge:migrate "from serde_json 1.0 to 2.0"` → rendered migration prompt.
- `/ctxforge:migrate "move the TUI from tui-rs to iocraft v2"` → rendered migration prompt.

## Related

- `/ctxforge:refactor` — for structural changes within the same stack.
- `/ctxforge:explain` — for understanding the current code before migrating.
