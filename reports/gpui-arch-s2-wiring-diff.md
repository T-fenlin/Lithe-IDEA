# GPUI 架构缺口 · 第 2 步：lithe-ui 组件库 widget 与宿主引用的差集

**取证脚本**：`reports/gpui-arch-s2-probe.ps1`（只读检索，已实跑）
**运行方式**：仓库根目录 `pwsh -NoProfile -File reports/gpui-arch-s2-probe.ps1`
**基线**：提交 `3085f5d1`，分支 `feat/menu-bar-opaque`
**运行状态**：**已实跑**（2026-10-12，Windows PowerShell 7）。

---

## 结论

组件库 `rust/lithe-ui/src/editor/` 已经交付了**编辑器所需的绝大多数 widget**，而宿主 `rust/lithe-gpui` 对其中**大半零引用**。这不是「功能没写」，是「写完没接」——本次盘点的最大一类架构债。

**量化的差集**（脚本 §B，`\b<名字>\b` 全词匹配，`rust/lithe-gpui` 全目录）：

| 类别 | widget | 宿主引用数 | 判定 |
|---|---|---|---|
| **完全零引用**（12 个） | `SymbolOutline`、`GoToSymbol`、`CallHierarchy`、`RenameInput`、`CodeActionMenu`、`SignatureHelp`、`ProblemsPanel`、`ReferencesPanel`、`SearchFilters`、`LspStatus`/`LspState`、`LineEnding`、`LineNumbers` | **0** | 组件库有、宿主没接 |
| **零引用（装饰类）** | `CodeLens`、`InlayHint`、`GhostText`、`DiffHunk`、`GitMark`、`ReadOnlyBanner`、`VimMode`/`VimModeIndicator` | **0** | 同上 |
| **零引用（面板/控制类）** | `ExtensionsPanel`、`ExtensionState`、`SettingsEditor`、`TaskRunner`/`TaskState`、`ConsoleEntry`/`DebugConsole`/`OutputPanel`、`ThemePreview`、`ProjectSwitcher`、`WelcomePage`、`IndentSettings`、`LanguageMode`、`BranchIndicator`、`NotificationBell`、`CursorShape`、`FindOptions` | **0** | 同上 |
| **已接线** | `CompletionMenu`(2)、`CompletionItem`(2)、`HoverInfo`(3)、`Problem`(30)、`SearchPanel`(6)、`Symbol`(20)、`SymbolKind`(21)、`Keybinding`(47)、`RecentProjects`(57) | >0 | 正常 |
| **只被注释提到** | `FindWidget` | 2（**全是注释**） | 见下 |

### `FindWidget` 的 2 处命中是注释，不是调用

```
rust/lithe-gpui/crates/workbench/src/menu_bar.rs:503:
  // 「查找」「在文件中替换」没有可派发的动作：`lithe_ui` 的编辑器有 FindWidget，
rust/lithe-gpui/crates/workbench/src/menu_bar.rs:2175:
  // 「查找」「在文件中替换」要等编辑器的 FindWidget 接进 `EditorPane`
```

⇒ **`FindWidget` 实际引用数为 0**。源码自己承认了这个缺口（`:2175` 明写「要等…接进 `EditorPane`」）。

**但要说准确**：`Find`/`FindAndReplace` 是 `MenuItem::Action`（`menu_bar.rs:673-674`），点击后的链路是：

```
workspace.rs:1877-1879   MenuAction::Find | FindAndReplace => dispatch_editor_action(...)
workspace.rs:1985-2001   dispatch_editor_action:
                            action.keybind_action()  → menu_bar.rs:506 返回 None
                            → diagnose_menu_run(action, "no_action")  然后 return
menu_bar.rs:1054         diagnose_run → eprintln!("S1_MENU_RUN id={} state={state}")
```

⇒ **不是静默失败**：会打出 `S1_MENU_RUN id=lithe.menu.find state=no_action`，且菜单上**刻意不画键位胶囊**（`menu_bar.rs:2174-2184` 把这三条的 `bool` 标为 `false`，注释写「宁可不画，也不画一颗按不出的键」）。

**所以准确判定是「点了有日志、界面无反应」，不是「假入口」也不是「静默吞错」**——作者是有意识处理的，且遵守了「降级可见」纪律。排期时它属于 P0 体验缺口（Ctrl+F 是最高频操作），但不属于「违反纪律」的问题。

