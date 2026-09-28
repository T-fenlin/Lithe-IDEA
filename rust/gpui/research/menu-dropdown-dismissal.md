# 菜单栏下拉「收不掉」根因侦察（只读）

> 侦察范围：`gpui/crates/workbench/src/menu_bar.rs`、仓库内既有实现（`project_menu.rs`）、
> 上游 `gpui-component-0.6.6` / `gpui-pre-0.3.6`、真源 `windows/tauri/src`、以及**既有**验收工件
> `.artifacts/p7/*.png`。本轮**没有跑 cargo、没有启动 Lithe、没有改任何产品代码**。
> 每条结论都标了【事实】（读码或读像素可复现）与【推断】（需要实机才能定）。

---

## 1. 结论速览

**根因不是一个，是两条彼此独立的缺陷同时存在：**

1. **上游的"点外部关闭"机制存在并会触发，但我们没接它的出口。**
   `PopupMenu` 自己挂了 `on_mouse_down_out` → `dismiss()` → `cx.emit(DismissEvent)`；
   `MenuBar` 从头到尾**没有订阅 `DismissEvent`**，所以 `MenuBar::selected` 永远不被清，
   面板照旧被渲染出来。
   【事实】最强证据：`gpui-component-0.6.6/src/menu/popup_menu.rs:1475`（挂监听）、
   `:1110-1117` → `:1084-1108` → `:1055-1057`（emit）、`:1407`（`EventEmitter<DismissEvent>`）；
   反证：`gpui/crates/workbench/src/menu_bar.rs` 全文 grep `DismissEvent` / `subscribe` **零命中**
   （同 crate 唯一订阅在 `project_menu.rs:733,784,949`），且 `MenuBar::close()`（`menu_bar.rs:611-620`）
   在本仓库**唯一调用点是 `:625`**（`toggle_compact`）。

2. **下拉面板的摆位写错了，面板落在标题栏那一行、把自己的触发器盖住。**
   `popup_for` 用 `deferred(anchored().anchor(Anchor::BottomLeft)…)`（`menu_bar.rs:997-1011`），
   而 `anchored()` 在 `anchor_position = None` 时取的是**锚点元素自己在布局里的 origin**
   （`gpui-pre-0.3.6/src/elements/anchored.rs:27-36,273-277`），配合 `Anchor::BottomLeft`
   会把面板的**左下角**放在触发器左上角 → 面板整体在触发器**上方** → 溢出窗口上沿被
   `snap_to_window_with_margin(8)` 夹回 `y=8`（`anchored.rs:183-187,203-205`）→ 面板压回标题栏行。
   面板 `.occlude()`（`menu_bar.rs:1008`、`popup_menu.rs:1479`）= `HitboxBehavior::BlockMouse`
   （`gpui-pre-0.3.6/src/window.rs:877-905`），`hit_test` 命中即 `break`（`:1106-1123`），
   被盖住的标题既拿不到 hover 也拿不到 `on_click`。
   【事实】这条**仓库里已经有人实测并写进注释**：`project_menu.rs:633-639`
   （"实测它把面板顶到了窗口左上角 —— 面板的 top 落在触发器顶上，把触发器整个盖住"）。
   本轮又从既有工件量到同样的数（见 §4）。

**两个子现象各对应一条**：点空白收不掉 = 缺陷 1；切到另一个菜单后**新的那个也收不掉** = 缺陷 2
（面板盖住了它自己的标题，那一下点击没进 `on_click`）。**状态机本身是对的**，不是状态机的错。

---

## 2. 现状实现链（逐段行号）

### 2.1 状态归属：`MenuBar`（`Entity`，由外壳持有）

