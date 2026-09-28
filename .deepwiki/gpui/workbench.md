# Workbench

`rust/gpui/crates/workbench/` is the shell: 15 modules, ~287 kB in `workspace.rs` alone.
Its public surface is deliberately tiny — only `ShellWorkspace` is public, plus
`right_tool_window::RightToolWindowView` (re-exported via the module path only, because
`app/src/main.rs:48-50` needs to name it). Area modules expose only their render
functions (`lib.rs:104-108`).

## Layout

The canonical tree is an ASCII diagram in `workspace.rs:1-18`:

```
v_flex
├─ title bar row 1                30    menu bar
├─ title bar row 2                38    project dropdown / branch / run / window controls
├─ project tab strip              32
├─ workspace h_flex flex-1 gap 4
│  ├─ left activity rail        38+4
│  ├─ left side pane           320    (runtime-draggable; SIDEBAR_WIDTH_SPEC)
│  ├─ central column            flex-1 = v_flex[ editor island, bottom pane 240 ]
│  ├─ right tool window         400    (collapsible)
│  └─ right activity rail      38+4
└─ status bar                     24
```

Implementation is `ShellWorkspace::render` at `workspace.rs:3834-3981`. Region owners:

| Region | Render function | Site |
| --- | --- | --- |
| Title bar (both rows) | `title_bar::title_bar` | `:3869-3875` |
| Menu bar (two visual forms) | `menu_bar::render_bar` | `:3550` |
| Project dropdown / branch trigger | `render_project_menu` / `branch_trigger` | `:3554-3565` |
| Project tab strip | `project_tabs::project_tabs` | `:3881-3890` |
| Workspace-config error banner | `render_workspace_config_error` | `:3321`, `:3896` |
| Left / right rails | `activity_bar(ActivitySide::.., ..)` | `:3747-3768` |
| Left pane + drag handle | `side_pane` / `drag_handle` | `:3999`, `:4084` |
| Editor island / bottom pane | `editor_island` / `bottom_pane` | `:4175`, `:4197` |
| Right tool window | `right_tool_window` | `:3781-3788` |
| Status bar | `status_bar::status_bar` | `:3971-3977` |
| Dialog / sheet / notification layers | gpui-kit `Root::render_*_layer` | `:3530-3534`, attached `:3979-3981` |

`Root`'s three overlay layers **must** be attached to the outermost view, or dialogs,
sheets and notifications silently do not show. They are computed first because they borrow
`cx` and everything after needs it.

## Module map

| File | Responsibility |
| --- | --- |
| `workspace.rs` | `ShellWorkspace` (struct `:787-1061`, `new` `:1071-1600`, `Render` `:3528-3985`), `ShellStartup`, activity indices, metric constants, lazy-scan state, ~25 unit tests |
| `title_bar.rs` | Two-row title bar with drag regions and self-drawn 56×38 window controls |
| `project_tabs.rs` | Stateless project tab strip; `ProjectTab` is a pure value with only a `name` |
| `activity_bar.rs` | Stateless left/right icon rails; `ActivityItem`, `ActivityIcon`, `ActivitySide` |
| `status_bar.rs` | Stateless status bar; `StatusEntry` |
| `right_tool_window.rs` | `RightToolWindowView` enum, `resolve_click`, `right_tool_window`, `diagnose` |
| `menu_bar.rs` | 9 top-level menus / 89 items, two visual forms, `MenuAction`, `MenuRequest` queue, thread-local handles |
| `command_palette.rs` | `Ctrl+Shift+P` overlay built from `Dialog` + `Command` |
| `project_menu.rs` | Title-bar project dropdown (3 sections, 2 dividers) |
| `branch_panel.rs` | Branch trigger + popup (read-only v1) |
| `session.rs` | `.lithe/session.local.json` timing layer: `SessionTracker`, `SessionPlan`, 300 ms debounce |
| `notifications.rs` | `NotificationPanel` entity: search, severity filter, expandable rows, clear-all |
| `maven.rs` / `spring.rs` | Right-window **data layers** (`maven.scan` / `spring.index` → view models) |
| `spring_paths.rs` | Bounded input list for `spring.index` (`MAX_SPRING_PATHS = 5_000`, `MAX_SCANNED_ENTRIES = 200_000`) |

## How child panes are hosted

All four are long-lived `Entity`s owned as `ShellWorkspace` fields, constructed in this
order in `new()` (`:1130-1226`) — the order is load-bearing:

1. `Entity<notify::Store>` — created **first** because the editor receives its
   `WeakEntity`.
2. `Entity<EditorPane>` — then immediately `pane.prepare_java(root)`.
3. `Entity<Explorer>` — constructed with `on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App)>`
   closing over `editor.downgrade()`. **This is how the shell decouples explorer from
   editor** (`:1144-1158`).
4. `Entity<ChangesView>` — no fetch at construction; the first read happens on `activate`.
5. `Entity<TerminalPane>` — no session; created lazily on first visibility.
6. `Entity<BottomPane>` — the Git log.

