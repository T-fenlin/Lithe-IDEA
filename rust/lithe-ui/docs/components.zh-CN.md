# Lithe-UI 组件总览（中文）

> 覆盖 `rust/lithe-ui/src/` 下 **45 个模块的全部公开导出**。
> 每个条目给：组件名 / 所在模块路径 / 一句话中文说明 / 最小可用代码片段（签名取自源码）。
> 类型一列：`组件` = 可渲染；`数据模型` = 数据结构（不是组件）；`枚举` = 状态/分类；`函数` = 工具函数。
> 想看完整链式方法与典型组合，打开 [`components/`](./components) 下对应域的文件。

图例：`id` 都接受 `impl Into<ElementId>`（`&str`、整数、元组均可）。

---

## 1. `src/account/` — 账户、登录、计费、团队

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `LoginForm` | 组件 | 邮箱 + 密码 + 记住我，含"忘记密码/注册"入口；Enter 提交 |
| `SignupForm` | 组件 | 姓名、邮箱、新密码（带规则勾选）、条款；全部合法才可提交 |
| `MagicLinkForm` | 组件 | 邮箱登录链接表单 |
| `ForgotPassword` | 组件 | 找回密码：填地址 → 提示已发送 |
| `TwoFactorInput` | 组件 | 六格验证码，填满即回调；失败后自动清空，支持恢复码 |
| `OAuthButtons` | 组件 | "Continue with GitHub" 这类第三方登录按钮堆叠 |
| `ProfileCard` | 组件 | 头像/姓名/职位/简介 + 自定义操作 |
| `ProfileEditor` | 组件 | 编辑资料（头像、姓名、用户名、邮箱、简介、地点） |
| `AccountSwitcher` / `WorkspaceSwitcher` | 组件 | 已登录账号 / 工作区切换菜单 |
| `UserMenu` | 组件 | 头像菜单：当前账号 + 自定义行 + 退出登录 |
| `SessionList` | 组件 | 登录设备列表，可单独或全部登出 |
| `ApiKeyManager` | 组件 | API 密钥管理：创建（密钥只显示一次）、吊销 |
| `TeamMemberTable` | 组件 | 团队成员表：改角色、移除 |
| `InvitationList` | 组件 | 邀请列表 + 新建邀请 |
| `RoleSelector` | 组件 | 角色选择（每个角色带权限说明） |
| `PaymentMethodForm` | 组件 | 银行卡表单（号/有效期/CVC/持卡人），带校验 |
| `SubscriptionCard` | 组件 | 当前套餐卡片：价格、续费、升级/取消 |
| `UsageQuota` | 组件 | 配额使用条（接近上限转琥珀色、到顶转红） |
| `BillingHistory` | 组件 | 历史账单列表，可下载 |
| `UpgradePrompt` | 组件 | 超限提示：升级引导（可作付费墙，也可只显示一行） |
| `Login` / `Signup` | 数据模型 | 表单交给宿主的凭据结果 |
| `Profile` | 数据模型 | 一份资料 |
| `Account` / `Workspace` | 数据模型 | 账号 / 工作区 |
| `Session` | 数据模型 | 一个登录会话（设备、地点、最后活跃） |
| `ApiKey` | 数据模型 | 密钥（只留首尾） |
| `Member` / `Role` / `Invitation` | 数据模型 | 成员 / 角色 / 邀请 |
| `Subscription` / `Quota` / `Invoice` | 数据模型 | 套餐 / 配额 / 账单 |
| `CardDetails` / `CardBrand` | 数据模型 / 枚举 | 卡信息 / 卡组织 |
| `Standing` / `InvoiceState` / `ExpiryError` | 枚举 | 订阅状态 / 账单状态 / 有效期错误 |
| `password_rules` / `luhn` / `expiry` / `quota_line` | 函数 | 密码规则、卡号校验、有效期校验、配额文案 |

```rust
LoginForm::new("login")
    .on_login(|login, _window, cx| { /* 交给宿主 */ });

ProfileCard::new("me", "颜思齐")
    .title("UI 设计师")
    .email("yan@example.com");

UpgradePrompt::new("upgrade", "已达项目上限")
    .benefits(["无限项目", "团队共享"])
    .on_upgrade(|_, _, cx| {});
```

---

## 2. `src/agent/` — 智能体（Agent）运行与工具调用

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `AgentStepList` | 组件 | 智能体步骤时间线（每步带状态标记） |
| `AgentPlan` | 组件 | 计划清单：等待/进行/完成，已完成置灰划掉 |
| `AgentProgress` | 组件 | 进度总览：当前步、已完成数、耗时、分步进度条 |
| `AgentStatus` | 组件 | 状态胶囊（规划中/工作中会动） |
| `ToolRegistry` | 组件 | 工具注册表：按来源分组、允许/询问/拒绝 |
| `ToolCallCard` | 组件 | 一次工具调用（状态、名称、耗时，展开看入参与返回） |
| `ToolCallGroup` | 组件 | 折叠的多次调用，如"读取 3 个文件" |
| `ToolApprovalDialog` | 组件 | 工具执行前的授权对话框（拒绝/运行） |
| `PermissionPrompt` | 组件 | 就地询问：允许一次 / 始终允许 / 拒绝 |
| `FileChangeCard` / `MultiFileDiffReview` | 组件 | 单文件改动卡 / 多文件改动统一审阅 |
| `ArtifactPanel` | 组件 | 产物面板：标题、版本切换、复制、下载、预览或源码 |
| `MemoryPanel` / `CheckpointList` | 组件 | 长期记忆管理 / 回滚检查点列表（可 Rewind） |
| `McpServerList` | 组件 | MCP 服务器与连接器列表（状态、工具数、开关） |
| `SandboxStatus` | 组件 | 沙箱状态（名称、占用、运行时长） |
| `CostBreakdown` | 组件 | 成本构成：总额 + 各部分占比条 |
| `HumanInputRequest` | 组件 | 智能体等待人类选择/输入的卡片 |
| `BrowserPreview` / `ComputerUseViewer` / `LivePreview` | 组件 | 浏览器画面 / 电脑操作画面（含指针高亮）/ 实时预览 |
| `AgentStep` | 数据模型 | 一步：做了什么、状态、说明、耗时 |
| `FileChange` / `Memory` / `Checkpoint` / `McpServer` / `RegisteredTool` | 数据模型 | 改动 / 记忆 / 检查点 / 服务器 / 工具 |
| `AgentState` / `ChangeState` / `SandboxState` / `ServerState` / `ToolAccess` / `Permission` | 枚举 | 各类状态与权限 |

```rust
AgentStepList::new("steps", steps);
ToolRegistry::new("tools", tools).query(&query);
PermissionPrompt::new("perm", "运行 shell 命令");
```

---

## 3. `src/buttons/` — 按钮与按钮组

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Button` | 组件 | 主按钮：`Primary/Secondary/Ghost/Subtle/Success/Danger/Link/Custom` 八种变体 |
| `ButtonGroup` | 组件 | 边缘相接的按钮组 |
| `ToggleButton` / `ToggleGroup` / `ToggleItem` | 组件 | 可保持按下的按钮 / 单选或多选按钮组 / 其中一项 |
| `SegmentedControl` | 组件 | 分段控件，滑块滑到选中项，过窄自动省略或横向滚动 |
| `ActionBar` / `BulkActionBar` | 组件 | 区域底部操作条（停靠或浮动胶囊）/ 选中后出现计数与批量操作 |
| `FloatingActionButton` | 组件 | 悬浮圆形主操作按钮（带文字则成胶囊） |
| `buttons::presets`（`more_button` 等） | 函数 | 预设图标按钮：`back_button` / `close_button` / `more_button`；**没有独立的 `IconButton` 类型**，返回的就是 `Button` |
| `ConfirmButton` | 组件 | 危险操作二次确认（长按或再点一次） |
| `CopyButton` | 组件 | 复制文本并显示对勾 |
| `ShareButton` | 组件 | 打开系统分享面板（仅 macOS） |
| `QuickAction` / `QuickActions` | 组件 / 数据模型 | 快捷操作项（含快捷键）/ 快捷操作列表（指针或方向键选择，Enter 执行） |
| `ActionSheet` | 组件 | 底部升起的选择列表（带取消） |
| `ButtonVariant` / `ConfirmMode` | 枚举 | 变体 / 确认方式 |
| `ButtonCustomVariant` | 数据模型 | 自定义配色（底、前景、hover、active、阴影） |

```rust
Button::new("save")
    .label("保存")
    .variant(ButtonVariant::Primary)
    .size(Size::Medium)
    .icon(IconName::Save)
    .shortcut("secondary-s")
    .on_click(|_, window, cx| { /* … */ });

