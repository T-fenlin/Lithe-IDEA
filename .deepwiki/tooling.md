# Tooling & Validation

The repository has a large `scripts/` directory and eight GitHub workflows, but **almost
nothing Rust-related is in CI**. Knowing which checks are real gates and which are
local-only is the single most practically useful thing on this page.

## The local gate

From `README.md:195-207`, which is the authoritative pre-submit list:

```bash
cargo fmt --manifest-path rust/Cargo.toml -p lithe-core -- --check
cargo test --manifest-path rust/Cargo.toml -p lithe-core
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-app
./scripts/verify-rust-core.sh
./scripts/verify-shared-contracts.sh
node scripts/verify-agent-notes.mjs
node scripts/test-classify-ci-changes.mjs
```

`README.md:207` states it plainly: *"The gpui host has no CI lane yet, so those checks are
the gate."*

`.agents/skills/develop-lithe/SKILL.md:218-230` extends this with a change→check table:

| Change | Minimum validation |
| --- | --- |
| Agent Notes / architecture decision | `node scripts/verify-agent-notes.mjs` |
| Test code or test infrastructure | `./.agents/skills/write-stable-tests/scripts/verify-test-stability.sh`, then the affected Rust timing harness |
| Shared contracts or JSON fixtures | `./scripts/verify-shared-contracts.sh` |
| Rust Core, or the Core↔host contract | `./scripts/verify-rust-core.sh` |
| `gpui/crates/*` | `cargo test -p <crate>` for affected crates + `cargo fmt -- --check` |
| CI lane or path classifier | `node scripts/test-classify-ci-changes.mjs` |
| Java semantic ownership | `node scripts/verify-java-semantic-ownership.mjs` |

`gpui/PLAN.md:1948-1961` adds collaboration discipline: run cargo only on the changed
scope, never `-p a -p b`, never workspace-wide; one cargo writer at a time; verify at
interaction level with the screenshot scripts. Note that 125 % DPI means screenshot pixels
= logical × 1.25.

## `verify-rust-core.sh` — the primary Rust gate

`scripts/verify-rust-core.sh:15-30` runs, in order:

1. `node --test scripts/test-rust-core-comments.mjs` (+ JUnit XML to
   `.artifacts/test-stability/`)
2. `verify-rust-core-comments.sh` — every production module starts with `//!`, no Han
   characters in any comment, public `unsafe fn` needs `///` + `# Safety`, then
   `cargo rustdoc -D missing_docs -D rustdoc::broken_intra_doc_links`
3. `verify-rust-core-layout.sh` — only `lib.rs` at `lithe-core/src` root; the 8 package
   facades (`protocol runtime project execution languages git lsp tests`) must exist;
   legacy `crate::…` imports that bypass the package layout are rejected
4. `cargo fmt -p lithe-core -- --check`
5. timed `cargo test -p lithe-git-host` (120 s cap)
6. `cargo test -p lithe-core`
7. `cargo test -p lithe-gpui-shared -p lithe-gpui-app -p lithe-gpui-workbench`

**Step 7 covers only 3 of the 10 gpui crates.** `editor`, `explorer`, `git`, `terminal`,
`java`, `settings` and `notify` are exercised only by an ad hoc
`cargo test -p <crate>`.

## Other validation scripts

| Script | What it enforces |
| --- | --- |
| `verify-rust-core-comments.sh` | The Rust Core comment standard: module docs, English-only comments, exported rustdoc, unsafe safety sections |
| `verify-rust-core-layout.sh` | Package-facade layout for `lithe-core/src` |
| `verify-shared-contracts.sh` | JSON validity for all fixtures/schemas + 7 structural assertion blocks (see [Contracts](contracts.md)) |
| `verify-java-semantic-ownership.mjs` | Fails if any of 6 retired local entry/test scanner identifiers reappear in `lithe-core/src` or `gpui/crates/java/src` — "JDT must remain the only entry/test authority" |
| `verify-agent-notes.mjs` | Note lifecycle, class, required headings, status lines, links |
| `classify-ci-changes.sh` | The CI path classifier (see below) |
| `validate-stable-release-notes.mjs` | 9 required bilingual headings in order, with content |
| `verify-download-cache.mjs` | Cargo/JDTLS/JDK/Bun download caches against the `third_party` manifests |
| `invoke-cargo-with-cache-fallback.ps1` | Windows cargo wrapper: hard-fails if the target dir escapes the repo, then clears and retries **once**, only when `LITHE_CARGO_BUILD_CACHE_RESTORED=true` |
| `test-rust-core-comments.mjs`, `test-classify-ci-changes.mjs`, `test-lithe-issue-claim.mjs`, `test-prepare-lithe-pr-review.mjs`, `test-prepare-lithe-review-stage.mjs`, `test-sync-atomgit-release.mjs`, `test-validate-stable-release-notes.mjs`, `test-verify-download-cache.mjs` | The scripts' own test suites. `test-classify-ci-changes.mjs` builds a temp git repo, copies the classifier in, and makes it classify **its own** repository |

