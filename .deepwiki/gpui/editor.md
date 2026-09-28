# Editor

`gpui/crates/editor/` is 5 modules, ~5 400 lines, and holds the tab bar, buffers,
navigation, diagnostics, completion and code actions. There is no `tests/` directory —
tests are inline `#[cfg(test)] mod tests` in every file.

| File | Responsibility |
| --- | --- |
| `lib.rs` | Crate contract, the public surface, the three actions, and an explicit out-of-scope register |
| `buffer.rs` | Data side of one open buffer: read/degrade, `Buffer` struct, file-type icon, syntax language, tab display-name disambiguation |
| `editor_view.rs` | `EditorPane` + presentation: open/close/save/auto-save/reload, tab bar, tab context menu, empty state, navigation landing, diagnostics scheduling, `Render`, `Drop` |
| `navigation.rs` | JDTLS-first target resolution with Core lightweight fallback, the three column converters, `JumpHistory` |
| `diagnostics.rs` | Diagnostics data side: sync + bounded re-fetch, whole-snapshot write, severity mapping |
| `completion.rs` | Java completion: JDTLS-first with `lsp.builtinCompletions` fallback, generation-guarded staleness, trigger policy |
| `code_actions.rs` | Java quick fixes (`Ctrl+.`), and it **applies the edit itself** |

Public surface (`lib.rs`): `EditorPane`, `CursorPosition`, `SessionRestore`,
`TabMenuHostActions`, `install_actions`. One action is registered here: `Ctrl+S`.

## `Buffer`

`buffer.rs:280-343`, constructed only via `Buffer::new` (`:347-369`).

| Field | Role |
| --- | --- |
| `path` | Open-time path; the **dedup key** and the save target |
| `name`, `icon` | File name and tab icon. The icon is resolved **once at open**, light/dark pinned to the open-time theme (a deliberate trade with a visible consequence) |
| `editor: Entity<EditorState>` | The text state, one per tab |
| `is_dirty` | **Lifecycle-based, not content-based**: an `InputEvent::Change` sets it, a successful write clears it. Reverting an edit by hand still leaves it dirty, and no full-body materialization happens per keystroke (`:298-304`) |
| `writable` | Whether the body faithfully represents the file; gates readonly, dirty and save |
| `save_generation` | Debounced auto-save version |
| `revision` | Monotonic text revision, **never reset**; guards navigation and diagnostics |
| `auto_save_task`, `diagnostics_task` | `Option<Task<()>>` — replacing the slot **is** the `clearTimeout` |
| `diagnostics_generation` | Generation of the newest scheduled refresh |
| `_subscriptions: Vec<Subscription>` | Must be held; drop unsubscribes |

**There is no line index and no undo stack in this crate.** Both live in the upstream
`EditorState` (a `Rope` plus `RopeExt`). Undo is exercised only indirectly: code actions
use `EditorState::apply_lsp_edits` specifically so the change enters the undo stack,
`set_value` would suppress event emission (`code_actions.rs:58-63`).

## The three column conventions

The most carefully documented invariant in the crate. Three conventions exist and are
only equal on ASCII (`navigation.rs:39-49`):

1. **byte offset** — `EditorState::cursor()`
2. **character column** — `Position.character`, used by the status bar and `DiagnosticSet::push`
3. **UTF-16 code-unit column** — Core/JDT `utf16Column`

Two functions are the single source of truth:

```rust
core_position(text, offset)   -> (u64 line, usize utf16_column)   // navigation.rs:231-245
editor_position(text, line, utf16_column) -> Position              // navigation.rs:247-258
```

Used by navigation, diagnostics (both endpoints), completion (both `text_edit`
endpoints) and code actions (at apply time). Non-BMP tests with `🎉`/`🙂` prove the two
are not interchangeable (`navigation.rs:492-510`, `diagnostics.rs:393-448`,
`completion.rs:582-594`, `code_actions.rs:505-537`).

## Tabs

One tab per `Buffer`, deduped by `path`: reopening an open path only activates it, never
re-reads (`editor_view.rs:762-778`). `jdt://` buffers are keyed by the virtual URI and are
read-only (`open_virtual`, `:1272-1354`).

| Field | Line | Role |
| --- | --- | --- |
| `buffers: Vec<Buffer>` | `:350-351` | Order **is** the tab-bar order |
| `active: Option<usize>` | `:352-353` | `None` → empty state |
| `closed: Vec<PathBuf>` | `:420-430` | LIFO stack, `MAX_CLOSED_TABS = 20` |
| `hovered_tab: Option<usize>` | `:407-419` | The × only shows on non-active tabs. Tracked **manually** because gpui's `.group_hover` was measured not to repaint (artifact hashes are in the comment) |
| `history: JumpHistory` | `:357-358` | `←/→` definition navigation |

Element identity is **path-derived**, not index-derived: `editor-tab:<path>` and
`editor-tab-close:<path>` (`:2752-2772`), with a regression test proving order
independence (`:2856-2875`).