SegmentedControl::new("view", selected).item("列表").item("看板");
ToggleGroup::new("filters").multiple(true).item(ToggleItem::new("done", "已完成"));
```

---

## 4. `src/calendar/` — 日历与日程

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `CalendarMonthView` | 组件 | 月视图：跨天事件画成横条，其余按时段画成 chip |
| `CalendarDays` / `TimeGrid` | 组件 | 多日视图（泛型 `DAYS`）/ 日网格（拖拽空白处新建、拖下边缘改结束时间） |
| `AgendaView` | 组件 | 议程：按天列出事件 |
| `YearView` | 组件 | 年视图：十二个小月，有事件的日带点 |
| `AllDayRow` | 组件 | 全天事件泳道 |
| `CurrentTimeIndicator` | 组件 | 时间网格上的"当前时刻"红线 |
| `TimezoneOverlay` | 组件 | 在小时列旁显示另一时区的时钟 |
| `EventCard` / `EventChip` | 组件 | 事件的块状卡片 / 行内 chip |
| `EventPopover` | 组件 | 事件详情气泡（标题、时间地点、参与人、操作） |
| `EventEditor` | 组件 | 事件编辑表单（标题、时间、地点、颜色、重复、提醒、备注） |
| `RecurrenceEditor` | 组件 | 重复规则编辑器（日/周/月/年、间隔、截止） |
| `ReminderPicker` | 组件 | 提醒时间选择（可添加多个、可移除） |
| `AttendeeList` | 组件 | 参与人列表（头像、姓名、答复状态） |
| `AvailabilityPicker` | 组件 | 开放时段选择（按天列出可选时段） |
| `Event` | 数据模型 | 事件：键、标题、时间（`When`）、配色、地点 |
| `EventDraft` | 数据模型 | 编辑器交回的事件各字段 |
| `When` / `Frequency` / `Ends` / `Answer` | 枚举 | 时间形态 / 重复频率 / 结束方式 / 邀请答复 |
| `Recurrence` | 数据模型 | 重复规则 |

```rust
CalendarMonthView::new(month_args);          // 见 components/14-其余业务域.md
EventChip::new("chip-1", event).on_click(|_, _, cx| {});
RecurrenceEditor::new("recur", first_day, None);
```

---

## 5. `src/canvas/` — 画布、白板、节点图

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `InfiniteCanvas` | 组件 | 无限画布（平移/缩放） |
| `Whiteboard` | 组件 | 白板（画笔、图形、便签） |
| `NodeGraph` | 组件 | 节点图（节点 + 连线 + 端口） |
| `MindMap` | 组件 | 思维导图 |
| `FlowChart` | 组件 | 流程图（步骤 + 连线） |
| `Artboard` / `Shape` | 组件 / 数据模型 | 画板 / 图形（含 `ShapeKind`） |
| `SelectionBox` / `TransformHandles` / `SnapIndicator` | 组件 | 选框 / 变换控制点 / 吸附指示线 |
| `ToolPalette` / `BrushSettings` | 组件 | 工具面板 / 笔刷设置 |
| `InspectorPanel` / `ColorPanel` / `TypographyPanel` / `LayerPanel` / `AssetPanel` / `HistoryPanel` | 组件 | 检查器 / 颜色 / 字体 / 图层 / 资源 / 历史面板 |
| `BorderEditor` / `ShadowEditor` | 组件 | 描边 / 阴影编辑器 |
| `AlignmentToolbar` | 组件 | 对齐工具条 |
| `AutoLayoutControls` / `ConstraintsEditor` | 组件 | 自动布局 / 约束编辑 |
| `MiniMap` / `ZoomControls` / `ZoomIndicator` | 组件 | 缩略图 / 缩放控制 / 缩放指示 |
| `ExportPanel` | 组件 | 导出面板（格式、设置） |
| `ToolLayer` | 组件 | 笔迹绘制层 |
| `Frame` / `Viewport` | 数据模型 | 画框 / 视口（x, y, zoom） |
| `Node` / `Edge` / `Port` / `Link` / `Topic` / `Step` / `Board` | 数据模型 | 图元与结构 |
| `Brush` / `Handle` / `Tool` / `Align` / `Flow` / `Place` / `Pin` / `Constraints` / `AutoLayout` / `TypeStyle` / `Border` / `Shadow` / `Dash` / `Corner` / `Guide` | 数据模型 / 枚举 | 画布状态与样式 |
| `ExportFormat` / `ExportSetting` / `export_files` | 枚举 / 数据模型 / 函数 | 导出 |

```rust
InfiniteCanvas::new("board", viewport);
MindMap::new("mind", root_topic, viewport);
ToolPalette::new("tools", Tool::Select);
```

---

## 6. `src/charts/` — 图表

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `LineChart` / `AreaChart` / `BarChart` / `ScatterChart` | 组件 | 折线 / 面积 / 柱状 / 散点（气泡） |
| `PieChart` | 组件 | 饼图（`donut` 中空显示合计） |
| `RadarChart` | 组件 | 雷达图（多系列，图例可点掉） |
| `Histogram` / `BoxPlot` / `ViolinPlot` / `WaterfallChart` | 组件 | 分布与构成 |
| `FunnelChart` / `SankeyChart` / `ChordDiagram` | 组件 | 漏斗 / 桑基 / 弦图 |
| `GanttChart` | 组件 | 甘特图（依赖箭头、今日线） |
| `BurndownChart` | 组件 | 燃尽图（理想线 vs 实际） |
| `CalendarHeatmap` | 组件 | 全年热力日历 |
| `HeatmapChart` / `Treemap` | 组件 | 热力网格 / 矩形树图 |
| `Sunburst` | 组件 | 旭日图 |
| `BulletChart` / `ProgressChart` | 组件 | 子弹图 / 环形目标进度 |
| `ParallelCoordinates` | 组件 | 平行坐标 |
| `NetworkGraph` | 组件 | 力导向关系图（可拖节点） |
| `RealtimeChart` | 组件 | 实时流折线 |
| `ChartLegend` / `ChartTooltip` | 组件 | 图例 / 数值提示 |
| `Series` / `Points` / `Slice` / `Bullet` / `Task` | 数据模型 | 系列、点、切片、度量、任务 |

```rust
LineChart::new("chart", categories).series(Series::new("本周", values));
PieChart::new("share").donut(true).slice("直接访问", 42.0);
GanttChart::new("gantt").task(Task::new("联调", start, end));
```

---

## 7. `src/chat/` — AI 对话

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `ChatContainer` | 组件 | 对话容器（消息列 + 底部输入器） |
| `MessageBubble` | 组件 | 单条消息（用户右侧气泡 / 助手正文 / 系统居中 / 错误） |
| `MessageHeader` / `MessageFooter` / `MessageAvatar` | 组件 | 消息头（谁、何时、模型）/ 尾（token 与耗时 + 操作）/ 头像 |
| `MessageActions` / `MessageEditor` / `BranchNavigator` / `QuoteReply` | 组件 | 消息操作行 / 编辑 / 多版本切换 / 引用回复 |
| `PromptInput` | 组件 | 输入器：附件与上下文、Enter 发送、`/` 命令、`@` 上下文、拖文件即附件 |
| `SendButton` / `AttachmentButton` / `InputHint` / `DragDropOverlay` | 组件 | 发送 / 附件 / 按键提示 / 拖放遮罩 |
| `MessageList` / `ScrollToBottomButton` / `DateSeparator` | 组件 | 虚拟消息列 / 回到底部 / 日期分隔 |
| `StreamingText` / `StreamingMarkdown` / `StreamingCursor` | 组件 | 流式文本 / 流式 Markdown / 等待光标 |
| `ThinkingIndicator` / `ThinkingBlock` / `ThinkingDuration` | 组件 | 思考中 / 推理过程 / 思考耗时 |
| `CodeBlock` | 组件 | 消息中的代码块（复制、下载、运行、应用） |
| `SourceList` / `SourceCard` / `CitationBadge` | 组件 | 来源列表 / 来源卡 / 正文引用角标 |
| `SearchProgress` / `WebResultCard` / `DocumentChunkPreview` | 组件 | 检索步骤 / 网页结果 / 文档片段 |
| `ConversationList` / `ConversationItem` | 组件 | 会话列表 / 会话行（重命名、置顶、删除） |
| `ParameterPanel` / `SystemPromptEditor` / `TokenCounter` / `CostEstimator` | 组件 | 参数面板 / 系统提示词 / token 计数 / 成本估算 |
| `WelcomeScreen` / `SuggestionChips` / `FollowUpSuggestions` / `CapabilityCards` | 组件 | 欢迎页 / 起始 chip / 追问 / 能力卡 |
| `ImageMessage` / `ImageGrid` / `FileMessage` / `LinkPreviewCard` | 组件 | 图片 / 图片网格 / 文件 / 链接预览 |
| `AudioMessage` / `VideoMessage` / `VoiceInputButton` / `VoiceWaveform` | 组件 | 语音消息 / 视频消息 / 语音输入 / 录音波形 |
| `ErrorMessage` / `RateLimitNotice` / `FeedbackForm` | 组件 | 错误 / 限流提示 / 反馈表单 |
| `ProjectList` / `ProjectKnowledgePanel` / `SharedConversationView` / `ConversationExport` | 组件 | 项目列表 / 项目知识 / 分享只读视图 / 导出 |
| `Conversation` / `Project` / `Source` / `Attachment` / `Parameter` | 数据模型 | 会话 / 项目 / 来源 / 附件 / 参数 |
| `Role` / `StepState` | 枚举 | 消息角色 / 步骤状态 |

```rust
ChatContainer::new("chat").messages(list).composer(
    PromptInput::new("prompt", &field).on_send(|_, _, cx| {})
);
MessageBubble::new("m1", Role::User, "颜思齐").body("这个菜单为什么是透明的？");
```

---

## 8. `src/collab/` — 协同编辑

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `PresenceAvatars` | 组件 | 在线者头像（各自颜色描边，悬浮显示位置） |
| `LiveIndicator` | 组件 | "有人在线"呼吸点 + 人数 |
| `FollowMode` | 组件 | 跟随他人视图（彩色边框 + 姓名条，Esc 退出） |
| `RemoteCursor` / `RemoteSelection` | 组件 | 他人光标（带姓名旗）/ 他人选区（覆盖在输入框上） |
| `CommentThread` / `CommentSidebar` / `CommentMarker` | 组件 | 评论线程 / 评论侧栏 / 正文上的评论标记 |
| `Reply` | 组件 | 回复输入 + 发送（Enter 也可） |
| `Reactions` / `ReactionPicker` | 组件 | 表情回应（切换我的）/ 表情选择器 |
| `AnnotatedText` | 组件 | 带高亮批注的文本（悬浮看批注） |
| `TrackChanges` / `SuggestionMode` | 组件 | 修订模式（删除划掉、新增下划线，逐条接受/拒绝）/ 模式切换 |
| `ShareDialog` / `AccessList` / `InviteInput` / `LinkAccess` / `PermissionSelect` | 组件 | 分享对话框 / 权限名单 / 邀请输入 / 链接权限 / 权限选择 |
| `Annotation` / `Comment` / `Thread` / `Reaction` / `Peer` / `Suggestion` / `Member` | 数据模型 | 批注 / 评论 / 线程 / 回应 / 协作者 / 建议 / 成员 |
| `EditMode` / `Decision` / `Role` / `FollowMode`(枚举) | 枚举 | 编辑模式 / 评审决定 / 角色 |
| `accept` / `suggested` / `toggled` | 函数 | 修订与回应的辅助函数 |

```rust
PresenceAvatars::new("peers", peers);
CommentThread::new("t1", thread);
TrackChanges::new("track", suggestions).mode(EditMode::Suggest);
```

---

## 9. `src/dashboard/` — 仪表盘与监控

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `DashboardGrid` | 组件 | 可拖拽、可缩放的仪表盘网格（窄屏自动单列） |
| `DashboardCard` | 组件 | 组件框（标题、操作、主体、页脚） |
| `DashboardFilterBar` / `DashboardFilter` | 组件 | 筛选条（时间窗口 + 各筛选器 + 清除）/ 单个筛选器 |
| `RefreshIntervalSelector` | 组件 | 刷新间隔选择 + 立即刷新 + 上次刷新时间 |
| `EventStream` | 组件 | 实时事件流（按类型过滤、暂停时计数） |
| `AlertList` / `IncidentCard` | 组件 | 告警列表（可确认）/ 事故卡片（含更新流） |
| `ServiceStatus` / `HealthCheck` | 组件 | 服务状态总览 / 健康检查（点 + 词 + 耗时） |
| `UptimeBar` | 组件 | 按天的可用性条 |
| `Tile` | 数据模型 | 网格中一个瓦片的位置与尺寸 |
| `TimeWindow` / `Health` / `Check` / `AlertState` / `IncidentPhase` | 枚举 | 时间窗 / 健康度 / 检查结果 / 告警状态 / 事故阶段 |
| `MonitorAlert` / `Incident` / `IncidentUpdate` / `Service` / `StreamEvent` | 数据模型 | 告警 / 事故 / 更新 / 服务 / 事件 |
| `arranged` / `stepped` / `overall` / `uptime` | 函数 | 布局与统计辅助 |

```rust
DashboardGrid::new("grid", tiles);
DashboardCard::new("活跃用户").body(statistic);
AlertList::new("alerts", alerts).on_open(|_, _, cx| {});
```

---

## 10. `src/data_display/` — 数据展示小件

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Avatar` / `AvatarGroup` / `UserChip` | 组件 | 头像（图片/首字母/图标 + 在线点）/ 头像组 / 姓名 pill |
| `Badge` / `CountBadge` / `DotBadge` / `Tag` | 组件 | 状态徽标 / 数字徽标（可叠在图标上）/ 小圆点 / 标签（可移除） |
| `Statistic` / `KpiCard` / `TrendIndicator` | 组件 | 数值（滚动动画、单位、变化）/ KPI 卡 / 趋势箭头 |
| `Meter` / `UsageBar` / `Gauge` | 组件 | 占用条（接近上限变色）/ 分段占用条 / 仪表盘 |
| `Sparkline` | 组件 | 迷你图（折线/面积/柱状） |
| `Stars` | 组件 | 星级评分（只读，支持小数） |
| `Timeline` / `TimelineItem` | 组件 | 时间线 / 时间线事件 |
| `ActivityFeed` | 组件 | 活动流（"谁对什么做了什么"） |
| `DescriptionList` / `PropertyGrid` / `PropertyGroup` | 组件 | 描述列表 / 属性表（可折叠分组）/ 分组 |
| `Changelog` / `Release` | 组件 / 数据模型 | 变更日志（时间轴）/ 一个版本 |
| `Carousel` | 组件 | 轮播（箭头、圆点、键盘） |
| `Gallery` | 组件 | 图片网格（点击开灯箱） |
| `BeforeAfter` | 组件 | 前后对比（拖分割线） |
| `Comparison` | 组件 | 方案对比表 |
| `QrCode` / `Barcode` | 组件 | 二维码 / 条形码 |
| `Watermark` | 组件 | 平铺水印（不拦截点击） |
| `Presence` / `Tone` / `Change` | 枚举 | 在线状态 / 徽标色调 / 变更类型 |
| `BarcodeError` / `QrError` | 枚举 | 编码错误 |

