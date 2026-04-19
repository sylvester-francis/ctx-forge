---
description: Remove every item from the current ctxforge bundle. Preserves saved profiles.
---

# /ctxforge:clear

Empty the current bundle. Does not touch saved profiles, memory notes, or the cache.

## Steps

1. Call `ctxforge_clear` with no arguments.
2. Report the tool response verbatim.
3. If the tool errors with a schema complaint like `missing field 'path'`, tell the user the on-disk bundle is corrupt and suggest renaming `.ctxforge/bundle.json` out of the way (they can do this themselves via a Bash step — the tool can't recover a malformed file).

## Examples

- `/ctxforge:clear` → empty the bundle.

## Related

- `/ctxforge:save <name>` — save current bundle before clearing.
- `/ctxforge:rm` — remove specific items instead of all.
