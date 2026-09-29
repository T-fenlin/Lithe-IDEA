# Core: Language Tooling

Language features in Core split by **who owns the truth**:

```
lsp/lightweight/   in-process, no server      -> always available
lsp/interface/     generic LSP client+engine  -> transport-neutral, scripted-server testable
lsp/languages/     per-provider policy         -> JDT LS specifics, pure adapters
languages/         source inspection          -> Java / Spring / MyBatis, no LSP at all
project/           workspace facts             -> tree, search index, history, Maven, Markdown
```

The facade `lsp/mod.rs` exists purely to keep the Core command API stable while the
internals are reorganised by responsibility.

## `lsp/interface/` — the process boundary

Deliberately split so platform spawn is *substitutable* and lifecycle rules are testable
against a scripted server:

| File | Role |
| --- | --- |
| `client.rs` | Pure client-state transitions and JSON-RPC message construction |
| `engine.rs` | Stateful sessions coordinating client state + child processes |
| `process.rs` | Traits only — "The language-server process boundary" |
| `transport.rs` | Bounded encoding and incremental parsing of LSP frames |
| `types.rs` | Serializable client state and wire models |
| `scripted.rs` | A scripted language server for engine tests: delivers bytes/exits at chosen moments so timing rules are asserted directly |
| `engine_real_jdt_tests.rs` | Opt-in end-to-end against a real JDT LS, gated on `LITHE_JDTLS_SMOKE_*` |

`scripted.rs` is the key idea: every claim about bounded waiting, request deadlines and
graceful/forced termination is asserted against a server whose behaviour the test chooses,
not against a real JVM.

## `lsp/languages/` — JDT LS policy

Pure adapters that keep JDT-specific semantics out of the generic client. `catalog.rs`
also carries a 7-point "adding a new language server" checklist and loads
built-in + workspace provider catalogues.

Ownership boundaries worth memorising:

- `java_entrypoints.rs` — "never reads Java source to add, drop, or second-guess an entry."
  It only normalizes JDT's `resolveMainClass`.
- `java_tests.rs` — normalizes `vscode.java.test.findTestTypesAndMethods`; "no annotation
  or naming rules."
- `jdt_build.rs` — coordinates `vscode.java.buildWorkspace` as a deterministic state
  machine, one build per session.
- `jdt.rs` — "The adapter is deliberately pure… The process engine remains responsible for
  creating directories and performing all I/O." It also owns `normalized_workspace_identity`
  (lowercased drive letter, `//` UNC recognized, case-folded) and the SHA-256
  `lsp.jdtWorkspaceKey` over it, with the optional structural fingerprint mixed in as a
  NUL-delimited suffix (`jdt.rs:988-1005`).
- `jdt_progress.rs` — throttled workspace-import progress; fixed 2 000 ms interval, 5 %
  step, `BTreeMap`/`BTreeSet` for determinism.

`project_preparation.rs` is the one place the four-state async model is implemented in
Core:

```rust
struct ProjectPreparation { phase: &'static str, status: &'static str, blocks_run: bool }
// status ∈ {idle, loading, ready, failed}
```

Mapping at `:27-43`: `Created|ProcessStarting → ("starting","loading")`,
`Initializing → ("importing","loading")`, `Failed → ("starting","failed")`,
`Stopping|Stopped → ("stopped","idle")`, `Ready` + profiles running or configuring →
`("configuring","loading")`, `Ready` + building → `("building","loading")`,
`Ready` + `Failed|TimedOut|PartiallySucceeded|Cancelled` → `("configuring","failed")`,
otherwise `("ready","ready")`. Invariants pinned by
`shared/fixtures/lsp/project-preparation-v1.json`: *"Ready means project preparation, not
compilation, completed"*, failures and stops never report ready, and `ServiceReady` does
not bypass project configuration.

## `lsp/lightweight/` — the always-available path

This is what makes Lithe useful before JDT LS finishes indexing (or if it never starts).