| 事实 | 行号 |
| --- | --- |
| 状态字段 `selected: Option<usize>`（当前打开哪一项，`None` = 都收起） | `menu_bar.rs:492` |
| `compact_visible`（形态 A 整层浮动条是否展开） | `menu_bar.rs:497` |
| `pending: Vec<MenuAction>`（点过什么，外壳下一帧执行） | `menu_bar.rs:505` |
| `action_context: FocusHandle`（`PopupMenu` 收起时归还焦点） | `menu_bar.rs:508` |
| `shell: Option<WeakEntity<ShellWorkspace>>`（状态变了通知外壳重绘） | `menu_bar.rs:518` |
| 构造：`selected: None` | `menu_bar.rs:527-538`（`:532`） |
| 外壳侧持有 `menu_bar: Entity<MenuBar>`；`MenuBar::new` 与 `set_menu_bar` 在 `ShellWorkspace::new` | `workspace.rs:611,963-972` |

### 2.2 标题点击 → 状态迁移

| 事实 | 行号 |
| --- | --- |
| 顶级项是**自制可点 `div`**（不是 `Button`）：`.id(("lithe-menu-trigger", index))` | `menu_bar.rs:949-970` |
| `on_mouse_down(Left, …stop_propagation())`（只为不冒泡；菜单栏已是拖拽区兄弟，无需 `prevent_default`） | `menu_bar.rs:965` |
| `on_click` → `handle.update(cx, \|bar, cx\| bar.toggle(index, cx))` | `menu_bar.rs:966-968` |
| `toggle(index, cx)`：`resolve_menu_toggle(self.nav_state(), index)` 是唯一迁移入口 | `menu_bar.rs:582-601`（`:587`） |
| 纯函数：`Open(current) if current == index => Closed`，否则 `Open(index)` | `menu_bar.rs:461-469`（`:466`） |
| `toggle` 的 Closed 分支：清 `selected` + `compact_visible`，打 `S1_MENU_CLOSE state=toggle` | `menu_bar.rs:589-593` |
| `toggle` 的 Open 分支：置 `compact_visible = true` / `selected = Some(opened)`，打 `S1_MENU_OPEN` | `menu_bar.rs:594-598` |
| 单测（**只测纯函数**，不建窗口）：`Open(5) → Closed`、`Open(2) → Open(5)`、越界不动 | `menu_bar.rs:1249-1275` |

形态 A 的图标按钮：`Button::new("lithe-menu-bar-trigger")…on_click → toggle_compact`（`menu_bar.rs:881-891`）。

### 2.3 浮层渲染（两形态共用）

| 事实 | 行号 |
| --- | --- |
| 入口 `menu_bar(bar, window, cx)` 按形态分派 | `menu_bar.rs:821-826` |
| `render_bar(&Entity<MenuBar>, …)`：`bar.downgrade().update(cx, \|bar, cx\| menu_bar(bar, window, cx).into_any_element())`——**这里拿到的其实是 `&mut Context<MenuBar>`**，但内层签名一律写 `&mut App` | `menu_bar.rs:836-844` |
| 外壳每帧调用：`let menu = crate::menu_bar::render_bar(&self.menu_bar, window, cx);` | `workspace.rs:1655` |
| 常驻形态：24px 胶囊 + 9 个 `menu_item(..)` | `menu_bar.rs:847-870`（`:853-868`） |
| 形态 A：`Button` 图标 + 展开时的浮动胶囊（同样 `anchored()` 写法） | `menu_bar.rs:876-936`（胶囊 `:913-925`） |
| 每个顶级菜单的**外层** `div().relative()`（定位上下文；高度只有触发器 20px） | `menu_bar.rs:981-985` |
| **只有** `bar.selected == Some(index)` 时这一层才追加 `popup_for(..)` —— 面板是否出现**完全**由 `selected` 决定 | `menu_bar.rs:986-988` |
| 面板摆位：`deferred(anchored().anchor(Anchor::BottomLeft).snap_to_window_with_margin(px(8.)).child(div().occlude().top_1().child(popup))).with_priority(1)` | `menu_bar.rs:997-1011` |
| 面板**每次打开现建**（模块文档里写明这是有意为之） | `menu_bar.rs:481-485,1013-1019` |
| `build_popup`：`PopupMenu::build(window, cx, …)`；`action_context` + `min_w(240)` | `menu_bar.rs:1014-1030` |
| 动作项：`PopupMenuItem::new(..).icon(..).on_click(→ bar.push_run(action, cx))` | `menu_bar.rs:1047-1054` |
| 主题子菜单（动态项） | `menu_bar.rs:1056-1063,1074-1099` |

