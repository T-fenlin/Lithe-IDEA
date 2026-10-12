# GPUI 统一前端：相对 Windows 与 macOS 的缺口、优化项与功能排期（2026-10）

> 基线提交 `3085f5d1`，分支 `feat/menu-bar-opaque`，盘点日期 2026-10-12。
> 证据台账见 [GPUI 三端缺口证据台账](gpui-gap-evidence-2026-10.md)；
> 专家团交接与覆盖缺口见 [GPUI 缺口盘点专家团交接文件](gpui-gap-team-handoff-2026-10.md)。

## 先说结论

**GPUI 端 = `rust/lithe-gpui`（13 个宿主 crate、`crates/` 下 123 个 `.rs`）；对照端 =
`windows/tauri`（Windows 发行版）与 `macos/`（macOS 发行版）。**

四句话：

1. **对照基准的 92 项这个数要重新算，前版的「112 − 20 = 92」不成立。** 矩阵 Windows 列
   `implemented` 确实是 112 项，但落在 `windows/tauri/src/config/backend-capabilities.ts`
   7 个 `false` 开关组内的**只有 10 项**且这 10 项在矩阵里都被记成 `implemented`
   （`database-connections`/`-crud-export`/`-query-history`/`-schema`、`docker-compose`、
   `github-prs`/`-reviews-comments`、`remote-workspace`、`wsl-paths`/`-boundaries`）。
   另外 20 条 `agent-*` 在 Windows 列是 **11 missing + 9 partial**，本来就不是 implemented。
   所以可核实的折算只能是 **112 − 10 = 102 项**；「92 项」是既有计划
   `gpui-parity-plan-2026-10.md:5-10` 自己的口径，其算法未在该文档中给出，本文件不复述它。
   排期 §3.3 的排除线仍按「旧端自己也是待开发」这条**理由**执行，不按 92 这个**数**。
2. **缺口按行数算，「完全缺失」是最大一类。** 113 行缺口按 GPUI 现状列自报：
   **缺失 52 行（46.0%）、未接线 33 行 + 已实现未接线 4 行 = 37 行（32.7%）**、部分 10 行、
   有 5 行、未知 3 行、骨架 2 行、平台专属 2 行、阻塞 1 行、已接线（更正项）1 行。
   **注意口径**：若改按「能力域」而非「行」分类，两类数量接近（12 个完全无实现的域
   vs 15 个含未接线项的域，见交接文件 §1），且同一组 `MISSING_EDITOR_FEATURES` 内部
   既有「实现已在」也有「完全没写」。**两种口径都不支持「缺口以未接线为主」这个单一结论，
   本文件不采用它。**
   连文案键都没有的真空白是 5 个：MyBatis（#66）、Host API（#99）、社区（#102）、
   MCP（#98）、Notebook；另有 6 个**有文案键无实现代码**：内嵌查看器（#109）、新手引导（#111）、
   日志面板（#112）、运行操作（#110）、本地历史（#78）、AI 对话（#83–#85）——见 §4 第 8 条。
3. **最大的结构性缺口不在功能，而在构建链路：GPUI 端在 18 个 CI 工作流里检索命中 0。**
   Windows 侧只有 `scripts/sign-windows.ps1` 一个引用，macOS 侧**完全没有**产出 `lithe-gpui`
   产物的入口。这意味着 macOS 端任何 GPUI 验收在当前仓库里都做不了——这必须排在排期最前面。
4. **本月可完成的是「止血 + 编辑器基本功 + Git 主动操作 + 语义导航」**，也就是把矩阵 Windows 列
   112 项 `implemented` 中经旧端能力开关折算后的 **102 项**（112 − 10，见第 1 点）继续推进；
   AI 对话、数据库、Docker、远程、插件市场等按「旧端自己也是待开发，平替无参照」明确排除。
   既有计划记的「已具备 23 / 降级或未接线 23 / 完全缺失 46」三项之和不等于 92，
   且 92 的算法未在其文档中给出，本文件不沿用该数。

---

## 1. 缺口清单

