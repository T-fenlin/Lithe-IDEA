# Agent 笔记：gpui 通知中心是唯一的消息落点

状态：已实现

## 先说结论

gpui 的活动栏铃铛 + 右工具窗「通知」是「事情发生过」的唯一落点：代码里的错误信息与各类
通知都写进通知中心。gpui-kit 的角落 toast 与状态栏那条 4 秒自动消失的小字都已删除。通知条目存
「稳定事件码 + 参数」而不是拼好的文案，角标是挂在已有铃铛上的一个**圆点**（红=有未读错误、
蓝=有未读其它）而不是数字。三条常驻红/绿条一根都没动。

开发者最需要记住两件事：**「已经发生过的事」走通知中心，「要用户处理的失败」走常驻红/绿条**，
这两类在本仓库刻意不合并；以及**通知文案一律走 `tr`**，条目里不许存拼好的字符串。

## 问题

### 铃铛是个空壳

右工具窗已经有铃铛图标、标题和空态「暂无通知。」，但 `gpui/crates/workbench/src/right_tool_window.rs`
的模块文档自己写着「**未做**：搜索 / 过滤 / 分组 / 详情 —— 没有通知数据源」。用户点进去只得到一个
空面板。`gpui/crates/shared/locales/lithe.zh-CN.yml` 里已经躺着 20 个 `notifications.*` 键
（搜索 / 过滤 / 复制 / 详情），全部无人引用。

### 同一个产品名下的两套通知语义

macOS 产品有完整的一套：`macos/Sources/Lithe/Application/Features/WorkbenchNotificationFeatureModel.swift`
管 4 秒显示、最多 3 条可见、100 条历史、悬停暂停，95 个 `showNotification(` 调用点。gpui 侧原本
什么都没有。macOS 那套的缺陷也已经被记下来了（`gpui/docs/archive/ui-map-macos.md:96`）：HUD 与
通知中心两套样式、HUD 所有通知同色无类型区分、通知中心一打开就 `markAllNotificationsRead()`
导致未读红点形同虚设。gpui 侧要建，就不要把这三个缺陷一起继承过来。

### 错误有四个落点，没有一个能回看

gpui 原本的用户可见消息面：角落 toast（`editor_view.rs` 两处）、状态栏临时消息（4 秒，带代数
防串，4 个调用点）、工作区配置失败红条、Git 写失败红条 / 绿条。

前两个是「事情发生了」，说完就没了，用户事后无从查证；后两个是「事情失败了要你处理」。
`workspace.rs` 的字段文档写死了后两个**刻意不是** toast：「不用 toast，因为它比红条更容易被
用户错过」。这两类必须分开，不能一起塞进一个自动消失的面板。

### 已落地决策给 store 划了一条边界

`.agents/notes/implemented/bug-fix/2026-09-27-exclude-write-must-be-read-back.md:73-77`
否决「用通知报失败」时给了一条技术理由：异步结果回来时只保证**外壳实体**还活着（换根会重建
外壳），而对话框与通知要求窗口与 `Root` 就位。通知中心由 `ShellWorkspace` 渲染，所以同一条
理由对 store 直接成立。

## 决策

### 落点唯一，临时面全废

任何「事情发生了」都写进通知中心。gpui-kit 的 `component::Notification` 与
`ShellWorkspace::show_status_notice` 及其 4 个调用点、`status_notice_generation`、
`STATUS_NOTICE_MS` 常量全部删除；`editor_view.rs` 的两个 `push_notification` 调用点改道。

**即时反馈由状态栏承担**：状态栏左组第一格是**通知中心里最新那一条的文案，点它打开右工具窗
的「通知」**。它不是另一条临时提示，而是同一条数据的第二个视图 —— IDEA 的口径正是这样
（官方文档：状态栏里能看到的消息「点它会在工具窗里打开」）。所以不存在「哪条通知只进了状态栏
没进中心」的分裂。这条要求给 `StatusEntry` 加了 `on_click`（`status_bar.rs` 的模块文档第 5 条
早就预言了这个扩展点）。

### 条目模型：存事件码 + 参数，不存文案

`shared/contracts/application-boundary.md:227-228` 已经强制了这个切分：「领域层返回稳定原因，
用户可见文案归各产品展示层所有」。文案由 `gpui/crates/workbench/src/notifications.rs` 的
`render_message` 在渲染时用 `tr_args` 拼 —— 那是**唯一**拼文案的地方。

这么定还有个具体理由：gpui 一半功能还没建（`gpui/HANDOFF.md:265` 记着缺 Java 智能提示与项目
模型），存了文案就等于把文案写死在错误的抽象层上。

### 角标是圆点，未读用水位线

