# CI builds and test downloads

CI 构建缓存、架构并行、测试产物和 artifact 保留策略见
[`2026-09-13-ci-build-cache-and-artifact-strategy.md`](../.agents/notes/implemented/process/2026-09-13-ci-build-cache-and-artifact-strategy.md)。
本文只保留当前 CI 使用说明、下载方式和历史观测。

当前仓库只保留统一 GPUI 前端（`rust/lithe-gpui`，独立 cargo workspace）的三平台
流水线：GPUI CI（三平台构建与测试）、Linux CI（共享 Rust Core）、GPUI preview
release（滚动预发布）。`.github/workflows/` 只包含这三个工作流；此前的
macOS / Windows 产品端及其 CI/CD、打包与签名脚本已移除，相关历史观测不再随
本文件更新。

- GPUI CI 的每个平台 job 独立编译宿主目标并运行 workspace 测试；产物为
  `test-stability-gpui-<platform>` 测试报告，保留 14 天。
- Linux CI 只编译和测试共享 Rust Core，不产出可安装产物，不触碰任何发布契约。
- GPUI preview 每日构建（或手动指定分支），发布到滚动 prerelease
  `preview-gpui-<version>`；资产清单见下文，与打包脚本产出一一对应。

## GPUI CI（三平台）

`ci-gpui.yml` covers the unified GPUI front end, which is a separate cargo
workspace under `rust/lithe-gpui/` (with the component library at
`rust/lithe-ui/`).

- The change classifier selects the lane through its `gpui` output: changes
  under `rust/lithe-gpui/` or `rust/lithe-ui/`, edits to the GPUI workflows or
  packaging scripts, and non-test Rust Core changes (the GPUI workspace links
  `lithe-core` in-process through `lithe-gpui-shared`) all run the lane.
- One job per platform — `macos-latest` (aarch64-apple-darwin),
  `windows-latest` (x86_64-pc-windows-msvc) and `ubuntu-latest`
  (x86_64-unknown-linux-gnu) — each with its own Cargo download and build
  caches keyed by target, compiler commit and the GPUI manifests. The in-tree
  `rust/lithe-gpui/target/` is used on CI; local sandboxes still need an
  external `CARGO_TARGET_DIR`.
- Each job runs the workspace tests through the shared Rust timing runner and
  then builds `lithe-gpui-app` for the host target. Test reports upload as
  `test-stability-gpui-<platform>` artifacts.
- Linux jobs install the GPUI desktop system libraries (xkbcommon, fontconfig,
  wayland/X11 client libraries, Vulkan loader) before building.
- The formatting check is informational (`continue-on-error` with a warning)
  because the workspace still carries rustfmt drift; it becomes blocking once
  `cargo fmt --all` is clean.

## Linux CI（共享 Rust Core）

`ci-linux.yml` adds Linux coverage for the shared Rust workspace
(`rust/lithe-core`, `lithe-git-host`, `lithe-agent-host`, `lithe-ide-host`).
Linux has no Lithe product form, so this lane only compiles and tests; it
produces no installable artifacts and touches no release contract. The
database crates already run on `ubuntu-latest` in the shared Rust Core lanes,
so they are not duplicated here.

## GPUI preview release

`release-gpui.yml` is the nightly/preview channel for the GPUI front end
(scheduled daily, or `workflow_dispatch` with a `source_branch`). It builds
release binaries for macOS (arm64 and x86_64), Windows x64 and Linux x64 and
publishes them to a rolling prerelease tagged `preview-gpui-<version>`, where
`<version>` comes from `rust/lithe-gpui/crates/app/Cargo.toml`.

- Archives are produced by `scripts/package-gpui-release.sh`:
  `Lithe-GPUI-<version>-<platform>.tar.gz` (`.zip` on Windows) plus a
  `.sha256` file. Windows archives keep the side-by-side DLLs next to
  `Lithe.exe`. The GPUI front end has no installer yet, so archives contain the
  bare cargo binary.
- Windows binaries are signed with `scripts/sign-windows.ps1` when the
  Authenticode certificate secrets are configured. Without them the archive is
  published unsigned and both the release notes and the step summary say so.
- Before upload the publisher prunes stale `Lithe-GPUI-*` assets it no longer
  owns (other assets on the release are never touched); after upload
  `scripts/verify-release-assets.mjs` fails the job unless the published asset
  list matches the expected manifest exactly.
- GPUI preview assets are deliberately excluded from every stable Release:
  promoting GPUI to stable requires installers and an update story first.

Git 路径往返集成测试 `rust/lithe-core/tests/git_path_roundtrip.rs` 需要 Git 和
Node.js 22.6+（用于直接加载实际前端 TypeScript 路径规范化函数），不需要安装
Bun 或前端依赖。CI 复用计时脚本已使用的 runner Node.js，本地运行时需满足
上述最低版本；该测试随 SharedRust 计时测试执行，结果写入现有
`.artifacts/test-stability/` 报告。