### 1.1 编辑器（Editor）

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 1 | 新建文件/文件夹 | 缺失 | 有 | 有 | `explorer/src/explorer_view.rs`（只有 Open/复制路径/刷新）；`menu_bar.rs:606` NotWired MISSING_FILE_LIFECYCLE |
| 2 | 重命名文件 | 缺失 | 有 | 有 | 同上，菜单 `MISSING_FILE_LIFECYCLE`（定义在 `menu_bar.rs:119`） |
| 3 | 删除文件 | 缺失 | 有 | 有 | 同上 |
| 4 | 新建标签页 | 未接线 | 有 | 有 | `menu_bar.rs:600` NotWired MISSING_FILE_LIFECYCLE |
| 5 | 编辑器内查找 | 未接线 | 有 | 有 | `lithe-ui/src/editor/find.rs:60` 有 `FindWidget`，`CodeEditor` 未接；`menu_bar.rs:503`、`:2175` 注释自述 |
| 6 | 查找替换 | 未接线 | 有 | 有 | 同 #5；`menu_bar.rs:506` 的 `Self::Find | Self::FindAndReplace => return None`（`keybind_action` 查不到动作），入口在 `:673/674` 是 `MenuItem::Action` 但点下去无行为 |
| 7 | 在文件中替换（apply） | 部分 | 有 | 有 | `replace_in_files.rs:753` 只有 `REPLACE_PREVIEW_COMMAND`；Core `protocol/command.rs` 无 apply 命令 |
| 8 | 保存全部 | 未接线 | 有 | 有 | `menu_bar.rs:625` NotWired MISSING_EDITOR_ENTRY |
| 9 | 另存为 | 未接线 | 有 | 有 | `menu_bar.rs:620` NotWired MISSING_EDITOR_ENTRY |
| 10 | 还原文件 | 未接线 | 有 | 有 | `menu_bar.rs:630` NotWired MISSING_EDITOR_ENTRY |
| 11 | 跳转历史后退 | 已实现未接线 | 有 | 有 | `editor/src/editor_view.rs:1083` `go_back` 标 `#[allow(dead_code)]`；`navigation.rs:310` `JumpHistory`；全仓无 action/键位 |
| 12 | 跳转历史前进 | 已实现未接线 | 有 | 有 | `editor_view.rs:1094` `go_forward`；同上 |
| 13 | 后退/前进菜单入口 | 未接线 | 有 | 有 | `menu_bar.rs:781/786` NotWired MISSING_EDITOR_ENTRY |
| 14 | 大纲/结构面板 | 缺失 | 有 | 部分 | matrix `outline-symbols`（mac=partial, win=implemented）；`right_tool_window.rs` 无 Outline 变体 |
| 15 | 面包屑 | 缺失 | 有 | 有 | `editor` crate 无对应文件 |
| 16 | 切换注释 | 未接线 | 有 | 有 | `menu_bar.rs:676` NotWired MISSING_EDITOR_FEATURES |
| 17 | 复制行 | 未接线 | 有 | 有 | `menu_bar.rs:689` NotWired MISSING_EDITOR_FEATURES |
| 18 | 删除行 | 未接线 | 有 | 有 | `menu_bar.rs:694` NotWired MISSING_EDITOR_FEATURES |
| 19 | 上移/下移行 | 未接线 | 有 | 有 | `menu_bar.rs:699/704` NotWired MISSING_EDITOR_FEATURES |
| 20 | 分屏 | 未接线 | 有 | 有 | `menu_bar.rs:742` NotWired MISSING_EDITOR_FEATURES |
| 21 | 缩略图（minimap） | 未接线 | 有 | 有 | `menu_bar.rs:747`；`lithe-ui` 有 `minimap()` builder，宿主未开 |
| 22 | 行号开关 | 未接线 | 有 | 有 | `menu_bar.rs:753` |
| 23 | 空白字符显示 | 未接线 | 有 | 有 | `menu_bar.rs:758` |
| 24 | 文件编码切换 | 部分 | 有 | 有 | matrix `editor-file-encoding-reopen`/`-save`；状态栏 UTF-8 是硬编码字符串 |
| 25 | 行尾符切换 | 缺失 | 有 | 有 | 同上 |
| 26 | Markdown 预览大纲 | 缺失 | 缺失 | 有 | matrix `markdown-preview-outline`（mac=implemented, win=missing） |
| 27 | Vim 模式 | 缺失 | 有 | 缺失 | matrix `vim-mode`（mac=missing, win=implemented）；GPUI 检索 `vim` 仅 `terminal/src/constants.rs` 1 处 |

### 1.2 语义与 LSP（Language）

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 28 | 重命名符号 | 未接线 | 有 | 有 | `menu_bar.rs:808` NotWired MISSING_LSP_REQUESTS |
| 29 | 查找引用 | 未接线 | 有 | 有 | `menu_bar.rs:803` NotWired MISSING_LSP_REQUESTS |
| 30 | 转到实现 | 未接线 | 有 | 有 | `menu_bar.rs:793` NotWired MISSING_LSP_REQUESTS |
| 31 | 转到类型定义 | 未接线 | 有 | 有 | `menu_bar.rs:798` NotWired MISSING_LSP_REQUESTS |
| 32 | 格式化文档 | 未接线 | 有 | 有 | `menu_bar.rs:709` NotWired MISSING_LSP_REQUESTS |
| 33 | 格式化选区 | 未接线 | 有 | 有 | `menu_bar.rs:714` NotWired MISSING_LSP_REQUESTS |
| 34 | 参数提示 | 未接线 | 有 | 有 | `menu_bar.rs:682` NotWired MISSING_LSP_REQUESTS |
| 35 | 诊断波浪线 | 骨架 | 有 | 有 | `editor/src/diagnostics.rs`；依赖 jdtls，缺失时 `S1_JAVA_FAILED` |
| 36 | 代码动作/快速修复 | 骨架 | 有 | 有 | `editor/src/code_actions.rs`；同上 |
| 37 | 补全/hover/跳转内置回退 | 有 | 有 | 有 | `completion.rs:69`、`hover.rs:103`、`navigation.rs:22` 有非 LSP 回退 |
| 38 | PHP 运行与测试 | 部分 | 部分 | 有 | matrix `php-run-test`（win=partial） |

### 1.3 工作台与界面（Workbench / UI）

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 39 | 关闭项目 | 未接线 | 有 | 有 | `menu_bar.rs:613` NotWired MISSING_PROJECT_LIFECYCLE |
| 40 | 关闭窗口 / 退出 | 未接线 | 有 | 有 | `menu_bar.rs:642/654` NotWired MISSING_WINDOW_ROUTING |
| 41 | 快捷键视图 | 未接线 | 有 | 有 | `menu_bar.rs:870/901` NotWired MISSING_SHORTCUTS_VIEW |
| 42 | 帮助 / 关于 | 未接线 | 有 | 有 | `menu_bar.rs` 的 `lithe.menu.help` 子菜单下 7 个 NotWired 起始行：**896**（文档，MISSING_HELP_ABOUT）、**901**（快捷键，MISSING_SHORTCUTS_VIEW）、**906**（新增功能）、**911**（变更日志）、**917**（报告问题）、**922**（请求功能）、**928**（检查更新，MISSING_UPDATER） |
| 43 | 更新器 | 未接线 | 有 | 有 | `menu_bar.rs:928` NotWired MISSING_UPDATER |
| 44 | 更新失败回滚 | 未知 | 有 | 部分 | matrix `updates-failure-rollback`（mac=partial, win=implemented） |
| 45 | 终端分屏 | 未接线 | 有 | 有 | `menu_bar.rs:831/836` NotWired MISSING_TERMINAL_SPLIT |
| 46 | Spring 面板 | 已实现未接线 | 有 | 有 | `spring_panel.rs`（54 行）+ `spring.rs`(360) + `spring_paths.rs`(342) 全部实现；`right_tool_window.rs:47` `from_rail_index` 不映射 Spring |
| 47 | IntelliJ Islands 外观 | 未知 | 有 | 部分 | matrix `workbench-islands-appearance`（mac=partial, win=implemented） |
| 48 | 项目颜色渐变 | 未知 | 有 | 部分 | matrix `workbench-project-color-gradient`（mac=partial, win=implemented） |
| 49 | macOS 共享下拉控件 | 缺失 | 平台专属 | 有 | matrix `macos-shared-product-dropdowns`（mac=platform-specific） |
| 50 | macOS 滚动条样式 | 缺失 | 平台专属 | 有 | matrix `macos-shared-scrollbar-style` |
| 51 | macOS 悬浮提示样式 | 缺失 | 平台专属 | 有 | matrix `macos-shared-hover-tooltip-style` |
| 52 | macOS 默认 UI 字体 | 缺失 | 平台专属 | 有 | matrix `macos-bundled-default-ui-font` |
| 53 | 侧栏标题/展开图标 | 缺失 | 平台专属 | 有 | matrix `project-sidebar-header-icons` / `-tree-disclosure-icons` |
| 54 | macOS IME 呈现 | 缺失 | 平台专属 | 有 | matrix `editor-macos-ime-presentation`（win=platform-specific） |
| 55 | 菜单栏紧凑档 | 有 | 平台专属 | 平台专属 | matrix `workbench-compact-menu-bar`（mac=platform-specific, win=implemented）；`MenuBarMode` 四档已实现 |