Dirty-close confirmation uses one shared dialog with three outcomes
(`PendingConfirm::{Single,Batch,Reload}`, `:310-319`). A batch close asks **once** for N
dirty files (`:1976-2019`) and applies closing **high index first** to avoid index-shift
corruption (`:2026-2035`).

## States

There is no explicit loading/failed state machine; states are encoded per buffer.

| State | How it appears |
| --- | --- |
| loading | `restore_entry` reopens a closed target by path and awaits `open` (`:1387-1417`); a `FALLBACK_LINE_HEIGHT` compensates because `line_height()` is `None` before first layout (`:149-162`) |
| ready | normal `Editor` render |
| failed / degraded | `read_body` substitutes a `// `-prefixed notice body and marks it non-writable, so the editor is read-only and **cannot overwrite the real file** (`buffer.rs:93-148`, `editor_view.rs:845-851`). Four classes: unreadable, > 2 MiB, binary (contains NUL), non-UTF-8 lossy — all four tested (`buffer.rs:409-481`) |

Navigation failure: the cursor does not move, `S1_NAV_FAILED reason=…` prints, and a
deduplicated notification-centre entry is recorded. Save failure: dirty stays true,
`S1_EDITOR_SAVE_FAILED` to stderr, plus an Error notification with a per-failure dedup key.
Teardown: `impl Drop` calls `service.shutdown()` **synchronously** — a detached thread
would be killed at `main` return and orphan the JVM (`:2803-2815`).

## Navigation

`F12` → `NavigateToDefinition` (registered at `lib.rs:236-240`, handled on the pane root
at `:2830-2834` so the action bubbles from the focused editor). `Ctrl+click` uses the
identical path via `on_mouse_down` on the wrapper div, guarded by `Modifiers::secondary()`
and by `input_bounds().contains(position)` so clicking empty space does not jump
(`:1226-1270`).

`resolve_target` has three decision rules, written at the call site
(`navigation.rs:113-168`):

1. not a `.java` file, or no service → **lightweight** (Core `lsp.builtinNavigation`)
2. JDTLS returned a result — even "no positions" → **authoritative, no fallback**
3. JDTLS returned `Err` → fallback + `S1_JAVA_UNAVAILABLE`

`apply_definition` performs a three-part staleness check (generation, buffer still present,
revision unchanged) before landing `Local` / `File` / `Virtual`, and records history **only
after success**. Each jump logs `S1_NAV_JUMP … via=builtin|jdtls|jdtls-virtual`.

`JumpHistory` stores `JumpEntry { path, position }` — **path, not buffer index**, because
closing a tab shifts indices. `MAX_JUMP_ENTRIES = 100`; on overflow the oldest is dropped
**and the index decremented**. Current status: the `←/→` buttons were removed from the tab
bar on 2026-09-27, so `go_back` / `go_forward` currently have no production call sites
(`#[allow(dead_code)]`); the suggested re-attachment is `Alt+←` / `Alt+→`.

## Diagnostics

Sourced from JDTLS `publishDiagnostics` snapshots, read after `sync_document`, and written
through the upstream host seam `EditorState::diagnostics_mut()` — wiring, not custom
painting.

- **Write semantics: clear then full write, never merge** (`diagnostics.rs:200-210`).
- **No caching or replay.** Upstream resets the `DiagnosticSet` on every text change, so
  replaying a cached snapshot would put squiggles back on fixed code (`:52-54`).
- **Throttling:** `CHANGE_DEBOUNCE = 400 ms` for text changes, `Duration::ZERO` for
  open/prepare/reload. Longer than completion's 120 ms because diagnostics are far more
  expensive (`:100-105`).
- **Bounded re-fetch:** `RETRY_BACKOFF_MS = [250, 500, 1000, 2000, 4000]`, ≤ 5 attempts,
  ≤ 7.75 s. Stop criterion is **non-empty AND identical to the previous poll**; an empty
  snapshot never stops early (`:125-132, 158-170`).
- **Three-layer staleness guard**, each with its own log line: buffer still present,
  generation matches, revision matches (`editor_view.rs:693-735`).
- **Known coupling:** `highlight_lines` returns early with no highlighter *before* diagnostic
  styles are computed, so "no syntax highlighting ⇒ no squiggles" (`diagnostics.rs:23-27`).

## Completion

- **JDTLS first, then Core** `lsp.builtinCompletions` (process-free, current-file
  identifiers with a prefix `textEdit`).
- **Trigger:** upstream calls `is_completion_trigger` on every text change, so pop-on-typing
  needs no key binding; the predicate accepts identifier chars, `.` and `@`.
- **`autoCompletion` gates the trigger, not the installation.** The provider stays installed
  so a future `Ctrl+Space` can call `completions()` directly, and one shared
  `Rc<AtomicBool>` flip affects all open buffers (`completion.rs:108-129`).
