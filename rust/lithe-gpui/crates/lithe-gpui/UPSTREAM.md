# 上游来源与改动记录

本 crate 是 **Ely GPUI Components** 的衍生副本（vendored fork）。移植决策见
`.agents/notes/implemented/architecture/` 下记录该决策的 Agent Note。

## 来源

| 项 | 值 |
| --- | --- |
| 项目 | Ely GPUI Components |
| 仓库 | https://github.com/ZacharyZhang-NY/Ely-GPUI-Components |
| 提交 | `b4716bb09bae4ee9c26afc7197d0f8f6c08f92c2`（2026-10-05） |
| 许可 | MIT（Copyright (c) 2026 Ely GPUI Component contributors） |
| 许可原文 | 同目录 `LICENSE-ELY` |

MIT 允许衍生与再分发，**前提是保留版权声明与许可文本**。所以：

- `LICENSE-ELY` 必须随本 crate 一起保留，不得删除。
- 源码中的 `Ely` 字样（`src/**` 的文档注释 30 处）是**来源署名**，不是待改的旧名，
  不要为了“统一命名”清掉它们。要改的是**标识符与用户可见文案**（见下）。

## 上游原始形态

- 单 crate `ely-gpui-component`，edition 2024，MSRV 1.95。
- 依赖 `gpui` / `gpui_platform` 于 zed 上游 git rev
  `1a28cff4b409169bac058bca40dfbfeb7621d19b`。
- 不用 `gpui-base` / `gpui-component`：直接构建在 gpui 框架本身之上。

## 本副本的改动清单

移植时**只做必要改动**，其余保持与上游逐文件一致，以便对照与同步。

### 1. 包与依赖（`Cargo.toml`）

- 包名改为 `lithe-gpui`。
- **gpui 依赖改为包重命名**，指到 Lithe 钉住的版本线：

  ```toml
  gpui = { package = "gpui-pre", version = "=0.3.7", default-features = false, features = ["stacker"] }
  gpui_platform = { package = "gpui-pre-platform", version = "=0.3.7", features = ["font-kit"] }
  ```

  这一步让源码里 800+ 处 `use gpui::…` / `gpui_platform::…` **一行都不用改**。
- `[profile.web]` 移除：member crate 不生效（只在 workspace 根有效）。
- 未改动其余依赖。

### 2. 标识符与文案（源码批量替换）

| 旧 | 新 | 说明 |
| --- | --- | --- |
| `ely_gpui_component` | `lithe_gpui` | crate 引用（217 处） |
| `ely-gpui-component` | `lithe-gpui` | 同上（连字符形式） |
| `"ely: …"` | `"lithe-gpui: …"` | panic 文案 |
| `actions!(ely, …)` | `actions!(lithe, …)` | 动作命名空间 |
| `"ely-<id>"` | `"lithe-<id>"` | 元素 ID / 临时文件名 |
| `ely_live_` / `ely_icon` / `ely_terminal` / `ely-theme-` | `lithe_*` | 内部标识 |
| `elyPicked` / `elyRoute` | `lithePicked` / `litheRoute` | macOS 原生桥标识 |
| `ELY_GALLERY_ASSETS` | `LITHE_GALLERY_ASSETS` | 画廊构建期环境变量 |

**只改标识符，不改任意字符串数据。** 测试夹具里的 URL、示例文本、临时目录名
（如 `ely.dev`、`"Ely, ely, ELY and élan"`）**一律保持上游原样**：它们是被解析/断言的
数据，改了会连带改坏硬编码的 range 断言，却没有命名收益。（初版移植曾一并改掉，
被 Ely 自带测试抓出 3 处 range 偏差后回退——这也是本副本第一次用上游测试
验证自身改动的记录。）

### 3. 未改动（有意）

- **30 处文档注释里的 `Ely`**：署名，保留。
- 全部组件逻辑、主题、布局、章节目录结构。

## 尚未完成

- **图标体系与字体**：上游 `assets/`（3.0 MB，318 文件）含 IBM Plex Sans 与它自己的
  图标集。Lithe 的字体政策（CONTEXT.md：内置字体 JetBrains Mono、界面字体用系统 UI 字体）
  与图标体系（既有 Agent Note）尚未合并，当前先原样随 crate 带入。
- **API 对齐**：上游保留自己的词汇（`ControlSize`、`IconName`、`new(id, label)`），
  未对齐 gpui-kit 的 `Sizable`/路径注入等约定。
- **语义差异**：仅修到能编译为止；同名 API 的行为差异需靠测试逐步暴露。

## 同步上游

上游仍在演进。同步时：

```bash
# 在 .artifacts 下拉取上游
git -C .artifacts/ely fetch --depth 1 origin main
git -C .artifacts/ely log -1 --format='%H %ad %s' --date=short

# 对照差异（先看改了什么，再决定是否跟进）
git -C .artifacts/ely diff <旧rev> HEAD --stat -- src/
```

跟进后更新本文件的**提交**行，并重跑上面的标识符替换（上游可能新增 `ely_*` 标识）。
