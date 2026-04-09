# Maintainer runbook

Operational notes for **Sylvester Francis** (project owner). This
document is not for contributors — it captures one-time setup,
release process, and lingering manual steps the tooling can't
automate for you.

---

## 1. One-time setup

### 1a. CLA Assistant bot — Personal Access Token

The CLA enforcement bot lives at `.github/workflows/cla.yml` and uses
[`contributor-assistant/github-action`](https://github.com/contributor-assistant/github-action).
It cannot write signatures back to the repo using the built-in
`GITHUB_TOKEN` (GitHub security rule), so it needs a Personal Access
Token with `repo` scope stored as a repository secret.

1. Visit https://github.com/settings/tokens and click **Generate new
   token (classic)**.
2. Name: `ctx-forge CLA signatures`. Scope: check only the **`repo`**
   parent box. Expiration: your preference (90 days or no expiration
   for convenience).
3. Generate and copy the token. It is shown only once.
4. Visit https://github.com/sylvester-francis/ctx-forge/settings/secrets/actions
   and click **New repository secret**.
5. Name: `PERSONAL_ACCESS_TOKEN` (exactly, case-sensitive). Value:
   paste the token. Save.
6. The bot activates on the next pull request automatically.

**To test it:** open a trivial PR from a second GitHub account (or ask
a friend to). The bot should comment asking for CLA acceptance. Post
the exact phrase `I have read the CLA Document and I hereby sign the CLA`
as a PR comment. The bot writes to `signatures/version1/cla.json`.

### 1b. Crates.io account + API token

To publish to crates.io:

1. Create an account at https://crates.io (sign in with GitHub).
2. Visit https://crates.io/me and create a new API token. Scope: at
   minimum `publish-new` and `publish-update`. Name: `ctxforge-publish`.
3. Copy the token.
4. On your local machine:
   ```bash
   cargo login <token>
   ```
5. The token is stored in `~/.cargo/credentials.toml` — never commit
   this file.

You only need to log in once per machine.

### 1c. GitHub repo settings (recommended)

- **Branch protection on `main`** — Settings → Branches → Add rule:
  - Require status checks before merging: CI, CLA
  - Require pull request reviews (even from yourself)
  - Restrict who can push to matching branches
- **Issues and Discussions** — enable both for community engagement
- **Sponsor button** — optional, Settings → General → Features

---

## 2. Release process

### Cutting a new release (vX.Y.Z)

1. **Update the version** in `Cargo.toml`:
   ```toml
   version = "0.X.Y"
   ```
2. **Update `CHANGELOG.md`** with the new entry at the top. Include
   breaking changes, new features, bug fixes.
3. **Verify locally:**
   ```bash
   cargo test
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   cargo build --release
   ```
4. **Commit:**
   ```bash
   git add -A
   git commit -m "release: vX.Y.Z"
   ```
5. **Tag and push:**
   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```
6. **Dry-run the publish** to catch issues before committing the
   upload to crates.io:
   ```bash
   cargo publish --dry-run
   ```
   Expected: `Finished` with no errors. Watch for warnings about
   package size, missing metadata, or included files.
7. **Publish:**
   ```bash
   cargo publish
   ```
8. **Create a GitHub release** (optional but encouraged):
   - Go to https://github.com/sylvester-francis/ctx-forge/releases/new
   - Choose tag `vX.Y.Z`
   - Title: `ctxforge vX.Y.Z`
   - Body: copy the relevant CHANGELOG entry
   - Publish

### Troubleshooting publish

- **`error: crate <name> already exists`** — the name is taken. For
  the initial publish, this should not happen because `ctxforge` was
  verified available during v0.1 prep.
- **`error: failed to get a token`** — run `cargo login <token>`.
- **`error: working directory has uncommitted changes`** — commit or
  stash first, or use `--allow-dirty` (not recommended).
- **`error: invalid license expression`** — verify `license =
  "AGPL-3.0-or-later"` in `Cargo.toml`. The SPDX identifier must
  match a value from https://spdx.org/licenses/.
- **Published the wrong thing?** — crates.io does NOT allow
  deletion, only `cargo yank <version>`. Yanked versions are hidden
  from new installs but remain available for existing consumers. A
  fresh version is required to "fix" a bad publish.

---

## 3. Answering common contributor questions

**"Why do I have to assign my copyright? That seems aggressive."**
> This is the same model used by Qt, MongoDB, and Canonical. It lets
> us maintain and defend the codebase without tracking down every
> past contributor. You retain rights to use your code outside
> ctxforge; inside ctxforge, the project needs a single owner. If
> you're uncomfortable, fork under AGPL and run your own version.

**"Can I contribute without signing the CLA?"**
> No. The CLA is a hard requirement. Without a consolidated copyright
> holder, the project cannot be maintained or relicensed long-term.

**"I'm contributing on behalf of my employer. What do I do?"**
> Get written permission from your employer. CLA.md Section 5.3
> covers this case. Your employer may also need to execute a
> Corporate CLA — in which case, open an issue so we can handle it
> separately.

**"Can you relicense to MIT?"**
> No, and that's deliberate. AGPL closes the SaaS loophole and
> protects the community. If you need a non-copyleft license for a
> specific commercial use case, open an issue to discuss a
> separately-negotiated commercial license.

---

## 4. Jurisdiction and legal caveats

- The CLA is modeled on US copyright practice and general
  international law. It has not been reviewed by a lawyer specific to
  any jurisdiction. For the vast majority of OSS contribution flows,
  this is sufficient.
- **EU contributors** — some EU jurisdictions (Germany especially)
  recognize inalienable "moral rights" that cannot be fully assigned.
  Section 2 of the CLA includes "to the fullest extent permitted by
  applicable law" language to handle this, but if a significant
  contributor is from such a jurisdiction, consult a lawyer.
- **Contributors outside these zones** — the assignment is
  generally enforceable as-is.
- **Pre-CLA contributions** — v0.1.0 was written entirely by Sylvester
  Francis (with AI assistance). No third-party copyright claims
  exist. The CLA applies to all contributions from v0.1.1 forward.

If you ever face a legal dispute, the existence of signed CLAs in
`signatures/version1/cla.json` is your audit trail. Each entry
contains the contributor's GitHub username and a timestamp for the
signing comment.

---

## 5. Roadmap reminder

Future development follows the plans in `docs/superpowers/plans/`:

- **Plan 2** — Memory + continuity (`ctxforge note`, `recall`, `resume`)
- **Plan 3** — XML + JSON exports
- **Plan 4** — `ctxforge pipe <agent>`
- **Plan 5** — TUI hero feature (ratatui composer)
- **Plan 6** — MCP server (`ctxforge mcp`)
- **Plan 7** — Tree-sitter extraction behind `--features=extract`
- **Plan 8** — Launch assets (GIFs, Medium article, reels)

The v1.0 launch target is **after Plan 5 + Plan 6** ship — at that
point ctxforge has its viral hook (TUI) and its second-wave story
(persistent memory via MCP). Before then, v0.x releases are
foundation work.

---

## 6. Open items and TODOs

These are things the automated tooling could not fix for you:

- [ ] Set `PERSONAL_ACCESS_TOKEN` repo secret (Section 1a above)
- [ ] Run `cargo login <token>` on your primary dev machine (1b)
- [ ] Enable branch protection on `main` (1c)
- [ ] Create GitHub Release for v0.1.1 from the tag
- [ ] Verify the CLA bot works by opening a test PR
- [ ] Decide on commercial-license path (if anyone asks)
- [ ] Write Plan 2 (memory + continuity) after v0.1.1 publish lands
