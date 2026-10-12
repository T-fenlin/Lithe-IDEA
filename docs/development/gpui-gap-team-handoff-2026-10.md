# GPUI 缺口盘点专家团交接文件（2026-10-12）

## 说明

本文件是 s2 步骤的交付物。原计划通过「Lithe 编辑器开发团」并行下发简报给六名成员
（界面设计师陆井然、UI 设计师颜思齐、编辑器架构师闻本立、核心开发工程师娄码通、
GPUI 插件扩展工程师柯展之、构建发布工程师常通关），收集各自结论后汇编。

**实际结果：六名成员全部未返回实质结论**（专家团引擎返回 `disabled`，max-tokens / error）。
按团队规则「没有可用的成员结果，不得据此编造实质性汇总结论」，本文件改为：

1. 基于台账 `docs/development/gpui-gap-evidence-2026-10.md` 的已有证据，对「缺口以未接线为主」
   做定量检验，完成 s2 的检验目标。
2. 逐条标注覆盖缺口——每个维度说明本应由哪位成员回答、当前缺什么。
3. 不编造成员意见。

---

## 1. 检验「缺口以未接线为主」

### 检验方法

把 GPUI 相对旧 Windows 端的缺口按成因分三类：

- **A. 已实现但未接线**：在 `rust/lithe-gpui` 中有实现代码（函数、数据结构、枚举变体），
  但零外部调用点、标了 `#[allow(dead_code)]`、或菜单入口是 `NotWired`。
- **B. 完全未实现**：在 `rust/lithe-gpui` 全目录检索关键词命中 0，没有任何实现入口。
- **C. 平台专属**：刻意只属于某一端，不以跨平台对齐为目标。

### 检验结果

#### A 类（已实现但未接线）—— 15 个缺口组

| 缺口 | 实现位置 | 未接线证据 | 调用点检索 |
|---|---|---|---|
| 跳转历史后退/前进 | `editor_view.rs:1083` go_back、`:1094` go_forward；`navigation.rs:310/328` JumpHistory | 两个函数都标 `#[allow(dead_code)]`；全仓检索 `GoBack\|GoForward` 只命中 i18n 文案（`i18n.rs:45-48`）与 menu_bar 未接线项 | 0 个 action 定义、0 个键位绑定 |
| Spring 面板 | `spring_panel.rs`（54 行）、`spring.rs`（360 行）、`spring_paths.rs`（342 行） | `right_tool_window.rs:47-52` 的 `from_rail_index` 不映射 Spring（只映射 1→Notifications, 2→Maven）；测试 `rail_index_maps_to_view` 确认 index 3 返回 None | 枚举变体存在但 rail 上点不出来 |
| Git merge/rebase/cherry-pick | `git/src/remote.rs:45` IntegrationOperation、`:515` integration_gate | 无 UI 消费者；`gpui-parity-plan-2026-10.md:280` 自述此项 | 全仓 `IntegrationOperation` 只在 git crate 内部与测试出现 |
| Git 身份配置页 | `git/src/identity.rs:104` parse_setup（标 `#[allow(dead_code)]`） | `gpui-parity-plan-2026-10.md:283` 自述 set_git_identity_host 零调用 | 全仓 `set_git_identity_host` 0 命中 |
| 清除断点 | `workbench/src/debug.rs:1590` clear_breakpoints、`workspace.rs:1533`（标 `#[allow(dead_code)]`） | `workspace.rs:1534` 只自调 `self.debug.update(cx, \|pane, cx\| pane.clear_breakpoints(cx))`，无外部触发入口 | 全仓 `.clear_breakpoints` 只命中 workspace 内部 |
| 编辑器搜索栏 | `explorer/src/explorer_view.rs:348` toggle_search、`:363` clear_search（都标 `#[allow(dead_code)]`） | toggle_search 只被 clear_search 内部调（`:358`），无外部入口 | 全仓 `.toggle_search` 只命中 explorer_view 内部 |
| 调试过滤器开关 | `workbench/src/debug.rs:24` SET_BREAKPOINTS_COMMAND（标 `#[allow(dead_code)]`）、`:358` SteppingFiltersToggle | SET_BREAKPOINTS_COMMAND 在 debug.rs 内部被调（`:1646`、`:1664`、`:2665`）但整个 debug 会话从 workspace 层是否有入口触发存疑 | — |
| 保存全部/另存为/还原文件 | `menu_bar.rs:625/620/630` 三个 NotWired | 菜单入口存在但点击走 NotWired | — |
| 新建标签页/新建文件 | `menu_bar.rs:600/606` 两个 NotWired | 菜单入口存在但点击走 NotWired | — |
| 格式化文档/选区 | `menu_bar.rs:709/714/793/798/803/808` 六个 NotWired | MISSING_LSP_REQUESTS | — |
| 切换注释/复制行/删除行/上下移行 | `menu_bar.rs:676/682/689/694/699/704` 六个 NotWired | MISSING_EDITOR_FEATURES | — |
| 分屏/缩略图/行号/空白字符 | `menu_bar.rs:742/747/753/758` 四个 NotWired | MISSING_EDITOR_FEATURES | — |
| 后退/前进导航 | `menu_bar.rs:781/786` 两个 NotWired | MISSING_EDITOR_ENTRY（见上方 go_back/go_forward） | — |
| 终端分屏 | `menu_bar.rs:831/836` 两个 NotWired | MISSING_TERMINAL_SPLIT | — |
| 帮助/关于/快捷键视图/检查更新 | `menu_bar.rs:870/896/901/906/911/917/922/928` 八个 NotWired | MISSING_HELP_ABOUT + MISSING_SHORTCUTS_VIEW + MISSING_UPDATER | — |