```rust
Avatar::new("u1", "颜思齐").presence(Presence::Online);
Badge::new("已发布").tone(Tone::Success);
Statistic::new("stat", "本周提交", 128.0).unit("次");
Meter::new("disk", "磁盘", 0.82);
```

---

## 11. `src/debug/` — 调试器界面

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `DebugToolbar` | 组件 | 调试工具条（继续/暂停、三种单步、重启、停止），提示里带各调试器的惯用键 |
| `BreakpointList` | 组件 | 断点列表（开关、条件、命中次数、跳转、删除） |
| `CallStack` / `ThreadList` | 组件 | 调用栈（库帧折叠）/ 线程列表 |
| `VariablesPanel` / `WatchPanel` | 组件 | 变量树（按类型着色，变化高亮）/ 监视表达式 |
| `Disassembly` | 组件 | 反汇编（地址、字节、指令着色、源码行、断点） |
| `HexViewer` | 组件 | 十六进制查看器（选字节、数值解析） |
| `NetworkInspector` | 组件 | 网络请求列表 + 瀑布图 + 展开看头与体 |
| `Flamegraph` | 组件 | 火焰图（悬停读帧，点击下钻） |
| `TimelineProfiler` | 组件 | 时间线性能分析（滚轮缩放、平移） |
| `Breakpoint` / `StackFrame` / `Thread` / `Variable` / `Watch` / `Instruction` / `ProfileFrame` / `Placed` / `Span` / `Request` | 数据模型 | 各类调试数据 |
| `DebugState` / `ThreadState` / `ValueKind` / `DebugCommand` | 枚举 | 运行状态 / 线程状态 / 值类型 / 调试命令 |
| `place` / `zoomed` | 函数 | 火焰图布局 / 时间线缩放 |

```rust
DebugToolbar::new("dbg", DebugState::Paused).on_command(|cmd, _, cx| {});
VariablesPanel::new("vars", variables);
BreakpointList::new("bps", breakpoints);
```

---

## 12. `src/devtools/` — 开发者工具（API / 数据库 / 容器）

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `ApiRequestBuilder` | 组件 | 构造请求（方法、地址、鉴权、头、体） |
| `ResponseViewer` / `HeadersTable` | 组件 | 响应查看（状态、体）/ 头表格 |
| `CollectionTree` | 组件 | 请求集合树（可移动） |
| `ConnectionManager` / `ConnectionForm` | 组件 | 连接管理 / 连接表单（带测试） |
| `QueryHistory` / `QueryPlanViewer` | 组件 | 查询历史 / 执行计划 |
| `SchemaTree` / `ERDiagram` / `TableStructureEditor` / `IndexManager` | 组件 | 库表树 / ER 图 / 表结构编辑 / 索引管理 |
| `JsonViewer` / `YamlViewer` / `XmlViewer` | 组件 | JSON / YAML / XML 查看器 |
| `JsonEditor` | 组件 | JSON 编辑器（基于 `CodeEditor`） |
| `RedisKeyBrowser` | 组件 | Redis 键浏览 |
| `WebSocketConsole` | 组件 | WebSocket 控制台 |
| `GraphQLExplorer` | 组件 | GraphQL 浏览器（类型、字段） |
| `ContainerList` / `PodList` | 组件 | 容器列表 / Pod 列表 |
| `ResourceMonitor` | 组件 | 资源监控（读数图表） |
| `ColorContrastChecker` | 组件 | 对比度检查（可用 `contrast` / `luminance` / `over` / `grades` 函数） |
| `RegexTester` | 组件 | 正则测试（标志、匹配高亮） |
| `Encoder` | 组件 | 编解码（base64 / hex / percent） |
| `EnvironmentSelector` | 组件 | 环境选择 |
| `ApiRequest` / `ApiResponse` / `Connection` / `Datum` / `Schema` / `Field` / `DbTable` / `Index` / `Container` / `Pod` / `RedisKey` / `RedisValue` / `Saved` / `Environment` / `Outcome` / `QueryRun` / `PlanStep` / `Found` / `SocketMessage` / `SocketState` / `XmlNode` / `XmlElement` / `SchemaType` / `FieldKey` / `Reading` | 数据模型 | 各类工具数据 |
| `Method` / `Auth` / `Engine` / `ConnectionState` / `TestState` / `ContainerState` / `PodPhase` / `RegexFlag` / `Direction` / `TypeKind` / `Code` | 枚举 | 方法与状态 |
| `read_json` / `read_yaml` / `read_xml` / `read_geojson`(maps) / `base64` / `hex` / `unbase64` / `unhex` / `unpercent` / `percent` / `resolve` / `find` / `moved` / `reason` / `status_tone` | 函数 | 解析与编解码工具 |

```rust
ApiRequestBuilder::new("req", request);
ResponseViewer::new("res", response);
ColorContrastChecker::new("a11y");
```

---

## 13. `src/documents/` — 文档与 Markdown

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `MarkdownEditor` | 组件 | Markdown 编辑器三态：可视化 / 源码 / 分屏 |
| `RichTextEditor` | 组件 | 富文本（边写边生效、工具条、`@`/`:` 建议、字数统计） |
| `MarkdownRenderer` | 组件 | Markdown 渲染（标题、列表、任务、引用、代码、表格、数学、脚注） |
| `BlockEditor` | 组件 | 块编辑器（块数据 + 事件） |
| `FixedFormatToolbar` | 组件 | 固定格式工具条（行内样式、标题、引用、列表） |
| `DocumentViewer` | 组件 | 文档查看器（分页、缩放、缩略图、查找高亮、便签） |
| `EpubReader` | 组件 | EPUB 阅读器（章节跳转、字号调节） |
| `VersionHistory` / `PageHistoryDiff` | 组件 | 版本历史 / 逐词差异 |
| `Backlinks` / `Favorites` / `TrashBin` | 组件 | 反向链接 / 收藏 / 回收站 |
| `TemplatePicker` | 组件 | 模板选择（分类、预览、使用） |
| `PrintPreview` / `PrintSettings` | 组件 / 数据模型 | 打印预览 / 打印设置（纸张、方向、页边距、份数） |
| `Checklist` / `TodoItem` | 组件 | 待办清单 / 待办项 |
| `MarkdownRenderer` 相关：`outline` | 函数 | 生成大纲 |
| `LinkEditor` | 组件 | 链接编辑面板（校验、应用、移除、打开） |
| `ZenMode` / `ReadingProgress` / `Footnote` / `TooltipTerm` / `Glossary` | 组件 | 沉浸模式 / 阅读进度 / 脚注 / 术语提示 / 术语表 |
| `PageCover` / `PageIcon` | 组件 | 页面封面 / 页面图标 |
| `PageThumbnailList` | 组件 | 页面缩略图列表 |
| `WordCount` / `ReadingTime` | 组件 | 字数 / 阅读时长 |
| `BlockData` / `BlockKind` / `BlockEvent` / `Version` / `Backlink` / `Favorite` / `Trashed` / `Template` / `Chapter` / `DocPage` / `PageHit` / `PageNote` / `Node` / `Inline` / `Media` | 数据模型 | 文档数据 |
| `Format` / `MarkdownMode` / `Paper` / `Margins` / `Mark` / `Style` | 枚举 | 格式与模式 |
| `Align` | 枚举 | 对齐（与 `documents::blocks` 共用） |

```rust
MarkdownEditor::new("md", &state).mode(MarkdownMode::Split);
MarkdownRenderer::new(source);
DocumentViewer::new("doc", pages);
```

---

## 14. `src/editor/` — 代码编辑器与 IDE 部件

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `CodeEditor` | 组件 | 代码编辑器：语法着色、多光标、折叠、行号槽、缩略图、叠加层 |
| **`KeybindingsEditor`** | 组件 | **快捷键编辑器**：命令/键位/上下文/来源四列，查询过滤，冲突标记，双击录制新键 |
| `FindWidget` | 组件 | 查找/替换框（匹配数、上下切换、选项） |
| `SearchPanel` / `ReferencesPanel` | 组件 | 跨文件搜索结果 / 引用列表 |
| `SearchResultItem` / `FileHits` | 组件 / 数据模型 | 一条匹配行 / 一个文件的匹配 |
| `CompletionMenu` / `SignatureHelp` | 组件 | 补全菜单 / 签名帮助 |
| `HoverInfo` | 组件 | 悬浮信息 |
| `CodeActionMenu` / `RenameInput` | 组件 | 灯泡菜单（修复/重构/源码动作）/ 重命名输入框 |
| `ProblemsPanel` | 组件 | 问题面板（按文件分组、按严重级别统计） |
| `OutputPanel` / `DebugConsole` | 组件 | 输出面板（按通道）/ 调试控制台（REPL） |
| `SymbolOutline` / `GoToSymbol` / `CallHierarchy` | 组件 | 符号大纲 / 符号跳转面板 / 调用层次 |
| `TaskRunner` | 组件 | 任务运行器（按分组、运行/停止、展开看配置） |
| `ExtensionsPanel` | 组件 | 扩展市场面板 |
| `SettingsEditor` | 组件 | 设置编辑器（控件视图 / JSON 视图） |
| `WelcomePage` / `RecentProjects` / `ProjectSwitcher` | 组件 | 欢迎页 / 最近项目 / 项目跳转面板 |
| `ThemePreview` | 组件 | 主题缩略预览（用该主题自己的配色画一个小编辑器） |
| `InlineChat` | 组件 | 行间对话（在代码里提改动） |
| `CursorPosition` / `LanguageMode` / `Encoding` / `LineEnding` / `IndentSettings` / `BranchIndicator` / `LspStatus` / `NotificationBell` | 组件 | 状态栏各段 |
| `Diagnostic` / `InlayHint` / `CodeLens` / `GhostText` / `DiffHunk` / `ReadOnlyBanner` / `VimModeIndicator` | 组件 | 代码内的装饰层 |
| `Buffer` / `Selection` | 数据模型 | 源文本与行首偏移 / 光标选区 |
| `Keybinding` / `Setting` / `Problem` / `Task` / `Symbol` / `Call` / `Project` / `Hit` / `CompletionItem` / `Signature` / `CodeAction` / `Extension` / `ConsoleEntry` / `StartAction` | 数据模型 | 各类编辑器数据 |
| `KeySource` / `SymbolKind` / `LineNumbers` / `CursorShape` / `PopupKey` / `EditorEvent` / `VimMode` / `GitMark` / `ActionKind` / `SettingValue` / `TaskState` / `ExtensionState` / `ExtensionAction` / `LspState` / `FindOptions` / `SearchFilters` | 枚举 | 编辑器状态与选项 |
| **`conflicts`** | 函数 | **求与某绑定键位冲突的命令名（同一上下文内）** |
| `find_all` / `completions` / `passes` / `settings_json` | 函数 | 查找全部 / 补全 / 过滤判定 / 生成 settings.json |

```rust
// 快捷键编辑器（用户"设置里快捷键可自定义"的落点）
KeybindingsEditor::new("keymap", bindings)
    .query(query)
    .recording(recording_command)
    .on_record(|command, window, cx| { /* 双击某行 → 进入录制 */ })
    .on_bind(|command, keystroke, window, cx| { /* 录到新键 / 清除 */ });

let clash = conflicts(&bindings, &binding);        // 同一 when 下键位相同的其它命令
CodeEditor::new(text, window, cx);
```

---