### 2.4 关闭路径（**这是缺口所在**）

| 路径 | 事实 | 行号 |
| --- | --- | --- |
| 点菜单项 | `push_run`：`pending.push` + 只手清 `selected = None; compact_visible = false`，**故意不调 `close()`** | `menu_bar.rs:634-644`（`:641`） |
| 点左上角图标（形态 A） | `toggle_compact` → `close("compact_toggle")` | `menu_bar.rs:623-632`（`:625`） |
| `Esc` | `PopupMenu` 自绑 `escape → Cancel`（`popup_menu.rs:24`）+ `on_action(dismiss)`（`:1474`）→ `dismiss()` → emit `DismissEvent`（`:1055-1057`） | 无订阅者 |
| 点面板外 | `PopupMenu::render` 挂 `on_mouse_down_out`（`popup_menu.rs:1475`）→ `handle_dismiss`（`:1084-1108`）→ `dismiss()`（同上） | 无订阅者 |
| `MenuBar::close(state, cx)` | 文档写"（`Esc` / 点面板外 / 点某一项之后）"（`menu_bar.rs:611`），但**全仓库唯一调用点是 `:625`** → `close()` 实际只服务"点图标收起浮动条" | `menu_bar.rs:611-620` |
| 诊断字符串 | `diagnose_close` 只可能在 `:592`（`toggle`）与 `:618`（`close()` 内）被调用 → 输出值只有 `toggle` / `compact_toggle`；`:430` 文档与 `:639` 注释里提到的 **`state=run` 不可能被打印** | `menu_bar.rs:430-433,592,618,639` |
| 外壳根元素 | `v_flex().size_full().track_focus(&self.focus)`，**没有任何 `on_mouse_down_out` / 全局 dismiss** | `workspace.rs:1908-1915` |

> 【事实】`:638-640` 的注释写着"真正的收起来自 `PopupMenu` 自己的 `dismiss` →
> `S1_MENU_CLOSE state=run`"。这条**注释描述的接线根本不存在**：既没有订阅者，也没有任何
> 代码会在 `dismiss` 之后把 `selected` 清掉。缺口的位置正是"从 `AppMenuBar` 抄结构时漏掉的那一半"
> ——对比上游 `app_menu_bar.rs:142-145`（`popup_menu` + `_subscription` 两个字段）、
> `:184-185`（`cx.subscribe_in(&popup_menu, window, Self::handle_dismiss)`）、
> `:206-218`（`handle_dismiss` → `menu_bar.on_cancel(..)`）。

---

## 3. 「点空白收不掉」的原因

**缺的是"出口"，不是"机制"**（机制在上游，且**确实会触发**）：

1. 【事实】`on_mouse_down_out` 的语义 = **Capture 阶段**、且鼠标位置**不在**该 hitbox 内
   （`gpui-pre-0.3.6/src/elements/div.rs:263-276`，判据在 `:269-272`）。
2. 【事实】gpui 派发鼠标事件时，把**当前帧登记的全部 mouse listener** 在 Capture 阶段跑一遍，
   与"光标落在谁的 hitbox 上"无关；源码注释直说这一相就是给"检测某个 Bounds 之外的事件"用的
   （`gpui-pre-0.3.6/src/window.rs:5766-5776`，注释在 `:5768-5769`）。
   → 所以点编辑器/文件树空白，`PopupMenu` 的 `on_mouse_down_out` **会**被调到。