Build / packaging scripts: `build-database-sidecar.sh`, `build-database-mcp.sh`,
`prepare-jdk.{sh,ps1}` (download + SHA-verify + stage the Temurin 21 JDK into
`.artifacts/jdk[-<arch>]`), `create-git-graph-fixture.sh`.

Smoke tests requiring a built sidecar or docker: `database-sidecar-smoke.sh`,
`database-validation-smoke.sh`.

## CI: what actually runs

Eight workflows in `.github/workflows/`:

| Workflow | Trigger | What it does |
| --- | --- | --- |
| `ci-database.yml` | PR, push to `main`, dispatch | 3 jobs: `changes` (classify) → `rust-database-tests` (only if `rust_database == 'true'`) → `gate` (`if: always()`, asserts the classifier succeeded and the test job was success **or** skipped). Test job: `docker compose config --quiet`, `cargo fmt -p lithe-db-sidecar -p lithe-db-mcp --check`, `cargo test` for both. 20 min timeout |
| `verify-agent-notes.yml` | PR touching `.agents/notes/**`, `.agents/skills/agent-notes/**`, 4 script paths, 2 workflows; + dispatch | `node scripts/verify-agent-notes.mjs` on Node 20 |
| `deploy-agent-notes-board.yml` | push to `preview`; + dispatch | Verify notes → `build-agent-notes-board.mjs --bundle` → GitHub Pages |
| `lithe-pr-review.yml` | `issue_comment: created` | `prepare` (trusted tooling from the default branch, authorize via `vars.LITHE_ALLOWED_REVIEWERS`, upload prompt artifact) → `review` (`openai/codex-action@v1`, read-only sandbox, JSON-schema-validated output) → `publish` (upsert a bot comment) |
| `lithe-issue-claim.yml` | `issue_comment: created` on non-PR issues, daily cron, dispatch | `/assign` / `/unassign` via trusted `.github/lithe-issue-claim/logic.mjs` |
| `lithe-issue-priority.yml` | `issues: opened, edited` on non-PR issues | Parses `### Priority` / `### Platform` headings into `platform:*` / `area:*` / priority labels |
| `sync-atomgit-release.yml` | `workflow_call` + dispatch | **Orphaned** — its only callers (`release-macos.yml` / `release-windows.yml`) were deleted with the frontends |
| `update-repo-charts.yml` | daily cron; + dispatch | `gh api` stargazers + merged PRs → `update-repo-charts.py` → force-push SVGs to the orphan `chart-assets` branch |

### Coverage matrix

| Target | CI coverage |
| --- | --- |
| `lithe-db-sidecar`, `lithe-db-mcp` | yes (`ci-database.yml`) |
| Agent Notes | yes, path-filtered |
| `lithe-core` | **none** — the classifier computes `rust_core` but nothing consumes it |
| `lithe-git-host` | **none** |
| `lithe-gpui-shared` / `-app` / `-workbench` | **none** |
| `lithe-gpui-editor`, `-explorer`, `-git`, `-terminal`, `-java`, `-settings`, `-notify` | **none** |
| `verify-shared-contracts.sh`, `verify-java-semantic-ownership.mjs`, `test-classify-ci-changes.mjs`, the test-stability gate | **none** |

**`ci-rust.yml` does not exist**, even though the classifier has a dedicated case for it
(`classify-ci-changes.sh:82-85`) and `test-classify-ci-changes.mjs:131` creates a stub of it
as a test input. The classifier still computes five outputs (`rust_core`, `rust_database`,
`gpui`, `rust_comments`, `metadata`); **only `rust_database` is consumed**
(`ci-database.yml:22,64,69,81`).

