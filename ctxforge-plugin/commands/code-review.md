---
description: Render a ctxforge code-review prompt for the current bundle. Output IS the crafted prompt.
argument-hint: "<focus area or PR description>"
---

# /ctxforge:code-review

Render a code-review prompt wrapping the current bundle. Output is the finished prompt.

## Steps

1. Parse `{{args}}` as the task description. If empty, ask what to review and stop.
2. Call `ctxforge_status`. If empty bundle, tell the user to `/ctxforge` or `/ctxforge:add` first.
3. Invoke MCP prompt `ctxforge_code_review` with `task = <args>`.
4. Emit the rendered prompt verbatim.

## Examples

- `/ctxforge:code-review "focus on error handling in the new gh:// fetcher"` → rendered review prompt.
- `/ctxforge:code-review "PR #87 — adding FetchConfig::auth_token"` → rendered review prompt.

## Related

- `/ctxforge:bugfix` — for a known bug.
- `/ctxforge:refactor` — for proposing structural changes.