3. 【事实】它接着走 `handle_dismiss` → `dismiss()`：清高亮、`cx.emit(DismissEvent)`、并归还焦点
   （`popup_menu.rs:1110-1117 → 1084-1108 → 1055-1072`）。
4. 【事实】**没有任何人听这个事件**：`menu_bar.rs` 全文没有 `DismissEvent` / `subscribe`
   （crate 内唯一订阅在 `project_menu.rs:733,784,949`）。
5. 【事实】面板是否被画出来只取决于 `MenuBar::selected`（`menu_bar.rs:986-988`）→ `selected`
   仍是 `Some(i)` → **下一帧照旧画面板**。

**可观察的推论**（用来和"机制根本没触发"区分开）：
- 【推断】点空白之后，`PopupMenu` 内部其实已经 dismiss（首项高亮被清、焦点被还给
  `action_context`），但**面板仍然留在屏幕上**；
- 【事实】这一条路径**一行日志都不会打**（`dismiss` 里没有 `eprintln!`，`S1_MENU_*` 全在
  `MenuBar` 侧）→ **"点空白后日志里没有新的 `S1_MENU_*`"本身就是判据**。

**顺带（同一根因）**：【推断】`Esc` 也收不掉 —— `escape → Cancel`（`popup_menu.rs:24`）走
`dismiss`（`:1474`）→ 同样没有订阅者。旧工件 `.artifacts/p7/NOTES.md:110-112` 也承认 Esc
"未做机器验证"。

**顺带（旧结论需要作废）**：`.artifacts/p7/NOTES.md:104-109` 写着"点别处收起 ✅（实机）"，
但它引的证据 `v4-a-view-open.png → v4-b-after-run.png` 是 **`--menu-probe view lithe.menu.toggleStatusBar`
把菜单项执行掉之后**的关闭 —— 那条走的是 `push_run`（`menu_bar.rs:641` 显式清 `selected`），
**不是"点空白"**。【推断】上一轮把"执行菜单项后的关闭"误读成了"点外部关闭"，这正是缺陷 1
漏到维护者手里的原因。

---

## 4. 「切到另一个菜单后，新的那个也收不掉」的原因

### 4.1 状态机没有错（先把这条排除）

- 【事实】再点同一标题的迁移是 `resolve_menu_toggle(Open(i), i) = Closed`（`menu_bar.rs:466`），
  单测已断言（`menu_bar.rs:1264-1265`）；`toggle` 是它的唯一消费者（`:587`）。
- 【事实】触发器 `on_click` 无条件调 `toggle(index)`（`menu_bar.rs:966-968`）。
- → 【事实】**读码上"再点同一标题"必须收起**。它收不掉，只能是**那一下点击没进 `on_click`**。

### 4.2 为什么没进：面板盖住了触发器（含像素实测）

**机制**（读码，逐步给行号）：

1. 【事实】`anchored()` 默认 `anchor_position = None`、`position_mode = Window`（`anchored.rs:27-36`）；
   Window 模式下锚点位置缺省取 `bounds.origin` = **anchored 元素自己在布局里的 origin**
   （`anchored.rs:273-277`）。
2. 【事实】anchored 自身是 `position: absolute` 且**没有设 inset**（`anchored.rs:111-115`），
   它的父层是 `menu_item` 的无内边距 `div().relative()`（`menu_bar.rs:982-985`）
   → 这个 origin 落在**触发器左上角**。
3. 【事实】`Anchor::BottomLeft` 把**面板的左下角**放到该 origin（`anchored.rs:39-43` + `:275`）
   → 面板在触发器**上方**（意图是"下方"，方向反了）。
4. 【事实】面板高 ≈ 一张 5 行面板的 34–41 逻辑 px，向上必然溢出窗口上沿 →
   `snap_to_window_with_margin(px(8.))` 把 top 夹到 `8`（`anchored.rs:183-187` 取边距、`:203-205` 夹 top）
   → 面板被拉回标题栏那一行，压住**自己的触发器**与右边 240 逻辑 px 内的其他触发器。
