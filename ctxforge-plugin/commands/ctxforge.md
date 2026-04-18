Build a context bundle for the current task using ctxforge.

1. Call `ctxforge_status` to check the current bundle state.
2. Call `ctxforge_recall` to retrieve recent memory notes.
3. Ask the user what they're working on (if not already clear from the conversation).
4. Use the ctxforge skill to build an appropriate context bundle:
   - Add source files via `ctxforge_add_files` / `ctxforge_add_function` / `ctxforge_add_type`.
   - Run `ctxforge_docs_detect` so the prompt includes per-dep doc URLs, releases, and open-issues links from the project's manifest.
   - If the task references a specific GitHub issue or PR, attach it via `ctxforge_add_files` with a `gh:///owner/repo/issues/N` URI.
   - Call `ctxforge_suggest` to flag any imported package that isn't in the Project stack; apply fixes with `ctxforge_suggest_apply`.
5. Report the result: items added, tokens used, percentage of budget.
