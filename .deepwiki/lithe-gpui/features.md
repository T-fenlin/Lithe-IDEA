# Feature Panes: Explorer, Git, Terminal

The three side/bottom panes the shell hosts. All three follow the same shape: a
data-side module that owns every Core call and every metric constant, plus a view-side
module that owns rendering only. All three reach Core through
`shared::core_client`, and all three are plain gpui `Entity`s owned by `ShellWorkspace`.

---

## Explorer — the project tree

Smallest of the three. `model.rs` (data) + `explorer_view.rs` (view).

**One Core command**: `workspace.snapshot { root }` (`model.rs:241`), issued inside
`cx.background_spawn` and applied via `this.update`. The snapshot is a flat list of
workspace-relative paths; `PathTree::insert` (`:66-91`) aggregates it into a multi-level
tree, then `build_tree_items` (`:129-157`) flattens that back into gpui `TreeItem`s for the
`uniform_list`. `RENDER_LIMIT = 5_000` (`:35`).

State: `LoadState { Loading, Ready, Failed(String) }` where the string is the **Core error
code verbatim** (`explorer_view.rs:130-141`), plus `expanded: BTreeSet<SharedString>` of
`dir:<relpath>` ids, `active`, and a `WeakEntity`/callback pair for opening.

Three non-obvious rules:

- **The empty-directory placeholder is load-bearing.** `TreeItem::is_folder()` is
  `!children.is_empty()` in gpui, so a childless directory would render as a *file* row
  and clicking it would call `on_open` with a directory path. `empty_placeholder`
  (`model.rs:113-119`) attaches one disabled row (`:28-30`).
- **Row element ids derive from `TreeItem.id`, not the row index**, because the `Tree` is a
  `uniform_list` and index-based keys leak hover/selection state between rows after
  expand/collapse/filter (`explorer_view.rs:711-713`, test `:756-775`).
- **`rebuild` clears selection**, because `TreeState::set_items` does (`:336-337`).

**There is no filesystem watcher in this crate.** `Explorer::refresh` exists for the shell
to wire, but the right-click refresh action was not implemented, so the only caller is
`ShellWorkspace::new` immediately after construction. `git.status` decorations and
auto-reveal are in the crate's "not implemented" register (`lib.rs:86-106`).

**No staleness guard.** `load` (`:289-332`) has no request-serial or generation check, so
two overlapping snapshots can resolve out of order. This is a real gap relative to the
`git` crate's `request_serial` pattern.

---

## Git — two independent features

`lithe-gpui-git` hosts **two unrelated features plus two data boundaries**:

| File | Layer |
| --- | --- |
| `model.rs` | Everything non-rendering for the bottom pane: Core calls, pagination cursor lifecycle, row data, and **~40 metric constants** (`:78-233`) that are the single source of truth for layout |
| `log_view.rs` | `BottomPane` — the commit log + console shell (`:105-150` struct, 27 fields) |
| `changes.rs` | Left-rail data layer: `ChangeKind`, `ChangesSnapshot`, `load`, `write`, `stage`/`unstage`/`commit`/`discard` |
| `changes_view.rs` | `ChangesView` — the source control view |
| `branch_info.rs` | Read-only data boundary for the title-bar branch popup; `BranchSnapshot::load` is **synchronous** by design |
| `identity.rs` | Git identity data layer implementing the settings page's `GitIdentityHost` hook |

### Two independent state machines

`BottomPane.load_state` is a **five**-variant enum: `Loading, Ready, Stale, Failed,
NoRepository` (`model.rs:242-261`). `Stale` means "there is data but the last refresh
failed" — a distinction the two-state views do not make.

Its `cursor: Option<HistoryCursor>` **holds a live `git log` child process** inside Core.
That makes cursor lifetime the most important invariant in the crate: it must be returned
on exactly three paths — first load (taken and closed before the background task runs,
`:779-781`), a failed "load more" (Core already stopped the session, `:336-340`), and
`impl Drop for BottomPane` (`:2034-2042`). Forgetting one leaks a process.

