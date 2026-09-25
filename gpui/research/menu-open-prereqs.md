# 菜单/打开项目 —— B4/B5 前置侦察（§4 三条未确认项 + `open_window` 顺带）

> 对象：`gpui/research/menu-and-open-project-plan.md` §4 的三条未确认项，外加 §4-4（`App::open_window`）。
> 性质：**只读侦察**。本文是唯一被写入的文件；`gpui/crates/**`、`rust/**`、`windows/**`、`shared/**`、`.agents/**` 一行未改。
> HEAD：`bd38b66a`（`feat/gpui-shell-rewrite`）。未跑 cargo，未启动 Lithe。
>
> **路径简写**（下文 `[REG]` 指 vendored 上游源码，不在仓库里）：
> `[REG]` = `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`
> 仓库内路径一律相对仓库根。**不加标记的句子是事实（我读到的原文/行号）；标 `【推断】` 的才是我的判断。**

---

## 1. gpui-kit 0.6.6 有没有"带 checkbox 的二选一对话框"

### 结论

**有这个"能力"，但没有这个"组件"——必须自己拼。** 上游能提供的三块积木都在，缺的那块（对话框内置 checkbox）不存在：

| 需求 | 上游有没有 | 怎么做 |
| --- | --- | --- |
| 标题 | 有 | `Dialog::title(..)`（`[REG]gpui-component-0.6.6/src/dialog/dialog.rs:323`） |
| 一行说明 + checkbox | **没有内置** | 作为**自定义 child** 塞进 Dialog 的正文槽；checkbox 用 `component::checkbox::Checkbox` |
| 三个按钮 | **没有内置**（内置只有 ok/cancel 两槽） | 设 `.footer(..)`，**自己画三个 `Button`**；设了 footer 之后 `button_props` 被忽略 |

所以答案是：**能照真源做**，拼法 = `open_dialog` ＋ `.title` ＋ `.child(说明)` ＋ `.child(Checkbox)` ＋ `.footer(h_flex 三个 Button)`。**不需要**任何新组件、不需要绕开 gpui-kit。

### 证据

**（1）gpui-kit 0.6.6 只是门面，组件实现全在 gpui-component**

- `[REG]gpui-kit-0.6.6/src/lib.rs:143`：`pub use ::gpui_component as component;`（整个组件库重导出）。
- `[REG]gpui-kit-0.6.6/src/lib.rs:156-161`：`init(cx)` 转调 `gpui_component::init`。
- 所以组件路径是 `gpui_kit::component::…`；本仓库现有代码就是这么用的（`gpui/crates/settings/src/dialog.rs:151` 的 `window.open_dialog` 走 `WindowExt`）。

**（2）`Dialog`：正文槽开放、footer 可覆盖、按钮 props 只有 ok/cancel**

- `Dialog` 结构：`[REG]…/dialog/dialog.rs:256-275`；字段里有 `children: Vec<AnyElement>`（`:261`）与 `footer: Option<AnyElement>`（`:265`）。
- `impl ParentElement for Dialog`：`[REG]…/dialog/dialog.rs:460-464` → `.child(..)` / `.children(..)` 合法（`Dialog` 本身不是 `Div`，但实现了 `ParentElement`）。
- children 的落点：`[REG]…/dialog/dialog.rs:663-675` —— children 被放进一个 `flex_1 + overflow_y_scrollbar` 的 `v_flex` **正文区**（标题之下、footer 之上）。这正是"标题 + 一行说明 + checkbox"要的位置。
- footer 的落点：`[REG]…/dialog/dialog.rs:677-684`（独立一行，左右留 `paddings`）。
- **"设了 footer 就自己画按钮"是上游写死的语义**，不是我们的猜测：`[REG]…/dialog/dialog.rs:336-342` 的 `footer` 文档原文 —— *"When you set the footer, the `button_props` will be ignored, you need to render the action buttons by yourself."*
- 内置按钮只有两个槽：`DialogButtonProps`（`[REG]…/dialog/dialog.rs:25-36`）字段是 `ok_text` / `ok_variant` / `cancel_text` / `cancel_variant` / `show_cancel` / `on_ok` / `on_cancel` / `on_close`；`show_cancel` 默认 `false`（`:45`）。渲染见 `render_ok`（`:106-120`）/ `render_cancel`（`:122-136`）——**没有第三个按钮的位置**。
- `AlertDialog` 同样只有两槽：`[REG]…/dialog/alert_dialog.rs:196-203`（`show_cancel` 是 `confirm()` 的同义开关，`:96-99`），但它也有 `.footer(..)` 覆盖口（`[REG]…/dialog/alert_dialog.rs:141-149`）。→ 二选一对话框用 `Dialog` 还是 `AlertDialog` 都行，三按钮都必须走 footer。

**（3）Checkbox 存在、可用，但**上游从未把它接进 Dialog**

- 定义：`[REG]gpui-component-0.6.6/src/checkbox.rs:16-35`（`#[derive(IntoElement)] pub struct Checkbox`）。
- API：`Checkbox::new(id)`（`:39`）、`.label(..)`（`:72`）、`.checked(bool)`（`:86`）、`.on_change(|&bool, window, cx| ..)`（`:102-105`，`on_click` 是它的别名 `:92`）。
- **它是受控值**：`:96-101` 的文档原文 —— *"This is a controlled value: the owner must write the requested value and call `cx.notify()` to render it."*
- 经 gpui-kit 可达：`[REG]gpui-component-0.6.6/src/lib.rs:35` 的 `pub mod checkbox;` ＋ `[REG]gpui-kit-0.6.6/src/lib.rs:143` → `gpui_kit::component::checkbox::Checkbox`。
- **本仓库从未用过 `Checkbox`**：在 `gpui/crates` 全目录 grep `Checkbox|checkbox` **零命中**。仓库里现有的布尔控件先例是 `Switch`，而且三处都在 `settings` 视图自己的 `Render` 里（`gpui/crates/settings/src/dialog.rs:1378`、`:1931`、`:2532`），**没有一处在 `window.open_dialog` 的 builder 里**。

**（4）本仓库可照抄的"标题 + 说明 + 自定义多按钮 footer"先例**

- **首选：`gpui/crates/editor/src/editor_view.rs:1699-1766`**（`open_unsaved_dialog`）。它就是"标题（`:1710`）+ 一行灰色说明（`:1711-1716`）+ 自定义 footer 三按钮（`:1717-1764`，`h_flex().justify_end().gap_2()`，左「取消」`.on_click(|_, window, cx| window.close_dialog(cx))` `:1726-1729`、中「放弃修改」次样式 `:1730-1746`、右主操作 `.primary()` `:1747-1763`）"的完整样板。B4 那个对话框与它的差别只有两点：多一个 checkbox child、文案与按钮数（3 个而不是"取消+结果词"）。
- 其它 `open_dialog` 调用点（形态可参考，但不是三按钮样板）：`gpui/crates/settings/src/dialog.rs:151`（设置窗，`p_0` + `content` 全自绘）、`:2999`；`gpui/crates/workbench/src/branch_panel.rs:964`；`gpui/crates/workbench/src/command_palette.rs:332`；`gpui/crates/git/src/changes_view.rs:618`。

