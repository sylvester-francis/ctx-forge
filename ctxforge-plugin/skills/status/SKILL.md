---
description: Show the current bundle's token count against the model window.
disable-model-invocation: true
---

# /ctxforge:status

Print a one-line summary of the current bundle: item count, total tokens, and percentage of the model's context window used.

## Steps

1. Call `ctxforge_status` with no arguments.
2. Report the tool output verbatim.
3. If the tool errors with a schema complaint, tell the user the on-disk bundle is corrupt and direct them to `/ctxforge:clear`.

## Examples

- `/ctxforge:status` → `Bundle: 7 items, 14,200 tokens (7.1% of 200,000)`.

## Related

- `/ctxforge:list` — full item listing (not just totals).
- `/ctxforge:export` — render the bundle for copying or piping.
