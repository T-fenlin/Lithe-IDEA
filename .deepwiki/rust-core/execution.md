# Core: Execution & Support Subsystems

The smaller half of Core: run configuration, debugging, AI, GitHub, plugins, community,
diagnostics and the editor line transforms. Each one follows the same shape — *plan in
Core, own I/O in the host*.

## `execution/` — run configurations and project detection

`execution/configuration.rs` holds the schema, layered overrides and deterministic
generation. `execution/types.rs` holds shared classification types. Two modules carry
unusually load-bearing constraints:

**`execution/launch_command.rs`** keeps an assembled launch command inside the OS limit
(Windows: 32 767 UTF-16 units) by rewriting it into a JDK `@argfile`. "Core owns the text,
the host owns the file" (`:10-14`). Companion decision:
`.agents/notes/implemented/bug-fix/2026-09-20-oversized-java-launch-command.md`.

**`execution/detectors/`** — 12 detectors, one shared bounded walk.
`detectors/scan.rs` owns traversal and every detector must use it; a detector that walks
the tree itself is a bug (`detectors/mod.rs:1-12`). Detectors: `npm`, `python`, `go`,
`cargo`, `make`, `gradle`, `maven`, `compose`, `procfile`, `shell`.

| Detector | Classification rule |
| --- | --- |
| `npm` | Uses the package manager **the lockfile names**; inherits the workspace package manager; `bun` scripts keep `bun` without consuming `node` |
| `gradle` | Reads the build scripts but never starts Gradle or evaluates them. Kotlin DSL, subproject tasks from the build root, same-named modules under different parents, ignore builds with no start task, ignore commented plugins |
| `maven` | From the declared reactor and applied plugins only |
| `compose` | Reported as **infrastructure**, not an application service |
| others | Module roots, conventional layouts, framework declarations |

Two invariants are asserted in `src/tests/detectors.rs`: *every* built-in detector must
produce a launch plan (`:289`), and a detector must not claim an id `maven.scan` already
produced nor disturb its configurations (`:398,440`). Repeated names are qualified by
directory (`:1555`).

`RunConfiguration` layering is `project environment → overrides → generate`, with three
mitigations that all have tests (`src/tests/run_configuration.rs`): a stale generator
revision is invalidated (`:74`), project-scoped toolchain paths must stay relative and
inside the project (`:1940`), and entries whose recorded source was deleted are dropped
(`:2544`).

## `debug/` — transport-neutral DAP

- `engine.rs` — "Stateful DAP reducer whose byte transport and process lifecycle remain
  platform owned." Breakpoints, stepping filters, scopes and variables are ordered
  deterministically (`:174,208,245,337`).
- `protocol.rs` — bounded `Content-Length` framing plus JSON helpers, no native transport.
- `types.rs` — stable requests, updates, events, inspection results.
- `java_test.rs` — deterministic Java test launch configuration.
- `breakpoint_relocation.rs` — moves source breakpoints across UTF-16 editor edits.

Fixtures: `shared/fixtures/debug/` (8 files). The round-trip through the JSON boundary is
tested including a base64 `outboundFrames` payload that must decode to a real framed
`initialize` request (`src/tests/protocol.rs:19-61`).

## `ai/` — commit message generation

Two pure modules:

- `configuration.rs` — parses externally supplied configuration text, "without reading
  files or process environment."
- `generation.rs` — validates commit inputs, **budgets the diff**, and translates
  supported provider wire protocols.

The host owns I/O and secrets (`ai/mod.rs:1`). `ai/tests.rs` covers configuration import,
secret boundaries and the commit wire protocols. Contract: `shared/contracts/ai-commit.md`
with its own `AI_COMMIT_*` error-code namespace.

## `github/` — request planning

Keeps REST paths, payloads, response shapes and error translation identical across
products; transport and credentials stay platform-owned (`github/mod.rs:3-5`).
Determinism is explicit: `BTreeMap` query parameters (`:99`, "Deterministically ordered
query parameters"), labels sorted by name and assignees by login (`:193-196`).

Two security rules are tested: remote components that could change a planned path are
rejected (`src/tests/github.rs:26`), and HTTP failures use stable error categories and
**never leak the response body into `details`** (`:213`).

## `plugins/`, `community/`, `diagnostics/`

- `plugins/` — manifest parsing, compatibility check, deterministic catalog merge using
  `BTreeMap`/`BTreeSet`. An incompatible host is rejected deterministically
  (`src/tests/plugins.rs:47`).
- `community/discourse.rs` — API-key authorization shared by every host.
- `diagnostics/` — "Pure deterministic scrubbing + manifest shaping only. No filesystem or
  process access happens here" (`:6-8`). `redaction.rs` applies text rules;
  `manifest.rs` shapes the bundle. This is what lets any host export a support bundle
  without leaking credentials or machine paths.

## `editor/` — line-level transforms

`editor/line_edit.rs` is pure: text + selection → replacement + range + selection, over
**UTF-16 code units**. `editor/tests.rs` pins expectations against IDEA semantics and the
review regressions from PR #642. The module doc notes the migration status: macOS already
routed here, and these fixtures were the reference for the Windows implementation
(`editor/mod.rs:1-7`).

## The 4-state async contract, host-side

Core states the rule once (`application-boundary.md:18`) and implements it in exactly one
place. Every gpui view mirrors it with its own naming:

| Layer | Enum | Site |
| --- | --- | --- |
| Core | `ProjectPreparation { status }` | `lsp/languages/project_preparation.rs:8-43` |
| Git log pane | `LoadState { Loading, Ready, Stale, Failed, NoRepository }` — adds `Stale` = "has data, last refresh failed" | `lithe-gpui/crates/git/src/model.rs:249-261` |
| Git commit inspector | `FilesState { Idle, Loading, Ready, Failed }` | `lithe-gpui/crates/git/src/model.rs:408-417` |
| Explorer | `LoadState { Loading, Ready, Failed(String) }` — no `idle`, because empty root short-circuits to `Ready` | `lithe-gpui/crates/explorer/src/explorer_view.rs:130-141` |
| Source control | `LoadState { Loading, Ready, Failed(String) }` | `lithe-gpui/crates/git/src/changes_view.rs:213-219` |
| Settings run page | `RunPageMode { NoProject, Failed, Loading, Ready }` | `lithe-gpui/crates/settings/src/dialog.rs:561-570` |

## Sources

- `rust/lithe-core/src/execution/` (all files)
- `rust/lithe-core/src/debug/` (all files)
- `rust/lithe-core/src/{ai,github,plugins,community,diagnostics,editor}/` (all files)
- `rust/lithe-core/src/tests/{detectors,run_configuration,github,plugins,ai}.rs`
- `rust/lithe-core/src/lsp/languages/project_preparation.rs`
- `shared/contracts/{application-boundary,ai-commit,github}.md`
- `shared/fixtures/debug/`, `shared/fixtures/ai/`, `shared/fixtures/github/`
- `lithe-gpui/crates/{explorer,git}/src/`, `lithe-gpui/crates/settings/src/dialog.rs`