- `edits.rs` — UTF-16-aware text edit validation and application.
- `snippets.rs` — LSP snippet → insertion-ready plain text. The host deliberately calls
  `lsp.plainSnippet` rather than reimplementing it, so the shown candidate and the
  inserted text cannot diverge (`lithe-gpui/crates/java/src/service.rs:1178-1202`).
- `symbols.rs` — in-process document symbols, references, renames, semantic-token helpers.

Command surface: `lsp.builtinCompletions`, `lsp.builtinNavigation`,
`lsp.builtinSymbols`, `lsp.plainSnippet`, `lsp.semanticTokens`.

## `languages/` — no LSP involved

- `java.rs` — lightweight Java source + Maven-aware run-configuration inspection.
- `java_syntax.rs` — deterministic syntax classification for native renderers.
- `spring.rs` — Spring Boot configuration → profile → bean → endpoint semantic index.
- `mybatis.rs` — mapper-interface ↔ XML statement index, ordered by namespace and id.

## `project/` — workspace facts

| File | Responsibility |
| --- | --- |
| `files.rs` | Traversal, read/write with containment, search, replacement preview. The 14 built-in hidden directories, 2 MiB searchable / 32 MiB openable caps, `max_results` clamp to 10 000, symbol clamp to 50. |
| `search_index.rs` | Trigram + symbol index with a global `INDEX_CACHE` keyed by `{root, hidden dirs, patterns}`. Postings intersected via `binary_search`, then sorted by numeric id so hash iteration order cannot leak. |
| `document_lifecycle.rs` | Pure state machine: `Clean`/`Dirty`/`Saving`/`Conflict` × 9 events → 6 actions. Enforces `saved_revision ≤ save_revision ≤ revision` and rejects blank operation ids. |
| `history.rs` | Versioned local snapshots: 2 MiB, 100 entries per file, 30-day retention, `HISTORY_VERSION = 2`. Newest-first with a total tiebreak (timestamp, id, path). |
| `markdown.rs` | Comrak with raw HTML enabled and `tagfilter = false` so the Ammonia pass can drop dangerous elements *with* their contents — "the security boundary" (`:26-27`). |
| `maven.rs` | Reactor scan, profiles, source roots, launch/dependency plans, six output caps (`500_000` chars, 10 000 nodes, depth 64). |
| `maven_test_reports.rs` | Surefire/Failsafe XML reading, restricted to reports for the requested classes written at/after run start. |

Path safety is worth citing because it is enforced in three places:
`safe_relative_path` for reads (`files.rs:654-671`), `writable_relative_path` for writes
(`:673-713`, which additionally **rejects symlinks** and canonicalizes through the nearest
surviving parent for not-yet-existing files), and `invalid_relative_path`
(`protocol/error.rs:80-87`) which normalizes `\`→`/` *before* rejecting `..`, `:`, NUL and
absolute input — so a Windows-shaped path cannot bypass validation on another host.

## Host-side counterpart

`lithe-gpui/crates/java/` owns everything platform-specific about JDT LS: discovery, JDK
resolution, the workspace-index cache, the synchronous session envelope, and a single
`std::thread` event pump that is the only consumer of `lsp.waitEvents`. See
[Language Service](../lithe-gpui/java.md).

## Sources

- `rust/lithe-core/src/lsp/` (all files)
- `rust/lithe-core/src/languages/` (all files)
- `rust/lithe-core/src/project/` (all files)
- `rust/lithe-core/src/protocol/contracts.rs`
- `rust/lithe-core/src/protocol/error.rs`
- `rust/lithe-core/src/tests/{project,languages,protocol}.rs`
- `shared/fixtures/lsp/`, `shared/fixtures/search/`, `shared/fixtures/documents/`
- `shared/contracts/application-boundary.md`
- `lithe-gpui/crates/java/src/service.rs`
- `.agents/notes/implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md`
- `.agents/notes/implemented/architecture/2026-09-21-java-entrypoints-owned-by-jdt.md`