- **Staleness:** a per-provider `Arc<AtomicU64>` generation; a non-current generation returns
  an **empty** response so the menu simply does not update, rather than showing candidates
  for an older prefix.
- **Column correctness:** `text_edit` endpoints converted with `editor_position` on the
  buffer's own rope; `insert_text_format` forced to `PLAIN_TEXT`.
- **Known boundary:** upstream's `insert_completion` ignores `additional_text_edits`, so
  JDT's "auto-add import on accept" does not happen (`:338-340`).

## Code actions

`Ctrl+.`. The provider is installed **only when a service handle exists** because it has no
fallback source. Three non-obvious requirements:

- Edit ranges stay in **UTF-16** in the `CodeAction` value and are converted only at apply
  time against the then-current rope — deferred to the single place that can compute it
  correctly.
- Edits are applied **back-to-front** because `apply_lsp_edits` applies sequentially.
- The apply must happen **inside the returned `Task`**: upstream calls
  `perform_code_action` while already holding a mutable borrow of the same entity, which
  would panic (`code_actions.rs:19-43`).

## Syntax highlighting

A two-key switch: extension → language *name* via `language_for_file`, and grammar
availability via a **Cargo feature**. `gpui-kit` is declared with
`features = ["tree-sitter-java"]`, which implies `tree-sitter` + `tree-sitter-json` — so
Java and JSON light up together. `tree-sitter-languages` (35 grammars) is deliberately off
for build and binary cost (`editor/Cargo.toml:8-19`).

`language_for_file` has only two entries: `.java → "java"`, `.json`/`.jsonc` → "json"; a
typo here is invisible, so all three cases are tested (`buffer.rs:488-518`). `S1_EDITOR_LANG`
reads back the stored name — which proves the name landed, not that the text is coloured.

## Performance discipline

| Constant | Value | Site |
| --- | --- | --- |
| `AUTO_SAVE_DEBOUNCE` | 150 ms | `editor_view.rs:164-166` |
| `MAX_EDITOR_BYTES` | 2 MiB (hard refuse, no degrade) | `buffer.rs:15-21` |
| `CHANGE_DEBOUNCE` | 400 ms | `diagnostics.rs:100-106` |
| `COMPLETION_DEBOUNCE_MS` | 120 ms | `completion.rs:68-74` |
| `COMPLETION_MENU_WIDTH_PX` | 480 (upstream 320 truncates Java signatures) | `completion.rs:55-66` |
| `MAX_CLOSED_TABS` | 20 | `editor_view.rs:440-445` |
| `MAX_JUMP_ENTRIES` | 100 | `navigation.rs:73-75` |
| `TAB_MAX_WIDTH` | 200 spec px | `editor_view.rs:140-147` |

**Cancellation-by-drop is the debounce primitive.** Every background job is stored in a
field, so replacing the slot cancels the previous one — `auto_save_task`,
`diagnostics_task`, `nav_task`, `java_task`.

Layout discipline worth knowing: `min_h_0()` + `overflow_hidden()` on every container;
explicit `h(relative(1.))` on the editor because multi-line mode is `.h_auto()` and
defaults to 2 rows; the tab close button is **absolutely positioned** so tab width does not
jump when switching tabs; scroll wheel is hand-wired because upstream editor styles have no
`overflow: scroll` and gpui therefore never routes wheel events to it.

**`rem_px(rem, spec_px) = AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)`** is
the one correct rem→pixel conversion in the repo (`editor_view.rs:103-113`, tested at
`:2889-2905`). It exists because `TabBar::max_width`, `Button::rounded`,
`Dialog::width` and `Size::with_size` accept only `Pixels`. `/ 4.` is wrong; 1 rem = 16 px.

**Work still in render, flagged honestly:** `render_tab_bar` recomputes
`display_names(&self.buffers)` every frame (`:2208`; the O(n·depth) algorithm is at
`buffer.rs:197-264`). Small for tens of tabs, but not memoized.

## Out of scope

No splits or editor groups. `buffers` is a flat `Vec` with a single pane. Split-right,
split-down and editor-group-lock are absent from the tab menu because they depend on a pane
tree that does not exist (`lib.rs:172-174`, `editor_view.rs:1841-1853`). The repository's
resizable-UI performance note places no obligation on this crate.

## Sources

- `gpui/crates/editor/src/` (all files)
- `gpui/crates/editor/Cargo.toml`
- `gpui/crates/java/src/service.rs`
- `gpui/crates/shared/src/core_client.rs`
- `rust/lithe-core/src/editor/line_edit.rs`, `tests.rs`
- `rust/lithe-core/src/lsp/lightweight/`
- `.agents/notes/implemented/bug-fix/2026-09-15-external-document-changes-and-guarded-save.md`
- `.agents/notes/implemented/feature/2026-09-22-editor-run-markers-and-test-outcomes.md`
