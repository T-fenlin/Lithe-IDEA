# Lithe Windows 前端 → Rust + gpui-kit 0.6.6 界面规格：数据库 / AI 助手 / 全局搜索

> 本文为只读逆向调研产物。界面规格唯一来源为 Windows 前端 `windows/tauri/src/`（Tauri v2 + React + TypeScript + Tailwind + shadcn/Base UI）。
> 覆盖功能面：**数据库**（连接管理、对象树、SQL 编辑器、结果网格、执行计划）、**AI 助手**（chat/agent 面板与数据通道）、**全局搜索**、**快速打开**。
> 未读取、未参考 `macos/`（旧 macOS 规格已作废）；未修改任何已有文件；未运行构建或测试。
> 每条结论给出 `相对路径:行号`（相对仓库根 `D:\developmentProjects\rust\Lithe-IDEA`）；无法确证处明确写「未找到」，不做推测。
> gpui-kit 侧证据根目录记作 `<REG>` = `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837`，实际 vendored 版本 `gpui-kit-0.6.6` / `gpui-component-0.6.6` / `gpui-base-0.6.6`（底层 GPUI 为 `gpui-pre-0.3.6`，crate 名 `gpui`，`gpui-kit-0.6.6/Cargo.toml:354-356`）。

---

## 0. 换算与来源约定

| 约定 | 内容 | 证据 |
| --- | --- | --- |
| 1 Tailwind 单位 | = 0.25rem = **4px** | 根字号见下一行 |
| 根字号 | `html { font-size: calc(16px * var(--app-ui-scale)) }`，`--app-ui-scale: 1`（默认） | `windows/tauri/src/styles/theme.css:220`、`windows/tauri/src/styles/theme.css:113` |
| 字号工具类 | `.ui-text-caption / .ui-text-chrome / .ui-text-sm / .ui-text-base` 是自定义 CSS 类，非 Tailwind 内置 | `windows/tauri/src/styles/utilities.css:30-44` |
| 实际字号 | `--app-ui-font-size: 13px`；`--ui-text-sm = --ui-text-base = 13px`；`--ui-text-caption: 12px`；`--ui-text-chrome: 13px` | `windows/tauri/src/styles/theme.css:112-117` |
| 舒适密度覆盖 | caption 13px / chrome 14px / tab 高度 2rem(32px) / 控件高 1.75rem(28px) | `windows/tauri/src/styles/theme.css:188-200` |
| 行高令牌 | `--leading-row: 1.35` | `windows/tauri/src/styles/theme.css:4` |
| 圆角 | `--radius: 8px` → `rounded-sm` 4.8px、`rounded-md` 6.4px、`rounded-lg` 8px、`rounded-xl` 11.2px、`rounded-2xl` 14.4px | `windows/tauri/src/styles/theme.css:6-12`、`:134` |
| 颜色来源 | 主题 JSON `colors.<key>` → CSS 变量 `--<key>`，由 `root.style.setProperty` 内联写入 `documentElement`；`data-theme` / `data-theme-type` 标记 | `windows/tauri/src/extensions/themes/theme-registry.ts:92-99`；默认主题族 `windows/tauri/src/extensions/themes/builtin/lithe.json`（经 `windows/tauri/src/extensions/themes/default-theme.ts:36-54` 读取） |
| 滚动条 | `--app-scrollbar-size: 11px`、`--app-scrollbar-thin-size: 9px`、圆角 999px | `windows/tauri/src/styles/scrollbars.css:1-11` |
| 面板尺寸令牌 | `--lithe-pane-header-height: 2.25rem`(36px)、`--lithe-tab-height: 1.75rem`(28px)、`--lithe-tab-max-width: 12.5rem`(200px)、`--lithe-chrome-control-height: 1.5rem`(24px) | `windows/tauri/src/styles/theme.css:118-133` |
| gpui-kit 默认行高 | `Size::table_row_height()`：XSmall 26 / Small 30 / Medium **32（默认）** / Large 40（px）；单元格外边距 Medium 上下 4 左右 8 | `<REG>\gpui-component-0.6.6\src\sizing.rs:57-65`、`:69-96` |

---

## 1. 区域清单（组件层级 + 数据来源结构）

### 1.0 三个功能面的前置事实（**先读这段，决定重写范围**）

| 事实 | 证据 |
| --- | --- |
| Windows 后端**没有**任何数据库命令实现 | `windows/tauri/src-tauri/src/main.rs:112-191` 的 `generate_handler!` 清单里没有 `run_database_provider_command` / `connect_database` / `list_saved_connections` / `save_connection` / `delete_saved_connection` / `test_connection` / `store_db_credential` / `get_db_credential`；`windows/tauri/src-tauri/src/platform.rs` 全文 grep `database` **0 命中** |
| 数据库能力被前端硬开关关闭 | `windows/tauri/src/config/backend-capabilities.ts:5` `database: false`；`windows/tauri/src/platform/tauri-core.ts:145-154` 把 `command.includes("database")` 与 `db_credential` / `list_saved_connections` / `save_connection` / `delete_saved_connection` / `test_connection` 归入 capability `database`；命中即在前端 reject，错误文本 = `待开发: database (<command>)`（`windows/tauri/src/platform/tauri-core.ts:99-104`、`windows/tauri/src/config/backend-capabilities.ts:1`） |
| Windows 后端**没有**任何 ACP / Codex / chat-DB 命令实现 | `windows/tauri/src-tauri/src/main.rs:112-191` 无相关注册；`ai_commit_*` 之外的 `platform_invoke` 只做 `translate()`→`lithe_core::execute_json`（`windows/tauri/src-tauri/src/platform.rs:8-97`）；`rust/lithe-core/src` 无 `acp` 命中 |
| AI 能力同样被硬开关关闭 | `windows/tauri/src/config/backend-capabilities.ts:4` `agent: false`；`windows/tauri/src/platform/tauri-core.ts:172-180` 把 `acp_` / `codex_` / `ai_provider` / `_chat` / `get_available_agents` 归入 capability `agent` |
| 菜单里数据库入口显式禁用 | `windows/tauri/src/features/window/components/window-menu-bar.tsx:442-448`：`disabled={!isBackendCapabilityAvailable("database")}` + `title={BACKEND_UNAVAILABLE_TOOLTIP}` |
| 数据库连接管理器（命令面板视图）**无渲染分支**，是死代码 | `DatabaseCommandContent` 定义于 `windows/tauri/src/features/database/components/database-sidebar.tsx:72`，**全仓库无任何 import/挂载**（grep `DatabaseCommandContent` 仅 2 命中：定义处与其 props interface）；命令面板 view id `"databases"` 存在于类型联合（`windows/tauri/src/features/command-palette/types/view.types.ts:7`）并被 `database.connect` 使用（`windows/tauri/src/features/keymaps/commands/command-registry.ts:1064-1069`），但 `windows/tauri/src/features/command-palette/components/command-palette.tsx:402-439` 只处理 `color-theme` / `icon-theme` / `local-history` / `outline` / 扩展视图，`"databases"` **落回根命令列表** |
| `createDatabaseActions`（命令面板 Database 分类项）同样**无调用方** | 定义 `windows/tauri/src/features/command-palette/constants/database-actions.tsx:8`；grep `createDatabaseActions` 仅定义处 1 命中 |
| 数据库 sidecar 二进制**在本仓库没有对应 crate** | 扩展清单声明 `sidecar: { "win32-x64": "bin/lithe-db-sqlite.exe", ... }`（`windows/tauri/src/extensions/database/database-provider-extensions.ts:25-31`、`:45-51`、`:65-71`、`:85-91`、`:105-111`、`:125-131`）；打包脚本调用 `cargo build -p lithe-database ... --bin lithe-db-<provider>`（`windows/tauri/src/extensions/tooling/package-database-sidecars.ts:136`），但全仓库 `Cargo.toml` 中**没有 `name = "lithe-database"` 的包**（glob 全部 Cargo.toml 匹配该 name → 0 命中）；`getDatabaseProviderExtensions()` 实际只返回 sqlite 一项（`windows/tauri/src/extensions/database/database-provider-extensions.ts:136-137`） |
| 全局搜索的「浮层」形态未使用 | `isGlobalSearchVisible` / `setIsGlobalSearchVisible` 定义于 `windows/tauri/src/features/window/stores/ui-state/modal-slice.ts:11,32,52,186-200`，但**无任何调用方**（grep 仅 store 内部与三个 `state.isGlobalSearchVisible ||` 的互斥判断，见 `windows/tauri/src/features/github/components/github-notifications-menu.tsx:67`、`windows/tauri/src/features/git/components/git-branch-manager.tsx:182`、`windows/tauri/src/features/run-actions/components/run-actions-button.tsx:108`） |
| 全局搜索的「pane buffer」形态也未使用 | `openGlobalSearchBuffer`（`windows/tauri/src/features/editor/stores/buffer.store.ts:1328-1330`）**无调用方**；`case "globalSearch"`（`windows/tauri/src/features/panes/components/pane-container.tsx:981-982`）因此不可达 |
| `AgentLauncher` 组件**无挂载点** | `windows/tauri/src/features/ai/components/agent-launcher.tsx:124` 全仓库仅此一处；`setIsAgentLauncherVisible(true)` 只被 `windows/tauri/src/features/ai/hooks/use-new-agent-action.ts:4-11` 调用，而该 action 的键位 `cmd+shift+space`（`windows/tauri/src/features/keymaps/defaults/default-keymaps.ts:390-394`、`windows/tauri/src/features/keymaps/commands/command-registry.ts:624-628`）落地后没有任何组件消费该状态 |
| 主题色真实取值 | `windows/tauri/src/extensions/themes/builtin/lithe.json:12-27`（lithe-light）、`:79-94`（lithe-dark）；派生别名 `--popover/--muted/--card = surface`、`--input = border`、`--ring = border-strong`（`windows/tauri/src/styles/theme.css:140-148`） |

> 结论：**这三个功能面的 React/Vue 层 UI 规格完整可逆向，但数据通道在 Windows 上尚未实现**。gpui-kit 重写时，UI 可以照本规格一比一复刻；数据层必须同时决定「把驱动放到哪一侧」。

---

### 1.1 数据库

#### 1.1.1 挂载链

```
MainLayout                                   windows/tauri/src/features/layout/components/main-layout.tsx:81
├─ ConnectionDialog（全局模态）              main-layout.tsx:369-372
│   └─ ConnectionDialog                      features/database/components/connection/connection-dialog.tsx:216
└─ PaneContainer → case "database"           features/panes/components/pane-container.tsx:1011-1029
    └─ getDatabaseViewer(dbType)（lazy，按 provider 缓存）  pane-container.tsx:62-68、:1013
        ├─ SQLite / DuckDB / Postgres / MySQL → SqlDatabaseViewer   providers/sql/sql-provider-viewer.tsx:17-46
        │   └─ SqlDatabaseViewer             providers/sql/sql-database-viewer.tsx:38
        ├─ MongoDB → MongoDBViewer           providers/mongodb/mongodb-viewer.tsx:36
        └─ Redis   → RedisViewer             providers/redis/redis-viewer.tsx（default export）
```

- provider 注册表（label / 是否文件型 / 默认端口 / 文件扩展名 / lazy viewer）：`windows/tauri/src/features/database/providers/provider-registry.ts:19-74`（sqlite `.sqlite/.db/.sqlite3`、duckdb `.duckdb/.duck`、postgres 5432、mysql 3306、mongodb 27017、redis 6379）。
- viewer props 是二选一联合：`{ databasePath } | { connectionId }`（`provider-registry.ts:4`）；`pane-container.tsx:1014-1027` 按 `isFileBased` 决定，缺 `connectionId` 时渲染 `Empty` + `panes.missingDatabaseConnection`（`:1019-1025`）。
- tab：buffer 类型 `database`（`windows/tauri/src/features/panes/types/pane-content.types.ts:161-165`，含 `databaseType`、可选 `connectionId`）；tab 标题 `panes.databaseViewer`（`panes/components/pane-container.tsx:209-210`）；buffer 名字取连接名/文件名（`features/editor/stores/buffer.store.ts:1716-1729`）；打开入口 `openDatabaseBuffer(path, name, databaseType, connectionId?)`。

#### 1.1.2 组件层级（SqlDatabaseViewer）

```
SqlDatabaseViewer                            providers/sql/sql-database-viewer.tsx:170-416
├─ TableToolbar                              components/table-toolbar.tsx:48
│   ├─ Database 图标 + fileName + "{tables}t {indexes}i"   :103-113
│   ├─ 视图切换段（Data / Schema / Info，圆角容器内 3 个 Button）  :114-131
│   └─ 右侧动作：列类型开关 / 结果摘要 chip / SQL 编辑器 / 订阅（创建·启停·刷新·删除）/ CSV / JSON  :133-246
├─ 左面板 TableSidebar（w-64）               components/table-sidebar.tsx:35
│   ├─ SidebarTitleBar："对象（N）" + hover 显示的「创建表」  :59-69
│   ├─ SidebarSectionLabel × 分组（表/视图/物化视图/订阅/索引）  :71-99
│   ├─ SidebarListItem × 对象（active / description=owner / 右键菜单）  :78-96
│   └─ SqlHistoryList（compact）             components/sql-history-list.tsx:25
├─ 右面板 databasePanelClassName("flex-1 border border-border/70 bg-background")  :240
│   ├─ QueryBar                              components/query-bar.tsx:183
│   │   ├─ 过滤模式（非自定义查询）：InputGroup + Search 图标 + 清除按钮  :311-342
│   │   └─ SQL 模式：SqlEditor（pre 高亮层 + 透明 Textarea + 补全 chips）  :46-181、:262-308
│   ├─ ColumnFilters（viewMode=data 且表可写）  components/column-filters.tsx:35
│   ├─ Alert tone=error（store.error）        :268-272
│   ├─ Empty + Spinner（isBusy）              :274-280
│   ├─ DataGrid（viewMode=data 且有结果）      components/data-grid.tsx:92
│   │   ├─ 结果计数行 h-9 + hover 显示的「添加行」  :401-422
│   │   ├─ 滚动容器（tabIndex=0，onKeyDown 网格导航）  :423-429
│   │   └─ <table>：sticky thead + 虚拟化 tbody + 上下 padding 行  :430-630
│   │       ├─ th "#"（w-10，点击全选） :433-439
│   │       ├─ th 每列：图标 + 列名 + 排序箭头 + FK 标记 + 过滤按钮 + 类型/PK/NN 副行 + 1px 拖拽手柄  :440-508
│   │       ├─ td 行号（(page-1)*pageSize+ri+1，点击选整行）  :530-535
│   │       ├─ td 每格：CellRenderer 或内联 Input 编辑态  :556-616
│   │       └─ Dropdown（单元格右键复制菜单）  :633-638
│   ├─ PostgresSubscriptionSchemaView（viewMode=schema 且对象为订阅）  :317-319
│   ├─ SchemaView（viewMode=schema）          components/schema-view.tsx:50
│   ├─ InfoView（viewMode=info）              components/info-view.tsx:22
│   └─ Pagination（totalPages>1）             components/pagination.tsx:17
├─ SqlTableMenu（表/视图/索引右键）            components/context-menus.tsx:7-60
├─ SqlRowMenu（行右键）                        components/context-menus.tsx:62-98
├─ CreateRowModal / EditRowModal / CreateTableModal   components/crud-modals.tsx:25 / :（EditRowModal）
└─ CreateSubscriptionDialog                  providers/postgres/components/create-subscription-dialog.tsx
```

CellRenderer 的分支（`components/cell-renderer.tsx`）：`null/undefined` → `Badge muted` "NULL"（`:32-38`）；JSON 字符串 → 可展开 `Button` + `<pre>`（`:41-60`）；ISO 日期 → `toLocaleString` 短格式（`:63-69`）；Unix 秒 → 日期 + `title="Raw: n"`（`:72-79`）；外键 → 主色下划线按钮（`:82-95`）；object/array → `JSON.stringify`（`:98-104`）；长文本 >100 字符 → 可展开（`:107-126`）；其余 → 截断 span（`:129-136`）。判定启发式：`isIsoDate`（`:141-144`）、`isUnixTimestamp`（2000–2100 区间，`:146-149`）、`isJsonString`（`:151-166`）。

#### 1.1.3 MongoDB / Redis viewer 层级

```
MongoDBViewer                                providers/mongodb/mongodb-viewer.tsx:67-324
├─ header chip（Database 图标 + fileName）+ 数据库 Select + Collections 计数  :69-93
├─ 左面板 w-56：Collections 列表（Button，选中 bg-selected）  :96-120
└─ 右面板 flex-1：Filter JSON Input + Sort JSON Input + Apply/Reset/Refresh  :122-167
    ├─ Empty（未选集合）/:isLoading Spinner / Alert error  :169-192
    ├─ 文档卡片列表（databaseCardClassName + <pre> JSON）  :194-245
    ├─ Empty（无文档）  :247-256
    └─ 分页（首页/上一页/下一页/末页 + 每页 Select）  :258-321

RedisViewer                                  providers/redis/redis-viewer.tsx
├─ header chip（Server 图标 + fileName）+ Info 切换 + Refresh keys  :92-119
├─ 左面板 w-64：Search 图标 + key pattern Input + 扫描按钮；key 列表（type Badge 前三字母 + key + TTL）  :122-192
└─ 右面板 flex-1：server info 卡片 / 选中 key 的 type+TTL+value 编辑器（type 相关配色 TYPE_COLORS）  :194-275
```

### 1.2 AI 助手

#### 1.2.1 挂载链

```
App → LocaleProvider → WorkbenchApp            windows/tauri/src/App.tsx:92-98、workbench-app.tsx:85
└─ PaneContainer → case "agent"                features/panes/components/pane-container.tsx:948-949
    └─ AgentTab（lazy）                        pane-container.tsx:55-59
        └─ AIChat                              features/ai/components/chat/ai-chat.tsx:53-1157
```

- buffer 类型 `agent`（`features/panes/types/pane-content.types.ts:121-122`，`{type:"agent"; sessionId?: string}`）；`openAgentBuffer` 生成 `agent://{sessionId}` 路径、展示名 `Agent ${n}`、缺省 sessionId `agent-tab-${Date.now()}`（`features/editor/stores/buffer.store.ts:724-765`、`:1324-1325`）；tab 图标分支 `features/tabs/components/tab-bar-item.tsx:190`。
- `AgentTab` 只做两件事：标题回写 buffer.name（`components/agent-tab.tsx:22-25`）、把内容限制在 `mx-auto size-full max-w-4xl`（896px）并渲染 `AIChat`（`:28-37`）。

#### 1.2.2 组件层级（AIChat）