**（5）对话框 builder 没有自己的状态槽 —— 这是"不再询问"唯一的真难点**

- `WindowExt::open_dialog` 的签名：`[REG]…/gpui-component-0.6.6/src/window_ext.rs:29-32`（`F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static`）；实现在 `:140-148`（转 `Root::update` → `Root::open_dialog`）。
- `Root` 里存的是 `ActiveDialog { builder: Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog> }`（`[REG]…/gpui-component-0.6.6/src/root.rs:67-74`），**没有实体、没有状态字段**。
- 每次窗口重绘都会**重跑** builder：`[REG]…/root.rs:256-262`（`render_dialog_layer` 里 `(active_dialog.builder)(dialog, window, cx)`）。
- 【推断】因此"不再询问"的 bool **不能**放在 builder 闭包捕获的普通局部变量里（每帧重建会丢/错），只有两条路：
  - **(a) 放进窗口内某个实体**（例如 `ShellWorkspace`，或一个专为这个对话框建的小 `Entity`），builder 每帧**读**它；勾选回调里写它并 `cx.notify()`（该实体的 context）→ 窗口重绘 → `render_dialog_layer` 重跑 builder → 勾选生效。仓库已有同款口径的先例：`gpui/crates/workbench/src/command_palette.rs:355-357` 的注释原文 —— *"浮层的 builder 每帧重建，**状态必须存在 `Entity` 里**，否则每帧都会被重置"*（那里的状态是 `CommandPalette.command: Entity<CommandState>`，`:358-360`）。
  - **(b) `window.use_keyed_state`**：上游 `Checkbox::render` 自己就这么取 focus handle（`[REG]…/checkbox.rs:232-235`）。但它只解决"存得住"，`cx.notify()` 的重绘对象仍然要落到某个实体上，所以 (a) 更顺。
- 【推断】另外注意：`render_dialog_layer` 只在**窗口重绘**时才重跑 builder（`[REG]…/root.rs:290-294` 只是拼元素），所以勾选态的写回必须伴随一次能触发重绘的 `notify`。

### 对 B4 / B5 的影响

- **B4 不阻塞**：`projectOpen.*` 六个键（`gpui/crates/shared/locales/lithe.zh-CN.yml:3526-3537`，en 侧同号 `lithe.en.yml:3526-3536`）确实**零引用**（在 `gpui/crates` 全目录 grep `projectOpen\.` 只命中这两个 locale 文件本身），可以直接拿来用。
- **B4 要多写一个状态字段**（不是多写一个组件）：`ShellWorkspace`（或一个专用小实体）上加一个 `ask_where_to_open_projects: bool` / `open_folders_in_new_window: bool` 之类的窗口级状态，对话框 builder 读它。这正好与 B4 已经要做的设置键（`askWhereToOpenProjects` / `openFoldersInNewWindow`）同源 —— 【推断】把"这次勾不勾"作为**本地待写入值**，先打开成功再落盘（照 `project-open-destination.ts:112-131`），落盘后这个窗口字段就不再需要单独维护。
- **B5 无影响**：对话框没有额外单例，checkbox 状态跟着窗口实体走即可。
- 照抄清单（动手时按这个顺序读）：`editor_view.rs:1699-1766` 抄结构 → `[REG]…/checkbox.rs:39,72,86,102` 抄控件 → `[REG]…/dialog/dialog.rs:336-342` 记住"设 footer 就自绘"。

---

## 2. `Window::replace_root` 之后，旧根里的进程会不会自己关掉

### 结论

**会自己关掉，不需要在 `replace_root` 之前显式 `shutdown`。** Drop 链完整覆盖 JDTLS 与终端：

```
window.root 被替换 → 旧 gpui-component Root 被释放 → 它持有的 AnyView 视图被释放
→ ShellWorkspace 被释放 → editor: Entity<EditorPane> / terminal: Entity<TerminalPane> 被释放
→ EditorPane::drop   → JavaLanguageService::shutdown → lsp.stopServer + lsp.destroyServer + 泵停
→ TerminalPane::drop → 每个页签 Session::shutdown      → writer 关 + child.kill() + child.wait()
```

但有三件事**必须知道**（都会改变 B4 的实现细节，见下面"对 B4 的影响"）：

1. 释放**不是**发生在 `replace_root` 里面，而是在**最外层 `App::update` 结束时**的 `flush_effects` 里一次性级联完成；
2. 所以整条 shutdown 是**同步跑在 UI 线程上**的（会停顿，不是白送的）；
3. 有一个**看起来像兜底、其实不关 JVM** 的 `Session::Drop`（只停事件泵）——这是"如果 Drop 链断了一环就会留孤儿 JVM"的真正风险点，也是 B4 验收线"不留残留进程"必须实测的原因。

### 证据

**（1）`replace_root` 的语义：整根替换，不调用任何清理**

- `[REG]gpui-pre-0.3.6/src/window.rs:2241-2254`：
  - `:2250` `let view = cx.new(|cx| build_view(self, cx));`
  - `:2251` **`self.root = Some(view.clone().into());`** ← 旧值在这里被 drop（赋值语句右侧求值后覆盖左值）
  - `:2252` `self.refresh();`
  - 函数体**只有这三步**，没有任何 `shutdown` / `close` / 通知旧根的逻辑。
- 根字段类型：`[REG]gpui-pre-0.3.6/src/window.rs:1176` `pub(crate) root: Option<AnyView>`；除了它，`Window` 里没有别的 `AnyView`/`Entity` 字段（同文件 grep `AnyView` 只有 `:8` 的 import、`:1176`、`:6938` 的 inspector 局部变量、`:7319` 的 `update` 形参）。

**（2）"Entity 值什么时候真的被 drop"——到 effect 周期末尾，不是同步**

- `[REG]gpui-pre-0.3.6/src/app/entity_map.rs:360-385`（`impl Drop for AnyEntity`）：`:368` 引用计数减一；`:370-374` 当且仅当 `prev_count == 1` 时 `entity_map.dropped_entity_ids.push(self.entity_id)` —— **只是记账，值还在 `entities` 表里**。
- `[REG]gpui-pre-0.3.6/src/app/entity_map.rs:190-209`（`take_dropped`）：把记账的 id `remove` 出来，返回值由调用方 drop。
- `[REG]gpui-pre-0.3.6/src/app.rs:1850-1870`（`release_dropped_entities`）：`loop { take_dropped(); if empty break; .. }` —— **循环直到空**，所以级联释放（Root → ShellWorkspace → EditorPane → …）在**同一轮**里做完。
- `[REG]gpui-pre-0.3.6/src/app.rs:1778-1783`：`flush_effects` 的文档原文 *"Called at the end of `App::update` to complete any side effects"*，第一句就是 `release_dropped_entities()`。
- `[REG]gpui-pre-0.3.6/src/app.rs:1169-1177`（`finish_update`）：`if !self.flushing_effects && self.pending_updates == 1 { flush_effects() }` —— **只有最外层那次 `App::update`** 才 flush。
- 旁证（上游自己的用词）：`[REG]gpui-pre-0.3.6/src/app/entity_map.rs:683` 的断言文案 *"entity was recently dropped but resources are retained until the end of the effect cycle."*
- 现有产品路径就是这个机制在兜底：`[REG]gpui-pre-0.3.6/src/app.rs:1057-1067`（`App::shutdown`）= `self.windows.clear(); … self.flush_effects();` —— 退出时靠"清窗口 + 手动 flush"触发全部 Drop。

