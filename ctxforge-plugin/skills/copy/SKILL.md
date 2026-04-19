---
description: Copy the current bundle to the system clipboard (markdown by default). Uses pbcopy / wl-copy / xclip.
argument-hint: [--format md|xml|json] [--template <name> --task "..."]
disable-model-invocation: true
---

# /ctxforge:copy

Render the bundle and place it on the user's system clipboard. Requires one of: `pbcopy` (macOS), `wl-copy` (Wayland), `xclip` (X11) on `$PATH`.

## Steps

1. Parse `$ARGUMENTS` using the same rules as `/ctxforge:export` (no `--output` here).
2. Call `ctxforge_export` with the parsed args.
3. Detect the clipboard binary via Bash:
   - If `pbcopy` exists, use it (macOS).
   - Else if `wl-copy` exists, use it (Wayland).
   - Else if `xclip` exists, use `xclip -selection clipboard`.
   - Else print an error: "No clipboard binary found. Install pbcopy/wl-copy/xclip."
4. Pipe the tool's rendered content into the clipboard binary via Bash.
5. Report: `Copied <N> chars to clipboard.` (character count comes from the content length).

## Examples

- `/ctxforge:copy` → copy bundle as markdown.
- `/ctxforge:copy --xml` → copy as XML (Claude-optimised).
- `/ctxforge:copy --template explain --task "walk me through the OAuth flow"` → wrap in template, then copy.

## Related

- `/ctxforge:export` — export without copying (to chat or file).
- `/ctxforge:pipe` — stream directly to another agent CLI.