## 15. `src/feedback/` — 反馈、状态与空态

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Alert` | 组件 | 页面内消息（严重级别图标 + 底色 + 标题 + 说明 + 操作 + 关闭） |
| `Banner` | 组件 | 顶部横幅 |
| `Callout` | 组件 | 阅读中的提示块（左侧色条 + 图标） |
| `InlineMessage` / `StatusMessage` | 组件 | 行内消息 / "刚刚发生了什么" |
| `Notification` / `NotificationCenter` | 组件 | 通知条目 / 通知中心（按天分组、全部已读、清空） |
| `Toast` / `Toaster` / `ToastViewport` | 组件 | 吐司（可带操作/撤销倒计时）/ 吐司队列（每窗口一个）/ 容器（右下角堆叠） |
| `EmptyState` | 组件 | 空状态（图标、标题、说明、操作） |
| `ErrorView` / `ErrorBoundary` | 组件 | 错误视图 / 错误边界（捕获并显示错误） |
| `ResultView` | 组件 | 结果页（自绘对勾/叉环 + 标题 + 说明 + 操作） |
| `SavingIndicator` / `SyncStatus` / `ConnectionStatus` | 组件 | 保存状态 / 同步状态 / 连接状态 |
| `Countdown` / `Timer` | 组件 | 倒计时（归零回调一次）/ 计时器（可暂停） |
| `ConfirmationCard` | 组件 | 就地二次确认 |
| `Suspense` / `AsyncView` | 组件 | 加载占位 / 后台加载值后渲染 |
| `SaveState` / `SyncState` | 枚举 | 保存 / 同步（严重级别用 `primitives::Severity`） |

```rust
Alert::new("a1", Severity::Warning).title("主题文件缺失").body("已回落到默认配色");
Toast::new("已保存").action("撤销", |_, _, cx| {});
EmptyState::new("empty", IconName::Inbox, "还没有项目").action("新建项目", |_, _, cx| {});
```

---

## 16. `src/files/` — 文件浏览与管理

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `FileExplorer` | 组件 | 文件夹三种视图：列表（按列排序）/ 磁贴 / 沿路径的分栏 |
| `FileGrid` | 组件 | 文件磁贴网格（键盘按格移动、Enter 打开） |
| `FileItem` | 组件 | 文件/文件夹行（图标 + 名称 + 大小 + 修改时间），可直接放进 `SelectableList` |
| `FileIcon` | 组件 | 按宿主图标主题取文件/文件夹图标 |
| `FilePreview` | 组件 | 快速预览（图片、文本前几行或类型图标 + 元信息） |
| `RecentFiles` | 组件 | 最近打开（按 Today / Yesterday / 日期分组） |
| `FileSearch` | 组件 | 文件搜索（宿主持有输入框，结果带所在目录与计数） |
| `ArchiveViewer` | 组件 | 压缩包内容树（体积、压缩率、总计） |
| `DuplicateFinder` | 组件 | 重复文件分组（标记删除、可释放空间统计） |
| `FileOperationProgress` | 组件 | 复制/移动进度（字节、剩余时间、暂停/继续/取消） |
| `TransferQueue` / `DownloadManager` | 组件 | 传输队列（上下行混合）/ 下载管理（进行中 + 已完成） |
| `DirEntry` / `RecentFile` / `FoundFile` / `ArchiveEntry` / `Duplicates` / `Transfer` | 数据模型 | 各类文件数据 |
| `FileView` / `FileOperation` / `TransferState` | 枚举 | 视图模式 / 操作类型 / 传输状态 |

```rust
FileExplorer::new("explorer", entries).view(FileView::List);
FileItem::new("f1", entry);
FileSearch::new("find", &field).results(found);
```

---

## 17. `src/finance/` — 交易与行情

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `CandlestickChart` | 组件 | K 线图（含 `ChartKind`、`ChartSync`） |
| `RenkoChart` / `PointFigureChart` | 组件 | 砖形图 / 点数图 |
| `DepthChart` / `OrderBook` / `DomLadder` / `TimeAndSales` | 组件 | 深度图 / 订单簿 / DOM 阶梯 / 逐笔成交 |
| `MarketHeatmap` / `MarketOverview` / `TradingSessionClock` | 组件 | 市场热力图 / 总览 / 交易时段时钟 |
| `TickerTape` / `QuoteCard` / `PriceText` / `PriceChangeBadge` | 组件 | 跑马灯 / 报价卡 / 价格文本 / 涨跌徽标 |
| `OptionChain` / `GreeksTable` | 组件 | 期权链 / 希腊值表 |
| `PayoffDiagram` | 组件 | 到期损益图 |
| `OrderEntry` / `OrderConfirmDialog` / `QuickTradeButtons` | 组件 | 下单 / 下单确认 / 快捷交易按钮 |
| `PositionTable` / `OrderTable` / `TradeHistoryTable` / `PnLDisplay` | 组件 | 持仓 / 委托 / 成交 / 盈亏 |
| `PortfolioSummary` / `PerformanceChart` / `DividendTable` / `FinancialStatementTable` | 组件 | 组合概览 / 业绩曲线 / 分红表 / 财务报表 |
| `RiskMeter` / `MarginIndicator` / `LeverageSlider` | 组件 | 风险 / 保证金 / 杠杆 |
| `BidAskBar` / `SpreadIndicator` | 组件 | 买卖盘条 / 价差指示 |
| `Watchlist` / `Screener` / `SymbolSearch` / `SymbolBadge` | 组件 | 自选 / 筛选器 / 代码搜索 / 代码徽标 |
| `DrawingToolbar` / `IndicatorSelector` | 组件 | 画图工具 / 指标选择 |
| `EarningsCalendar` / `EconomicCalendar` / `NewsFeed` | 组件 | 财报日历 / 宏观日历 / 新闻流 |
| `CurrencyConverter` / `CryptoWalletCard` / `TransactionList` | 组件 | 汇率换算 / 钱包卡 / 交易明细 |
| `Candle` / `Strike` / `OptionQuote` / `Leg` / `Trade` / `Side` / `Order` / `Position` / `Working` / `Fill` / `Dividend` / `Statement` / `Overlay` / `Study` / `Drawing` / `Earnings` / `Release` / `Story` / `Transfer` | 数据模型 | 行情与交易数据 |
| `ChartKind` / `Arrangement` / `Session` / `MarketStatus` / `OrderKind` / `Tool` / `FIB` / `Settled` | 枚举 / 常量 | 图表类型与状态 |
| `INTERVALS` / `RANGES` | 常量 | 周期与区间预设 |

```rust
CandlestickChart::new("kline", candles).kind(ChartKind::Candle);
OrderEntry::new("order", order, 123.45);
Watchlist::new("watch", watches);
```

---

## 18. `src/forms/` — 表单与输入控件（最大域）

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Input` / `PasswordInput` | 组件 | 文本框框体（边框、焦点、前后插槽）/ 带锁与显隐眼睛的密码框 |
| `InputState` | 数据模型 | 文本状态实体：**所有文本类控件的真源**（`text()` / 光标 / 选区 / 历史） |
| `SearchInput` | 组件 | 搜索框（放大镜、清除、空态显示最近搜索） |
| `EmailInput` / `UrlInput` | 组件 | 邮箱 / 链接（失焦时校验） |
| `NumberInput` / `ScrubInput` / `UnitInput` / `Stepper` | 组件 | 数字框（方向键/步进器/拖动擦洗）/ 可拖动标签 / 带单位 / 只能点加减 |
| `MaskedInput` / `PhoneInput` / `PinInput` | 组件 | 模式化输入 / 电话（国家码 + 号码模式）/ 一次性验证码分格 |
| `CodeInput` / `JsonInput` / `RegexInput` / `ExpressionInput` | 组件 | 单行代码 / JSON / 正则 / 表达式，边输入边着色校验 |
| `MentionInput` / `TagInput` | 组件 | `@`/`#` 唤起建议 / 标签输入（逗号或回车添加，退格删最后一个） |
| `InlineEdit` | 组件 | 点击文字变输入框（Enter 保存、Esc 还原） |
| `InputState::is_multi_line()` | 方法 | 多行输入**没有独立的 `Textarea` 类型**，由 `InputState` 自己承担（`forms/text/mod.rs:50`） |
| `Checkbox` / `CheckboxGroup` / `Radio` / `RadioGroup` / `Switch` | 组件 | 复选框（含半选）/ 复选组 / 单选 / 单选组 / 开关 |
| `CheckboxCard` / `RadioCard` | 组件 | 卡片式复选 / 单选 |
| `Select` / `Combobox` / `MultiSelect` / `ListBox` | 组件 | 下拉选择 / 可输入过滤 / 多选（chip 显示）/ 就地列表选择 |
| `Cascader` | 组件 | 级联选择（每列打开下一列） |
| `ChoiceChips` | 组件 | chip 组（单选或多选） |
| `Slider` / `RangeSlider` / `Knob` | 组件 | 滑块 / 双端滑块 / 旋钮 |
| `Rating` | 组件 | 星级评分（悬浮预览，点当前值可清除） |
| `ColorPicker` / `ColorSwatch` / `ColorPalette` / `GradientEditor` / `EyeDropper` | 组件 | 颜色选择 / 色块 / 色板 / 渐变编辑 / 吸管 |
| `FontPicker` / `IconPicker` / `EmojiPicker` | 组件 | 系统字体 / 库图标 / emoji 选择（可搜索） |
| `DatePicker` / `DateRangePicker` / `DateTimePicker` / `TimePicker` / `MonthPicker` / `WeekPicker` / `QuarterPicker` / `YearPicker` / `Calendar` / `DurationPicker` / `RelativeDatePicker` / `TimezoneSelect` / `CronEditor` | 组件 | 日期时间全系列 |
| `FileInput` / `DropZone` / `Upload` / `UploadList` / `AvatarUpload` / `PathInput` | 组件 | 文件选择 / 拖放区 / 上传项 / 上传列表 / 头像上传 / 路径选择 |
| `SignaturePad` | 组件 | 手写签名板（可清除） |
| `TransferList` | 组件 | 左右两栏穿梭选择 |
| `KeyValueInput` / `FieldArray` / `ListInput` | 组件 | 键值对 / 自定义行数组 / 文本列表 |
| `InputGroup` / `InputAddon` | 组件 | 带前后缀的输入框组合 |
| `Form` / `FormSection` / `FormField` / `FormLabel` / `FormError` / `InlineForm` / `DirtyIndicator` | 组件 | 表单骨架：Cmd-Enter 提交、分组、带标签控件、错误信息、行内表单、未保存指示 |
| **`HotkeyInput`** | 组件 | **快捷键录制框**：聚焦后按一次即录下，Backspace 清除，Esc 退出 |
| `Choice` / `Stroke` / `UploadState` / `CheckState` / `CronRule` / `RelativeRange` / `GradientStop` / `History` / `InputEvent` / `Highlight` / `Code`(见 devtools) | 数据模型 / 枚举 | 选项、笔迹、上传状态、复选状态等 |
| `is_email` / `is_url` / `evaluate` / `code_highlights` / `json_highlights` / `regex_highlights` / `mention_highlights` / `expression_highlights` / `read_csv` | 函数 | 校验与高亮 |

```rust
let state = cx.new(|cx| InputState::new(cx));
Input::new(&state).placeholder("搜索组件").prefix(Icon::new(IconName::Search).size(IconSize::Sm));

Select::new("lang", choices).selected("zh-CN").on_change(|value, _, cx| {});
Switch::new("auto-save", true).on_change(|on, _, cx| {});

// 快捷键录制（设置页里"改键"的那一个框）
HotkeyInput::new(("keymap", "keys-3"), binding.keys.clone())
    .on_change(|keystroke, window, cx| { /* None = 已清除 */ });
```

---

## 19. `src/generative/` — 生成式 AI 工具

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `PromptPlayground` | 组件 | 提示词试验场 |
| `PromptEnhancer` | 组件 | 提示词增强 |
| `GenerationGrid` / `VariationPicker` / `ABCompareView` | 组件 | 生成结果网格 / 变体选择 / A-B 对比 |
| `GenerationQueue` | 组件 | 生成队列（任务卡 + 状态） |
| `ModelCard` / `ModelDownloadManager` | 组件 | 模型卡（状态）/ 模型下载管理 |
| `InpaintCanvas` / `MaskBrush` | 组件 | 局部重绘画布 / 蒙版笔刷 |
| `StylePresetPicker` / `AspectRatioPicker` / `SeedInput` / `TTSVoicePicker` | 组件 | 风格预设 / 画幅 / 随机种子 / 语音选择 |
| `AudioGenerationPlayer` / `VideoGenerationTimeline` | 组件 | 音频生成播放 / 视频生成时间线 |
| `EmbeddingVisualizer` | 组件 | 向量可视化 |
| `HardwareMonitor` | 组件 | 硬件读数（显存/利用率） |
| `FineTuneJobCard` | 组件 | 微调任务卡 |
| `PromptDiff` / `PromptVersionHistory` | 组件 | 提示词差异 / 版本历史 |
| `PromptVersion` / `Job` / `Download` / `Outcome` / `Reading` / `Voice` / `StylePreset` / `Shot` / `MaskStroke` / `Verdict` / `Embedded` | 数据模型 | 生成相关数据 |
| `ModelState` / `ModelStatus` / `DownloadState` / `JobState` / `TunePhase` | 枚举 | 状态 |

