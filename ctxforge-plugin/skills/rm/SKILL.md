---
description: Remove items from the current ctxforge bundle by path or 1-based index.
argument-hint: <path-or-index> [<path-or-index> ...]
disable-model-invocation: true
---

# /ctxforge:rm

Remove one or more items from the current bundle.

## Steps

1. Parse `$ARGUMENTS` as a whitespace-separated list. Each token is either a numeric index (1-based) or a file path.
2. If no arguments, tell the user the expected shape and stop.
3. Call `ctxforge_remove` once with `{targets: [<all-tokens>]}`.
4. Report the tool output verbatim — which items were removed and which, if any, were not found.

## Examples

- `/ctxforge:rm 3` → remove item #3.
- `/ctxforge:rm src/auth/session.rs` → remove by path.
- `/ctxforge:rm 1 2 5` → remove multiple by index.

## Related

- `/ctxforge:list` — show current items with their indices.
- `/ctxforge:clear` — remove all items at once.
