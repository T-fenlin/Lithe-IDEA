# Notification Centre

`rust/lithe-gpui/crates/notify/` is the smallest crate in the host and the one with the cleanest
state model. It depends on `gpui-kit` (only for `SharedString`) and `shared` (only for
`CoreError`). It **never calls Core** — it only consumes `CoreError` values produced
elsewhere.

| File | Responsibility |
| --- | --- |
| `model.rs` | `Entry`, `Target`, dedup, search |
| `severity.rs` | `Severity`, `Badge`, the Core-code table |
| `store.rs` | The store: `record`, `clear`, `mark_all_read`, `badge` |
| `core_bridge.rs` | `CoreError` → `Input`, `is_reportable` |
| `lib.rs` | Layering docs, `now_ms`, the public surface |

The UI itself lives in `workbench/src/notifications.rs` (`NotificationPanel`: search,
severity filter, expandable rows, clear-all).

## State

`Store { entries: Vec<Entry>, next_seq: u64, read_upto_seq: u64 }`
(`store.rs:125-135`). `entries` is **always newest → oldest** and is never sorted —
`record` inserts at index 0 (`:196-211`).

`Entry` stores a `tr` **key plus interpolation params, never finished text**
(`model.rs:6-11`), per `application-boundary.md:227-228` — domain layers return stable
reasons, the presentation layer owns localized text.

**Unread is a watermark, not a per-entry `read: bool`** (`model.rs:19-24`): the macOS
product's `markAllNotificationsRead()` on open made a per-entry flag meaningless.

Three well-designed state-machine properties:

- **Dedup** — same `key` ⇒ `replace_with` in place, moved to the front, `seq` refreshed so
  it **becomes unread again**, `id`/`created_at_ms` preserved, `occurrences` incremented.
  No key ⇒ always a new row.
- **Capacity** — over `MAX_ENTRIES = 100`, truncate from the **back** (oldest).
- **`clear()` couples entries and watermark** and deliberately does **not** reset
  `next_seq`. Clearing the watermark is what makes the next project's entries unread, and
  not recycling ids avoids silently deleting an entry the UI still holds.

Because the watermark is seq-based, deletion, dedup-replace and truncation never distort
it.

## Severity mapping

`Severity::for_core_code` (`severity.rs:59-75`):

| Severity | Codes | Why |
| --- | --- | --- |
| *(not recorded)* | `cancelled` | Caller-cooperative; recording it would flood the centre with normal user actions |
| `Error` | `invalid_request`, `workspace_not_found`, `permission_denied`, `process_start_failed`, `process_failed`, `parse_failed` | The thing did not happen; the user must act |
| `Warning` | `not_supported`, `runtime_missing`, `timed_out`, `unknown` | Degraded but usable |
| `Warning` (fallback) | anything else | **Never escalate an unknown code to error** — we do not know whether it needs action, and the red dot is the one signal users actually watch |

A test asserts the count is still 11, so adding a Core code breaks the build rather than
silently falling through (`:146-157`).

Notification keys are **separate** from Core codes
(`notification_code_for_core_code` maps to camelCase keys under
`lithe.notifications.core.*`, with a test asserting the namespace and the absence of `_`,
because a wrong key shows raw braces in the UI rather than erroring). Severity is never
duplicated into that table — it always comes from `for_core_code`, to prevent drift
(`:82-84`).

`Severity` is `Info | Warning | Error` — `success` is deliberately absent, because success
is not something you review afterwards and it uses the existing green bar
(`model.rs:14-18`).

`Badge` is `None | Info | Error`, and **red beats blue**: any unread error ⇒ red
(`store.rs:262-281`). Mirrors JetBrains — colour encodes severity, not count. Numeric
badges are explicitly deferred (`:110-114`).

## The Core-error bridge

`core_bridge.rs` maps a `CoreError` to a notification `Input`, and `is_reportable` is the
predicate the shell uses to decide whether to record at all. Two rules:

- `MissingData` never becomes a notification (`:59`): `ok: true` with no `data` usually
  means "nothing to show", so reporting it would produce a stream of "success but no data"
  noise.
- `InvalidResponse` maps to `parse_failed` (`:53-57`).

`from_core_error` adds `detail` (Core's message) and `command` params, so the detail view
shows *which command* failed and search can find it. And `is_reportable(&CoreError)` must
**always** agree with `from_core_error(..).is_some()` — a test pins that over 8 cases
(`:157-184`).

Diagnostics still go to the shell's `S1_*` stderr lines; this bridge **adds** a
user-visible record, it does not replace the diagnostic line (`:39-42`).

## Where the store lives — and why not a `Global`

`ShellWorkspace` holds **one** `Entity<Store>`. Entities that emit notifications receive a
**`WeakEntity<Store>`** — `editor/src/editor_view.rs:349`. A gpui `Global` is not usable
because `try_global` / `set_global` exist only on `App`, and `&mut Context<T>` cannot reach
`&mut App` (`lib.rs:32-41`).

The store lives in a **separate crate** because `workbench` depends on every feature
crate; if the store lived in `workbench`, `git`'s write failures and `editor`'s save
failures would have to depend back on `workbench` — a cycle (`:24-31`).

Rebuilding the root rebuilds the shell, so the next project's centre is naturally empty
(new store, watermark 0) with no explicit clear. The accepted trade-off: notifications
arriving before the root change die with the old store.

The shell's entry point is `ShellWorkspace::notify(Input, cx)`
(`workbench/src/workspace.rs:3273-3292`), which stamps `now_ms` and logs
`S1_NOTIFICATION action=record code=… severity=… state=inserted|replaced`.

## Search semantics are deliberately asymmetric

The stable code is matched **ASCII case-insensitively** (it is always an ASCII
identifier), but **param values are matched verbatim** — case folding would surprise on
Chinese text and case-sensitive paths (`model.rs:207-224`). Param *names* are not
searchable (`:310-316`). The rendered text is matched by the view via `matches_text`
(`:227-235`).

`now_ms` is a **wall clock**, not monotonic (`lib.rs:59-69`) — timestamps are user-visible,
and pre-1970 degrades to 0 rather than failing a frame. `Store::record` takes the clock as
a **parameter** so tests never touch real time.

## Known gaps

- `Target { File { path, line }, ToolWindow { id } }` has **no producer** — it is reserved
  for IDEA's "description + a jump link" pattern and a dead link is worse than no link
  (`model.rs:50-57`).
- `occurrences` is stored but not displayed, because the right tool window is only ~350 px
  wide; keeping the field means a future "×N" needs no migration (`:147-148`).
- The doc example in `core_bridge.rs:24-26` calls `lithe_gpui_notify::record(cx, input)`,
  which is not exported (`lib.rs:54-57`). The real path is
  `store.update(cx, |s, _| s.record(input, now_ms))`. Stale example.

## Sources

- `rust/lithe-gpui/crates/notify/src/` (all files)
- `rust/lithe-gpui/crates/notify/Cargo.toml`
- `rust/lithe-gpui/crates/workbench/src/notifications.rs`, `workspace.rs`
- `rust/lithe-gpui/crates/editor/src/editor_view.rs`
- `rust/lithe-gpui/crates/shared/src/core_client.rs`
- `rust/lithe-core/src/protocol/error.rs`
- `shared/contracts/application-boundary.md`
- `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`