**（3）本仓库"拥有子进程"的类型的 Drop 清单（决定旧根是否会自己关）**

| 类型 | Drop | 在哪触发 | 做了什么 |
| --- | --- | --- | --- |
| `ShellWorkspace` | **没有 `impl Drop`** | —— | 靠字段 drop：`editor: Entity<EditorPane>`（`gpui/crates/workbench/src/workspace.rs:563`）、`terminal: Entity<TerminalPane>`（`:566`）。全仓库 `impl Drop for` 结果里没有它。 |
| `EditorPane` | **有**：`gpui/crates/editor/src/editor_view.rs:2417-2423` | ShellWorkspace 字段 drop | `self.java.take()` → `service.shutdown()`；`java: Option<Arc<JavaLanguageService>>`（`:296`）。理由注释在 `:2411-2416`（同步调、不能交给分离线程）。 |
| `JavaLanguageService` | 没有 `Drop`，只有 `pub fn shutdown(&self)`：`gpui/crates/java/src/service.rs:623-630` | `EditorPane::drop` | 锁 `state` → `Ready(session)` 取出 → `session.shutdown()`（`:626`）；`Idle`/`Failed` 时空操作，置 `Closed`。 |
| `java::Session` | `shutdown(mut self)` **消费型**：`gpui/crates/java/src/session.rs:382-421` | 被 `JavaLanguageService::shutdown` 消费 | `lsp.stopServer`（`:385`，信封超时 20s）→ 有界等终态（`:395-397`，`SHUTDOWN_SETTLE_TIMEOUT`）→ `lsp.destroyServer`（`:407`）→ `pump.stop()`（`:420`）。 |
| `java::Session` | `impl Drop`：`gpui/crates/java/src/session.rs:434-440` | 兜底路径（如 `wait_ready` 失败） | **只** `self.pump.stop()`（`:438`）。注释原文 `:436-437`：*"兜底：`wait_ready` 失败等路径上 `Session` 会被直接丢掉，不会走 `shutdown()`"* —— **即：它不关 JDTLS 子进程。** |
| `java::events::EventPump` | `impl Drop`：`gpui/crates/java/src/events.rs:551` | `pump.stop()` / drop | 停事件读取线程。 |
| `TerminalPane` | **有**：`gpui/crates/terminal/src/terminal_view.rs:627-638` | ShellWorkspace 字段 drop | 逐页签 `tab.session.shutdown()`（`:632`）＋ `S1_TERMINAL_DROP tabs=N`（`:635`）。 |
| `terminal::Session` | 没有 `Drop`，只有 `pub(crate) fn shutdown(&mut self)`：`gpui/crates/terminal/src/session.rs:172-181` | `TerminalPane::drop` | `writer = None` → `child.kill()` → `child.wait()`（`:174-180`；注释 `:169` 明写 `wait()` 不能省，否则留僵尸）。 |

**（4）"有没有别的强句柄吊住旧根"——查过了，没有发现**

- 3 处 `thread_local` 单例装的都是 **`WeakEntity`**，且注释里把理由写死了：`gpui/crates/workbench/src/command_palette.rs:115`（`SHELL`）、`:141`（`SHELL_FOCUS`，`FocusHandle` 不是 Entity 句柄）、`gpui/crates/workbench/src/menu_bar.rs:766`（`MENU_BAR`）、`:774`（`MENU_BAR_SHELL`）、`gpui/crates/workbench/src/project_menu.rs:813`（`PROJECT_MENU`）。原文：`menu_bar.rs:764` *"用 `WeakEntity` 而不是强引用：窗口关掉之后这里不能吊住整个视图。"*
- 唯一的强引用单例是 `gpui/crates/settings/src/identity.rs:311-313` 的 `HOST: RefCell<Option<Rc<GitIdentityHost>>>` —— 但它装的是 `'static` 闭包 + `PathBuf`（登记点 `gpui/crates/workbench/src/workspace.rs:784-785` 的 `workspace_root: identity_root.clone()`），**不含任何 Entity 句柄**（全仓库 `Entity<ShellWorkspace>` / `Entity<EditorPane>` / `Entity<TerminalPane>` 只出现在 `workspace.rs:553-629` 的字段声明里）。
- 【推断】因此旧 `Root` → `ShellWorkspace` → `EditorPane`/`TerminalPane` 的引用计数会归零，第 (2) 条的级联释放成立。**这一条我没有实机验证**（不许启动 Lithe），所以 B4 的验收线"旧实例的 JDTLS/终端不留残留进程"仍然必须用 `S1_JAVA_SESSION stopped` / `S1_TERMINAL_DROP` 日志实测。

**（5）窗口级浮层：换根后不残留（连你想留的也留不住）**

- 设置窗走 `window.open_dialog`：`gpui/crates/settings/src/dialog.rs:151`；"已经开着对话框就不叠第二层"的保护在 `open_settings_dialog_at` 的 `:135`（`if window.has_active_dialog(cx) { return; }`，函数定义 `:134`；入口 `open_settings_dialog` 在 `:105`）。
  - **顺带订正一处错注释**：`gpui/crates/workbench/src/command_palette.rs:317` 把这个保护写成"`settings/src/dialog.rs:91-94`"，但 `:91-94` 实际是 `App::on_action` 焦点理由的注释，保护在 `:135`。这与 B1 里"订正错误注释"是同一类问题。
- 对话框存在 **`Root` 实体自己身上**：`[REG]…/root.rs:41` `active_dialogs: Vec<ActiveDialog>`，写入 `Root::open_dialog`（`[REG]…/root.rs:297-…`），读取只在 `render_dialog_layer`（`[REG]…/root.rs:242-295`，`:246` 用 `window.root::<Root>()` 取根）。
- 对话框层**由 `ShellWorkspace` 渲染**（不是 `Root::render`）：`gpui/crates/workbench/src/workspace.rs:1645`（`Root::render_dialog_layer(window, cx)`）与 `:1647`（notification 层）。`[REG]…/root.rs:580-612` 的 `Root::render` 只画 `self.view` + 三个 overlay，**不含 dialog layer**。
- 新 `Root` 是全新实体，`active_dialogs` 初始化为空：`[REG]…/root.rs:104-108`（`active_dialogs: Vec::new()`）。

### 对 B4 / B5 的影响

