---
name: ctxforge
description: Use when the user invokes /ctxforge or asks to build, manage, or export a context bundle for AI coding. On-demand only.
---

# ctxforge — Context Engineering Skill

You have access to 31 ctxforge MCP tools for assembling token-disciplined prompts with per-dep documentation links, pinned GitHub resources, and gap detection.

## When to Use

- User invokes `/ctxforge`
- User asks to "build context", "assemble a bundle", "add files to context", "attach docs for this dep", "inline this GitHub issue", or similar

Do NOT use proactively — only when the user asks.

## Workflow

### 1. Assess current state

Call `ctxforge_status` to check if a bundle already exists.
Call `ctxforge_recall` to retrieve any relevant prior memory notes.

Report what you find: "You have X items (Y tokens, Z% of context window)" or "Bundle is empty, let's build one."

### 2. Understand the task

Ask the user what they're working on if not clear. This determines which files, functions, library docs, and GitHub resources to include.

### 3. Build the bundle

**Source files** — start narrow:
- `ctxforge_add_files` — files, globs, or line ranges (`path:start-end`)
- `ctxforge_add_function` — a single function by name (tree-sitter)
- `ctxforge_add_type` — a single type/struct/class by name

Prefer a function or type over a whole file when only part of the file is relevant.

**Library docs (Project stack)** — run once per bundle:
- `ctxforge_docs_detect` — scans Cargo.toml / package.json / pyproject.toml / go.mod; attaches canonical doc URLs + GitHub releases + open-issues links per dep (for framework-tier deps by default; set `all: true` to include library tier)
- `ctxforge_docs_add` — manually attach a dep by name (e.g. an indirect dep the user explicitly wants)
- `ctxforge_docs_rm` — remove a stale docs entry by name
- `ctxforge_docs_refresh` — re-read lock files and bump versions on existing docs entries

**Specific GitHub resources** — inline when the user references them:
- `ctxforge_add_files` with a `gh:///owner/repo/issues/N` URI → inlines the issue body, title, state, author
- `gh:///owner/repo/pull/N` → inlines a PR (with merged-state flag)
- `gh:///owner/repo/releases/tag/v1.2.3` → inlines a release's notes
- `gh:///owner/repo/blob/<ref>/<path>` → inlines a file at a ref (branch or SHA)

Pasted `https://github.com/...` URLs are auto-canonicalised to the `gh://` form.

**Arbitrary URLs** — `ctxforge_add_url` for anything else (specs, docs pages, RFC text). Cached with TTL.

### 4. Close the gaps

Run `ctxforge_suggest` once the bundle is roughly assembled. It scans source-file imports (Rust `use`, JS/TS `import`, Python `from`/`import`, Go `import`) against the attached Project stack and reports:

- **Missing** — a file imports a package with no matching docs entry → suggest `docs_add`
- **Stale** — a docs entry exists but no bundle file imports it → suggest `docs_rm`

Apply the recommendations with `ctxforge_suggest_apply` (optionally pass a `names` subset to cherry-pick).

### 5. Monitor token budget

Call `ctxforge_status` after adding items. Watch the budget:

- **Under 50%** — plenty of room, add more if helpful
- **50–75%** — good range for most tasks
- **Over 75%** — consider trimming with `ctxforge_remove` or `ctxforge_clear`
- **Over 90%** — too full; the model needs room for its response

Use `ctxforge_list_items` for per-item token counts to identify what to trim.

### 6. Persist (optional)

- `ctxforge_save_bundle` — snapshot this bundle as a named profile
- `ctxforge_load_bundle` — restore a profile later
- `ctxforge_profiles_rm` — delete a stale profile
- `ctxforge_note` — write a memory note for decisions, gotchas, or cross-session context

### 7. Apply a template (optional)

- `ctxforge_list_templates` — see built-in and project-local templates
- `ctxforge_apply_template` — wrap the bundle with a template + task description
- `ctxforge_templates_new` — scaffold a new project template (optionally from a built-in starter)
- `ctxforge_templates_rm` — delete a project template

Built-in starters: `bugfix`, `code-review`, `explain`, `refactor`, `migrate`.

### 8. Export and report

- `ctxforge_export` — markdown / xml / json of the assembled prompt
- Tell the user: items count, total tokens, % of window, and which strategies were applied (docs detected, gh resources attached, suggestions applied).

## Tool reference (31 tools)

| Tool | Purpose | Mutates |
|------|---------|---------|
| `ctxforge_status` | Token budget check | no |
| `ctxforge_list_items` | List items with token counts | no |
| `ctxforge_list_sources` | Group items by scheme + freshness | no |
| `ctxforge_recall` | Search memory notes | no |
| `ctxforge_note` | Write a memory note | yes (memory) |
| `ctxforge_add_files` | Add files / globs / ranges / URLs / gh:// | yes |
| `ctxforge_add_function` | Add function by name (tree-sitter) | yes |
| `ctxforge_add_type` | Add type by name (tree-sitter) | yes |
| `ctxforge_add_url` | Attach a URL as a cached source | yes |
| `ctxforge_remove` | Remove items by path or index | yes |
| `ctxforge_clear` | Clear the bundle | yes |
| `ctxforge_refresh` | Force-refresh stale URL sources | yes (cache) |
| `ctxforge_export` | Export bundle (markdown / xml / json) | no |
| `ctxforge_save_bundle` | Save bundle as profile | no |
| `ctxforge_load_bundle` | Load a profile | yes |
| `ctxforge_list_profiles` | List profiles | no |
| `ctxforge_profiles_rm` | Delete a profile | yes |
| `ctxforge_list_templates` | List templates | no |
| `ctxforge_apply_template` | Render a template with bundle + task | no |
| `ctxforge_templates_new` | Scaffold a new template | yes (FS) |
| `ctxforge_templates_rm` | Delete a template | yes (FS) |
| `ctxforge_docs_detect` | Scan manifests, attach per-dep doc URLs | yes |
| `ctxforge_docs_add` | Manually attach one dep's docs | yes |
| `ctxforge_docs_rm` | Remove a docs entry by name | yes |
| `ctxforge_docs_list` | List docs entries | no |
| `ctxforge_docs_refresh` | Re-read lock files, bump versions | yes |
| `ctxforge_suggest` | Flag missing / stale docs entries | no |
| `ctxforge_suggest_apply` | Apply suggest results (subset or all) | yes |
| `ctxforge_cache_list` | Inspect cached fetches by scheme | no |
| `ctxforge_cache_clear` | Clear cache (stale-only or all) | yes (cache) |
| `ctxforge_cache_verify` | SHA + HMAC walk | no |

## Best practices

- **Detect deps first, then add source files.** `ctxforge_docs_detect` runs fast and gives the LLM a map of what your project uses. Source files are added in the context of that stack.
- **Attach `gh://` when the task references a known issue or PR.** Pinning the exact issue body avoids the LLM hallucinating the symptoms.
- **Run `ctxforge_suggest` before export.** It's cheap and catches common gaps ("you added `sqlx` but there's no sqlx docs entry").
- **Prefer functions over files.** If only one function matters, use `ctxforge_add_function` — keeps the token budget lean.
- **Watch the gauge.** The budget is your constraint. Respect it.
- **Use profiles.** Save useful bundles for repeated workflows (`frontend`, `api-layer`, `perf-tuning`).
- **Write memory notes.** Record architectural decisions and gotchas for future sessions.
- **Don't duplicate retrieval.** ctxforge is deterministic — it never calls an LLM. The LLM does retrieval/reasoning on the output. Don't try to make it "smart."
