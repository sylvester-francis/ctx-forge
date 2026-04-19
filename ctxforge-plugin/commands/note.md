---
description: Write a memory note that persists across sessions.
argument-hint: [--tag <tag>] <note body>
---

# /ctxforge:note

Record a note that the next session can `recall`. Tags help group notes by topic (`auth`, `tls`, `p3`, etc.). Untagged notes go to `decisions.md`.

## Steps

1. Parse `{{args}}`:
   - If args start with `--tag <tag>`, capture the tag and consume the rest as the body.
   - Otherwise the whole string is the body, no tag.
2. If the body is empty, ask the user what to record and stop.
3. Call `ctxforge_note` with `{body, tag?}`.
4. Report the tool response verbatim (it echoes a timestamped confirmation).

## Examples

- `/ctxforge:note --tag auth "JWT uses RS256 not HS256"` → tagged note.
- `/ctxforge:note "decided to ship P3 before P4"` → untagged decision.

## Related

- `/ctxforge:recall` — read back memory notes.
- `/ctxforge:resume` — bundle + recent notes in one view.