1. **不需要写"先 shutdown 再换根"的代码**（可以省掉一段最危险的顺序逻辑）。要写的只有 `window.replace_root(cx, |window, cx| { … Root::new(ws, window, cx) })` 本身（形态照 `gpui/crates/app/src/main.rs:709`）。
2. **副作用（必须写进 B4 的实现/验收）**：
   - shutdown 跑在**最外层 `App::update` 末尾**、UI 线程上，且在 `Root::new` 之后、下一帧之前。`java/session.rs:387` 的信封超时 20s、`:397` 的终态等待是上界（`editor_view.rs:2415` 的注释说实测 <1s）。**换根要预期一次同步停顿**，日志里应当能同时看到 `S1_JAVA_SESSION stopped` 与 `S1_TERMINAL_DROP`。
   - 【推断】把 `replace_root` 放在**旧 `ShellWorkspace` 自己的回调**（`cx.listener`）里是安全的：`finish_update` 只在最外层、且 lease 已归还（`[REG]…/app/entity_map.rs:169-171,225-227` 的 lease 出/归位 vs `[REG]…/app.rs:1170-1177` 的 flush 时机）之后才 flush。但**不要**在 `render` 阶段调（`Root::render_dialog_layer` 正在遍历 `active_dialogs`；`[REG]…/root.rs:248` 克隆的是快照，风险不在这一句，而在"换根会把正在画的层次换掉"）。
   - **换项目对话框自己会被换掉**：对话框属于旧 `Root` 的 `active_dialogs`（见上），而 dialog 层由 `ShellWorkspace` 画（`workspace.rs:1645`）。【推断】所以顺序必须是"点「此窗口」→ 先 `window.close_dialog(cx)` 或先让回调返回 → 再 `replace_root`"，否则替换后新 `Root` 里没有那一条 dialog，视觉上对话框会直接消失（功能上无害，观感上是"点了按钮闪一下"）。
3. **小泄漏（不是进程泄漏，值得记一句）**：`set_shell_focus(workspace.focus.clone())`（`workspace.rs:1033` → `command_palette.rs:145-147`）往 `SHELL_FOCUS` 存了一份**强** `FocusHandle`；App 只在 refcount == 0 时清 focus（`[REG]…/app.rs:1873-1893`）。新窗口的 `ShellWorkspace::new` 会覆盖这个槽（`workspace.rs:1031,1033`），所以不存在"新窗口读到旧句柄"；但旧句柄在覆盖前不会被回收。与 JDTLS/终端无关。
4. **B5 需要的证据**：本文第 (4) 条只做到"代码路径上没有强句柄"，**没有实测**。B5 拆单例时如果把某个 `WeakEntity` 手滑改成强引用（或把 `Rc<GitIdentityHost>` 里塞进 Entity），会让 `replace_root` 与关窗都不再释放旧树 —— 这是 B5 必须专门测的回归点。

---

## 3. `git.write` 的 `clone` 在"目标目录还不存在"时 `root` 传什么

### 结论

- **`destination`** = 目标路径，**允许不存在**（Core 直接交给 `git clone`，由 git 建目录）。
- **`root`** = `git clone` 子进程的工作目录，**必须是一个已存在的目录**（Core 会校验，不满足返回 `WorkspaceNotFound` / "Workspace does not exist"）。它**不是**目标目录；目标目录由 `destination` 表达。
- **契约对 `root` 的通用语义"未定义"**：`shared/contracts/rust-core-api.md` 里 `root` 只按操作逐个描述，`clone` 这一条只写了 `remote`/`destination`（`:610-612`），**没有**写 `root` 是不是父目录、是否允许不存在。上表结论来自 **Core 实现 + Core 测试**，不是契约。
- 实现与测试给出的"最权威口径"：测试里 `root` 用的是**已存在的源仓库目录本身**（`remote` 也传它），`destination` 传一个**此前不存在的兄弟目录**（`rust/lithe-core/src/tests/git.rs:1997-2010`）。【推断】因此"目标目录还不存在"时，`root` 传**任意已存在目录**都能跑通；产品语义上最自然的是传"父目录"或"当前工作区根"。**契约没定义，别替它编。**
- 另外两条硬事实（会改变 B3 的文案口径）：`clone` 请求**没有**递归子模块 / `depth` / 分支字段；响应里**没有**克隆出来的路径 —— 调用方必须自己记住 `destination`。

### 证据

**（1）契约（`shared/contracts/rust-core-api.md`）**

- `:543-554`：`git.write` 接受 typed mutation request，必需 `operation` 值清单（`:548` 含 `clone`）；可选字段清单在 `:550-553` —— `paths, reference, referenceKind, gitReference, revision, revisions, name, message, remote, destination, mode, includeUntracked, checkout, amend, force, pushTags, expectedPush, autoStash, worktreeMode, noCheckout, expectedState`。**没有** `recursive` / `submodules` / `depth` / `branch` / 凭据类字段。
- `:556-557`：*"The core validates pathspecs, revisions, branch names, references, reset modes, stash references, and operation-specific required fields before invoking Git."* —— 校验清单里**没有** root 存在性（那是实现层面的通用校验）。
- `:588-592`：成功响应形状 = `{ arguments, output, stdout, stderr, exitCode, invocations, operationError?, stashRestore?, historyRewrite?, warnings }` —— **没有目标路径字段**。
- `:608-609`：Git 启动前的参数非法走 `invalid_request`。
- **`:610-612`（clone 唯一一条专门说明）**：*"`checkout` uses `referenceKind` values `local`, `remote`, or `tag`; **`clone` uses `remote` as its source and `destination` as its target path**."*
- `:617`：`git.pushPreview` 的写法是 *"accepts `root`, an optional complete local `gitReference` …"* —— 即 `root` 一直是"请求里的一个字段"，但契约**从未**给过它的一般定义。
- 全文唯一的"root 必须存在"式表述在别处：`:216` *"`root` must be an existing workspace directory"*（那是 `maven.testResults`）。`Envelope` 一节（`:39-77`）只有 `id`/`operationId`/`timeoutMilliseconds`/`command`/`payload`，不含 root。
- **判断**：就问题"目标目录还不存在时 root 传什么"，契约 **未定义**；契约只定义了 `remote` 与 `destination` 的角色。

**（2）Core 实现（`rust/lithe-core/src/git/mod.rs`）**

- 请求结构：`:414-477` `GitWriteRequest`。逐字核对字段：`root: String`（`:418`）、`operation`（`:420`）、`fetch_options`（`:423`）、`paths`（`:425`）、`reference`（`:427`）、`git_reference`（`:431`）、`reference_kind`（`:434`）、`revision`（`:436`）、`revisions`（`:439`）、`name`（`:441`）、`message`（`:443`）、`remote`（`:445`）、`destination`（`:447`）、`mode`（`:450`）、`include_untracked`（`:452`）、`checkout`（`:454`）、`worktree_mode`（`:457`）、`no_checkout`（`:461`）、`amend`（`:463`）、`force`（`:465`）、`push_tags`（`:468`）、`expected_push`（`:471`）、`auto_stash`（`:473`）、`expected_state`（`:476`）。→ 与契约一致：**没有子模块/深度/分支/凭据字段。**
- 入口：`:853-856` `pub fn write(request)` → `:858` `write_with_trace`。
- **root 校验在 match 之前，clone 也不例外**：`:865` `let root = validate_root(&request.root)?;`
- clone 不做仓库租约（唯一例外）：`:866-872` —— 注释原文 `:866-867` *"Every typed writer shares the same repository lease, including linked worktrees. **Clone has no existing repository whose state it could race.**"*，`:868-870` `if request.operation == "clone" { None } else { .. }`。
- `validate_root` 定义：`:3078-3088` —— `canonicalize_simplified(Path::new(raw_root))`，失败 → `CoreError::new(ErrorCode::WorkspaceNotFound, "Workspace does not exist")`（`:3079-3080`）；再 `if !root.is_dir()` → 同一个错误（`:3081-3086`）；返回规范化后的字符串。→ **root 必须是已存在的目录**；**不要求**是 git 仓库（clone 分支不碰任何仓库状态）。
- clone 分支本体：`:1239-1249`
  - `:1240` `let remote = required_text(request.remote.as_deref(), "clone source")?;`
  - `:1241` `let destination = required_text(request.destination.as_deref(), "clone destination")?;`
  - `:1242-1247` destination 以 `-` 开头或含 `\0` → `invalid_request` / "Invalid clone destination"
  - `:1248` `arguments = vec!["clone".into(), "--".into(), remote, destination];`
