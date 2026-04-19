---
description: List every item in the current bundle with its index, path, type, and token count.
argument-hint: [--sources] [--scheme <file|range|func|type|url|gh|docs>] [--stale]
disable-model-invocation: true
---

# /ctxforge:list

Enumerate bundle items. Two modes:

- Default: call `ctxforge_list_items` for a flat per-item listing with token counts.
- With `--sources`: call `ctxforge_list_sources` — groups items by scheme and shows freshness for cacheable sources.

## Steps

1. Parse `$ARGUMENTS`:
   - `--sources` → sources mode.
   - `--scheme <x>` → filter to scheme `x` (only valid with `--sources`).
   - `--stale` → only show stale items (only valid with `--sources`).
2. In default mode, call `ctxforge_list_items`.
3. In sources mode, call `ctxforge_list_sources` with the parsed filters.
4. Report the tool output verbatim — don't reformat the table.

## Examples

- `/ctxforge:list` → numbered table of every item.
- `/ctxforge:list --sources` → grouped by scheme with freshness.
- `/ctxforge:list --sources --scheme url --stale` → only stale URL items.

## Related

- `/ctxforge:status` — just the totals.
- `/ctxforge:refresh` — re-fetch stale sources.
