# Language Service (`lithe-gpui-java`)

`rust/lithe-gpui/crates/java/` owns **everything platform-specific** about Eclipse JDT LS. It
deliberately has **no `gpui-kit` dependency** (`Cargo.toml:7-15`) — it is a pure
data / request-construction layer, which is what lets it own a blocking `std::thread`
event pump.

The split is the whole point (`lib.rs:8-26`): Core owns the child process, stdin/stdout,
framing, JSON-RPC ids, document versions, pending deadlines, capabilities, diagnostics and
graceful/forced termination. This crate owns **zero LSP protocol code and zero Java
syntax**.

| Module | Responsibility | Detail |
| --- | --- | --- |
| `jdtls.rs` | Platform discovery of the JDT LS payload and the JDK to run it on | below |
| `workspace.rs` | Workspace index cache key, cache directory, expiry reclaim, Maven context | below |
| `toolchain.rs` | Host injection points for `javaHomePath` and `mavenSettingsPath` | below |
| `session.rs` | The synchronous envelope for the Core LSP commands | [session child](java/session.md) |
| `events.rs` | The event pump — the **only** consumer of `lsp.waitEvents` | [session child](java/session.md) |
| `service.rs` | The `JavaLanguageService` facade | [session child](java/session.md) |

## State

```
JavaLanguageService                     service.rs:187-194
  workspace_root: PathBuf
  state: Mutex<State>
      Idle              never tried, or policy said "not a Java workspace"
      Ready(Box<Session>)
      Failed(String)    failed once, never retried
      Closed            app exiting
  installation: Mutex<Option<(JdtlsInstallation, JavaRuntime)>>
```

It is held as `Option<Arc<JavaLanguageService>>` inside `EditorPane`
(`editor/src/editor_view.rs:374`), so all gpui interaction goes through `prepare_java`,
`shutdown`, and `EditorPane::drop`. **All `JavaLanguageService` methods block** — the
caller puts them in `cx.background_spawn`, because the `state` mutex is held across the
whole `sync_document` + `request` sequence and Core's session is stateful (document
versions, in-flight requests); two interleaved threads would desynchronise versions and
results.

## Discovery of the JDT LS payload

`jdtls::resolve(workspace_root)` (`jdtls.rs:104-153`):

1. `find_executable` (`:336-350`) — first `is_file()` wins, over:
   - **bundled roots** (`:356-374`): `current_exe().parent()/LanguageServers/jdtls`, then a
     **12-level upward walk** from the exe dir looking for `.artifacts/jdtls`. Using
     `current_exe` rather than a resource dir is what makes `cargo run` work.
   - every `PATH` directory × `["jdtls.bat", "jdtls.cmd", "jdtls.exe", "jdtls"]`
   - **external install roots** (`:377-409`): `JDTLS_HOME`, `LITHE_JDTLS_ROOT`,
     `<LOCALAPPDATA|ProgramFiles|ProgramFiles(x86)|XDG_DATA_HOME>` variants,
     `<USERPROFILE|HOME>/{.jdtls, scoop/...}`, and
     **`<workspace_root>/.lithe/toolchains/jdtls`**
2. `installation_roots(executable)` (`:256-278`) — if the parent is named `bin`, the root
   is *its* parent; the parent is also pushed; the `canonicalize`d variant is appended
   **after**.

   > Do not sort this list. `canonicalize` on Windows returns verbatim `\\?\D:\...`; `\`
   > (0x5C) sorts before `D` (0x44), so a sorted list would pick the verbatim variant and
   > every resource path would carry `\\?\` — the JVM then cannot find the launcher jar
   > (observed: `initialize` fails instantly with `Unable to add \\?\...lombok.jar`).
   > Documented at `:250-255`.
3. Per root, `resources_for_root` (`:166-188`) validates all-or-nothing:
   `plugins/org.eclipse.equinox.launcher_*.jar` (required, first by sorted filename),
   `java-debug/…debug.plugin-*.jar` (optional), `java-test/extensions.txt` **plus every
   listed file** (required — declaration order *is* the OSGi resolution order, and a
   listed-but-missing bundle is an **error** because partial loading breaks the whole
   extension), a per-platform `config_*` directory, and `lombok/lombok.jar`. Any entry
   containing `/`, `\` or `..` is rejected as path traversal (`:230-235`).
4. Version (`:282-333`): `manifest.json` → else the core plugin filename
   `org.eclipse.jdt.ls.core_1.61.0.<ts>.jar` taking the **first three dot-separated
   segments** → else the embedded `BUNDLED_MANIFEST`. It only affects the cache key, never
   usability.

## JDK resolution

`resolve_runtime` judgement order — **the enum order is the order** (`:512-548`):

1. `Override` — the settings page `javaHomePath` (the user's explicit choice)
2. `ExplicitEnv` — `LITHE_JDTLS_JAVA`
3. `Bundled` — `current_exe()/LanguageServers/jdk`, upward `.artifacts/jdk`
4. `JdtlsBundled` — `<jdtls_root>/jre`
5. `JavaHome` — `JAVA_HOME`
6. `Path` — each `PATH` directory
7. `InstalledRoot` — enumerated common install roots

> The override is deliberately ranked **above** `LITHE_JDTLS_JAVA`, pinned by a test
> (`:1013-1042`). The env var is documented as "only overrides derivation, not the regular
> entry point", while the settings value is a product-level user selection the settings
> page labels "已选择". Letting a verification switch override the user's choice would make
> the settings page "stored but not effective" a third time (`:498-508`).

`MINIMUM_JAVA_MAJOR = 21` (`:80`). `probe_java` runs `java -version`, concatenates
stdout+stderr (`-version` writes to stderr on modern JDKs), and `major_version` handles
`1.8.0_402 → 8`, `21.0.2 → 21`, `25 → 25`. A rejected candidate is recorded and the chain
continues — **an unusable override is not a startup failure** (`:630-631`).

Only the `Override` branch also accepts the extension-less `java`, because that is what
the settings page accepts (`java/src/service.rs:720-722`).

`S1_JAVA_TOOLCHAIN` is one line in four shapes, with `reason=` always last because it may
contain spaces (`:577-623`):

```
S1_JAVA_TOOLCHAIN override=-        state=absent  result=automatic java=… javaVersion=21.0.8
S1_JAVA_TOOLCHAIN override=<path>  state=used    result=used      java=… javaVersion=21.0.8
S1_JAVA_TOOLCHAIN override=<path>  state=invalid fallback=automatic result=automatic … reason=版本 1.8.0_221 低于 21
S1_JAVA_TOOLCHAIN override=<path>  state=invalid fallback=automatic result=failed reason=找不到 java.exe
```

### Injection chain

```
settings file javaHomePath  ─┐
                             ├─▶ project::resolve_overrides ──▶ set_java_toolchain_override