- 执行：`root` 作为子进程 cwd 传入 `execute_git(&root, …)`。clone 分支只设 `arguments`（`:1248`），随后落到 match 之后的**统一收尾** `:1292` `execute_git(&root, &arguments, None)`（`execute_git` 定义在同文件 `:1295-1301`；`root: &str` 是它的第一个参数，即 cwd）。→ **`root` 只决定"在哪跑 git"，与 `destination` 无关。**
- 响应结构：`:166-208` `GitCommandResponse`（`arguments/output/stdout/stderr/exit_code/invocations/operation_error/stash_restore/tag_deletion/branch_deletion/history_rewrite/warnings`）→ **没有** clone 结果路径。

**（3）Core 测试（最权威的语义证据）**

- 测试函数：`rust/lithe-core/src/tests/git.rs:1874` `git_write_executes_checkout_preflight_clone_and_validation`。
- `:1875` `let root = git_write_repository("git-write-checkout-workflows");` —— `root` 是**已存在**的临时仓库。
- `:1997-2000`：
  ```rust
  let clone = root
      .parent()
      .expect("temporary root should have a parent")
      .join(format!("lithe-core-clone-{}", std::process::id()));
  ```
  → `clone`（= `destination`）是 `root` 的**兄弟目录**，此前**不存在**。
- `:2001-2007`：`request("clone", json!({ "remote": root.to_string_lossy(), "destination": clone.to_string_lossy() }))` —— `remote` 传的也是 `root`（同一个已存在目录）。
- `:2008-2010`：`assert_eq!(clone_result["ok"], true); assert!(clone.join(".git").exists()); fs::remove_dir_all(clone)…` —— **目标目录此前不存在，clone 成功并建出来了**。
- 请求构造器（`root` 进 payload 的地方）：`:4689-4719` `fn git_write_request(root: &Path, operation, overrides)`，`:4694` `"root": root`，`:4696-4707` 其余字段默认 `null`/`false`，`:4710-4714` 用 overrides 覆盖。→ `clone` 走的也是这条，即 `root` 与 `destination` 是两个独立字段。
- 【推断】这条测试没有断言 `root` 是否必须是仓库，但 `root` 传的是仓库目录；配合 `:866-872` 的"clone 没有现有仓库状态可竞争"注释，可以说 **Core 层面 root 只要求"存在且是目录"**。

**（4）本仓库 gpui 侧的泛型调用口（`gpui/crates/git/src/changes.rs:506-522`）**

```rust
pub fn write(operation: &str, root: &str, payload: serde_json::Value) -> WriteOutcome {
    let mut request = serde_json::json!({ "root": root, "operation": operation });
    if let (Some(target), Some(extra)) = (request.as_object_mut(), payload.as_object()) {
        for (key, value) in extra { target.insert(key.clone(), value.clone()); }
    }
    let data = match execute_core("git.write", request) { … };
```
- `:506` 签名；`:507` 组 `{root, operation}`；`:508-512` 把 `payload` 的键**平铺**进同一个对象；`:514` 发 `git.write`。
- **这一层不做任何 root 校验**（不 `exists()`、不 canonicalize），信封级失败在 `:516-520`，`ok:true` 但 Git 非 0 在 `:538-545` 起判。→ 【推断】做 clone UI 时可以直接复用 `write("clone", root, json!({"remote": …, "destination": …}))`，不必也不能在 gpui 侧先建目录（建了反而破坏"destination 允许不存在"的语义，虽然 `git clone` 对已存在的**空**目录也能成功）。
- `:502-505` 的 `redact` 会剪掉 stderr 里的仓库根绝对路径 —— 【推断】clone 失败信息里带的是 `destination`，而 `redact(.., root)` 只剪 `root`，所以指向不存在目录/权限错误的路径可能原样露出。要做 clone UI 时值得复查一次（B3 现在只写占位文案，不阻塞）。

### 对 B4 / B5 的影响

- **B4 无影响**（B4 不含克隆）。
- **B3 文案口径得到确认**：`git.write` 确实已含 `clone`（`rust/lithe-core/src/git/mod.rs:1239-1249` ＋ 测试 `tests/git.rs:1997-2010`），所以"克隆仓库…"进"能力已在、缺 UI"那张表是准确的；**缺的具体是**：URL/凭据输入、目标目录选择、进度展示。其中"凭据"这一条有实现层证据支持 —— 请求里**没有任何凭据字段**（`git/mod.rs:414-477`）。**【推断】** 递归子模块与 shallow depth 也都没有字段（若真源需要，那是 Core 契约要扩，不只是 UI 工作量）。
- **真要实现时的最小调用**：`lithe_gpui_git::changes::write("clone", <已存在的目录>, json!({"remote": url, "destination": target}))`，然后**自己**记住 `target`（响应不带路径），失败要自己拼话（`WriteOutcome.failure`）。
- **B5 无影响。**

---

## 4.（顺带）`App::open_window` 造第二个窗口需要什么

### 结论

造第二个窗口在 API 层面**没有障碍**（`AsyncApp::open_window` 可重复调用），真正的工作量在"把窗口级初始化重复一遍"＋"把 6 个 `thread_local` 静态量改成按窗口路由"。最小需要的东西是四件：**`WindowOptions`、`build_root_view` 闭包、一个 `ShellWorkspace`、一个 `Root`**；外加"必须重复做的窗口级初始化"与"必须先变成窗口级可读的 `compact_menu` 取值"。

### 4.1 `App::open_window` 的签名与语义

- `[REG]gpui-pre-0.3.6/src/app.rs:1347-1351`：
  ```rust
  pub fn open_window<V: 'static + Render>(
      &mut self,
      options: crate::WindowOptions,
      build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
  ) -> anyhow::Result<WindowHandle<V>>
  ```
