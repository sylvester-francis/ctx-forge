---
description: "Pick up where you left off": show current bundle plus the most recent memory notes.
argument-hint: [--memory-limit N]
disable-model-invocation: true
---

# /ctxforge:resume

Show a combined view of the current bundle and the last few memory notes, so a user returning after a break can re-orient quickly. This mirrors the `ctxforge resume` CLI subcommand.

## Steps

1. Parse `$ARGUMENTS` for `--memory-limit <N>`. Default `N = 5`.
2. Call `ctxforge_status` and capture the result.
3. Call `ctxforge_recall` with `{limit: N}` and capture the result.
4. Print the status line first, then a blank line, then the recall output labelled `Recent notes:`.
5. If the bundle is empty AND there are no notes, say so and suggest `/ctxforge:ctxforge` (the guided workflow).

## Examples

- `/ctxforge:resume` → status + 5 most recent notes.
- `/ctxforge:resume --memory-limit 10` → status + 10 most recent notes.

## Related

- `/ctxforge:ctxforge` — guided bundle-building workflow for a fresh session.
- `/ctxforge:recall` — search notes with tags/filters.