```rust
PromptPlayground::new("play", &template);
GenerationGrid::new("grid", outcomes);
ModelDownloadManager::new("dl", downloads);
```

---

## 20. `src/git/` — 版本控制界面

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `DiffViewer` | 组件 | 单文件差异（统一/并排，词级高亮，长差异虚拟滚动） |
| `ThreeWayMerge` / `ConflictResolver` | 组件 | 三向合并（ours/base/theirs 逐区）/ 冲突标记文件解决器 |
| `ChangesList` / `CommitInput` | 组件 | 工作区改动列表（暂存区分开）/ 提交信息输入（50 字计数、Cmd-Enter、amend） |
| `CommitList` / `CommitItem` | 组件 | 提交历史列表 / 提交行（作者首字母、主题、ref、短 id、时间） |
| `FileHistory` | 组件 | 单文件历史（时间线） |
| `BranchList` / `BranchSelector` | 组件 | 分支列表（领先/落后、切换、删除）/ 分支选择面板（可新建） |
| `TagList` / `StashList` | 组件 | 标签列表 / 贮藏列表（apply/pop/drop） |
| `BlameView` / `GitBlameAnnotation` | 组件 | 逐行 blame（作者、时间、主题，按新旧褪色）/ 行尾注解 |
| `PullRequestCard` / `ReviewComment` | 组件 | PR 卡片（状态、分支、检查、评审人、评论、体量）/ 评审线程 |
| `GitStatusBadge` / `DiffStat` | 组件 | 文件状态字母徽标 / 增删统计（五个点） |
| `Commit` / `Branch` / `GitTag` / `Stash` / `Blame` / `PullRequest` / `ReviewNote` / `Changed` / `DiffLine` / `GraphRow` / `Stroke` / `Region` / `Conflict` | 数据模型 | Git 数据 |
| `LineKind` / `Stretch` / `Half` / `DiffLayout` / `GitMark` / `PullState` / `RegionKind` / `Take` / `ChangeAction` / `StashAction` | 枚举 | 差异与状态分类 |
| `diff` / `pairs` / `stat` / `lanes` / `regions` / `conflicts` / `resolve` / `result` | 函数 | 差异计算与合并求解 |

```rust
DiffViewer::new("diff", old, new).layout(DiffLayout::Split);
ChangesList::new("changes", changed).on_stage(|path, _, cx| {});
CommitInput::new("commit", &field).on_commit(|message, _, cx| {});
```

---

## 21. `src/i18n/` — 国际化

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `SkipLink` | 组件 | 仅在获得焦点时出现的跳转链接（跳到主内容），不占布局 |
| `Locale` | 数据模型 | 一种语言（数字、日期、行方向的写法） |
| `Direction` | 枚举 | 文本方向（LTR / RTL） |
| `LOCALES` | 常量 | 内置语言表 |

```rust
SkipLink::new("skip", "跳到主内容", target_handle);
let locale = LOCALES.iter().find(|locale| locale.id == "zh-CN");
```

---

## 22. `src/interaction/` — 交互行为（不是视觉件）

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Resizable` | 组件 | 可拖拽改变尺寸（右/下边与右下角把手，方向键步进） |
| `Rotatable` | 组件 | 可旋转的 svg（带旋钮） |
| `RovingFocus` | 组件 | 一组元素共用一个 Tab 停靠点（方向键在轴向上移动焦点） |
| `SelectionArea` | 组件 | 框选（拖出选区，Shift/Cmd 增减，键盘可遍历） |
| `ScrollSync` | 组件 | 多个滚动容器同步滚动 |

```rust
Resizable::new("box", size).min(px(120.)).max(px(480.));
SelectionArea::new("files").on_change(|keys, _, cx| {});
```

---

## 23. `src/language/` — 语言与语法高亮注册（非 UI）

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Language` | 数据模型 | 一门语言：id、名称、扩展名、文件名、语法（`grammar`）、高亮查询、行注释 |
| `Highlighter` | 数据模型 | 增量高亮器（`new` / `reparse` / `language`） |
| `Slot` | 枚举 | 高亮槽位（由 tree-sitter capture 名映射，可取自 `Syntax` 配色） |
| `register` / `all` / `by_extension` / `by_filename` / `by_path` | 函数 | 注册与查询语言 |

```rust
language::register(&LANGUAGES);
let rust = language::by_extension("rs");
let mut highlighter = Highlighter::new(rust.unwrap());
```

---

## 24. `src/layout/` — 布局与容器

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `h_stack()` / `v_stack()` | 函数 | 横向 / 纵向栈（项目里最常用的两个入口） |
| `ZStack` / `AspectRatio` / `Container` | 组件 | 层叠（首个子元素决定尺寸）/ 固定宽高比 / 居中有最大宽度的列 |
| `Card` / `CardHeader` / `Panel` / `Section` / `Fieldset` / `Frame` / `Well` | 组件 | 各类表面：卡片（头/体/脚）/ 卡片头 / 带标题栏的区域 / 页面分块 / 字段组 / 只有边框 / 下陷区域 |
| `Page` / `AppShell` / `MasterDetail` | 组件 | 页面（标题 + 说明 + 操作 + 主体）/ 窗口骨架（标题栏、侧栏、内容、状态栏）/ 左列表右详情 |
| `Dock` / `DockPanel` / `DockLayout` / `DockSide` / `Spot` / `SideLimits` / `PanelTitle` / `DockEvent` | 组件 / 数据模型 | 可停靠面板系统（侧边停靠与浮动、保存/恢复布局） |
| `SplitPane` | 组件 | 可拖动分隔的窗格（水平或垂直） |
| `PaneGroup` / `PaneLayout` / `PaneId` | 组件 / 数据模型 | 编辑器式分屏（右/下拆分、关闭、聚焦） |
| `Sidebar` | 组件 | 可折叠成图标轨的侧栏 |
| `Sheet` / `Drawer` / `FloatingPanel` | 组件 | 从边缘滑出的面板 / 底部抽屉（下拉关闭）/ 可拖动的浮动面板 |
| `ScrollArea` / `Scrollbar` | 组件 | 滚动区域（覆盖式滚动条，使用时出现）/ 滚动条 |
| `ScrollShadow` / `ScrollToTop` / `StickyHeader` | 组件 | 边缘阴影 / 回顶按钮 / 吸顶标题 |
| `Collapsible` / `Accordion` / `AccordionItem` | 组件 | 可折叠区（高度动画）/ 手风琴（一次开一个或可多个） |
| `SimpleGrid` / `Masonry` | 组件 | 等宽列网格 / 瀑布流 |
| `Viewport` / `Transform` | 组件 / 数据模型 | 可平移缩放的视口 / 内容到视口的映射 |
| `Workspace` | 数据模型 | 保存的窗口布局（dock + 编辑器窗格，JSON） |
| `resize_handle` / `resize_edge` / `on_axis` / `WORKSPACE_VERSION` | 函数 / 常量 | 分隔条与滚动辅助 |

```rust
v_stack().gap_2().child(Card::new().header(CardHeader::new("组件")).body(content));
SplitPane::new("split", Axis::Horizontal, px(160.)).child(left).child(right);
Dock::new("dock", layout);
```

---

## 25. `src/lists/` — 列表与树

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `List` / `ListItem` | 组件 | 列表容器（可加分隔线）/ 列表行（前导、标题、副标题、尾部） |
| `SelectableList` | 组件 | 可选列表（单选/多选、方向键、Shift 连选、Cmd-A、首字母跳转） |
| `SortableList` | 组件 | 可拖动重排（或 Alt+↑/↓ 移动当前行） |
| `Tree` / `TreeNode` / `TreeSelect` | 组件 | 树（键盘导航、过滤）/ 树节点 / 树选择 |
| `VirtualList` / `InfiniteList` | 组件 | 虚拟滚动长列表 / 触底加载更多 |
| `GroupedList` | 组件 | 分组列表（组头吸顶） |
| `FileTree` / `ProjectTree` | 组件 | 项目文件树（文件夹优先、图标、git 状态、过滤并展开命中目录） |
| `DirectoryListing` | 组件 | 目录列表（按列排序、路径回上一级） |
| `MessageScroller` / `MessageScrollerState` | 组件 | 消息滚动器（贴底、新消息计数） |
| `Outline` | 组件 | 文档大纲（按层级缩进 + 游标指示） |
| `SwipeableListItem` / `SwipeAction` | 组件 | 可左滑露出操作的行 / 一个滑动操作 |
| `DirEntry` / `TreeNode` / `ProjectNode` / `GitStatus` / `DropAt` / `ProjectIcon` / `ProjectAsset` / `ProjectAction` / `ProjectRowKind` / `ProjectLabels` / `ProjectIconRequest` | 数据模型 / 枚举 | 列表与项目数据 |
| `ROW_LIMIT` / `PROJECT_ACTIONS` / `row_menu_entries` | 常量 / 函数 | 项目树常量与右键菜单项 |

```rust
List::new().divided(true).child(ListItem::new("row-1", "按钮").sub("八种变体"));
SelectableList::new("files").multiple(true).child(/* … */);
Tree::new("tree", nodes);
```

---

## 26. `src/mail/` — 邮件

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `MailboxList` | 组件 | 邮箱/文件夹/标签列表（未读计数突出） |
| `MailList` / `MailItem` | 组件 | 邮件列表（多选）/ 邮件行（未读圆点、主题、摘要、时间、附件回形针、星标） |
| `MailReader` | 组件 | 读信（主题、发件人、收件人、时间、正文、附件、回复操作） |
| `MailThreadView` | 组件 | 会话视图（最新展开，其余折叠成一行） |
| `MailComposer` | 组件 | 写信（To/Cc/Bcc 建议、主题、正文、附件、签名、定时发送、丢弃） |
| `LabelPicker` | 组件 | 标签选择（可按名称创建） |
| `ScheduleSend` / `SnoozePicker` | 组件 | 定时发送 / 稍后处理（预设时间或自选） |
| `SignatureEditor` | 组件 | 签名编辑（带预览） |
| `QuotedText` | 组件 | 折叠的引用文本 |
| `Mailbox` / `Mail` / `Draft` / `Message` / `Contact` / `Label` | 数据模型 | 邮件数据 |

```rust
MailList::new("inbox", mails).on_select(|mail, _, cx| {});
MailComposer::new("compose").on_send(|draft, _, cx| {});
```

---

## 27. `src/maps/` — 地图

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `MapView` | 组件 | 瓦片地图（宿主供瓦片）、标记与气泡、拖拽平移、Cmd/Ctrl+滚轮缩放 |
| `WorldMap` | 组件 | 世界地图（按 ISO 3166 alpha-2 上色 + 图例） |
| `ChoroplethMap` / `ChoroplethLayer` | 组件 | 分级着色地图 / 着色图层 |
| `LocationPicker` | 组件 | 地图选点（十字准星 + 坐标显示 + 复制） |
| `MapMarker` / `MapPopup` | 组件 | 地图标记（键控，移动地图不失焦点）/ 指向某点的卡片 |
| `RouteLine` / `GeoJsonLayer` / `GeoHeatmap` | 组件 | 路线 / GeoJSON 图层 / 热力图层 |
| `CoordinateDisplay` | 组件 | 坐标显示（等宽数字 + 复制） |
| `LatLon` / `MapViewport` / `Tile` / `Feature` / `Geometry` | 数据模型 | 地理数据 |
| `MapLayer` / `ChoroplethKey` / `CoordFormat` | 枚举 | 图层类型 / 取值方式 / 坐标格式 |
| `read_geojson` / `MAX_LAT` | 函数 / 常量 | 读取 GeoJSON / 纬度上限 |