角标挂在**已有的**铃铛上（`workspace.rs` 的 `right_activity_items()` 用的
`idea::BELL_ICON`，不新增图标），用 gpui-kit 的 `Badge::dot()`。圆点两色沿用 IDEA 的含义：
红=有未读错误，蓝=有未读其它，无未读不画。**红优先于蓝**。

未读不用逐条 `read: bool`，用**水位线**：记一个「上次打开中心时已见的最大序号」，`seq` 超过它
即为未读。判的是**序号**而不是「数组长度」或「下标」，所以删除、去重替换、容量截断都不会让
水位线失真；而且「你正看着中心时新来的那条」一定仍是未读 —— 逐条 boolean 做不到这件事，
`ui-map-macos.md:96` 点名的「一打开就全标已读」在逐条 boolean 下会换个形式复现。

### 重复事件原地替换

调用方给一个可选去重键，命中就原地替换 + 置顶 + 次数加一，**不新增行**。旧 Windows 产品的
`notifications.store.ts:26-50` 就是这个语义。理由是旧产品有 269 个以上 `toast.*` 调用点、
大量是高频信息，逐条追加的话 100 条上限几秒就用完。

出现次数存在模型里（`Entry::occurrences`）但列表**不画**「×N」——右工具窗约 350px 宽。

### 严重性分三档，`cancelled` 不入中心

| 档 | Core 错误码 | 圆点 |
| --- | --- | --- |
| `error` | `invalid_request`、`workspace_not_found`、`permission_denied`、`process_start_failed`、`process_failed`、`parse_failed` | 红 |
| `warning` | `not_supported`、`runtime_missing`、`timed_out`、`unknown` | 蓝 |
| 不入中心 | `cancelled` | — |

码取自 `rust/lithe-core/src/protocol/error.rs:11-34`，**snake_case** 形态（同文件 `:6` 的
`#[serde(rename_all = "snake_case")]`），可在 `shared/src/core_client.rs:225,261` 对上。

**中心没有 `success` 档**：旧产品的角标本来就排除 success（`notifications-trigger.tsx:24`），
而成功不是事后需要回看的信息，它只该占用 `git/src/changes_view.rs:913` 那条绿条。

**认不出的码按 `warning` 处理**，方向是「不谎报严重性」：Core 以后新增码时我们不知道它要不要
用户处理，先按降级处理。

### 搜索同时搜文案、错误码和参数值

三个都搜，参数值那一路最关键 —— 路径、行号、扩展名基本只出现在参数里，文案里只有「打开失败」
四个字。稳定码按 ASCII 大小写不敏感匹配；参数值与渲染后的文案**原样**匹配、不折叠大小写
（折叠会在含中文或大小写敏感的路径上产生意外）。

### 生命周期

历史上限 **100** 条（与 macOS 的 `maximumHistoryCount = 100` 一致，也是三个已有数字里最宽松
的）。**不跨启动持久化**。换根 = 换项目，所以新外壳拿到的是一个**空 store**（水位线也是 0），
不需要显式清空。

### store 的位置

新建 `lithe-gpui-notify` crate，压在 `lithe-gpui-shared` 之上、其余所有 crate 之下。硬约束是
**依赖方向**：`workbench` 依赖全部其它 crate，而其余每个只依赖 `shared`；store 放 `workbench`
会让 `git` / `editor` / `java` 反向依赖而成环。

store **由 `ShellWorkspace` 持有一个 `Entity<Store>`**，并把**弱引用**注入给要发通知的业务实体
（当前只有 `EditorPane`）。既不放全局槽也不放 `lithe-gpui-shared`：全局槽走不通（gpui 的
`Global` 只有 `App` 上有 `try_global` / `set_global`，`gpui-pre-0.3.6/src/app.rs:2134,2164`，
而 `&mut Context<T>` 拿不到 `&mut App`）；`shared` 是无状态的 `tr()` / icons / locales，塞一个
有状态的 store 是改它的角色。

### 三条红/绿条不动

`workspace.rs` 的 `workspace_config_error`、`changes_view.rs` 的 `render_write_error`（红条）
与 `render_notice`（绿条）一根都没碰。它们是 `2026-09-27-exclude-write-must-be-read-back.md`
的产物。「要用户处理的失败」放进一个需要用户主动点开的面板里等于把它藏起来 —— 这与当初写下
那句注释的理由正好相反。

推论：中心里**全部**是「已经发生过的事」，用户事后可以回看、可以清掉。

### 面板是独立实体，且不用 gpui-kit 的 `List`

搜索框 / 筛选 / 展开都住在 `gpui/crates/workbench/src/notifications.rs` 的
`NotificationPanel` 自己身上。`ShellWorkspace` 已经 287KB，再塞 4 个视图状态字段会让「哪部分
状态归谁」更难看。

