# Changelog

## 0.1.1 — 2026-04-09

### Licensing
- **Relicensed from MIT to AGPL-3.0-or-later.** The new license closes the SaaS loophole: anyone who distributes a modified version or runs ctxforge as a hosted network service must release their modifications under AGPL-3.0-or-later.
- **Added Contributor License Agreement (`CLA.md`).** All contributions to ctxforge are assigned to Sylvester Francis, the project's copyright holder. Contributors retain the right to use their code elsewhere but cannot claim ownership of it inside ctxforge. Same model as Qt, MongoDB, and Canonical.
- **Added `CONTRIBUTING.md`** explaining the CLA and contribution workflow.
- **Added `NOTICE`** with explicit copyright attribution.

## 0.1.0 — 2026-04-09

Initial release.

### Features
- `ctxforge add` with file globs, line ranges, `--exclude`, and `--diff <branch>`
- `ctxforge rm` by 1-based index or path
- `ctxforge clear`
- `ctxforge status` with per-item token counts and window percentages
- `ctxforge export` to stdout or file (markdown)
- `ctxforge copy` to the system clipboard
- `ctxforge save` / `ctxforge load` / `ctxforge profiles` / `ctxforge profiles rm`
- Token counting: exact `tiktoken` for OpenAI models, `chars/4` estimate for others
- Model registry with context-window sizes
- `.gitignore`-aware file walking

### Not yet
- Interactive TUI (v0.2)
- Cross-session memory (v0.2)
- MCP server (v0.4)
- XML/JSON exports (v0.2)
- Tree-sitter `--fn` / `--type` (v0.3 behind `--features=extract`)
