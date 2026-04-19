---
description: Manage the ctxforge content cache — list / clear / verify integrity.
argument-hint: [list [--scheme <url|gh|description>] | clear [--stale|--all] | verify]
disable-model-invocation: true
---

# /ctxforge:cache

Inspect or manage cached content from URL, gh://, and docs-registry sources.

## Steps

1. Parse `$ARGUMENTS`. First token is the action.
2. Dispatch:
   - `list [--scheme <x>]` → call `ctxforge_cache_list` with `{scheme?}`.
   - `clear --stale` → call `ctxforge_cache_clear` with `{stale: true}`.
   - `clear --all` → call `ctxforge_cache_clear` with `{all: true}`.
   - `clear` without either flag → **refuse**. Print: "Pass --stale or --all. Bare `clear` would wipe the cache."
   - `verify` → call `ctxforge_cache_verify`.
3. Report the tool response verbatim.

## Examples

- `/ctxforge:cache list` → all cached entries.
- `/ctxforge:cache list --scheme gh` → only gh:// entries.
- `/ctxforge:cache clear --stale` → remove only expired.
- `/ctxforge:cache clear --all` → wipe everything.
- `/ctxforge:cache verify` → SHA + HMAC integrity walk.

## Related

- `/ctxforge:refresh` — force-refresh live URL sources.