5. 【事实】面板 `.occlude()`（`menu_bar.rs:1008`）与 `PopupMenu` 根 `.occlude()`（`popup_menu.rs:1479`）
   = `HitboxBehavior::BlockMouse`（`window.rs:877-905`）；`hit_test` 从最后画的往前扫，
   命中 `BlockMouse` 立刻 `break`（`window.rs:1106-1123`，判据 `:1119-1121`）
   → 被盖住的标题 `is_hovered() == false`，其 `on_click` 的 bubble 处理器不跑。
6. 【事实】同一条已被仓库自己的注释实测记录：`project_menu.rs:633-639`
   （"本侧实测它把面板顶到了窗口左上角 —— 面板的 top 落在触发器顶上，把触发器整个盖住
   （截图里只露出徽标的 1px）"），并且 `:645-656` 给出了仓库现成的替代写法。

**像素实测**（读既有的 `.artifacts/p7/*.png`，125% DPI；未启动应用、未重新截图）：

| 事实 | 数值 | 来源 |
| --- | --- | --- |
| 菜单栏胶囊上下边框 | `y=10` / `y=39`（30 物理 = 24 逻辑） | `v1-pinned-baseline.png` 竖扫 `x=200`；与 `NOTES.md:96-99` 一致 |
| 触发器盒（20 逻辑）大致 | `y ≈ 12.5..37.5` | 由上面 + `h_5()`（`menu_bar.rs:954`）推出 |
| 无面板时 9 个标题的文字列范围 | 文件 29..63、编辑 80..114、视图 131..164、转到 182..216、终端 233..267、运行 284..318、工具 335..369、窗口 387..419、帮助 438..470 | `v1-pinned-baseline.png` 行带 `y[16,34)` 亮度扫描 |
| 「视图」面板的边界 | 左边框 `x=122`、上边框 `y=13`、右边框 `x≈425` | `v1-pinned-view-open.png` 游程扫描 `y=20` / `x=400` |
| 于是「视图」面板盖住 | 视图/转到/终端/运行/工具/窗口（`x≥122`），**只露出 文件(29..70)、编辑(80..121)、帮助** | 同上（截图里正是 `文件 编辑 … 帮助`） |
| 「文件」面板的边界 | `x 19..323`、上边框 `y=13` | `v5-file-open.png` 游程扫描 |
| 于是「文件」面板盖住 | 文件(21..70)、编辑(71..121)、视图、转到、终端、运行 —— **连它自己的标题一起** | 同上 |

**结论**（两条子现象的对应关系）：

- 【事实】「视图」开着时，面板左边界 `122` 正好落在「编辑」右边界 → **文件/编辑 露在面板外**，
  点它们能进 `on_click` → `toggle` → 切到新的（旧的收起）。这**完全解释**了维护者那句
  "只能点击其他的，如编辑才可以关闭原先的"。
- 【事实】切过去之后，新菜单的面板左边界 = **它自己的**触发器左边界 → 自己也被盖住 →
  再点它（或点空白）都收不掉 = "编辑显示的又无法收起了"。
- 【推断，与维护者原话有冲突】维护者给的序列是"点**文件** → 点**编辑**才关掉原先的"，但按上表，
  「文件」面板 `x 19..323` 把「编辑」`71..121` 也盖住了，点编辑**本应无效**。两种可能：
  (a) 维护者当时实际是在「视图」等"有左邻"的菜单上试的；(b) 点到了标题最上沿约 1px 的缝
  （面板 top `13` 与触发器 top `12.5` 之间）。→ 列入 §7，并给一条实机判据（§4.3）。

### 4.3 最简验证办法

**读码已能定**（不需要实机）：缺陷 1 = `menu_bar.rs` 没有 `DismissEvent` 订阅（grep 可复现）；
缺陷 2 = `project_menu.rs:633-639` 的实测记录 + 上表的像素测量。

**若要实机确认（3 步，30 秒）**：

