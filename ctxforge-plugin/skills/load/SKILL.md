---
description: Replace the current bundle with a saved profile.
argument-hint: <profile-name>
disable-model-invocation: true
---

# /ctxforge:load

Replace the current bundle with the contents of a saved profile. The current bundle is discarded — save it first with `/ctxforge:save` if you want to keep it.

## Steps

1. Parse `$ARGUMENTS`. Trim whitespace. If empty, call `ctxforge_list_profiles` to show options, then ask the user to pick one, and stop.
2. Call `ctxforge_load_bundle` with `{profile: "<parsed>"}`.
3. Report the tool response verbatim.
4. If the tool errors with `profile not found`, list available profiles via `ctxforge_list_profiles` so the user can see what's available.

## Examples

- `/ctxforge:load backend` → load profile named `backend`.

## Related

- `/ctxforge:save` — save the current bundle as a profile.
- `/ctxforge:profiles` — list or remove profiles.