```
AIChat（根）                                  components/chat/ai-chat.tsx:1016-1155
├─ ChatHeader                                 components/chat/chat-header.tsx:96
│   ├─ PaneChip + ProviderIcon / EditableChatTitle（点按重命名）  :150-152、:27-79
│   ├─ 按钮：消息搜索 / Agent 历史 / 新建 Agent  :165-201
│   ├─ 消息搜索条（Input h-7 + 计数 + 上/下/关闭）  :205-271
│   └─ ChatHistoryDropdown                    components/history/chat-history-dropdown.tsx:34
├─ 初始态分支（无消息且无 ACP 事件）           ai-chat.tsx:987-988、:1044-1083
│   └─ AgentShortcuts + AIChatInputBar presentation="initial"
├─ 消息态                                      ai-chat.tsx:1085-1107
│   └─ MessageScrollerProvider autoScroll / MessageScroller / Viewport / Button
│       └─ ChatMessages                       components/chat/chat-messages.tsx:37-185
│           ├─ ChatMessage                    components/chat/chat-message.tsx:76-317
│           │   ├─ user：Message align=end → Bubble(secondary) → 编辑态 Textarea  :129-198
│           │   ├─ tool-only：ToolCallGroupDisplay  :201-207
│           │   ├─ 空流式：ChatLoadingIndicator label=t("ai.thinking")  :209-216
│           │   └─ assistant：AttachmentGroup → GenerativeUIRenderer → PlanBlockDisplay｜MarkdownRenderer → ToolCallGroupDisplay → MessageFooter  :223-313
│           └─ ChatFollowUpActions（仅最后一条 assistant）  :175-180
├─ AcpPermissionPrompt（permissionQueue 非空时）  ai-chat.tsx:1111-1117、components/chat/acp-permission-prompt.tsx:68
└─ AIChatInputBar（消息态）                    components/input/chat-input-bar.tsx:66-1314
    └─ ChatComposer → ChatComposerBody → ChatComposerEditable + ChatComposerToolbar   components/input/chat-composer.tsx:5/42/66/95
        ├─ ContextSelector                     components/selectors/context-selector.tsx:58
        ├─ ChatPreferencesMenu（mode/agent/provider/model）  components/input/chat-preferences-menu.tsx:284
        ├─ 发送 / 停止按钮、队列 Badge、语音 Toggle、斜杠按钮  chat-input-bar.tsx:1051-1165
        ├─ FileMentionDropdown / SlashCommandDropdown  :1270-1296
        ├─ SkillsCommand                       components/skills/skills-command.tsx:58
        └─ ProviderApiKeyCommand               components/provider-api-key-command.tsx:49
```

> 注意：`ModelSelector`（`components/selectors/model-selector.tsx:21`）与 `ProviderSelector`（`components/selectors/provider-selector.tsx:22`）**不在 composer 内**，只被设置页使用（`features/settings/components/tabs/ai-settings.tsx:19-20,453,481,490`）；composer 内的模型/提供商选择由 `ChatPreferencesMenu` 的 `DropdownMenuRadioGroup` 承担（`chat-preferences-menu.tsx:180-213`），显示哪些子菜单由 `components/input/chat-preferences-model.ts:11-38` 决定。

#### 1.2.3 数据来源结构（谁提供数据、通道是什么）

| 通道 | 说明 | 证据 |
| --- | --- | --- |
| (a) **直连 provider HTTP**（`agentId === "custom"`） | `AIChatService` 组装 messages/systemPrompt → provider 实现 `buildHeaders/buildPayload/buildUrl` → fetch。`shouldUseTauriFetchForProvider(providerId)` 为真时用 `@tauri-apps/plugin-http` 的 fetch，否则用 webview 原生 fetch；流式解析在 `utils/stream-utils.ts::processStreamingResponse` | `features/ai/services/ai-chat-service.ts:133-314`（判定 `:36-38`、fetch 选择 `:292-298`、流解析 `:309`）；需要 Tauri fetch 的 provider：gemini / ollama / anthropic / openrouter（`services/providers/ai-provider-registry.ts:175-186`） |
| (b) **经 Tauri host**：密钥 | `get_secure_secret` / `store_secure_secret` / `remove_secure_secret`，key 前缀 `ai-provider-token/`；Rust 实现在 `src-tauri/src/secure_storage.rs:12,19,28`（keyring），注册于 `main.rs:143-145` | `features/ai/services/ai-token-service.ts:9-55` |
| (b) **经 Tauri host**：聊天库（SQLite） | `init_chat_database` / `save_chat` / `update_chat_metadata` / `load_all_chats` / `load_chat` / `delete_chat` 全部走 `platform_invoke`；失败语义 `Query returned no rows` | `features/ai/services/ai-chat-history-service.ts:59,177,187,199,226,241`（`:11-51` 是 snake_case 镜像类型，`:89,229` 是错误语义） |
| (c) **ACP 子进程流** | `AcpStreamHandler` 是唯一 ACP 客户端：`listen<AcpEvent>("acp-event")` + `get_acp_status` / `start_acp_agent` / `install_acp_agent` / `get_available_agents` / `send_acp_prompt` / `respond_acp_permission` / `stop_acp_agent` / `cancel_acp_prompt` / `list_acp_sessions` / `delete_acp_session` / `logout_acp_agent`；超时 status 5s / start 15s / prompt 10s / 首响应 20s | `features/ai/services/acp-stream-handler.ts:60-711`（listen `:305-309`、超时 `:42-45`、命令逐个见 §4.2） |
| (c) **Codex 集成** | `listen("codex-event")` + `start_codex_thread` / `start_codex_turn` / `respond_codex_request` / `interrupt_codex_turn` / `get_codex_status`；设置存 localStorage `lithe-codex-integration-settings` | `features/ai/integrations/codex/codex-integration-service.ts:24,32-42,69,76,83,125,203,208,217` |
| (c) **终端型 agent** | Claude Code / Antigravity CLI 不开聊天流，只开终端 | `features/ai/lib/terminal-agents.ts:5-21`、`lib/terminal-agent-terminal.ts:5-13`、`lib/claude-code.ts:1-9` |
| store | zustand + persist + immer；persist key `lithe-ai-chat-settings-v7`，version 3，只持久化 `mode/outputStyle/selectedAgentId/sessionModeState`，`acpStatus` 强制归 null | `features/ai/stores/ai-chat.store.ts:11-48`（`:22,24-29,41`）；state 形状 `stores/ai-chat/ai-chat-store.types.ts:29-46`；初值 `stores/ai-chat/ai-chat-state.ts:3-22` |
| 面板内事件 | 技能插入 `lithe-ai-insert-skill`（`lib/skill-events.ts:3-16`，监听 `chat-input-bar.tsx:760-769`）；菜单 `menu_toggle_ai_chat`（`features/window/hooks/use-menu-events.ts:49`）；侧栏资源拖入 `SIDEBAR_RESOURCE_DROP_ON_AI_EVENT`（仅 `surfaceId === "activity-sidebar"` 生效，**该 surfaceId 无调用方**） | `chat-input-bar.tsx:303-314` |
| **未使用** | AI 流**没有**用 Tauri `Channel`；`Channel` 仅用于 git | `windows/tauri/src/platform/tauri-core.ts:113-118` |

### 1.3 全局搜索

**唯一可达形态：主侧栏视图 `search`**（不是浮层、不是 pane buffer）。

```
SidebarPaneSelector（活动栏放大镜，快捷键提示 Mod+Shift+F）  features/layout/components/sidebar/sidebar-pane-selector.tsx:134-150
MainSidebar → pane id "search" → <GlobalSearchBuffer compact />   features/layout/components/sidebar/main-sidebar.tsx:797-804
活动栏右键菜单「搜索」                             main-sidebar.tsx:717-720
命令 workbench.showGlobalSearch（cmd+shift+f）/ workbench.showProjectSearch（cmd+shift+shift+h）  features/keymaps/commands/command-registry.ts:644-663
  └─ openGlobalSearchSidebar()                    features/layout/actions/workbench-tool-window-actions.ts:37-49
```

```
GlobalSearchBuffer（compact 时隐藏左导航栏）       features/global-search/components/global-search-buffer.tsx:54、:518-588
├─ GlobalSearchToolbar                            components/global-search-toolbar.tsx:46-208
│   ├─ SearchReplaceToggle（展开/收起详情）       :95-100
│   ├─ 搜索框：h-7 圆角容器 + MagnifyingGlass + CommandInput + 清除  :101-129
│   ├─ ToggleGroup（区分大小写 / 全字匹配 / 正则，iconOnly segmented）  :130-146
│   └─ Badge（searchWarning 优先，否则 resultLabel；max-w-56/w-64）  :147-165
│   └─ 详情区：SearchReplaceRow（替换框 h-8 + 替换 + 全部） + 包含/排除文件两列  :167-206
├─ GlobalSearchResults（有结果且非初始 busy）      components/global-search-results.tsx:35-107
│   ├─ FileNavigatorSidebar（默认宽 224px，可拖 176–420）  features/file-explorer/components/file-navigator-sidebar.tsx:269、lib/file-navigator-layout.ts:1-3
│   └─ ScrollArea（both 方向）→ SearchExcerptResults     components/search-excerpt-results.tsx:166-223
│       └─ SearchExcerptItem × N                   :35-151
│           ├─ MultibufferFileHeader（文件名 + ":行号" + "N matches" + 展开/收起上下文按钮）  :111-139
│           └─ SearchExcerptCode（行号 gutter + 语法高亮行 + 命中高亮 + 点击跳行）  components/search-excerpt-code.tsx:126-184
└─ GlobalSearchState（无结果/加载/错误/不可用）    components/global-search-state.tsx:39-117
```

数据流：`useContentSearch`（`hooks/use-content-search.ts:87-466`）→ `searchFilesContent` → lithe-core `workspace.search`（`features/file-search/lib/file-search-api.ts:87-152`）；索引状态 `fffScanStatus` → `workspace.snapshot`（`:158-168`、`:200-207`）；WSL 工作区改走 `searchProviderFilesContent`（`services/provider-content-search.ts`，调用点 `use-content-search.ts:183-199`）。

### 1.4 快速打开（Quick Open）

**全局浮层**，挂在 `MainLayout` 顶层（`features/layout/components/main-layout.tsx:363-366`），可见性由 `useUIState.isQuickOpenVisible` 控制（`features/window/stores/ui-state/modal-slice.ts`）。

```
QuickOpen                                      features/quick-open/components/quick-open.tsx:16-211
└─ Command（浮层外壳，见 §2.5）               ui/command.tsx:121
    ├─ CommandHeader：CommandInput + 计数徽标（FileCountBadge / Symbol 计数）
    └─ CommandList（ScrollArea）
        ├─ @ 符号模式：SymbolListItem × N      components/symbol-list-item.tsx
        ├─ # 工作区符号模式：SymbolListItem（showFilePath）
        ├─ 无结果：EmptyState                 components/empty-state.tsx:14
        └─ 文件模式：三段（打开的标签页 / 最近 / 其他）各为 FileListItem   components/file-list-item.tsx:19
```

- 模式判定：`query.startsWith("@")` → 当前文件符号；`query.startsWith("#")` → 工作区符号（LSP）——`hooks/use-quick-open.ts:47-48`。
- 数据源：文件列表 `useFileLoader`（`hooks/use-file-loader.ts:21-115`，`workspace.snapshot` + 忽略规则过滤）；模糊检索 `useFffSearch`（`features/file-search/hooks/use-fff-search.ts:5-57`，默认 limit 100，前端 `includes` 过滤 + `score = limit - index`，见 `file-search-api.ts:170-181`）；符号 `useSymbolSearch` / `useWorkspaceSymbolSearch`（后者走 `LspClient.getWorkspaceSymbols`，`hooks/use-workspace-symbol-search.ts:71-94`）。

---

## 2. 度量表

### 2.1 数据库

| 元素 | 类名 / 值 | 证据 | px |
| --- | --- | --- | --- |
| 面板头部（Redis/Mongo 用） | `mx-2 mt-2 rounded-xl bg-background/85 px-3 py-2` | `components/database-surface.ts`（`windows/tauri/src/features/database/components/database-surface.ts:4`） | 上/左右 8 / 圆角 11.2 / 12·8 |
| 面板容器 | `flex min-w-0 flex-col overflow-hidden rounded-xl bg-background/85` | `database-surface.ts:8` | 圆角 11.2 |
| chip | `inline-flex items-center gap-1.5 rounded-full bg-surface/70 px-2.5 py-1` | `database-surface.ts:12` | gap 6 / 10·4 / 胶囊 |
| 卡片 | `rounded-xl border border-border/60 bg-surface/40` | `database-surface.ts:16` | 圆角 11.2 |
| 代码块 | `whitespace-pre-wrap rounded-lg bg-surface/40 p-3 ui-text-sm leading-5` | `database-surface.ts:21` | 圆角 8 / 12；字号 13 / 行高 20 |
| 工具条外壳 | `px-3 py-2` | `components/table-toolbar.tsx:100` | 12·8 |
| 视图切换段 | `rounded-lg border border-border/60 bg-surface/60 p-0.5`；按钮 `px-2.5 ui-text-sm`、`size="xs"`(h-6) | `table-toolbar.tsx:114,120-122`；`ui/button.tsx:23` | 圆角 8 / 内边距 2；按钮 h24 / 10 |
| 结果摘要 chip | `databaseChipClassName("px-2 font-sans ui-text-sm text-subtle-foreground")` | `table-toolbar.tsx:148` | 8 |
| 左面板（SQL） | `w-64` | `components/table-sidebar.tsx:58` | 256 |
| 左面板（Mongo） | `w-56` | `providers/mongodb/mongodb-viewer.tsx:96` | 224 |
| 左面板（Redis） | `w-64` | `providers/redis/redis-viewer.tsx:122` | 256 |
| 对象树标题栏 | `SidebarTitleBar`：`h-(--lithe-pane-header-height)` + `px-3`，标题 `pl-2 ui-text-lg` | `ui/sidebar.tsx:38,43` | 36 / 12；标题 pl 8 |
| 分组标签 | `SidebarSectionLabel`：`h-(--lithe-chrome-control-height) gap-(--lithe-chrome-gap-loose) px-2` + 调用处 `px-2.5 py-1 uppercase` | `ui/sidebar.tsx:367`；`table-sidebar.tsx:75` | 24 / gap 6 / 8；10·4 |
| 对象行 | `SidebarListItem`：`min-h-(--lithe-tab-height) gap-(--lithe-chrome-gap-loose) rounded-(--lithe-chrome-radius) px-2 py-1 ui-text-chrome`，active `bg-accent/80` | `ui/sidebar.tsx:240,290` | 28 / 6 / 圆角 4 / 8·4 / 13 |
| 结果计数行 | `flex h-9 items-center justify-between border-b px-3` | `components/data-grid.tsx:401` | 36 / 12 |
| 表头单元格 | `px-2 py-1.5 border-b bg-surface`；`sticky top-0 z-10` | `data-grid.tsx:431,449` | 8·6 |
| 行号列 | `w-10` | `data-grid.tsx:434` | 40 |
| 数据单元格 | `max-w-75 border-b px-2 py-1.5 ui-text-sm` | `data-grid.tsx:560` | 上限 **300** / 8·6 |
| **列宽策略** | `MIN_COLUMN_WIDTH = 60`、`DEFAULT_COLUMN_WIDTH = 150`；`style={{width}} + minWidth: 60`（th 与 td 都设）；拖拽手柄 `absolute right-0 w-1 cursor-col-resize hover:bg-primary/40` | `data-grid.tsx:40-41,450,500,566` | 最小 **60** / 默认 **150** / 手柄 **4** |
| **行高策略** | `ESTIMATED_ROW_HEIGHT = 34`（`estimateSize`）、`overscan: 12`、`measureElement` 在非 Firefox 下用 `getBoundingClientRect().height`（真实测量，行高可变）；虚拟化用上下 padding `<tr>` 占位 | `data-grid.tsx:42,131-140,512-516,621-628` | 估算 **34** |
| 单元格选中 / 激活 | 选中 `bg-primary/10`；激活 `outline outline-1 outline-primary/70 -outline-offset-1`；主键格 `bg-accent/55`；行 hover `hover:bg-accent/25` | `data-grid.tsx:527,562-564` | — |
| 内联编辑框 | `w-full rounded-lg border-border/70 bg-surface/80 ui-text-sm focus:border-primary/60` | `data-grid.tsx:595` | 圆角 8 |
| SQL 编辑器容器 | `relative h-20 overflow-hidden bg-surface/60` | `components/query-bar.tsx:94` | **高 80** |
| SQL 编辑器文本 | `px-3 py-2 font-mono ui-text-sm leading-5`（pre 与 textarea 同款，textarea 文字透明） | `query-bar.tsx:99,151` | 12·8 / 13 / 20 |
| 补全 chips | `Button size="xs"` + `h-6 border border-border/60 px-2`，容器 `mt-1 flex flex-wrap gap-1` | `query-bar.tsx:158,165` | 24 / 4 |
| 筛选器条 | `databaseCardClassName("mx-3 mb-2 bg-surface/60 px-3 py-2")`；行 `gap-2 ui-text-sm`；Select/Input `size="xs"` | `components/column-filters.tsx:47,78-84` | 12·8 / gap 8 |
| 分页条 | `border-t bg-surface px-3 py-2`；页码输入 `h-6 w-12 px-1 py-0 text-center ui-text-sm`；Select `min-w-16 size="xs"` | `components/pagination.tsx:54,68,99` | 12·8 / 24×48 / 64 |
| 每页可选值 | 10 / 25 / 50 / 100 / 500；默认 `pageSize: 50`；clamp 1–500 | `pagination.tsx:59-65`；`providers/sql/stores/create-sql.store.ts:254`、`:180-183` | — |
| 订阅 schema 卡片 | `databaseCardClassName("mx-3 mb-3 divide-y divide-border/60")`；行 `px-3 py-2 hover:bg-accent` | `components/schema-view.tsx:65,72` | 12·8 |
| Info 视图 | `divide-y divide-border`，三块各 `p-3`；统计行 `gap-4 ui-text-sm` | `components/info-view.tsx:40,42,44` | 12 / gap 16 |
| SQL 历史卡 | 标题 `p-2` + `px-2 py-1 uppercase`；列表 `max-h-32`（compact）/ `max-h-56 px-1`；行按钮 `px-2.5 py-1.5` | `components/sql-history-list.tsx:40-41,56,70` | 8；**128 / 224** |
| 历史条数上限 / 预览长度 | `SQL_HISTORY_LIMIT = 10`、`SQL_HISTORY_PREVIEW_LIMIT = 96` | `lib/sql-history.ts:1-2` | — |
| 连接对话框 | `Dialog size="md" max-w-md`；backdrop `bg-black/40 backdrop-blur-[2px]`；`classNames.modal: "max-w-md"`、`content: "space-y-4"` | `components/connection/connection-dialog.tsx:221-225`；`ui/dialog.tsx:59,65-67` | **≤448** |
| Dialog 通用 | `max-h-[90vh] max-w-[calc(100vw-2rem)] rounded-xl border bg-background shadow-(--shadow-dialog)`；size sm 384 / md 448 / lg 512；header `px-4 pt-4 gap-1.5`；footer `border-t px-4 py-3` | `ui/dialog.tsx:59,64-67,148,159` | 圆角 11.2；16·16；16·12 |
| 右键菜单（单元格/行/表） | `Dropdown`：菜单容器 `min-w-44 rounded-md bg-surface p-1 text-foreground shadow-(--shadow-popover) ring-1 ring-border/70`，z-10070；菜单项 default `gap-3 rounded-lg px-2.5 py-1.5 ui-text-sm` | `ui/dropdown.tsx:753,39` | 最小 **176** / 圆角 6.4 / 4；圆角 8 / 10·6 |
| MongoDB 文档卡 | `databaseCardClassName("group p-3 shadow-[0_10px_30px_-28px_rgba(0,0,0,0.55)]")`，卡片间距 `space-y-2` | `providers/mongodb/mongodb-viewer.tsx:206,217-219` | 12 / gap 8 |
| MongoDB Filter / Sort 输入 | Filter `flex-1`；Sort `w-56` | `mongodb-viewer.tsx:125,133` | 224 |
| Redis key 行 | `h-auto w-full justify-start gap-1.5 px-2 py-1 leading-row`，选中 `bg-selected`；type Badge `px-1.5 uppercase` | `providers/redis/redis-viewer.tsx:155-167` | 6 / 8·4 / 行高 1.35 |
| Redis key 值 `min-w-35` | `font-sans min-w-35 text-subtle-foreground` | `providers/redis/redis-viewer.tsx:218` | 140 |
| 空态 / 加载 / 错误 | `Empty`：`flex-1 items-center justify-center gap-2 rounded-lg border-dashed p-3`；描述 `ui-text-sm leading-relaxed`；`Spinner` `size-4`（compact `size-3`）；结果网格空态 `EmptyState(message=t("database.noData"))` | `ui/empty.tsx:18,84`；`ui/spinner.tsx:27`；`components/data-grid.tsx:395-397` | 圆角 8 / 12；16 / 12 |