1. 打开 `--menu-probe view`（等价于点「视图」），截一帧：**期望**
   `视图/转到/终端/运行/工具/窗口` 六个标题**看不见**，`文件/编辑/帮助` 看得见。
   → 对照既有工件 `v1-pinned-view-open.png` 与 `v1-pinned-baseline.png` 即可，**不必重跑**。
2. 用真鼠标点「视图」标题**正中**（即被面板盖住的位置）：**期望**日志**不出现任何**
   `S1_MENU_CLOSE`、面板不消失。再点「文件」标题：**期望**出现
   `S1_MENU_CLOSE state=toggle` + `S1_MENU_OPEN id=file`（切换成功）。
   若"点正中"也能收起 → 缺陷 2 被推翻，需要复查（说明面板没盖住标题）。
3. 点编辑器空白：**期望**面板仍在、日志无新行（`dismiss` 路径不打日志）。若面板消失 →
   说明另有关闭路径存在，本报告的缺陷 1 判断需要改。

> 日志判据补充：`S1_MENU_CLOSE` 的 `state` 目前**只可能**是 `toggle`（`menu_bar.rs:592`）
> 或 `compact_toggle`（`:625`）。若实机/修复后看到 `state=run` / `state=dismiss` / `state=escape`，
> 说明这次接线接上了（`state=run` 是 `:639` 注释预言过、但今天不可达的那条）。

---

## 5. 修复方向（最小改动）

**两条都要做**：只做 A，"再点同一标题"仍会因面板压住而失效；只做 B，"点空白 / Esc"仍收不掉。

### A. 接上 dismissal（缺陷 1；仓库里已有现成写法，照抄即可）

- 落点：`MenuBar`（`menu_bar.rs:486-519`）加两个字段，照 `project_menu.rs:720-736`：
  `popup: Option<Entity<PopupMenu>>` + `_dismiss_subscription: Option<Subscription>`
  （`project_menu.rs:733-736` 明写"必须被持有：`Subscription` 一 drop 就取消"）。
- 订阅点：面板**真的被建出来**的那一次，`cx.subscribe_in(&popup, window, MenuBar::handle_dismiss)`
  —— 抄 `project_menu.rs:946-956`（`:949`）或上游 `app_menu_bar.rs:184-185`。
- 处理函数：照 `project_menu.rs:780-792` / `app_menu_bar.rs:206-218`：
  `selected = None; compact_visible = false; popup = None; _dismiss_subscription = None;`
  再 `diagnose_close("dismiss")` + `notify_shell(cx)`。顺手把 `menu_bar.rs:611` 那句
  "（`Esc` / 点面板外 / 点某一项之后）"改成事实（今天它只服务 `:625`）。
- **借用形状要先改**：整条渲染链现在写成 `&mut App`
  （`menu_bar.rs:821,836-842,847,876,981,997,1014,1074`），而 `subscribe_in` 要
  `&mut Context<MenuBar>`。`render_bar`（`:836-844`）里 `downgrade().update(…)` 拿到的本来就是
  `&mut Context<MenuBar>`，把它透传下去即可（需要 `App` 的地方靠 deref）。
- **注意"每次打开现建"这条既有决策**（`menu_bar.rs:481-485`）：一旦持有实体与订阅，关闭时必须
  照 `project_menu.rs:766-777` 一起把 `popup` / `_dismiss_subscription` 置 `None`，
  否则旧订阅会吊住已被替换的面板。
- 另外：`build_popup` 建出面板后还需要把焦点交给它（`project_menu.rs:950-955`），
  否则 `Esc` / `↑↓` / `Enter` 的 `key_context` 不在焦点上（今天只有 `action_context` 单向归还焦点，
  `menu_bar.rs:1027-1029`）。

### B. 换面板摆位（缺陷 2；同样有现成写法）

