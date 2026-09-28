# GPUI App Shell

`rust/gpui/crates/app/` is the **composition root**. Five files, one binary target
(`Lithe`), no `lib.rs`, no `tests/`. Its whole mandate is stated at
`app/src/main.rs:1-6`: *"App Shell 只组合窗口与 Feature，不承载任何具体 Feature 逻辑"*.

| File | Responsibility |
| --- | --- |
| `main.rs` | CLI parsing, launch-root resolution, the 21-step startup order, window bounds, settings/theme wiring, and every verification probe driver (1 307 lines) |
| `assets.rs` | `LitheAssets`: a `rust_embed` table over `gpui/assets/**` + an `AssetSource` that falls back to gpui-kit's `AllAssets` (436 lines) |
| `build.rs` | Windows only: compiles `lithe.rc` to embed `icon.ico` as PE resource **ID 1** |
| `lithe.rc` | The 22-line `.rc`; documents why the ID must be 1 and why no MANIFEST may be added |

`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` (`:37`) — release is
a GUI subsystem, debug keeps the console so `S1_*` diagnostics stay visible.

## Startup order

There is exactly one legal order in GPUI Kit 0.6.6. Reordering fails silently or panics.
This is the single most useful thing to know about this crate.

| # | Step | Site | Why here |
| --- | --- | --- | --- |
| 0 | `parse_options()` → `resolve_launch_root(root)` | `:849-859` | Needs no `App`; must fail loudly before any state exists |
| 1 | `lithe_gpui_settings::load()` → `locale` | `:891-894` | The settings file is the *regular* source of UI language; `--locale` is only an override. Pure file read, no `App` needed |
| 2 | Resolve `--right-view` / `--left-view` ids | `:866-887` | Resolved *before* the window so a bad id errors without mutating startup state |
| 3 | `gpui_kit::application().with_assets(LitheAssets).run(..)` | `:896-921` | `with_assets` may be called **once** — a second call overwrites. Must be `AllAssets` (1 830 Lucide glyphs), not the default 101-glyph subset, because the source-control UI needs `git-*` glyphs |
| 4 | `set_locale(&locale)` | `:966` | Must precede `gpui_kit::init` |
| 5 | `gpui_kit::init(cx)` | `:967` | — |
| 6 | `Theme::change(Dark)` | `:972` | gpui-kit defaults to **light**; Lithe's product default is dark. Flipping explicitly means even the frames before the real theme lands look right |
| 7 | `Theme::set_scrollbar_mode(Always)` | `:975` | gpui-kit default fades the scrollbar; an IDE wants a permanent thin one |
| 8 | `init_store(cx, Init { loaded, theme_override })` | `:980-989` | Registers the settings `Entity` + `Global`. **Does not apply the theme** — see below |
| 9 | `watch_lithe_themes(cx)` | `:987` | Seeds 7 built-in themes into `<config>/themes/` and starts `ThemeRegistry::watch_dir`. **The load callback is where the theme is actually applied**, so `S1_THEME applied=` appears exactly once |
| 10 | `install_actions(cx)` | `:988` | `Ctrl+,` |
| 11 | `set_shell_startup(cx, ..)` | `:998-1003` | An `App::Global`, because project switching *rebuilds* `ShellWorkspace` inside `window.replace_root` where `main`'s locals cannot be captured. Consequence: the rebuilt shell re-applies `--right-view`/`--left-view` |
| 12 | `install_open_project_action(cx)` | `:1008` | `Ctrl+O`. `App::on_action` is **cumulative** and `ShellWorkspace::new` runs on every project switch, so registering there would stack handlers |
| 13 | `menu_bar::install_key_actions(cx)` | `:1016` | Six more bindings, same "only here" reason |
| 14 | `startup_window_bounds(cx)` | `:1018` | — |
| 15 | `cx.spawn(.. cx.open_window(..))` | `:1020-1027` | The window is opened **inside a spawned task**; the `run` closure returns immediately. No resident loop in `main` — a sleep loop starves the window-creation task (`:1245-1251`) |
| 16 | `ShellWorkspace::new(root, ..)` | `:1028-1037` | The composition root call |
| 17 | `store.attach_window(window, cx)` | `:1040-1042` | The system-appearance watcher needs a `&mut Window` |
| 18 | Probe drivers, each via `window.on_next_frame` | `:1043-1199` | Overlays may only open from an event callback or task. **Exception:** session probes run synchronously right after window creation, because frame callbacks never fire in an unattended launch (`session.rs:30-36`) |
| 19 | `cx.new(\|cx\| Root::new(workspace, window, cx))` | `:1201` | **`Root` must be the window's first layer** — it hosts dialogs, sheets and notifications. Opening any overlay before this panics |
| 20 | `--palette-keys` dispatch | `:1210-1238` | `dispatch_keystroke` synchronously calls `Window::draw`, which requires the root to be installed; starts after a 400 ms background timer |
| 21 | `return root_entity` | `:1239-1241` | The closure must return the root view |

### Why `init_store` does not apply the theme

`ThemeRegistry::watch_dir` loads **asynchronously**. At `init_store` time the registry
contains only upstream's `Default Light`/`Default Dark`, so validating the theme id at
that point would judge `Lithe Dark` missing and silently substitute — a bug that actually
happened (`settings/src/store.rs:165-187`, `settings/src/lib.rs:44-46`).

## CLI

