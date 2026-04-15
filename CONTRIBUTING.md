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

Every pull request is automatically checked by the **CLA Assistant bot**
([contributor-assistant/github-action](https://github.com/contributor-assistant/github-action)).
The bot's flow:

1. You open a pull request.
2. The bot posts a comment linking to [CLA.md](CLA.md) and asking you
   to sign.
3. You reply in the PR with this exact comment:
   ```
   I have read the CLA Document and I hereby sign the CLA
   ```
4. The bot records your signature (GitHub username + timestamp) in
   `signatures/version1/cla.json`.
5. Once signed, the CLA status check turns green and your PR becomes
   eligible to merge. Future PRs you open are auto-approved.

Your signature is a **legally affirmative act** — it is the point at
which the assignment of copyright in Section 2 of the CLA takes effect.
Do not post the signature comment unless you have read and understood
the CLA in full.

If you do not wish to sign the CLA, you may still fork the project and
use it under the terms of the AGPL-3.0-or-later, but your changes
cannot be merged back into the upstream repository.

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

## 6. Commit messages & releases

Commit messages should follow [Conventional Commits](https://www.conventionalcommits.org/)
so [release-plz](https://release-plz.ieni.dev/) can auto-bump the version:

- `feat: …` → **minor** version bump (new feature)
- `fix: …` → **patch** version bump
- `chore: …`, `docs: …`, `test: …`, `refactor: …`, `style: …` → no bump (shown in CHANGELOG only)
- Any commit with `!` after the type (e.g. `feat!: …`) or a `BREAKING CHANGE:`
  footer → **major** version bump

Scopes are optional (e.g. `feat(tui): …`). When a PR merges to `main`:

1. The `Release-plz` workflow opens (or updates) a "Release PR" that
   bumps the version in `Cargo.toml` and prepends a new section to
   `CHANGELOG.md` based on the commits since the last release.
2. Review that Release PR and merge it when you're ready to ship.
3. Merging it triggers the `release` job, which runs `cargo publish`
   and cuts a GitHub Release.

Maintainers only: the `CARGO_REGISTRY_TOKEN` secret must be set in
GitHub repo settings (`Settings → Secrets and variables → Actions`)
for the publish step to work.

## 7. Questions?

Open an issue. Please mention that you have read this CONTRIBUTING.md
and the CLA so the maintainer knows you are aware of the terms.