### 2.2 AI 助手

| 元素 | 类名 / 值 | 证据 | px |
| --- | --- | --- | --- |
| 面板内容最大宽 | `mx-auto size-full max-w-4xl` | `ai/components/agent-tab.tsx:29` | **896** |
| 根字体 | `ui-text-sm` | `ai/components/chat/ai-chat.tsx:1018` | 13 |
| 初始态容器 | `px-8 py-10`；内容 `max-w-180` | `ai-chat.tsx:1045-1046` | 32·40；**720** |
| header 行 | pane chrome：`min-h-7 items-center gap-1.5 px-1.5 py-1`；`relative z-10020 bg-background` | `features/panes/components/pane-chrome.tsx:6`；`chat-header.tsx:146` | ≥28 / 6 / 6·4 |
| header 图标 chip | `h-5`；图标容器 `size-6` | `pane-chrome.tsx:11`；`chat-header.tsx:150` | 20 / 24 |
| 会话标题 | `truncate rounded-md px-2 py-1 ui-text-sm` | `chat-header.tsx:72` | 6.4 / 8·4 |
| 重命名输入 | `min-w-24 max-w-52` | `chat-header.tsx:64` | 96–208 |
| header 动作按钮 | `Button size="icon-xs"` = `size-6` | `chat-header.tsx:168`；`ui/button.tsx:27` | 24 |
| 消息搜索条 | `border-t px-1.5 py-1`；Input `h-7 bg-surface/45`；计数 `min-w-10 text-right` | `chat-header.tsx:206,231,234` | 6·4；28；40 |
| 消息项 padding | 普通 `px-4 py-2`；工具-only `px-4 pt-2 pb-1` / `px-4 py-1`；含计划 `pt-2` | `chat-messages.tsx:137-140` | 16·8；16·8·4；16·4；8 |
| 空态列表 | `justify-end px-4 pb-2 pt-4` + `max-w-sm` | `chat-messages.tsx:100-101` | 16·8·16 / **384** |
| Message | `gap-2 ui-text-sm` | `ui/message.tsx:26` | 8 / 13 |
| MessageContent / Footer / Actions | `gap-1`；`gap-1.5 px-3 ui-text-sm` 色 `subtle-foreground/55`；`mt-2 gap-1.5` | `ui/message.tsx:52,78,104` | 4；6·12；8 / 6 |
| Bubble 最大宽 / 内边距 | 用户气泡 `max-w-[80%]`；ghost 变体 `max-w-full p-0`；BubbleContent `rounded-xl px-3 py-2 leading-relaxed` | `ui/bubble.tsx:16,29,65` | **80%**；圆角 11.2 / 12·8 |
| 编辑态 textarea | `w-full max-w-[80%]`；`min-h-16 resize-y p-0` | `chat-message.tsx:135,151` | 64 |
| 消息操作图标 | `size-3.5` | `ui/message.tsx:133` | 14 |
| Markdown 代码块 | `my-2 p-2 rounded border ui-text-sm` | `ai/components/messages/markdown-renderer.tsx:158-159,190` | 8 / 8 / 圆角 4.8 |
| Markdown 表格 / 引用 / h1 | `my-2 min-w-max ui-text-sm`；`my-2 border-l-2 pl-3`；`mt-3 mb-1.5 ui-text-sm font-semibold` | `markdown-renderer.tsx:512-513,824,348` | 8 / 13；8 / 12；12·6 |
| 错误块详情 | `bg-destructive/8 p-2` | `markdown-renderer.tsx:330` | 8 |
| 计划卡 | `my-2 rounded-2xl`；头 `gap-1.5 px-3 py-2`；步骤区 `space-y-1.5 p-3` | `messages/plan-block-display.tsx:44-45,55` | 8 / 圆角 14.4；6·12·8；6 / 12 |
| 计划步骤 | 容器 `rounded-xl`；执行按钮 `px-2.5 py-2`；描述 `px-3 py-2.5 ui-text-sm` | `messages/plan-step-display.tsx:36,44,60` | 11.2；10·8；12·10 |
| 活动行 | `min-h-4 gap-2 ui-text-sm`；MarkerIcon `size-4`；展开详情 `mt-1 max-h-64 pl-6 ui-text-sm` | `ui/marker.tsx:7,60`；`chat/chat-activity-line.tsx:65` | 16 / 8；16；4 / **256** / 24 |
| 加载指示 | MarkerIcon `size-5` + `ThinkingOrb size={20}`；文字 `ui-text-shimmer` 动画 | `chat/chat-loading-indicator.tsx:26-29`；`styles/utilities.css:84-104` | 20 |
| 权限条 | 外层 `px-3 pt-2`；条 `h-9 rounded-lg px-2 gap-2` | `chat/acp-permission-prompt.tsx:85-86` | 12·8；**36** / 圆角 8 / 8 |
| 后续操作按钮 | `Button size="xs"`（h-6）+ 图标 `size-3.5` | `chat/chat-follow-up-actions.tsx:69-77` | 24 / 14 |
| composer 初始态外框 | `rounded-2xl bg-surface/55 shadow-(--shadow-card)` | `input/chat-composer.tsx:29` | 14.4 |
| composer 默认态外框 | `SidebarFooter`：`mx-2 mb-2 rounded-xl border-border/60 bg-background p-0 pb-1 ui-text-chrome` | `ui/sidebar.tsx:75` | 8 / 圆角 11.2 / 4 |
| 可编辑区 | `max-h-35 min-h-16 px-3 pt-3 pb-2 whitespace-pre-wrap`，`lineHeight: 1.4` | `chat-composer.tsx:77-78,85` | max **140** / min **64** / 12·12·8 |
| 初始态可编辑区 | `max-h-48 min-h-28 px-4 py-4 ui-text-base` | `chat-input-bar.tsx:1025` | **192 / 112** / 16 |
| 工具条 | `items-end gap-2 px-2 pb-2 pt-1`；初始 `px-3 pb-3 pt-0` | `chat-composer.tsx:99`；`chat-input-bar.tsx:1033` | 8·8·8·4 |
| 发送/停止按钮 | `Button size="icon-sm"` = `size-7` | `chat-input-bar.tsx:1162`；`ui/button.tsx:28` | 28 |
| 队列 Badge / 脉冲点 | `px-2.5`；点 `h-1.5 w-1.5` | `chat-input-bar.tsx:1052-1053` | 10；6 |
| 上下文 chip | `gap-1.5 rounded-lg px-1.5 py-1`，media `w-7`；组 `gap-3 py-1`，容器 `px-2 pb-2`（初始 `px-3 pb-3`） | `ui/attachment.tsx:15,56,169`；`chat-input-bar.tsx:1171` | 6·6.4·6·4 / 28；12·4；8·8 |
| 粘贴图片缩略图 | `w-24 … w-30`；组 `px-3 pt-3`（初始 `px-4 pt-4`） | `ui/attachment.tsx:19`；`chat-input-bar.tsx:990` | 96–120；12·12 |
| @mention / /command token | `min-h-6 max-w-45 px-1.5 py-0.5 rounded-full ui-text-sm`（斜杠为 `bg-accent/70`） | `chat-input-bar.tsx:858,899` | 24 / **180** / 6·2 / 13 |
| @ 提及面板 | maxHeight **240**；宽 `min(360, max(220, 输入宽-24))` | `input/mentions/file-mention-dropdown.tsx:97`；`chat-input-bar.tsx:361` | 220–360 |
| / 命令面板 | maxHeight **240**；宽 `min(320, max(180, 输入宽-24))` | `input/mentions/slash-command-dropdown.tsx:89`；`chat-input-bar.tsx:371` | 180–320 |
| 上下文面板 | maxHeight **320**；列表 `max-h-66` = 264 | `selectors/context-selector.tsx:101,260` | 320 / 264 |
| 技能面板 | maxHeight 320（编辑器态 440） | `skills/skills-command.tsx:565` | 320 / 440 |
| 偏好菜单 | 主 `min-w-60`；agent 子菜单 `min-w-56`；provider `min-w-48`；model `max-h-80 min-w-64` | `input/chat-preferences-menu.tsx:323,63,179,197` | 240；224；192；320 / 256 |
| ComposerAttachedPanel | `w-(--anchor-width) max-w-[calc(100vw-16px)] rounded-t-2xl border-b-0`；`maxHeight: min(N, var(--available-height))`；side=top align=start sideOffset=-1 collisionPadding=8 | `input/composer-attached-panel.tsx:35-48` | 圆角 14.4 |
| ProviderApiKeyCommand | `max-h-107.5 w-140`；左列 `grid-cols-[200px_minmax(0,1fr)]` | `provider-api-key-command.tsx:55,179` | **430 × 560** / 200 |
| Agent 侧栏行 | HoverCard `w-72`；行 `min-h-6 px-2 pr-12 ui-text-sm`；pin/archive `size-5`，图标 `size-3` | `agent-session-sidebar-item.tsx:133,81,100,109` | 288；24 / 8·48；20；12 |
| 更多历史下拉 | `maxHeight: 320, width: 240` | `features/layout/components/sidebar/sidebar-history.tsx:334` | 320 × 240 |
| AgentLauncher（无挂载） | `w-[min(680px,calc(100vw-24px))]` | `agent-launcher.tsx:133` | ≤680 |

### 2.3 全局搜索

| 元素 | 类名 / 值 | 证据 | px |
| --- | --- | --- | --- |
| 工具条 | `border-b bg-surface/55 py-2` + `px-2`（compact）/ `px-3` | `components/global-search-toolbar.tsx:93` | 8；8 / 12 |
| 搜索框容器 | `flex h-7 items-center gap-2 rounded-lg border border-border/70 bg-background/65 px-2` | `global-search-toolbar.tsx:101` | **28** / 圆角 8 / 8 |
| 搜索框图标 | `size-4` | `global-search-toolbar.tsx:102` | 16 |
| 结果/警告 Badge | `max-w-56`（结果）/ `max-w-64`（警告）`shrink-0 truncate` | `global-search-toolbar.tsx:150,159` | 224 / 256 |
| 详情区 | `mt-2 space-y-2`；替换行 `border-t pt-1.5`；包含/排除 `grid-cols-2 gap-2`（compact 单列） | `global-search-toolbar.tsx:168-169,185,259` | 8 / 6 / 8 |
| 替换框 | `Input h-8 rounded-lg border-border/80 bg-background py-1`（`ui-text-sm`）；前置图标块 `size-8 rounded-lg border` | `ui/search.tsx:271,260` | 32 / 圆角 8；32 |
| 包含/排除输入 | `h-7 rounded-md border border-border/70 bg-background/65 px-2` | `global-search-toolbar.tsx:190,199` | 28 / 圆角 6.4 |
| 结果区布局 | `flex h-full min-h-0 overflow-hidden`；左导航 `my-2 ml-2 h-auto self-stretch`；右 ScrollArea `min-h-0 flex-1 bg-background`，`contentClassName="px-2 pb-2"`，`orientation="both"` | `components/global-search-results.tsx:60,73,76-79` | 8·8 |
| 文件导航栏宽 | `DEFAULT_FILE_NAVIGATOR_WIDTH = 224`、`MIN = 176`、`MAX = 420`、父宽上限 50%、键盘步长 16、拖拽热区 `-right-1 w-2` | `features/file-explorer/lib/file-navigator-layout.ts:1-5,28`；`components/file-navigator-sidebar.tsx:45,465` | 224 / 176 / 420 |
| 摘录容器 | `rounded-xl bg-background`；代码区外层 `-mt-px rounded-b-xl border-x border-b`；列表 `flex flex-col gap-2 rounded-xl` | `components/search-excerpt-results.tsx:109,140,207` | 圆角 11.2 / gap 8 |
| 摘录最小高 | `minHeight: "104px"`（按编辑器字号动态，见下）；代码区 `border-t bg-background py-2` | `components/search-excerpt-code.tsx:154,164` | **104** / 8 |
| 摘录字号 | `fontSize = 编辑器 settings.fontSize × zoom`；`lineHeight = calculateLineHeight(...)`；`tabSize = settings.tabSize`；行号 gutter 宽 `calculateTotalGutterWidth(max(行数, 最大行号))`，`border-r pr-3 text-right tabular-nums` | `search-excerpt-results.tsx:184-193`；`search-excerpt-code.tsx:59-61,143-150` | 动态 |
| 命中高亮 | 普通命中 `rounded-sm bg-warning/20`；当前命中 `bg-warning/40 ring-1 ring-inset ring-warning/60`；行 hover `hover:bg-accent/25`；不可映射行（"..."）`text-subtle-foreground` | `search-excerpt-code.tsx:80-81,117,111` | 圆角 4.8 |
| 上下文行数 | 默认 `DEFAULT_CONTEXT_LINES = 2`；展开 `EXPANDED_CONTEXT_LINES = 7`；面板内 `CONTEXT_LINES = 2` | `components/global-search-buffer.tsx:24-25`；`hooks/use-content-search.ts:24` | — |
| 结果分页/渲染上限 | `CONTENT_SEARCH_PAGE_SIZE = 140`、`CONTENT_SEARCH_INITIAL_RENDER_LIMIT = 40`、`CONTENT_SEARCH_RENDER_INCREMENT = 40`、防抖 `SEARCH_DEBOUNCE_DELAY = 200` | `constants/limits.ts:6-11` | — |
| 索引轮询 | `INDEX_STATUS_POLL_DELAY = 150ms`；provider 文件缓存 TTL 2000ms | `hooks/use-content-search.ts:25-26` | — |
| 懒加载哨兵 | `IntersectionObserver`，`rootMargin: "640px 0px"` | `global-search-buffer.tsx:497-516` | 640 |
| 语法高亮预取 | `IntersectionObserver`，`rootMargin: "240px 0px"`，首屏仅第 0 项高亮 | `search-excerpt-results.tsx:32-33,80-91` | 240 |
| 底部「加载更多」 | `ui-text-sm px-3 py-3 text-center text-subtle-foreground` | `global-search-results.tsx:96-99` | 12·12 |

### 2.4 快速打开（浮层）

浮层外壳走 `ui/command.tsx`（与命令面板同款）：

| 元素 | 类名 / 值 | 证据 | px |
| --- | --- | --- | --- |
| 浮层容器 | `relative z-10 flex max-h-[min(68vh,32rem)] w-[min(44rem,calc(100vw-2rem))] flex-col overflow-hidden rounded-xl border border-border bg-background shadow-(--shadow-dialog)` | `ui/command.tsx:29` | max-h ≤ **512** / 宽 ≤ **704** / 圆角 11.2 |
| 定位 | 外层 `fixed inset-0 z-10060 flex items-start justify-center pt-16` | `ui/command.tsx:143` | 顶距 **64** / z 10060 |
| 出现动画 | `opacity 0→1, scale 0.98→1, y -8→0, blur(2px)→0`；`prefersReducedMotion` 时 `instantTransition` | `ui/command.tsx:157-168` | — |
| header | `border-b px-4 py-3`；关闭按钮 `Button size="sm"` | `ui/command.tsx:52,80` | 16·12 |
| 输入框 | `h-7 ui-text-base leading-[1.4] placeholder-subtle-foreground` | `ui/command.tsx:55` | 28 / 13 |
| header 徽标 | `h-auto min-h-7 max-w-40 bg-surface/70 px-2 ui-text-base` | `ui/command.tsx:92` | ≥28 / **160** |
| 列表 | `ScrollArea` `flex-1`；viewport `command-list-viewport h-auto min-h-0 flex-1 overscroll-contain`；内容 `p-2` | `ui/command.tsx:244-246`；`styles/utilities.css:6-8` | 8 |
| 列表项（default 密度） | `ui-text-base mb-1 min-h-8 gap-2.5 rounded-lg px-2.5 py-2 leading-row`；选中 `bg-selected`，未选中 hover `bg-accent` | `ui/command.tsx:41,37-38` | min **32** / 10 / 圆角 8 / 10·8 |
| 列表项（compact 密度） | `ui-text-sm mb-0.5 min-h-7 gap-1.5 rounded-md px-2 py-1 leading-normal` | `ui/command.tsx:42` | min **28** / 圆角 6.4 |
| 项图标 | `size-5`（`framed` 变体额外 `rounded-md border bg-surface/70`） | `ui/command.tsx:554-555` | 20 |
| 项徽标 | `h-auto max-w-32` | `ui/command.tsx:567` | **128** |
| 空态 | `ui-text-base p-3 text-center leading-row text-subtle-foreground` | `ui/command.tsx:727` | 12 |
| footer（数据库侧栏用） | `sticky bottom-0 border-t bg-background px-3 py-3`，内部 `flex flex-wrap gap-2` | `ui/command.tsx:343-345` | 12 |
| 快速打开去抖 | `SEARCH_DEBOUNCE_DELAY = 100`；上限 `MAX_RESULTS = 20`、`MAX_OPEN_BUFFERS_SHOWN = 20`、`MAX_RECENT_FILES_NO_QUERY = 10`、`MAX_OTHER_FILES_SHOWN = 20` | `features/quick-open/constants/limits.ts:1-8` | — |
| 按钮尺寸表（全局复用） | `default h-8 px-3`、`xs h-6 px-1.5`、`sm h-7 px-2.5`、`lg h-9 px-4`、`icon size-8`、`icon-xs size-6`、`icon-sm size-7` | `ui/button.tsx:22-28` | 32 / 24 / 28 / 36 / 32 / 24 / 28 |
| Badge 尺寸 | `h-6 rounded-full`，`default px-2 py-0.5` / `compact px-1.5 py-0.5` | `ui/badge.tsx:6,18-19` | 24 / 8·2 / 6·2 |