#### B 类（完全未实现）—— 12 个能力域

| 能力域 | 检索关键词 | 命中 | 对照端位置 |
|---|---|---|---|
| AI 对话面板（会话/历史/providers） | `ai_chat`/`AiChat` | 0 | `windows/tauri/src/features/ai`（113 文件）；macOS `Views/Agent/` |
| MyBatis | `mybatis`/`MyBatis` | 0 | `windows/tauri/src/features/mybatis`（7 文件） |
| 内嵌查看器（图片/PDF/etc） | `viewer` | 0 | `windows/tauri/src/features/viewer`（30 文件） |
| 本地历史 | `local-history`（连字符） | 0 | `windows/tauri/src/features/local-history`（5 文件） |
| 运行操作菜单 | `run-actions`/`runActions` | 0 | `windows/tauri/src/features/run-actions`（8 文件） |
| 新手引导 | `onboarding` | 0 | `windows/tauri/src/features/onboarding`（5 文件） |
| 日志面板 | `logging` | 0 | `windows/tauri/src/features/logging`（6 文件） |
| Host API 扩展契约 | `host-api`/`HostApi` | 0 | `windows/tauri/src/features/host-api`（6 文件） |
| 社区（LINUX DO 插件市场） | `community`/`Community` | 0 | 矩阵 community-auth / community-feed（两端均 missing） |
| MCP 连接 | `mcp`/`MCP` | 0 | 矩阵 ide-mcp-connection（两端 implemented，但 GPUI 侧无实现入口） |
| Notebook | `notebook` | 0 | — |
| Vim 模式 | `vim`（仅 terminal constants 1 处） | 1（非编辑器 Vim 模式） | 矩阵 vim-mode（Windows implemented） |

#### C 类（平台专属）—— 19 条

直接引用台账 §1 的 `platform-specific` 条目，共 19 条（macOS 9 条 + Windows 10 条），
包括 macOS 共享控件（dropdowns/scrollbar/tooltip/font）、Windows 任务栏/Java 菜单/WSL 等。
这些不以跨平台对齐为目标。

### 统计

**两种口径给出相反结论，必须分开说：**

| 口径 | A（已实现但未接线） | B（完全未实现） | 结论 |
|---|---|---|---|
| **能力域**（本文件上面列的 15 vs 12） | 15 域 | 12 域 | A 占 55.6%（15/27） |
| **缺口行**（最终文档 §1 的 113 行，按 GPUI 现状列自报） | 37 行（未接线 33 + 已实现未接线 4） | 52 行（缺失） | **B 占 46.0%，A 占 32.7%** |

**退回说明（2026-10-12 修正）**：本文件最初只报了「能力域」口径的 55.6% 并据此判 A 类过半，
这是**选择性口径**——15 个域与 12 个域不是可比单位（一个域可能含多行缺口，例如「AI 对话」
的 3 行被归入 1 个域）。按最终文档逐行自报的现状列，占多数的是 **B 类（缺失 52 行）**，
不是 A 类。因此本条判断（「缺口以未接线为主」）在行口径下**不成立**，
在域口径下成立；两种口径都写出来，不下单一结论。

#### 逐项核验（支撑行口径）

「未接线」的 33 行 + 「已实现未接线」的 4 行共 37 行，其中可确证「实现已在、只缺调用点」的是：