- 内部步骤（`[REG]…/app.rs:1352-1380`）：`:1354` 分配 `WindowId` → `:1357` `Window::new(handle, options, cx)` → `:1359` build_root_view（`&mut Window, &mut App`）→ **`:1360` `window.root.replace(root_view.into());`**（注意：`open_window` 自己装根，`build_root_view` 返回什么就成根）→ `:1361` defer 一次 `appearance_changed` → `:1366-1367` **同步 `window.draw(cx)` 一次**（注释 `:1363-1365`：Windows 上不加这一帧会崩在 `DispatchTree::root_node_id`）→ `:1369-1370` 登记 handle。
- `AsyncApp` 版本：`[REG]…/app/src/async_context.rs:192-207`（`&self`，`:203-205` 若 `quitting` 直接 `bail!("app is quitting")`，否则转 `App::open_window`）。**main.rs 现在走的就是这一条**（在 `cx.spawn` 的闭包里）。
- `WindowOptions` 定义：`[REG]…/platform.rs:2172-2243`，字段：`window_bounds`（`:2178`，`None`=继承）、`titlebar`（`:2181`）、`focus`（`:2184`）、`show`（`:2187`）、`kind`（`:2190`）、`is_movable`（`:2195`）、`app_owns_titlebar_drag`（`:2208`）、`inactive_frame_interval`（`:2213`）、`is_resizable`（`:2216`）、`is_minimizable`（`:2219`）、`display_id`（`:2223`）、`window_background`（`:2226`）、`app_id`（`:2229`）、`window_min_size`（`:2232`）、`window_decorations`（`:2236`）、`icon`（`:2239`）、`tabbing_identifier`（`:2242`）。
- **`WindowOptions` 没有 `Default`**（只有 `#[derive(Debug)]`，`[REG]…/platform.rs:2173`）→ 必须给全字段。现有代码用 `..TitleBar::window_options()` 补齐，那个函数在 `[REG]gpui-component-0.6.6/src/title_bar.rs:81`（`pub fn window_options() -> WindowOptions`）。
- **为什么必须建 `Root`**：它是 gpui-component 的浮层宿主（dialog/sheet/notification/tooltip/native-menu + 焦点归还）。`Root::new(view, window, cx)` 在 `[REG]gpui-component-0.6.6/src/root.rs:100-120`；`render_dialog_layer` 在 `[REG]…/root.rs:242-295`（靠 `window.root::<Root>()` 反查，`:246`），而**本仓库是 `ShellWorkspace` 调它的**（`gpui/crates/workbench/src/workspace.rs:1645`）。窗口第一个 view 不是 `Root` 时打开浮层会 panic —— 这条在 main.rs 里有两处写死的注释（`gpui/crates/app/src/main.rs:475-476`、`:649`）与 `window.open_dialog` 的调用点注释（`command_palette.rs:42`）。

### 4.2 本仓库现有那一次 `open_window`（`gpui/crates/app/src/main.rs:616-751`）

完整参数，逐项：

| 位置 | 内容 |
| --- | --- |
| `:614` | `let bounds = startup_window_bounds(cx);`（函数在 `:67-86`：94% 可视区居中，最小 1024×680） |
| `:616` | `cx.spawn(async move \|cx\| { … }).detach();`（`AsyncApp`，因为 `open_window` 要 `&self` 而 `application().run` 里只有 `&mut App`） |
| `:617-621` | `WindowOptions { window_bounds: Some(bounds), window_min_size: Some(size(px(1024.), px(680.))), ..TitleBar::window_options() }` —— **只有这三个字段被显式指定，其余全来自 `TitleBar::window_options()`** |
| `:623` | `cx.open_window(window_options, move \|window, cx\| { … })` |
| `:624-633` | `let workspace = cx.new(\|cx\| ShellWorkspace::new(root, compact_menu_bar, branch_panel_probe, window, cx));`（`root: PathBuf` 来自 CLI `Options.root`） |
| `:636-638` | `lithe_gpui_settings::try_store(cx)` → `store.attach_window(window, cx)`（系统外观监听，要有窗口之后才能注册） |
| `:643-707` | 四个**诊断探针**（`--open-settings` `:643-647`、`--open-palette` `:659-663`、`--menu-probe` `:667-676`、`--theme-probe` `:678-687`、`--project-menu-probe` `:689-691`、`--right-view` `:702-707`），全部 `on_next_frame` / 首帧之后 |
| `:709` | `let root_entity = cx.new(\|cx\| Root::new(workspace, window, cx));` |
| `:718-746` | `--palette-keys`：`shell_focus()` 取焦点锚点 → `root_entity.update(..→ cx.spawn_in(window, ..))` → 400ms 后 `on_next_frame` 派发按键 |
| `:747` | 返回 `root_entity` |
| `:749` | `.expect("failed to open window");` |

**"造第二个窗口要重复做哪些事"**（按 App 级 / 窗口级分）：

- **App 级，只做一次，窗口闭包外**（`main.rs` 现有的，都不用重做）：`gpui_kit::component::set_locale`（`:589`）、`gpui_kit::init(cx)`（`:590`）、`Theme::change(ThemeMode::Dark, …)`（`:595`）、`Theme::set_scrollbar_mode(..)`（`:598`）、`lithe_gpui_settings::init_store`（`:603-609`）、`watch_lithe_themes`（`:610`）、`install_actions`（`:611`）、`with_assets`（`:541`）。
- **窗口级，每个窗口都要做一次**：
  1. `cx.new(|cx| ShellWorkspace::new(root, compact_menu, probe_flags, window, cx))`（`:624-633`）。它内部还会做一串**窗口级**登记：`lithe_gpui_editor::install_actions`（`workspace.rs:662`）、`lithe_gpui_git::install_actions`（`:666`）、`install_command_palette`（`:670`）、`register_java_toolchain`（`:681`）、`EditorPane::new` + `prepare_java(root)`（`:684,688`）、资源管理器/更改/终端/底部 Git/分支面板（`:698-715,717,773,984`）、`set_git_identity_host`（`:784`）、`set_menu_bar_shell`/`set_menu_bar`（`:968,972`）、`set_project_menu`（`:978`）、`set_shell`/`set_shell_focus`（`:1031,1033`）、`window.focus(&workspace.focus, cx)`（`:1037`）。
     - 注意 `install_actions` 那几个是 `cx.bind_keys`，重复调用是否幂等**我没有验证**（`editor/git/command_palette` 三个 `install_actions`）—— 这是 B5 动手时必须实测的一条。
  2. `store.attach_window(window, cx)`（`:636-638`）。
  3. `cx.new(|cx| Root::new(workspace, window, cx))`（`:709`）并作为 `build_root_view` 的返回值。
- **必须先变成"窗口外可读"的值**：`compact_menu`。
  - 事实：它是 CLI `Options` 的字段（`main.rs:134` `compact_menu_bar: bool`），解构于 `:484`，被 `open_window` 闭包捕获（`:628`），成为 `ShellWorkspace::new` 的形参 `compact_menu: bool`（`workspace.rs:650-656` 的 `:652`），最后 `mode_for(compact_menu)`（`workspace.rs:969`）。
  - 事实：全仓库**唯一**的 `Global` 是设置存储 `SettingsHandle`（`gpui/crates/settings/src/store.rs:105` `impl Global for SettingsHandle {}`、`:171` `cx.set_global(..)`、`:109` `cx.global::<SettingsHandle>()`）。
  - 【推断】所以计划里"`compact_menu` 要先从 `main.rs` 局部变量提成 Global"是**必要且正确**的：第二个窗口的闭包不能捕获第一个窗口闭包里 `move` 走的那份局部值。可选形态二：把它连同 `branch_panel_probe` 等一起塞进一个 App 级 `Global`（照 `SettingsHandle` 的写法，`store.rs:105/171`）。

