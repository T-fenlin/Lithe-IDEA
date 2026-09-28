# Rust Core

`rust/lithe-core/` is the deterministic command surface. One function is the whole
public entry point:

```rust
// rust/lithe-core/src/lib.rs:25-27
pub fn execute_json(request: &str) -> String { runtime::execute_json(request) }
```

~82 k lines across 125 files, 930 public symbols. The gpui host links the crate
directly and calls this synchronously on a background thread.

## Module map

| Module | Responsibility | Visibility |
| --- | --- | --- |
| `protocol/` | Command names, response models, error codes, events, cancellation | glob re-exported |
| `project/` | Workspace traversal, search + index, local history, document lifecycle, Markdown, Maven | crate-private |
| `lsp/` | Generic LSP client/engine, JDT LS policy, in-process language features | crate-private |
| `languages/` | Java / Spring / MyBatis source inspection, independent of LSP transport | crate-private |
| `runtime/` | JSON dispatch and the (retired) C ABI | crate-private |
| `execution/` | Run configuration generation, 12 project detectors, launch-command length handling | crate-private |
| `debug/` | Transport-neutral DAP state machine, Java test launch, breakpoint relocation | crate-private |
| `git/` | ~6 900 lines of Git inspection + mutation, console folding, event stream | crate-private |
| `ai/` | Provider config parsing, commit-message request planning | `pub mod ai` |
| `github/` | REST path/payload/response planning, error translation | crate-private |
| `plugins/` | Manifest parsing, compatibility, catalog merge | `pub mod plugins` |
| `community/` | Discourse API-key authorization | crate-private |
| `diagnostics/` | Bundle redaction + manifest shaping (no I/O) | crate-private |
| `editor/` | Line-level text transforms over UTF-16 code units | crate-private |

See [Core: Git](rust-core/git.md), [Core: Language Tooling](rust-core/language-tooling.md)
and [Core: Execution & Support](rust-core/execution.md) for depth.

## The envelope

Request — `protocol/command.rs:9-27`, all `camelCase`:

| Field | Meaning |
| --- | --- |
| `id` | Correlation id, copied to the response |
| `operationId` | Cooperative cancellation **and stale-result handling**; defaults to `id` |
| `timeoutMilliseconds` | Deadline for operations that support bounded execution |
| `gitExecution` | Host-owned Git preferences, scoped to this request only |
| `command` | Stable name, resolved by `CoreCommand::parse` |
| `payload` | Omitted payloads deserialize as JSON `null` |

Response — `protocol/contracts.rs:10-21`:

```rust
pub struct CoreResponse {
    pub id: Option<String>,
    pub ok: bool,                        // discriminator: data XOR error
    pub data: Option<ResponseData>,      // skipped when None
    pub error: Option<CoreError>,        // skipped when None
}
```

`CoreResponse::failure` always sets `data: None` (`:48-55`) — a failure never carries
partially successful data.

`CoreCommand` (`command.rs:34-305`) has **no `Serialize` derive**. It exists only to hold
the name→domain mapping, and `CoreCommand::parse` (`:310-448`) is the single place that
mapping exists. Its doc is explicit: it resolves a compatibility name *"without accepting
aliases or case variations that could behave differently across hosts"*. Adding a command
means adding a variant **and** a match arm; both are covered by
`command.rs:451-542`.

## Dispatch pipeline

`runtime/dispatcher.rs` is one 2 300-line `match`. The order matters:

1. `:54-63` parse JSON → `invalid_request` with the serde error in `details`.
2. `:66` `operationId` defaults to `id`.
3. `:67-70` install a `cancellation::Scope` for this thread.
4. `:71-85` register Git execution events, early cancellation check, Git execution scope.
5. `:79-85` `CoreCommand::parse`; unknown name → `not_supported` with the name in `details`.
6. `:87-2232` per-arm `from_value::<XRequest>` → domain function → `success`/`failure`.
7. `:2233-2240` **final gate** — a success is downgraded to `cancelled`/`timed_out` if the
   operation was cancelled or timed out after the domain function returned.

Because the closure point is single, no domain can invent a code shape.

## Cancellation, deadlines, staleness

`protocol/cancellation.rs` is a per-thread scope over a global registry:

```rust
State { cancelled: Arc<AtomicBool>, deadline: Option<Instant> }   // :12-15
type Registrations = HashMap<String, Vec<Arc<AtomicBool>>>;      // :19  (a VECTOR, not one token)
thread_local! static CURRENT: RefCell<Option<State>>             // :22-24
```

The vector matters: a nested request reusing the same `operationID` does not unregister a
still-live caller (`:17-18`). `Scope::begin` installs a deadline only when
`milliseconds > 0` (`:53-55`); `Drop` restores the previous state and removes only its own
token via `Arc::ptr_eq` (`:66-81`). `check()` (`:96-116`) returns `cancelled` or `timed_out`
and, on timeout, also sets the cancelled flag so downstream cleanup stops.