- 落点：`popup_for`（`menu_bar.rs:997-1011`）把
  `deferred(anchored().anchor(Anchor::BottomLeft).snap_to_window_with_margin(px(8.)))`
  换成 `deferred(Positioner::side(trigger_bounds).placement(Placement::Bottom).align(Align::Start)
  .offset(px(4.)).margin(px(8.)).occlude().child(popup)).with_priority(1)`
  —— 逐字照 `project_menu.rs:645-656`。
- 需要量触发器 bounds：在 `menu_item` 的外层**无内边距包装层**上挂 `on_prepaint`
  （`project_menu.rs:971-989`；注意那层现在没有 `.id(..)`，`menu_bar.rs:982` 需要补一个），
  并把量到的值存进 `MenuBar`（`Rc<Cell<Option<Bounds<Pixels>>>>`，`project_menu.rs:726-732`）。
- 同一条也适用于形态 A 的浮动胶囊（`menu_bar.rs:913-925`）：实测
  `v1-compact-view-open.png` 里胶囊落在**标题栏那一行**，而真源是 `absolute top-full left-0 mt-1`
  （`window-menu-bar.tsx:539`）→ 这里也是同一个 `anchored()` 用法问题（另见 §7 第 5 条）。

### C. 可选（不作为本次最小修复）

悬停切换：上游 `AppMenuBar::handle_hover`（`app_menu_bar.rs:241-254`）在"已有菜单打开"时
悬停即换，对应真源 `openOnHover`（`ui/menubar.tsx:84`）。本侧目前没有这条（属于对齐真源的加项，
不是本次 bug 的一部分）。

---

## 6. 真源应有的行为（逐条对照）

真源 = `windows/tauri/src/ui/menubar.tsx`（`@base-ui/react` 的 `Menu`/`Menubar` 薄封装）
+ `windows/tauri/src/features/window/components/window-menu-bar.tsx` + `title-bar/title-bar.tsx`。
**顶级项的"打开哪一项"是受控单值**：`activeMenu: string | null`
（`window-menu-bar.tsx:30-31`、`title-bar.tsx:68`）→ `<Menubar value={activeMenu ?? ""} onValueChange={v => setActiveMenu(v || null)}>`
（`window-menu-bar.tsx:543-545`），每个 `MenubarMenu` 是 `open={value === this}`（`menubar.tsx:50-70`）。

| 行为 | 真源依据 | 是否明确 |
| --- | --- | --- |
| **点外部关闭** | 仓库代码里**没有**任何显式实现：`ui/menubar.tsx` 只是 `@base-ui/react` 的 `Menu.Portal`/`Positioner`/`Popup` 封装（`:1,103-135`），外部点击由库内部处理；`node_modules` 未安装，`@base-ui/react": "^1.6.0"`（`windows/tauri/package.json:30`）的实现本轮**读不到** | **真源未明确**（由第三方库提供）。间接可见的只有"关闭后把受控值清空"这一半：`menubar.tsx:57-66`、`window-menu-bar.tsx:47-50` |
| **`Esc` 关闭** | 同上，仓库无实现 | **真源未明确**（同上） |
| **再点同一标题收起** | `MenubarMenu.onOpenChange(false)` 且 `menubar.value === value` → `onValueChange("")`（`menubar.tsx:63-65`），配合 `Menu.Trigger` 自身"再点关闭"（base-ui 行为） | **半明确**：仓库可见"关闭后清空受控值"（`menubar.tsx:54-68` + `window-menu-bar.tsx:544-545`）；触发这一段的那次点击在库内 |
| **悬停切换** | `MenubarTrigger` 传 `openOnHover`（`menubar.tsx:84`）；`menubar.tsx:61-62`（open 时把受控值设为该项）；调研结论同一条：`gpui/research/windows/10-menu-bar.md:81`（"顶级项悬停即开（`MenubarTrigger openOnHover`）；`activeMenu` 受控"） | **明确** |
| **选中项后关闭** | 每个菜单项 `onClick` 都走 `handleClickEmit` / `handleCommand` / `handleNewWindow`，它们末尾都 `closeMenu()`（`window-menu-bar.tsx:105-129`，`closeMenu` 定义 `:47-50`）；`MenubarItem` 的 `onClick` 在 `ui/menubar.tsx:151-154` | **明确** |
| 形态 A 图标按钮 = 整层开合 | `handleCompactMenuToggle` / `handleCompactMenuClose`：同时清 `activeMenu` 与 `isCompactMenuVisible`（`title-bar.tsx:165-173`） | **明确**（本侧 `toggle_compact` 已对齐，`menu_bar.rs:623-632`） |
| 下拉面板位置 | `Menu.Positioner` `side="bottom"` / `align="start"` / `sideOffset=4` / `collisionPadding=8`（`menubar.tsx:106-110`） | **明确** → 与 §5-B 的 `Positioner::side(..).placement(Bottom).align(Start).offset(4).margin(8)` 一一对应（正是 `project_menu.rs:645-656` 的写法） |