### 2.5 主题色（默认主题族 lithe-light / lithe-dark）

来源 `windows/tauri/src/extensions/themes/builtin/lithe.json`；映射层 `windows/tauri/src/styles/theme.css:14-49`；写入机制 `theme-registry.ts:92-99`。

| CSS 变量 | 浅色 lithe-light | 深色 lithe-dark |
| --- | --- | --- |
| `--background` | `#ffffff`（lithe.json:12） | `#1e1f22`（:79） |
| `--surface` | `#f7f8fa`（:13） | `#2b2d30`（:80） |
| `--foreground` | `#1f2328`（:14） | `#dfe1e5`（:81） |
| `--muted-foreground` | `#4f5965`（:15） | `#b4b8bf`（:82） |
| `--subtle-foreground` | `#68717d`（:16） | `#8b929e`（:83） |
| `--border` | `#dfe1e5`（:17） | `#43454a`（:84） |
| `--accent` | `#edf3ff`（:18） | `#393b40`（:85） |
| `--selected` | `#d4e2ff`（:19） | `#2e436e`（:86） |
| `--selection` | `rgba(53,116,240,0.2)`（:20） | `#214283`（:87） |
| `--primary` | `#3574f0`（:21） | `#3574f0`（:88） |
| `--destructive` | `#cf3f4f`（:25） | `#db5c5c`（:92） |
| `--success` | `#27864f`（:26） | `#57965c`（:93） |
| `--warning` | `#a86400`（:27） | `#d6ae58`（:94） |

派生（两套主题共用）：`--popover / --muted / --card = var(--surface)`、`--input = var(--border)`、`--border-strong = color-mix(border 72%, foreground 28%)`、`--ring = var(--border-strong)`、`--success = var(--primary)`、`--warning = var(--muted-foreground)`（`windows/tauri/src/styles/theme.css:140-157`）。**注意 `--success`/`--warning` 的语义被改写成 primary/muted-foreground，UI 里 `text-success` / `text-warning` 实际拿到的不是绿/黄。**

---

## 3. 状态与交互

### 3.1 数据库

**连接态**
- `ActiveConnection.status: "connecting" | "connected" | "disconnected" | "error"`（`stores/connection.store.ts:22-28`）；连接成功/失败用请求 id 防竞态（`connection.store.ts:52-73,228-279`）。
- 侧栏行：`connected` 显示 `PlugsConnected` + `database.connected` 徽标；`connecting` 或不匹配的 busy id 时 `disabled`；删除按钮 `tone="danger"`（`components/database-sidebar.tsx:584-616`）。
- viewer 内 `isBusy = store.isLoading || store.isCustomQueryLoading`，busy 时用 `Empty` + `Spinner label=t("ui.loading")` 覆盖内容区（`providers/sql/sql-database-viewer.tsx:67,274-280`）。

**加载 / 失败 / 空态**
- 错误：`Alert tone="error" className="mx-3 mb-2 w-auto"` + `normalizeDatabaseError()`（`sql-database-viewer.tsx:268-272`；错误规范化把 `Database sidecar panic:` / 协议版本不匹配 / 缺字段 / 超时等翻译成人话，`lib/database-errors.ts:20-41`）。
- 空结果：`DataGrid` 在 `rows.length === 0` 时直接返回 `EmptyState(message=t("database.noData"))`，**不渲染表头**（`components/data-grid.tsx:395-397`）。
- 未选表 / 未选集合 / 未选 key：MongoDB `Empty`（`mongodb-viewer.tsx:169-178`）、Redis `Empty`（`redis-viewer.tsx:215+`）。

**右键菜单**
- 表/视图/物化视图/索引：`SqlTableMenu` = 「添加新行」（仅 table）+ 分隔符 + 删除（label 随 kind 变 `deleteView`/`deleteMaterializedView`/`deleteIndex`/`deleteTable`），坐标取自 `e.clientX/clientY`（`components/context-menus.tsx:18-59`；触发点 `sql-database-viewer.tsx:82-95`）。
- 数据行：`SqlRowMenu` = 「编辑行」+「删除行」，行数据按 `columns` 拉平成 `Record`（`context-menus.tsx:62-98`；`sql-database-viewer.tsx:97-112`）。
- 单元格：`Dropdown` 菜单 = 复制值（选中区域时变「复制所选」）+ 条件出现的「复制所选及表头」（`use-cell-copy.ts:12-59`；items `data-grid.tsx:367-384`）。

**结果网格交互**
- 键盘（`data-grid.tsx:268-365`，事件挂在滚动容器上）：`Ctrl/Cmd+C` 复制选区（`Shift` 附带表头）、`Ctrl/Cmd+A` 全选、`Escape` 清编辑+清激活+清锚点、`↑/↓/←/→` 移动激活格（`Shift` 扩选）、`PageUp/PageDown` 移动 10 行、`Tab/Shift+Tab` 按展平索引跨行移动、`Home/End`（带 `Ctrl/Cmd` 时跳首/末行）、`Enter` 进入编辑（主键列与外键列除外）。
- 鼠标：单击设激活+锚点、`Shift+单击` 扩选、双击进入编辑（命中 `button,a,input,textarea` 时跳过）、行号单击选整行（`Shift` 扩选）、表头 `#` 单击全选、表头单击排序、表头 `↕` 分栏手柄拖动改列宽（`pointerdown/move/up` + `setPointerCapture`，最小 60）。
- 选区复制格式：制表符分隔、`\n` 换行；带表头时表头行去重（重名列变 `name_2`）（`utils/data-grid-selection.ts:151-207`）；NULL 复制为字符串 `"NULL"`，对象走 `JSON.stringify(value, null, 2)`（`utils/clipboard.ts:3-15`）。
- 单元格菜单坐标、关闭逻辑：`hooks/use-cell-copy.ts`。

**SQL 编辑器交互**
- 非自定义查询模式只显示「表内过滤」输入 + 250ms 去抖（`components/query-bar.tsx:204-214`）。
- 进入 SQL 模式后 textarea 自动聚焦并把光标放到末尾（`query-bar.tsx:220-242`）；`Cmd/Ctrl+Enter` 执行（`:269-274`）；有选区时只跑选区，按钮文案变「运行所选」（`:252-260,303`）。
- 语法高亮：透明 textarea 覆盖在 `<pre>` 上，`onScroll` 同步 `scrollTop/scrollLeft`；token 来自 `useTokenizer({filePath:"query.sql", bufferId:"database-query-editor", languageIdOverride:"sql", incremental:false})`，失败/为空时回退到内置正则高亮（`query-bar.tsx:95-116,146-150`；`lib/sql-highlight.ts:97-135`）。
- 补全：光标处计算 `getSqlCompletions(value, cursor, {tables, columns})`，非空时在编辑器下方渲染 chips；`Tab` 接受第 0 项；失焦 100ms 后清除（`query-bar.tsx:73-90,138-145,157-178`）。
- 运行中显示「停止」按钮走 `cancelCustomQuery`；执行耗时显示 `database.lastRunMs`（`query-bar.tsx:280-306`）。

**分页**
- `pageSize` 归一化 1–500，默认 50（`create-sql.store.ts:180-183,254`）；自定义查询结果在前端切页（`paginateQueryResult`，`lib/query-result-pagination.ts:31-46`）；表数据模式下由 provider 的 `query_<db>_filtered` 带 `page/page_size` 查询（`create-sql.store.ts:623-649`）。
- 页码输入只接受纯数字，越界回退到当前页（`lib/query-result-pagination.ts:8-21`；`components/pagination.tsx:27-45`）。

**列宽与状态持久化**
- `columnWidths: Record<table, Record<column, number>>` 存在 store 内存态（`create-sql.store.ts:57,264,1256-1259`）；store 由 `createSqlStore()` 经 `useState(() => createStore())` 每次挂载新建（`providers/sql/sql-provider-viewer.tsx:19,36`），且 **store 未接 `persist` 中间件**（`create-sql.store.ts:1-4,350`），所以列宽在关闭 tab 后丢失。
- 唯一持久化的是 SQL 历史：`localStorage`，key `lithe:database:sql-history:v1:<dbType>:<mode>:<connectionKey>`，上限 10 条，写失败静默忽略（`lib/sql-history-storage.ts:6-14,61-84`；`lib/sql-history.ts:1`）。

**连接对话框交互**
- 表单/连接字符串两 Tab（文件型 provider 禁用字符串 Tab）（`connection-dialog.tsx:274-287`）；测试连接按钮仅在非文件型且 `installedDbTypes.length > 0` 时出现（`:228-241`）；失败显示 `database.connectionTestFailed`（`:165`）；成功显示 `MarkerContent` + `database.connectionTestSuccessful`（`:418`）。
- 校验规则（`components/connection/connection-validation.ts:53-77`）：文件型必须选路径；字符串模式必须非空；否则 host 非空、port 为 1–65535 整数、非 redis 时 database 非空。
- 拖放：`database-sidebar.tsx:338-362` 接受拖入的数据库文件；`isDraggingFile` 时叠一层 `absolute inset-1 z-30 ... border-primary bg-background/85 backdrop-blur-sm` 提示（`:653-657`）。

### 3.2 AI 助手

**idle / loading / failed / empty**
- 空会话（`messages.length === 0 && acpEvents.length === 0`）→ 初始态版式：顶部 4 个技能快捷键 + 居中的 initial composer（`ai-chat.tsx:987-988,1044-1083`；`chat-messages.tsx:98-104`）。
- 流式中且尚无内容 → `ChatLoadingIndicator label=t("ai.thinking") state="breathing" compact`，文字用 `ui-text-shimmer` 扫描动画（`chat-message.tsx:209-216`；`chat-loading-indicator.tsx:26-29`；`styles/utilities.css:84-104`）。
- 空响应 → 注入 `[ERROR_BLOCK] title: No Response / code: EMPTY_RESPONSE`（`ai-chat.tsx:565-578`）。
- 失败 → `MarkdownRenderer` 识别 `[ERROR_BLOCK]` 渲染 `ErrorBlock`（标题 + `(code)` + 详情折叠 + `AUTH_REQUIRED`/`CONFIG_REQUIRED` 时的「重启 Agent 会话」「打开 Agent 终端」恢复动作）（`messages/markdown-renderer.tsx:884-893,199-343`）；错误码映射 429/401/403/500/400（`ai-chat.tsx:608-637`）；ACP 配置/认证错误识别（`:639-667`）；`canReconnect` → `Connection Lost / RECONNECT`（`:669-672`）。
- 无 API key：composer 禁用 + placeholder 提示（`chat-input-bar.tsx:133,958-964`）。

**流式渲染**
- 5 类回调驱动单个气泡状态机：`onChunk`（累积原文并实时抽取 follow-up 块）、`onComplete`、`onError`、`onNewMessage`（新建气泡）、`onToolUse/onToolUpdate/onToolComplete`、`onPermissionRequest`（入队）、`onAcpEvent`、`onImageChunk/onResourceChunk`（`ai-chat.tsx:522-902`）。

**自动跟随**
- `MessageScrollerProvider autoScroll defaultScrollPosition="last-anchor"`；消息项 `scrollAnchor={message.role === "user"}`；「回到末尾」按钮 `MessageScrollerButton`（`ai-chat.tsx:1085,1106`；`chat-messages.tsx:154`；`ui/message-scroller.tsx:99`）。**内部实现不在本仓库**（来自 `@shadcn/react/message-scroller`，`ui/message-scroller.tsx:1-6`）。

**发送 / 取消 / 快捷键**
- `Enter` 发送、`Shift+Enter` 换行（`chat-input-bar.tsx:578-581`）。
- 生成中再发送 → 入队（`messageQueueRef`），按钮 tooltip 变 `ai.addToQueue`，队列 Badge 显示数量，出队间隔 500ms（`ai-chat.tsx:925,930-945`；`chat-input-bar.tsx:1051-1058,1156-1158`）。
- 生成中按钮变停止：`isStreaming = isTyping && !!streamingMessageId`；`shortcut={isStreaming ? "escape" : "enter"}`（`chat-input-bar.tsx:134,1142-1165`）——**该 `escape` 仅作 tooltip 展示，未找到对应 keydown 处理**。
- `Cmd/Ctrl+F` 打开消息搜索；搜索框 `Enter` 下一个 / `Shift+Enter` 上一个 / `Esc` 关闭（`ai-chat.tsx:142-147`；`chat-header.tsx:211-226`）。
- `/` 斜杠下拉：`↑↓` 移动、`Enter`/`Tab` 选择、`Esc` 关闭；面板内 `Cmd/Ctrl+P` 转快速打开、`Shift` 转命令面板（`chat-input-bar.tsx:488-503`；`slash-command-dropdown.tsx:56-74`）。
- `@` 提及：`↑↓`、`Enter`/`Tab`、`Esc`；检测条件 = 光标前最后一个 `@` 其后无空格/无 `]` 且 <50 字符，**防抖 150ms**；文本含 `@` 且总长 <500 才启用（`chat-input-bar.tsx:504-521,585-609,645-649`）。
- `/` 触发正则 `(?:^|\s)\/([^\s/]*)$` 且搜索词 <50（`chat-input-bar.tsx:628-639`）。
- 已选上下文 chip：`←/→` 移焦点、`Del/Backspace` 移除（`chat-input-bar.tsx:1184-1228`）。
- 粘贴：图片进 `pastedImages`，纯文本剥格式插入（`chat-input-bar.tsx:772-835`）。
- 取消生成：`stopStreaming` → Codex `cancel()` 或 `AcpStreamHandler.cancelPrompt()`，并批量拒绝挂起权限（`ai-chat.tsx:330-364`）。

**权限提示**
- FIFO 队列，只展示 `permissionQueue[0]`，其余显示 `+N`（`ai-chat.tsx:986,1111-1117`；`acp-permission-prompt.tsx:96-100`）。
- 选项标签按 `kind` 映射：`allow_once`→允许 / `allow_always`→始终 / `reject_once`→拒绝 / `reject_always`→永不（`acp-permission-prompt.tsx:20-33`）；无 options 时回退 `Deny`/`Allow`（**硬编码英文**，`:15-18`）；响应写入事件时间线（`ai-chat.tsx:989-1013`）。

**工具调用 / 计划**
- `ChatActivityLine` 本地 `isExpanded`，有 children 时可点，`CaretRight` 旋转 90°；详情 `<pre>` 显示 kind/locations/input/output(diff 或文本)/error；动作按钮：打开 diff（kind 为 edit/delete/move 或含 diff 输出）、打开文件、打开终端（`chat-activity-line.tsx:33,52-59`；`messages/tool-call-display.tsx:316-384`）。
- 计划步骤状态 `pending | current | completed`，图标 Circle/Play/CheckCircle2，色 subtle-foreground/primary/success；但 `getStepStatus` 实际只返回 `current`（首个）或 `pending`（`messages/plan-step-display.tsx:15-31`；`plan-block-display.tsx:31-34`）；「执行计划」把 step 0 作为提示词发送（`:24-29`），单步模板 `ai.executePlanStepPrompt`（`chat-message.tsx:94-105`）。

**消息操作**
- 用户消息 copy + edit；编辑可用条件 = agent 为 `custom` 且有 key 且未流式（`ai-chat.tsx:1094-1099`；`chat-message.tsx:176-194`）；编辑提交走 `replaceUserMessage` 截断后续消息（`stores/ai-chat/chat-actions.ts:320-344`）。
- 助手消息 copy（桌面默认 hover 才显形）（`chat-message.tsx:303-313`）。**未找到 retry（重试）动作。**

**历史下拉**
- 打开时清空搜索并聚焦输入；`Esc` 关闭并回焦触发按钮；`↑↓`/`Enter` 选择；过滤按 title 或 agentId；标签为置顶 / 已归档 / agent 名 / 相对时间；动作为恢复归档与删除；空态与无匹配文案（`history/chat-history-dropdown.tsx:62-97,51-60,164-198,128-131`）。

**其他**
- 权限/adapter 直连意图（"open X on web" / "in terminal"）在本地解析并直接开 Web Viewer / 终端；`ui_action` 事件同路径（`ai-chat.tsx:477-515`；`lib/acp-ui-intents.ts:38-55`；`acp-stream-handler.ts:447-476`）。

### 3.3 全局搜索

**唤起方式（关键）**
- 命令 `workbench.showGlobalSearch`，默认键位 **`cmd+shift+f`** → `openGlobalSearchSidebar()`（设置 `activeSidebarView = "search"` + `isSidebarVisible = true`）（`windows/tauri/src/features/keymaps/commands/command-registry.ts:644-650`；`features/layout/actions/workbench-tool-window-actions.ts:37-49`）。
- 命令 `workbench.showProjectSearch`，默认键位 `cmd+shift+h` → 同一函数（`command-registry.ts:657-663`）。
- 活动栏放大镜按钮，tooltip `workbench.search` + 快捷键 `Mod+Shift+F`（`sidebar-pane-selector.tsx:134-150`）。
- 活动栏右键菜单「搜索」（`main-sidebar.tsx:717-720`）。
- 打开后 `requestAnimationFrame` 聚焦并全选搜索框（`global-search-buffer.tsx:395-402`）。
- **不是浮层**：`isGlobalSearchVisible` / `openGlobalSearchBuffer` 均无调用方（见 §1.0）。

**状态机**（`hooks/use-content-search.ts` + `components/global-search-state.tsx`）
- `availability`：`no-workspace`（未开项目）/ `unsupported`（`remote://`、`diff://`、非 `wsl://` 的其它协议头）/ `ready`（`use-content-search.ts:28-41`）。
- `isSearchPending` = 查询未去抖完成，或 `resultsSearchKey !== searchKey`（`:144-148`）。
- busy 文案优先级：索引中 → `Indexing N files`；pending → `Preparing search`；搜索中 → `Searching a/b files`；加载更多 → `Loading more results`（`global-search-buffer.tsx:451-486`，**英文硬编码**）。
- 结果标签：`N results (M total)`（`global-search-buffer.tsx:487-491`，**英文硬编码、未走 i18n**）。
- 正则非法时后端返回 `regex_fallback_error` → 转成 `searchWarning` = `Invalid regular expression; showing literal matches`（`stores/global-search.store.ts:57-58`，**英文硬编码**），并以 `Badge variant="warning"` 顶替结果标签（`global-search-toolbar.tsx:147-156`）。
- 错误：`Empty role="alert"` + `search.failed` + 错误详情 + 「重试」按钮 → `refreshSearch()`（`global-search-state.tsx:85-99`）。
- 无结果：`search.noResults` + `search.noResultsFor` / `search.noResultsForWithFilters`（`:101-114`）。
- 索引轮询：`fffScanStatus` 每 150ms 轮询，连续 3 次失败 → `failIndexing`（`use-content-search.ts:369-425`）。

