# Lithe — 概览

Lithe 是一个面向 Java / Spring 开发的“低内存 IntelliJ IDEA 替代方案”，它采用纯 Rust 仓库实现。它的卖点是：打开一个项目后，常驻内存可控制在约 300–400 MB 左右，而传统完整 IDE 往往需要数 GB 以上的占用。语言服务器、终端、构建工具、调试器和数据库辅助工具都会按需启动。

仓库状态（在假设某个功能可用前请先阅读）：Swift / macOS 前端和 React / Tauri / Windows 前端已被移除。现在保留下来的只有 `rust/gpui/` 下面的单一 GPUI Kit 宿主，它仍在积极开发中，且目前没有正式发布流水线。GitHub Releases 上的安装包来自已删除的旧源代码，只保留作为参考（`README.md:87-93`）。

## 技术栈

| 层级 | 选择 | 位置 |
| --- | --- | --- |
| UI 框架 | GPUI Kit `0.6.6`（加上 `gpui-pre 0.3.6`, `lsp-types 0.97`） | `rust/Cargo.lock:2800-2803` |
| 语言 | Rust 2021（核心 crates） / Rust 2024（gpui crates） | `rust/Cargo.toml:10-13` |
| i18n | `rust-i18n 4.2`，YAML 目录 | `rust/gpui/crates/shared/locales/` |
| 资源 | `rust-embed 8`（`LitheAssets`） | `rust/gpui/crates/app/src/assets.rs:110-122` |
| 语言服务 | Eclipse JDT LS 1.61.0（清单固定版本，构建时下载） | `third_party/jdtls/manifest.json` |
| 运行时 JDK | Temurin 21（代码中强制要求最低 major 21） | `rust/gpui/crates/java/src/jdtls.rs:80` |

## 仓库地图

```
Lithe-IDEA/
├─ rust/                    唯一的 Rust 工作区（rust/Cargo.toml:15-23）
│  ├─ lithe-core/           确定性命令表面，无 UI，无平台依赖
│  ├─ lithe-git-host/       原生 Git 进程 + AskPass 适配器
│  ├─ lithe-db-sidecar/     单次 JSON 数据库子进程（尚未接到 gpui）
│  ├─ lithe-db-mcp/         通过 sidecar 暴露的 MCP stdio 服务（尚未接到 gpui）
│  └─ gpui/crates/          10 个 crate 的 GPUI 宿主（app, workbench, editor, explorer,
│                           git, terminal, java, settings, notify, shared）
├─ shared/                  契约（文档 + JSON schema）和跨宿主 fixture
├─ .agents/                 skills + 中文架构决策笔记
├─ scripts/                 校验门禁、打包、发布自动化
├─ .github/workflows/       CI 流水线（请看 tooling.md；大多数 Rust 部分没有独立 lane）
├─ third_party/             上游 manifests，仅保留清单，不包含源码
├─ infra/                   数据库校验用 docker compose
├─ docs/                    发布说明、截图、一个 JSON schema
└─ rust/gpui/               主题、资产、UI map、PLAN.md、研究笔记
```

## 构建与运行

工作区根目录是 `rust/Cargo.toml`。`rust/gpui/Cargo.toml` 不存在——前端删除后，两棵树被合并进同一份 Rust 工作区（`rust/Cargo.toml:1-13`）。

```bash
cargo build --bin Lithe
./rust/target/debug/Lithe <workspace-root>     # 位置参数为可选
```

`--theme <id|name>` 和 `--locale <tag>` 可在一次启动中覆盖设置，但不会写回到配置；`LITHE_GPUI_SETTINGS_FILE` 会把 `settings.json` 和 `recent-projects.json` 重定向到临时路径（`rust/gpui/crates/settings/src/paths.rs:81-100`）。

## 建议阅读入口

| 需要回答的问题 | 页面 |
| --- | --- |
| 各个 crate 是怎么协作的？边界规则是什么？ | [Architecture](architecture.zh-CN.md) |
| Core 暴露了哪些命令，如何分发？ | [Rust Core](rust-core.zh-CN.md) |
| 应用如何启动，为什么顺序是固定的？ | [GPUI App Shell](gpui/app.zh-CN.md) |
| 窗口布局是怎样的？ | [Workbench](gpui/workbench.zh-CN.md) |
| 文本编辑是怎么工作的？ | [Editor](gpui/editor.zh-CN.md) |
| 宿主如何调用 Core，以及共享层包含什么？ | [Host Shared](gpui/shared.zh-CN.md) |
| JDT LS 是如何被发现、启动和停止的？ | [Language Service](gpui/java.zh-CN.md) |
| Explorer / Git / Terminal 面板 | [Feature Panes](gpui/features.zh-CN.md) |
| 设置、主题、通知中心 | [Settings & Notify](gpui/settings.zh-CN.md) |
| 什么是兼容性表面？ | [Contracts & Fixtures](contracts.zh-CN.md) |
| 在交接前必须通过什么检查？ | [Tooling & Validation](tooling.zh-CN.md) |
| 为什么它会这样设计？ | [Decisions & Notes](decisions.zh-CN.md) |
| Git、数据库 crate、上游 pinning | [Supporting Crates](supporting-crates.zh-CN.md) |

## 本仓库的阅读约定

- **注释和模块文档在一等代码中使用中文**；只有 `rust/lithe-core/` 中必须使用英文，这是脚本强制要求的（`.agents/skills/develop-lithe/SKILL.md:96-124`）。
- **`S1_*` 诊断行是一种刻意设计的、可被机器抓取的接口。** 几乎所有非平凡的状态转移都会输出它；如果顺序不重要则使用 `println!` 打到 stdout，而在顺序重要时使用 `eprintln!` 打到 stderr（stdout 在重定向时是块缓冲的，进程存活期间可能丢失输出 —— `rust/gpui/crates/app/src/main.rs:926-931`）。
- **“Not wired” 是一个真实且命名的状态。** 未实现的功能会使用 `MenuItem::NotWired { .. }`、`NotWired` 标志、空状态页，或在某个 crate 的 `lib.rs` 底部声明显式的 “not implemented” 注册；从不以“死控件”方式悄悄隐藏。

## 参考资料

- `README.md`
- `rust/Cargo.toml`
- `AGENTS.md`
- `.agents/skills/develop-lithe/SKILL.md`
- `rust/gpui/crates/app/src/main.rs`
- `rust/gpui/crates/app/src/assets.rs`
- `rust/gpui/crates/shared/locales/`
- `third_party/jdtls/manifest.json`