### 1.4 版本控制（Git）

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 56 | 新建分支 | 未接线 | 有 | 有 | `log_view.rs:1048/1099` 工具栏按钮 `enabled=false` + 空闭包 |
| 57 | 提交 diff 查看 | 未接线 | 有 | 有 | `log_view.rs:1512` 图标按钮 + 空 handler |
| 58 | 主动 merge/rebase/cherry-pick | 已实现未接线 | 有 | 有 | `remote.rs:45` `IntegrationOperation` + `:515` `integration_gate` 齐备；但在 `workbench`/`explorer`/`settings`/`editor` 四个 UI crate 内检索这两个符号**命中 0**，即无 UI 消费者 |
| 59 | 标签管理 | 缺失 | 有 | 有 | 需新增 |
| 60 | Stash | 缺失 | 有 | 有 | 只有 checkout 的 `autoStash` 开关（`remote.rs:88`） |
| 61 | Git 身份配置页 | **已接线**（非缺口，列出以更正前版误记） | 有 | 有 | host 在生产路径注入：`workspace.rs:883` `set_git_identity_host(Some(GitIdentityHost {..}))`（该文件 `cfg(test)` 起点 `:3395`），消费者 `settings/src/dialog.rs:264`。仅 `git/src/identity.rs:104` 的 `parse_setup` 标 `#[allow(dead_code)]`，只在同文件测试（`:218/241/267`，`cfg(test)` 起点 `:176`）里被调用 |
| 62 | Git gutter 行内操作 | 部分 | 有 | 有 | matrix `editor-git-gutter-peek`/`-stage`/`-setting` |

### 1.5 Java 工具链

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 63 | 执行 Maven 目标 | 缺失 | 有 | 有 | 设置页文案自述「gpui 侧还没有执行 Maven 的通路」 |
| 64 | 依赖树与 Profile | 缺失 | 有 | 有 | `maven.rs`(271 行) 只有 reactor 扫描与模块树 |
| 65 | Maven settings 直达语言服务 | 缺失 | 有 | 有 | 需打通 |
| 66 | MyBatis 导航 | 缺失 | 有 | 有 | `crates/` 下全文件检索 `mybatis`/`MyBatis` **命中 0**（文案键也没有）；旧端 `features/mybatis`（7 文件） |
| 67 | Java main / 测试运行 | 缺失 | 有 | 有 | Core 有 `debug.javaTestLaunch`，gpui 侧零引用 |
| 68 | Spring 端点导航 | 未接线 | 有 | 有 | 面板实现见 #46，跳转控制器方法仍需接 |
| 69 | New Java 菜单（Windows 专属） | 平台专属 | 有 | 平台专属 | matrix `windows-java-new-menu` |

### 1.6 运行与调试

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 70 | 断点（设置/清除） | 部分 | 部分 | 有 | `debug.rs:1590` `clear_breakpoints` 无外部触发；`workspace.rs:1533` 标 `#[allow(dead_code)]`，只自调 |
| 71 | 调试会话状态 | 部分 | 部分 | 有 | matrix `debug-state` 两端列 win=partial |
| 72 | 调试过滤器开关 | 部分 | 有 | 有 | `debug.rs:24` `SET_BREAKPOINTS_COMMAND` 标 `#[allow(dead_code)]`；`:358` 步进过滤器 |
| 73 | 启动调试会话 | 阻塞 | 有 | 有 | `workspace.rs:1388` `start_debug_session` 首步要 `service.lsp_session_id()`，拿不到即 `status=blocked reason=no-java-service` |
| 74 | Docker / Compose | 缺失 | 有 | 部分 | matrix `docker-compose`（mac=partial, win=implemented）；GPUI 检索 `docker` 仅图标资源 |

### 1.7 工作区与文件

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 75 | 克隆仓库 | 未接线 | 有 | 有 | `menu_bar.rs:173` `NOT_WIRED_OUTSIDE_MENUS` 指向 MISSING_CLONE_UI |
| 76 | 新建项目脚手架 | 未接线 | 有 | 有 | `menu_bar.rs:176` NOT_WIRED_OUTSIDE_MENUS 指向 MISSING_NEW_PROJECT_SCAFFOLDING |
| 77 | 项目树批量选择与移动 | 部分 | 缺失 | 有 | matrix `workspace-tree-batch-files`（mac=implemented, win=missing）；`explorer` 只有 `SelectAllInTree`/`CopySelection` |
| 78 | 本地历史 | 缺失 | 有 | 有 | **实现代码口径**：GPUI `crates/` 下 `.rs` 检索 `local-history` 命中 0。**全文件口径**：本地化里有 4 处文案键（`locales/lithe.en.yml:7102`、`lithe.zh-CN.yml:7102`、`locales/source/locale.ts:3662`、`:7981` 的 `commandPalette.actions.file-local-history.label`），无实现代码。`menu_bar.rs:635` NotWired MISSING_LOCAL_HISTORY；旧端 `features/local-history`（5 文件） |
| 79 | 远程工作区 | 缺失 | 有 | 缺失 | matrix `remote-workspace`（mac=missing, win=implemented）；GPUI 检索 `remote` 全部命中在 git crate 内部 |
| 80 | WSL 路径与边界 | 缺失 | 有 | 平台专属 | matrix `wsl-paths`/`wsl-boundaries`；GPUI 检索 `wsl` 10 处全在设置项 |
| 81 | Windows 任务栏项目标题 | 平台专属 | 有 | 平台专属 | matrix `windows-taskbar-project-titles` |
| 82 | 依赖浏览器 | 有 | 有 | 有 | matrix `dependency-browser` 两端 implemented |

