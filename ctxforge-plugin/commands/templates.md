---
description: List / create / apply / delete ctxforge prompt templates.
argument-hint: [list | apply <name> "<task>" | new <name> [--from <starter>] | rm <name>]
---

# /ctxforge:templates

Manage `.ctxforge/templates/*.md` — author-written prompt wrappers with `{{bundle}}` and `{{task}}` placeholders. Built-in starters: `bugfix`, `code-review`, `explain`, `refactor`, `migrate`.

## Steps

1. Parse `{{args}}`. First token is the action (`list`, `apply`, `new`, `rm`). Default = `list` if no args.
2. Dispatch:
   - `list` → call `ctxforge_list_templates` (shows built-ins + project-local).
   - `apply <name> <task...>` → call `ctxforge_apply_template` with `{template: "<name>", task: "<rest-of-line>"}`.
   - `new <name> [--from <starter>]` → call `ctxforge_templates_new` with `{name, from?}`.
   - `rm <name>` → call `ctxforge_templates_rm` with `{name}`.
3. Report the tool response verbatim.

## Examples

- `/ctxforge:templates` → list all templates.
- `/ctxforge:templates apply bugfix "fix JWT expiry race"` → render the `bugfix` template.
- `/ctxforge:templates new my-template --from explain` → scaffold a copy of `explain`.
- `/ctxforge:templates rm old-template` → delete a project-local template.

## Related

- `/ctxforge:export --template <name> --task "..."` — export with a template wrapper.
- `/ctxforge:bugfix` / `/ctxforge:explain` / etc. — dedicated scenario commands.
