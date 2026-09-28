# Language Service: Session & Events

The runtime half of `rust/gpui/crates/java/`. Discovery, JDK resolution and the
workspace index cache are on the [parent page](../java.md); this page covers the session
envelope, the event pump, and the semantic request path.

State recap:

```
Session                    session.rs:101-107
  id, events: Arc<SessionEvents>, pump: EventPump

SessionEvents              events.rs:139-161
  pending: Mutex<HashMap<operationId, Sender<Result<Value,String>>>>  # in-flight waiters
  lifecycle: Mutex<Lifecycle> + Condvar                              # queued Vec<Value> + terminal
  diagnostics: Mutex<HashMap<uri, Vec<JavaDiagnostic>>>              # latest snapshot only
```

## `lsp.startServer` payload

`Session::start` (`session.rs:127-214`) builds a fully structured **direct launch** — no
wrapper arguments. The envelope timeout is **30 s** (`:181-185`), covering only "spawn a
process and write one frame", so a UI background task is never hung on the `initialize`
deadline. Every path goes through `jdtls::normalize_path` (`:882-890`), which converts
`\\?\UNC\server\share` → `//server/share`, strips verbatim `\\?\`, and turns `\` into `/`
— mandatory because the JVM rejects verbatim paths.

The pump is started **immediately after `startServer` succeeds** (`:210-211`): the
handshake's `stateChanged` / `log` / `serverInfoChanged` events are already queued, and
starting later would miss the ready signal. A pump-start failure fails the whole start.

## Event pump

```
loop {
  if stopping -> break
  match wait_events(session_id, PUMP_SLICE = 1s) {
    Ok(batch)  -> dispatch each; if terminal_reason().is_some() -> break
    Err(error) -> break   // ANY failure means the session is unusable
  }
}
events.mark_terminal("事件泵已退出（{reason}）")
```

`PUMP_SLICE` is **not** a polling period: the pump blocks on Core's condvar and returns
immediately on activity. It only bounds how often an idle session asks Core and how long
`stop()` takes to join (`events.rs:51-56`).

| Event | Handling |
| --- | --- |
| `requestCompleted` | Remove the waiter by `operationId` and `send` the result; an unknown id increments `unclaimed` and logs `S1_JAVA_PUMP unclaimed` |
| `diagnostics` | Per-`uri` latest snapshot + `S1_JAVA_DIAGNOSTICS uri=… version=… count=… dropped=…` |
| `log` | `S1_JAVA_LOG level=… message=…`, consecutive duplicates suppressed |
| `stateChanged` / `serverInfoChanged` | Pushed to a **queue** + `Condvar::notify_all`; terminal states also `mark_terminal` |
| anything else | Counted, and `S1_JAVA_PUMP ignored type=…` printed **only on first sight** of that type |

**Single-consumer discipline.** `lsp.waitEvents` has drain semantics, so a second consumer
would steal the other's events. `wait_events` is therefore a **free function** taking
`session_id` rather than a `&Session`, so nobody can casually obtain a second read handle
(`session.rs:442-446`).

`Lifecycle::queued` is a **queue, not a cell** (`:168-169`): a "latest state" cell would
let a `stateChanged: ready` that arrived before the caller started waiting be overwritten,
hanging `wait_ready` to the absolute deadline.

Lock order is `pending → lifecycle`, and `mark_terminal` deliberately **releases
`lifecycle` before taking `pending`** so the two never nest (`:383-394`).

`EventPump` is a plain `std::thread`, deliberately not a gpui `Task` — a dropped `Task`
cancels, and the pump must outlive the effect (and the crate must stay UI-free)
(`events.rs:499-505`).

## `wait_ready`

Loops on `events.wait_lifecycle(min(remaining, 5s))` and **does not read events itself**
(`:222-288`). On an empty slice it checks `terminal_reason()` so a dead session is reported
immediately instead of after the 600 s absolute deadline. `stateChanged: failed` prints the
**whole event** — it is the only crash forensics available.

## `Session::request` — registration before send

```
operation_id = "lithe-gpui-java-{n}"
body += { sessionId, operationId, operation }
let waiter = events.register(&operation_id)?;      // MUST precede the send
if let Err(e) = core_json("lsp.request", body) { events.unregister(..); return Err(e) }
match waiter.recv_timeout(deadline - now) { .. }
events.unregister(&operation_id);
```

> Core proactively terminates in-flight requests whose document has gone stale
> (`lsp/interface/engine.rs:4289-4346`) and the response window can be extremely short, so
> late registration would make the result look unclaimed and drop it (`:345-346`).
> `register` also rejects a duplicate `operationId` and fails fast if the session is already
> terminal.

## Shutdown — three mandatory steps

`Session::shutdown(self)` is consuming (`:382-421`):

1. `lsp.stopServer`, envelope timeout 20 s.
2. **Bounded wait for terminal** — `events.wait_terminal(now + 5s)`. The pump is still
   alive and is what observes `stateChanged: stopped`. If it never becomes terminal:
   `S1_JAVA_SESSION destroy_skipped reason=not-terminal`, printed loudly because the JVM
   may be orphaned.
3. `lsp.destroyServer`, then `pump.stop()`.

> Both steps are required and the intermediate wait is **mandatory**. `stopServer` only
> *initiates* a bounded graceful shutdown and **keeps the session record**; `destroyServer`
> rejects a non-terminal session ("A running language-server session cannot be
> destroyed."). Calling them back to back without the wait yields `invalid_request` and
> leaves a **corrupted index** in `<cacheDirectory>/jdtls/<key>` and an **orphaned JVM**
> (`:374-381`).

`impl Drop for Session` and `impl Drop for EventPump` are the backstops for paths that drop
a `Session` without calling `shutdown()` (e.g. a failed `wait_ready`) — without them a
thread would hang forever on `waitEvents`.

## `JavaLanguageService` orchestration

`start_locked` (`service.rs:661-746`), with the state lock already held:

```
1. jdtls::resolve(&workspace_root)                            -> S1_JAVA_JDTLS
2. jdtls::resolve_runtime(installation_root(&executable))     -> S1_JAVA_TOOLCHAIN
3. workspace::plan(&root, &installation.version)              -> S1_JAVA_CACHE directory= key= reuse=
4. workspace::cleanup_expired(&cache, &key)                   -> S1_JAVA_CACHE_RECLAIM
5. workspace::maven_context(&root)                            -> S1_JAVA_MAVEN
6. directory_uri(&root)                                       -> S1_JAVA_START
7. Session::start(&spec)                                      -> S1_JAVA_SESSION started
8. session.wait_ready(now + 600s)                             -> S1_JAVA_READY elapsedMs= logEvents=
9. cache (installation, runtime) into self.installation
```

`is_java_workspace` (`:754-792`) calls `workspace.snapshot` **again** rather than reusing
the project tree's data, so the policy input comes from Core and not from `Explorer`'s
private state. The cost is one extra scan at project open (measured < 1 s). It is
**not** consulted by `definition` / `completion` / `code_actions` / `sync_document` — the
contract lets a host start on demand when the user opens a `.java` file (`:234-236`).

## Semantic requests

| Operation | Timeout | Notes |
| --- | --- | --- |
| `definition` | 60 s | `JavaTarget::{File, Virtual}`; `Virtual` carries a `jdt://` uri + display path + text |
| `completion` | 15 s | `textEdit` **must** be preserved — without it `Sys` + `System` becomes `SysSystem` (`:117-121`) |
| `codeActions` | 15 s | `context.diagnostics` is **mandatory** — JDT reverse-looks-up problem locations from the request's diagnostics and `getProblemId` parses `code` as an integer (`:381-393`) |
| `virtualDocument` | — | decompiled source for a `jdt://` target |