**键盘**
- 结果间导航由 `useKeyboardNavigation` 处理：`Esc` 或 `Ctrl/Cmd+K` 关闭（有查询时先清空查询，否则失焦）；`↑/↓` 首尾环绕；`Enter` 打开选中项；带修饰键的按键直接放行；**不挂全局监听**（`listenGlobally: false`）（`hooks/use-keyboard-navigation.ts:69-102,104-109`；调用点 `global-search-buffer.tsx:193-225`）。
- 选中项滚动：通过 `data-excerpt-index` 找 `<section>` 并 `scrollIntoView({block:"nearest"})`（`global-search-buffer.tsx:209-222`）。

**替换**
- `canReplace = 有去抖查询 && displayedCount > 0 && 无 searchWarning && 未在替换 && 非初始 busy`；`canReplaceAll = canReplace && !hasMoreResults`（`global-search-buffer.tsx:492-495`）。
- 「全部」在仍有未加载结果时禁用并提示 `Load all search results before replacing all`（**英文硬编码**）（`global-search-buffer.tsx:539-541`）。
- 替换成功 toast `search.replacedMatches`，失败 toast `search.replaceMatchesFailed`（`global-search-buffer.tsx:376,380`）。
- 替换后自动 `refreshSearch()` 重查（`:343-345,374-377`）。

**排序 / 分组**
- 结果按文件聚合，摘录按文件顺序排列；左导航列出有命中的文件并带命中计数（`global-search-buffer.tsx:143-161`）。
- 选中文件时优先渲染该文件的摘录并 `block:"start"` 滚动（`:238-257`）。
- 上下文展开：首次展开按需 `readFileContent(filePath)` 读全文，行数 2 → 7（`global-search-buffer.tsx:284-319`；`constants` 见 §2.3）。

### 3.4 快速打开

- 唤起：命令 `file.quickOpen`，默认键位 **`cmd+p`**（`features/keymaps/defaults/default-keymaps.ts:490`；预设里还有 `cmd+shift+n`、`cmd+shift+o`，`keymaps/defaults/keybinding-presets.ts:95,120,163`）；命令面板「转到：快速打开」（`features/command-palette/constants/navigation-actions.tsx:102-113`）；标题栏按钮（`features/window/components/title-bar/title-bar.tsx:242,272,346`）。
- 输入框内允许触发的命令白名单：`file.quickOpen`、`workbench.commandPalette`（`windows/tauri/src/features/keymaps/hooks/use-keymaps.ts:38`）。
- 模式：`@` 当前文件符号、`#` 工作区符号，其余为文件（`use-quick-open.ts:47-48`）。
- 键盘：`Esc` 或 `Ctrl/Cmd+K` 关闭；`↑/↓` 首尾环绕；`Enter` 选中（`hooks/use-keyboard-navigation.ts:69-101`）；选中项滚动 `block:"nearest"`（`:104-117`）。
- 选区保持：结果变化时按 path 记住选中项，找不到则 clamp（`use-keyboard-navigation.ts:45-61`）。
- 打开行为：写最近文件 → `handleFileSelect` → 关闭（`use-quick-open.ts:154-165`）；工作区符号会先压入 jump-list 再按 1-based 行列打开（`:120-152`，注释说明 LSP 位置是 0-based）。
- 空态优先级：无根目录 → `quickOpen.openFolder`；有去抖查询 → `quickOpen.noMatch`；有查询但未去抖 → `quickOpen.searching`；无文件 → `quickOpen.noFilesInProject`；否则 `quickOpen.noFiles`（`components/empty-state.tsx:23-37`）。

---

## 4. 数据与命令

### 4.1 数据库

前端调用面（全部经 `@/platform/tauri-core` 的 `invoke`）：

| 前端命令字符串 | 发起处 | capability 判定 | Rust 落点 |
| --- | --- | --- | --- |
| `run_database_provider_command` | `features/database/services/database-provider-sidecar.ts:52`（payload `{providerId, command, payload}`） | `database`（`tauri-core.ts:146-147`，因为含 `database`）→ **false**，前端 reject | **未找到** |
| `list_saved_connections` | `stores/connection.store.ts:213,318` | `database`（`tauri-core.ts:148`）→ **false** | **未找到** |
| `connect_database` | `connection.store.ts:251` | `database`（含 `database`）→ **false** | **未找到** |
| `disconnect_database` | `connection.store.ts:289` | `database` → **false** | **未找到** |
| `save_connection` | `connection.store.ts:302` | `database`（`tauri-core.ts:149`）→ **false** | **未找到** |
| `delete_saved_connection` | `connection.store.ts:346` | `database`（`tauri-core.ts:150`）→ **false** | **未找到** |
| `test_connection` | `connection.store.ts:371` | `database`（`tauri-core.ts:151`）→ **false** | **未找到** |
| `store_db_credential` | `connection.store.ts:354` | `database`（含 `db_credential`）→ **false** | **未找到** |
| `get_db_credential` | `connection.store.ts:362` | `database` → **false** | **未找到** |

provider 子命令命名规则（`getSqlProviderCommandMap`，`providers/sql/stores/create-sql.store.ts:137-150`）：

- `get_<db>_tables`、`query_<db>`、`query_<db>_filtered`、`execute_<db>`、`insert_<db>_row`、`update_<db>_row`、`update_<db>_row_by_values`、`delete_<db>_row`、`delete_<db>_row_by_values`、`get_<db>_foreign_keys`（`db` ∈ `sqlite|duckdb|postgres|mysql`）
- Postgres 额外 schema 命令 `get_postgres_table_schema` / `get_mysql_table_schema`（`:152-154`）
- Postgres 订阅：`get_postgres_subscription_info` / `get_postgres_subscription_status` / `create_postgres_subscription` / `drop_postgres_subscription` / `set_postgres_subscription_enabled` / `refresh_postgres_subscription`（`:128-135`）
- MongoDB：`get_mongo_databases` / `get_mongo_collections` / `query_mongo_documents` / `insert_mongo_document` / `update_mongo_document` / `delete_mongo_document`（`providers/mongodb/stores/mongodb.store.ts:7-14`）
- Redis：`redis_scan_keys` / `redis_get_value` / `redis_set_value` / `redis_delete_key` / `redis_get_info`（`providers/redis/stores/redis.store.ts:7-13`）

命令串 → provider 的解析：按 `sqlite|duckdb|postgres|mysql|mongo|redis` token 匹配，多匹配报 `Ambiguous database provider command`，无匹配报 `Cannot resolve database provider for command`（`services/database-provider-sidecar.ts:4-36`）。

**驱动位置结论**：规格设计意图是**独立 sidecar 子进程**（M1：扩展清单里每个 provider 声明跨平台 `sidecar` 可执行文件名，`windows/tauri/src/extensions/database/database-provider-extensions.ts:25-31` 等；打包脚本 `cargo build -p lithe-database --no-default-features --features <provider> --bin lithe-db-<provider>`，`windows/tauri/src/extensions/tooling/package-database-sidecars.ts:136`；错误处理里有 `Database sidecar panic:` / `Unsupported database sidecar protocol version` / `Database sidecar timed out` 分支，`lib/database-errors.ts:20-41`）。但**本仓库没有 `lithe-database` crate，也没有任何 Tauri 侧 spawn/stdio 代码**（`main.rs` 无注册、`platform.rs` 无 `database` 命中、全仓库无 `name = "lithe-database"` 的 Cargo.toml）。因此现状是：**规格 = 独立进程 sidecar + Tauri 中转命令；实现 = 缺失**。

### 4.2 AI 助手

| 通道 | 命令 / 事件 | 发起处 | Rust 落点 |
| --- | --- | --- | --- |
| 密钥（原生命令） | `store_secure_secret` / `get_secure_secret` / `remove_secure_secret` | `features/ai/services/ai-token-service.ts:22,35,48` | `src-tauri/src/secure_storage.rs:12,19,28`（已注册 `main.rs:143-145`） |
| 聊天库（经 `platform_invoke`） | `init_chat_database` / `save_chat` / `update_chat_metadata` / `load_all_chats` / `load_chat` / `delete_chat` | `services/ai-chat-history-service.ts:59,177,187,199,226,241` | **未找到** |
| ACP（经 `platform_invoke`） | `get_acp_status` / `start_acp_agent` / `install_acp_agent` / `get_available_agents` / `send_acp_prompt` / `respond_acp_permission` / `stop_acp_agent` / `cancel_acp_prompt` / `list_acp_sessions` / `delete_acp_session` / `logout_acp_agent`；另有 mode/config 两个在 `stores/ai-chat/acp-actions.ts:39,69` | `services/acp-stream-handler.ts:99,119,154,166,169,171,641,655,664,673,679,684,699,706`；`hooks/use-agent-options.ts:38,95` | **未找到** |
| ACP 事件 | `listen<AcpEvent>("acp-event")` | `services/acp-stream-handler.ts:305-309`；UI 同步 `components/chat/ai-chat.tsx:158-199` | **未找到** |
| Codex | `start_codex_thread` / `start_codex_turn` / `respond_codex_request` / `interrupt_codex_turn` / `get_codex_status`；事件 `codex-event` | `integrations/codex/codex-integration-service.ts:69,76,83,125,203,208,217`；`codex-settings.tsx:48,50-53,166,170` | **未找到** |
| 文件读取（@ 提及内容注入，原生命令） | `read_file_custom` | `lib/file-mentions.ts:30` | `src-tauri/src/host.rs`（注册 `main.rs:163`） |
| 直连 provider HTTP | 无 Tauri 命令；`fetch`（必要时用 `@tauri-apps/plugin-http`） | `services/ai-chat-service.ts:1,292-298`；`services/web-content-service.ts:1,13` | 不涉及 |
| 硬能力判定 | 上述 ACP/Codex/chat 命令全部命中 capability `agent` → `agent: false` → 前端 reject（`待开发`） | `platform/tauri-core.ts:172-180`；`config/backend-capabilities.ts:4`；reject 点 `tauri-core.ts:99-104` | — |

超时（前端）：ACP status 5s / start 15s / prompt 10s / 首响应 20s（`acp-stream-handler.ts:42-45`）。prompt 组装为 `AcpPromptContentBlock[]`：`text` +（`acpAgentCapabilities.promptCapabilities.embeddedContext` 为真时 `resource`，否则 `resource_link`）；slash 命令必须为首 token（`:247-303`）。**Windows 侧 stdio/spawn 细节未找到**（`rust/lithe-core/src` 无 `acp` 命中；`shared/contracts/` 无 AI chat/ACP 契约；`rust/lithe-core/src/ai/{configuration,generation}.rs` 只服务 AI 提交信息生成，与聊天面板无关）。

TS 类型：`features/ai/types/ai-chat.types.ts`（`Message` `:37-50`、`Chat` `:55-69`、`ToolCall` `:14-25`、`AIChatProps` `:71-84`）、`types/acp.types.ts`（`AcpEvent` 20 种联合 `:173-288`、`AgentConfig` `:3-16`、`AcpPermissionOption` `:139-143`、`SlashCommand` `:87-91`、`SessionConfigOption` `:106-116`）、`types/chat-ui.types.ts:1-8`、`types/chat-composer.types.ts:1-28`。

### 4.3 全局搜索 / 快速打开

| 用途 | 命令 / 事件 | 发起处 | Rust / Core 落点 |
| --- | --- | --- | --- |
| 内容搜索 | Core 命令 `workspace.search`，payload `{root, query, caseSensitive, wholeWords, regularExpression, maxResults, fileMask}`（每个 root 各发一次，`Promise.all`） | `features/file-search/lib/file-search-api.ts:87-152` | 命令枚举 `rust/lithe-core/src/protocol/command.rs:325`（`WorkspaceSearch`）；契约测试 `rust/lithe-core/src/tests/project.rs:57,143,170,297` |
| 文件清单 / 索引状态 | Core 命令 `workspace.snapshot`，payload `{root}` → `{root, files[]}` | `file-search-api.ts:158-168,200-207` | `rust/lithe-core/src/protocol/command.rs:320`（`WorkspaceSnapshot`） |
| Core 调用通道 | 原生命令 `core_execute`（`@/core/lithe-core-client` 的 `executeCore`） | `windows/tauri/src/platform/tauri-core.ts:27`（native 白名单） | `windows/tauri/src-tauri/src/core.rs:1-17` → `lithe_core::execute_json` |
| 取消 | 原生命令 `core_cancel` | `tauri-core.ts:26` | `windows/tauri/src-tauri/src/core.rs:19-22` |
| 替换 | 无 Core 命令；前端读全文 → 文本替换 → **走文档写入命令** | `features/global-search/utils/source-replace.ts`（`replaceNextInSource` / `replaceAllInSources`，调用点 `global-search-buffer.tsx:332-372`） | 未逐行核对（见 §7） |
| 快速打开的文件清单 | `workspace.snapshot`（同上传） | `features/quick-open/hooks/use-file-loader.ts:12-19,45-63` | 同上 |
| 快速打开的模糊检索 | **无后端命令**：`fffSearchFiles` 在前端对文件清单做 `includes` + 位置计分 | `file-search-api.ts:170-181`；`hooks/use-fff-search.ts:33` | — |
| 快速打开的工作区符号 | LSP 请求（`LspClient.getWorkspaceSymbols`，逐工作区并发） | `features/quick-open/hooks/use-workspace-symbol-search.ts:81-88` | LSP 侧（`windows/tauri/src-tauri/src/lsp.rs` 为 Tauri 侧；具体方法名未核对） |
| 可达性说明 | `fffEnsureWorkspaces` / `fffTrackAccess` 存在但**无调用方**（`file-search-api.ts:154-156,196-198`） | — | — |

---

## 5. 文案（`windows/tauri/src/i18n/locale.ts` 键名 → 中文原文）

中文目录从 `windows/tauri/src/i18n/locale.ts:4468` 起。以下键名与中文原文逐条照抄。

### 5.1 数据库（`database.*` + 相关）

