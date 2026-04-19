---
description: Detect / add / list / remove / refresh dep-docs entries in the current bundle.
argument-hint: [detect [--all] [<path>] | add <name> [--ecosystem rust|js|python|go] | list | rm <name> | refresh]
disable-model-invocation: true
---

# /ctxforge:docs

Library documentation gatherer. Scans your project's manifest (Cargo.toml / package.json / pyproject.toml / go.mod) and attaches per-dep doc URLs.

## Steps

1. Parse `$ARGUMENTS`. First token is the action. Default = `detect` if no args.
2. Dispatch:
   - `detect [--all] [<path>]` → call `ctxforge_docs_detect` with `{all?: bool, path?: string}`.
   - `add <name> [--ecosystem <eco>]` → call `ctxforge_docs_add` with `{name, ecosystem?}`.
   - `list` → call `ctxforge_docs_list`.
   - `rm <name>` → call `ctxforge_docs_rm` with `{name}`.
   - `refresh` → call `ctxforge_docs_refresh`.
3. Report the tool response verbatim.

## Examples

- `/ctxforge:docs` → detect deps in the current project.
- `/ctxforge:docs detect --all` → include Library-tier deps too.
- `/ctxforge:docs add tokio --ecosystem rust` → add specific dep.
- `/ctxforge:docs list` → show currently-attached docs items.
- `/ctxforge:docs rm clap` → remove the clap docs entry.
- `/ctxforge:docs refresh` → re-read lock files, update versions.

## Related

- `/ctxforge:suggest` — find import/docs mismatches.