```rust
MapView::new("map", viewport).marker(MapMarker::new("a", LatLon::new(31.23, 121.47)));
WorldMap::new("world", values);
```

---

## 28. `src/media/` — 音视频与图片

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `VideoPlayer` | 组件 | 视频播放器（悬浮控制、快捷键 Space/K/J/L/C/F） |
| `AudioPlayer` / `Playlist` | 组件 | 音频播放器（封面、进度、传输键、音量）/ 播放列表 |
| `AudioWaveform` / `AudioSpectrum` | 组件 | 波形（可拖动定位）/ 频谱 |
| `MediaControls` / `VolumeControl` / `PlaybackSpeedControl` | 组件 | 传输键 / 音量 / 倍速 |
| `Scrubber` / `VideoThumbnailStrip` | 组件 | 进度条（章节、缓冲）/ 缩略图条（可裁剪） |
| `SubtitleEditor` | 组件 | 字幕编辑（时间码、就地编辑、校验） |
| `ImageViewer` / `ImageThumbnail` / `ImageCropper` / `ImageAnnotator` / `ImageUpload` | 组件 | 看图（缩放平移旋转）/ 缩略图 / 裁剪 / 标注（框、箭头、自由笔、编号钉）/ 上传（含裁剪） |
| `CameraPreview` / `DeviceSelector` / `ScreenRecorderControls` / `MicLevelMeter` | 组件 | 相机预览 / 设备选择 / 录屏控制 / 麦克风电平 |
| `ModelViewer` | 组件 | 3D 模型查看（拖拽旋转、缩放、复位） |
| `Device` / `Track` / `Cue` / `Crop` / `Orbit` / `Mark` / `Stroke` | 数据模型 | 媒体数据 |
| `DeviceKind` / `Repeat` / `Recording` / `Tool` / `SPEEDS` | 枚举 / 常量 | 设备类型 / 循环模式 / 录制状态 / 标注工具 / 倍速档位 |

```rust
VideoPlayer::new("video", frames).on_play(|_, _, cx| {});
ImageViewer::new("viewer", source);
MediaControls::new("ctl", playing).on_pause(|_, _, cx| {});
```

---

## 29. `src/menus/` — 菜单（本轮重点域）

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `MenuBar` | 组件 | 窗口内自绘的应用菜单栏（给没有系统菜单栏的平台/布局用） |
| `DropdownMenu` | 组件 | 按钮 + 其下弹出的菜单 |
| `TriggerMenu` | 组件 | 调用方自画触发器（可受控开合，适合菜单栏这种"一排触发器"） |
| `SplitButton` | 组件 | 主操作 + 箭头里的相关操作 |
| `OverflowMenu` | 组件 | 图标按钮 + "放不下的操作"菜单 |
| `ContextMenu` | 组件 | 子元素 + 右键在其上开菜单（可 `manual` 程序化开启） |
| `SearchableMenu` | 组件 | 顶部带过滤框的菜单（输入即过滤并高亮命中字母） |
| `PieMenu` / `PieItem` | 组件 | 环形菜单（右键在指针处打开，按方向选中） |
| `Menu` | 数据模型 | 菜单内容：`item` / `items` / `separator` / `group` / `width` / `max_height` |
| `MenuItem` | 数据模型 | 菜单行：`new` / `check` / `radio` / `submenu` / `icon` / `icon_element` / `keys` / `disabled` / `on_click` / `element`（整行自绘） |

```rust
let menu = Menu::new()
    .item(MenuItem::new("重命名").keys("F2").on_click(|_, cx| {}))
    .item(MenuItem::check("显示行号", true))
    .separator()
    .item(MenuItem::submenu("外观", theme_menu));

DropdownMenu::new("view", "视图", menu).icon(IconName::Eye);
TriggerMenu::new("file", menu, |open, _window, cx| trigger_face(open, cx))
    .open(Some(open))
    .on_toggle(|open, cx| { /* 库自己关掉时回调 */ });
ContextMenu::new("editor", menu).child(editor);
```

> 菜单面板的外观统一由 `forms::surface` 决定（圆角 `Radius::Lg`、`bg(colors.overlay)`、`border_1 + border`、`Elevation::Floating` 阴影），行高 `control_height(Size::Medium)`、高亮 `colors.hover`、分隔线 `colors.border`。

---

## 30. `src/messaging/` — 即时消息与通话

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `ChannelList` / `ChannelItem` / `DirectMessageItem` | 组件 | 频道列表（分组：收藏/频道/私信）/ 频道行（# 或锁、未读数）/ 私聊行（头像带在线状态） |
| `ChatMessage` | 组件 | 聊天消息（同一人五分钟内合并，时间悬停显示） |
| `ThreadPanel` / `MessageThread` | 组件 | 话题讨论面板 / 消息下的话题标记 |
| `ChannelHeader` | 组件 | 频道头（名称、主题、成员、置顶、通话） |
| `MemberList` | 组件 | 成员列表（在线/离线分组） |
| `PinnedMessages` | 组件 | 置顶消息列表（可取消置顶） |
| `CallControls` / `VoiceCallBar` / `HuddleIndicator` | 组件 | 通话控制（麦克风、摄像头、共享、离开）/ 通话条 / Huddle 指示 |
| `VideoCallGrid` / `ParticipantTile` / `ScreenShareView` | 组件 | 通话宫格 / 参会者瓦片 / 屏幕共享视图 |
| `IncomingCallDialog` | 组件 | 来电对话框（Esc 拒接） |
| `UserProfileCard` / `OnlineStatus` | 组件 | 个人名片 / 在线状态点 |
| `ReadReceipt` | 组件 | 已读回执（单勾/双勾/已读变色） |
| `GifPicker` / `StickerPicker` | 组件 | GIF 选择 / 贴纸选择（按包、可搜索） |
| `StatusSetter` / `Status` | 组件 / 数据模型 | 状态设置（表情 + 文案 + 清除时间）/ 状态 |
| `UnreadDivider` | 组件 | "New" 未读分隔线 |
| `Member` / `Gif` / `Gifs` / `Sticker` / `StickerPack` / `PinnedMessage` / `ChatRow` | 数据模型 / 枚举 | 消息数据 |
| `Delivery` / `Presence`(见 data_display) / `ClearAfter` | 枚举 | 送达状态 / 在线 / 清除时机 |

```rust
ChannelList::new("channels").section("频道", channels);
ChatMessage::new("m1", "陆井然", at).body("菜单栏的状态命名统一为 idle/hover/active-open/disabled");
VideoCallGrid::new("grid", tiles);
```

---

## 31. `src/misc/` — 杂项小工具与对话框

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Calculator` | 组件 | 计算器（键盘可输入，按优先级运算） |
| `Clock` / `WorldClock` | 组件 | 时钟（模拟表盘 + 数字）/ 世界时钟（多时区对照） |
| `Stopwatch` | 组件 | 秒表（计次、暂停后可重置） |
| `UnitConverter` | 组件 | 单位换算（长度/面积/体积/质量/温度/速度/数据，可换位） |
| `Captcha` | 组件 | 图形验证码（宿主供图与校验） |
| `QrCodeScanner` | 组件 | 二维码扫描（子线程解码，结果回调一次） |
| `ConsentDialog` / `CookieBanner` | 组件 | 同意条款对话框 / Cookie 横幅（全部接受/拒绝/自定义） |
| `ImportDialog` / `CsvImporter` / `ExportDialog` | 组件 | 导入映射（列 → 字段）/ CSV 导入 / 导出格式选择 |
| `LicenseViewer` | 组件 | 第三方许可证查看（列表 + 全文） |
| `Poll` / `Quiz` / `Survey` | 组件 | 投票 / 测验（判题与得分）/ 问卷（单选、多选、填空） |
| `Flashcards` | 组件 | 抽认卡（点击翻面） |
| `WebView` / `IframeEmbed` | 组件 | 原生 WebView（仅 macOS，绘制在 gpui 之上）/ 内嵌框架页 |
| `CaptchaState` / `ExportFormat` / `Question` / `Answer` / `QuizQuestion` / `WebSource` / `ImportField` / `Package` | 数据模型 / 枚举 | 杂项数据 |

```rust
Calculator::new("calc");
UnitConverter::new("units");
ImportDialog::new("import", fields, columns);
```

---

## 32. `src/motion/` — 动效与占位

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Transition` / `Stagger` | 组件 | 出入场（淡入、上升、滑动）/ 子元素依次入场 |
| `Flip` | 组件 | 列表顺序变化时行滑到新位置（FLIP） |
| `AnimatePresence` | 组件 | 行进出时折叠淡入淡出（`on_gone` 通知可移除） |
| `Reorder` | 组件 | 拖拽重排（被拖行跟随指针，其余让位） |
| `Skeleton` / `SkeletonText` / `SkeletonAvatar` / `SkeletonCard` / `SkeletonTable` / `Shimmer` | 组件 | 占位骨架（呼吸/扫光，reduced motion 下静止） |
| `Spinner` | 组件 | 忙指示（六种动作，reduced motion 下停在一帧） |
| `ProgressBar` / `ProgressRing` | 组件 | 进度条（缓冲、分段、未知时扫动）/ 进度环（可内嵌内容） |
| `Flash` / `Glow` / `Pulse` / `Blink` / `Shake` | 组件 | 值变化时闪一下 / 发光 / 脉冲 / 闪烁 / 左右抖动（拒绝输入的反馈） |
| `Marquee` | 组件 | 循环跑马灯 |
| `Confetti` / `ParticleBackground` / `Ripple` / `AnimatedGradient` | 组件 | 彩带 / 粒子背景 / 涟漪 / 渐变流动 |
| `LazyLoad` / `LoadingOverlay` / `RefreshIndicator` / `TypingIndicator` | 组件 | 进入视口才构建 / 加载遮罩 / 下拉刷新指示 / "正在输入" |
| `SpinnerStyle` / `Entrance` / `Refresh` | 枚举 | 动效样式 / 入场方式 / 刷新状态 |
| `curve::*` | 函数 | 缓动曲线与时长常量（如 `motion::THEME`、`motion::FAST`） |

```rust
Skeleton::new("sk").shimmer();          // 等价于 Shimmer 包裹
Spinner::new("spin").style(SpinnerStyle::Dots);
Transition::new("panel", shown).entrance(Entrance::Rise).child(panel);
```

---