`release-lithe/SKILL.md:20` explicitly lists "a real CI lane running
`cargo build --release` and `cargo test`, not just Core" as a **precondition that is not
met**.

## The path classifier

`scripts/classify-ci-changes.sh:45-172` reads
`git diff --name-status --find-renames base head` and emits five
`key=false|true` GITHUB_OUTPUT lines. Notable rules:

- Any rename or copy (`R*` / `C*`) ⇒ `enable_all_validation` (`:57-63`).
- `*.md` / `*.mdx`, `.github/*`, `docs/*`, `.agents/*`, `.idea/*`, `.gitignore`, `license`
  ⇒ no lane (`:69-71, 86-87`).
- Comment-only edits under `rust/lithe-core/src/*.rs` set `rust_comments` **instead of**
  `rust_core` (`:88-96`, detector at `:18-39`).
- `gpui/crates/*/src/*` sets **both** `gpui` and `rust_core` (`:100-109`, with an
  explicit warning about ordering vs. the `rust/*` catch-all).
- `rust/Cargo.toml` / `Cargo.lock` ⇒ all three lanes (`:120-126`).
- `shared/*` ⇒ `rust_core` + `gpui` (`:136-141`).
- Anything unclassified ⇒ all lanes, **fail closed** (`:160-164`).

## Build profiles

`rust/Cargo.toml` is worth reading before tuning anything:

- `resolver = "3"` is **forced** — the gpui crates are edition 2024 while the four core
  crates are 2021, and Cargo forbids resolver 2 on a workspace containing edition 2024
  members. Resolver 3 changes feature-merge behaviour (no longer unifies across
  target/build-dep), so the `[profile.dev.package.*]` overrides below are **required, not
  an optimization** (`:10-13`).
- Release: `strip`, `lto`, `codegen-units = 1`, `opt-level = 3`. The comment at `:25-29`
  explains the change from `"s"`: the size motivation was producing a staticlib for the
  removed Swift/Tauri hosts, and with ~8.5 MB of embedded assets the size win is
  negligible next to the frame-rate cost.
- `num-bigint-dig` is optimized in **dev and test** too, because the Discourse login flow
  generates a 2048-bit key on every authorization (`:36-39`).
- The gpui / ICU / text stack (`gpui-pre`, `gpui-component`, `gpui-kit`, `gpui-kit-assets`,
  `gpui-pre-macros`, `gpui-pre-platform`, `rustybuzz`, `taffy`, `ttf-parser`) is optimized
  in dev builds so `cargo run` is usable. These are per-package, so
  `cargo test -p lithe-core` does not pay for GPUI (`:41-62`).

## Themes and assets tooling

| Script | Purpose |
| --- | --- |
| `gpui/tools/generate-idea-icons.mjs` | Regenerates `shared/src/icons/idea.rs`; `--check` verifies it |
| `gpui/tools/extract-locale.mjs` | Regenerates `shared/locales/*.yml` from the vendored `source/*.ts` |
| `gpui/tools/check-ui-px.mjs` | Pixel-level UI verification |
| `gpui/capture-screenshot.ps1`, `ui-click.ps1` | Interaction-level verification for the GPUI host |

Adding a file to `gpui/themes/` **requires** registering it in
`BUNDLED_THEMES` (`gpui/crates/settings/src/theme.rs:68-70`) or a test fails.

## Cleanup obligation

`AGENTS.md` mandates that any Lithe process started for a build, test, debug or preview
must be closed before handing back, with bounded best-effort cleanup and an explicit report
if a process cannot be stopped. Do not leave a test-built app in the user's app list.

## Sources

- `README.md`, `AGENTS.md`, `CLAUDE.md`
- `rust/Cargo.toml`
- `scripts/verify-rust-core.sh`, `verify-rust-core-comments.sh`, `verify-rust-core-layout.sh`
- `scripts/verify-shared-contracts.sh`, `verify-java-semantic-ownership.mjs`
- `scripts/verify-agent-notes.mjs`, `classify-ci-changes.sh`
- `scripts/test-classify-ci-changes.mjs`
- `scripts/verify-download-cache.mjs`, `invoke-cargo-with-cache-fallback.ps1`
- `.github/workflows/` (all files)
- `gpui/PLAN.md`, `gpui/README.md`
- `.agents/skills/{develop-lithe,write-stable-tests,release-lithe}/SKILL.md`