### 1.8 AI / Agent / 扩展 / 其他子系统

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据路径 |
|---|---|---|---|---|---|
| 83 | AI 对话会话 | 缺失 | 有 | 缺失 | **实现代码口径**：`crates/` 下 `.rs` 检索 `ai_chat`/`AiChat`/`aiChat` 命中 0。**全文件口径**：本地化里 8 处文案键（`layout.resizeAiChat`、`keybindings.commands.workbench.toggleAIChat.title`，`lithe.en.yml:6912/7838` 等），无实现代码。matrix `ai-chat-session`（mac=missing, win=implemented） |
| 84 | AI 对话历史 | 缺失 | 有 | 缺失 | matrix `ai-chat-history` |
| 85 | AI 对话 providers | 缺失 | 有 | 缺失 | matrix `ai-chat-providers` |
| 86 | Agent 待办/计划展示 | 缺失 | 缺失 | 有 | matrix `agent-plan-display`（win=missing）等 11 条 AI 能力 Windows 列 missing |
| 87 | Agent 审批模式呈现 | 缺失 | 缺失 | 有 | matrix `agent-approval-mode-presentation` |
| 88 | Agent 对话历史 | 缺失 | 缺失 | 有 | matrix `agent-conversation-history` |
| 89 | Agent 入口可发现性 | 缺失 | 缺失 | 有 | matrix `agent-entry-discoverability` |
| 90 | Agent 模式同步 | 缺失 | 缺失 | 有 | matrix `agent-mode-sync` |
| 91 | Agent 推理展示 | 缺失 | 缺失 | 有 | matrix `agent-reasoning-display` |
| 92 | Agent 斜杠命令 | 缺失 | 缺失 | 有 | matrix `agent-slash-commands` |
| 93 | Agent 思考等级标签 | 缺失 | 缺失 | 有 | matrix `agent-thinking-level-labels` |
| 94 | Agent 工具活动时间线 | 缺失 | 缺失 | 有 | matrix `agent-tool-activity-timeline` |
| 95 | Agent 工具栏品牌标识 | 缺失 | 缺失 | 有 | matrix `agent-toolbar-brand-marks` |
| 96 | Agent 连接模型切换 | 缺失 | 缺失 | 有 | matrix `agent-connection-model-switching` |
| 97 | AI 提交信息生成 | 有 | 有 | 有 | matrix `ai-commit-generation` 两端 implemented；`docs/development/windows-ai-commit.md` |
| 98 | MCP 连接 | 缺失 | 有 | 有 | `crates/` 下全文件（`.rs`/`.yml`/`.ts`）检索 `mcp`/`MCP` **命中 0**（文案键也没有）；matrix `ide-mcp-connection` 两端 implemented |
| 99 | Host API（插件宿主契约） | 缺失 | 有 | 有 | `crates/` 下全文件检索 `host-api`/`hostApi`/`HostApi` **命中 0**；旧端 `features/host-api`（6 文件） |
| 100 | 插件能力 API | 部分 | 有 | 有 | matrix `ide-api-plugins` 两端 implemented；`crates/extensions` 仅 10 KB |
| 101 | 扩展管理页 | 部分 | 有 | 有 | `extensions_page.rs`（652 行） |
| 102 | 社区（插件市场/动态） | 缺失 | 缺失 | 缺失 | matrix `community-auth`/`community-feed` 两端 missing；`crates/` 下全文件检索 `community`/`Community` **命中 0** |
| 103 | 数据库连接 | 缺失 | 有 | 有 | GPUI 检索 `database` 5 处全是图标资源；matrix 4 条 database-* 两端 implemented |
| 104 | 数据库 CRUD 与导出 | 缺失 | 有 | 有 | matrix `database-crud-export` |
| 105 | 数据库查询历史 | 缺失 | 有 | 有 | matrix `database-query-history` |
| 106 | 数据库 schema | 缺失 | 有 | 有 | matrix `database-schema` |
| 107 | GitHub PR | 缺失 | 有 | 有 | `menu_bar.rs:735` NotWired MISSING_GITHUB；matrix `github-prs` 两端 implemented |
| 108 | GitHub 评论与审查 | 缺失 | 有 | 有 | matrix `github-reviews-comments` |
| 109 | 内嵌查看器（图片/PDF/Web） | 缺失 | 有 | 有 | **实现代码口径**：`crates/` 下 `.rs` 检索 `viewer` 命中 0。**全文件口径**：本地化里有 207 处文案键（`locale.ts` 100、`lithe.en.yml` 57、`lithe.zh-CN.yml` 50，如 `panes.databaseViewer`、`commandPalette.actions.open-web-viewer.label`、`viewer.enterUrl`），无实现代码。旧端 `features/viewer`（30 文件） |
| 110 | 运行操作菜单（run actions） | 缺失 | 有 | 有 | **实现代码口径**：`crates/` 下 `.rs` 检索 `run-actions`/`runActions` 命中 0。**全文件口径**：本地化里 68 处文案键（`locale.ts` 34、`lithe.zh-CN.yml` 17、`lithe.en.yml` 17），无实现代码。旧端 `features/run-actions`（8 文件） |
| 111 | 新手引导 | 缺失 | 有 | 有 | **实现代码口径**：`crates/` 下 `.rs` 检索 `onboarding` 命中 0。**全文件口径**：本地化里有 125 处文案键（`lithe.en.yml:7218` `commandPalette.actions.open-onboarding.label`、`:8532-8590` 整组 `onboarding.*`，含 keybindingPreset 各预设、enableVimMode、importSettings），无实现代码。旧端 `features/onboarding`（5 文件） |
| 112 | 日志面板 | 缺失 | 有 | 有 | **实现代码口径**：`crates/` 下 `.rs` 检索 `logging` 命中 0。**全文件口径**：本地化里有 13 处文案键（`lithe.en.yml` 6、`locale.ts` 6、`lithe.zh-CN.yml` 1，如 `settings.logs.diagnosticMode`、`settings.logs.fallback.logging_unavailable`），无实现代码。旧端 `features/logging`（6 文件） |
| 113 | 可观测性（诊断导出等） | 有 | 有 | 有 | matrix 4 条 可观测性 两端 implemented；GPUI 检索 `diagnostics` 353 处 |

