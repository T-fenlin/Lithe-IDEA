# Supporting Crates & Upstream Pinning

Four things that are not part of the Core↔host path but are load-bearing for the
product story: the Git process adapter, the database subprocess pair, and the `third_party`
manifest discipline that keeps upstream code and licences honest.

---

## `rust/lithe-git-host` — native Git process adapter

Edition 2021, library only, no `[[bin]]`. Deps: `libc` (unix), `windows-sys`
(Win32 Pipes / JobObjects / ToolHelp / Threading / Security), `rand`, `serde`,
`serde_json`.

Its whole reason to exist is to isolate everything platform-specific about running a Git
child process so `lithe-core` stays deterministic (`src/lib.rs:1-5`): *"Native Git process
ownership, incremental pipe reads, and bounded cleanup. This adapter does not construct
Git arguments or interpret Git output."*

### Public API

```rust
pub enum Stream { Stdout, Stderr }                    // :26-29
pub struct Outcome { stdout, stderr, status, failure } // :32-37
pub enum Failure { Start, Io, Cancelled, OutputLimit, Cleanup }   // :41-47
pub fn run(&mut Command, input, cancelled, started, output) -> Outcome   // :52-151
pub mod authentication;
pub mod configuration;
```

`run` does a nonblocking dual-stream drain with three bounds:
`MAX_CAPTURE_BYTES = 32 MiB`, `CLEANUP_TIMEOUT = 2 s`, `TERMINATION_GRACE = 500 ms`, and
checks cancellation every 10 ms (`:17-21, 96-142`).

### `authentication` — the AskPass transport

`src/authentication.rs:1-287`. A `Session` opens a loopback `TcpListener` and mints a
32-byte hex nonce token. `configure(&mut Command)` (`:81-94`) sets `GIT_ASKPASS`,
`SSH_ASKPASS`, `LITHE_GIT_ASKPASS_MODE=1`, `SSH_ASKPASS_REQUIRE=force`, `DISPLAY`,
`LITHE_GIT_ASKPASS_ADDRESS` and `LITHE_GIT_ASKPASS_TOKEN`. `poll()` accepts at most 4
concurrent prompts, caps frames at 16 KiB, and allows at most 3 attempts per prompt
(`:96-191`).

- `respond(request_id, Option<String>) -> bool` — single-use, 8 KiB cap, rejects
  NUL/CR/LF (`:195-207`).
- `helper_main(prompt) -> i32` — the askpass entry point (`:210-245`).
- `Confirmation` + `wait(cancelled)` — post-failure retry decisions (`:248-280`).

`configuration` provides `executable(selected)` PATH resolution (`:6-33`),
`configure(command, values)` appending `GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n` plus
`GIT_PAGER=cat`, `LC_ALL=C` and `GIT_TERMINAL_PROMPT=0` (`:37-50`), and `version()` with
an identity-keyed 16-entry cache (`:69-114`).

### Tests

`tests/process.rs` — 2 tests, cancel-before-exit and non-zero exit retaining both streams;
the test binary re-executes itself as the fixture (`:1-26`).
`tests/authentication.rs` — 2 tests: single-use response, wrong-token rejection, the 8 KiB
bound, replay rejection, drop-cancels, and retry-decision bounds. Plus one inline unit test
in `configuration.rs`. No dev-dependencies, no Git and no network required.

`scripts/verify-rust-core.sh:21-24` runs them under a 120 s per-suite timing harness with a
JUnit report.

### ⚠️ AskPass is unwired at the host

`Session::configure` sets `GIT_ASKPASS` to `std::env::current_exe()` (`authentication.rs:70-79`),
i.e. the `Lithe` binary. But `git_askpass_main` is reachable only through the **retired C
ABI** (`rust/lithe-core/src/runtime/ffi.rs:134-140`), and no file under `rust/lithe-gpui` references
`git_askpass_main` or `LITHE_GIT_ASKPASS_MODE`.

So interactive Git authentication has **no host-side consumer today**:
`shared/contracts/rust-core-api.md:463-467` still documents the AskPass contract, but the
implementation path is unreachable. This is a documented-contract vs. implementation gap,
not a crash — the flow simply never starts. Anyone wiring it up needs an early-helper
branch in `app/src/main.rs` before any GPUI initialisation.

---

## `rust/lithe-db-sidecar` — one-shot database subprocess

