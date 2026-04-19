---
description: Search memory notes by tag, keyword, or recency.
argument-hint: [--tag <tag>] [--search <text>] [--since <1w|3d|12h|30m>] [--limit N]
---

# /ctxforge:recall

Read memory notes with optional filters.

## Steps

1. Parse `{{args}}`:
   - `--tag <tag>` → `tag` arg.
   - `--search <text>` → `search` arg (rest of line until next `--`).
   - `--since <dur>` → `since` arg (e.g. `1w`, `3d`, `12h`, `30m`).
   - `--limit <N>` → `limit` arg (default 20 — omit if user didn't pass).
2. Call `ctxforge_recall` with the parsed args (omit anything the user didn't pass).
3. Report the tool output verbatim. Don't summarise.

## Examples

- `/ctxforge:recall` → last 20 notes.
- `/ctxforge:recall --tag auth` → all notes tagged `auth`.
- `/ctxforge:recall --search JWT --limit 5` → up to 5 notes mentioning `JWT`.

## Related

- `/ctxforge:note` — write a new note.
- `/ctxforge:resume` — bundle + notes together.