### 4.3 5 个（实为 6 个）`thread_local` 单例与最小改造点

| # | 静态量 | 声明 | 类型 / 装什么 | 写点 | 读点 | 最小改造点（一句话 ＋ 行号） |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `command_palette::SHELL` | `command_palette.rs:106-116`（`static` 在 `:115`） | `RefCell<Option<WeakEntity<ShellWorkspace>>>`，**记录当前窗口的工作台** | `set_shell` `:119-121`（由 `workspace.rs:1031` 调） | `shell()` `:124-126` → 被 `open_command_palette:322`、`CommandPalette::confirm:377`、`CommandPalette::render:398` 调用（这三处**都有 `&mut Window`**） | 把 `SHELL` 换成按 `WindowId` 索引的容器（`window.window_handle().window_id()`，上游同款取值见 `[REG]…/root.rs:119`），并把 `fn shell()`（`:124-126`）签名加上 `window: &Window`；三个读点 :322/:377/:398 与写点 `workspace.rs:1031` 跟着传窗口。 |
| 2 | `command_palette::SHELL_FOCUS` | `:128-142`（`static` 在 `:141`） | `RefCell<Option<FocusHandle>>`，**外壳根元素的兜底焦点锚点**（理由注释 `:129-140`） | `set_shell_focus` `:145-147`（由 `workspace.rs:1033` 调） | `shell_focus()` `:150-152`（被 `main.rs:720` 与 `command_palette` 内部读） | 同样按 `WindowId` 索引；额外注意这是**强** `FocusHandle`（见 §2 影响 3），换根时旧值要显式清掉。 |
| 3 | `command_palette::PENDING_FOCUS` | `:154-169`（`static` 在 `:168`） | `RefCell<bool>`，"面板刚打开、下一帧要把焦点给搜索框" | `open_command_palette` `:331`（该函数有 `&mut Window`，`:318`） | `CommandPalette::render` `:388-392`（有 `&mut Window`，`:386`） | 按 `WindowId` 索引（或直接挂到 `CommandPalette` 实体上——面板本来就是每窗口一个 `Entity`，`:328`）。**两个读/写点都有 window**，改造面最小。 |
| 4 | `menu_bar::MENU_BAR` | `menu_bar.rs:753-776`（`static` 在 `:766-767`） | `RefCell<Option<WeakEntity<MenuBar>>>` | `set_menu_bar` `:784-786`（由 `workspace.rs:972` 调） | `handle()` `:791-795`（`pub`，给 `--menu-probe`），`take_pending_runs(cx: &mut App)` `:800-802` | 按 `WindowId` 索引；**`take_pending_runs`（`:800`）当前只收 `&mut App`，必须加窗口参数**（调用方是 `ShellWorkspace::render`，有 `window`）。另外 `MenuBar::shell`（读 `MENU_BAR_SHELL`，`:529`）同改。 |
| 5 | `menu_bar::MENU_BAR_SHELL` | 同块 `:774-775` | `RefCell<Option<WeakEntity<ShellWorkspace>>>` | `set_shell` `:779-781`（由 `workspace.rs:968` 调） | `MenuBar::shell` `:529` | 与 #1 完全同形；它存在的唯一理由是"菜单栏建出来之前要能登记外壳"（注释 `:771-773`），改成窗口级后这条理由仍然成立（登记时窗口已知）。 |
| 6 | `project_menu::PROJECT_MENU` | `project_menu.rs:807-814`（`static` 在 `:813`） | `RefCell<Option<WeakEntity<ProjectMenu>>>` | `set_project_menu` `:817-819`（由 `workspace.rs:978` 调） | `handle()` `:824-828`（`pub`，给 `--project-menu-probe`） | 按 `WindowId` 索引；`handle()` 加窗口参数（它在 `main.rs` 的探针路径与 `workspace.rs` 渲染路径被调用）。 |
| （7） | `settings::identity::HOST` | `identity.rs:308-313`（`static` 在 `:312`） | `RefCell<Option<Rc<GitIdentityHost>>>` —— **强 `Rc`**，装"工作区根 + 两个 `Box<dyn Fn>`" | `set_git_identity_host` `:319-321`（由 `workspace.rs:784` 调） | `GitIdentityPage` / `host_workspace_root()`（`identity.rs:327-333` 起；消费点 `settings/src/dialog.rs:2628`） | 计划 B5 也点了它（`:311-312`）。**它不是 `WeakEntity`，但也不持有任何 Entity 句柄**（§2 证据 (4)），所以它不影响 Drop；改成窗口级的原因是**语义**：两个窗口两个工作区根时，设置页会读到后登记的那个。最小改造 = 按 `WindowId`（或按 workspace root）索引这份 host。 |

**其它窗口级全局态（顺带扫过，用于估算 B5 工作量）**

- `impl Global` 全仓库只有 1 处：`SettingsHandle`（`settings/src/store.rs:105,171`）—— App 级，本来就该全局，**不用改**。
- 进程级 `static`（非 `thread_local`）：`java/src/toolchain.rs:70` `static OVERRIDE: Mutex<Option<JavaToolchainOverride>>`（存 `PathBuf`，注释 `:20-28` 解释了为什么不能是 `thread_local`）—— App 级值，多窗口下**同一份设置值**，实际无害但登记会被覆盖；`shared/src/icons/file_icon.rs:208` `ACTIVE_THEME`、`workbench/src/right_tool_window.rs:258` `RIGHT_PANEL_EPOCH`、`terminal/src/profile.rs:172` `DETECTED` 都是进程级缓存，与"每窗口一份"无关。
- 【推断】没有发现除上表 6 个之外还藏着窗口级可变状态的地方；但**本文只覆盖 `thread_local!` 与 `Global`/`OnceLock`/`Mutex` 四种写法**，若 B5 还要排查"实体字段形式的隐式单例"，需要另做一轮（例如 `MenuBar` 的 `pending_runs` 之类是否真在实体上）。

### 对 B4 / B5 的影响

- **B4**：换根不需要动单例（新 `ShellWorkspace::new` 会覆盖全部 6 个槽，`workspace.rs:968,972,978,1031,1033` + `:784`），所以 **B4 可以在不拆单例的前提下做完**。这正好解释了为什么 Q19 的"新窗口"这一轮只给"尚未接入：缺窗口级句柄路由"。
- **B5**：`App::open_window` 本身够用；真正的清单是 4.2 的三步窗口级初始化 ＋ 4.3 表里的 6 处改写。**风险最高的一条**是 #4 `take_pending_runs(cx: &mut App)`（`menu_bar.rs:800`）—— 它是菜单动作的唯一出口，签名变化会波及 `ShellWorkspace::render`；**成本最低**的是 #3 `PENDING_FOCUS`（读/写点都已持有 `window`）。
- **B5 的两个必须实测项**（本文未验证）：① `lithe_gpui_editor/git/command_palette::install_actions` 被第二个窗口重复调用是否幂等（`workspace.rs:662,666,670`）；② 强引用手滑（见 §2 影响 4）。

---

## 5. 未确认项 + 下一步确认办法

按"会不会改变实现方案"排序。