`publish = false`. Deps: `sqlx 0.8` (any/mysql/postgres/sqlite), `tiberius 0.12.3`
(SQL Server), `mongodb 3.2.5` (pinned), `redis 0.27` (rustls), `reqwest 0.12` (rustls),
`tokio`, `tokio-util`, `base64`, `csv`, `chrono`, `futures-util`, `hex`, `rust_decimal`,
`sha2`, `urlencoding`, `uuid`.

**Protocol: one JSON request on stdin, one JSON response on stdout, then exit**
(`src/main.rs:246-268`). Request `{id, method, params}` (`:26-33`), response
`{id, ok, result?, error?}` with `exit(1)` on non-`ok` (`:35-49`). `method == "capabilities"`
returns `protocolVersion: 1`, 8 database types (`mysql, mariadb, postgresql, sqlite,
sqlserver, mongodb, redis, nacos`) and 24 feature flags (`:272-277`).

Dispatch (`:287-299`): `redis` and `nacos` in-file; `sqlserver` → `sqlserver_adapter.rs`
(tiberius over `TcpStream`, INFORMATION_SCHEMA / sys catalog queries, rejects `values`
params at `:54-59`); `mongodb` → `mongodb_adapter.rs`; everything else → `sql_database_method`
(sqlx `AnyPool` with per-driver pool options at `:1557-1585`).

Shared behaviours: optional SSH tunnel (`:1679-1738`, spawns `ssh -N -L …` with
`BatchMode=yes`), and three write guards — `ensure_write_allowed`,
`ensure_mutations_allowed`, `ensure_transaction_allowed` (`:2801-2864`) which translate
`read_only` and `production_protection` + dangerous SQL into `read_only` /
`confirmation_required`.

**Tests: 11 in `main.rs:3995-4180` + 2 in `mongodb_adapter.rs:472-530`, all pure/unit** —
mutation SQL generation, identifier validation, dangerous-SQL classification, export
framing. `verify-rust-core.sh` does **not** cover this crate; only
`.github/workflows/ci-database.yml:107-109` does.

### ⚠️ Not wired to the gpui host

The **only** consumer of the sidecar is `lithe-db-mcp`. No gpui Rust code references it
(a grep for `database` in `rust/lithe-gpui/crates/**/*.rs` finds 6 unrelated hits). The database
menu item renders as `MenuItem::NotWired { label_key: "lithe.menu.databases", .. }`
(`lithe-gpui/crates/workbench/src/menu_bar.rs:1375-1379`), `database-icons/` is `#[exclude]`d
from the embedded assets (`lithe-gpui/crates/app/src/assets.rs:119`), and the
`dev.lithe.database` built-in module is `defaultState: "disabled"`
(`shared/fixtures/modules/built-in-v1.json:24-36`).

---

## `rust/lithe-db-mcp` — MCP stdio server

`publish = false`. Deps: `serde`, `serde_json`, `uuid` only — **no DB drivers**; it
delegates.

Newline-delimited JSON-RPC 2.0 over stdin/stdout (`src/main.rs:10-49`), with config from
`LITHE_DB_MCP_CONNECTIONS`, `LITHE_DB_MCP_POLICY`, `LITHE_DB_MCP_AUDIT_LOG` and
`LITHE_DB_MCP_RECOVERY_DIR` (default `temp_dir()/lithe-db-mcp-recovery`).

Methods: `initialize` (protocol `2024-11-05`, `serverInfo: lithe-db-mcp`), `tools/list`,
`tools/call` (`:64-77`). 13 tools (`:80-97`): `db_list_connections`, `db_list_tables`,
`db_describe_table`, `db_list_objects`, `db_plan_sql`, `db_query`, `db_explain`,
`db_diagnostics`, `db_backup`, `db_execute`, `db_transaction`, `db_schema_change`,
`db_restore`.

Safety model: write tools require a policy permission **and** `confirmed=true`; restore
references are canonicalized and must stay under the recovery directory (`:500-507`); every
mutation is appended to an optional audit log. 4 unit tests (`:600-660`) cover SQL
classification and policy scoping.

`sidecar_call` (`:517-553`) resolves `LITHE_DB_SIDECAR_EXECUTABLE`, else a sibling
`lithe-db-sidecar` next to `current_exe()`, and spawns **one child per request**.

## Database validation environment

