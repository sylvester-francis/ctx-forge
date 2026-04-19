---
description: List saved profiles, or remove one by name.
argument-hint: [rm <profile-name>]
disable-model-invocation: true
---

# /ctxforge:profiles

Manage saved profiles.

## Steps

1. Parse `$ARGUMENTS`.
2. If empty: call `ctxforge_list_profiles` and report verbatim.
3. If starts with `rm <name>`: call `ctxforge_profiles_rm` with `{name: "<name>"}` and report.
4. Any other input: print the expected shapes and stop.

## Examples

- `/ctxforge:profiles` → list all profiles.
- `/ctxforge:profiles rm backend` → delete the `backend` profile.

## Related

- `/ctxforge:save` — save a new profile.
- `/ctxforge:load` — restore a profile.
