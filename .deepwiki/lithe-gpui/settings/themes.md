# Themes

`rust/lithe-gpui/crates/settings/src/theme.rs` plus the theme directory in
`rust/lithe-gpui/themes/`. Seven built-in themes ship with the binary: gruvbox, jetbrains,
lithe-dark, lithe-light, nord, one, vscode.

## Seed, don't watch

The decisive constraint: gpui-kit's `ThemeRegistry` **can watch only one directory** —
`watch_dir` assigns `self.themes_dir` and `reload()` reads only that
(`theme.rs:4-9`). Rejected alternatives are documented at `:19-29`; the accepted design is
seeding:

```
startup -> write the 7 built-in themes (include_str!'d into the binary) to <config>/themes/
        -> existing files are NOT touched, not a byte
        -> watch_dir(<config>/themes/) loads and watches
```

The accepted cost: a shipped upgrade does **not** refresh a seeded theme — the user
deletes the file and restarts. Buying "user edits are never overwritten" is judged more
important (`theme.rs:26-29`).

`seed_bundled_themes_into` (`:132-162`) uses tmp+rename per file. A plain `fs::write` could
leave a **truncated** theme that "already exists" next launch and is therefore permanently
corrupt (`:117-131`). It also deliberately does **not** re-seed a file that exists but
fails to parse — that would silently replace a hand-broken theme (`:129-131`).

`theme_index()` (`:259-287`) lists the **user directory first, then bundled**, so a
user-renamed seeded theme still resolves; it is cached on the user directory's mtime, with
the known edge case documented at `:256-258`.

Adding a file to `rust/lithe-gpui/themes/` **requires** registering it in `BUNDLED_THEMES`
(`theme.rs:68-70`) or a test fails.

## Persist by id, apply by display name

Persistence stores `themes[].id` — a field Lithe added to the theme files and upstream
silently ignores — while the registry indexes by `themes[].name`. That asymmetry is why
`ThemeIndex::from_documents` parses the raw JSON itself (`schema.rs:174-186`,
`theme.rs:35-40`).

`ThemeIndex::canonicalize` accepts **both** forms, exact first then case-insensitive, so
pre-upgrade files containing `"Lithe Dark"` keep working without migration
(`schema.rs:253-276`).

## `apply_theme_by_name` — a 5-step order that must not be reordered

`theme.rs:446-465`:

```
1. Theme::change(mode)                  # ThemeMode must be set BEFORE apply_config
2. stamp_builtin_highlight_if_absent
3. apply_config
4. sync_base
5. refresh_windows
```

Steps 1 and 2 are the two documented traps.

**Trap 1** — the file root is `ThemeSet`, not `ThemeConfig`, and a parse failure is
silently ignored (`theme.rs:42-49`).

**Trap 2** — `stamp_builtin_highlight_if_absent` (`:486-513`) fixes a real bug: `Lithe
Dark` (no `highlight` block) → `Gruvbox Light` (has one) → back to `Lithe Dark` would
leave **light syntax colours on a dark background**, because `apply_config` replaces
`highlight_theme` wholesale and `Theme::change` replays the slot. The known uncovered spot
is documented at `:482-485`.

## Why the theme is applied from the watch callback, not `init_store`

`ThemeRegistry::watch_dir` loads **asynchronously**. At `init_store` time the registry
contains only upstream's `Default Light` / `Default Dark`, so `Lithe Dark` is not there
yet. Validating the theme id at that point would misjudge `Lithe Dark` as missing and
silently substitute the user's choice — a bug that actually happened
(`store.rs:165-187`, `settings/src/lib.rs:44-46`).

So `watch_lithe_themes(cx)` (`:389-423`) seeds, starts `watch_dir`, and the **load
callback** applies "whatever should be in effect now" — it asks the store rather than
remembering a name. The visible consequence is that `S1_THEME applied=` appears exactly
once per launch rather than once per callback.

`init_store` also skips `normalize_with_themes` for the same reason; that check lives in
`apply_theme_for` (`store.rs:1183-1200`).

## Font size and the rem base

`apply_theme_font_size` writes `Theme.font_size`, which is the rem base that `Root::render`
re-applies every frame. `enforce_rem_base` (`store.rs:1230-1238`) is a **global `Theme`
observer** that repairs it whenever the registry's own observer re-runs `Theme::change` —
which would otherwise write the theme file's `font.size: 16.0` back. That is why
`init_store` installs it: so callers cannot forget (`:235-243`).

`apply_editor_font_size` writes `Theme::mono_font_size` separately, which is why the
terminal keeps its own `font_size` field rather than reusing this one (see
[Feature Panes](../features.md)).

## Font families are validated before being written

`theme.rs:549-584`. GPUI **panics on first layout** of a line when a family cannot be
loaded, and these are user-typed strings. So an unknown family is rejected with
`S1_THEME font_family_rejected key=… reason=not_installed` **while the user's value is still
persisted** — maybe the font simply is not installed on this machine. `.SystemUIFont` is
always allowed (`:547, 599-608`). `installed_font_names` is process-cached (`:590-593`).

## Theme directory resolution

`themes_dir()` follows the settings file's parent unless `LITHE_GPUI_THEMES_DIR` is set
(`paths.rs:120-128`), so one variable moves both into a temp directory.
`repository_themes_dir()` is a **build-tree** path used only for `include_str!` seeding and
tests — the old implementation read it at runtime and silently broke in install packages
(`:130-139`).

## Sources

- `rust/lithe-gpui/crates/settings/src/theme.rs`, `store.rs`, `schema.rs`, `paths.rs`, `lib.rs`
- `rust/lithe-gpui/themes/` (7 JSON themes + `README.md` + `themes/legacy-builtin/`)
- `rust/lithe-gpui/crates/app/src/main.rs` (steps 6–9 of the startup order)
- `rust/lithe-gpui/crates/terminal/src/terminal_view.rs` (why the terminal has its own font size)
- `.agents/notes/implemented/feature/2026-09-27-font-family-and-terminal-font-size.md`
- `.agents/notes/implemented/architecture/2026-09-27-config-document-semantics-and-workspace-config.md`
