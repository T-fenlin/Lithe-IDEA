# Settings

`rust/lithe-gpui/crates/settings/` owns persistence, theming and the settings dialog. It depends
on `gpui-kit`, `shared`, `serde`, `serde_json`, the third-party `notify 7` filesystem
watcher and `async-channel 2`. It depends **only downward** — never on `workbench` or
`app` (`lib.rs:5-7`).

The companion crate `lithe-gpui-notify` (the notification centre) has its own page:
[Notification Centre](notify.md). They are split out because `settings` cannot see
`git` or `java`; the shell wires them together.

| File | Responsibility |
| --- | --- |
| `schema.rs` | The 24-key `Settings` model, ranges, whitelists, `ThemeIndex` |
| `paths.rs` | The only place allowed `#[cfg(target_os)]`; three-platform table + env overrides |
| `persistence.rs` | Pure read/write: load, tolerant parse, atomic save, `DebounceState` |
| `watch.rs` | Parent-directory watcher for external edits |
| `theme.rs` | Bundled theme seeding, `ThemeRegistry` wiring, font families → [child](settings/themes.md) |
| `workspace.rs` | The 9-key appearance overlay and the **single** merge implementation |
| `store.rs` | `SettingsStore` — the only module that needs `App` |
| `row.rs` | Settings row/group render parts |
| `identity.rs` | The `GitIdentityHost` hook the shell fills in |
| `project.rs` | JDK / Maven discovery for the Project page |
| `run.rs` | Run-configuration page data (`runConfig.generate`, not `resolve`) |
| `recent_projects.rs` | `recent-projects.json` |
| `dialog.rs` | The settings dialog, 11 categories, `Category::IMPLEMENTED` |
| `restart.rs` | `restart_application` |
| `lib.rs` | Module wiring + the **mandatory startup order** |

## The three-layer value model

`SettingsStore` keeps two copies and they must never be collapsed
(`store.rs:96-99, 1286-1288`):

| Field | Role |
| --- | --- |
| `settings` | The **effective** value: global + two workspace appearance overlays. Everything the UI and the side effects read |
| `global` | The `settings.json` layer — the **only** thing ever written**, and the sole source for non-appearance keys |
| `workspace_shared` / `workspace_local` | `.lithe/settings.json` (team) and `.lithe/settings.local.json` (personal) overlays |

Two real data-corruption paths are called out by name: writing the effective value to the
global file, and "change subtitle size" leaking a workspace override into the cross-project
default.

`SettingsStore` is a gpui `Entity`; `SettingsHandle(Entity<SettingsStore>)` is a `Global`
that only carries *where to find it*. `Entity` rather than a plain `Global` because
`Entity::notify()` is the broadcast mechanism and `cx.observe(&store, ..)` is all a
consumer needs (`store.rs:4-9`). Only `store(cx)` and `try_store(cx)` exist — the latter
because the theme watch callback can fire before or after `init` (`:149-153`).

## The settings model

24 keys, all serialized with the **Windows camelCase key names** verbatim, and a test
asserts every one of them (`schema.rs:846-883`).

| Key | Default | Effect timing |
| --- | --- | --- |
| `theme`, `syncSystemTheme`, `autoThemeLight`, `autoThemeDark` | `lithe-dark` / `false` / `lithe-light` / `lithe-dark` | immediate |
| `uiFontSize` | 13.0 (10–24, step 0.5) | immediate → rem base |
| `showStatusBar` | `true` | immediate |
| `displayLanguage` | `zh-CN` | **after restart** |
| `fontSize` | 14.0 (10–22) | immediate → `Theme::mono_font_size` |
| `fontFamily`, `monoFontFamily` | `""` (no override) | immediate |
| `tabSize` | 2 (∈ {2,4,8}) | immediate, via the shell → `EditorPane` |
| `terminalDefaultShellId` | `""` (∈ 4 ids) | **new sessions only** |
| `terminalFontSize` | 0.0 (unset) | immediate, via the shell |
| `confirmBeforeDiscard` | `true` | immediate, via the shell → `ChangesView` |
| `javaHomePath` | `""` | **next language-service start** |
| `mavenSettingsPath` | `""` | **next language-service start** |
| `mavenExecutablePath`, `mavenJavaHomePath`, `mavenLocalRepositoryPath` | `""` | **no consumer today** |
| `autoCompletion` | `true` | immediate → editor provider |
| `askWhereToOpenProjects`, `openFoldersInNewWindow` | `true` | project-open decision |