## Build time and caches

GPUI CI 的每个平台 job 使用两级 Cargo 缓存，均放在 job 内隔离的
`CARGO_HOME=$GITHUB_WORKSPACE/.artifacts/cargo-home` 下：

- 下载缓存（`.artifacts/cargo-home/registry/cache`）：key 为
  `lithe-<os>-<arch>-gpui-cargo-downloads-v1-` 加 `rust/lithe-gpui/Cargo.lock`
  的哈希；还原后由 `scripts/verify-download-cache.mjs` 对照 lock 校验，失败
  即丢弃该缓存并正常下载。
- 构建缓存（`rust/lithe-gpui/target/` 的 fingerprint/build/deps/incremental）：
  key 为目标三元组、rustc commit 哈希加四个 manifest（lithe-gpui、lithe-ui、
  lithe-core 各层 `Cargo.toml`）的联合哈希。`rust/lithe-core/**/Cargo.toml`
  进 key 意味着 Core 频繁变更会刷新 GPUI 构建缓存——这是正确性优先的取舍。
- 两个缓存步骤都设 `continue-on-error`：缓存服务不可用时流程不失败，退化为
  完整构建。
- 每个 job 用 `CARGO_BUILD_TARGET` 固定目标三元组，使 `cargo test` 与
  `cargo build` 共用同一个 target 目录，避免同一依赖图编译两次。

release-gpui 的构建步骤复用同一套缓存键（profile 段为 `release`），并在
release + LTO  profile 下编译；超时预算见工作流内的 `timeout-minutes`。

### 本地编译的统一入口

本地（非 CI）编译、跑测试、启动统一走 `scripts/build-gpui.ps1`，不要再手工拼
`cargo` 参数。这个脚本把此前只存在于文档和记忆里的配方固化成一处：

```powershell
./scripts/build-gpui.ps1 -Action build                          # 编译 debug 产物
./scripts/build-gpui.ps1 -Action test -TestPackage lithe-gpui-settings
./scripts/build-gpui.ps1 -Action run -AppArgument . --open-settings   # 编译并启动
./scripts/build-gpui.ps1 -Action launch                         # 只启动已有产物
```

脚本固定三件事，都是有依据的取舍而不是默认值：

- **`CARGO_TARGET_DIR` 落在工作区之外**（默认 `<仓库父目录>/lithe-gpui-target`）。
  工作区被沙盒施加 ACL，位于其中的可执行文件运行时被拒绝一切写入；脚本会拒绝
  指向工作区内的 target 目录。
- **`RUST_MIN_STACK=67108864` 与 `-j 2`**：内存紧张时 rustc 会崩成
  `STATUS_STACK_BUFFER_OVERRUN`，linker 报 `LNK1102`，而崩溃会污染 target 缓存。
  默认**不改** `CARGO_PROFILE_DEV_DEBUG`（改它会换 profile 指纹、作废已有增量产物）；
  确实吃紧时加 `-Lean`。
- **启动时注入 `LITHE_JDTLS_ROOT`**：开发期 exe 在 target 目录下，向上找 12 层也够不到
  仓库根的 `.artifacts/jdtls`。正式打包后 `LanguageServers/jdtls` 就在 exe 旁边，不需要它。

CI 走的是等价但不同的命令（`--locked`、显式 `--target`、树内 `target/`），见
`.github/workflows/ci-gpui.yml` 的 “Build the GPUI app binary” 一步；两者不需要一致，
本地不带 `--locked` 是刻意的。

### 独立工作树的本地编译

Git worktree 只共享 Git 对象，不共享各自的 `.artifacts` 目录。如果从一个
工作树单独创建另一个工作树进行编译，优先复用原工作树已经下载或构建完成的
资源，避免重复等待网络下载和资源准备。

在新工作树根目录运行资源复用脚本，并通过 `--source` 指向已有缓存的同仓库
工作树：

```bash
node scripts/reuse-worktree-resources.mjs --source /path/to/existing-worktree
```

脚本默认处理注册表中的全部资源，也可以重复传入 `--resource` 只处理指定资源：

```bash
node scripts/reuse-worktree-resources.mjs \
  --source /path/to/existing-worktree \
  --resource cargo \
  --resource jdtls \
  --resource jdk
```

`node scripts/reuse-worktree-resources.mjs --list` 可以查看当前注册资源。脚本只允许
同一 Git 仓库的 linked worktree 互相复用资源，不修改源工作树。它先把源资源
复制到目标工作树的临时目录，按照对应的 manifest、lockfile 或完整性清单校验，
再原子替换目标目录并进行第二次校验。目标已有不少于源缓存的有效文件时会保留
目标，不重复复制；校验失败的文件不会发布到目标缓存。

