# Architecture

Lithe is split into a **deterministic command surface** (`rust/lithe-core`) and a
**single GPUI Kit host** (`gpui/crates/*`). Everything interesting in the design
follows from keeping those two apart.

## Crate graph

```
                    ┌──────────────────────────────┐
                    │  lithe-gpui-app  (bin Lithe) │  composition root
                    │  CLI, window bounds, startup │
                    └───────────┬──────────────────┘
                                │ app → workbench, app → settings
             ┌──────────────────┼───────────────────────┐
             ▼                  ▼                       ▼
   ┌──────────────────┐  ┌──────────────┐      ┌──────────────┐
   │ lithe-gpui-      │  │ lithe-gpui-  │      │ lithe-gpui-  │
   │ workbench        │  │ settings     │      │ (java, notify │
   │ shell layout     │  │ model+theme  │      │  via shell)  │
   └────────┬─────────┘  └──────┬───────┘      └──────┬───────┘
            │                   │                     │
   ┌────────┴─────────┐         │                     │
   ▼        ▼   ▼     ▼         │                     │
 editor  explorer git terminal │                     │
   │        │     │     │       │                     │
   └────────┴─────┴─────┴───────┘                     │
                        │                             │
                        ▼                             ▼
              ┌─────────────────────────────────────────────┐
              │ lithe-gpui-shared                           │
              │ core_client · icons · i18n · workspace_config│
              └──────────────────────┬──────────────────────┘
                                     │ Rust crate, direct link
                                     ▼
                        ┌────────────────────────┐
                        │ lithe-core             │  no GPUI, no platform
                        │ protocol · project ·   │
                        │ lsp · git · execution  │
                        │ debug · ai · github    │
                        └───────────┬────────────┘
                                    ▼
                        ┌────────────────────────┐
                        │ lithe-git-host         │  processes, pipes, askpass
                        └────────────────────────┘
```

Declared direction: `app → workbench → {editor, explorer, git, terminal} → shared`, plus
`app → settings → shared` (`gpui/README.md:70-74`). Verified against
`rust/Cargo.lock:4427-4531` — the graph is a DAG, nothing depends upward.

**Edges beyond the documented diagram**, all deliberate and documented in-manifest:

| Edge | Why it is allowed |
| --- | --- |
| `workbench → java` | The settings page's `javaHomePath` must reach the language service, and `java` must not depend on `settings`. Single call site: `register_java_toolchain` (`workbench/src/workspace.rs:4582`). |
| `workbench → notify` | The notification-centre state layer lives with the shell. |
| `editor → notify` | `editor` is *below* `workbench`, so it may only use a narrow injected `WeakEntity` (`editor/src/editor_view.rs:349`). |
| `editor → java` | The editor wires JDTLS; all protocol code stays in `java`. |
| `shared → lithe-core` | In-process Rust link, not the C ABI (`shared/Cargo.toml:11-12`). |

Note `lithe-gpui-java` deliberately has **no `gpui-kit` dependency at all**
(`java/Cargo.toml:7-15`) — it is a pure data/request-construction layer, which is what
lets it own a blocking `std::thread` event pump.

## The three boundary rules

**1. Core is deterministic and UI-free.** `rust/lithe-core/` contains no GPUI and no
platform implementation. It is a command surface: request JSON in, response JSON out.
Modules that must be pure say so in their module doc — e.g. `editor/line_edit.rs:6-7`
"The transforms are pure", `diagnostics/mod.rs:6-8` "No filesystem or process access
happens here", `lsp/languages/jdt.rs:3-6` "The adapter is deliberately pure".

**2. Features never talk to Core or to each other directly.** Each gpui feature owns a
GPUI `Entity`-backed view, reaches Core only through `shared::core_client`, and
collaborates with siblings only by callback injection from the shell. Concretely,
`Explorer` receives an `on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App)>` that the
shell closes over the editor (`workbench/src/workspace.rs:1144-1158`) — so `explorer`
never names `editor`.

**3. Upstream owns language facts; Lithe owns orchestration.** Java symbols, source
roots, classpaths, entry points and test discovery come from JDT LS via Core's LSP
runtime. Core only validates and normalizes them. A verifier enforces this:
`node scripts/verify-java-semantic-ownership.mjs` fails if any of six retired local
entry-point/test-scanner identifiers reappear.

## The Core call pattern

`lithe_core::execute_json` is **synchronous**. The mandatory host shape is three steps,
documented in `shared/src/core_client.rs:27-28`:

```rust
// 1. flip a state so the UI can show loading
// 2. cx.spawn -> cx.background_spawn -> blocking Core call
// 3. this.update(cx, ..) to write the result on the UI thread
cx.spawn(async move |this, cx| {
    let result = cx.background_spawn(async move { load_snapshot(&root) }).await;
    let _ = this.update(cx, |this, cx| { /* write + cx.notify() */ });
}).detach();
```

Canonical instance: `explorer/src/explorer_view.rs:289-332`. Two corollaries appear
everywhere:

- **Bound everything.** `DEFAULT_TIMEOUT_MILLIS = 120_000`
  (`shared/src/core_client.rs:37`); Git deliberately uses 60 s
  (`git/src/model.rs:486-491`); result caps are explicit (`MAX_FILE_SIZE = 2 MiB`,
  search ≤ 10 000, symbols ≤ 50 — `core/src/project/files.rs:33,203,295`).
- **Guard every result against staleness.** Host-side this is a monotonic counter
  (`git/src/changes_view.rs:328-358`, `git/src/log_view.rs:233-251`,
  `editor/src/editor_view.rs:693-735`). Core-side it is `operationID` ownership
  (`core/src/project/document_lifecycle.rs:35,128`) and "stale preview" errors for
  reviewed Git mutations.

## Determinism rules

Stated once in `shared/contracts/application-boundary.md:11-19`:

- UTF-8 JSON at every process/language boundary.
- Workspace paths are **relative and `/`-separated**, on every host
  (`core/src/project/files.rs:715-722`; `invalid_relative_path` at
  `core/src/protocol/error.rs:80-87` rejects `..`, `:`, NUL and absolute input in both
  separator conventions so a Windows-shaped path cannot slip through elsewhere).
- Product-facing lines are 1-based; editor/LSP positions are 0-based line + UTF-16
  column; missing locations are `null`.
- **Lists have deterministic ordering** so fixtures can be compared byte-for-byte.
- Every async operation exposes `idle` / `loading` / `ready` / `failed`.
- Failures carry a stable `code` + user-facing `message`; platform detail goes in
  `details`.

Eleven stable error codes live in `core/src/protocol/error.rs:11-34`:
`invalid_request`, `workspace_not_found`, `permission_denied`, `not_supported`,
`runtime_missing`, `process_start_failed`, `process_failed`, `parse_failed`,
`cancelled`, `timed_out`, `unknown`. The host maps them to severity in
`notify/src/severity.rs:59-75` and deliberately degrades unknown codes to `warning`
rather than `error` — "don't lie about severity".

## Ownership table

| Path | Owns |
| --- | --- |
| `rust/lithe-core/` | Commands, models, validation, the JSON envelope, deterministic ordering |
| `rust/lithe-git-host/` | Git child processes, pipes, AskPass transport, bounded cleanup |
| `gpui/crates/app/` | Composition root, CLI, window sizing, startup order, assets |
| `gpui/crates/workbench/` | Shell chrome, pane layout, project tabs, root swap |
| `gpui/crates/editor/` | Buffers, tabs, navigation, diagnostics, completion, code actions |
| `gpui/crates/explorer/` | Project tree |
| `gpui/crates/git/` | Source control + commit log views, branch data, Git identity |
| `gpui/crates/terminal/` | Process sessions, ANSI stripping, scrollback |
| `gpui/crates/java/` | JDT LS discovery, workspace index cache, session envelope, event pump |
| `gpui/crates/settings/` | Settings model, persistence, themes, settings dialog |
| `gpui/crates/notify/` | Notification store and severity mapping |
| `gpui/crates/shared/` | Core client, icons, i18n, `.lithe/` workspace config |
| `shared/` | Contracts and fixtures — documentation, never compiled |
| `infra/` | Docker compose for database validation |
| `third_party/` | Upstream manifests (immutable revision + checksum), no vendored code |

The authoritative Chinese version of this table is
`.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md`.

## Data flow: opening a project

```
main.rs
  parse_options -> resolve_launch_root (recent-projects.json)
  lithe_gpui_settings::load()                       # pure file read
  gpui_kit::init -> Theme::change -> init_store -> watch_lithe_themes
  cx.spawn(open_window) -> ShellWorkspace::new(root, ..)
       ├─ Entity<notify::Store>                     # created first: editor gets a WeakEntity
       ├─ Entity<EditorPane> -> prepare_java(root)  # JDTLS plan, session starts on demand
       ├─ Entity<Explorer>        on_open = |p| editor.open(p)
       ├─ Entity<ChangesView>                            # no fetch; first read on activate
       ├─ Entity<TerminalPane>                          # no session until first visible
       ├─ Entity<BottomPane>   (git log)
       ├─ register_java_toolchain(settings.javaHomePath)
       └─ prepare_workspace_config -> .lithe/project.json (UUID v4)
  Root::new(workspace)                                 # MUST be the window's first layer
```

`ShellWorkspace::new` is called from exactly two places: the launch path
(`app/src/main.rs:1030`) and the project-switch root swap
(`workbench/src/workspace.rs:3042`).

## Sources

- `rust/Cargo.toml`, `rust/Cargo.lock`
- `gpui/README.md`
- `gpui/crates/*/Cargo.toml`
- `gpui/crates/shared/src/core_client.rs`
- `gpui/crates/app/src/main.rs`
- `gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-core/src/protocol/error.rs`, `contracts.rs`
- `rust/lithe-core/src/project/files.rs`
- `rust/lithe-core/src/project/document_lifecycle.rs`
- `gpui/crates/notify/src/severity.rs`
- `shared/contracts/application-boundary.md`
- `.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md`