不用 `List` 的原因：`ListDelegate` 要求**所有行等高**（`gpui-component-0.6.6/src/list/delegate.rs:41`），
而本面板的行要能展开详情、展开后高度可变。改用 `v_flex().overflow_y_scrollbar()` 自己列行。

### 诊断行

记录 / 展开 / 清除 / 打开各有可 grep 的 `S1_NOTIFICATION` 诊断行，接替被删掉的
`S1_STATUS_NOTICE`。交互类改动在本仓库只能靠维护者手动验证（`gpui/docs/grill.md` D1a），
这行可 grep 的证据是唯一的机器判据，所以**必须**跟着迁移。

### 复用清单

不重新造：铃铛图标 `idea::BELL_ICON`、右工具窗外壳与 `RightToolWindowView::Notifications`、
20 个 `notifications.*` locale 键、浮层三层挂载、滚动条（`ScrollableElement::overflow_y_scrollbar`）。

### 正确做法

```rust
// 业务 crate 只依赖 lithe-gpui-notify，把 Core 的错误码原样传进去：
let Some(input) = lithe_gpui_notify::Input::for_core_code(&error.code) else {
    return; // cancelled：用户自己取消的，不进中心
};
input.param("detail", &error.message);
```

```rust
// 界面侧拼文案 —— 唯一入口是 notifications::render_message：
tr_args(entry.code().as_ref(), &args)
```

### 不要这样做

- **不要用 toast 报「需要用户处理的失败」。** 自动消失等于把它藏起来，而且 `Root` 未必就位时
  调用会失败。走常驻红条。
- **不要写整句英文文案进条目。** 领域层只返回稳定原因。写死文案会让 macOS 对齐时整份模型重做。
- **不要用逐条 `read: bool`。** 水位线是按序号判定的，逐条 boolean 解决不了「正看着中心时
  新来的那条也标已读」。
- **不要新增通知图标**，也不要复活 `windows/tauri/src/ui/toast.tsx`（224 行、零引用的死代码）。
- **不要把 `Badge` 塞进 `Button` 的 children。** `Button` 的子元素落在
  `h_flex().overflow_hidden()` 里（`button.rs:698-733`）会被裁掉；必须由 `Badge` 包住 `Button`。
- **不要给可点的 `div()` 忘了 `.id(..)`。** `on_click` 在 `StatefulInteractiveElement` 上
  （`div.rs:1300`），只有 `Stateful<Div>` 实现它（`:4074`）。

## 考虑过的备选方案

### 保留 gpui-kit 角落 toast，中心只是存档

最省事：两个 `push_notification` 调用点不用改，`render_notification_layer` 已经挂好了。

不采用的理由是它产生一个必须回答的额外问题 —— 弹了但没进中心、或进了中心但用户当时没看到，
两种不一致都要处理。`gpui/UI-MAP.md:158` 已经把这个病记成「macOS 手搓 HUD 与通知中心两套
样式」。落点唯一就绕开了它。

### 保留 `status_notice` 当轻提示

改动最小。不采用的理由是它必然重新制造分裂：同一条消息要么只进状态栏、要么两边各存一份，
用户关掉工作台就查不到了。而且保留它就等于临时面还在，只是换了个长相。

### 数字角标

旧产品的 `9+` 有真源依据，`Badge::count()` + `.max(9)` 也能一比一画出来。

不采用的理由是它要求一整套 read/unread 状态机，而那套状态机唯一的实际效果就是
`ui-map-macos.md:96` 点名的缺陷。IDEA 干脆把这套模型删了，只留一个红/蓝点。

### 逐条 boolean 标记未读

`ui-map-macos.md:96` 自己建议的方向，实现上比水位线直观。

不采用的理由是它解决不了「一打开就全标已读」—— `WorkbenchView.swift:1902-1907` 连
`.onAppear` 和 `.onChange(of: model.notifications.count)` 两处都调了 `markAllRead()`，
逐条 boolean 换个写法照样复现。

### 中心加 `success` 档

旧产品有四档。不采用的理由是成功不是事后需要回看的信息，占的是 100 条里的容量。

### 把三条红/绿条也收进中心

统一入口很诱人。不采用的理由见「三条红/绿条不动」。

### store 放 `lithe-gpui-shared` crate

少一个 crate。不采用的理由见「store 的位置」。

### store 放全局槽（活过换根）

`2026-09-27-exclude-write-must-be-read-back.md` 的技术理由会让这个方案看着更稳。

不采用有两个理由：一是 gpui 的 `Global` 只在 `App` 上，从 `&mut Context<T>` 拿不到
`&mut App`；二是「跨换根存活」本来就不是需求 —— 换根就是换项目，旧项目的通知挂到新项目上
才是坏事。新外壳拿一个空 store 恰好就是想要的语义。

### 只做样式，先不接真实数据

`gpui/docs/ui-mockup-idea.md:419` 当时的建议是「没有真事实时画个假数字，比不画更危险」。

