---
name: develop-lithe
description: Apply Lithe repository architecture, Core contracts, coding rules, hardcoding restrictions, and validation workflow. Use for every implementation, refactor, review, debugging, test, build, or documentation task in the Lithe repository.
---

# Develop Lithe

Follow these instructions for all work in this repository. Prefer the existing
architecture, nearby code, and executable verification scripts over generic
framework conventions.

Architecture decisions and trade-offs live in Chinese Agent Notes under
`.agents/notes/`. For creating, migrating, updating, archiving, or reviewing
those notes, load `.agents/skills/agent-notes/SKILL.md`.

## Read the relevant source of truth

- Read the implementation and tests around a change before editing it.
- For ownership or dependency changes, read the relevant `implemented` Agent
  Note under `.agents/notes/implemented/architecture/`.
- For behavior that crosses the Rust Core boundary, read
  `shared/contracts/application-boundary.md`, `shared/contracts/rust-core-api.md`,
  and the related fixtures.
- For resizable panels, splitters, continuous dragging, or other high-frequency
  UI interaction, also read
  `.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`.
- Do not introduce a new architectural direction as part of an unrelated task.

## Reuse mature developer tooling before building replacements

Treat reimplementing established IDE infrastructure as an exceptional architectural
decision, not a normal feature-development shortcut. Before adding or expanding
language intelligence, project import, build modeling, dependency resolution,
compilation, formatting, refactoring, test discovery, run/debug planning, or
LSP/DAP behavior:

- Inventory the capabilities already available in Lithe's bundled upstream tools,
  their current versions, official extension points, and compatible mature
  open-source alternatives. Check whether upgrading or enabling an existing
  capability closes the gap before writing a parallel implementation.
- Reuse the largest coherent upstream subsystem whose license, distribution model,
  resource cost, and platform support satisfy the product requirement. Do not reuse
  a few commands while independently recreating the subsystem's project state,
  dependency graph, lifecycle, or semantic model.
- Keep upstream-owned facts in their owning engine. For example, Java symbols,
  source roots, module ownership, classpaths, compilation state, and debug targets
  should come from the selected Java/project backend when it exposes them. Core may
  validate and normalize those results, but must not become a second source of
  truth merely to make the behavior cross-platform.
- Keep Lithe-owned value focused on product orchestration: bounded lifecycle,
  cancellation, stale-result protection, resource budgets, stable cross-platform
  contracts, presentation, and end-to-end workflows.
- A custom implementation is acceptable only when the upstream capability is
  absent, cannot meet a demonstrated requirement, cannot legally be distributed,
  or violates a measured product constraint. Record the evidence, rejected reuse
  options, ownership boundary, and removal or migration path in the relevant Agent
  Note when the decision creates or expands a long-lived subsystem.
- Never reverse-engineer, copy, bundle, or design around proprietary tooling beyond
  its license. Public behavior and open standards may inform an independent
  implementation only when the applicable terms permit it.

Partial reuse must preserve the upstream subsystem's correctness boundary. Define
and test the complete user-visible sequence, such as save -> synchronize project
state -> build -> resolve runtime paths -> launch, instead of testing only that
individual upstream commands were called.

## Respect repository ownership

Lithe is a pure Rust repository. The Swift macOS product, the React/Tauri Windows
product, and the shared Monaco editor package were removed; `rust/gpui/` is the only
host.

| Path | Responsibility |
| --- | --- |
| `rust/lithe-core/` | Deterministic commands, models, validation, and the JSON command envelope |
| `rust/gpui/crates/app/` | App Shell: composition root, window sizing, startup order, settings load and theme application |
| `rust/gpui/crates/workbench/` | Workbench shell: title bar, project tabs, activity bar, status bar, central column |
| `rust/gpui/crates/editor/` | Editor presentation, buffer, navigation, diagnostics, completion |
| `rust/gpui/crates/explorer/` | Project tree |
| `rust/gpui/crates/git/` | Git feature state and views |
| `rust/gpui/crates/terminal/` | Terminal session, ANSI parsing, rendering |
| `rust/gpui/crates/java/` | JDTLS session, workspace fingerprint, Maven context |
| `rust/gpui/crates/settings/` | Settings model, persistence, themes, and the settings dialog |
| `rust/gpui/crates/notify/` | Notification center store and model |
| `rust/gpui/crates/shared/` | Cross-crate primitives: icons, i18n, Core client, workspace config |
| `shared/` | Contracts and fixtures, not compiled implementation |
| `infra/` | Repository-level development and validation infrastructure |
| `third_party/` | Upstream code; leave unchanged unless the task explicitly targets it |

