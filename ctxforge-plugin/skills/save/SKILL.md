---
description: Save the current bundle as a named profile for later reuse.
argument-hint: <profile-name>
disable-model-invocation: true
---

# /ctxforge:save

Snapshot the current bundle under a name. You can later restore it with `/ctxforge:load <name>`.

## Steps

1. Parse `$ARGUMENTS`. Trim whitespace. If empty, ask the user for a profile name and stop — do not invent one.
2. Call `ctxforge_save_bundle` with `{name: "<parsed>"}`.
3. Report the tool response verbatim.

## Examples

- `/ctxforge:save backend` → save under name `backend`.
- `/ctxforge:save pr-123-review` → save with a descriptive name.

## Related

- `/ctxforge:load` — restore a saved profile.
- `/ctxforge:profiles` — list all saved profiles.