JDTLS 和 JDK 使用
[`third_party/jdtls/manifest.json`](../third_party/jdtls/manifest.json) 与
[`third_party/jdk/manifest.json`](../third_party/jdk/manifest.json) 中的
SHA-256；Cargo 使用各 workspace 的 lockfile 与缓存完整性清单。当前
自动复用的资源由 [`scripts/worktree-resources.json`](../scripts/worktree-resources.json)
统一注册：

- `.artifacts/cargo-home/registry/cache/`：Cargo crate 下载归档；按
  `rust/Cargo.lock`、`rust/lithe-gpui/Cargo.lock`、`rust/lithe-ui/Cargo.lock`
  中的 checksum 校验。三个 workspace 共用这一个注册表缓存，校验清单覆盖全部
  lock，避免任一 workspace 的归档被误判为"当前 lock 未引用"而清掉。
- `.artifacts/jdtls-downloads/`：JDTLS、Lombok、Java Debug/Test 和 license。
- `.artifacts/jdk-downloads/`：各平台与架构的 bundled JDK 下载归档。

以下目录不应直接复制或跨工作树共享：

- Java 启动临时文件：`<system-temp>/lithe-run/launch-<pid>-<counter>.argfile`
  和同目录的 `.classpath.jar` 由平台启动 adapter 为单次执行独占创建，包含该次
  执行的绝对类路径、工作目录、JDK 版本及编码语义，没有可复用的版本、平台、架构
  或工具链 identity stamp。准备失败、准备完成前已取消、启动失败或进程退出后由
  所有者删除；系统临时目录由平台解析，不写安装包或 JDK，不影响签名或增量更新。
  `excludedResources.java-launch-temporaries` 经复用脚本的排除路由直接拒绝，任何
  复制阶段都不得共享；内容哈希相同也不能转移进程所有权。

- Agent CLI 的用户级安装与下载缓存：npm 的 global prefix/cache、Homebrew 的
  Cellar/Caskroom/cache、用户目录下 `.local/share/claude/versions`。它们由运行时
  `PATH` 和原安装器决定，不属于工作树；包版本、平台与架构由原安装器校验，
  没有工作树构建身份 stamp，任何复制阶段都禁止复用。注册表的
  `excludedResources.agent-cli-runtime` 记录此边界，脚本显式拒绝选择它。

- Codex 短重连配置 helper：`<system-temp>/lithe-codex-retry-<UUID>/` 由单次
  Agent 连接独占，内容来自当前 Rust Host 内嵌的一方源码，原生 CLI 路径和预算
  经非敏感环境字段传递，密钥仍只走标准输入。系统临时目录由平台解析；准备、
  启动失败、取消或进程树结束后删除。它没有可复用的版本、平台、架构、工具链
  stamp，任何复制阶段都不得共享；`excludedResources.codex-retry-relay` 走脚本
  的排除路由。helper 不修改已安装适配器、用户配置或安装目录，因此不改变
  发布基线。

- Agent 历史注释：平台偏好设置键 `lithe.agent-history.v1.<workspace-agent-digest>` 保存收藏、自定义标题和隐藏状态，按标准化工作区与 Agent ID 隔离。它是用户可变状态，不受版本、平台、架构或工具链构建身份约束，不存在可验证的构建 stamp；Markdown 导出写到用户选择的位置。两者都禁止在任何复制阶段跨工作树复用，`excludedResources.agent-history-metadata` 由资源脚本显式拒绝。

- `.artifacts/jdtls/`、`.artifacts/jdk-*` 等可以由已验证下载重新生成的解压输出；
- `rust/lithe-gpui/target/`、`rust/target/` 中与当前源码、编译器或构建参数绑定的构建输出；
- LSP workspace `-data`、运行时数据库、测试报告和其他会被进程修改的状态。
- 平台缓存中 JDT `-data/.lithe/maven/` 的 settings 副本：按工作区隔离，可能含凭据，
  在缓存过期或重建索引时清理；文件内容哈希不是版本、平台、架构或工具链构建
  identity stamp，任何复制阶段都禁止共享。资源清单 `jdt-maven-settings` 显式排除，
  复用脚本直接拒绝该资源，不进入下载或生成物校验路由。

PHPUnit 测试夹具的 `shared/fixtures/phpunit-project/vendor` 由当前工作树独立
安装，Composer 依赖取决于该工作树的 PHP 版本与依赖解析，没有可靠 identity
stamp，不跨 worktree 复制。

如果后续新增可复用资源，必须同步更新注册表、校验器、脚本测试和本节说明。
生成资源只有在构建流程写入可验证的源码、配置、平台、架构和工具链 identity
stamp 后才能加入注册表，不能仅凭目录存在就跨 worktree 复制。

The artifact behavior and compression setting follow
[actions/upload-artifact](https://github.com/actions/upload-artifact), and cache
reuse follows GitHub's
[branch access restrictions](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).