**缺口清单共 113 行。**

---

## 2. 优化项

以下不是「缺功能」，而是已经拖后腿、必须在排期里占位置的部分。每项写明现状证据与目标。

### 2.1 构建链缺失（最高优先级，不是功能问题）

**现状证据**：`.github/workflows/` 18 个工作流逐个检索 `lithe-gpui`，**命中 0**。
`scripts/` 下唯一引用是 `scripts/sign-windows.ps1:3` 的默认参数
（`BinaryDirectory = lithe-gpui-target/debug`）。`scripts/build-windows.ps1` 构建的是
`windows/tauri`（`$windowsApp = Join-Path $root "windows/tauri"`，第 11 行），
`scripts/build-macos.sh:64` 只调 `build-rust-core.sh` 构建 `lithe-core`，
`rust/lithe-gpui/Cargo.toml` 的 `[profile.*]` 无平台条件，`crates/app/Cargo.toml` 唯一的
平台条件是 `[target.'cfg(windows)'.build-dependencies] embed-resource = "3"`。

**目标**：Windows 与 macOS 各有一条能产出 `lithe-gpui` 产物并签名的 CI 工作流；
Windows 侧复用 `sign-windows.ps1`，macOS 侧需要新建（对照 `release-macos.yml` 的结构）。

**为什么排在功能之前**：没有链路 ⇒ 没有可发布的产物 ⇒ macOS 端任何 GPUI 验收都无法执行，
矩阵回填也无从谈起。

### 2.2 构建时长 15 分钟/次

**现状证据**：`docs/development/gpui-parity-plan-2026-10.md:20` 记录
「`cargo build -p lithe-gpui-app` 会级联重编整条依赖链（2026-10-11 实测 14m48s，
`lithe-ui` 一改就全链路重编）」，因此每天有效构建只有 3～4 次。

**依赖链证据**（各 crate 的 Cargo.toml）：`workbench` 依赖全部 12 个其他 crate；
`app` 依赖 `languages`/`lithe-ui`/`settings`/`shared`/`workbench`；`settings` 依赖
`lithe-ui`/`shared`/`run`；`editor` 依赖 `lithe-ui`/`notify`/`shared`/`java`/`languages`。
即 `workbench`（988 KB）与 `settings`（562 KB）都在依赖链末端，一改就全量重编。

**目标**：把「改一个 UI 细节」的反馈循环从 15 分钟降到 ≤5 分钟。可测量方法：
记录 `cargo build -p lithe-gpui-app` 在「只改 `lithe-ui` 一个文件」与「只改 `workbench`
一个文件」两种情形下的耗时，作为基线；再评估 `-Zthreads`、增量编译配置、
或把 `workbench` 拆分为更细粒度 crate 的收益。**该项需要先测基线再定方案**，
不建议在排期里预设结论。

### 2.3 内存制约与 target 缓存污染

**现状证据**：同文档 `:87-101` 记录「`explorer.exe` 会泄漏到 17 GB 以上，32 GB 内存被占掉
28 GB 后 rustc 会崩成 `STATUS_STACK_BUFFER_OVERRUN` 或 ICE，linker 报 `LNK1102 内存不足`；
崩溃会污染 target 缓存，后续重试变成 `E0786 invalid metadata files` /
`only metadata stub found`，必须 `cargo clean` 才能摆脱」。已知的缓解配方：
`RUST_MIN_STACK=67108864`、`CARGO_PROFILE_DEV_DEBUG=0`、`-j 2`。

**目标**：把「构建失败导致 `cargo clean` 重来」从常态降到例外。可测量方法：连续 5 次
构建的成功率与平均耗时。

### 2.4 `#[allow(dead_code)]` 掩盖的未接线面

**现状证据**：`rust/lithe-gpui` 全目录共命中 `allow(dead_code)` **19 处**。其中 **18 处**在
`crates/*/src/` 的生产代码里，**1 处**是测试辅助模块的 crate 级标注
（`crates/workbench/tests/common/mod.rs:1` 的 `#![allow(dead_code)]`）。
下面的目标与 B3 验收都按 **18 处（生产代码口径）** 计数，测试辅助模块不计入。

| 文件（生产代码） | 处数 | 代表符号 |
|---|---|---|
| `explorer/src/explorer_view.rs` | 3 | `search_open`（:85）、`toggle_search`（:348）、`clear_search`（:363） |
| `terminal/src/constants.rs` | 3 | :77、:79、:86 |
| `terminal/src/session.rs` | 3 | :298、:312、:321 |
| `editor/src/editor_view.rs` | 2 | `go_back`（:1082）、`go_forward`（:1093） |
| `editor/src/navigation.rs` | 2 | `can_go_back`（:291）、`can_go_forward`（:296） |
| `workbench/src/debug.rs` | 2 | `SET_BREAKPOINTS_COMMAND`（:24）、步进过滤器 `set`（:358） |
| `git/src/identity.rs` | 1 | `parse_setup`（:104） |
| `terminal/src/profile.rs` | 1 | :56 |
| `workbench/src/workspace.rs` | 1 | `clear_breakpoints`（:1532） |
| **生产代码小计** | **18** | |
| `workbench/tests/common/mod.rs`（测试辅助，不计入目标） | 1 | `#![allow(dead_code)]`（:1） |
| **全目录合计** | **19** | |