`infra/docker/database-validation/` is the only `infra/` content:
`compose.yaml` (project `lithe-database-validation`; `mariadb:11.4` on 53307,
`mongodb:7.0` on 57019, `redis:7.4-alpine` on 6381 with `--requirepass`, plus a one-shot
`redis-seed` gated on `service_healthy`), `mariadb-init.sql`, `mongodb-init.js`,
`redis-seed.sh`. All services bind to `${LITHE_BIND_ADDRESS:-127.0.0.1}` — **never
`0.0.0.0`** — all have healthchecks, all use named volumes. Validated in CI by
`docker compose -f infra/docker/database-validation/compose.yaml config --quiet` and
exercised by `scripts/database-validation-smoke.sh`.

There is no `infra/` README; its ownership is stated only as one line in
`develop-lithe/SKILL.md:87`.

---

## `third_party/` — manifests, not code

`third_party/README.md:1-16` is the rule. Keep **only**: a manifest with the upstream
repo, an immutable revision, a download URL and a checksum; the applicable licence/notice
beside any retained artifact (or a pointer when the artifact lives in its owning resource
folder); and narrowly scoped patched source only when Lithe actually compiles it.

Explicitly forbidden: complete upstream repos, release binaries, doc sites, screenshots,
tests, or CI configs kept "for reference alone". Build-time downloads belong in ignored
artifact caches (`.artifacts/`); runtime assets belong in the packaging directory.

The directory currently holds **three manifests and no source**:

| Manifest | Contents |
| --- | --- |
| `jdtls/manifest.json` | JDT LS **1.61.0** milestone tarball + SHA-256; EPL-2.0 text + SHA; Lombok 1.18.46 jar + SHA + licence + SHA; `vscode-java-debug` 0.59.0 VSIX + SHA with inner plugin SHA and java-debug 0.53.2 licence + SHA; `vscode-java-test` 0.46.0 VSIX + SHA with plugin SHA, runner SHA and licence SHA; `minimumJavaVersion: 21` |
| `jdk/manifest.json` | Temurin **21.0.12.1+1**; four platform entries (`macos-aarch64`, `macos-x86_64`, `windows-aarch64` = `21.0.12+8`, `windows-x86_64`), each with URL, `sha256` and `jdkRoot` |
| `dbx/manifest.json` | `t8y2/dbx` @ `996ce42e…`, `usage: "reference-only"`, `buildDependency: false`, `runtimeDependency: false`, `packaged: false`; `retainedArtifacts` points at `rust/lithe-gpui/assets/database-icons/` with its `NOTICE.txt` and `LICENSE-APACHE-2.0.txt` |

The JDK and JDTLS manifests are consumed by `scripts/prepare-jdk.{sh,ps1}` and validated by
`scripts/verify-download-cache.mjs`, which is itself an `enable_all_validation` trigger in
the path classifier (`classify-ci-changes.sh:152-154`).

`BUNDLED_MANIFEST` is `include_str!`-ed into the binary
(`lithe-gpui/crates/java/src/jdtls.rs:34`) so version detection has a compiled-in fallback when
no `manifest.json` is present.

## `.lithe/` is per-project, not repo-root

The repository root has **no** `.lithe/`. That directory belongs to each *opened project*
and is created on open, unconditionally — a product decision that accepts a `.lithe/` even
in a non-Git directory (`lithe-gpui/crates/workbench/src/workspace.rs:1104-1108, 4439-4442`).
The document set is enumerated in `lithe-gpui/crates/shared/src/workspace_config/paths.rs:60-160`.

Root-level hidden entries that do exist: `.agents/`, `.deepwiki/`, `.github/`, `.gitignore`.
`.gitignore:1-11` covers `.idea/`, `rust/target/`, `.artifacts/`,
`shared/fixtures/projects/**/target/`, `design-qa-artifacts/`, `.reasonix/`,
`reasonix.toml`, `.codex/`.

## Sources

- `rust/lithe-git-host/` (all files, incl. `tests/`)
- `rust/lithe-core/src/runtime/ffi.rs`, `src/lib.rs`
- `rust/lithe-core/Cargo.toml`
- `rust/lithe-db-sidecar/src/`, `Cargo.toml`
- `rust/lithe-db-mcp/src/main.rs`, `Cargo.toml`
- `rust/lithe-gpui/crates/java/src/jdtls.rs`
- `rust/lithe-gpui/crates/app/src/assets.rs`
- `rust/lithe-gpui/crates/workbench/src/menu_bar.rs`
- `scripts/{prepare-jdk.sh,verify-download-cache.mjs,build-database-*.sh,database-*-smoke.sh}`
- `infra/docker/database-validation/`
- `third_party/README.md`, `third_party/*/manifest.json`
- `shared/fixtures/modules/built-in-v1.json`
- `shared/contracts/rust-core-api.md`
- `.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md`