| key | 中文 |
| --- | --- |
| database.selectDatabaseFile | 选择数据库文件（:5146） |
| database.enterConnectionString | 输入连接字符串（:5147） |
| database.enterHost | 输入主机（:5148） |
| database.enterValidPort | 输入有效端口（:5149） |
| database.enterDatabaseName | 输入数据库名称（:5150） |
| database.openWorkspaceBeforeAdding | 请先打开工作区再添加数据库。（:5151） |
| database.dropSupportedDatabaseFile | 拖入 SQLite 或 DuckDB 数据库文件。（:5152） |
| database.databaseFilePathMissing | 数据库文件路径缺失。（:5153） |
| database.backToCommands | 返回命令（:5154） |
| database.searchDatabases | 搜索数据库（:5155） |
| database.addDatabase | 添加数据库（:5156） |
| database.databases | 数据库（:5157） |
| database.noProvidersInstalled | 未安装数据库提供商。（:5158） |
| database.openExtensions | 打开扩展（:5159） |
| database.chooseOrDropFile | 选择或拖入 {provider} 文件（:5160） |
| database.host | 主机（:5161） |
| database.port | 端口（:5162） |
| database.database | 数据库（:5163） |
| database.username | 用户名（:5164） |
| database.password | 密码（:5165） |
| database.savePasswordSecurely | 安全保存密码（:5166） |
| database.openWorkspaceToAdd | 打开工作区后可添加数据库。（:5167） |
| database.loadingDatabases | 正在加载数据库（:5168） |
| database.noMatchingDatabases | 没有匹配的数据库。（:5169） |
| database.noWorkspaceDatabases | 此工作区中没有数据库。（:5170） |
| database.connected | 已连接（:5171） |
| database.deleteConnection | 删除 {name}（:5172） |
| database.detected | 已检测（:5173） |
| database.dropDatabaseFile | 拖入数据库文件（:5174） |
| database.connectionTestFailed | 连接测试失败（:5175） |
| database.connectToDatabase | 连接到数据库（:5176） |
| database.testConnection | 测试连接（:5177） |
| database.testing | 正在测试（:5178） |
| database.test | 测试（:5179） |
| database.openDatabase | 打开数据库（:5180） |
| database.connect | 连接（:5181） |
| database.connecting | 正在连接（:5182） |
| database.installProviderFromExtensions | 请从“设置 > 扩展”安装数据库提供商后再连接数据库。（:5183） |
| database.formMode | 表单模式（:5184） |
| database.form | 表单（:5185） |
| database.connectionStringMode | 连接字符串模式（:5186） |
| database.connectionString | 连接字符串（:5187） |
| database.connectionName | 连接名称（:5188） |
| database.databaseFile | 数据库文件（:5189） |
| database.selectSqliteDatabaseFile | 选择 SQLite 数据库文件（:5190） |
| database.connectionTestSuccessful | 连接测试成功（:5191） |
| database.data | 数据（:5192） |
| database.schema | 结构（:5193） |
| database.info | 信息（:5194） |
| database.visibleRows | {count} 行可见数据（:5195） |
| database.queryRows | {count} 行查询结果（:5196） |
| database.visibleQueryRowsOnPage | 第 {page}/{pages} 页，{count} 行可见查询结果（:5197） |
| database.exportVisibleQueryPageCsv | 将当前查询页导出为 CSV（:5198） |
| database.exportVisiblePageCsv | 将当前页导出为 CSV（:5199） |
| database.copyVisibleQueryPageJson | 将当前查询页复制为 JSON（:5200） |
| database.copyVisiblePageJson | 将当前页复制为 JSON（:5201） |
| database.exportAsCsv | 导出为 CSV（:5202） |
| database.copyAsJson | 复制为 JSON（:5203） |
| database.switchToView | 切换到{view}视图（:5204） |
| database.toggleColumnTypes | 切换列类型显示（:5205） |
| database.hideColumnTypes | 隐藏列类型（:5206） |
| database.showColumnTypes | 显示列类型（:5207） |
| database.openSqlEditor | 打开 SQL 编辑器（:5208） |
| database.createSubscription | 创建订阅（:5209） |
| database.disableSubscription | 禁用订阅（:5210） |
| database.enableSubscription | 启用订阅（:5211） |
| database.refreshSubscription | 刷新订阅（:5212） |
| database.dropSubscription | 删除订阅（:5213） |
| database.insertSqlCompletion | 插入 SQL {detail} {label}（:5214） |
| database.selectionWillRun | 将运行所选内容（:5215） |
| database.lastRunMs | 上次运行 {ms}ms（:5216） |
| database.cmdCtrlEnterToRun | 按 Cmd/Ctrl+Enter 运行（:5217） |
| database.runSelection | 运行所选（:5218） |
| database.execute | 执行（:5219） |
| database.rowsPerPage | 每页行数（:5220） |
| database.perPage | 每页（:5221） |
| database.currentPage | 当前页（:5222） |
| database.addRowToTable | 向 {table} 添加行（:5223） |
| database.editRowInTable | 编辑 {table} 中的行（:5224） |
| database.required | 必填（:5225） |
| database.optional | 可选（:5226） |
| database.addRow | 添加行（:5227） |
| database.saveChanges | 保存更改（:5228） |
| database.createNewTable | 创建新表（:5229） |
| database.tableName | 表名（:5230） |
| database.enterTableName | 输入表名（:5231） |
| database.columns | 列（:5232） |
| database.columnName | 列名（:5233） |
| database.setColumnNotNull | 将 {column} 设为非空（:5234） |
| database.columnNumber | 第 {number} 列（:5235） |
| database.removeColumn | 移除 {column}（:5236） |
| database.addColumn | 添加列（:5237） |
| database.createTable | 创建表（:5238） |
| database.expandJson | 点击展开 JSON（:5239） |
| database.expandCell | 点击展开（:5240） |
| database.filterEquals / filterNotEquals | `=` / `!=`（:5241-5242，与英文同值） |
| database.filterContains | 包含（:5243） |
| database.filterStartsWith | 开头为（:5244） |
| database.filterEndsWith | 结尾为（:5245） |
| database.filterGreaterThan / GreaterThanOrEqual / LessThan / LessThanOrEqual | `>` / `>=` / `<` / `<=`（:5246-5249，与英文同值） |
| database.filterBetween | 介于（:5250） |
| database.filterIsNull | 为 NULL（:5251） |
| database.filterIsNotNull | 非 NULL（:5252） |
| database.filtersCount | {count} 个筛选器（:5253） |
| database.addFilter | 添加筛选器（:5254） |
| database.add | 添加（:5255） |
| database.clearAllFilters | 清空所有筛选器（:5256） |
| database.clearAll | 全部清空（:5257） |
| database.value | 值（:5258） |
| database.to | 到（:5259） |
| database.removeFilter | 移除筛选器（:5260） |
| database.copySelection | 复制所选（:5261） |
| database.copyValue | 复制值（:5262） |
| database.copySelectionWithHeaders | 复制所选及表头（:5263） |
| database.noData | 无数据（:5264） |
| database.resultCount | {count} {label}（:5265） |
| database.rows | 行（:5266） |
| database.databaseRows | 数据库行（:5267） |
| database.selectAllVisibleCells | 选择所有可见单元格（:5268） |
| database.filterByColumn | 按 {column} 筛选（:5269） |
| database.recent | 最近（:5270） |
| database.clearRecentQueries | 清空最近查询（:5271） |
| database.openQuery | 打开查询：{preview}（:5272） |
| database.runQueryFromHistory | 运行历史查询：{preview}（:5273） |
| database.runQuery | 运行查询（:5274） |
| database.copyQueryFromHistory | 复制历史查询：{preview}（:5275） |
| database.copyQuery | 复制查询（:5276） |
| database.removeQueryFromHistory | 从历史中移除查询：{preview}（:5277） |
| database.removeFromHistory | 从历史中移除（:5278） |
| database.selectDatabase | 选择数据库（:5279） |
| database.collectionsCount | {count} 个集合（:5280） |
| database.collections | 集合（:5281） |
| database.selectCollection | 选择集合 {collection}（:5282） |
| database.mongodbFilterQuery | MongoDB 筛选查询（:5283） |
| database.mongodbSortQuery | MongoDB 排序查询（:5284） |
| database.applyQuery | 应用查询（:5285） |
| database.apply | 应用（:5286） |
| database.resetQuery | 重置查询（:5287） |
| database.reset | 重置（:5288） |
| database.refresh | 刷新（:5289） |
| database.selectCollectionTitle | 选择集合（:5290） |
| database.selectCollectionDescription | 从侧边栏选择集合以浏览文档。（:5291） |
| database.documentsCount | {count} 个文档（:5292） |
| database.documentNumber | 文档 {number}（:5293） |
| database.deleteDocument | 删除文档 {id}（:5294） |
| database.noDocumentsFound | 未找到文档（:5295） |
| database.emptyFilterResult | 当前筛选条件返回了空结果集。（:5296） |
| database.documentsPerPage | 每页文档数（:5297） |
| database.pageOf | 第 {page}/{pages} 页（:5298） |
| database.firstPage | 第一页（:5299） |
| database.lastPage | 最后一页（:5300） |
| database.toggleServerInfo | 切换服务器信息（:5301） |
| database.refreshKeys | 刷新键（:5302） |
| database.refreshingKeys | 正在刷新键（:5303） |
| database.keyPatternPlaceholder | 模式（例如 user:*）（:5304） |
| database.keyPattern | 键模式（:5305） |
| database.searchKeys | 搜索键（:5306） |
| database.scanningKeys | 正在扫描键（:5307） |
| database.selectKey | 选择键 {key}（:5308） |
| database.loadingMoreKeys | 正在加载更多键（:5309） |
| database.loadingKeysEllipsis | 正在加载键...（:5310） |
| database.moreKeysEllipsis | 更多键...（:5311） |
| database.serverInfo | 服务器信息（:5312） |
| database.deleteKey | 删除键（:5313） |
| database.selectKeyTitle | 选择键（:5314） |
| database.selectKeyDescription | 从侧边栏选择 Redis 键以查看其值。（:5315） |
| database.create | 创建（:5316） |
| database.name | 名称（:5317） |
| database.publications | 发布（:5318） |
| database.slotName | Slot 名称（:5319） |
| database.leaveBlankForDefault | 留空则使用默认值（:5320） |
| database.enabled | 已启用（:5321） |
| database.createSlot | 创建 Slot（:5322） |
| database.copyExistingData | 复制现有数据（:5323） |
| database.connectImmediately | 立即连接（:5324） |
| database.enableFailoverSlotSync | 启用故障转移 Slot 同步（:5325） |
| database.objectsCount | 对象（{count}）（:5326） |
| database.tables | 表（:5327） |
| database.views | 视图（:5328） |
| database.materializedViews | 物化视图（:5329） |
| database.subscriptions | 订阅（:5330） |
| database.indexes | 索引（:5331） |
| database.selectObject | 选择 {kind} {name}（:5332） |
| database.onOwner | 位于 {owner}（:5333） |
| database.objects | 对象（:5334） |
| database.tablesCount | {count} 张表（:5335） |
| database.indexesCount | {count} 个索引（:5336） |
| database.currentObject | 当前：{name}（:5337） |
| database.recentQueries | 最近查询（:5338） |
| database.deleteView | 删除视图（:5339） |
| database.deleteMaterializedView | 删除物化视图（:5340） |
| database.deleteIndex | 删除索引（:5341） |
| database.deleteTable | 删除表（:5342） |
| database.addNewRow | 添加新行（:5343） |
| database.editRow | 编辑行（:5344） |
| database.deleteRow | 删除行（:5345） |
| database.queryRowsOnThisPage | 当前页查询结果行（:5346） |
| database.queryRowsLabel | 查询结果行（:5347） |
| menu.databases | 数据库（:7809） |
| keybindings.commands.database.connect.title | 显示数据库（:8388） |
| commandPalette.actions.database-connect.label | 数据库：显示数据库（:7974） |
| commandPalette.categories.Database | 数据库（:7946） |
| panes.databaseViewer | {type} 查看器（:4682） |
| panes.missingDatabaseConnection | 缺少数据库连接（:4698） |
| extensions.databases | 数据库（:7581） |

**未走 i18n 的数据库可见文案**：`SELECT * FROM table_name`（编辑器 placeholder，`components/query-bar.tsx:114,152`）；`#`（表头全选列，`data-grid.tsx:438`）；`FK`（外键标记，`data-grid.tsx:470`）与 `FK: {table}.{col}`（title/tooltip，`data-grid.tsx:468`、`cell-renderer.tsx:89`）；`NULL`（`cell-renderer.tsx:35`）；`{n} columns`（`schema-view.tsx:63`）；`Raw: {value}`（`cell-renderer.tsx:75`）；Mongo 输入 placeholder `Filter JSON, e.g. {"name": "John"}` / `Sort JSON, e.g. {"createdAt": -1}`（`mongodb-viewer.tsx:126,134`）；`v{version}`（`info-view.tsx:47`）；`{tables}t {indexes}i`（`table-toolbar.tsx:110`）；`/ {totalPages}`（`pagination.tsx:101`）；数据库错误规范化文案（全英文，`lib/database-errors.ts:20-41`）。

### 5.2 AI 助手（`ai.*` / `aiShortcut.*` / `aiHistory.*`）

| key | 中文 |
| --- | --- |
| ai.suggestions | 建议（:4807） |
| ai.agentInstalled | {name} 已安装（:4808） |
| ai.agentInstallFailed | 安装 {name} 失败（:4809） |
| ai.unknownError | 未知错误（:4810） |
| ai.planSummary | 计划（{count} 个{label}）（:4811） |
| ai.step / ai.steps | 步骤 / 步骤（:4812-4813） |
| ai.executePlan | 执行计划（:4814） |
| aiShortcut.planImplementation | 规划实现方案（:4815） |
| aiShortcut.planImplementationContent | 查看相关代码，并为此任务提出聚焦的实现计划：（:4816） |
| aiShortcut.findFixBug | 查找并修复 Bug（:4817） |
| aiShortcut.findFixBugContent | 调查此 Bug，找出根因，实施修复并验证：（:4818） |
| aiShortcut.writeTests | 为变更编写测试（:4819） |
| aiShortcut.writeTestsContent | 查看相关行为，并为此变更添加聚焦测试：（:4820） |
| aiShortcut.reviewChanges | 审查当前更改（:4821） |
| aiShortcut.reviewChangesContent | 审查当前工作区更改，查找 Bug、回归和缺失测试：（:4822） |
| aiHistory.searchPlaceholder | 搜索 Agent 历史...（:5380） |
| aiHistory.empty | 暂无 Agent 历史（:5381） |
| aiHistory.noMatches | 没有匹配“{query}”的会话（:5382） |
| aiHistory.archived | 已归档（:5383） |
| aiHistory.restoreSession / restoreSessionNamed | 恢复会话 / 恢复 {title}（:5384-5385） |
| aiHistory.deleteSession / deleteSessionNamed | 删除会话 / 删除 {title}（:5386-5387） |
| ai.newSession | 新会话（:5388） |
| ai.renameSession | 重命名 {title}（:5389） |
| ai.clickToRenameSession | 点击重命名会话（:5390） |
| ai.searchMessages | 搜索消息（:5391） |
| ai.agentHistory | Agent 历史（:5392） |
| ai.agents | Agents（:5393） |
| ai.toggleAgentHistory | 切换 Agent 历史（:5394） |
| ai.newAgent | 新建 Agent（:5395） |
| ai.previousMatch / previousSearchMatch | 上一个匹配项 / 上一个搜索匹配项（:5396-5397） |
| ai.nextMatch / nextSearchMatch | 下一个匹配项 / 下一个搜索匹配项（:5398-5399） |
| ai.closeSearch / closeMessageSearch | 关闭搜索 / 关闭消息搜索（:5400-5401） |
| ai.executePlanStepPrompt | 执行计划的第 {number} 步：{title}\n\n{description}（:5402） |
| ai.editPrompt | 编辑提示词（:5403） |
| ai.cancel | 取消（:5404） |
| ai.send | 发送（:5405） |
| ai.copyPrompt | 复制提示词（:5406） |
| ai.thinking | 正在思考...（:5407） |
| ai.generatedContentNumber | AI 生成内容 {number}（:5408） |
| ai.generatedImageNumber | 生成的图片 {number}（:5409） |
| ai.openResource | 打开 {name}（:5410） |
| ai.copyResponse | 复制回复（:5411） |
| ai.unpinSession / pinSession | 取消置顶会话 / 置顶会话（:5412-5413） |
| ai.archiveSession | 归档会话（:5414） |
| ai.agent | Agent（:5415） |
| ai.agentDefault | Agent 默认值（:5416） |
| ai.model | 模型（:5417） |
| ai.project | 项目（:5418） |
| ai.branch | 分支（:5419） |
| ai.enterApiKey | 输入 API key...（:5420） |
| ai.pleaseEnterApiKey | 请输入 API key。（:5421） |
| ai.invalidApiKey | API key 无效。（:5422） |
| ai.failedValidateApiKey | 验证 API key 失败。（:5423） |
| ai.failedRemoveApiKey | 移除 API key 失败。（:5424） |
| ai.searchApiKeyProviders | 搜索 API key 提供商...（:5425） |
| ai.noProvidersFound | 未找到提供商（:5426） |
| ai.apiKeySaved | API key 已保存（:5427） |
| ai.apiKeyRequired | 需要 API key（:5428） |
| ai.apiKeySavedWithPeriod | API key 已保存。（:5429） |
| ai.openDashboard | 打开控制台（:5430） |
| ai.remove | 移除（:5431） |
| ai.validating | 正在验证（:5432） |
| ai.saveKey | 保存 key（:5433） |
| ai.selectProvider | 选择提供商（:5434） |
| ai.copyCode | 复制代码（:5435） |
| ai.applyCodeToCurrentBuffer | 将此代码应用到当前缓冲区（:5436） |
| ai.apply | 应用（:5437） |
| ai.error | 错误（:5438） |
| ai.agentSessionRestarted | Agent 会话已重启（:5439） |
| ai.couldNotRestartAgentSession | 无法重启 Agent 会话（:5440） |
| ai.agentSetup | Agent 设置（:5441） |
| ai.couldNotOpenAgentTerminal | 无法打开 Agent 终端（:5442） |
| ai.hideDetails / ai.details | 隐藏详情 / 详情（:5443-5444） |
| ai.restarting | 正在重启...（:5445） |
| ai.restartAgentSession | 重启 Agent 会话（:5446） |
| ai.opening | 正在打开...（:5447） |
| ai.openAgentTerminal | 打开 Agent 终端（:5448） |
| ai.finishAgentSetupThenRestart | 完成 Agent 设置后重启会话。（:5449） |
| ai.completeLoginThenRestart | 在 Agent CLI 中完成登录后重启会话。（:5450） |
| ai.addContext | 添加上下文（:5451） |
| ai.terminal | 终端（:5452） |
| ai.databaseContext | {type} 数据库（:5453） |
| ai.pullRequestNumber | 拉取请求 #{number}（:5454） |
| ai.actionRunNumber | Actions 运行 #{number}（:5455） |
| ai.searchContext | 搜索上下文...（:5456） |
| ai.noMatchingContextFound | 未找到匹配的上下文（:5457） |
| ai.openTabs | 打开的标签页（:5458） |
| ai.added | 已添加（:5459） |
| ai.notSynced / syncing / syncPaused / synced | 未同步 / 正在同步 / 同步已暂停 / 已同步（:5460-5463） |
| ai.searchAvailableSkills | 搜索可用技能...（:5464） |
| ai.searchSkills | 搜索技能...（:5465） |
| ai.new | 新建（:5466） |
| ai.mySkills | 我的技能（:5467） |
| ai.browse | 浏览（:5468） |
| ai.loadingAvailableSkills | 正在加载可用技能...（:5469） |
| ai.noPublishedSkillsYet | 暂无已发布技能（:5470） |
| ai.publishedSkillsWillAppear | Lithe 技能注册表可用后，已发布技能会显示在这里。（:5471） |
| ai.noAvailableSkillsMatch | 没有可用技能匹配“{query}”（:5472） |
| ai.add | 添加（:5473） |
| ai.noSkillsYet | 暂无技能（:5474） |
| ai.noSkillsMatch | 没有技能匹配“{query}”（:5475） |
| ai.marketplace | 市场（:5476） |
| ai.localOverride | 本地覆盖（:5477） |
| ai.editSkill / ai.editNamedSkill | 编辑技能 / 编辑 {title}（:5478-5479） |
| ai.deleteSkill / ai.deleteNamedSkill | 删除技能 / 删除 {title}（:5480-5481） |
| ai.newSkill | 新建技能（:5482） |
| ai.marketplaceSkill | 市场技能（:5483） |
| ai.marketplaceSkillWithLocalOverride | 带本地覆盖的市场技能（:5484） |
| ai.title | 标题（:5485） |
| ai.skillTitlePlaceholder | 代码审查清单（:5486） |
| ai.markdown | Markdown（:5487） |
| ai.skillContentPlaceholder | 编写此技能的指令或可复用上下文...（:5488） |
| ai.save | 保存（:5489） |
| ai.skills | 技能（:5490） |
| ai.open / recent / files | 打开 / 最近 / 文件（:5491-5493） |
| ai.searchFiles | 搜索文件...（:5494） |
| ai.fileList | 文件列表（:5495） |
| ai.openLower | 打开（:5496） |
| ai.installingAgent | 正在安装 {name}（:5497） |
| ai.install | 安装（:5498） |
| ai.mode | 模式（:5499） |
| ai.provider | 提供商（:5500） |
| ai.configureCustomModel | 配置自定义模型...（:5501） |
| ai.apiKeys | API Keys（:5502） |
| ai.aiPreferences | AI 偏好设置（:5503） |
| ai.settings | 设置（:5504） |
| ai.ask / ai.plan | 提问 / 计划（:5505-5506） |
| ai.microphoneAccessFailed | 麦克风访问失败。请检查系统设置 -> 隐私与安全 -> 麦克风。（:5511） |
| ai.voiceNotSupportedWebview | 此 webview 不支持语音输入。（:5513） |
| ai.voiceStoppedUnexpectedly | 语音输入意外停止。（:5514） |
| ai.voiceCouldNotStart | 无法启动语音输入。（:5515） |
| ai.noOllamaModelsDetected | 未检测到模型。请在 Ollama 中安装模型。（:5516） |
| ai.noModelsFound | 未找到模型。（:5517） |
| ai.failedFetchModels | 获取模型失败（:5518） |
| ai.loadingModels | 正在加载模型...（:5519） |
| ai.selectModel | 选择模型（:5520） |
| ai.openDiff / openFile / openTerminal | 打开 diff / 打开文件 / 打开终端（:5521-5523） |
| ai.toolCall | 工具调用（:5524） |
| ai.toolCalls | {count} 个工具调用（:5525） |
| ai.toolStatusFailed / Pending / Running / Completed | 失败 / 等待中 / 运行中 / 已完成（:5526-5529） |
| ai.toolChangedFile / toolChangedFiles | 已更改 {file} / 已更改 {count} 个文件（:5530-5531） |
| ai.toolTerminalOutput / toolTerminals | 终端输出 / {count} 个终端（:5532-5533） |
| ai.toolDetailWithSummary | {status} - {summary}（:5534） |
| ai.allow / always / deny / never | 允许 / 始终 / 拒绝 / 永不（:5535-5538） |
| ai.allowOnce / alwaysAllowRequestType / denyOnce / alwaysDenyRequestType | 允许一次 / 始终允许此类请求 / 拒绝一次 / 始终拒绝此类请求（:5539-5542） |
| ai.permission | 权限（:5543） |
| ai.messageInput | 消息输入（:5544） |
| ai.showSlashCommands | 显示斜杠命令（:5545） |
| ai.stopVoiceInput / startVoiceInput | 停止语音输入 / 开始语音输入（:5548-5549） |
| ai.stopGeneration | 停止生成（:5550） |
| ai.addToQueue | 加入队列（:5551） |
| ai.sendMessage | 发送消息（:5552） |
| ai.selectedContext | 已选上下文（:5553） |
| ai.removeContextItemHint | {name}。按 Delete 可从上下文中移除。（:5554） |
| ai.selectAiModel | 选择 AI 模型（:5555） |
| ai.useCustomValue | 使用 {value}（:5556） |
| ai.typeModelName | 输入模型名称并按 Enter（:5557） |
| ai.selectAiProvider | 选择 AI 提供商（:5558） |
| ai.slashCommandSuggestions | 斜杠命令建议（:5559） |
| ai.noMatchingSlashCommands | 没有匹配的斜杠命令（:5560） |
| ai.noSlashCommandsAvailable | 暂无可用斜杠命令（:5561） |
| layout.resizeAiChat | 调整 AI 聊天大小（:7886） |
| commandPalette.actions.toggle-ai-chat-view.enableLabel / disableLabel | 视图：显示 AI 聊天 / 视图：隐藏 AI 聊天（:8107-8108） |
| commandPalette.actions.toggle-ai-chat-feature.enableLabel / disableLabel | 功能：启用 AI 聊天 / 功能：禁用 AI 聊天（:8092-8093） |