**目标**：生产代码口径的 `allow(dead_code)` 从 18 处降到 ≤5 处，每一处剩余项在注释里写明
「为什么现在不删」。理由：这个标注是「实现已写完但没人调」的可靠信号，清掉它就等于做掉
#11/#12/#46/#58/#70 一批缺口。
### 2.5 错误处理与 unwrap 分布

**现状证据**：把 `#[cfg(test)]` 之后的代码排除后，`rust/lithe-gpui` 非测试代码里的
`.unwrap()` / `.expect(` 共 **10 处**（`shared/src/workspace_config/test_repo.rs` 4、
`app/src/main.rs` 2、`app/build.rs` 1、`settings/src/dialog.rs` 1、`workbench/src/menu_bar.rs` 1、
`workbench/src/project_menu.rs` 1）。既有盘点里点名的「大数字」
（`keymap_io.rs` 12、`recent_projects.rs` 10、`plan.rs` 4、`changes.rs` 1）**全部落在
`#[cfg(test)]` 模块内**（各自起始行：220、344、243、483），不是生产路径问题。

**目标**：保持非测试代码 unwrap ≤10 处；新增代码遵守 `develop-lithe` SKILL 的
「Handle failures explicitly」——sharing 层如实返回 `Err`，由有上下文的调用方决定容忍度并留诊断日志。

### 2.6 测试密度不均

**现状证据**（`#[test]` / `#[gpui::test]` 计数）：

| crate | 测试数 | crate | 测试数 |
|---|---|---|---|
| workbench | 259 | explorer | 13 |
| settings | 216 | app | 12 |
| shared | 80 | terminal | 8 |
| editor | 76 | languages | 6 |
| git | 49 | extensions | 6 |
| java | 45 | — | — |
| notify | 32 | run | 16 |

`crates/extensions` 只有 6 个测试但承载 10 KB 的插件目录模型；`app`（组装根）只有 12 个。
集成测试目录只有 `crates/workbench/tests` 一处。

**目标**：`extensions` 与 `app` 的测试数翻倍；至少再有一个 crate 建立集成测试目录。
新增测试遵守 `write-stable-tests` SKILL 的有界等待与确定性时间规则。

### 2.7 键位档位缺失

**现状证据**：`crates/settings/src/keymap.rs:62` 明确写着「**不做 macOS 档**：⌘ 在 Windows
上没有意义」；`:44` 与 `:420` 记录 `ECLIPSE_BINDINGS` 为空（「macOS 那两份 Eclipse 表是 ⌘ 方言，
Windows 上不成立」）。macOS 端的真源在
`macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift` 的 `ideaClassicBindings`。
`:22-26` 记录了一条硬约定：写 `secondary-` 不写 `cmd-`，因为 `gpui::Keystroke::parse` 把
`cmd` 解析成 `modifiers.platform`（Windows 上就是 Win 键），有
`no_keystroke_uses_the_cmd_dialect` 测试守着。

**目标**：GPUI 端在 macOS 上运行时，`secondary-` 已被 gpui 归一化为 `platform`，
所以**理论上不需要单独的 macOS 档**——但这一点在 macOS 实机验证之前只是推断。
排期里作为「等构建链路建好后第一次 macOS 实机验收」的检查项。

### 2.8 工具脚本与布局未适配

**现状证据**：`rust/lithe-gpui/README.md` 自述「`tools/check-ui-px.mjs` 等脚本是从既有分支
一起搬来的，**其中的路径与 crate 名仍是旧布局**（它排除的 `VENDORED_CRATE = "lithe-gpui"`
现在已改名叫 `lithe-ui`），尚未适配。」

**目标**：`tools/` 下脚本的路径与 crate 名与当前布局一致，且能在 CI 里跑。

### 2.9 crates 数量与文档不一致

**现状证据**：`rust/lithe-gpui/README.md` 写「`crates/*`：12 个宿主 crate（`app` / `workbench` /
`editor` / `explorer` / `git` / `java` / `terminal` / `settings` / `run` / `notify` / `shared` /
`extensions`）」，但实际 `crates/` 下有 **13 个**目录：app、editor、explorer、extensions、git、
java、languages、notify、run、settings、shared、terminal、workbench —— README 漏了 `languages`。
`rust/lithe-gpui/Cargo.toml` 的注释同样写「12 个」。

**目标**：README、workspace 注释与实际目录三者一致。

### 2.10 矩阵缺 gpui 平台字段

**现状证据**：`shared/platform-feature-matrix/features/*.json` 只有 `macos` 与 `windows`
两列；`metadata.json` 的 `statusDefinitions` 也只定义这两套状态。
`docs/development/gpui-parity-plan-2026-10.md:344-348` 把此列为**待定**：
「新统一前端的状态该新增一个平台字段，还是暂记到 `windows` 端？这决定了阶段 5 回填时的写法，
需要早点定，否则返工。」

**目标**：在排期第一阶段内定下登记方式。推荐**新增 `gpui` 平台字段**，理由：
①记到 `windows` 端会让「Windows 端」这个字段同时表示 Tauri 与 GPUI 两套实现，语义冲突；
②matrix 的 CI 门禁按字段内容比较，新增字段是向后兼容的；
③GPUI 是目标前端，旧 Tauri 端最终退场，字段应该能独立演进。
**此项需要人确认后才能改 `metadata.json` 与生成脚本。**

---

## 3. 功能排期

排期沿用既有计划的五条硬约束（构建 15 分钟/次、构建需绕开工作区 ACL、内存制约、
每批功能必须同步矩阵、验收用 `S1_*` 打点），并**新增**：任何 macOS 相关验收都必须在
阶段 A 完成之后。

**工作量口径**：人日按「一个熟悉本仓库的 Rust 工程师、含自测与矩阵回填」估算。
构建受限使每天有效产出低于标准值，估算里已含这个折扣。

### 3.1 依赖关系图

```
阶段 A（构建与登记地基）
   ├──> 阶段 B（编辑器基本功）
   │        └──> 阶段 C（语义与导航）
   ├──> 阶段 D（Git 完整化）      [仅依赖 A 的 Windows 侧]
   ├──> 阶段 E（Java 工具链）      [依赖 C 的 jdtls 通路]
   └──> 阶段 F（macOS 首轮实机验收）[依赖 A 的 macOS 区块 + B/C 的成果]

阶段 G（矩阵回填与收尾）    [依赖 B/C/D/E 全部完成]
```

