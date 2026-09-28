# Decisions & Notes

This repository keeps its architecture decisions as **Agent Notes**: Chinese-language,
lifecycle-tagged, schema-validated markdown under `.agents/notes/`. It is the real source
of truth for *why* things are built a certain way, and it is the first place to look before
changing an ownership or compatibility boundary.

Start from `AGENTS.md` at the repo root, then load the relevant skill:

| Skill | Load when |
| --- | --- |
| `develop-lithe` | Any implementation, refactor, review, debug, test, build or doc task |
| `agent-notes` | Creating, migrating, updating, archiving or reviewing notes |
| `write-stable-tests` | Creating, modifying or reviewing test code or test infrastructure |
| `release-lithe` | Preparing, validating or publishing a stable release |

`CLAUDE.md` is a five-line pointer to `AGENTS.md`.

## Note taxonomy

Lifecycle is the first path segment, class is the second
(`.agents/notes/manifest.json:5-19`, `.agents/notes/AGENTS.md`).

| Lifecycle | Meaning |
| --- | --- |
| `implemented` | Landed. The default thing to read. |
| `proposed` | Not landed yet |
| `rejected` | Denied. Read before re-proposing something similar |
| `archived` | Superseded; kept for the reasoning |

Classes: `feature`, `bug-fix`, `simplification`, `architecture`, `process`, `testing`.

Filenames are `yyyy-mm-dd-<english-slug>.md`. The body is **Chinese** (`zh-CN`), new notes
open with `## 先说结论`, and each lifecycle has a **fixed required-heading set** enforced by
`scripts/verify-agent-notes.mjs:24-28`:

| Lifecycle | Required headings |
| --- | --- |
| `implemented` | 问题 / 决策 / 考虑过的备选方案 / 后果 / 验证 / 适用范围 |
| `proposed` | … / 提案 / … / 验收标准 / 风险 / 适用范围 |
| `rejected` | … / 否决理由 / … |

`node scripts/verify-agent-notes.mjs` checks lifecycles, classes, headings, status lines and
links. It runs in CI, but only path-filtered to `.agents/notes/**`,
`.agents/skills/agent-notes/**`, four script paths and two workflows.

## The nine architecture notes you should read

`.agents/notes/implemented/architecture/`:

| Note | What it decides |
| --- | --- |
| `2026-09-13-repository-ownership-and-sharing-boundaries.md` | **The master ownership table.** Every path → responsibility, for `lithe-core`, `lithe-git-host`, `lithe-db-mcp` / `-sidecar`, all 10 gpui crates, `shared/`, `infra/`, `third_party/`, plus the fixed crate dependency direction. Read this before any ownership change. |
| `2026-09-13-language-tooling-and-lsp-runtime-ownership.md` | LSP runtime ownership: which side owns the child process, framing, document versions and deadlines. Cited by both `application-boundary.md:220-221` and `rust-core-api.md:1152-1154`. |
| `2026-09-12-git-execution-and-project-console.md` | Git process and console model. Cited by `rust-core-api.md:470`. |
| `2026-09-13-resizable-ui-performance-boundaries.md` | Splitter / drag / panel-size performance rules. `AGENTS.md` and `develop-lithe` both link it from their entry sections. |
| `2026-09-18-java-project-build-and-launch-boundary.md` | Where the Java project build and launch facts live |
| `2026-09-21-java-entrypoints-owned-by-jdt.md` | Why JDT is the only entry-point and test authority (enforced by `verify-java-semantic-ownership.mjs`) |
| `2026-09-21-maven-settings-reach-the-language-server.md` | Why `mavenSettingsPath` is injected into JDT LS |
| `2026-09-22-editor-run-markers-and-test-outcomes.md` | How JDT facts + recorded test outcomes become editor Run markers |
| `2026-09-27-config-document-semantics-and-workspace-config.md` | Document semantics (tolerant parse, unknown-key preservation, version monotonicity) and the `.lithe/` model |

Plus `2026-09-27-gpui-notification-center.md`.

`proposed/architecture/` holds the two live proposals:

- `2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md` — the whole GPUI direction.
  This is the note that explains why the Swift and React frontends were removed.
- `2026-09-26-workspace-configuration-layers.md` — the configuration layering that
  `gpui/crates/shared/src/workspace_config/` implements ahead of full sign-off.

`implemented/process/` has `2026-09-13-agent-notes-board-and-pages-publishing.md` and
`2026-09-22-atomgit-release-sync.md`.

`archived/architecture/` preserves the pre-GPUI world — module runtime boundaries, macOS
service composition, the Windows/Tauri parity plan, the Monaco feasibility probe. Useful
only when tracing why something was abandoned.