Non-obvious requirements in code-action handling:

- Edits are sorted **reverse-positionally** so applying them does not shift later ranges.
- Actions are filtered to what can land in the current buffer: edits on other files and
  anything carrying a `command` are dropped and counted in
  `S1_JAVA_CODE_ACTION … other_file_edits=… commands=… malformed=…`.
- `diagnostics_for` uses the cursor **line** for an empty selection — with a zero-width
  range, `Ctrl+.` on an indent would find nothing and the menu would degrade to source
  actions only (`:1049-1075`).
- Windows `/C:/…` normalization: Core's `file_path_from_uri` leaves a leading slash on
  `file:///C:/x`, so one leading `/` is stripped **only** when followed by an `X:` drive
  pattern (`:1027-1039`).
- Completion snippets are converted by **Core** (`lsp.plainSnippet`), not locally; on
  failure the original is returned rather than dropping the candidate (`:1178-1202`).
- `file_uri` is **public on purpose** — the editor's quick-fix provider must use the same
  URI mapping (`:912`).

## Staleness and failure handling

Three mechanisms, all in this crate:

1. **`is_superseded`** (`:938-940`) matches error prefixes `staleDocumentVersion@` and
   `requestCancelled@`. Both are **normal during fast typing** and are converted to
   `Ok(empty)` rather than an error — otherwise the editor would treat a cancelled
   completion as "server unavailable" and flap between JDTLS and builtin candidates. The
   error string format `"{code}@{stage}：{message}"` is a **compatibility surface** produced
   only by `events::completion_outcome` (`:456-459`); changing one without the other breaks
   both, and tests pin both sides.