## 33. `src/navigation/` — 导航与跳转

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Tabs` / `TabBar` / `Tab` / `TabPlacement` | 组件 / 数据模型 | 标签页（下划线滑动、方向键切换）/ 仅标签条 / 一条标签数据 / 标签条位置 |
| `EditorTabs` / `EditorTab` | 组件 | 文档标签（关闭、置顶、拖动重排、预览态、末尾列出全部） |
| `TabOverflowMenu` | 组件 | 标签太多时的"列出全部标签"按钮 |
| `Breadcrumb` / `Crumb` | 组件 | 面包屑（有同级时点击列出同级） |
| `NavItem` / `NavGroup` | 组件 | 侧边导航项（折叠时只显示图标）/ 可折叠分组 |
| `NavigationMenu` | 组件 | 顶部导航菜单（悬停切换面板） |
| `Pagination` / `LoadMore` | 组件 | 分页（长序列折叠成省略号）/ 触底加载更多 |
| `Steps` / `Wizard` | 组件 | 步骤条（已完成打勾可回看）/ 向导（Back/Next，最后一步变 Finish） |
| `TableOfContents` / `Sections` / `Anchor` | 组件 | 目录（跟随滚动高亮、点击带到顶部）/ 章节集合 / 章节锚点 |
| `BackForwardNavigation` | 组件 | 前进后退 + 历史列表（最新在前） |
| `GoToLine` | 组件 | 跳转到行/列（"12" 或 "12:8"） |
| `CommandPalette` / `Palette` / `Command` / `PaletteRow` | 组件 | 命令面板 / 面板模型 / 命令 / 行 |
| `QuickLauncher` / `QuickOpen` / `QuickSwitcher` / `SearchPalette` | 组件 | 快捷启动 / 快速打开文件 / 快速切换 / 搜索面板 |
| `fuzzy` / `marked` / `Group` / `OnPick` / `query_field` / `on_value` | 函数 / 类型 | 模糊匹配与命中高亮（`fuzzy` 被 `KeybindingsEditor` 复用） |

```rust
Tabs::new("tabs", selected).tab(Tab::new("all", "全部").content(list));
CommandPalette::new("cmds", commands).on_pick(|command, _, cx| {});
GoToLine::new("goto", &field, total_lines);
```

---

## 34. `src/onboarding/` — 上手引导

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `SetupChecklist` / `SetupTask` | 组件 / 数据模型 | 首次使用清单（完成数、进度分段条、逐项开始） |
| `OnboardingWizard` / `OnboardingStep` | 组件 | 首次运行向导（第 N/M 步、进度条、可跳过） |
| `FeatureHighlight` | 组件 | 新功能提示卡（带 New 徽标、试用与稍后） |
| `Hotspot` | 组件 | 带扩散圆环的新功能圆点，点击出提示 |
| `HelpPanel` / `HelpArticle` | 组件 / 数据模型 | 帮助面板（搜索 + 列表 + 正文，底部可挂"联系支持"与"键盘快捷键"）/ 帮助文章 |
| `KeyboardShortcutCheatsheet` | 组件 | **快捷键速查对话框**（内含 `settings::KeyboardShortcutsList` 与搜索，打开即聚焦） |
| `WhatsNewDialog` | 组件 | "有什么新变化"对话框（内含 `Changelog`） |
| `FeedbackWidget` | 组件 | 反馈按钮 + 简表（四个表情 + 备注） |
| `ContactSupport` / `InlineHelp` | 组件 | 联系支持（主题 + 留言）/ 行内帮助（可挂"了解更多"） |
| `Sentiment` | 枚举 | 反馈情绪 |

```rust
KeyboardShortcutCheatsheet::new("cheatsheet", shortcuts, &search);
SetupChecklist::new("setup", tasks).on_start(|task, _, cx| {});
```

---

## 35. `src/overlays/` — 浮层

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Dialog` | 组件 | 对话框卡片（标题 + 说明 + 内容 + 操作行；Esc 关闭、Tab 不出框、关闭必还焦点） |
| `AlertDialog` / `ConfirmDialog` / `PromptDialog` | 组件 | 知会 / 二次确认（破坏性操作为红）/ 单行输入（校验不通过显示原因） |
| `Popover` | 组件 | 任意内容的气泡面板（下方空间不够则向上；Esc 或点外面关闭） |
| `HoverCard` | 组件 | 悬停卡片（指针停留后打开，**不抢焦点**） |
| `FloatingToolbar` | 组件 | 文本选区上方的浮动工具条（按钮不抢走输入框焦点） |
| `Peek` | 组件 | 行内展开的预览面板（如"速览定义"） |
| `Lightbox` / `Slide` | 组件 | 图片灯箱（左右切换、Esc 关闭）/ 一张幻灯片 |
| `Spotlight` / `Tour` / `TourStep` | 组件 | 聚光灯（变暗并高亮某个框）/ 多步导览（上一步/下一步/完成/跳过） |

```rust
Dialog::new("settings").title("设置").child(body).footer(actions);
ConfirmDialog::new("del", "删除这个项目？").danger(true).on_confirm(|_, _, cx| {});
Popover::new("more", |_open, _window, cx| Button::new("btn")).child(panel);
```

---

## 36. `src/primitives/` — 原语（无样式的行为件）

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Icon` / `IconElement` / `IconName` | 组件 / 枚举 | 图标（来自 `assets/icons/`）/ 自绘图标工厂 / 图标名 |
| `Image` | 组件 | 主题化 `img()`，填满所给盒子 |
| `Divider` | 组件 | 细分割线（横向或纵向） |
| `Disclosure` | 组件 | 展开/收起的箭头 |
| `Tooltip` / `TooltipTrigger` / `HelpTooltip` | 组件 | 提示气泡（给 `.tooltip()` 用）/ 自带延迟与跟随指针 / 带问号标记的帮助提示 |
| `Backdrop` / `Place` | 组件 / 枚举 | 全窗口遮罩（子内容居中或贴边）/ 位置 |
| `FocusScope` / `FocusRing` / `FocusNext` / `FocusPrev` | 组件 | 焦点作用域（可困住 Tab）/ 焦点环 / 焦点前后移动动作 |
| `Pressable` | 组件 | 给任意内容加按下态与长按 |
| `Measure` / `IntersectionObserver` | 组件 | 尺寸变化上报 / 进入或离开视口上报 |
| `DragGhost` | 组件 | 拖拽时跟随指针的幽灵元素 |
| `Raised` / `raise` | 组件 / 函数 | 把元素画到页面之上 |
| `IconTheme` | 数据模型 | 文件图标主题（按名称/扩展名规则，可覆盖库内映射） |
| `Severity` | 枚举 | 消息严重级别（info/success/warning/danger 等，配图标与色） |

```rust
Icon::new(IconName::Check).size(IconSize::Sm).color(cx.theme().colors.success);
Tooltip::text("保存当前文件");
Measure::new("m", |bounds, _, cx| { /* 尺寸变了 */ });
```

---

## 37. `src/project/` — 项目管理

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `KanbanBoard` / `KanbanColumn` / `KanbanCard` | 组件 | 看板（跨列拖拽、键盘 Option+方向移动，每次移动回调） |
| `TaskList` / `TaskItem` | 组件 | 任务列表（方向键、Enter 打开、勾选完成）/ 任务行（优先级、编号、标题、截止、负责人） |
| `IssueCard` | 组件 | issue 卡片（状态、编号、负责人、标题、优先级、标签、截止） |
| `TaskDetailPanel` | 组件 | 任务详情面板（状态、标题就地编辑、优先级、负责人、截止、标签、备注、子步骤） |
| `DatabaseView` | 组件 | 同一批记录的五种视图：表格 / 看板 / 画廊 / 日历 / 时间线 |
| `Roadmap` / `Initiative` | 组件 / 数据模型 | 路线图（按月泳道、进度填充、今日线）/ 一个举措 |
| `MilestoneProgress` | 组件 | 里程碑进度（是否跟得上、完成占比条、截止） |
| `TimeTracker` | 组件 | 计时器（开始/停止、累计、明细与总计） |
| `PomodoroTimer` | 组件 | 番茄钟（专注/短休/长休，环形倒计时、轮次点） |
| `WorkloadView` / `Load` | 组件 / 数据模型 | 每人负载条（超出容量转红）/ 负载数据 |
| `LabelManager` | 组件 | 标签管理（改色、改名、删除、新建，名称唯一） |
| `PriorityIndicator` / `IssueIdBadge` / `StatusSelect` / `AssigneePicker` | 组件 | 优先级条 / issue 编号徽标 / 状态选择 / 负责人选择 |
| `Task` / `Person` / `IssueId` / `TimeEntry` / `Standing` | 数据模型 | 项目数据 |
| `Status` / `Priority` / `Phase` / `DatabaseLayout` | 枚举 | 状态 / 优先级 / 番茄阶段 / 视图布局 |

```rust
KanbanBoard::new("board").column(KanbanColumn::new("todo", "待办")).on_move(|card, column, index, _, cx| {});
TaskList::new("tasks", tasks).on_open(|task, _, cx| {});
DatabaseView::new("db", tasks).layout(DatabaseLayout::Board);
```

---

## 38. `src/rendering/` — 自绘表面

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `RenderSurface` | 组件 | 宿主自绘像素的画布：按盒子尺寸（设备像素）出帧，BGRA，供 gpui `RenderImage` 使用；尺寸或宿主 revision 变化时重绘 |

```rust
RenderSurface::new("gpu", ratio, revision, |size, cx| { /* 返回一帧 BGRA */ });
```

---

## 39. `src/settings/` — 设置界面

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `SettingsLayout` / `SettingsSection` / `SettingsRow` | 组件 | 设置页骨架（左侧分区列表 + 右侧分区）/ 分区 / 一行设置（名称 + 说明 + 控件，窄时控件换行） |
| `SettingsSearch` / `SettingEntry` | 组件 / 数据模型 | 设置搜索（命中即列出，Enter 跳转）/ 可被搜到的一项设置 |
| `ThemeSelector` / `AccentColorPicker` / `FontSizeControl` / `DensitySelector` | 组件 | 明暗（三档含跟随系统）/ 强调色 / 字号 / 密度 |
| `ThemeEditor` / `ThemeDraft` | 组件 / 数据模型 | 主题编辑器（密度、圆角、字号、高对比、减少动效 + 各主色）/ 草稿 |
| `ThemeImporter` | 组件 | 导入 VS Code 主题（拖入或浏览，Apply 应用） |
| `SyntaxThemePicker` | 组件 | 代码配色选择（每张卡用该配色画一行代码） |
| **`KeyboardShortcutsList`** / **`Shortcut`** | 组件 / 数据模型 | **快捷键列表（按分组、带搜索）** / **一条快捷键（分组、名称、键位字符串）** |
| `ProxySettings` / `Proxy` | 组件 / 枚举 | 代理设置（关闭/系统/手动） |
| `PrivacySettings` / `NotificationSettings` / `StartupSettings` | 组件 | 隐私 / 通知渠道 / 启动行为 |
| `StorageSettings` / `ResetToDefault` / `ImportExportSettings` | 组件 | 存储占用（各 store 占比条、可清理）/ 已改动项与恢复默认 / 导入导出 |
| `FeatureFlags` / `DeveloperModeToggle` / `Flag` / `Stage` | 组件 / 数据模型 | 特性开关（带阶段与说明）/ 开发者模式 |
| `VsCodeTheme` / `read_vscode_theme` | 数据模型 / 函数 | VS Code 主题读取 |
| `matching` | 函数 | 设置搜索命中判定 |

```rust
// 只读的快捷键总表（用户问的"设置里的快捷键"）
KeyboardShortcutsList::new(shortcuts, &search_state);

