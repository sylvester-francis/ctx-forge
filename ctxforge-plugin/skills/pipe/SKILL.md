---
description: Pipe the current bundle to a local agent CLI (claude, agent, gemini, or any binary on PATH).
argument-hint: <target> [--format md|xml|json] [--template <name> --task "..."] [-- extra args]
disable-model-invocation: true
---

# /ctxforge:pipe

Pipe the rendered bundle to another CLI's stdin. Known targets auto-pick the best format: `claude` → XML, `agent` → markdown, `gemini` → markdown. Any other target is treated as a binary on `$PATH` (markdown default).

**Requires the `ctxforge` binary on PATH** (see plugin README for `cargo install ctxforge`).

## Steps

1. Parse `$ARGUMENTS`. First token is `<target>`. Remaining tokens are flags and (after `--`) extra args.
2. Check the binary exists via Bash: `command -v ctxforge`. If missing, tell the user to `cargo install ctxforge` and stop.
3. Invoke via Bash: `ctxforge pipe <target> [flags] -- [extra-args]`. Forward every flag the user passed — `--format`, `--template`, `--task`, `--no-memory`, `--memory-tag`, `--memory-limit`.
4. Stream the target CLI's stdout back into the chat unchanged.
5. If the target CLI exits non-zero, report exit code and its last 200 chars of stderr.

## Examples

- `/ctxforge:pipe claude` → pipe bundle (as XML) into `claude`.
- `/ctxforge:pipe gemini --template explain --task "why is this slow?"` → wrap in template, pipe to gemini.
- `/ctxforge:pipe my-tool -- --flag value` → pipe to `my-tool`, forward `--flag value` to it.

## Related

- `/ctxforge:copy` — clipboard instead of subprocess.
- `/ctxforge:export --output <path>` — write to file, then invoke a CLI yourself.