### 宿主自建了组件库已有的东西

`ProblemsPane` 是个典型：宿主在 `rust/lithe-gpui/crates/editor/src/problems.rs:172` **自己写了一个**问题面板（305 行），只从组件库取了 `layout` 与 `theme`：

```
rust/lithe-gpui/crates/editor/src/problems.rs:10: use lithe_ui::layout::{h_stack, v_stack};
rust/lithe-gpui/crates/editor/src/problems.rs:11: use lithe_ui::theme::ActiveTheme as _;
```

而组件库的 `ProblemsPanel`（`lithe-ui/src/editor/problems.rs`，经 `mod.rs:58` 导出）引用数 0。同类还有 `CodeActionMenu`：宿主在 `crates/editor/src/code_actions.rs:21` 改用 `lithe_ui::menus::{Menu, MenuItem}` 自组菜单，而不是用组件库现成的 `CodeActionMenu`（`refactor.rs:50`）。

⇒ **这不是简单的漏接线，是「双实现」**：同一能力在组件库与宿主各有一份。后续要么宿主改用库的，要么把库的删掉——留着两套会让改一处不生效（选型笔记 §3 已定过「不留死 API」的口径）。

---

## crates/editor 的 9 个模块现状（脚本 §C、§F）

| 模块 | 行数/大小 | 职责 | 相对两端的缺口 |
|---|---|---|---|
| `buffer.rs` | 11.9 KB | 文件读取、语言解析、只读判定、2 MiB 上限 | 无结构缺口 |
| `completion.rs` | 17.0 KB | 补全取数 + 接受时的文本编辑 | 已接线 |
| `hover.rs` | 7.2 KB | 悬停取数 + 交给库画 | 已接线 |
| `navigation.rs` | 18.9 KB | 跳转 + **编辑器列口径 ↔ LSP UTF-16 列口径的唯一换算点** | 只接 `definition`；`references`/`implementation`/`typeDefinition` 未接 |
| `diagnostics.rs` | 9.6 KB | JDTLS 诊断 → 编辑器字节偏移 | 已接线（带 generation 防过期） |
| `code_actions.rs` | 11.8 KB | 代码动作取数 + 落到文本 | 已接线（自组菜单） |
| `problems.rs` | 23.0 KB | 问题面板（**自建**） | 库有 `ProblemsPanel` 未用 |
| `go_to_line.rs` | 10.3 KB | 行跳转 | 已接线 |
| `editor_view.rs` | 104.1 KB | 编辑器宿主：标签页、跳转历史、断点槽、会话恢复 | `go_back`/`go_forward` 标 `#[allow(dead_code)]`（:1082、:1093），零调用点 |

**9 个模块里没有**：大纲/符号树、面包屑、编辑器分组（分屏）、查找替换面板、inlay hint、折叠、code lens、语义高亮。

### 关键词实测（脚本 §G）

| 关键词 | 命中 | 说明 |
|---|---|---|
| `breadcrumb` / `Breadcrumb` | **0** | 面包屑完全未实现 |
| `EditorGroup` / `PaneGroup` | **0** | 编辑器分组/分屏完全未实现 |
| `splitEditor` / `SplitEditor` | 1 / 1 | **只是菜单文案**（`menu_bar.rs:743` 的 `lithe.menu.splitEditor`），无实现 |
| `inlay` / `Inlay` | **0** | inlay hint 未接（Core 有 `InlayHints` 操作） |
| `foldingRange` | **0** | 折叠未接（Core 有 `FoldingRanges` 操作） |
| `documentSymbol` / `documentSymbols` | **0** | 文档符号未接（Core 白名单也不放行） |
| `outline` | 10 | **全部是 `lithe.outline.openFile` 命令 id 或 `ButtonVariant::Outline`**，无一个是符号大纲 |

---

## 15 个 MISSING_* 分组的归因（脚本 §D）

按「Core 缺命令 / 接线缺 / 完全未实现」三类归因。使用点 = 命中数 − 2（定义行 + `MISSING_GROUPS` 汇总行）。