// 可改键的编辑器见 editor::KeybindingsEditor；录制框见 forms::HotkeyInput；键帽见 typography::Kbd/KbdCombo
SettingsLayout::new("settings").section(SettingsSection::new("外观").row(SettingsRow::new("主题").control(selector)));
```

---

## 40. `src/shell/` — 窗口外壳

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `TitleBar` / `WindowControls` / `drag_region` | 组件 / 函数 | 标题栏（空白处可拖窗）/ 最小化最大化关闭 / 拖动区 |
| `ActivityBar` / `ActivityItem` | 组件 | 活动栏（图标轨，主项在上、页脚项在下） |
| `Toolbar` / `ToolbarGroup` / `ToolbarSeparator` | 组件 | 工具条 / 工具组 / 分组细线 |
| `StatusBar` / `StatusBarItem` | 组件 | 状态栏（底色取 `colors.status_bar`）/ 一项状态（图标、文字、可点击） |
| `TabBar` / `WindowTab` | 组件 | 窗口级标签条（浏览器式） |
| `Sidebar`(见 layout) / `ColumnShell` / `ShellColumn` | 组件 | 多列窗口壳（每列带可拖窗的标题头） |
| `WindowSwitcher` | 组件 | 窗口切换器（遮罩上列出托管窗口，方向键/Enter） |
| `MiniWindow` / `WindowManager` / `Hosted` / `Corner` / `open_hosted` | 组件 / 函数 | 小窗（置顶、贴角）/ 窗口管理 / 单组件窗口（Esc 关闭） |
| 以组件为根开窗口：`WindowRoot` / `WindowRootExt` / `open_rooted_window` / `dismiss_hosted` / `Close` / `priority` | 组件 / 特质 / 函数 | 把一个组件当作窗口的根来打开与关闭 |
| `AboutDialog` / `SplashScreen` | 组件 | 关于窗口 / 启动闪屏 |
| `UpdateBanner` / `UpdateDialog` / `UpdateState` | 组件 / 枚举 | 更新条（下载进度、请求重启）/ 更新窗口 / 更新状态 |
| `CrashReporter` | 组件 | 崩溃后：说明、报告、是否发送 |
| `OfflineIndicator` / `ZoomControl` / `Connectivity` | 组件 / 枚举 | 离线/重连提示胶囊 / 缩放（改窗口 rem 大小）/ 网络状态 |
| `TrayIcon` / `TrayItem` / `dock_badge` / `set_dock_badge` | 组件 / 函数 | 系统托盘图标与菜单 / Dock 角标 |
| `Vibrancy` | 组件 | 毛玻璃/亚克力面板（高对比时填实） |
| `ResizeBorder` | 组件 | 自绘窗口的缩放边距、细线与阴影 |

```rust
TitleBar::new("title").child(menu_bar).child(window_controls(window, cx));
StatusBar::new("status").item(StatusBarItem::new("branch").label("main"));
ActivityBar::new("activity").item(ActivityItem::new("explorer", IconName::Folder));
```

---

## 41. `src/tables/` — 表格与电子表格

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `DataTable` | 组件 | 数据表：点表头排序（升→降→关，Shift 追加）、拖表头换列、拖列边改宽、查询与过滤规则、分页或虚拟滚动、分组折叠、行内编辑、详情行、页脚汇总、数值列着色 |
| `Table` | 组件 | 朴素表格（表头 + 单元格 + 细线，无排序分页） |
| `HeatmapTable` | 组件 | 数值网格（按极值之间取深浅） |
| `TreeTable` / `TreeRow` | 组件 | 树形表格（首列按层级缩进并带展开箭头） |
| `PivotTable` | 组件 | 透视表（行字段 × 列字段 × 聚合 + 行列总计） |
| `DataGrid` | 组件 | 可编辑网格（方向键、范围选择、回车编辑、Delete 清空、复制粘贴） |
| `Spreadsheet` | 组件 | 电子表格（字母列、数字行、公式 `=SUM(A1:A5)`、合并单元格、冻结、填充柄） |
| `TableToolbar` | 组件 | 表格工具条（搜索、过滤器计数、列显隐、密度、导出） |
| `FilterBuilder` / `SortBuilder` | 组件 | 过滤规则构建器 / 排序键构建器（可拖拽排序） |
| `Column` / `Cell` / `Row` / `Aggregate` / `Align` / `FilterRule` / `SortKey` / `Test` | 数据模型 / 枚举 | 表格数据 |
| `to_csv` | 函数 | 导出 CSV |

```rust
DataTable::new("table", columns).rows(rows).query(&query).filter(rules);
Spreadsheet::new("sheet", 100, 26).on_change(|edits, _, cx| {});
TableToolbar::new("tools").columns(columns).on_export(|_, _, cx| {});
```

---

## 42. `src/terminal/` — 终端

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Terminal` | 组件 | 终端：pty 上的程序或回放输出，逐格绘制；可输入、拖选、Cmd+点击链接 |
| `TerminalToolbar` | 组件 | 终端上方工具条（标题、程序状态、shell 选择、拆分、清屏、停止、最大化） |
| `ShellSelector` | 组件 | 新终端的 shell 选择（默认 shell + 其它在箭头里） |
| `CommandBlock` | 组件 | 命令块：在哪跑、如何结束、耗时、输出可折叠、可复制、可重跑 |
| `CommandHistory` | 组件 | 历史命令面板（最新在前、可按字母搜索、选中即重跑） |
| `LogViewer` | 组件 | 日志查看（按级别过滤、查询、跟随新行、虚拟滚动） |
| `ProcessList` | 组件 | 进程表（按占用排序、可查询、可结束所选） |
| `AnsiText` | 组件 | 按 ANSI 转义码上色的文本（其它码丢弃） |
| `Launch` | 数据模型 | 终端要跑什么（程序、参数、cwd、环境变量） |
| `ShellChoice` / `Process` / `LogLine` / `LogLevel` / `PastCommand` / `CommandState` / `TerminalEvent` / `Target` | 数据模型 / 枚举 | 终端数据 |
| `links` | 函数 | 从终端文本里识别链接 |

```rust
Terminal::new(launch, window, cx);
LogViewer::new("logs", lines).level(LogLevel::Info).follow(true);
CommandHistory::new("history", past);
```

---

## 43. `src/theme/` — 主题与设计 token（**不是组件**）

| 名称 | 类型 | 说明 |
| --- | --- | --- |
| `Theme` | 数据模型 | 全局设计 token（每 app 一份）：`colors`、`density`、`radius_scale`、`font_scale`、`font_size`、`font_family`、`mono_font_size`、`mono_family`、`reduced_motion`、`platform` |
| `Palette` | 数据模型 | 语义色板（字段见本文件 §"三个必须先知道的约定"），`Palette::light/dark(high_contrast)` |
| `Syntax` / `SyntaxTheme` / `syntax_themes` | 数据模型 / 函数 | 代码配色（关键字、字符串、注释…）/ 具名配色 / 内置配色表 |
| `Mode` / `Platform` | 枚举 | 明暗模式 / 平台约定（窗口按钮、标题栏布局、键帽写法） |
| `Density` / `Size` / `TextSize` / `Radius` / `IconSize` / `AvatarSize` / `ContainerSize` / `Elevation` | 枚举 | 尺寸与层级档位 |
| `ActiveTheme` | trait | `cx.theme()` 的来源 |
| `Sizable` | trait | 组件的尺寸档位（`xsmall/small/medium/large`） |
| `CalendarSizes` / `CanvasSizes` / `ChartSizes` / `DashboardSizes` / `FileSizes` / `GenerativeSizes` / `InteractionSizes` / `MapSizes` / `MediaSizes` / `MessagingSizes` / `MiscSizes` / `OnboardingSizes` / `ProjectSizes` / `ToolingSizes` | 数据模型 | 各域自己那一章的度量 |
| `Mix` / `HUE_NAMES` | trait / 常量 | 调色板插值（主题切换的淡入淡出）/ 图表色名 |

常用取值（`src/theme/tokens.rs`）：`control_height` = XSmall 20 / Small 24 / Medium 28 / Large 32（密度 ±4）；`text_size` = Xs 11 / Sm 12 / Base 13 / Md 14 / Lg 16 / Xl 20 / Xxl 24 / Display 32；`radius` = Sm 4 / Md 6 / Lg 8 / Xl 12（乘 `radius_scale`）；`titlebar_height` 38；`menu_width` 200；`label_width` 160。

```rust
Theme::init(cx);                                   // 启动时装一次
Theme::apply(Mode::Dark, Some(palette), cx);       // 宿主用自己的配色（如 JetBrains 主题）
Theme::set_mode(Mode::Light, cx);                  // 或只切明暗（带淡入淡出）
```

---

## 44. `src/tooling/` — 开发与自测工具

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `DesignTokenViewer` | 组件 | 把当前主题的全部 token 列出来：色板、语法色、图表色、终端色（带 hex）与字号、圆角、控件/图标/头像/容器尺寸、阴影、间距 |
| `A11yChecker` | 组件 | 对比度审计：文本色对所在表面按 4.5:1、焦点环与图表色对页面按 3:1，未达标者排前 |
| `FpsMeter` | 组件 | 帧率与最差帧（只在测量时显示，会保持窗口唤醒） |
| `RenderCounter` | 组件 | 统计所在视图渲染了多少次 |
| `EventLogger` | 组件 | 记录落在其中的输入事件（按压、移动、滚轮、按键与修饰键），最新在下 |
| `Playground` / `Knob` / `Settings` / `Setting` | 组件 | 组件试验台：左侧旋钮改属性，右侧预览实时重绘 |
| `install_inspector` | 函数 | 安装元素检查器 |

```rust
DesignTokenViewer::new();
A11yChecker::new();
Playground::new("pg", knobs).preview(|settings, cx| preview(settings, cx));
```

---

## 45. `src/typography/` — 文字与排版

| 组件 | 类型 | 说明 |
| --- | --- | --- |
| `Heading` | 组件 | 标题 H1–H6（`new(level, text)` 或 `h1()`…`h6()`） |
| `Title` / `Subtitle` / `Paragraph` / `Label` / `Caption` / `Overline` | 组件 | 预设字号字重的文本件（小眉标 Overline 在标题之上） |
| **`Kbd`** / **`KbdCombo`** | 组件 | **单个键帽**（`"secondary-s"` 在 Mac 显示 ⌘S，其它平台 Ctrl+S）/ **组合键（`"cmd-k cmd-s"` 这种和弦）** |
| `Code` / `Blockquote` / `Highlight` | 组件 | 行内代码（等宽 + 浅底）/ 引用（左侧竖线）/ 带标记范围的文本（搜索命中高亮） |
| `Link` / `ExternalLink` | 组件 | 行内链接（给 `href` 或 `on_click`）/ 站外链接（带外链箭头） |
| `Ellipsis` / `MiddleEllipsis` / `EllipsisTooltip` | 组件 | 单行截断（尾部 / 中间，路径文件名用中间）/ 截断才在悬停时显示全文 |
| `NumberText` / `CurrencyText` / `PercentText` / `FileSizeText` / `DurationText` / `PluralText` / `RelativeTime` / `DateTimeText` | 组件 | 格式化文本：数字分组 / ISO 4217 金额 / 百分比 / 字节 / 时长 / 量词 / "3 分钟前"（在屏时自动刷新）/ 时间 |
| `AnimatedNumber` | 组件 | 数字滚动动画 |
| `Emoji` | 组件 | 系统彩色字体里的一个 emoji |
| `Typewriter` / `GradientText` / `ShimmerText` | 组件 | 打字机（带闪烁光标）/ 渐变文字 / 扫光文字 |
| `Latex` / `MathError` | 组件 / 枚举 | 行内数学（TeX 子集：上下标、`\frac`、`\sqrt`、希腊字母、运算符） |
| `CopyableText` / `SelectableText` | 组件 | 带复制按钮的文本 / 可选中复制的只读文本 |
| `tabular` / `literal` | 函数 | 给元素加等宽数字 / 字面排版 |
| `DurationStyle` / `Separators` | 枚举 / 数据模型 | 时长写法 / 排版符号 |

```rust
KbdCombo::new("cmd-shift-p");              // 菜单行右侧的快捷键就是它
Kbd::new("secondary-s");
Heading::h2("按钮");
RelativeTime::new("ago", timestamp);
```

---

## 附：快捷键相关的组件在哪（用户点名的那一条链）

| 需要 | 用哪个 | 位置 |
| --- | --- | --- |
| 显示键帽 | `Kbd` / `KbdCombo` | `src/typography/inline.rs:61` / `:87` |
| 录制新键 | `HotkeyInput` | `src/forms/hotkey.rs:18` |
| 可改键的键位表 | `KeybindingsEditor` + `Keybinding` + `KeySource` + `conflicts()` | `src/editor/keybindings.rs:55` / `:26` / `:19` / `:35` |
| 只读的快捷键总表 | `KeyboardShortcutsList` + `Shortcut` | `src/settings/shortcuts.rs:23` / `:15` |
| 快捷键速查对话框 | `KeyboardShortcutCheatsheet` | `src/onboarding/dialogs.rs` |
| 按钮上带快捷键 | `Button::shortcut("secondary-s")` | `src/buttons/button.rs:357` |
| 菜单行带快捷键 | `MenuItem::keys("cmd-shift-p")` | `src/menus/model.rs:102` |

---

## 维护说明

- 本清单的组件名全部来自各模块 `mod.rs` 的 `pub use`，说明文字译自源码里的 `///` 文档注释；
- 标注为「数据模型 / 枚举」的条目不是可渲染组件，列出来是为了让你知道"这个域有哪些类型"；
- 新增或修改组件后：同步本文件对应模块表 + `docs/components/` 下对应域文件 + 画廊页面与 `stories.json`。

