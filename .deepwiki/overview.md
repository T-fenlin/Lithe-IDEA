# Lithe — Overview

Lithe is a **low-memory IntelliJ IDEA alternative for Java/Spring development**, written
as a pure Rust repository. Its selling point is a ~300–400 MB baseline resident set after
opening a project, versus the multi-gigabyte footprint of a full IDE. Language servers,
terminals, build tools, debuggers and database helpers all start on demand.

Repository status (read this before assuming a feature works): the Swift/macOS and
React/Tauri/Windows frontends were **removed**. What remains is a single GPUI Kit host
under `rust/lithe-db-gpui/`, still under active development with **no release pipeline**. The
installers on GitHub Releases were built from the deleted source and are kept for
reference only (`README.md:87-93`).

## Tech stack

| Layer | Choice | Where |
| --- | --- | --- |
| UI framework | GPUI Kit `0.6.6` (+ `gpui-pre 0.3.6`, `lsp-types 0.97`) | `rust/Cargo.lock:2800-2803` |
| Language | Rust 2021 (core crates) / Rust 2024 (gpui crates) | `rust/Cargo.toml:10-13` |
| i18n | `rust-i18n 4.2`, YAML catalogues | `rust/lithe-db-gpui/crates/shared/locales/` |
| Assets | `rust-embed 8` (`LitheAssets`) | `rust/lithe-db-gpui/crates/app/src/assets.rs:110-122` |
| Language service | Eclipse JDT LS 1.61.0 (manifest-pinned, downloaded at build time) | `third_party/jdtls/manifest.json` |
| Runtime JDK | Temurin 21 (minimum major 21 enforced in code) | `rust/lithe-db-gpui/crates/java/src/jdtls.rs:80` |

## Repo map

```
Lithe-IDEA/
├─ rust/                    the only Rust workspace (rust/Cargo.toml:15-23)
│  ├─ lithe-core/           deterministic command surface, no UI, no platform
│  ├─ lithe-git-host/       native Git process + AskPass adapter
│  ├─ lithe-db-sidecar/     one-shot JSON database subprocess  (not wired to gpui)
│  ├─ lithe-db-mcp/         MCP stdio server over the sidecar   (not wired to gpui)
│  └─ lithe-db-gpui/crates/          10-crate GPUI host (app, workbench, editor, explorer,
│                           git, terminal, java, settings, notify, shared)
├─ shared/                  contracts (prose + JSON schema) and cross-host fixtures
├─ .agents/                 skills + Chinese architecture-decision notes
├─ scripts/                 validation gates, packaging, release automation
├─ .github/workflows/       CI lanes (see tooling.md — most Rust has no lane)
├─ third_party/             upstream manifests only, no vendored code
├─ infra/                   docker compose for database validation
├─ docs/                    release notes, screenshots, one JSON schema
└─ rust/lithe-db-gpui/               themes, assets, UI maps, PLAN.md, research notes
```

## Build and run

The workspace root is `rust/Cargo.toml`. `rust/lithe-db-gpui/Cargo.toml` does not exist — the two
trees were merged after the frontends were deleted (`rust/Cargo.toml:1-13`).

```bash
cargo build --bin Lithe
./rust/target/debug/Lithe <workspace-root>     # positional arg is optional
```

`--theme <id|name>` and `--locale <tag>` override settings for one launch **without
writing them back**; `LITHE_GPUI_SETTINGS_FILE` redirects `settings.json` *and*
`recent-projects.json` to a temp path (`rust/lithe-db-gpui/crates/settings/src/paths.rs:81-100`).

## Where to look

| Question | Page |
| --- | --- |
| How do the crates fit together? What are the boundary rules? | [Architecture](architecture.md) |
| What commands does Core expose, and how are they dispatched? | [Rust Core](rust-core.md) |
| How does the app boot, and why is the order fixed? | [GPUI App Shell](lithe-db-gpui/app.md) |
| How is the window laid out? | [Workbench](lithe-db-gpui/workbench.md) |
| How does text editing work? | [Editor](lithe-db-gpui/editor.md) |
| How does the host call Core, and what is shared? | [Host Shared](lithe-db-gpui/shared.md) |
| How does JDT LS get discovered, started, and stopped? | [Language Service](lithe-db-gpui/java.md), [Session & Events](lithe-db-gpui/java/session.md) |
| Explorer / Git / Terminal panes | [Feature Panes](lithe-db-gpui/features.md) |
| Settings, themes, notification centre | [Settings](lithe-db-gpui/settings.md), [Themes](lithe-db-gpui/settings/themes.md), [Notification Centre](lithe-db-gpui/notify.md) |
| What is a compatibility surface? | [Contracts & Fixtures](contracts.md) |
| What must pass before handoff? | [Tooling & Validation](tooling.md) |
| Why is it built this way? | [Decisions & Notes](decisions.md) |
| Git, database crates, upstream pinning | [Supporting Crates](supporting-crates.md) |

## Reading conventions in this repository

- **Comments and module docs are in Chinese for first-party code**; English is required
  only inside `rust/lithe-core/`, which enforces it via a script
  (`.agents/skills/develop-lithe/SKILL.md:96-124`).
- **`S1_*` diagnostic lines are a deliberate, machine-greppable surface.** Nearly every
  non-trivial transition prints one. `println!` to stdout when ordering does not matter;
  `eprintln!` to stderr when it does (stdout is block-buffered when redirected and can
  lose lines while the process is alive — `rust/lithe-db-gpui/crates/app/src/main.rs:926-931`).
- **"Not wired" is a real, named state.** Unimplemented surfaces are declared with
  `MenuItem::NotWired { .. }`, a `NotWired` flag, an empty-state page, or an explicit
  "not implemented" register at the bottom of a crate's `lib.rs` — never a dead control.

## Sources

- `README.md`
- `rust/Cargo.toml`
- `AGENTS.md`
- `.agents/skills/develop-lithe/SKILL.md`
- `rust/lithe-db-gpui/crates/app/src/main.rs`
- `rust/lithe-db-gpui/crates/app/src/assets.rs`
- `rust/lithe-db-gpui/crates/shared/locales/`
- `third_party/jdtls/manifest.json`