| 分组 | 使用点 | 归因 | 依据 |
|---|---|---|---|
| `MISSING_EDITOR_FEATURES` | 9 | **接线缺为主** | 切换注释/复制行/删除行/移动行在 Core 有 `editor.lineEdit`（`LineEditOperation` 7 个变体，`line_edit.rs:16-31`），GPUI 侧引用 **0**；缩略图/空白字符/行号开关在组件库有 `minimap()`/`show_whitespace()`/`set_line_numbers`（`lithe-ui/src/editor/state.rs:292/303/325`），宿主引用 **0** |
| `MISSING_EDITOR_ENTRY` | 7 | **接线缺** | 另存为/全部保存/还原文件/前进/后退/上下标签页。`go_back`/`go_forward` 实现已在（`editor_view.rs:1083/1094`） |
| `MISSING_LSP_REQUESTS` | 7 | **5 接线缺 + 2 Core 缺** | 见第 1 步报告 |
| `MISSING_HELP_ABOUT` | 5 | 未实现 | 帮助/关于 |
| `MISSING_SUBSYSTEM` | 2 | 未实现 | — |
| `MISSING_FILE_LIFECYCLE` | 2 | 未实现 | 新建文件/文件夹/重命名/删除；`explorer` crate 只有 Open/复制路径/刷新 |
| `MISSING_WINDOW_ROUTING` | 2 | 接线缺 | 关闭窗口/退出 |
| `MISSING_SHORTCUTS_VIEW` | 2 | 接线缺 | 组件库有 `KeybindingsEditor`（`keybindings.rs`），宿主引用 3 处（`settings/dialog.rs:730`）——**部分已接** |
| `MISSING_TERMINAL_SPLIT` | 2 | 未实现 | 终端分屏 |
| `MISSING_PROJECT_LIFECYCLE` | 1 | 接线缺 | 关闭项目 |
| `MISSING_LOCAL_HISTORY` | 1 | **Core 缺** | `local-history` 关键词在 GPUI 命中 0；Core 有 `history.entries`/`history.record`，GPUI 引用 0 |
| `MISSING_GITHUB` | 1 | **Core 半通 + 未实现** | Core 有 `github.parseRemote`/`github.requestPlan`/`github.normalizeResponse`，GPUI 引用 0 |
| `MISSING_UPDATER` | 1 | 未实现 | — |
| `MISSING_CLONE_UI` | 1 | 未实现 | — |
| `MISSING_NEW_PROJECT_SCAFFOLDING` | 1 | 未实现 | — |

---

## 顺带复核的两处台账计数

脚本 §E 实测：

| 项 | 台账值 | 我的实测 | 结论 |
|---|---|---|---|
| `MENUS` 数组边界 | 未给 | **行 595–935** | 台账 §2 未写边界，此处补上 |
| 数组内 `MenuItem::NotWired` | 42 | **42** | 一致 |
| 数组内 `MenuItem::Action` | 52 | **51** | **不一致，差 1** |
| 全文件 `MenuItem::Action` | — | 63（另 12 处是 `action_count()` 等非数组引用） | 补充 |
| `NOT_WIRED_OUTSIDE_MENUS` | 2 | **2**（`menu_bar.rs:172-178`，数组声明 `[(&str, Missing); 2]`） | 一致 |
| 合计未接线 | 44 | **44**（42 + 2） | 一致 |

⇒ 台账的 44 这个**总数是对的**，但「可执行入口 `MenuItem::Action` 为 52」这一条实测是 **51**。总数自洽（42+2=44）说明台账没算错总数；差 1 的可能是我与台账对「数组内」的边界口径不同（例如把 `:941` 的 `action_count()` 里那处算进去就是 52）。**我没有改台账，只记录实测值与差异**——这条交给主理人决定是否订正。

---

## 风险与条件