**未走 i18n 的 AI 可见文案（硬编码英文，file:line）**：
- 输入框 placeholder：`"What do you want to create?"` / `"Ask anything... (@ files, / commands)"` / `"Ask anything... (@ to mention files)"` / `"Configure API key to enable Agent..."`（`features/ai/components/input/chat-input-bar.tsx:960,962-964`）
- `Remove ${image.name}`（`:1002`）、`Remove ${item.name} from context`（`:1258`）、文件回退名 `"Unknown"`（`:440`）
- 提及面板 `ariaLabel="File suggestions"`（`components/mentions/file-mention-dropdown.tsx:96`）、`emptyLabel = "No matching files found"`（`components/mentions/ai-file-selector.tsx:73`）
- 权限回退选项名 `"Deny"` / `"Allow"`（`components/chat/acp-permission-prompt.tsx:16-17`）
- 斜杠项快捷提示 `"Enter"`（`components/mentions/slash-command-dropdown.tsx:119`）
- 活动行标签：`"Permission requested"` / `"Permission response"` / `"Session title updated"` / `"Agent error"` / `Plan updated (${n} steps)` / `"No plan steps"`（`components/chat/ai-chat.tsx:778,997,836,872,857,854`）
- 直连意图与兜底：`"Web Viewer is disabled. Enable it in Settings > Features to open URLs."`（`:485`）、`Opened ${url} in Lithe web viewer.`（`:495`）、`` Opened terminal and ran `${cmd}`. ``（`:504`）、`` Applied `/${cmd}`. ``（`:552`）、`"Session updated."`（`:552`）、`"The selected agent did not return a visible response..."`（`:567`）、`"The selected provider did not return a visible response..."`（`:568`）、`No Response` / `EMPTY_RESPONSE`（`:572-573`）
- 错误标题：`API Error` / `Rate Limit Exceeded` / `Authentication Error` / `Access Denied` / `Server Error` / `Bad Request` / `Agent Configuration Required` / `Authentication Required` / `Connection Lost`（`:597-671`）、`"Error: Failed to connect to Agent service. Please check your API key and try again."`（`:907`）
- 默认会话名 `"New Session"`（`stores/ai-chat/chat-actions.ts:57`）、Agent buffer 名 `Agent ${n}`（`features/editor/stores/buffer.store.ts:744`）
- `LITHE_AGENT_OPTION`：`"Lithe Agent"` / `"Use Lithe Agent settings and provider configuration"`（`hooks/use-agent-options.ts:14-18`）、回退描述 `"ACP-compatible coding agent"`（`:77`）
- 终端 agents：`"Claude Code"` / `"Open Claude Code in an Lithe terminal"`（`lib/claude-code.ts:6-7`）；`"Antigravity CLI"` / `"Open Antigravity CLI in an Lithe terminal"`（`lib/terminal-agents.ts:12-15`）
- Codex：`"Codex tool"`（`codex-integration-service.ts:50`）、`"Workspace"` / `"Codex needs approval to continue"` / `Allow` / `Deny`（`:170-174`）、`Unknown Lithe tool: ${toolName}`（`:122`）、`"Codex app-server error"`（`:190`）、`"Codex turn failed"`（`:184`）
- 技能市场作者前缀 `by {author}`（`components/skills/skills-command.tsx:383`）
- 滚动按钮 sr-only `"Scroll to end"` / `"Scroll to start"`（`ui/message-scroller.tsx:109`）

### 5.3 全局搜索 / 快速打开

| key | 中文 |
| --- | --- |
| search.emptyTitle | 在项目中搜索（:5879） |
| search.emptyDescription | 输入关键词，即可在整个项目中查找匹配的文件和代码行。（:5880） |
| search.openProjectTitle | 打开项目后再搜索（:5881） |
| search.openProjectDescription | 全局搜索需要先打开一个项目文件夹。（:5882） |
| search.unsupported | 当前工作区类型不支持全局搜索。（:5883） |
| search.failed | 搜索失败（:5884） |
| search.retry | 重试（:5885） |
| search.noResults | 未找到结果（:5886） |
| search.noResultsFor | 未找到 “{query}” 的结果（:5887） |
| search.noResultsForWithFilters | 未找到 “{query}” 的结果（已应用当前文件筛选）（:5888） |
| search.matchCase | 区分大小写（:5889） |
| search.matchWholeWord | 全字匹配（:5890） |
| search.useRegex | 使用正则表达式（:5891） |
| search.search | 搜索（:5892） |
| search.hideDetails | 隐藏详细信息（:5893） |
| search.showDetails | 显示详细信息（:5894） |
| search.clear | 清除搜索（:5895） |
| search.close | 关闭搜索（:5896） |
| search.previousMatch | 上一个匹配项（:5897） |
| search.nextMatch | 下一个匹配项（:5898） |
| search.hideReplace | 隐藏替换（:5899） |
| search.showReplace | 显示替换（:5900） |
| search.replaceWith | 替换为...（:5901） |
| search.replace | 替换（:5902） |
| search.replaceAllShort | 全部（:5903） |
| search.replacedMatches | 已替换 {count} 处匹配（:5904） |
| search.replaceMatchesFailed | 替换搜索匹配失败（:5905） |
| search.options | 搜索选项（:5906） |
| search.filesToInclude | 要包含的文件（:5907） |
| search.filesToExclude | 要排除的文件（:5908） |
| workbench.search | 搜索（:4607） |
| workbench.searchInFiles | 在文件中搜索...（:4608） |
| panes.searchResults | 搜索结果（:4684） |
| fileNavigator.searchResultFiles | 搜索结果文件（:7492） |
| fileNavigator.files | 文件（:7485） |
| fileNavigator.viewAria | 文件导航器视图（:7486） |
| fileNavigator.flatList | 平铺列表（:7487） |
| fileNavigator.fileTree | 文件树（:7488） |
| fileNavigator.showingCount | 显示 {visible} / {total}（:7489） |
| fileNavigator.noFilesMatch | 没有匹配的文件（:7490） |
| fileNavigator.resizeAria | 调整文件导航器大小（:7491） |
| quickOpen.searchFiles | 输入以搜索文件...（:7823） |
| quickOpen.searchSymbols | 输入以筛选符号...（:7824） |
| quickOpen.searchWorkspaceSymbols | 在项目中搜索符号...（:7825） |
| quickOpen.filesCount | {count} 个文件（:7826） |
| quickOpen.fileCountOne | {count} 个文件（:7827，与复数键同值） |
| quickOpen.symbolsCount | {count} 个符号（:7828） |
| quickOpen.openFolder | 打开文件夹后再搜索文件（:7829） |
| quickOpen.noMatch | 未找到匹配的文件（:7830） |
| quickOpen.searching | 正在搜索...（:7831） |
| quickOpen.noFilesInProject | 项目中未找到文件（:7832） |
| quickOpen.noFiles | 没有可用文件（:7833） |
| quickOpen.indexing | 正在索引项目文件（:7834） |
| quickOpen.loadingFiles | 正在加载文件（:7835） |
| quickOpen.loadingSymbols | 正在加载符号...（:7836） |
| quickOpen.noSymbols | 未找到符号（:7837） |
| menu.quickOpen | 快速打开（:7791） |
| keybindings.commands.file.quickOpen.title | 快速打开（:8285） |
| commandPalette.actions.quick-open.label | 转到：快速打开（:8026） |
| commandPalette.actions.search-global.label | 搜索：全局搜索（:8024） |
| commandPalette.title | 命令面板（:7904） |
| commandPalette.placeholder | 输入命令...（:7905） |
| commandPalette.noCommands | 未找到命令（:7906） |
| commandPalette.close | 关闭命令面板（:7938） |
| commandPalette.clearPersistedActions | 清除持久命令（:7939） |

**未走 i18n 的搜索可见文案**：`Indexing N files` / `Indexing files` / `Preparing search` / `Searching a/b files` / `Searching N files` / `Searching files` / `Loading more results`（`components/global-search-buffer.tsx:453-474`）；`N results (M total)`（`:490`）；`Invalid regular expression; showing literal matches`（`stores/global-search.store.ts:58`）；`Load all search results before replacing all`（`global-search-buffer.tsx:540`）；`Showing N of M results`（`components/global-search-results.tsx:102`）；`match`/`matches`（`components/search-excerpt-results.tsx:120`）；`Collapse context`/`Expand context`（`:130-131`）；`Open line N`（`search-excerpt-code.tsx:119`）；`Failed to expand search context`（`global-search-buffer.tsx:306`）；`Search failed: ...`（`hooks/use-content-search.ts:303,351`）；`Search indexing failed: ...`（`:401`）。

---

## 6. gpui-kit 0.6.6 对应建议

判定口径：**可一比一** = gpui-kit 0.6.6 有直接对应类型且语义足够；**需自行组合** = 有积木但要自己拼/自己写渲染逻辑；**gpui-kit 没有** = 必须落到 GPUI 本体或自研。

### 6.0 三个数据密集区的硬约束（**先记住**）

| 约束 | gpui-kit 事实 | 证据 |
| --- | --- | --- |
| `DataTable` **行高是表级值**，不能逐行设 | `TableState` 统一读 `self.options.size.table_row_height()`；`options.size` 由 `DataTable` 的 `Sizable`（`.small()/.x_small()/.with_size(Size::Size(px))`）写入。默认 Medium **32px** | `<REG>\gpui-component-0.6.6\src\table\state.rs:713,1814,1962,2376`；`table\data_table.rs:131-139`；`sizing.rs:57-65` |
| `TableState` / `ListState` **只有单选** | `selected_row()/set_selected_row(usize)`、`selected_cell()/set_selected_cell((usize,usize))`；字段是 `Option<usize>` / `Option<(usize,usize)>`；`enum SelectionMode` 是**私有**；没有公开 `Selection` 类型 | `<REG>\gpui-component-0.6.6\src\table\state.rs:240-245,458,463,529,548,36-40` |
| `gpui_component::list::ListState` 同样只有单选 | `set_selected_index(Option<IndexPath>)` / `selected_index()`；`ListDelegate` 要求 `set_selected_index` | `<REG>\gpui-component-0.6.6\src\list\list.rs:184,194`；`list\delegate.rs:119` |
| `gpui::uniform_list` **行高取第 0 行测量高度** | 文件头注释：「simply measures the first element and then lays out all remaining elements in a line based on that measurement」；`item_to_measure_index: 0` | `<REG>\gpui-pre-0.3.6\src\elements\uniform_list.rs:2-5,43`；`measure_item()` `:658-680` |
| `uniform_list` **没有任何内建选择状态** | 全文件无 selection/selected API | `<REG>\gpui-pre-0.3.6\src\elements\uniform_list.rs`（全树 grep 无命中） |
| gpui 本体的 `list` / `ListState`（变高）**也没有选择支持** | `elements\list.rs` 无 selection API；gpui-kit 自己的 `Sidebar` 直接 import gpui 的 `ListState/list` | `<REG>\gpui-pre-0.3.6\src\elements\list.rs:24,37,54`；`<REG>\gpui-component-0.6.6\src\sidebar\mod.rs:10-11` |
| `gpui::ListDelegate` **不存在** | grep `trait ListDelegate\|struct ListDelegate` 扫 `<REG>\gpui-pre-0.3.6\src` → 0 命中 | — |
| `TreeDelegate` **不存在** | 树的渲染回调直接传给 `Tree::new(&Entity<TreeState>, render_item)` | `<REG>\gpui-component-0.6.6\src\tree.rs:41-52` |
| `TabPanel` **不存在**（0.6.6 改名 `TabGroup`） | 全 crate grep 仅 2 处注释 | `<REG>\gpui-component-0.6.6\src\dock\mod.rs:200-202` |
| `Splitter` / `PanelGroup` / 布局 `Stack` **不存在** | 分栏唯一名字是 `ResizablePanelGroup` + `h_resizable`/`v_resizable` | `<REG>\gpui-base-0.6.6\src\resizable\mod.rs:17,22`；`resizable\panel.rs:31` |
| `Modal` / `Window::open_modal` **不存在** | 模态 = `Root` + `window.open_dialog(...)` / `open_sheet(...)` | `<REG>\gpui-component-0.6.6\src\root.rs:37,297`；`window_ext.rs:109` |
| 代码高亮编辑器**有**，但需 feature `tree-sitter` | `gpui_component::input::{Editor, EditorState}`；`EditorState::new` 即 code editor，`.language("sql")` 切语言；无 `tree-sitter` 时 `input_highlighter_factory()` 返回 `None` | `<REG>\gpui-base-0.6.6\src\input\base\state.rs:9230-9240`；`<REG>\gpui-component-0.6.6\src\highlighter\mod.rs:11-40` |
| i18n **只有集成，无文案体系** | `gpui_component::locale()` / `set_locale(&str)` + `rust_i18n::i18n!`（`locales\ui.yml`）；无 `Locale`/`Translations` 类型 | `<REG>\gpui-component-0.6.6\src\lib.rs:126,153,158` |

### 6.1 数据库逐元素建议

| 前端元素 | gpui-kit 0.6.6 类型（`模块路径::类型名`） | 判定 |
| --- | --- | --- |
| tab bar / 数据库 tab | `gpui_component::tab::{Tab, TabBar}`；pane 级布局用 `gpui_component::dock::*`（`DockArea` + `TabGroup`） | 可一比一 |
| 左面板 / 右面板分栏（`flex gap-2 p-2`） | `gpui_base::resizable::{h_resizable, resizable_panel, ResizablePanelGroup, ResizableState}`（经 `gpui_component::{h_resizable, v_resizable, resizable_panel}`） | 可一比一 |
| `SidebarPanel` + `SidebarTitleBar` | `gpui_component::sidebar::{Sidebar, SidebarHeader, SidebarFooter, SidebarGroup, SidebarItem}` | 可一比一（默认宽 255px / 折叠 48px，需 `.side()` 与宽度覆盖） |
| 对象树分组（表/视图/物化视图/订阅/索引） | 分组标题用 `gpui_component::label::Label` 或 `group_box::GroupBox`；树本身 `gpui_base::tree::{Tree, TreeState, TreeItem}`（经 `gpui_component::tree::{Tree, TreeItem, TreeState}`） | 需自行组合（树无内建缩进与图标，必须自己在 `render_item` 里按 `TreeEntry::depth` 画） |
| 对象行选中态（`bg-accent/80`） | `TreeState::{selected_index, set_selected_index}` | 可一比一（单选足够，对象树就是单选） |
| 对象行右键菜单 | `gpui_component::tree::Tree::context_menu(Fn(usize, &TreeEntry, PopupMenu, ...) -> PopupMenu)`；菜单本体 `gpui_component::menu::{PopupMenu, PopupMenuItem}` | 可一比一 |
| 结果网格（虚拟化 + 列宽拖拽 + 排序 + 行右键 + 单元格编辑） | `gpui_component::table::{DataTable, TableState, TableDelegate, Column, TableEvent, ColumnFixed, ColumnSort}` | **需自行组合**：表格外壳可一比一（`Column::width/min_width/max_width/resizable/fixed_left/sortable`、`TableDelegate::render_td`、`TableDelegate::context_menu`、`TableEvent::ColumnWidthsChanged`、`perform_sort`），但**多格区间选择 + 键盘网格导航 + 表头全选 + 行号列点击选整行**全部要自研（kit 只有单选） |
| 网格行高 34px（前端 `ESTIMATED_ROW_HEIGHT`） | `DataTable::with_size(Size::Size(px(34.)))` 或 `.small()`(30) —— 只能整表设 | **需自行组合**（无法逐行变高）；若必须支持展开行，需放弃 `DataTable`，改成 `gpui::uniform_list`（行高取第 0 行）或 `gpui_base::VirtualList`（变高，`.with_item_to_measure_index`） |
| 单元格内容渲染（NULL Badge / JSON 展开 / 日期格式化 / 外键链接 / 长文本截断） | `TableDelegate::render_td` 内自绘；可用 `gpui_component::badge::Badge`（NULL）、`gpui_component::button::Button`（可展开/可点）、`gpui_component::text::TextView` 或 `gpui_component::highlighter::*`（JSON） | 需自行组合 |
| 单元格右键复制菜单 | `TableDelegate::context_menu` + `PopupMenu::{menu, menu_with_icon, separator}` | 可一比一（但「复制所选」的多选语义要自己维护） |
| 列宽拖拽手柄 | `Column::resizable(true)`（默认 true）+ `TableState::col_resizable(true)` + `TableEvent::ColumnWidthsChanged(Vec<Pixels>)`；手柄尺寸 `HANDLE_SIZE 2px` / `HANDLE_PADDING 4px` | 可一比一（注意前端是 **4px** 手柄 + 最小 60px，kit 的 `Column::min_width` 需显式设 60） |
| 列宽持久化（前端仅在 store 内存态） | gpui-kit 无持久化能力；`TableEvent::ColumnWidthsChanged` 里自己落盘 | gpui-kit 没有 |
| SQL 编辑器（透明 textarea 叠高亮 pre + 补全 chips + Tab 接受） | `gpui_component::input::{Editor, EditorState}`（`.language("sql")`、`.line_number(true)`）；补全列表可用 `gpui_component::popover::Popover` + `gpui_component::list::List` | **需自行组合**：前端是「textarea + 独立 pre 高亮层」的廉价方案，gpui-kit 的 `Editor` 是真编辑器（需 `tree-sitter` feature 才有高亮）。若不做 LSP，用 `gpui_component::input::{Textarea, TextareaState}`（`.rows(3)`、`.auto_grow(min,max)`）就够，高亮另想办法 |
| 视图切换段（Data/Schema/Info） | `gpui_component::tab::{TabVariant::Segmented, TabBar::segmented()}` 或 `gpui_component::button::Button` + `ButtonVariants` | 可一比一（kit 无独立 `Segmented` 类型） |
| 工具条图标按钮（列类型开关 / SQL 编辑器 / 导出 / 复制 / 订阅动作） | `gpui_component::button::Button::new(id).icon(IconName::...)`；`IconButton` 类型**不存在**（`ButtonIcon` 仅 `pub(crate)`） | 可一比一 |
| 工具条 tooltip + 快捷键提示 | `Tooltip::new(text)` / `Tooltip::action(&dyn Action, Option<&str>)` + `ManagedTooltipExt`（`.tooltip(...)`） | 可一比一 |
| 结果摘要 chip / 计数 Badge | `gpui_component::badge::Badge` 或 `gpui_component::tag::Tag` | 可一比一 |
| 列筛选器行（Select + Select + Input + X，可加多行） | `gpui_component::select::{Select, SelectState}`（**必须自带 `SearchableListDelegate`，`SearchableVec::new(items)` 最省事**）+ `gpui_component::input::{Input, InputState}` + `Button` | 可一比一 |
| 分页（PageSize Select + 页码 Input + 上/下页 + `/ N`） | `gpui_component::select::SelectState` + `gpui_component::input::{Input, InputState}` + `Button` + `Label`；kit 自带 `gpui_component::pagination::Pagination` 但样式语义是「页码组」，与前端形态不同 | 需自行组合 |
| 空态 / 加载 / 错误 | `gpui_component::empty::Empty`（存在，需核对其组合 API）、`gpui_component::spinner::Spinner`、`gpui_component::alert::{Alert, AlertVariant}`、`gpui_component::skeleton::Skeleton` | 可一比一 |
| 单元格 / 行 / 表右键菜单浮层 | `gpui_component::menu::{ContextMenuExt, PopupMenu}`（`div().context_menu(|menu, window, cx| ...)`） | 可一比一 |
| 连接对话框（provider Select + 表单/字符串 Tabs + 字段 + 测试 + 连接） | `WindowExt::open_dialog(cx, build)` + `gpui_component::dialog::{Dialog, DialogButtonProps, DialogHeader, DialogTitle}` + `gpui_component::form::{Form, Field}` + `gpui_component::tab::{TabBar, Tab}` + `Select` + `Input` | 可一比一 |
| 回车键提交 / 拖放数据库文件 | `.on_action` / `KeyBinding`（`gpui::KeyBinding::new`）；拖放走 GPUI 本体（`on_drop`），gpui-kit 无封装 | 需自行组合 |
| CRUD 模态（新增行 / 编辑行 / 建表 / 建订阅） | `WindowExt::open_dialog` + `Dialog` + `Form`/`Field` + `Input`/`Select`/`Checkbox`（`gpui_component::checkbox::Checkbox`） | 可一比一 |
| MongoDB 文档卡 + `<pre>` JSON | `gpui_component::group_box::GroupBox` 或自绘 div（`.rounded(...).border_1().bg(...)`）+ `gpui_component::text::TextView`（`text::markdown`/`html` 子模块） | 需自行组合 |
| Redis key 列表（type Badge + key + TTL）：无限滚动 | `gpui_base::VirtualList` + `v_virtual_list`；或 `gpui_component::list::{List, ListState}` + `ListDelegate::{has_more, load_more_threshold, load_more}` | 可一比一（单选足够） |
| SQL 历史列表（最多 10 条，hover 显示 3 个动作） | `gpui_component::list` 或直接 `div` + `Button`（`.on_hover` 控 opacity） | 需自行组合 |
| Schema 视图（列清单 + 类型 + 约束标签 + FK 标签） | `gpui_component::description_list::DescriptionList` 或 `gpui_component::table::Table`（无状态简单表） | 可一比一 |
| Info 视图（统计 + 对象分组 + 最近查询） | `gpui_component::description_list::DescriptionList` + `group_box::GroupBox` | 需自行组合 |
| 面板圆角/半透明背景等视觉常量 | 主题令牌 `gpui_component::theme::{Theme, ActiveTheme}`，`cx.theme().radius` / `.radius_lg` / `.background` / `.border` / `.primary` / `.table` / `.table_head` / `.table_row_border` / `.table_active`；应用自有组件建议用 `gpui_base::SemanticThemeTokens`/`ColorTokens`/`RadiusTokens`/`SpacingTokens` | 可一比一 |

