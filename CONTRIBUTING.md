# Contributing to ctxforge

Thanks for your interest in ctxforge. Before you open a pull request,
please read this document carefully — there are two important things
you need to know.

## 1. License: AGPL-3.0-or-later

ctxforge is licensed under the [GNU Affero General Public License
v3.0 or later](LICENSE). This is strong copyleft — any derivative work,
including modifications you distribute or run as a network service, must
also be released under AGPL-3.0-or-later.

If you want to use ctxforge inside a project that cannot comply with
AGPL-3.0, please open an issue to discuss — a separate commercial
license may be available in the future.

## 2. Contributor License Agreement (CLA)

**Every contribution to ctxforge is subject to a Contributor License
Agreement that assigns copyright in your contribution to Sylvester
Francis.** Read the full text in [CLA.md](CLA.md).

In short:

- You retain the right to use code you wrote for your own purposes
  outside of ctxforge.
- Inside ctxforge, Sylvester Francis becomes the sole copyright holder
  of every contribution.
- This allows the project to be maintained, defended, and relicensed in
  the future without needing permission from every past contributor.

This pattern is the same one used by Qt, MongoDB, and Canonical.

### How to accept the CLA

By submitting a pull request or other contribution, you affirm that you
have read and agree to [CLA.md](CLA.md). For external pull requests, a
CLA assistant bot may require a click-through signature before the PR
can be merged.

## 3. Development setup

```bash
git clone https://github.com/sylvester-francis/ctx-forge
cd ctx-forge
cargo build
cargo test
```

Requirements:

- Rust 1.75 or later
- A C toolchain (for vendored libgit2 compilation on first build)

## 4. Before you open a pull request

- [ ] Run `cargo test` — all tests pass
- [ ] Run `cargo clippy --all-targets -- -D warnings` — no lint warnings
- [ ] Run `cargo fmt --check` — formatting is clean
- [ ] Add tests for any new behavior
- [ ] Update CHANGELOG.md under the unreleased section
- [ ] Read and agree to [CLA.md](CLA.md)

## 5. Code style

Follow the existing file structure:

- Each module has one clear responsibility
- Files stay small and focused
- Business logic lives in the core modules (`bundle`, `tokens`, `resolve`,
  `format`, `memory`, `profile`), not in `commands/`
- `commands/` handlers are thin — they call into core modules

## 6. Questions?

Open an issue. Please mention that you have read this CONTRIBUTING.md
and the CLA so the maintainer knows you are aware of the terms.