| 风险 | 影响 | 触发条件 | 缓解 |
|---|---|---|---|
| **双实现漂移** | 组件库与宿主各有一份（`ProblemsPanel` vs `ProblemsPane`、`CodeActionMenu` vs 自组 `Menu`），改一处不生效 | 后续任一侧改视觉或行为 | 排期时先定「谁是真源」，另一份删掉；不留死 API（选型笔记 §3 口径） |
| **假入口** | `Find`/`FindAndReplace` 是 `MenuItem::Action`（亮着可点）但 `keybind_action()` 返回 None，`FindWidget` 未接 ⇒ 点击只打 `S1_MENU_RUN ... state=no_action`，界面无反应 | 用户点 Ctrl+F | 要么接上，要么退回 `NotWired` 并显示角标。注意作者**已有意识处理**（不画键位胶囊 + 打诊断日志），所以这不是纪律问题，是体验缺口 |
| **零引用 ≠ 该删** | 组件库有 659 项测试（选型笔记 §3），零引用只说明宿主没接，不说明该能力无价值 | 有人按「零引用」清理组件库 | 清理前先确认是「产品不需要」还是「还没接」 |
| **outline 命中 10 是噪声** | 误判「大纲已实现」 | 只 grep 关键词不看上下文 | 判定口径：看是否为 `SymbolOutline`/`GoToSymbol` 类型引用，不是看 `outline` 子串 |

### 假设与限制

1. **本文的引用计数是「名字出现次数」，不是「有效调用点数」**。`Keybinding`(47)、`RecentProjects`(57) 这类高命中里含大量类型声明与测试，不能反推「接线完整」。判定单个能力是否接线，我逐项看了上下文（如 `FindWidget` 的 2 处确认为注释）。
2. **未验证组件库 widget 的实际可用性**：`SignatureHelp`、`SymbolOutline` 等只确认了导出与构造签名（`hints.rs:246`、`symbols.rs:162`），**没有运行**它们，不知道在真实数据下是否正常渲染。这属于运行验证，本次**未做**。
3. **组件库有 659 项测试**（选型笔记 §3 所述）可作回归网，但我**未运行** `cargo test -p lithe-ui`，不知道当前是否全绿。

---

## 建议与验收

| 优先级 | 行动 | 前置 | 完成标准 |
|---|---|---|---|
| P0 | 接 `FindWidget` 到 `EditorPane`（Ctrl+F / Ctrl+H） | 无 | `menu_bar.rs:673-674` 两条 Action 的 handler 不再返回 None；启动日志有查找打点；**删掉 `:2175` 那条注释** |
| P0 | 接编辑器显示开关：缩略图 / 空白字符 / 行号 | 无（组件库 builder 齐备） | `MISSING_EDITOR_FEATURES` 的这 3 条换成 Action；`lithe-ui` 的 `minimap()`/`show_whitespace()`/`set_line_numbers` 在宿主有调用点 |
| P0 | 接行编辑命令：切换注释 / 复制行 / 删除行 / 移动行 | 无（Core `editor.lineEdit` 已就绪） | 4 条换 Action；Core 引用数从 0 变 4 |
| P1 | 接 `go_back`/`go_forward` | 无（实现已在） | 去掉两处 `#[allow(dead_code)]`；`GoBack`/`GoForward` 有 action 定义与键位 |
| P1 | 接大纲面板（`SymbolOutline`） | 依赖 Core 放行 `documentSymbol` 或复用 `java.structure` | 右栏或侧栏出现符号树 |
| P2 | 定 `ProblemsPanel` / `ProblemsPane` 的真源 | 需与 UI 设计师核对 | 只留一份；`UPSTREAM.md` 留痕 |
| P2 | 订正台账 `MenuItem::Action` 计数 | 交主理人 | 台账与实测一致（51 或说明口径） |

**验收方式**：`S1_*` 启动日志打点，不是截图。每接一根线同步删掉对应 `MenuItem::NotWired` 与 `MISSING_*` 使用点。

**工作量（架构维度，不含视觉设计与实现工时）**：
- P0 三项接线（查找 + 3 个显示开关 + 4 个行编辑命令）：**2 人日**
- P1 跳转历史接线：**0.5 人日**
- P1 大纲面板接线：**1 人日**（若需先补 Core 的 `documentSymbol` 白名单则另计，且不计入本方向）
- P2 双实现收口：**0.5 人日**（定真源 + 删一份）
- **依赖**：P0 三项之间无依赖，可同一批写完再一起构建（构建 15 分钟/次）；大纲依赖 Core 或 `java.structure` 通路，排在其后。

---

## 未运行验证清单

- 未运行任何 `cargo` 命令（未构建、未跑测试，含 `cargo test -p lithe-ui`）。
- 未启动 Lithe 进程，无 `S1_*` 日志。
- 未验证 `lithe-ui` 各 widget 在真实数据下的渲染结果。
- 引用计数为静态全词匹配，未区分类型引用 / 测试引用 / 注释。