### 6.2 AI 助手逐元素建议

| 前端元素 | gpui-kit 0.6.6 类型 | 判定 |
| --- | --- | --- |
| 面板容器（`max-w-4xl` 居中） | 自绘 `div().max_w(px(896.)).mx_auto()` | 可一比一 |
| header（标题可重命名 + 3 个动作 + 可展开搜索行） | `gpui_component::tab::Tab` 之外的 `gpui_component` 无 pane-header 专用类型；用 `gpui_component::sidebar::SidebarHeader` 结构或自绘 | 需自行组合 |
| 会话标题就地重命名 | `gpui_component::input::{Input, InputState}` + `InputEvent::PressEnter`（`gpui_base::input::InputEvent::{Change, PressEnter{secondary,shift}, Focus, Blur}`） | 可一比一 |
| 消息滚动 + 粘底自动跟随 | `gpui_base::VirtualList`（变高，`.track_scroll(&handle)`、`.with_sizing_behavior`）+ `ScrollStrategy::{Top,Center,Bottom,Nearest}` 或 `UniformListScrollHandle::{scroll_to_bottom, is_scrolled_to_end}` | **需自行组合**：前端用的 `autoScroll` / `defaultScrollPosition="last-anchor"` / `useMessageScroller` 来自外部包 `@shadcn/react/message-scroller`（`ui/message-scroller.tsx:1-6`），**该算法本仓库没有**，必须自研（判定阈值、粘底恢复、`scrollAnchor`） |
| 消息气泡（用户右对齐 secondary / 助手 ghost 无背景） | `gpui_component::bubble::{Bubble, ...}`（模块存在）+ `gpui_component::message::{Message, ...}`；对齐/背景需自调 | 可一比一 |
| 消息 hover 才显形的 copy/edit 动作 | `gpui_component::button::Button` + `.on_hover` / `Stateful` 自己控 opacity；或 `gpui_component::menu::DropdownMenu` | 需自行组合 |
| Markdown 渲染（标题/表格/列表/引用/代码块/错误块） | `gpui_component::text::{TextView, markdown, html}` | 可一比一（**错误块 `[ERROR_BLOCK]` 是 Lithe 私有协议，需自己在 markdown 前后处理**） |
| 代码块 + 复制按钮 | `gpui_component::text::markdown` 的代码渲染 + `gpui_component::button` + `gpui_component::clipboard::Clipboard` | 可一比一 |
| 工具调用折叠行（activity line） | `gpui_component::collapsible::Collapsible` 或 `gpui_component::accordion::Accordion`；图标旋转自绘 | 需自行组合 |
| 计划卡 + 步骤状态 | `gpui_component::group_box::GroupBox` + `gpui_component::stepper::Stepper`（状态语义不完全一致：前端只有 pending/current/completed）或自绘 | 需自行组合 |
| 权限确认条（FIFO 队列 + 允许/始终/拒绝/永不） | `gpui_component::alert::{Alert, AlertVariant}` + `DialogButtonProps`（`ok_text/ok_variant/cancel_text/show_cancel/on_ok/on_cancel`）或 `Button` 组 | 可一比一 |
| 回到底部按钮 | `gpui_component::button::Button`（圆角/浮动自绘） | 可一比一 |
| composer（多行可增长输入 + 内联 token） | `gpui_component::input::{Textarea, TextareaState}`（`.rows(n)`、`.auto_grow(min,max)`、`.soft_wrap(bool)`）；**行内 token（@mention / /command 胶囊）kit 不支持** | **需自行组合**：token 胶囊要么在 `Editor`（`gpui_component::input::{Editor, EditorState}`）里做 decorations，要么自绘富文本层 |
| 发送 / 停止按钮 + 队列 Badge | `gpui_component::button::Button`（`.loading(bool)`、`.toggled(bool)`、`ButtonVariant`）+ `gpui_component::badge::Badge` | 可一比一 |
| `Enter` 发送 / `Shift+Enter` 换行 | `gpui::KeyBinding::new` + `gpui_base::input::InputEvent::PressEnter { secondary, shift }` | 可一比一 |
| `@` 提及 / `/` 斜杠下拉（锚定在输入框上方、宽随锚点） | `gpui_component::popover::Popover`（`.anchor`、`.default_open`、`.open(bool)`、`.on_open_change`、`.content(...)`）；列表可用 `gpui_component::list::{List, ListState}` 或 `searchable_list::{SearchableListState, SearchableVec}` | 需自行组合（`w-(--anchor-width)` 这种宽度绑定要靠 `Popover` + 自己量锚点宽） |
| 模型 / 提供商 / 上下文选择 | `gpui_component::menu::{DropdownMenu, PopupMenu}`（前端就是 `DropdownMenuRadioGroup`）或 `gpui_component::select::{Select, SelectState}` | 可一比一 |
| 聊天历史下拉（搜索 + 置顶/归档/删除） | `gpui_component::menu::PopupMenu` + `gpui_component::input::{Input, InputState}`；或 `gpui_component::list` | 需自行组合 |
| 上下文 chips（←→ 移焦点、Del 移除） | 自绘 `div` + `gpui_component::tag::Tag` / `Badge` + `KeyBinding` | 需自行组合 |
| 语音输入 / 附件缩略图 | gpui-kit 无语音与图片缩略图封装 | gpui-kit 没有 |
| 消息搜索（`Cmd+F` + 上/下/关闭） | `Input` + `Button` + `KeyBinding`；命中高亮需自绘 | 需自行组合 |
| 技能市场面板（列表 + 编辑器，宽 430×560） | `gpui_component::list` 或 `searchable_list` + `gpui_component::input::{Editor, EditorState}`（Markdown 编辑）+ `WindowExt::open_dialog` | 需自行组合 |
| 空态「4 个技能快捷键」 | `gpui_component::button::Button`（`.label/.icon`）或 `gpui_component::card::*` | 可一比一 |
| 流式 shader 文字动效（`ui-text-shimmer`） | gpui-kit 有 `gpui_component::shimmer::ShimmerText` | 可一比一 |
| 思维球（`ThinkingOrb`） | gpui-kit 无对应 | gpui-kit 没有（自绘或复用 `spinner::Spinner`） |

### 6.3 全局搜索 / 快速打开逐元素建议

| 前端元素 | gpui-kit 0.6.6 类型 | 判定 |
| --- | --- | --- |
| 搜索面板（侧栏形态） | `gpui_component::sidebar::{Sidebar, SidebarHeader}` + 自绘主体；或整体作为 dock 面板（`DockArea::add_panel` + `Panel` trait） | 可一比一 |
| 搜索输入框（h-7，圆角，内嵌放大镜 + 清除） | `gpui_component::input::{Input, InputState}` + `.prefix(...)` / `.suffix(...)`（`Input::new(&state).prefix(...).suffix(...).cleanable(bool)`） | 可一比一（`SearchInput` 类型**不存在**） |
| 大小写/全字/正则三连开关（segmented iconOnly） | `gpui_component::button::{Toggle, Button}` + `.toggled(bool)`；或 `gpui_component::tab::{TabBar, TabVariant>`（无独立 `Segmented`） | 需自行组合 |
| 结果计数 / 警告 Badge | `gpui_component::badge::Badge`（或 `tag::Tag`） | 可一比一 |
| 替换行（输入框 + 替换 + 全部） | `Input` + `Button` + `Tooltip::new(...)` | 可一比一 |
| 包含/排除文件两列输入 | `Input` × 2 + 自绘 grid | 可一比一 |
| 左侧命中文件导航（224px，可拖 176–420，含树/平铺切换、模糊过滤） | `gpui_base::resizable::{h_resizable, resizable_panel, ResizableState}`（`ResizablePanel::size_range`）+ `gpui_component::tree::{Tree, TreeState}` 或 `list::{List, ListState}` + `input::Input` | 需自行组合（`ResizablePanel::size_range` 可表达 176–420；树/平铺切换要自己换 renderer） |
| 结果摘录列表（虚拟化的多行代码卡片 + 语法高亮 + 命中高亮） | `gpui_base::VirtualList` / `v_virtual_list`（变高）+ `gpui_component::highlighter::*`（语法）+ 自绘 span 背景（`bg-warning/20`） | **需自行组合**：`uniform_list` 行高取第 0 行，摘录卡片高度不一，**必须用 `VirtualList`** 并显式设 `.with_item_to_measure_index` |
| 摘录行号 gutter（右对齐、等宽数字、按最大行号算宽） | 自绘 + `gpui_component::text::TextView`；等宽字体用 `cx.theme().mono_font_family` | 需自行组合 |
| 上下文展开/收起（按文件读全文，2→7 行） | `Button` + 自己异步读文件（`gpui::cx.spawn`） | 需自行组合 |
| 懒加载哨兵（IntersectionObserver 640px） | gpui-kit 无 IntersectionObserver；用 `VirtualList` 的 `visible_rows_changed` 回调或 `ListDelegate::load_more_threshold` 替代 | **需自行组合**（语义等价物存在，机制不同：gpui-kit 是委托回调，不是观察者） |
| 点击摘录跳转到编辑器行列 | `gpui_component::input::Editor` 的 `set_cursor_position` / 或自定义 action + `KeyBinding`；前端做法是 `onOpenLocation({line, column})`（`search-excerpt-code.tsx:105`） | 需自行组合 |
| 快速打开浮层 | `gpui_component::command::{Command, CommandState, CommandGroup, CommandItem}` | **可一比一**（这是 gpui-kit 的一等公民实现：`.item()/.group()/.searchable(bool)/.filterable(bool)/.on_query/.on_select/.on_confirm/.on_cancel/.placeholder/.empty/.max_h/.header/.footer`） |
| 浮层定位与动画（顶部 64px、scale/opacity/blur） | 前端是 `DialogPrimitive.Portal` + `motion`；gpui-kit 用 `WindowExt::open_dialog` 或自建 overlay layer（`Root` 的 dialog layer）；动画令牌 `Theme::motion` | 需自行组合 |
| 列表项（图标 + 标题 + 路径 + 徽标，default 32px / compact 28px） | `gpui_component::command::CommandItem` + `Icon`；或 `list::ListItem` | 可一比一 |
| `@`/`#` 模式切换（同一输入框换数据源与 placeholder） | `CommandState::set_query` + 自己按首字符切 delegate；placeholder 用 `Command::placeholder(...)` | 需自行组合 |
| 分段结果（打开的标签页 / 最近 / 其他） | `gpui_component::command::CommandGroup`（`CommandGroup::new().label(...).items(...)`） | 可一比一 |
| 搜索结果高亮（fuzzy 匹配字符加粗/变色） | 自绘 span；`gpui_component::highlighter` 面向代码，不适用 | 需自行组合 |
| 键盘导航（↑↓ 环绕、Enter、Esc / Cmd+K） | `gpui::KeyBinding::new` + `gpui_base::actions::{SelectUp, SelectDown, Confirm{secondary}, Cancel, SelectFirst, SelectLast}` + `.key_context(...)` | 可一比一（`Command` 自带 query/selected_index） |
| 文件图标（按扩展名着色） | `gpui_component::Icon` + 自己的图标名映射表 | 需自行组合 |

---

## 7. 未查清

1. **数据库 sidecar 的实际实现位置与协议**：`lithe-database` crate 在本仓库不存在（无 `name = "lithe-database"` 的 Cargo.toml），Windows Tauri host 也没有 spawn/stdio 代码；协议版本常量、帧格式、超时值只在错误文案里留下线索（`lib/database-errors.ts:20-41`：`Database sidecar panic:` / `Unsupported database sidecar protocol version` / `Invalid database sidecar (response|envelope)` / `missing (protocolVersion|result)` / `timed out after N seconds`）。**未找到**真正的实现与协议定义。
2. **数据库连接凭据的服务端落点**：`store_db_credential` / `get_db_credential` 在 Rust 侧未找到；是否复用 `secure_storage`（keyring）或另有实现，**未找到**。同理 `list_saved_connections` / `save_connection` / `delete_saved_connection` 的持久化位置（SQLite？JSON？store 插件？）**未找到**。
3. **AI 聊天库的表结构**：`init_chat_database` / `save_chat` / `load_chat` / `load_all_chats` / `update_chat_metadata` / `delete_chat` 在 Rust 侧未找到，TS 侧只有 snake_case 镜像类型（`services/ai-chat-history-service.ts:11-51`），**SQLite 表/索引定义未找到**。
4. **ACP 子进程 spawn/stdio 细节**：`start_acp_agent` / `send_acp_prompt` 等命令无 Windows Rust 实现；`rust/lithe-core/src` 无 `acp` 命中；`shared/contracts/` 无 AI chat/ACP 契约。**未找到**。
5. **AI 的 `acp-event` / `codex-event` 事件载荷的生产端**：前端只看到 `listen`（`acp-stream-handler.ts:305-309`、`codex-integration-service.ts:69`），Windows 侧无 `emit` 这些事件名的代码。**未找到**。
6. **`Esc` 停止生成**：`shortcut={isStreaming ? "escape" : "enter"}` 只用于 tooltip（`chat-input-bar.tsx:1160`），**未找到**对应 keydown 处理（Escape 仅用于关闭 slash/mention 下拉，`chat-input-bar.tsx:500,518`）。
7. **AI 消息搜索（`Cmd+F`）的匹配与高亮实现**：只确认到入口与计数（`ai-chat.tsx:142-147`、`chat-header.tsx:205-271`），`features/ai/utils/message-search.ts` 未逐行核对。
8. **全局搜索「替换」的写入通道**：`utils/source-replace.ts` 的 `replaceNextInSource` / `replaceAllInSources` 未逐行核对，其最终落点（文档写入命令 / Core 命令 / 直接 fs）**未确认**。
9. **快速打开的工作区符号 LSP 方法名与错误处理**：只确认前端调用 `LspClient.getWorkspaceSymbols`（`use-workspace-symbol-search.ts:84`），未核对 `features/editor/lsp/lsp-client.ts` 与 `windows/tauri/src-tauri/src/lsp.rs` 的实际方法/命令串。
10. **`useSymbolSearch`（`@` 当前文件符号）的数据来源**：未逐行核对该 hook（`hooks/use-symbol-search.ts`），不确定走的是 Monaco 模型还是 LSP。
11. **`@shadcn/react/message-scroller` 的内部实现**：自动滚动判定与粘底恢复算法不在本仓库（`ui/message-scroller.tsx:1-6`），**未找到**。
12. **gpui-kit 侧尚未核实的具体点**：`gpui_component::empty::Empty` 的组合 API（前端 `Empty/EmptyHeader/EmptyMedia/EmptyTitle/EmptyDescription/EmptyContent` 六件套是否都有对应）；`gpui_component::bubble` / `message` 的完整 API；`CommandState` 是否支持「同一输入框切数据源」；`Popover` 是否支持 `w-(--anchor-width)` 式宽度绑定。以上四项**未逐行读源码**（本次只确认了模块存在与大致形状）。
13. **`ai.*` 部分键的引用点**：`ai.validating`、`ai.saveKey`、`ai.notSynced/syncing/syncPaused/synced`、`ai.agentDefault`（除 `sidebar-history` 外）在 AI 组件中未观察到引用，是否为遗留键**未确认**。
14. **AI 设置页（`features/settings/components/tabs/ai-settings.tsx`）**：仅确认为 `ModelSelector` / `ProviderSelector` 的唯一使用处，未展开其内部规格（超出本次三个功能面的范围）。
15. **`features/database/components/database-sidebar.tsx` 的最终命运**：它是完整的「数据库连接管理器」UI（列表/选 provider/文件/网络四态 + 拖放 + 错误行），但当前无挂载点。是「待接线」还是「已废弃」，代码与配置里**没有线索**（无 TODO、无注释说明）。