`ChangesView.state` is `LoadState { Loading, Ready, Failed(String) }` for **read** failures
only. Write failures go to a separate `error: Option<String>` rendered as a persistent red
bar, and successes to `notice` as a green bar. `record_write` (`:683-709`) is the unified
tail: diagnose, surface the failure, then **always** `refresh`.

`git.write` returns `ok: true` even when Git exits non-zero, so `WriteOutcome::failure` is
derived from `operationError` / non-zero `exitCode` plus the first meaningful stderr/stdout
line, with the repository root **redacted out** of the message (`changes.rs:488-589`).

### The stale guard

`request_serial` is a monotonically increasing `u64`. `spawn_first_load` /
`refresh` bumps it; `apply_first` / `apply` drop any result whose serial is not current
(`log_view.rs:233-251`, `changes_view.rs:327-358`).

Two known holes: `BottomPane::apply_more` (`:333`) and `apply_commit_files` (`:398`) take
**no serial**, so a slow `historyPage` append or `commitFiles` for a previously selected
commit can land after a newer request. Only the first-load path is guarded.

### Deliberate deviations from library components

The crate documents 14 of them in `lib.rs`. The ones that cost something:

- **Hand-drawn icon buttons** in the log pane because `Button`'s icon renders at 18 px
  instead of the specified 14 px — the cost is **no hover tooltip**, only `aria_label`
  (`:148-151`). `ChangesView` took the opposite trade (real `Button`, because tooltips
  matter for stage buttons) at the cost of uniform row geometry.
- **A hand-written row instead of a `Tree`**, because the explorer's `Tree` row has no
  action slot and the list mixes category headers with variable-height rows, which
  `uniform_list` cannot do (`changes_view.rs:8-31`). Cost: no virtual scrolling, and the
  stage button is always visible rather than hover-only.
- **The three-column split uses `relative()` fractions, not `ResizablePanel`**
  (`0.19 / 0.57 / 0.24`, min widths 140 / 320 / 220) — so the columns are **not
  draggable** (`:132-137`).
- **A simplified swimlane graph**: per-lane 1.6 px vertical lines plus node circles;
  cross-lane parent edges are straight vertical segments, no arcs; missing parents use
  `opacity(0.7)` instead of `strokeDasharray` (`:138-140`).
- **The console tab is a shell only** — the full chrome renders (32 px toolbar, six icon
  buttons, all `enabled = false`) but the real output needs the Git execution event
  channel, which is out of scope for this crate (`log_view.rs:1875-1930`).

Product-level constraints: `%D` decorations cannot distinguish a local branch containing
`/` from a remote ref (`:623-624`); Core's commit format is `%s`, so there are no commit
bodies and the Inspector's description section is not drawn; commit always uses the index
because there is no per-file scope checkbox, which is why the count label is
`git.filesStaged` and not `filesSelected`.

### Timeout

`GIT_TIMEOUT_MILLIS = 60_000` (`model.rs:491`), applied via `with_timeout_millis`
(`:507`). It deliberately does **not** follow the shared 120 s default, because Git
operations are interactive and a hung prompt is worse than a bounded failure.

### Identity

`identity.rs` implements the settings page's `GitIdentityHost` hook. It returns Core JSON
**text** rather than `lithe_gpui_git` types, precisely so `settings` does not need to
depend on the git crate (`:232-260`). `IDENTITY_MAX_BYTES = 1024` is measured in **bytes,
not characters**, and `redact` strips the repository root from messages.

---

## Terminal — process sessions without a PTY

`ansi.rs` (pure state machine) + `profile.rs` (shell discovery) + `session.rs` (process
lifecycle) + `terminal_view.rs` (rendering) + `constants.rs`.

### No PTY

`Session::start` (`session.rs:98-148`) uses `std::process::Command` with three pipes.
Windows would use `portable-pty` + ConPTY; this crate does not. The consequences are
listed in the crate's always-visible capability notice: no full-screen TUI (`vim`, `top`,
`less`), `Ctrl+C` is unusable (closing the tab is the only way to stop a running command),
no selection / find / links / cursor / mouse, and soft wrap by viewport instead of a column
grid. That notice (`constants.rs:66`) is the only user-visible string not from the locale
catalogue, deliberately.