### 3.2 排期表

| 阶段 | 周次 | 内容 | 人日 | 依赖 | 验收方式 |
|---|---|---|---|---|---|
| **A1** | W1（10/13–10/14） | **定矩阵登记方式**：确认是否新增 `gpui` 平台字段；改 `metadata.json`、`generate-platform-feature-matrix.mjs`、`verify-platform-feature-matrix.*` | 1 | 无（需人确认方案） | `node scripts/generate-platform-feature-matrix.mjs` 与 `./scripts/verify-platform-feature-matrix.sh` 均通过；生成的 HTML 出现 gpui 列 |
| **A2** | W1（10/14–10/16） | **Windows 侧 GPUI 构建发布链路**：建 `release-preview-gpui-windows.yml`，复用 `sign-windows.ps1`；把 `build-windows.ps1` 的 jdtls/jdk 预置步骤抽成可复用片段 | 3 | A1 | CI 产出带签名的 `Lithe.exe` 并作为 artifact 上传；本地 `CARGO_TARGET_DIR=... cargo build -p lithe-gpui-app` 成功 |
| **A3** | W1–W2（10/16–10/21） | **macOS 侧 GPUI 构建链路**：建 `release-preview-gpui-macos.yml`，对照 `build-macos.sh` 与 `release-macos.yml` 的结构；需先确认 `gpui-pre` 在 macOS 上的构建可行性（见未覆盖范围 #2） | 5 | A1 | macOS runner 产出 `.app` 或可执行文件；能在 macOS 上启动并打出 `S1_*` 日志 |
| **A4** | W2（10/19–10/20） | **构建效率基线测量**：记录「只改 `lithe-ui`」与「只改 `workbench`」两种情形的构建耗时；给 2.2 的优化方案提供数据 | 1 | 无 | 产出测量记录（≥5 次采样），作为后续优化的对照基线 |
| **A5** | W2（10/20–10/21） | **阶段 0 止血**（既有计划已定位到行）：0.1 会话保存 exclude 失败降级、0.2 Spring 面板入口补齐、0.3 在文件中替换的 Core 命令 | 2 | 无 | `S1_WORKSPACE_CONFIG session_flush ... wrote=true`；右栏出现 Spring 图标且点击渲染端点列表；在文件中替换能落盘 |
| **B1** | W2–W3（10/21–10/26） | **编辑器基本功**：#1–#3 文件增删改名、#4 新建标签页、#5–#6 编辑器内查找替换、#8–#13 保存全部/另存为/还原文件/跳转历史前后退 | 6 | A5 | 新建→编辑→保存→重命名→删除全链路走通；`Ctrl+F` 在当前文件内查找替换；`←`/`→` 跳转历史可用；关掉再开项目上次打开的文件还在 |
| **B2** | W3（10/26–10/28） | **编辑器显示与编辑操作**：#16–#23 注释/复制行/删除行/上下移行/分屏/缩略图/行号/空白字符、#14–#15 大纲与面包屑 | 4 | B1 | 每个入口点击有可见效果；对应 `MenuItem::NotWired` 与 `MISSING_*` 分组同步删除，`not_wired_count()` 单调下降 |
| **B3** | W3（10/28–10/29） | **清理 `allow(dead_code)`**：2.4 的生产代码口径 18 处降到 ≤5 处 | 2 | B2 | `grep -rn "allow(dead_code)" rust/lithe-gpui/crates/*/src/` 计数 ≤5（测试辅助模块 `crates/workbench/tests/` 不计入），每处有注释说明 |
| **C1** | W4（10/29–11/02） | **语义与导航**：#28–#34 重命名/引用/实现/类型定义/格式化/参数提示；#35–#36 诊断与代码动作打磨 | 5 | A3（jdtls 通路）、B2 | 打开 `.java` 能看到诊断、`F2` 能重命名；`S1_JAVA_*` 不再 FAILED |
| **C2** | W4（11/02–11/03） | **PHP 与语言支持**：#38 | 1 | C1 | matrix `php-run-test` 从 partial 转 implemented |
| **D1** | W4（10/30–11/02） | **Git 主动操作**：#56–#58 新建分支/提交 diff/merge-rebase-cherry-pick | 4 | A2 | 能建分支、能看某提交 diff、能发起 merge 并在冲突时看到横幅 |
| **D2** | W5（11/03–11/04） | **Git 补齐**：#59–#61 标签管理/Stash/身份配置页 | 3 | D1 | 能 tag、能 stash 再 pop、身份配置页可保存 |
| **E1** | W5（11/04–11/06） | **Java 工具链**：#63–#66 Maven 目标执行/依赖树/Profile/MyBatis 导航（**前置：修 `mvn`**） | 4 | C1 | Maven 工具窗能执行 `clean install` 并看到输出 |
| **E2** | W5（11/06–11/07） | **Java 运行与 Spring 导航**：#67–#68 | 2 | E1 | 能从 Spring 端点跳到控制器方法；能跑 Java main 与测试 |
| **F1** | W6（11/07–11/10） | **macOS 首轮实机验收**：跑通 B/C/D 在 macOS 上的表现；验证 2.7 的 `secondary-` 归一化推断 | 3 | A3、B2、C1 | macOS 上启动应用、`secondary-` 系键位生效、编辑器基本功可用；记录所有 macOS 专属失败项 |
| **F2** | W6（11/10–11/12） | **macOS 视觉对齐**：#49–#54 共享控件/滚动条/提示/字体/图标/IME | 4 | F1 | 与 `macos/` 端截图并排比对，差异项列入后续 |
| **G1** | W6（11/12–11/13） | **矩阵全量回填** + `cargo test` + `verify-agent-notes.sh` | 2 | B/C/D/E 全部 | CI 门禁通过；`cargo test` 全绿 |
| **G2** | W6（11/13–11/14） | **端到端实机流程** + 进程清理确认 | 1 | G1 | 开项目→编辑→搜索替换→提交→运行→调试全通；无 Lithe 进程残留 |