Checkpoints are explicit `cancellation::check()?` calls at command boundaries and
workspace traversal points: `project/files.rs:385,400,574,628` and
`project/search_index.rs:367`.

**Stale-result protection has three distinct layers**, and it is worth knowing which one
you are in:

| Layer | Mechanism | Site |
| --- | --- | --- |
| Core, deterministic | `DocumentLifecycleState::Saving` carries `operation_id`; a completion whose owner does not match yields `IgnoreStaleResult` | `project/document_lifecycle.rs:35,128,253-281` |
| Core, Git | Preview → write. A changed local head or destination fails with a fixed message, e.g. `"Git push preview is stale; refresh and try again."` | `git/mod.rs:5459`; rebase `git/rebase_session.rs:753`; rewrite `git/rewrite.rs:608,724` |
| Host | Monotonic `request_serial` / `generation` / `revision` per view; a non-current value is dropped | `git/src/changes_view.rs:328-358`, `editor/src/editor_view.rs:693-735` |

## Events

Two mechanisms. `protocol/event.rs:10-24` defines `CoreEvent` but the live stream is
Git's: `git/execution_events.rs:16` installs an `EventSink`, and every event carries
`operationId` (`:21-25`). Kinds: `requestStarted`, `requestFinished`, `started`,
`output`, `finished` (`:34-65`). Bounds: 16 KiB per line, 512 KiB per diagnostic,
4 096 events (`:12-14`).

`ObserverScope` (`:78-88`) restores the previous sink so a nested request never inherits
the outer observer, and `deliver` (`:105-110`) installs `None` around the callback so a
synchronous `git.authRespond` cannot rewrite the observed operation's identity. That exact
nesting is tested (`git/execution_events.rs:537-599`).

## The C ABI is retired

`include/lithe_core.h` still exports 7 functions, all implemented in
`runtime/ffi.rs`: `lithe_core_version`, `lithe_core_git_askpass`,
`lithe_core_execute_json`, `lithe_core_execute_json_with_events`,
`lithe_core_lsp_provider_catalog_json`, `lithe_core_cancel`, `lithe_core_free_string`.

They have **no callers**. `shared/contracts/rust-core-api.md:3-11` states the C ABI is
retired, that the gpui host links the Rust crate directly, and that the header plus
`ffi.rs` are kept only as valid exports. The crate is still built as
`["rlib", "staticlib", "cdylib"]` (`Cargo.toml:9`). Treat this file as historical.

Consequence: `lithe_core_git_askpass` is the only path to `lithe_git_host::authentication::helper_main`,
and nothing in the gpui host registers `LITHE_GIT_ASKPASS_MODE`, so interactive Git
authentication is not wired today. See [Supporting Crates](supporting-crates.md).

## Tests

Two shapes, with different reach:

| Shape | Can reach | Count / examples |
| --- | --- | --- |
| `rust/lithe-core/tests/*.rs` | Only `lithe_core::execute_json` — a true black-box test | 2 files. `git_push.rs` drives a real `git` binary against temp bare remotes and asserts the reviewed-push contract: `--force-with-lease=refs/heads/main:<oid>` present, bare `--force` absent, refusal after the local head or `branch.*.pushRemote` changes with the exact stable message and no recorded push. `git_watch_context.rs` checks `git.watchContext` returns plain native paths (strips `\\?\`, rewrites `\\?\UNC\` → `\\`). |
| `rust/lithe-core/src/tests/*.rs` | `pub(crate)` internals | 15 modules. `git.rs` is the largest (~4 500 lines, ~55 tests). Module-local `#[cfg(test)]` blocks also exist in `command.rs`, `cancellation.rs`, `dispatcher.rs`, `lsp/`, `git/graph.rs`, `project/document_lifecycle.rs`, `editor/tests.rs`, `ai/tests.rs`, `git/console/tests.rs`. |

Roughly 90 JSON fixtures under `shared/fixtures/` pin the wire contract; only
`lithe-core` reads them (see [Contracts & Fixtures](contracts.md)).

## Sources

- `rust/lithe-core/src/lib.rs`
- `rust/lithe-core/src/protocol/{command,contracts,error,event,cancellation}.rs`
- `rust/lithe-core/src/runtime/{dispatcher,ffi}.rs`
- `rust/lithe-core/include/lithe_core.h`
- `rust/lithe-core/Cargo.toml`
- `rust/lithe-core/tests/{git_push,git_watch_context}.rs`
- `rust/lithe-core/src/tests/`
- `rust/lithe-core/src/project/document_lifecycle.rs`
- `rust/lithe-core/src/git/execution_events.rs`
- `shared/contracts/rust-core-api.md`