2. **Terminal propagation** (`events.rs:383-403`): once `mark_terminal` runs, `register`
   fails immediately and **all** in-flight waiters receive `Err(terminal_reason)` instead
   of blocking for their own 15 s / 60 s deadlines.
3. **Diagnostic snapshot settling** (`service.rs:524-542`): `code_actions` snapshots
   diagnostics *before* `sync_document`; if `changed == true` it waits up to 2 × 250 ms for
   a **different** snapshot (exit condition is "changed", not "non-empty", because fixing
   all errors should yield an empty snapshot). Bounded: if the new errors are
   byte-identical the wait simply expires and the snapshot in hand is still correct.

**No auto-retry after a start failure** (`service.rs:19-22`): start failures are
environmental, and retrying 30 s+ on every `F12` would make every navigation a long block.
One diagnostic line, then batch-1 lightweight navigation takes over.

Diagnostics are only a **latest snapshot per `uri`**; an empty array is ambiguous between
"server cleared" and "never published" (`events.rs:246-250`), which is why the editor
re-reads after a bounded delay.

## Core commands used

`workspace.snapshot`, `java.workspacePolicy`, `java.jdtWorkspaceFingerprint`,
`java.jdtCacheRetention`, `lsp.jdtWorkspaceKey`, `lsp.startServer`, `lsp.waitEvents`,
`lsp.syncDocument`, `lsp.request`, `lsp.stopServer`, `lsp.destroyServer`,
`lsp.plainSnippet`, `maven.scan`.

## Known gaps

- **`JavaLanguageService` has no `Drop`.** `EditorPane::drop` must call `shutdown()`
  synchronously; nothing enforces this at compile time.
- `installation` is private with no accessor, which is why the settings "detected language
  servers" row says "unreadable on this side"
  (`gpui/crates/settings/src/dialog.rs:823`).
- The real-JDTLS smoke test is gated on `LITHE_GPUI_JDTLS_SMOKE` and additionally needs
  `LITHE_JDTLS_JAVA` pointing at a JDK 21+ (`service.rs:1554-1561`).
- The service is single-workspace-only and not root-switchable, because
  `EditorPane::prepare_java` is an idempotent early-return.

## Sources

- `rust/gpui/crates/java/src/session.rs`, `events.rs`, `service.rs`
- `rust/gpui/crates/editor/src/{navigation,diagnostics,completion,code_actions}.rs`
- `rust/gpui/crates/editor/src/editor_view.rs`
- `rust/gpui/crates/settings/src/dialog.rs`
- `rust/lithe-core/src/lsp/interface/engine.rs`
- `rust/lithe-core/src/lsp/lightweight/snippets.rs`
- `shared/fixtures/lsp/`
- `shared/contracts/rust-core-api.md`
- `.agents/notes/implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md`
- `.agents/notes/implemented/architecture/2026-09-18-java-project-build-and-launch-boundary.md`