Callback injection into the editor follows at `:1172-1224`
(`TabMenuHostActions { reveal, open_terminal }`, both `Arc<dyn Fn(..)>`). `reveal`
shells out to `explorer.exe /select,<path>` → `open -R` → `xdg-open` with **runtime
probing** rather than `#[cfg]`, so neither `editor` nor `explorer` needs a platform layer.
Ordering constraint documented at `:1175-1178`: this must run *after* the terminal exists,
because `open_terminal` closes over `terminal.downgrade()`.

## Pane state machines

### Bottom pane — visibility is separate from kind

`BottomPaneKind { Terminal, Git, Run, Diagnostics }` (`:520-529`) with **hand-written
stable string ids** rather than a `serde` derive, so renaming a Rust variant cannot
silently change on-disk values (`:532-545`). `bottom_kind_from_id` returns `None` for an
unknown id rather than defaulting to Terminal (`:562-574`).

Visibility is deliberately a separate flag so collapsing preserves the last tab
(`:510-512`). One extra wrinkle: the Git tool window's own header has a Hide button that
calls `BottomPane::set_visible(false)`, so the shell ORs in `!bottom_git.visible()` for
`Git` — otherwise you get an empty frame (`:3799-3802`).
`TerminalPaneEvent::LastTabClosed` auto-collapses the bottom pane (`:1398-1407`).

### Left side pane — two states, IDEA semantics

`left_sidebar_visible` (whole rail + pane, `Ctrl+B`) and `left_pane_visible` (pane only,
toggled by clicking the *already-highlighted* top-group item) are independent
(`workspace.rs:820-834`, `:1835-1846`, `:1926-1936`). Content is a two-views-one-slot
switch; both entities are constructed unconditionally and stay resident, so switching does
not lose the other's scroll and expansion state (`:3772-3780`).

### Right tool window — two independent states

`right_view` and `right_visible`, matching the source product's
`isRightSidebarVisible` + `activeRightSidebarView`. The transition is a **pure function**
`resolve_click(clicked, current, visible) -> (view, visible)` (`:219`) so it is unit
tested.

### Activity bar selection contract

`activity_bar` takes `is_active: impl Fn(usize) -> bool` — a **per-item predicate**, not a
single `Option<usize>`, because the left rail's top and bottom groups can be lit
simultaneously and the right rail is a third independent set (`activity_bar.rs:61-67`).
Predicates are snapshotted into `Vec<bool>` before being handed to `'static` closures
(`:3738-3746`).

## State and data flow

There is no per-feature `*Store` in this crate. The pattern is:

- State lives in the view's own `Entity<T>`.
- Cross-feature reads go through a `Global`-held handle (settings) or `WeakEntity`
  injection (notify store).
- `cx.observe` / `cx.subscribe_in` results are **stored in a struct field** because
  GPUI's `Subscription` is RAII — dropping it unsubscribes (`:917-930, 1058-1059`).
- `Task`s must be held too (`session_save_task: Option<Task<()>>` at `:1050-1055`).
- Region render functions are **stateless**; they take data plus callbacks.

Settings → child-pane value forwarding (`:1328-1392`) is the canonical example: the child
crates do not know the settings crate, so the shell reads the store and pushes values in
— `EditorPane::set_tab_size` / `set_auto_completion`, `TerminalPane::set_default_shell` /
`set_font_size`, `ChangesView::set_confirm_before_discard`, plus
`register_java_toolchain`. An `S1_SETTINGS wiring=workbench …` line at `:1342-1349` is
the machine-checkable evidence the file was read back.

Cross-crate enums are mapped with a **local trait** `IntoGitIdentity` because
`impl From<External> for External` would violate the orphan rule (`:284-323`).

Because this crate has no `TestAppContext` / `#[gpui::test]`, every decision that can be
made pure *is* made pure and unit-tested (`workspace.rs:4625-5100`):
`resolve_project_open_destination`, `left_activity_index`, `visible_commands`,
`command_action`, `is_activity_active`, `RightScanState::request`, `right_scan_should_notify`,
`clamped_drag_width`, `bottom_kind_from_id`, `java_toolchain_override_from`,
`maven_settings_override_from`, `terminal_font_size_override`.

## Multi-project and project switching

**Only one project at a time.** `projects: Vec<ProjectTab>` is built as
`vec![ProjectTab::new(name)]` (`:1476`) — always exactly one tab — and the strip is hidden
entirely when `projects.len() <= 1` (`:3881-3890`). `ProjectTab` carries only a `name`;
the path of record is the single `ShellWorkspace.root` (`:795`). Closing a tab is
explicitly not implemented (`:3615-3616`).