| 缺口 | 已确证存在实现 | 证据 |
|---|---|---|
| Spring 面板（#46） | 是 | `spring_panel.rs` 54 行 + `spring.rs` 360 行 + `spring_paths.rs` 342 行全在；`right_tool_window.rs:47` `from_rail_index` 只映射 1/2 |
| 跳转历史前进/后退（#11/#12） | 是 | `editor_view.rs:1083/1094` 标 `#[allow(dead_code)]`；`navigation.rs:310/328` 的 `JumpHistory` |
| Git merge/rebase/cherry-pick（#58） | 是 | `remote.rs:45` `IntegrationOperation` + `:515` `integration_gate` |
| 清除断点（#70） | 是 | `debug.rs:1590` + `workspace.rs:1533`（标 `#[allow(dead_code)]`） |
| 编辑器搜索栏（#5） | 部分 | `lithe-ui/src/editor/find.rs:60` 有 `FindWidget`，`CodeEditor` 未接 |
| 切换注释（#16） | 部分 | `lithe-ui` 检索 `toggle_comment` 5 处、`toggleComment` 3 处 |
| 缩略图（#21） | 部分 | `lithe-ui` 检索 `minimap` 31 处；宿主未开 |
| 行号（#22）/空白字符（#23） | 部分 | `lithe-ui` 检索 `line_number` 4 处、`whitespace` 96 处 |
| 移动行（#19） | 部分 | `lithe-ui` 检索 `move_line` 3 处 |

但同一批 `MISSING_EDITOR_FEATURES` 里也有**检索为 0** 的：`duplicate_line`（#17 复制行）、
`delete_line`（#18 删除行）、`move_line_up`（#19 上移行）、`render_whitespace` 均为 0 命中。
所以「未接线」这一列是**混合类**——同一组里既有「实现已在」也有「完全没写」，
不能整体归为「已实现未接线」。

**修正后的判定**：按缺口行数，**「缺口以未接线为主」不成立**（B 类缺失 52 行 > A 类 37 行）；
按能力域，A 类 15 > B 类 12。本文件不据此下单一结论，两个口径都保留。
**独立评估者已据此推翻该判断**（2026-10-12），最终文档采用行口径。

---

## 2. 覆盖缺口（专家团失败的维度）

以下维度应由对应成员回答，但专家团未返回结果。s3 汇编时由主理人基于台账补写，
标注为「主理人基于台账补写，未经专家核验」。

| 成员 | 应回答的维度 | 成员状态 | s3 补写来源 |
|---|---|---|---|
| 陆井然（界面设计师） | 界面布局/信息架构/交互/键位缺口 | 未返回 | 台账 §2 + `menu_bar.rs` + `keymap.rs` |
| 颜思齐（UI 设计师） | 设计系统/配色/图标/视觉规范 | 未返回 | 台账 §1.6 平台专属条目 + `assets.rs` |
| 闻本立（编辑器架构师） | 内核架构/文本模型/LSP-DAP 集成 | 未返回 | 台账 §2.2-2.3 + `crates/editor` 目录 |
| 娄码通（核心开发工程师） | Rust 实现/并发/性能优化 | 未返回 | 台账 §2 + `allow(dead_code)` 分布 |
| 柯展之（插件扩展工程师） | 扩展目录/管理页/插件契约 | 未返回 | `crates/extensions` + `extensions_page.rs` |
| 常通关（构建发布工程师） | CI/CD/三平台打包签名 | 未返回 | 台账 §3 + 工作流/脚本 |

---

## 3. 待确认项（跨职责）

1. **矩阵是否新增 gpui 平台字段**：`gpui-parity-plan-2026-10.md:344-348` 把此列为待确认。
   这影响 s3 排期表里每条验收方式是「改矩阵 JSON」还是「只改代码」。
2. **macOS 键位档**：`keymap.rs:62` 写着「不做 macOS 档」，但选型笔记说目标是覆盖 macOS。
   这是否需要在排期里加一个独立的键位档阶段？
3. **IDEA Classic / Eclipse 两档键位**：`keymap.rs:44` 的 `ECLIPSE_BINDINGS` 为空，
   macOS端有这两份表（`macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift` 的
   `ideaClassicBindings` / `eclipseBindings`）。是否需要对齐？
4. **Spring 面板能否在 macOS 端验证**：实现已在 GPUI 侧但 rail 不映射，这是接线问题还是
   刻意不接（等 Spring Boot 项目探测器）？
