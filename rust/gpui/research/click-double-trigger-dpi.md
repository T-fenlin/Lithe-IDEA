# 右活动栏「点击双触发」根因诊断（2026-09-26）

> 触发：`gpui/PLAN.md` §16.9 ②、`gpui/HANDOFF.md` §0/§2。现象原文是
> 「本机 125% DPI 下点右活动栏图标，一次点击打出 `S1_RIGHT_PANEL … visible=true` 紧跟
> `visible=false`，面板被自己关掉；光标停在图标上还会被杂散点击再切一次」。
> 本文只做**只读诊断**（源码 + 现有日志），不改任何产品代码。
>
> **只读边界**：`windows/`、`macos/`、`rust/`、`shared/`、`.agents/`、`gpui/crates/**` 全部未改；
> 本次唯一写盘是本文。
>
> **路径前缀**（下面用简写引用，全部是 cargo registry 里的**实际编译版本**，
> 由 `gpui/Cargo.lock:2488-2491` 钉在 `gpui-pre 0.3.6`）：
>
> | 简写 | 绝对路径 |
> | --- | --- |
> | `gpui-pre-0.3.6/` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-pre-0.3.6\` |
> | `gpui-pre-windows-0.3.6/` | `…\rsproxy.cn-e3de039b2554c837\gpui-pre-windows-0.3.6\` |
> | `gpui-component-0.6.6/` | `…\rsproxy.cn-e3de039b2554c837\gpui-component-0.6.6\` |
>
> `gpui_kit::component` 就是 `gpui-component` 的再导出（`gpui-kit-0.6.6/src/lib.rs` 顶部模块表），
> 所以 `activity_bar.rs` 里的 `Button` = `gpui-component-0.6.6/src/button/button.rs` 的 `Button`。

---

## 1. 结论速览

**结论：现有证据不支持「一次点击在派发层变成两次 click」这个判断。它更像是「真的有两次输入」
（环境杂散点击 / 第二次脚本点击），我们目前的诊断仪表分不清这两件事。**

最强三条证据：

1. **派发层代码事实**：`ClickEvent::Mouse` 只在 `MouseUpEvent` 的 **bubble 阶段**、且
   down 阶段写过 `pending_mouse_down`、且 up 时 hitbox 仍被 hover 三个条件同时成立时产生**恰好一次**
   （`gpui-pre-0.3.6/src/elements/div.rs:2950-2963` 写 pending；`:3077-3121` 在 up 里产生 click）。
   `WM_LBUTTONDOWN`（`gpui-pre-windows-0.3.6/src/events.rs:138,486-514`）与
   `WM_LBUTTONUP`（`:144,516-542`）各自**只**派发 `MouseDownEvent` / `MouseUpEvent`，
   两边都不产生 `ClickEvent`。**真人手点一次 = 一次 click 回调。**
2. **关键不对称**：`pending_mouse_down` 是**单个** `Option`，第一个 `MouseUp` 就把它 take 掉
   （`div.rs:3084-3098`）。所以「多一个 `WM_LBUTTONUP`」**不可能**多出一次 click ——
   要凑出第二次 click，必须**完整多一对 DOWN+UP**。这把嫌疑从「派发层重复派发」推到了「输入侧多了一对消息」。
3. **可对齐注入次数的日志全部 1:1，没有一次双触发**：`.artifacts/right-panel/verify-console.log`
   里 8 次投递 → 8 行 `S1_RIGHT_PANEL`（`:13,26,34,42,49,55,64,71`，每一步恰好一行，
   连「再点同一项收起」那一步也只有一行）；`.artifacts/command-palette/keyprobe5.ps1:11,15`
   两次点击 → `k5.out.log:9-10` 两行。**同一台机器、同一个 `PostMessage` 注入器、同样的 125% DPI。**

因此：

- **不是**「gpui 在 125% DPI 下把一次点击派发两次」的产品缺陷；
- **是**「观测到的两行日志 ↔ 两次真实输入」无法区分的问题（仪表缺陷）+ 一个尚未定位的
  **第二次输入来源**（环境杂散点击 / 无障碍 Click 动作 / 脚本第二次点击，见 §5）；
- 我们的 `resolve_click` 语义与真机逐条一致，**不该改**（见 §6 P3）。

⚠️ 必须说清的一点：**「有两次输入」这个判断我没有直接证据**（注入脚本没保全、日志没有时间戳）。
我只能证明「派发层不会一发二」，以及「同一注入器在别的运行里是 1:1」。
「第二次输入从哪来」在 §5 里给候选与判据，标记为**待确认**。

---

## 2. 点击派发链（Win32 消息 → gpui 事件 → 我们的 `on_click`）

### 2.1 Win32 消息 → `PlatformInput::MouseDown/MouseUp`

| 环节 | 位置 | 事实 |
| --- | --- | --- |
| 窗口过程分派 | `gpui-pre-windows-0.3.6/src/events.rs:138` | `WM_LBUTTONDOWN => handle_mouse_down_msg(handle, MouseButton::Left, lparam)` |
| 同上 | `…/events.rs:144` | `WM_LBUTTONUP => handle_mouse_up_msg(handle, MouseButton::Left, lparam)` |
| 客户区没有 DBLCLK 分支 | `…/events.rs:117-137` | 只有 `WM_NCLBUTTONDBLCLK`（非客户区）走 down；客户区双击不做特殊处理，靠 `click_count` 自己数 |
| down 处理 | `…/events.rs:486-514` | `SetCapture(handle)`（`:492`）；`click_state.update(…)` 只算**双击计数**（`:500`）；构造 `PlatformInput::MouseDown`（`:503-509`），`position = logical_point(x, y, scale_factor)`（`:505`） |
| up 处理 | `…/events.rs:516-542` | `ReleaseCapture()`（`:522`，**在派发之前**）；构造 `PlatformInput::MouseUp`（`:532-537`） |
| 双击计数实现 | `gpui-pre-windows-0.3.6/src/window.rs:1282-1322` | `ClickState::update` 只增减 `current_count`（系统双击时限 `GetDoubleClickTime()` + 4px 容差），**不吞也不复制事件** |

**这一段里没有 `ClickEvent`。**

### 2.2 `PlatformInput` → `Window::dispatch_event` → 鼠标监听器

| 环节 | 位置 | 事实 |
| --- | --- | --- |
| 入口 | `gpui-pre-0.3.6/src/window.rs:5415` | `pub fn dispatch_event(&mut self, event: PlatformInput, cx: &mut App)` |
| 记住鼠标位置 | `…/window.rs:5441-5455` | MouseMove/Down/Up 都先 `self.mouse_position = <event>.position`（**逻辑坐标**） |
| 转鼠标派发 | `…/window.rs:5549-5550` | `event.mouse_event()` → `dispatch_mouse_event` |
| 每次鼠标事件重算命中 | `…/window.rs:5752-5757` | `let hit_test = self.rendered_frame.hit_test(self.mouse_position()); self.mouse_hit_test = hit_test;` |
| 命中实现 | `…/window.rs:1106-1128` | 从前往后遍历 hitbox，收集所有包含该点的 hitbox（`hover_hitbox_count`），**只有 `BlockMouse` 才 break** |
| hover 判定 | `…/window.rs:766-804`（`HitboxId::is_hovered`）、`:851-853`（`Hitbox::is_hovered`） | 读 `window.mouse_hit_test`；键盘模态下恒为 false；`captured_hitbox == Some(self)` 时恒为 true |
| 监听器两阶段 | `…/window.rs:5766-5789` | Capture（正序）→ Bubble（`.rev()`，祖先后于目标） |

### 2.3 `ClickEvent` 产生点（**唯一一处**）

| 环节 | 位置 | 事实 |
| --- | --- | --- |
| 类型定义 | `gpui-pre-0.3.6/src/interactive.rs:218-226`（`MouseClickEvent{down,up}`）、`:287-297`（`ClickEvent::Mouse`） | 文档原文：「A click event, generated when a mouse button is pressed and released」 |
| down：记 pending | `gpui-pre-0.3.6/src/elements/div.rs:2950-2963` | 条件：`Bubble` && 左键 && `hitbox.is_hovered(window)` → `*pending_mouse_down.borrow_mut() = Some(event.clone())`（`:2959`） |
| up：Capture 阶段取 pending | `…/div.rs:3084-3098` | `pending.is_some() && hitbox.is_hovered` → **take**（`:3087`）；`pending.is_some()` 但不再 hover → **take 且不触发**（`:3095`） |
| up：Bubble 阶段产生并派发 | `…/div.rs:3100-3121` | 构造 `ClickEvent::Mouse{down, up}`（`:3104-3107`），左键时 `for listener in &click_listeners { listener(...) }`（`:3111-3113`） |
| `on_click` 的存放 | `…/div.rs:591-596` | `self.click_listeners.push(Rc::new(listener))` —— 一个元素调 N 次 `on_click` 就**真的有 N 个**监听器 |
| `on_mouse_down` 不在这一堆里 | `…/div.rs:126-140` | 进的是 `mouse_down_listeners`，与 click 无关 |

**⇒ 一次 `WM_LBUTTONDOWN` + 一次 `WM_LBUTTONUP` ⇒ 一次 `ClickEvent::Mouse`。**

推论（§1 证据 2 的出处）：第一次 up 已把 pending take 成 `None`，第二次 up 两个分支都不进
（`div.rs:3086,3089`），`captured_mouse_down` 保持 `None`，bubble 阶段（`:3101`）什么都不做。
**重复的 `WM_LBUTTONUP` 产生 0 次额外 click。**

### 2.4 组件层：`Button` 只挂一个 click 监听器

| 环节 | 位置 | 事实 |
| --- | --- | --- |
| `on_click` 字段 | `gpui-component-0.6.6/src/button/button.rs:216`、构造时 `None`（`:262`）、setter（`:433-437`，覆盖式赋值，不是 push） | 一个 `Button` 最多一个用户回调 |
| 渲染时挂载 | `…/button/button.rs:797-806` | `when_some(self.on_click, … this.on_click(move |event, window, cx| … on_click(event, window, cx)))` —— **`.on_click(..)` 只被调用一次** |
| 另一处 `.on_click` | `…/button/button.rs:807-809` | `when(loading, …)` 才会再加一个「只 stop_propagation」的监听器；右栏图标 `loading` 恒为 false，不生效 |
| `.on_mouse_down` | `…/button/button.rs:784-795` | 只做 `window.prevent_default()` + 抑制文本选择；进的是 `mouse_down_listeners`（见上表），不产生 click |
| 可被无障碍调用 | `…/button/button.rs:734-740`（`role(Role::Button)`）、`:763-765`（`accessibility_label`） | 右栏按钮是**合法的 UIA 按钮节点**（见 §5 C2） |

### 2.5 我们的接线

| 环节 | 位置 | 事实 |
| --- | --- | --- |
| 每个图标项挂一次 | `gpui/crates/workbench/src/activity_bar.rs:403` | `.on_click(move |_event, window, cx| on_select(index, window, cx))`，在 `item_button` 内部 |
| `item_button` 每项只被调一次 | `activity_bar.rs:239-254` | 顶部组与底部组各 `iter().enumerate().filter(...)`，每项只落进其中一组 |
| 右栏整条只画一次 | `gpui/crates/workbench/src/workspace.rs:1672-1680`（构造）、`:1822`（插入） | `right_rail` 只 `let` 一次、只 `child` 一次；不存在"同一条 rail 画两遍" |
| 点击回调 | `workspace.rs:1613-1636` | `RightToolWindowView::from_rail_index(index)`（`:1617`）→ 纯函数迁移（`:1623-1624`）→ 懒扫（`:1629-1631`）→ `diagnose_right_panel`（`:1632`） |
| 状态迁移 | `gpui/crates/workbench/src/right_tool_window.rs:205-215` | `current_visible && current == clicked → (current, false)`，否则 `(clicked, true)` |
| 诊断行（**stdout**） | `right_tool_window.rs:221-223` | `println!("S1_RIGHT_PANEL view={} visible={visible}", …)` |
| 其余 4 个打同一条日志的入口 | `workspace.rs:998`（构造期）、`:1247`（菜单/命令 `ToggleMaven`）、`:1477`（`--right-view` 探针）、`:1644`（面板关闭按钮） | 所以「两行 `S1_RIGHT_PANEL`」理论上也可能来自**不同入口**；本例两行都是 rail 语义（`visible=true` 紧跟 `false`），`:1644` 只会打 `false`，`:1477`/`:1247` 不会被注入的鼠标点击触发 |

**⇒ 我们的代码没有任何"把一次点击记两遍"的写法。** 一条 rail 项 → 一个 `Button` → 一个 `on_click`。

---

## 3. 注入工具的消息序列，以及它为什么**不会**造成两次

工具：`.artifacts/p2/inject.ps1`（`.artifacts/right-panel/inject.ps1` 是它的副本 + `-Mode move`）。

```
144:    public static void Click(IntPtr hwnd, int x, int y) {
145:        IntPtr lp = (IntPtr)((y << 16) | (x & 0xFFFF));
146:        PostMessage(hwnd, WM_MOUSEMOVE,   IntPtr.Zero, lp);
147:        System.Threading.Thread.Sleep(60);
148:        PostMessage(hwnd, WM_LBUTTONDOWN, (IntPtr)1,   lp);
149:        System.Threading.Thread.Sleep(50);
150:        PostMessage(hwnd, WM_LBUTTONUP,   IntPtr.Zero, lp);
151:    }
```

`-Mode click` 的分支：`inject.ps1:217-224`（越界检查 `:219`，调用 `:222`，最后 `Start-Sleep -Milliseconds $SettleMs`（`:263`，默认 350，`：28`））。

**它投递的正好是一对**：1×`WM_MOUSEMOVE` + 1×`WM_LBUTTONDOWN`(wParam=1=MK_LBUTTON) +
1×`WM_LBUTTONUP`(wParam=0)。

它**没有**投递、因而不会触发的路径：

- `WM_LBUTTONDBLCLK`：客户区 gpui 根本不处理（`gpui-pre-windows-0.3.6/src/events.rs:117-137` 只映射 NC 的 DBLCLK），
  且 Windows 的 DBLCLK 由系统按真实鼠标生成，`PostMessage` 不会凭空造出来；
- `WM_NCLBUTTONDOWN/UP`：没有投递；
- `WM_MOUSEACTIVATE`：没有投递（gpui 在 `events.rs:96` 主动返回 `MA_ACTIVATE`，与本次无关）。

**唯一一处「会混淆观测」的设计缺陷**（不是双触发的原因，但是误判的温床）：

- 它**从不移动真实光标**。`WM_MOUSEMOVE` 是投到窗口队列的消息，只改 gpui 自己的
  `window.mouse_position`（`window.rs:5442`），`SetCursorPos` 一次都没调。
  于是**gpui 认为鼠标在图标上，操作系统认为鼠标在原地**。
  `gpui/HANDOFF.md` §3 里说 `.artifacts/ui-click.ps1` 是"真实鼠标注入"，那一支才会移光标；
  `PostMessage` 这一支不会。
  这有两个后果：① 面板打开后 gpui 仍以为指针压在图标上（hover 底色与 selected 同色，
  `.artifacts/right-panel/NOTES.md:28-31` 已经踩过这个坑）；② 任何**真实的**杂散点击
  （§5 C1/C4）如果落在真实光标所在处，命中判定用的是**真实光标的坐标**，
  与"gpui 以为的位置"无关 —— 两者是否重合纯靠运气。
- `.artifacts/right-panel/NOTES.md:30` 写「每次截图前 `-Mode move` 把指针挪到编辑区」——
  对 `PostMessage` 版来说这句话**不成立**（只挪了 gpui 的认知，没挪真光标）。
  这是现有笔记里的一处错误陈述，值得修。

**⇒ 单看消息序列，这个注入器一次调用只会造成一次 click。**
（它**不排除**脚本层调了两次：`keyprobe5.ps1:11,15` 就是同一坐标点两次，那也是两行日志的两行来源。）

---

## 4. 125% DPI 的影响分析

**结论：没有证据显示 DPI 参与。**「同一个物理点被两个图标命中」和「坐标被缩放两次」
在代码上都**不成立**，逐条给依据。

**① 坐标只被除一次。** `gpui-pre-windows-0.3.6/src/util.rs:150-156`：

```rust
pub(crate) fn logical_point(x: f32, y: f32, scale_factor: f32) -> Point<Pixels> {
    Point { x: px(x / scale_factor), y: px(y / scale_factor) }
}
```

调用点只有 4 处与鼠标坐标有关：`events.rs:392`（move）、`:505`（down）、`:534`（up）、
`:592`（滚轮，先 `ScreenToClient`）。**没有第二处再做 `/scale_factor` 或 `*scale_factor`。**
投给 gpui 的 `position` 与 hitbox 的 `bounds` 都是**逻辑坐标**，同一个空间，直接比较
（`window.rs:5753` → `:1106-1128`），所以不存在"缩放两次"。

- 唯一用 `DevicePixels` 存鼠标位置的地方是双击计数（`events.rs:499`、`window.rs:1311-1322`），
  它只影响 `click_count`，不影响命中、也不吞/复制事件。

**② 注入器自己是 Per-Monitor-V2 感知的。** `inject.ps1:41`(声明)、`:63-65`(包装)、`:193`(调用
`SetProcessDpiAwarenessContext(-4)`)，所以 `GetClientRect`（`:213`）拿到的是**物理像素**
（实测客户区 1823×1024，`.artifacts/right-panel/verify-console.log:5`），
`inject.ps1:219` 的越界检查与投递的 lParam 都是同一套物理客户区坐标。
`PostMessage` 的 lParam **不经过**任何 DPI 虚拟化（没有 `ClientToScreen`/`GetCursorPos` 参与），
所以感知级别在这里只影响越界检查，不会改变落点。

**③ 命中区不重叠（算术）。** 右栏图标项 28 逻辑 px 见方，项间距 `gap_1()` = 4 逻辑 px
（`activity_bar.rs:105`、`:277`/`:289` 的 `.gap_1()`、`:375-381` 的 `rems(28./16.)`），
所以**节距 32 逻辑 = 40 物理**，与实测的 y=74/114/154（间隔 40 物理，
`.artifacts/right-panel/NOTES.md:23`）一致。项盒 35 物理高、间隙 5 物理：
三个注入点都落在**项的中心**，离任何边界 ≥17 物理 px。
⇒ 「同一个物理点被两个图标命中」在这组坐标上不可能。
（`hit_test` 确实会**同时**收集多个 hitbox —— 包括祖先 —— 但只有 `BlockMouse` 才截断
（`window.rs:1119-1121`）。落进两个**带 click 监听器的兄弟元素**才会双触发；
右栏不存在这种重叠，且 rail 只画一次：`workspace.rs:1672-1680,1822`。）

**④ 面板显隐引起的重排不会改 ElementId。** 按钮 id 是
`ElementId::named_usize("lithe-activity-bar-right", index)`（`activity_bar.rs:375-385`），
与 `right_visible` 无关，面板开关不会让元素状态错位。

**⑤ DPI 变更不会合成点击。** `WM_DPICHANGED` 处理（`gpui-pre-windows-0.3.6/src/events.rs:878-941`）
只做 `SetWindowPos` + 缩放因子更新，不投任何输入消息。

**⇒ 「125% DPI」这个限定词目前是**相关但未被证明**：所有可复现的 1:1 观察都在 125% 下做的
（`.artifacts/right-panel/verify-console.log:12-13,25-26,…`），也就是说 125% 下**也能** 1:1。
DPI 的嫌疑应当降级。**

---

## 5. 「杂散点击」的候选解释与判断

**已登记的事实**（不是我的推断）：

- `gpui/PLAN.md:807-810`：「物理鼠标停在工作区某处时会有一次**杂散点击**落到窗口上
  （曾误触活动栏『设置』并打开对话框）……**改变光标位置后现象消失**，与代码无关。」
- `gpui/HANDOFF.md:137-138`：「这台会话里偶发杂散点击（光标停着时会落到窗口上）→
  测试前把光标挪离活动栏」。

**可以合成"完整一对 down+up"的代码路径**（一条合成对 = 一次 click，且**完全不经过鼠标**）：

| 候选 | 位置 | 说明 |
| --- | --- | --- |
| **C2 无障碍 `Action::Click`** | `gpui-pre-0.3.6/src/window.rs:6826-6846` | UIA 客户端请求 Click 时，gpui **自造** `MouseDown`+`MouseUp` 在节点中心（`:6830-6844`），走同一套 `dispatch_event` ⇒ 触发 `on_click` |
| C2 的接线（默认开启） | `…/window.rs:1606-1611`（除 `accessibility_force_disabled` 否则初始化）、`:1623-1638`（action 通道）、`:1676-1687`（消费者）；`gpui-pre-windows-0.3.6/src/window.rs:1073-1091`（`accesskit_windows::Adapter` + `A11yActionHandler`）；`…/events.rs:166`（`WM_GETOBJECT`） | 右栏按钮是合法 UIA 节点（`gpui-component-0.6.6/src/button/button.rs:734-740,763-765`；我们额外挂了 `accessibility_label`：`activity_bar.rs:399`） |
| C3 触摸 tap → 合成点击 | `gpui-pre-0.3.6/src/gestures.rs:734-765` → `window.rs:5678-5683` | 一次 tap 派发 `down`+`up` 两次 `dispatch_mouse_event`。需要**触摸数字化仪**；Parallels 可能把宿主触摸屏透给客户机 |
| C4 文件拖放 Submit | `window.rs:5507-5516` | 只合成一个 `MouseUp`。**单独不能产生 click**（没有 pending down）；只有在"已有 down 悬着"时才可能多触发一次 —— 与本例无关，仅登记 |
| C1 真实/宿主杂散点击 | 无源码，环境 | 第二次**真的**投到窗口队列的物理点击 |

**我的判断（推断，按可能性排序）**：

> ⚠️ **2026-09-26 结案（维护者确认，本条覆盖上面的更新）**：那次"无人点击却自动打开了文件、
> 按了 F12、并往夹具源码里敲进了字符"的输入，**是维护者本人手动点击与打字**。
> 所以第二次输入的来源是 **C1″ = 维护者本人的真实操作**（既不是"物理杂散点击"这种无法解释的
> 环境现象，**也不是**"并发注入会话在偷打窗口"）。上面 2026-09-26 那次把原因归到
> "脚本化注入"的更新**已被本条推翻**，保留它是为了留下推理痕迹。
>
> 不受影响的部分：**派发层没有"一次点击派发两次"的缺陷**（§2/§3 的读码事实与 1:1 的注入日志
> 都仍然成立），**不要**去改 toggle 语义、**不要**给面板加防抖。
> 操作纪律照旧保留（卫生，不是缺陷证据）：同一时刻只跑一个注入会话；优先用不依赖点击的
> 确定性路径（`--right-view` 探针、菜单探针、资源管理器搜索框）；注入前后用 `SetCursorPos`
> 移开真光标。

1. **C1″（维护者本人操作）已确认**（修正后的判断）。理由：① 派发层不可能一发二（§2/§3）、
   我们侧也只挂一次 `on_click`；② 可对齐注入次数的日志**全是 1:1**（8 次投递 → 8 行）；
   ③ 维护者本人确认那次输入来自他（此前记为"只有脚本/人手注入才能产生"，现在确定为"人手"）。
   原 C1（物理杂散点击）保留为**次要**可能：PLAN §8.8 记录的"光标停着掉一次点击"现象仍然存在，
   但它只能产生点击、无法解释敲字。
   **放大机制**：注入的 down 会让窗口 `SetCapture`（`gpui-pre-windows-0.3.6/src/events.rs:492`），
   在这 50ms（`inject.ps1:149`）里落在**任何地方**的真实按键消息都会被路由到 Lithe 的客户区
   （坐标系换成客户区坐标）。不过这只有 50ms，且 up 会 `ReleaseCapture`（`:522`），
   所以它是"放大器"而不是"主因"。
2. **C2（UIA `Action::Click`）值得单独排一次**。它是唯一一条**不需要任何鼠标事件、
   也能在光标完全静止时发生**的路径，且右栏按钮恰好是可被 UIA 调用的 Button。
   **判据**：框架在 a11y 激活时打 `log::info!("Accessibility activated")`
   （`window.rs:1630`），但 **Lithe 没有安装任何 logger**（`gpui/crates/app/src/**` 无 `log` 初始化，
   `gpui-kit-0.6.6/src/lib.rs` 里也没有），这句话**被丢掉了**。
   我全量 grep 了 `.artifacts/**`：**没有任何一次运行有这条日志** ——
   但因为日志本身没接线，这**不能**作为"当时没有 UIA 客户端"的证据。
   （我现在查过进程：`Narrator`/`Parallels`/自动化工具都没在跑，会话也不是锁屏态
   —— 但这是**现在**，不是 p14 那次运行。）
3. **C5（脚本自己点了两次）不可排除**。`p14/spring.out.log:66-70` 的四行最自然的读法就是
   「点 spring 开 → 点 spring 收 → 点 maven 开 → 点 maven 收」，与 `p6/after.out.log:50-55`
   （三个视图各开一次再各收一次）是同一个套路。**那次运行的脚本没有保全**
   （`.artifacts/p14/` 下没有任何 `.ps1`），所以无法判定。
4. **C6（派发层重复派发）可以排除**，依据见 §2.3 与 §1 证据 2/3。

**顺带一个真实但不属于本缺陷的问题**：`workspace.rs:1629-1631` 在 **MouseUp 的派发栈里
同步**跑 `scan_right_view_if_needed`，Spring 那一路要读依赖 JAR 里的
`spring-configuration-metadata.json`（`workspace.rs:1442-1460` 的模块文档自己写明了）。
UI 线程被卡住的这段时间里，任何真实/杂散点击都只能排在队列里、随后被立刻处理完
——**它会让"第二次点击"更容易和第一次挤进同一段观测窗口**，但不产生点击。
建议单独立项（性能），不要和本缺陷混在一起修。

---

## 6. 修复建议（按优先级）

### P0 —— 先修仪表，让「一次点击两行」变成可证伪（唯一必须做的一步）

现在两行 `S1_RIGHT_PANEL` **没有任何时间戳、没有序号、没有坐标**，所以"一次点击的两行"
和"两次输入的两行"在证据上完全同构。最小改动：

1. `right_tool_window.rs:221-223` 的 `diagnose` 增加 `seq`（进程内单调递增 `usize`）
   与 `t_ms`（相对进程启动的毫秒）；
2. 在 rail 点击回调里把**本次点击的位置**也带上（`on_select` 已经拿到 `window`，
   `activity_bar.rs:403`，`window.mouse_position()` 可用），例如
   `S1_RIGHT_PANEL seq=7 t_ms=8123 view=spring visible=true x=1421.6 y=123.2`；
   **同一条诊断改走 `eprintln!`**（HANDOFF §1.8 的规则；现在它走 stdout，与其它 `S1_*` 的
   相对顺序不可靠 —— 不过 stdout 内部顺序是可靠的，所以 `true`→`false` 的先后是真的）。

**机器判据（做完就能一次判定根因）**：

- `Δt < 1ms` 且两次坐标完全相同 ⇒ 同一次派发栈里的重复回调（才是真·派发层缺陷）；
- `Δt` 几十~几百 ms，或坐标不同 ⇒ **两次独立输入**，转 §5 查第二次输入的来源。

### P1 —— 免改代码的验证纪律（立刻可用，最可能直接消掉现象）

1. 每次注入前后，**把真实光标移离右活动栏**（`SetCursorPos`，即 `.artifacts/ui-click.ps1`
   那一支的做法）。`PostMessage` 版的 `-Mode move` **不移动真光标**，别再用它当"挪开指针"。
2. 判定不要只看 `S1_RIGHT_PANEL` 的行数，**配合像素证据**（右栏选中底色占比：选中 70.4% /
   静止 14.4%·9.3%·16%，`.artifacts/right-panel/NOTES.md:42`）—— 这是"到底开了还是关了"的
   独立通道。
3. 提高注入次数（同一坐标点 10~20 次），统计"每次投递产生的日志行数分布"。
   1:1 是期望值；任何 >1 的样本都留档，等 P0 的仪表去定性。
4. 校正 `.artifacts/right-panel/NOTES.md:30` 的错述（`-Mode move` 不挪真光标）。

### P2 —— 若要机器回归（可选，且要先接受它覆盖不到什么）

- **可以**在 `gpui` 的 test-support 里写一条集成测试：模拟 `MouseDown`+`MouseUp`
  （`gpui-pre-0.3.6/src/app/test_context.rs:902-931` 有 `simulate_mouse_down` / `simulate_mouse_up`，
  `:933-948` 还有 `simulate_click`），
  断言 `on_select` **恰好被调用一次**。
- **必须承认**：这条测试**在"修复前后都会是绿的**"，因为派发层本来就只发一次（§2.3）。
  它不是这个症状的回归测试，只是把"我们已知的派发语义"钉住。
  这一条按 `diagnosing-bugs` 的口径属于「没有正确的 seam」——**要写进结论而不是假装有覆盖**。
- 另外，`resolve_click` 已有的单测（`right_tool_window.rs:636-672`）覆盖的是**状态机**
  （点同一项收起 / 点别项切换 / 隐藏时打开），它同样覆盖不到本缺陷。

### P3 —— **不要做**的事

1. **不要改 `resolve_click` 的 toggle 语义，也不要给它加防抖/去重**。
   真机就是"点同一项收起"：`windows/tauri/src/features/layout/actions/right-tool-window-actions.ts:26-31`
   （`intent === "toggle" && isVisible → isRightSidebarVisible: false`），
   rail 上就是一次 `onClick` 一个 toggle（`…/components/plugin-activity-rail.tsx:62`）。
   用户**快速点两次**本来就该"开→关"（双击也一样：`click_count` 只是计数，
   `div.rs:3111` 对每个左键 click 都触发）。加防抖会误伤正常交互，而且会把真因埋掉。
2. **不要**为了绕开这个现象而去掉 `on_click` 换成 `on_mouse_down` 之类 ——
   真机是 click 语义（按下后移开再松手不该触发），换了会引入新偏差。

### 建议登记（不属于本次修复）

- `workspace.rs:1629-1631` 的同步懒扫放在鼠标派发栈里（Spring 会读 JAR）：单独立项。
- `S1_RIGHT_PANEL` 走 stdout 而其它 `S1_*` 走 stderr：统一到 stderr。
- a11y 日志未接线（`window.rs:1630` 的 `log::info!` 无人接收）：装 logger 前，
  §5 C2 无法用日志证伪。

---

## 7. 未确认项 + 下一步确认办法

| # | 未确认 | 为什么没确认 | 怎么确认（谁来做） |
| --- | --- | --- | --- |
| 1 | p14/spring 那 4 行到底对应几次注入 | 那次运行的 `.ps1` 没保全（`.artifacts/p14/` 下没有脚本）；日志无时间戳 | **实现代理**：照 P0 加 `seq`/`t_ms`/坐标，用 `p15/spring-fixture` 重跑同一场景 |
| 2 | 第二次输入来自哪里（C1 环境杂散 / C2 UIA / C5 脚本） | 无判据数据；a11y 日志被吞 | **实现代理**：先做 P0；若 `Δt` 大且坐标相同，再装 logger 看 `Accessibility activated`（`window.rs:1630`），或临时在 `handle_a11y_action`（`window.rs:6805`）加一行 stderr 探针 |
| 3 | 真机（人手）一次点击是否真的一发二 | **只有人能验**（本会话无人值守）。派发层读码结论是"一发一"，但真机受驱动/VM 输入链路影响 | **维护者/人工**：在真机 125% DPI 下点右栏图标 20 次，看是否出现 `true` 紧跟 `false`；这一条无法用注入代替 |
| 4 | 125% DPI 是否真的相关 | 125% 下已有 8/8 的 1:1 观测（`right-panel/verify-console.log`），无法证明"只在 125% 出现" | **实现代理**：把显示器切到 100% 复跑同一注入序列（约 10 次），做差分 |
| 5 | `SetCapture` 窗口（50ms）是否是杂散点击的放大器 | 无法在不改代码的前提下观测 | 低优先；若要查，用 ETW/`Spy++` 类工具看投递序列（需要桌面权限） |

**汇报口径提醒**：在任何后续文档里请把 §1 那句结论原样带着
（「派发层不会一发二」是**读码事实**；「第二次输入从哪来」是**待确认推断**），
不要把后者写成前者，也不要把本缺陷在 `PLAN.md:1656` / `HANDOFF.md:47` 的旧描述
（"125% DPI 下会双触发，真缺陷"）当成已证实的结论继续引用。