"Open in this window" therefore means **rebuild the whole shell**, not reset widgets
(`:3010-3083`). Sequence: `flush_session(true, cx)` first, then
`window.replace_root(cx, ..)` with a fresh `ShellWorkspace` + `Root`, then — only after
success — record in recents and write the preference keys. The release chain is a
cascading drop with no explicit shutdown; the only anchor diagnostic is
`impl Drop for ShellWorkspace { eprintln!("S1_WORKSPACE_DROP root=…") }` (`:3458-3462`).

Two hard evidence points for why a full rebuild is required: `EditorPane::prepare_java`
is an idempotent early-return, so an editor pane can never switch JDTLS workspaces; and
the root is baked into 6 entities plus 1 cross-crate hook with no `set_root` entry points.

**Multi-window is not implemented and not faked.** `OpenDestination::NewWindow`
short-circuits to `report_new_window_not_wired` (`:2920-2942`), logging
`S1_OPEN_PROJECT destination=new-window state=not_wired missing=window_handle_routing`.
The stated reason (`:2931-2936`): `menu_bar`, `project_menu`, `command_palette` and the
Git-identity host are all **process-wide `thread_local` singletons**; a second shell would
overwrite the first's handles. Exactly two call sites construct a `ShellWorkspace`:
`app/src/main.rs:1030` and `workspace.rs:3042` (stated at `:4425-4428`).

## Editor tabs and session persistence

Owned entirely by `EditorPane` (see [Editor](editor.md)). The *timing* is the shell's
job: `SessionTracker` (`session.rs:77-100`) with `SESSION_SAVE_DEBOUNCE_MS = 300` and
three flush guarantees — debounced 300 ms on content change, synchronous flush before
project switch, synchronous flush on `on_app_quit` (`:13-19`). It reuses
`lithe_gpui_settings::DebounceState`, a pure state machine, so tests never sleep. A held
`Task` slot plus a generation counter together guarantee "many clicks, one write"
(`:1764-1777`).

`restore_session` runs **last** in `new()`, after `--right-view`/`--left-view`, so explicit
CLI intent beats the restored session (`:1580-1584`).

## Workspace identity

`prepare_workspace_config` (`:4455-4510`) runs on a background executor:
`workspace_config::resolve_project_id` creates or reads `.lithe/project.json` (stable
UUID v4) and the two local toolchain files. It also pushes the overlay's "is it
Git-tracked" flag to the settings store, so the UI can distinguish "team settings" from
"this project's settings".

On failure it sets `workspace_config_error` → a **persistent, manually dismissible red
banner** (`:3321-3354`), not a toast: double-click users never see stderr, and a hidden
failure is a silent failure (`:4448-4454`).

## Notable engineering constraints

**Edition-2024 RPIT hazard** (`lib.rs:110-121`): edition 2024 return-position `impl
Trait` captures *all* in-scope lifetimes, so `&mut Window` / `&mut App` parameters get
captured into the returned `impl IntoElement`, producing E0499/E0502 (12 occurrences on
first build). Rule: every region render function takes `&Window` / `&App`.

**Reuse versus rebuild.** gpui-kit components are reused where they fit (`Dialog` +
`Command` for both the command palette and the branch popup; `PopupMenu` for menu items)
and the chrome is hand-drawn where the component hard-codes incompatible geometry:
`AppMenuBar` hard-codes 24 px items and `NativeMenu` on Windows never calls `SetMenu`
(`menu_bar.rs:31-44`); `component::TitleBar` unconditionally draws its own 34 px window
controls (`title_bar.rs:56-68`); the component's status bar has two shrinking regions
(`status_bar.rs:22-36`); `ListDelegate` requires uniform row heights so the notification
panel cannot use `List` (`notifications.rs:16-23`).

**Edition 2024 / gpui-kit version trap**: `app/Cargo.toml:26-29` warns that the
`versions/main` docs advertise `gpui_kit::open_window(..)`, which does not exist in 0.6.6.
The correct call is `cx.open_window`.

## Known gaps

- `workspace.rs:1023` references a non-existent `ShellWorkspace::clear_notifications_on_root_swap`.
- The file diagram says the left pane is 480 wide; the code says `SIDEBAR_WIDTH_SPEC = 320`
  with a comment claiming 380, and the value is now runtime-draggable — three numbers for
  one value.
- Duplicated doc-comment blocks around `render`, the removed collapse-sidebar button and
  `footer_left` read like an unresolved merge.
- `gpui/README.md:70-86` omits `java` and `notify` from both the diagram and the crate
  table.
- The crate has **no test coverage for anything requiring a live `App`**; only the pure
  halves are tested.

## Sources

- `rust/gpui/crates/workbench/src/` (all files)
- `rust/gpui/crates/workbench/Cargo.toml`
- `rust/gpui/crates/editor/src/editor_view.rs`
- `rust/gpui/crates/settings/src/store.rs`
- `rust/gpui/crates/notify/src/lib.rs`
- `rust/gpui/crates/app/src/main.rs`
- `rust/gpui/README.md`, `rust/gpui/PLAN.md`
- `.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`
- `.agents/notes/implemented/feature/2026-09-27-session-state-persistence.md`