## Preserve application boundaries

- Features receive a dedicated feature model. They must not call `lithe-core`
  directly, construct platform adapters, or reach into another feature's state.
- Feature models own UI state transitions and coordinate user actions.
- Workflows are orchestrated through ports. They must not directly create
  processes, PTYs, watchers, or persistence stores.
- `rust/lithe-core/` must stay free of GPUI and of any concrete UI or platform
  implementation. It is a deterministic command surface, not a UI library.
- `rust/gpui/crates/app` is the composition root. Platform capabilities belong in the
  crate that owns the feature, reached through a port.
- Deterministic behavior shared by features belongs in `rust/lithe-core/`.
  Filesystem, process, terminal, runtime, security, persistence, and UI behavior
  belongs in the owning feature crate.
- Cross-platform use alone does not justify duplicating semantics already owned by
  a mature upstream engine. Put only Lithe's stable normalization and orchestration
  contract in Core; keep language, build, project-model, and debugger facts in the
  selected provider.
- `rust/gpui/crates/*` must not import each other's private modules. The dependency
  direction is `app -> workbench -> {editor, explorer, git, terminal} -> shared`
  and `app -> settings -> shared`.

## Keep shared contracts deterministic

- Use UTF-8 JSON for process and language boundaries.
- Use workspace-relative paths with `/` separators as identifiers. Absolute
  paths are allowed only in platform-owned diagnostics.
- Use one-based line numbers and `null` for missing locations.
- Keep lists and serialized results deterministically ordered.
- Represent asynchronous operations with explicit `idle`, `loading`, `ready`,
  and `failed` outcomes where the application contract requires them.
- Return stable error codes and user-facing messages. Put platform-specific
  details in the contract's `details` field.
- Preserve `operationID`, cancellation, timeout, and stale-result semantics for
  process-backed features.
- Add or update a shared fixture before a second platform relies on new shared
  behavior.
- Treat command names, JSON fields, error codes, and the C ABI as compatibility
  surfaces. Update contract documentation and every consumer when they change.

## Follow the codebase's language conventions

Apply the style used by surrounding files. Prefer descriptive names, focused
types and functions, explicit ownership, and straightforward control flow.
Avoid unrelated cleanup, speculative abstractions, and new dependencies that
the existing stack can reasonably avoid.

### Rust

- Run `cargo fmt` and follow existing crate and module conventions.
- Keep shared results deterministic and preserve the JSON envelope.
- Return structured failures across the boundary; do not expose unstable Rust
  implementation details as contract error codes.
- Add tests in the owning crate for changes to commands, parsing, validation,
  ordering, cancellation, or serialization.

### GPUI host (`rust/gpui/`)

- The crate dependency direction is fixed: `app -> workbench -> {editor, explorer,
  git, terminal} -> shared`, and `app -> settings -> shared`. A feature must not
  depend on `app` or on a sibling feature it does not need.
- GPUI Kit is pinned to the published `0.6.x` line. Do not write code against
  APIs that only exist in `versions/main` docs; read the 0.6.6 source under
  `rust/gpui/docs/gpui-kit/` or the vendored crate source instead.
- The UI thread does not do blocking work. Long operations run on a background
  task and return through the async boundary; see
  `rust/gpui/crates/shared/src/core_client.rs` for how the Core client marshals
  between the two.
- Assets are served through `LitheAssets` (`rust/gpui/crates/app/src/assets.rs`).
  Directories that are on disk but not wired up are excluded there on purpose;
  removing an `#[exclude]` without wiring the asset only grows the binary.

#### Rust Core comments

