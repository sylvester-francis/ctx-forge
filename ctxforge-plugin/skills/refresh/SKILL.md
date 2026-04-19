---
description: Refresh cached URL / gh:// sources. No args = stale only; --all = everything.
argument-hint: [<uri>] [--all]
disable-model-invocation: true
---

# /ctxforge:refresh

Re-fetch cached sources.

## Steps

1. Parse `$ARGUMENTS`:
   - One URI-looking token (starts with `url://`, `gh://`, `https://`, `http://`) → `uri` arg.
   - `--all` flag → `all: true`.
2. Call `ctxforge_refresh` with the parsed args (omit args the user didn't pass).
3. Report the tool response verbatim.

## Examples

- `/ctxforge:refresh` → refresh all stale sources.
- `/ctxforge:refresh --all` → refresh everything, not just stale.
- `/ctxforge:refresh url://docs.rs/tokio/1.35.0/` → refresh one URI.

## Related

- `/ctxforge:cache list --stale` — see what's stale first.
- `/ctxforge:cache verify` — check integrity.
