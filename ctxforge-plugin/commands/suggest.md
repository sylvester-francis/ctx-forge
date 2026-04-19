---
description: Suggest missing / stale docs entries. Missing = import with no DocsSource; stale = DocsSource with no import.
argument-hint: [--all] [--missing-only] [--apply]
---

# /ctxforge:suggest

Scan bundle (or whole project with `--all`) and compare source-file imports against attached DocsSource entries. Reports the mismatch; `--apply` fixes it.

## Steps

1. Parse `{{args}}`:
   - `--all` → scan whole project tree, not just bundle.
   - `--missing-only` → skip the stale check.
   - `--apply` → apply the fixes after suggesting.
2. Call `ctxforge_suggest` with `{scan_all_project?: bool, missing_only?: bool}`.
3. Report the suggestions verbatim.
4. If `--apply` was set AND the suggestions list is non-empty:
   - Call `ctxforge_suggest_apply` with no `names` arg (applies all).
   - Report what was added / removed.

## Examples

- `/ctxforge:suggest` → list missing + stale docs entries.
- `/ctxforge:suggest --missing-only` → only flag missing.
- `/ctxforge:suggest --all` → scan everything, not just bundle.
- `/ctxforge:suggest --apply` → apply all fixes after listing them.

## Related

- `/ctxforge:docs detect` — initial docs attachment.
- `/ctxforge:docs add <name>` — attach one specific dep.