Apply the following comment standard to first-party code under
`rust/lithe-core/`. It does not require comment coverage in the database helpers,
Windows/Tauri Rust crates, generated code, or third-party sources.

- Write comments in English and keep them accurate when behavior changes.
- Start each production module with a concise `//!` description of its
  responsibility or architectural boundary.
- Use `///` for exported APIs, shared request and response types, core domain
  types, and C ABI functions. Document ownership and add `# Safety` for unsafe
  entry points; describe errors only when the failure contract is not obvious.
- Document enums, structs, variants, and fields whenever their names alone do
  not make their semantics, allowed values, units, ownership, or protocol role
  immediately clear. This requirement applies to internal types as well as
  exported contracts.
- Use `//` inside implementations to explain non-obvious decisions and
  constraints involving compatibility, determinism, ordering, security,
  performance, or cross-platform behavior.
- In tests, comment the scenario, regression risk, or boundary being protected
  when the test name and assertions do not make that intent clear.
- Do not narrate statements, restate descriptive names, or add comments to
  trivial accessors and straightforward control flow solely for coverage.
- Run `./scripts/verify-rust-core-comments.sh` before slower Rust Core checks.
  It enforces module documentation, exported Rustdoc, English comments, and
  unsafe API safety sections without requiring documentation on every internal
  helper. Still review changed internal types and implementations for the
  semantic cases above, which a static check cannot judge reliably.

## Avoid hardcoded environment details

- Never commit credentials, tokens, private endpoints, signing material, or
  personal data.
- Do not embed developer-machine paths, workspace roots, home directories,
  temporary directories, or tool installation paths in application logic.
- Resolve executables, storage locations, and platform defaults through the
  appropriate adapter or configuration mechanism.
- Keep stable product constants named and centralized. Do not duplicate magic
  strings or numbers across platforms.
- Test fixtures may use clearly fake values, but must not contain real secrets
  or machine-specific paths.

## Handle failures explicitly

- Do not silently discard errors. Return, translate, or log them at the layer
  that has enough context to act on them.
- Preserve stable contract error categories when crossing the Rust Core
  boundary or a process boundary.
- User-facing failures should be actionable without exposing credentials,
  environment contents, or unnecessary internal details.
- Comments should explain non-obvious constraints or decisions, not narrate the
  code.

## Run validation that matches the change

Run the smallest relevant checks while iterating, then the broader affected set
before handoff.

| Change | Minimum relevant validation |
| --- | --- |
| Agent Notes or architecture decision migration | `node scripts/verify-agent-notes.mjs` |
| Test code or test infrastructure | `./.agents/skills/write-stable-tests/scripts/verify-test-stability.sh`, then the affected Rust timing harness from `write-stable-tests` |
| Shared contracts or JSON fixtures | `./scripts/verify-shared-contracts.sh` |
| Rust Core, or the Core-to-host contract | `./scripts/verify-rust-core.sh` |
| `rust/gpui/crates/*` | `cargo test --manifest-path rust/Cargo.toml -p <crate>` for the affected crates, plus `cargo fmt --manifest-path rust/Cargo.toml -- --check` |
| CI lane or path classifier | `node scripts/test-classify-ci-changes.mjs` |
| Java semantic ownership | `node scripts/verify-java-semantic-ownership.mjs` |

Also run tests for directly affected crates. If the current machine cannot run
a check, state that clearly; do not claim an unexecuted check passed.

## Keep changes reviewable

- Preserve existing uncommitted work and avoid modifying unrelated files.
- Do not commit generated output such as `target/`, `.artifacts/`, fixture build
  directories, or local IDE settings.
- Do not perform broad formatting or dependency updates as part of a focused
  fix.
- Do not use destructive Git commands, create commits, push branches, or change
  release metadata unless the task explicitly requests it.
- Update the owning Agent Note when behavior, ownership, or a compatibility
  surface changes. Do not rewrite notes for an implementation-only refactor
  that leaves the documented decision intact.

## Complete the work honestly

Before reporting completion, confirm that the change is in the owning layer,
relevant tests or verification scripts were run, shared consumers were checked,
and no machine-specific hardcoding was introduced. Report what changed, what
was verified, and any remaining platform or test limitations.