**Only keys with a real consumer are in the model** (`schema.rs:20-25`). `parameterHints`
and `semanticTokens` are absent from the LSP page because upstream `gpui-base` has no
`SignatureHelp` interface and Java has zero semantic-token implementation — drawing a
permanently-dead switch violates the "no fake controls" rule (`:517-527`).

Forwarding from the shell (`workbench/src/workspace.rs:1328-1392`): the child crates do
not know the settings crate, so the shell reads the store and pushes values in —
`EditorPane::set_tab_size` / `set_auto_completion`, `TerminalPane::set_default_shell` /
`set_font_size`, `ChangesView::set_confirm_before_discard`, and
`register_java_toolchain` for the two injected toolchain keys.

## Overlay layering

```
built-in defaults  <  global settings.json  <  .lithe/settings.json  <  .lithe/settings.local.json
```

Only the **9 appearance keys** are overridable (`workspace.rs:58-127`): `theme`,
`syncSystemTheme`, `autoThemeLight`, `autoThemeDark`, `uiFontSize`, `fontSize`,
`fontFamily`, `monoFontFamily`, `terminalFontSize`. Language, terminal shell, tab size,
toolchain and Git keys are global-only; if they appear in a workspace file they are
preserved verbatim as unknown keys but do not change the effective value.

`AppearanceSource` has three workspace states plus `Global` (`:348-378`) — `Global`,
`ProjectShared { tracked: true }` → "team settings", `ProjectShared { tracked: false }` →
"this project's settings", `ProjectLocal` → "your personal override". Each has its own
locale key because the three must be distinguishable.

Appearance changes go to the workspace's **personal** layer when a workspace is open: "a
user's own change must not silently enter a team file" (`:718`). The shared layer is only
written by an explicit (not-yet-landed) "share this project's config" action.
`revert_appearance_to_global` clears **both** layers — clearing only the local layer would
drop the value to the *shared* layer's value, not the user's global value, which is what
the button promises (`:929-1001`).

`resolve_effective(global, shared, local) -> Settings` is the **only** merge
implementation (`:393-404`). Unset keys are **removed from the document** rather than
written as `null` (`:509-525`) — `null` is not this document's shape, and a user editing
by hand could not tell "cleared" from "never written".

## Persistence

| Situation | Behaviour |
| --- | --- |
| File missing | All defaults; **the file is not created** |
| Not valid JSON / not an object | All defaults + diagnostic; never panics |
| One key has the wrong type | **That key** falls back + a `bad_key key=…` diagnostic; the others still work |
| Unknown key | Ignored for the value, but **preserved on write-back** |
| `version` newer than supported | Read normally, but **read-only** — never overwrite the user's file |

Writes: atomic (tmp + rename, which on Windows is `MoveFileEx(MOVEFILE_REPLACE_EXISTING)`),
unknown keys survive via `merge_document` → `preserve_unknown`, and the key table is
**derived** from `Settings::default().serialize()` (`persistence.rs:31-36, 158-164`).
Writing a file the toolchain layer owns is refused when that document declares a newer
version (`workspace_config/toolchain.rs:406-434`).

Debounce is a **pure state machine**, `DebounceState { revision, flushed }`
(`:205-239`), so tests never sleep. Three flush paths: normal 300 ms debounce, immediate
(`restore_defaults`, `set_display_language`), and `cx.on_app_quit` plus `flush_pending` on
dialog close. Missing the quit hook loses changes made inside the debounce window.

`commit_global` / `commit_appearance` are idempotent **by semantics, not optimisation**
(`:695-712, 723-769`) — an unchanged value returns early, which also suppresses a spurious
disk write from number inputs that fire `Change` at creation time.

## Watching for external edits

`watch.rs` watches the **parent directory non-recursively**, filtered by file name,
because our own "tmp + rename" write replaces the file and would kill a file-level watch
(`:5-8`). `async_channel::bounded(16)` with `try_send` — dropping a new event is safe
because reload is idempotent. **No self-excitation:** `reload_from_disk` returns early when
the file content equals memory. **External edits win** over a pending debounced change.
A deleted file does not reset memory. The config directory is created if missing, because
the watcher must work on first external edit even though *reading* must not create the
file.