---

## 7. 未确认项

1. **base-ui 的 dismissal 到底怎么实现**（点外部 / Esc 的库内落点）：`node_modules/@base-ui` 未安装
   （`windows/tauri` 下也没有 `node_modules`），只有 `package.json:30` 的依赖声明 →
   无法引用库内 `file:line`，也无法排除"真源在 Windows 上还有别的显式关闭代码"。
2. **面板是否在**所有**索引 / 所有窗口尺寸下都盖住自己的标题**：本轮只量了 2 个索引
   （视图、文件）、1 个窗口尺寸（1823×1024 物理、125% DPI）。面板左边 = 触发器左边这条与字体宽度
   无关，但"盖住几个邻居"随窗口宽/语言/标题宽度变化。
3. **维护者原话与我量到的几何冲突**：他说"点文件 → 再点编辑可以关掉原先的"，但「文件」面板
   `x 19..323` 覆盖了「编辑」`71..121`。需维护者实机复核一次（按 §4.3 第 2 步）。
4. **Esc 是否收不掉**：【推断】是（无订阅者）。旧工件 `NOTES.md:110-112` 明确说没有机器验证。
5. **形态 A 浮动胶囊的位置偏差**：实测 `v1-compact-view-open.png` 里胶囊落在标题栏那一行，
   真源是 `top-full mt-1`（`window-menu-bar.tsx:539`）；这是同一个 `anchored()` 用法导致的
   （`menu_bar.rs:913-925`），但本轮没有逐像素核对这一形态的期望位置，也没确认它是否单独构成
   可感知缺陷。
6. **旧结论失效的原始证据没复跑**：`.artifacts/p7/NOTES.md:104-109` 的"点别处收起 ✅（实机）"
   我只做了读码/读工件层面的反驳（它引的 `v4-a → v4-b` 是 `push_run` 之后的关闭），
   没有重跑 `inject.ps1`（本轮禁止启动 Lithe；且 `NOTES.md:191-199` 记录该工作站注入不可用）。
7. **`--menu-probe` 打开的菜单与"真点标题"是否有差别**：`open_by_id`（`menu_bar.rs:663-672`）
   先清状态再 `toggle`，与点击回调调的是同一个 `toggle`；差别只在"操作系统送不送这一下点击"，
   所以 §4.3 第 1 步的截图证据可用，但"点正中收不起来"必须靠真人点击才能取证。

---

## 附：本轮测量方法（可复现，只读）

- 像素测量只读 `.artifacts/p7/{v1-pinned-baseline,v1-pinned-view-open,v5-file-open}.png`，
  用 Pillow 做"逐行亮度扫描 + 扫描线游程编码"定位菜单栏胶囊边框（`y=10/39`）、
  面板边框（`x=122` / `x=19..323`、`y=13`）与各标题文字列范围（`y[16,34)`）；临时脚本目录已删除。
- 读码只用了 `read` / `grep` / `git log`；**没有跑 cargo、没有启动 Lithe、没有改任何产品代码**。