**合计 53 人日**（18 个阶段行的人日列相加：1+3+5+1+2+6+4+2+5+1+4+3+4+2+3+4+2+1 = 53）。
按 1 人全职推进约 11 周（受构建 15 分钟/次与内存制约，单日有效产出低于标准值）；
按 2 人并行、其中 1 人专责构建链路与 macOS 通道，约 6 周可覆盖 A–G 全部阶段。

### 3.3 明确排除的项（本月不做）

沿用既有计划的排除线（19 项），理由统一为「旧端自己也是『待开发』，平替没有参照价值」：

GitHub PR / 评论审查（#107–#108）· 数据库四条（#103–#106）· Docker（#74）·
远程工作区 / WSL（#79–#80）· AI 聊天与 Agent（#83–#96）· 插件市场安装（#102）·
更新器（#43–#44）· 诊断导出（#113 的一部分）· IntelliJ Islands 外观与项目颜色渐变（#47–#48）·
任务栏项目标题（#81）· PHP 可选插件（#38 部分）· Notebook run actions。

**新增排除**：MyBatis（#66 保留在 E1，因为它依赖 Maven 通路已就位，边际成本低）。

### 3.4 高风险项（可能推翻排期）

| 风险 | 影响 | 触发条件 |
|---|---|---|
| `gpui-pre` 在 macOS 上无法构建或行为异常 | **整条 A3 与 F1/F2 作废**，macOS 端无 GPUI 产品 | A3 第一次 CI 运行 |
| 矩阵新增 `gpui` 字段的方案被否 | A1 重做，G1 回填写法返工 | 人确认环节 |
| `mvn` 修不好 | E1 阻塞，E2 的 Spring 导航仍可做 | `mvn -version` 仍报 `classworlds` |
| 构建时长优化不达预期 | B/C/D 各阶段实际耗时上浮 30–50% | A4 的基线测量结果 |
| `explorer.exe` 内存泄漏反复 | 构建失败 → `cargo clean` → 当日进度归零 | 每日开工内存检查 |

---

## 4. 未覆盖范围与未知项

1. **`gpui-pre` 在 macOS 上的可构建性未经验证。** 本工作区是 Windows 主机，无法编译 macOS 目标。
   选型笔记（`.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`）
   记录 `gpui-pre` 是「一个人重新打包的 Zed 快照」，其 platform 层是否完整支持 macOS 未见证据。
   **若它不支持 macOS，则 F1/F2 与 A3 全部作废，整个「一套 Rust 界面覆盖三端」的目标需要重新评估。**
   这是本次盘点里最大的单点未知。

2. **macOS 端 GPUI 的性能基线未知。** 仓库有 `docs/performance/macos-interaction-baseline.md`，
   但那是 Swift 端的基线。GPUI 在 macOS 上的交互延迟、内存占用、GPU 行为均无数据。

3. **Linux 端完全未盘点。** 选型笔记说目标是覆盖 macOS/Windows/Linux，鸿蒙 PC 排后面。
   检索 `.github/workflows` 与 `scripts/` 未发现任何 Linux 构建入口，但本次任务只对照
   Windows 与 macOS 两端，Linux 的缺口规模未评估。

4. **矩阵的 Windows 列与 GPUI 实际状态的对应关系未逐条核对。** 本次只核对了
   `backend-capabilities.ts` 明确标注为 `false` 的 7 类。矩阵里其余 92 条 `implemented`
   是否在旧 Tauri 端真的可用、GPUI 端各自差多少，需要逐条实机核对，本次未做。

5. **`right_tool_window.rs` 的 Spring 面板是「接线遗漏」还是「刻意不接」未确认。**
   `from_rail_index` 不映射 Spring，但 `from_id`/`id()`/`title()`/`icon()` 四个分支都已实现，
   且有测试断言 `from_rail_index(3) == None`。既有计划（`:150-154`）把它当接线遗漏处理，
   但也可能是等 Spring Boot 项目探测器就绪后才开放。**这个判断决定 A5 的工作量是 0.5 天还是 2 天。**

6. **专家团六名成员全部未返回实质结论。** 原计划由「Lithe 编辑器开发团」的界面设计师、
   UI 设计师、编辑器架构师、核心开发工程师、插件扩展工程师、构建发布工程师分头核验，
   实际引擎返回 `disabled`（Agent Team 服务或会话工具未就绪）。
   本文件的专业结论由主理人基于仓库事实与证据台账补写，
   **未经独立专家核验**；`docs/development/gpui-gap-team-handoff-2026-10.md` 逐条标注了覆盖缺口。

7. **`windows/tauri` 与 `macos/` 两端的实际可用性未实机确认。** 两端在矩阵里的状态是
   「初版静态盘点」（`metadata.json` 的 `review.status: initial-static-inventory`），
   macOS 列 130 条 `pending`、Windows 列 124 条 `pending`。
   本文件把矩阵状态当对照基准使用，**基准本身未经运行时验收**。

8. **「检索命中 0」有两种口径，本次盘点里混用过一次。** 实现代码口径（只搜 `.rs`）与
   全文件口径（含 `crates/shared/locales/**` 的 `.yml`/`.ts` 文案键）结果不同：
   `viewer` 实现代码 0 命中但文案键 207 处、`onboarding` 0 vs 125、`run-actions` 0 vs 68、
   `logging` 0 vs 13、`ai_chat` 0 vs 8、`local-history` 0 vs 4。
   前版把口径限定丢失，写成「GPUI 检索 X 命中 0」，被独立评估者指出与本地化源不符，本版已改。
   **后续核对必须写明口径**：本文件 §1 统一用「实现代码口径」并在证据列注明全文件命中数。

9. **前版有一处证据错误已更正**：#61「Git 身份配置页」前版记为「已实现未接线、生产代码零调用」，
   实际 `workspace.rs:883` 在生产路径注入了 host，`settings/src/dialog.rs:264` 是消费者。
   该行现标为「已接线」并保留在表中作为更正记录。**这条错误的来源是转引既有计划
   `gpui-parity-plan-2026-10.md:283` 而未复核**——提醒：本项目里既有文档的结论也要复核。