## The settings dialog

One binding in this crate: `ctrl-,` → `OpenSettings`, registered as an **`App::on_action`
global** rather than on an element (`dialog.rs:91-108`). The reason is exact: gpui
dispatches keystrokes with no focused element only via the window root node, and
`ShellWorkspace` is a **child** of `Root`, so an element-level handler would leave a
"Ctrl+, does nothing before you click anything" hole.

`window.open_dialog` may only be called from an event callback or a task; calling it from
`render` panics (`:37-42`), which is why `--open-settings` uses `window.on_next_frame`.

11 categories: `General, Appearance, Project, Run, Editor, Keyboard, Terminal, Lsp, Git,
Logs, Updates`. Seven are `Category::IMPLEMENTED`. `Git` is *conditionally* implemented
(needs a registered host hook) so it cannot be expressed in a `const` array. The rest are
**explicit empty states** that print one prerequisite sentence and **no controls** —
showing a control that can never take effect is worse than showing nothing (`:232-234`).
`Appearance` is a gpui-side extra the Windows dialog does not have.

`content()` must use `.h_full()` or the page cannot scroll (`:187-192`).

Page data is loaded lazily and idempotently: `page_loads_on_open(Category)` makes
Project / Run / Lsp probe on open so that opening straight to one of them (from the
command palette or a probe) does not hang at "正在检测…". `GitPageState` carries a
generation counter so switching scope invalidates older identity loads.

Toolchain keys save to the **project-local layer** when a workspace is open, else to the
global file (`schema.rs:468-476`); both layers are merged by the one implementation in
`shared::workspace_config::ToolchainPaths::resolve`, so the page's "effective value" and
the language service's registration can never disagree.

`displayLanguage` does **not** hot-swap — the store writes immediately and the dialog calls
`restart_application` (`restart.rs`). The cost is explicit: unsaved editor content and live
terminal sessions die (`:13-14`).

## Path overrides

`LITHE_GPUI_SETTINGS_FILE` moves **both** `settings.json` and `recent-projects.json`
(`paths.rs:81-100`) — the verification scripts rely on this, because a recent-projects
implementation deriving its own path from `APPDATA` would read and write the real user
directory. `LITHE_GPUI_THEMES_DIR` does the same for the themes directory.
`repository_themes_dir()` is a **build-tree** path and must never be used at runtime
(`:130-139`); the old implementation read it at runtime and silently broke in install
packages.

Env-mutating tests need a lock and a restore guard (`paths.rs:186-231`) because `set_var`
is process-global and `cargo test` parallelises within one binary.

## The startup contract

```rust
lithe_gpui_settings::load();                        // pure file read, no App needed
gpui_kit::component::set_locale(..);                // MUST precede gpui_kit::init
gpui_kit::init(cx);
lithe_gpui_settings::init_store(cx, Init{..});      // Global + font size (NOT the theme)
lithe_gpui_settings::watch_lithe_themes(cx);        // seed + watch; callback applies the theme
lithe_gpui_settings::install_actions(cx);           // Ctrl+,
```

`init_store` also installs the watcher, the quit flush, both font sizes and the rem-base
invariant, so callers cannot forget any of them (`:225-243`).

**`init_store` does not apply the theme**, because `ThemeRegistry::watch_dir` loads
**asynchronously** — see the [themes child page](settings/themes.md).

`init_store` also deliberately does not run `normalize_with_themes`, for the same reason;
that check lives in `apply_theme_for` (`:1183-1200`).

## Sources

- `rust/lithe-gpui/crates/settings/src/` (all files)
- `rust/lithe-gpui/crates/settings/Cargo.toml`
- `rust/lithe-gpui/crates/shared/src/document.rs`, `workspace_config/toolchain.rs`
- `rust/lithe-gpui/crates/git/src/identity.rs`
- `rust/lithe-gpui/crates/java/src/toolchain.rs`
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-gpui/themes/`
- `shared/contracts/application-boundary.md`
- `.agents/notes/implemented/feature/2026-09-27-appearance-workspace-overlay.md`
- `.agents/notes/proposed/architecture/2026-09-26-workspace-configuration-layers.md`