不采用是因为本轮就是要接真实事实 —— 已有 Core 的 11 个稳定错误码和若干 gpui 自有事件码可以
接。当年那个前提（没有通知子系统）现在不成立了。

### 中心跨启动持久化

不采用的理由是 IDEA 的口径是会话内，且持久化要额外处理「上次的通知属于哪个项目」以及路径
失效，现在没有需求支撑。

### 把通知形状写进 `shared/contracts/`

给三个产品一份统一契约能防漂移。本轮不采用：macOS 侧 95 个调用点与 4 秒/3 条/100 条的取值都在
自己的 feature model 里，这一轮不改 macOS，写出来的契约只有一个实现。

## 后果

- **收益**：错误与通知有唯一落点且可回看；角标不花一整套 read/unread 状态机就表达严重性；
  文案归展示层，macOS 以后对齐不用重做模型；通知数据不再被临时面切成四份。
- **代价**：状态栏左组第一格在有未读时被通知占住（项目名与分支被挤到第二、三格）；去重键
  复用率不高时 100 条上限可能被高频信息占满。
- **需要重新评估的触发条件**：① 通知量开始挤掉有用信息时，给高频事件补去重键或降级成不写
  中心；② 某个功能真的需要「跳到出错的地方」时，`lithe_gpui_notify::Target` 那个占位字段可以
  填上真实目的地；③ macOS 开始对齐时，本篇的模型是它的目标形状。
- **已知缺口**：`Input::for_core_code` 与 10 条 `notifications.core.*` 文案都已实现、已本地化、
  已单测，但**没有任何调用点把 Core 错误写进中心** —— gpui 现有的每一个 Core 失败都有一块
  刻意的失败面（红条或 `S1_*` 诊断行），把它们改成通知是逐个 feature 的决定，不该混在这一批里。
  所以那 10 个键目前「已注册但无调用点」，这是有意留着的。

## 验证

- `cargo test -p lithe-gpui-notify` —— 26 passed。含 11 个 Core 码逐个断言分档、
  `cancelled` 断言不入中心、同 key 原地替换、容量截断、水位线（`first_record_is_unread` 守着
  「第一条通知必须是未读」那个 off-by-one）、清除时水位线一起归零。
- `cargo test -p lithe-gpui-workbench notifications` —— 5 passed（筛选放行判据、四个元素 id
  互不相同、相对时间四档边界、时间戳在未来时不崩）。
- `cargo test -p lithe-gpui-shared i18n` —— 4 passed，含
  `every_wired_key_resolves_in_both_locales`：新增的 20 个键（10 个 Core 码 + 10 个面板键）
  逐个断言两个 locale 都解析得出且中文值一致。
- `cargo build --bin Lithe` —— 通过，无新增 warning。
- `node scripts/verify-agent-notes.mjs` —— 通过。
- ⚠️ `cargo test -p lithe-gpui-workbench` / `-p lithe-gpui-shared` / `-p lithe-gpui-editor` 里有
  50 个测试在本机失败，**全部**是 `std::env::temp_dir()` 不可写（`os error 5`）导致的，与本次
  改动无关：失败点在 `session.rs:324`、`shared/src/workspace_config/*`、`editor/src/buffer.rs`
  这些本次没碰的文件里。这是本仓库已记录的环境限制（`gpui/HANDOFF.md:237-238`：「`%TEMP%`
  与仓库外目录对**应用进程也**不可写」）。
- **未做机器验证**：角点显隐、筛选、搜索、展开详情、状态栏那格的点击，全部属交互类，本仓库
  只能靠维护者手动确认（`gpui/docs/grill.md` D1a）。可 grep 的判据是 `S1_NOTIFICATION` 那几行。

## 适用范围

- `gpui/crates/notify/`（`model` / `severity` / `store` 三个模块）
- `gpui/crates/workbench/src/notifications.rs`（面板）
- `gpui/crates/workbench/src/right_tool_window.rs`
- `gpui/crates/workbench/src/activity_bar.rs`
- `gpui/crates/workbench/src/status_bar.rs`
- `gpui/crates/workbench/src/workspace.rs`
- `gpui/crates/editor/src/editor_view.rs`
- `gpui/crates/shared/src/i18n.rs`
- `gpui/tools/extract-locale.mjs`
- 交叉参考：`.agents/notes/implemented/bug-fix/2026-09-27-exclude-write-must-be-read-back.md`
- 交叉参考：`.agents/notes/implemented/architecture/2026-09-13-macos-service-composition-boundaries.md`
- 交叉参考：`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`
- 交叉参考（不改动，仅取真源）：`macos/Sources/Lithe/Application/Features/WorkbenchNotificationFeatureModel.swift`、`windows/tauri/src/features/notifications/`