The decision board is published at <https://1lck.github.io/Lithe-IDEA/> by
`deploy-agent-notes-board.yml` on push to `preview`; `scripts/build-agent-notes-board.mjs`
renders `assets/agent-notes-board.html`. The generated `index.html` **must not be
committed** (`.agents/notes/README.md:54`).

## The Rust Core comment standard

Enforced mechanically by `scripts/verify-rust-core-comments.sh` and documented in
`develop-lithe/SKILL.md:96-124`. It applies to first-party code under
`rust/lithe-core/`:

- **English comments only** — the script fails on any Han character in a comment.
- Every production module starts with a concise `//!` responsibility or boundary statement.
- `///` for exported APIs, shared request/response types, core domain types and C ABI
  functions. Document ownership and add `# Safety` for unsafe entry points.
- Document enums, structs, variants and fields whenever their names alone do not make
  semantics, allowed values, units, ownership or protocol role clear — **including
  internal types**.
- `//` inside implementations for non-obvious decisions involving compatibility,
  determinism, ordering, security, performance or cross-platform behaviour.
- In tests, comment the scenario, regression risk or boundary being protected.
- Do not narrate statements or comment trivial accessors for coverage.

Run `./scripts/verify-rust-core-comments.sh` before slower Rust Core checks — it does not
need documentation on every internal helper, but you still review changed internal types
yourself, which a static check cannot judge.

## Where the design documents live

`docs/` contains **no prose architecture documentation** — only
`docs/reference/language-providers.schema.json`, `docs/releases/` (32 stable release
notes + `TEMPLATE.md` + `atomgit-sync.md`), `docs/assets/` (screenshots, contact QR codes,
sponsors) and `docs/visual-qa/`. `docs/architecture/` existed and was migrated into
`.agents/notes/`.

The GPUI design material lives under `rust/gpui/`:

| Path | Purpose |
| --- | --- |
| `PLAN.md` | The stage-by-stage port plan. §13 "Collaboration discipline" is the live part; the `shell-probe` binary it references was deleted. |
| `UI-MAP.md`, `UI-MAP-WINDOWS.md` | UI element inventories |
| `README.md` | Crate overview — **stale**: its diagram omits `java` and `notify`, and its build paths predate the workspace merge |
| `research/` | ~17 focused write-ups: `gpui-kit-0.6.6-api.md`, `gpui-kit-overlay-howto.md`, `editor-lsp-completion.md`, `editor-syntax-highlighting.md`, `menu-open-prereqs.md`, `open-project-and-windows.md`, `java-spring-maven-inventory.md`, `legacy-frontend-removal-audit.md`, `icon-asset-inventory.md`, `click-double-trigger-dpi.md`, and more. These are the closest thing to design notes. |
| `docs/ui-mockup-idea.md` + `.html` + mockup PNGs | UI mockups |
| `BLOCKERS.md`, `HANDOFF.md` | Work-in-progress state |
| `themes/` | 7 built-in themes + `themes/legacy-builtin/` |
| `assets/` | Icons, fonts, icon themes, images (see the `#[exclude]` table in [GPUI App Shell](gpui/app.md)) |
| `docs/gpui-kit/` | Vendored gpui-kit 0.6.6 documentation |

`develop-lithe` also warns: **do not write code against APIs that only exist in
`versions/main` docs** — read the 0.6.6 source under `rust/gpui/docs/gpui-kit/` or the
vendored crate instead. A concrete instance: the `versions/main` docs advertise
`gpui_kit::open_window(..)`, which does not exist in 0.6.6 (`gpui/crates/app/Cargo.toml:26-29`).

## Process conventions

- **Branch from the latest `preview` branch** for every feature or bug fix, naming the
  branch after the issue where possible.
- **Use the `gh` CLI** for GitHub operations rather than the computer-use skill. If
  authorization is needed, first check the host environment for an existing `gh` token and
  reuse it.
- **PR descriptions must enumerate every functional point**, including small ones, so a
  reviewer understands the change immediately.
- **Never** commit `target/`, `.artifacts/`, fixture build directories or local IDE
  settings.
- **Terminate every Lithe process** this session starts, and report explicitly if one cannot
  be stopped.

## Sources

- `AGENTS.md`, `CLAUDE.md`
- `.agents/notes/README.md`, `.agents/notes/AGENTS.md`, `.agents/notes/manifest.json`
- `.agents/notes/implemented/`, `proposed/`, `rejected/`, `archived/`
- `.agents/skills/{develop-lithe,agent-notes,write-stable-tests,release-lithe}/SKILL.md`
- `scripts/verify-agent-notes.mjs`, `verify-rust-core-comments.sh`, `build-agent-notes-board.mjs`
- `.github/workflows/verify-agent-notes.yml`, `deploy-agent-notes-board.yml`
- `docs/`
- `rust/gpui/{PLAN.md,README.md,UI-MAP.md,research/,docs/}`
- `third_party/README.md`