| # | 未确认项 | 为什么重要 | 下一步怎么确认（不改产品代码） |
| --- | --- | --- | --- |
| 1 | 旧树是否真的被释放（本文只做到"代码路径上没有强句柄"） | 决定 B4 是否**必须**加显式 `shutdown`。若是，最小调用顺序是：`editor.update(cx, \|pane, _\| pane.java_shutdown())`（目前**没有**这样的公开方法，`EditorPane.java` 是私有字段 `editor_view.rs:296`）→ `terminal.update(cx, \|pane, _\| pane.shutdown_all())`（同样**不存在**：`terminal_view.rs:627-638` 的 `Drop` 是唯一入口） | 实机：在 `replace_root` 前后各打一行日志（或在 `EditorPane::drop` / `TerminalPane::drop` 已有 `S1_*` 打印上 grep），跑一次"此窗口换项目"，确认日志里出现 `S1_JAVA_SESSION stopped sessionId=…`（`java/session.rs:413`）与 `S1_TERMINAL_DROP tabs=N`（`terminal_view.rs:635`），再用 `Get-Process`＋**按命令行**筛 `-eclipse.application=org.eclipse.jdt.ls.core.id1` 的 `java.exe` 确认为 0。**注意**：不要为了验证而给这两个类型加公开 shutdown 方法——先确认 Drop 已够。 |
| 2 | clone 的 `root` 契约语义（父目录 / 目标目录 / 允许不存在） | 只影响"真做 clone UI"那一批（B3 现在是占位，不阻塞） | 契约侧：在 `shared/contracts/rust-core-api.md` 的 `git.write` 一节补一句 `root` 语义是**契约变更**，需要维护者拍板（本文不代写）。实现侧口径已由 `rust/lithe-core/src/git/mod.rs:865 + :3078-3088 + :1239-1249` 与 `tests/git.rs:1997-2010` 钉住：【推断】传父目录最自然。若要走 macOS 真源口径，需要另查真源 clone 命令的 cwd。 |
| 3 | 对话框里的 `Checkbox` 状态落在哪个实体上、`cx.notify()` 是否足以让 dialog 层重跑 builder | 决定 B4 那个对话框的实现形态（是"加一个字段"还是"建一个小实体"） | 上游语义已明确（`[REG]…/root.rs:256-262` 每次渲染重跑 builder；`[REG]…/checkbox.rs:97-101` 受控值）。可验证办法：**不启动产品**，在 `gpui/crates/workbench` 里加一个 `#[gpui_kit::test]`——但那属于改测试代码，需要 B4 的写域许可。当前阶段建议：按"字段放 `ShellWorkspace` + `cx.notify()`"实现，并在验收时用 `S1_*` 或截图证明勾选态在重绘后仍在。 |
| 4 | 对话框在 `replace_root` 时被换掉这件事，产品上怎样最不别扭 | 决定"点「此窗口」"那一刻的顺序（先 `close_dialog` 还是先 `replace_root`）与观感 | 实机截图序列：点「此窗口」后立刻截一张（看对话框是否闪一下消失）。最稳的实现顺序（【推断】）：回调里先 `window.close_dialog(cx)`，再 `window.replace_root(..)`。 |
| 5 | `install_actions`（editor / git / command_palette 三处）在第二个窗口重复调用是否幂等 | 决定 B5 要不要把这三个登记也提到 App 级（只登记一次） | 读三处 `install_actions` 的实现，确认 `cx.bind_keys` 是否去重；若不去重，需要在 B5 里改成"进程只登记一次"（`cx.bind_keys` 是 App 级键表）。**本文未读这三处实现**。 |
| 6 | 两个窗口 + 两个不同工作区根时，进程级单值状态的语义（`java/src/toolchain.rs:70` 的 `OVERRIDE`；`settings/src/identity.rs:312` 的 `HOST`） | 决定"多窗口"是"多项目"还是"同项目多视图" | 维护者口径问题，不是代码问题。若要多项目，`OVERRIDE`（JDK 覆盖）与语言服务"一个工作区一次"（`editor_view.rs:435-441` 幂等、`toolchain.rs:46-50` 注释）都需要重新设计。**建议在 B5 开工前先问清**。 |
| 7 | `gpui-pre-0.3.6` 是 vendored 上游，本文的行号在升级 gpui-kit 后会漂 | 影响本文可复用性 | 升级时按符号名重搜：`replace_root`、`release_dropped_entities`、`finish_update`、`open_window`、`render_dialog_layer`、`Checkbox::on_change`。 |

---

## 附：本次侦察用到的一行式证据索引

- gpui-kit 门面：`[REG]gpui-kit-0.6.6/src/lib.rs:143,156-161`
- Dialog / AlertDialog / Checkbox：`[REG]gpui-component-0.6.6/src/dialog/dialog.rs:25-36,256-275,336-342,460-464,663-684`；`…/dialog/alert_dialog.rs:96-99,141-149,196-203`；`…/checkbox.rs:16-35,39,72,86,97-105,232-235`；`…/root.rs:41,67-74,100-120,242-295,580-612`；`…/window_ext.rs:29-32,140-148`；`…/title_bar.rs:81`
- 本仓库对话框先例：`gpui/crates/editor/src/editor_view.rs:1699-1766`；`gpui/crates/workbench/src/command_palette.rs:314-347,355-360,385-401`；`gpui/crates/settings/src/dialog.rs:151`；`gpui/crates/workbench/src/workspace.rs:1645-1647`
- replace_root 与释放时机：`[REG]gpui-pre-0.3.6/src/window.rs:1176,2241-2254`；`[REG]…/app/entity_map.rs:190-209,360-385,683`；`[REG]…/app.rs:1778-1783,1850-1870,1169-1177,1057-1067,1873-1893`
- 子进程 Drop 链：`gpui/crates/editor/src/editor_view.rs:296,435-441,2411-2423`；`gpui/crates/java/src/service.rs:623-630,637-642`；`gpui/crates/java/src/session.rs:382-421,434-440`；`gpui/crates/java/src/events.rs:551`；`gpui/crates/terminal/src/terminal_view.rs:627-638`；`gpui/crates/terminal/src/session.rs:167-181`；`gpui/crates/workbench/src/workspace.rs:563,566,1031,1033`
- 单例：`gpui/crates/workbench/src/command_palette.rs:106-116,119-126,128-152,154-169,318-347`；`…/menu_bar.rs:529,753-802`；`…/project_menu.rs:807-828`；`gpui/crates/settings/src/identity.rs:308-333`；`gpui/crates/settings/src/store.rs:105,109,171`；`gpui/crates/java/src/toolchain.rs:20-28,70`
- clone 契约与实现：`shared/contracts/rust-core-api.md:39-77,216,543-554,588-592,608-612,617`；`rust/lithe-core/src/git/mod.rs:166-208,414-477,854-872,1239-1249,3078-3088`；`rust/lithe-core/src/tests/git.rs:1874-2010,4689-4719`；`gpui/crates/git/src/changes.rs:500-522`
- open_window：`[REG]gpui-pre-0.3.6/src/app.rs:1347-1380`；`[REG]…/app/async_context.rs:192-207`；`[REG]…/platform.rs:2172-2243`；`gpui/crates/app/src/main.rs:67-86,134,614-751`