One dedicated reader thread per stream merged into a single `mpsc::channel<SessionEvent>`
(`:210-236`) — arrival order is display order. `mpsc` has no async receiver and the crate
does not depend on `async-channel`, so the pump wraps the blocking `recv()` in
`cx.background_spawn`; `blocking_recv` exists solely so the `!Send` `MutexGuard` never
escapes into the `Send` async block (`:258-264`).

### ANSI: strip, not interpret

`ansi.rs` is a pure state machine with no GPUI, so it is unit-testable.
`EscapeState { None, Esc, Csi, Osc, OscEsc }` (`:24-36`):

- `\r` sets `cursor = 0` and subsequent characters **overwrite** (`:181-188`)
- `\n` ends the line, `\b`/`\u{7f}` backspace
- `Esc` → `[` is `Csi`, `]` is `Osc`, anything else is dropped wholesale (so
  two-char sequences like `ESC ( B` vanish)
- `Csi` ends on a final byte `0x40..=0x7E`; **all SGR is discarded — there is no colour
  at all**
- `Osc` ends on BEL or `ESC \`

UTF-8 across chunk boundaries is handled with `valid_up_to()`: an incomplete tail
(`error_len() == None`) is kept for the next chunk so no spurious `U+FFFD` appears; a real
error is lossy-replaced and counted. Known limitation: `cmd.exe` at code page 936 yields
`�` for Chinese.

### Sessions are lazy

`TerminalPane::new` spawns **nothing** (`session.rs:275-280`); the host must call
`ensure_session` on first visibility. Empty `tabs` is the *correct* state, not
"not ready yet". `ensure_session` defers focus with `window.defer` because the input row is
only mounted after the current frame's render (`:331-344`).

The input row is drawn **only while `session.is_running()`** (`terminal_view.rs:509`), and
there is **no local echo** — `cmd.exe` and `powershell.exe` echo the prompt and the command
themselves under pipes (`:649`).

### Generation guard and cleanup

`TabSession.run` is the generation counter. `retry_tab` bumps `next_run` and reassigns
`tab.run`; the old pump, still holding the old value, returns `PumpStep::Stop`
(`:738-741`) and in `settle_exit` (`:808-810`). The same technique the source product used
with `execution_id`.

Three cleanup paths, all `kill()` then `wait()`: `close_tab`, `retry_tab`, and
`impl Drop for TerminalPane` which loops **every** tab. `Session::shutdown` drops the
writer first so the shell exits cleanly, then kills, then waits — `wait()` is explicitly
non-optional to avoid zombies (`:169-181`).

Exit detection: after both pipes hit EOF, `settle_exit` polls `try_wait` up to
`EXIT_POLL_ATTEMPTS = 10` times at `EXIT_POLL_INTERVAL = 20 ms`.

Scrollback is capped at `MAX_LINES = 10_000`, trimmed **from the front**
(`session.rs:784-787`), with the `MessageScrollerState` item count kept in sync on every
append.

### Settings wiring

`TerminalPane` has its own `font_size: Option<f32>` field rather than writing
`Theme::mono_font_size`, because that token is the editor's and changing it would restyle
the editor too (`terminal_view.rs:101-106`). `set_default_shell` and `set_font_size` are
**idempotent on purpose** — the shell re-pushes them on *every* settings change, and a naive
implementation would reset a profile the user picked from the tab-bar menu (`:468-475`).

Shell profiles are probed at runtime via `where.exe` then `which` — no `#[cfg]`
(`profile.rs:130-146`) — and cached in a `OnceLock`. `wsl` is only reachable when
explicitly named in settings.

## Sources

- `rust/lithe-gpui/crates/explorer/src/` (all files)
- `rust/lithe-gpui/crates/git/src/` (all files)
- `rust/lithe-gpui/crates/terminal/src/` (all files)
- `rust/lithe-gpui/crates/shared/src/core_client.rs`
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-core/src/project/files.rs`
- `rust/lithe-core/src/git/`
- `shared/contracts/terminal-profiles.md`
- `shared/fixtures/terminal/`, `shared/fixtures/git/`
