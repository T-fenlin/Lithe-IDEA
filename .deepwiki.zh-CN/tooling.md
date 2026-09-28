# 工具链与验证

这个仓库里有很大的 `scripts/` 目录和 8 个 GitHub workflows，但真正的 Rust 相关 CI 并不多。要理解这个项目最重要的一点是：哪些检查是必须的门禁，哪些只是本地辅助脚本。

## 本地门禁

`README.md` 中的 authority pre-submit 列表是最终的本地验证标准：

```bash
cargo fmt --manifest-path rust/Cargo.toml -p lithe-core -- --check
cargo test --manifest-path rust/Cargo.toml -p lithe-core
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-app
./scripts/verify-rust-core.sh
./scripts/verify-shared-contracts.sh
node scripts/verify-agent-notes.mjs
node scripts/test-classify-ci-changes.mjs
```

README 明确写道：GPUI 宿主尚无 CI lane，因此这些检查就是当前的门槛。

`develop-lithe` Skill 进一步补充了“变更→检查”的规则：

| 变更类型 | 最小验证 |
| --- | --- |
| Agent Notes / 架构决策 | `node scripts/verify-agent-notes.mjs` |
| 测试代码或测试基础设施 | `./.agents/skills/write-stable-tests/scripts/verify-test-stability.sh`，再用受影响的 Rust timing harness |
| Shared contracts 或 JSON fixture | `./scripts/verify-shared-contracts.sh` |
| Rust Core 或 Core↔host contract | `./scripts/verify-rust-core.sh` |
| `gpui/crates/*` | 受影响 crate 的 `cargo test -p <crate>` + `cargo fmt -- --check` |
| CI lane 或 path classifier | `node scripts/test-classify-ci-changes.mjs` |
| Java semantic ownership | `node scripts/verify-java-semantic-ownership.mjs` |

## `verify-rust-core.sh`：主要的 Rust 门禁

`scripts/verify-rust-core.sh` 的执行顺序大致为：

1. `node --test scripts/test-rust-core-comments.mjs`
2. `verify-rust-core-comments.sh`：要求所有生产模块以 `//!` 开头、注释不能含有 Han 字符、公开 `unsafe fn` 必须有 `///` 和 `# Safety`，并用 `cargo rustdoc -D missing_docs -D rustdoc::broken_intra_doc_links` 检查文档
3. `verify-rust-core-layout.sh`：只允许 `lithe-core/src` 根目录存在 `lib.rs`，并要求 8 个 package facade（protocol runtime project execution languages git lsp tests）存在
4. `cargo fmt -p lithe-core -- --check`
5. 限时 `cargo test -p lithe-git-host`（120 s）
6. `cargo test -p lithe-core`
7. `cargo test -p lithe-gpui-shared -p lithe-gpui-app -p lithe-gpui-workbench`

注意：第 7 步只覆盖 3 个 GPUI crate；其他 crate 需要按需单独做 `cargo test -p <crate>`。

## 其他校验脚本

| 脚本 | 约束内容 |
| --- | --- |
| `verify-rust-core-comments.sh` | Rust Core 注释标准：模块文档、英文注释、导出 rustdoc、unsafe Safety 章节 |
| `verify-rust-core-layout.sh` | `lithe-core/src` 的 package facade 结构 |
| `verify-shared-contracts.sh` | 所有 fixtures / schemas 的 JSON 有效性和 7 个结构断言 |
| `verify-java-semantic-ownership.mjs` | 防止旧私有 identifier 重现于 `lithe-core/src` 或 gpui java 目录 |
| `verify-agent-notes.mjs` | 检查 agent note 生命周期、分类及 required headings |
| `classify-ci-changes.sh` | CI path classifier |
| `validate-stable-release-notes.mjs` | 稳定版本说明的双语标题顺序和内容 |
| `verify-download-cache.mjs` | 核验 Cargo/JDTLS/JDK/Bun 缓存与 `third_party` 清单一致 |
| `invoke-cargo-with-cache-fallback.ps1` | Windows 下的 Cargo 包装器：若 target dir 越界将 hard fail，并重试一次 |

## CI：实际运行了什么

仓库中存在 8 个工作流：

| Workflow | 触发条件 | 办事内容 |
| --- | --- | --- |
| `ci-database.yml` | PR / push to main / dispatch | 3 jobs：changes → rust-database-tests → gate |
| `verify-agent-notes.yml` | `.agents/notes/**` 等路径触发 | 执行 Node 校验 |
| `deploy-agent-notes-board.yml` | `preview` 分支 push | 验证 notes 后生成 board |
| `lithe-pr-review.yml` | issue comment created | 系统 review 候选 |
| `lithe-issue-claim.yml` | issue comment created | `/assign` / `/unassign` |
| `lithe-issue-priority.yml` | issue opened/edited | 解析 priority / platform |
| `sync-atomgit-release.yml` | workflow_call + dispatch | 已经失效 |
| `update-repo-charts.yml` | daily cron | 更新 star / merged PR 图表 |

需要特别注意：`ci-rust.yml` 实际并不存在，虽然分类器里有对应 case；当前真正被消费的仅是 `rust_database` lane。

## 路径分类器

`classify-ci-changes.sh` 会分析 `git diff --name-status --find-renames base head`，输出五个 `GITHUB_OUTPUT`：

- `rust_core`
- `rust_database`
- `gpui`
- `rust_comments`
- `metadata`

它会按以下规则做判断：

- rename / copy ⇒ `enable_all_validation`
- `*.md` / `docs/*` / `.agents/*` 等路径一般不触发 lane
- Rust Core 注释-only 更新会触发 `rust_comments`，而不是 `rust_core`
- `gpui/crates/*/src/*` 会同时设置 `gpui` 和 `rust_core`
- `rust/Cargo.toml` / `Cargo.lock` 会触发全部 lane
- `shared/*` 同样会触发 `rust_core` + `gpui`

## 构建配置

`rust/Cargo.toml` 中有几个非常关键的配置：

- `resolver = "3"`：GPUI 采用 edition 2024，Core 使用 2021，必须使用 resolver 3
- release profile 中使用 `strip`、`lto`、`codegen-units = 1`
- dev/test 中对 `num-bigint-dig` 做优化，以减轻认证时的大整数计算开销
- GPUI 相关 crates 在 dev 构建中也被单独优化，避免 `cargo run` 过慢

## 主题和资产工具

以下脚本支持主题和 UI 资产的生成 / 校验：

- `gpui/tools/generate-idea-icons.mjs`
- `gpui/tools/extract-locale.mjs`
- `gpui/tools/check-ui-px.mjs`
- `gpui/capture-screenshot.ps1`
- `ui-click.ps1`

## 清理义务

AGENTS.md 明确要求：凡是启动用于构建、测试、调试或预览的 Lithe 进程，必须在任务结束前关闭，并在有界清理后说明是否还剩余进程。

## 结论

这个仓库的“真实门禁”不是 GitHub UI 上展示的那张大表，而是脚本与 contract 共同制定的少数几条验证路径。需要知道这些规则，才能在改动时判断“这次变更真正需不需要跑什么”。