.lithe/run.local.json      ─┘   (shared::workspace_config::ToolchainPaths::resolve)
                                        │
                                        ▼
                          static OVERRIDE: Mutex<Option<…>>     toolchain.rs:79
                                        ▼
                          jdtls::resolve_runtime re-reads it      jdtls.rs:635
```

`Mutex`, not `thread_local!`: the reader is `start_locked`, which runs on a gpui
background executor thread, so a `thread_local!` would read `None` forever and reproduce
the exact "saved but not effective" bug this code was written to fix
(`toolchain.rs:20-28`).

**Effect timing: restart.** The slot is re-read on every `resolve_runtime`, but a session
is per-workspace and `EditorPane::prepare_java` is idempotent, so a settings change takes
effect at the *next language-service start*. Changing the JDK mid-session is refused
because the index would be out of sync with the JVM version (`toolchain.rs:53-59`).

Of the five toolchain keys, only two are injected: `javaHomePath` and `mavenSettingsPath`.
`mavenExecutablePath` / `mavenJavaHomePath` / `mavenLocalRepositoryPath` are deliberately
not, because nothing executes Maven on the gpui side.

## Workspace index cache

`workspace::plan(root, jdtls_version)` (`workspace.rs:62-79`):

1. `collect_fingerprint_inputs` (`:136-198`) — **platform-observed** inputs only: for each
   of `["pom.xml", "build.gradle", "build.gradle.kts"]` at the root, a
   `{path, modifiedUnixMilliseconds, sizeBytes}` record; plus the **names of direct child
   directories containing a `pom.xml`** (non-recursive — recursion is JDT's job).
2. `resolve_fingerprint` → **`java.jdtWorkspaceFingerprint`**. Normalisation, sorting, dedup
   and hashing are all Core's.
3. `resolve_workspace_key` → **`lsp.jdtWorkspaceKey`**. Validated as 64 lowercase hex
   because it becomes a directory name (`:388-393`).
4. `state_directory() = cacheDirectory/jdtls/<workspaceKey>` (`:54-58`).
   `state_directory_existed` is the direct evidence of "reopen reuses the index".
5. `cleanup_expired(cache_directory, active_workspace_key)` (`:260-385`) reclaims expired
   entries; a failure logs `S1_JAVA_CACHE_RECLAIM_FAILED` and **never blocks startup** —
   reclaim only saves disk.

Platform cache roots: `%LOCALAPPDATA%\Lithe\cache` / `~/Library/Caches/Lithe` /
`$XDG_CACHE_HOME/lithe` → `~/.cache/lithe`, overridable by `LITHE_GPUI_CACHE_DIR`
(`:90-123`).

The legacy "path-only key" path is **not** used: a fingerprint is always sent, otherwise a
workspace whose structure changed would silently reuse a stale project model
(`:12-13`).

`maven_context` (`:432-457`) builds `{version, reactorPath, profiles, skipTests}` plus
`settingsPath` (override slot first, then the scanned one).
`localRepositoryPath` / `mavenExecutablePath` / `javaHomePath` are deliberately **left
out** — nothing reads them here, and passing unread values is worse than omitting them. A
failed scan returns `None` and does not block startup.

## Sources

- `rust/lithe-gpui/crates/java/src/jdtls.rs`, `workspace.rs`, `toolchain.rs`, `lib.rs`
- `rust/lithe-gpui/crates/java/Cargo.toml`
- `rust/lithe-gpui/crates/editor/src/editor_view.rs`
- `rust/lithe-gpui/crates/settings/src/project.rs`, `dialog.rs`
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-gpui/crates/shared/src/workspace_config/toolchain.rs`
- `rust/lithe-core/src/lsp/languages/jdt.rs`
- `shared/fixtures/lsp/`
- `.agents/notes/implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md`
- `.agents/notes/implemented/architecture/2026-09-18-java-project-build-and-launch-boundary.md`
- `.agents/notes/implemented/architecture/2026-09-21-maven-settings-reach-the-language-server.md`
- `rust/lithe-gpui/research/java-spring-maven-inventory.md`