Hand-rolled `parse_options()` over `std::env::args()`; no clap. `Options` at `:98-235`,
usage text at `:243-293`. Only three flags are product capability; the rest are
machine-verification probes, explicitly labelled as such (`:110-120`).

| Flag | Semantics |
| --- | --- |
| `[<workspace-root>]` | Optional. Omitted = "open the most recent existing project", else CWD — this is the double-click path |
| `--theme <id\|name>` | One-launch override, never written back. Accepts `themes[].id` or `themes[].name` |
| `--locale <tag>` | One-launch override, never written back. gpui-kit ships `en` / `zh-CN` / `zh-HK` |
| `--open-settings`, `--open-palette` | Deferred to `on_next_frame` |
| `--menu-probe <id> [action]`, `--menu-probe-delay`, `--menu-probe-open-ms` | Open a top-level menu; the `lithe.` prefix is the heuristic and a non-matching second arg is a hard error (the arg iterator is one-way) |
| `--theme-probe <index\|name>`, `--project-menu-probe`, `--branch-panel-probe` | — |
| `--right-view <id>`, `--left-view <id>` | `extensions`/`notifications`/`maven`/`spring`; `files`/`changes`/`search` |
| `--open-project-probe <dir>`, `--open-project-destination`, `--open-project-remember`, `--open-project-delay-ms` | Walk the "open folder" chain |
| `--appearance-revert <key>`, `--session-probe`, `--session-active`, `--session-hide-sidebar`, `--session-assert` | — |
| `--palette-keys <csv>` (repeatable) | Parsed with `gpui_kit::Keystroke::parse`; a parse failure exits |
| `--compact-menu-bar` | Menu bar in "top-left icon + floating capsule" form |

`-h` returns the usage as an `Err`, which `main` prints to stderr and exits **2**
(`:849-855`).

**Launch-root resolution is split in two for testability**: the pure precedence rule
`choose_launch_root(explicit, recent, fallback)` (`:703-711`, four unit tests at
`:1260-1307`) and the I/O wrapper `resolve_launch_root` (`:719-770`) which reads
`recent-projects.json`, marks missing entries, persists them, and falls back.
`fallback_directory()` never exits (`:790-815`).

## Window bounds

`startup_window_bounds` (`:69-88`): minimum 1024×680, fallback 1280×800. No display →
`Windowed`. Otherwise 94 % of `display.visible_bounds()` (taskbar already excluded),
clamped to the minimum and to the visible size, centred. Two rejected alternatives are
documented in the comment: copying a hard-coded macOS size, and `WindowBounds::Maximized`
/ `zoom_window()` — on this GPUI version those only set the *restore* size and do not
actually maximize.

## Assets

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../assets"]
#[exclude = "icon-themes/lithe/**"]  #[exclude = "icon-themes/pierre/**"]
#[exclude = "icon-themes/symbols/**"] #[exclude = "icon-themes/material/**"]
#[exclude = "fonts/**"]  #[exclude = "legacy-ide-icons/**"]
#[exclude = "gutter-icons/**"]  #[exclude = "database-icons/**"]
#[exclude = "feature-icons/**"]  #[exclude = "README.md"]
pub struct LitheAssets;                                        // assets.rs:110-122
```

Measured envelope: **269 files / 3 200 771 bytes (~3.05 MiB)** — `icons/**` 7 bitmaps,
`images/logo.png` 1, `ui-icons/**` 157 IntelliJ expui SVGs, `icon-themes/idea/**` 104.
Before narrowing it was 1 203 files / 8 567 891 B.

Every `#[exclude]` has a reason, and removing one without wiring the asset only grows the
binary:

| Excluded | Why |
| --- | --- |
| 4 icon-theme packages | 933 files / ~5.09 MiB, **unreachable**: the active file-icon theme id is a compile-time constant `ACTIVE_FILE_ICON_THEME = "idea"` and nothing enumerates the directory at runtime |
| `fonts/`, `gutter-icons/`, `database-icons/`, `feature-icons/`, `legacy-ide-icons/` | Archived from the removed frontends, not yet wired |
| `README.md` | Documentation, not an asset; editing it would drift the file/byte counts the regression test pins |

`#[folder]` must be an absolute string — the crate is at `gpui/crates/app` while the
assets are at `gpui/assets`, and rust-embed's `get()` does a naive `starts_with` boundary
check that a `..`-bearing path defeats (`assets.rs:95-100`).

The `AssetSource` impl (`:199-227`) tries the embed, then `AllAssets`, folding `Err` into
`Ok(None)` because `AllAssets::load` returns `Err` on a miss — using `?` would turn "glyph
not found" into a hard error.

Five regression tests guard the envelope, including
`embedded_range_stays_within_the_documented_envelope` (pins 269 / 157 / 104 / 7 / 1 and
the exact byte total) and `probe_paths_are_loadable_from_the_embedded_table`, which
asserts *presence in the embed table* and not just a successful `load` — because Lucide
happens to also contain `icons/settings.svg`, so a load-only assertion would be satisfied
by the fallback and prove nothing (`:403-418`).

## Sources

- `rust/gpui/crates/app/src/main.rs`
- `rust/gpui/crates/app/src/assets.rs`
- `rust/gpui/crates/app/build.rs`, `lithe.rc`, `Cargo.toml`
- `rust/gpui/crates/settings/src/lib.rs`, `store.rs`, `theme.rs`, `paths.rs`
- `rust/gpui/crates/workbench/src/workspace.rs`
- `rust/gpui/README.md`
