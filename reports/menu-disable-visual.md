# 菜单项禁用态（置灰）视觉基线 —— S5 菜单动态 `disabled`

> 作者：颜思齐（UI 设计）　分支：`feat/gpui-editor-basics`　状态：**待实现，口径已定死**
>
> 范围：workbench 菜单栏 / 右键菜单里「此刻不可用」的菜单行（无活动缓冲时 Save / SaveAs / SaveAll / RevertFile / CloseTab / Undo / Cut / Find …）长什么样。
>
> 本文只写口径，**不改动任何 Rust 代码**（按主理人分工，实现同学照做即可）。
>
> 上游依据（避免与之冲突）：
> - 既有视觉基线 `reports/editor-basics-visual-spec.md:260-261`（菜单行规格）、`:464-472`（右键菜单对比度表）、`:471` 与 `:484`（`fg_disabled` 的既有判定）。
> - 验收基线写法 `reports/editor-basics-remaining-design.md:526-531`（§10：实测数字 + 「验收基线取 X:1」，不给空泛要求）。
> - S5 条目 `reports/editor-basics-remaining-design.md:519`。

---

## 0. 一句话口径（先看这条）

**置灰 = 整行（文字 + 图标 + 键帽文字）落 `colors.fg_disabled`（不新增 token、不改主题配色）＋ 不画 hover 底 ／ 不设 pointer 光标 ／ 键盘导航跳过；自画图标的颜色由 workbench 在建行时按状态自己给；键位胶囊照画但键帽文字随行降到 `fg_disabled`（落点在 `typography/inline.rs`，**不是** `draw.rs:96`）。**

三件必须知道的事（都是核实过的，见下文引用）：

1. **库已经把「文字色 + hover + 光标 + 键盘跳过」做完了**，宿主只要调 `.disabled(true)`（`menus/model.rs:107`），不需要自己画。
2. **自画图标（`icon_element`）是唯一的宿主活儿**：库画不出它的禁用色（`menus/model.rs:94-95` 的明文陷阱），workbench 必须自己按状态选色 —— 见 §4。
3. **键位胶囊（`keys`）保留，但要跟着变淡；只改 `draw.rs:96` 没用** —— 那行 `text_color` 是**死样式**（键帽文字在 `typography/inline.rs:55` 硬取 `fg_muted`，「+」在 `:120` 硬取 `fg_subtle`），真要变淡得给 `KbdCombo` 开一个默认关闭的 `.disabled(bool)` —— 见 §3.5。

---

## 1. 禁用态设计 token

### 1.1 结论：用 `colors.fg_disabled`，**不新增 token**

| 候选 | 结论 | 理由 |
| --- | --- | --- |
| `colors.fg_disabled` | ✅ **采用** | 库内禁用态的唯一既定槽：`menus/draw.rs:21-25`、`forms/options.rs:124-128`、`forms/input.rs:142-146`、`forms/select.rs:125-126`、`forms/switch.rs:141`、`forms/check.rs:158`、`forms/radio.rs:124`、`forms/chips.rs:90`、`navigation/tabs.rs:367`、`navigation/menu.rs:184-188`。新增 `menu_fg_disabled` 之类等于开第二套禁用色，与「只用 token」的初衷相反。 |
| 新增 `fg_disabled_strong` / 提高明度 | ❌ | 那是在**菜单里**偷偷改主题：`fg_disabled` 同时供按钮/表单/下拉/标签页使用，动它要动主题单，不是 S5。见 §2.3。 |
| `colors.fg_muted` / `colors.fg_subtle` | ❌ | 这两个是「次要信息」，不是「不可用」。而且 `fg_subtle` 已被 workbench 的「未接线」行占用（`menu_bar.rs:1952` 注释明文：未接线行**刻意**不用 `fg_disabled`，因为它要传达「还没做」而不是「现在不能用」，且要保住键盘可达性）。置灰与它混了就把两种状态画成一个样。 |
| 整行 `opacity(0.45)`（按钮口径） | ❌ | 见 §1.4。 |

### 1.2 库默认取值（无宿主主题时，gallery / 裸库）

| 主题 | token | 值 | 定义处 | 菜单面板底 | 对比度 |
| --- | --- | --- | --- | --- | --- |
| 浅 | `fg_disabled` | `#acaaa8` | `rust/lithe-ui/src/theme/palette.rs:231` | `overlay` `#ffffff`（`palette.rs:219`） | **2.32:1** |
| 深 | `fg_disabled` | `#5c5a58` | `rust/lithe-ui/src/theme/palette.rs:311` | `overlay` `#201f1d`（`palette.rs:300`） | **2.40:1** |

（与既有基线的「浅 2.3:1 / 深 2.4:1」一致 —— `editor-basics-visual-spec.md:471`。）

面板底之出处：菜单面板 = `forms::surface`（`forms/options.rs:195-206`）`.bg(colors.overlay)`，由 `menus/draw.rs:176` 调用。

### 1.3 宿主实际取值（workbench 生效值 —— **这才是验收要看的**）

workbench 不用库默认调色板：`lithe_gpui_settings::theme::palette_for` 把 `themes/jetbrains.json` 落成组件库调色板（`crates/settings/src/theme.rs:348-495`，主题在 `theme.rs:14-17` 内置）。关键两条：

```rust
// crates/settings/src/theme.rs:456-460
if let Some(color) = color("muted.foreground") {
    palette.fg_muted = color;
    palette.fg_subtle = color;
    palette.fg_disabled = color.opacity(0.6);   // ← 禁用色 = 次要前景色 × 60% 不透明度
}
```

```rust
// crates/settings/src/theme.rs:441-448（浮层底色 = popover.background）
if let Some(popover) = popover { palette.overlay = ... popover ...; }
```

于是：

| 主题 | `muted.foreground` | `popover.background`（面板底） | `fg_disabled` = 前者 @ α0.6 叠在后者上 | 合成色 | 对比度 |
| --- | --- | --- | --- | --- | --- |
| IntelliJ Light | `#6e6e6e`（`themes/jetbrains.json:22`） | `#f2f2f2`（`:27`） | `#6e6e6e` @0.6 over `#f2f2f2` | ≈ `#a3a3a3` | **2.26:1** |
| Darcula | `#9aa0a6`（`:148`） | `#313335`（`:153`） | `#9aa0a6` @0.6 over `#313335` | ≈ `#707479` | **2.71:1** |

对照（同面板底，用于看「淡了多少」）：

| 内容 | 浅（IntelliJ Light） | 深（Darcula） |
| --- | --- | --- |
| 启用行文字（`foreground`：`#080808` / `#a9b7c6`，`:15` / `:141`） | 17.89:1 | 6.21:1 |
| **禁用行文字（上面的 `fg_disabled`）** | **2.26:1** | **2.71:1** |
| 启用 vs 禁用 的落差（两色同底互比） | 7.92:1 | 2.29:1 |

> Darcula 的落差（2.29:1）比浅色小得多，原因是 Darcula 的正文 `foreground` 本身就只有 6.21:1。**这是主题固有值，不因置灰而变**；禁用行仍有第二个信号兜底 —— 无 hover 反馈（§3.3）。

### 1.4 为什么不用「按钮的 `opacity(0.45)`」口径

`Button` 的禁用是整元素 `.opacity(0.45)` + `.cursor_not_allowed()`（`buttons/button.rs:493-499`）。菜单**不照抄**：

- 菜单是「一行文本 + 一个图标」，按钮是「一整块底 + 边框」；整行降不透明度会把**行内每个子元素的颜色一起拽向面板底**，得到的实际对比度随底色漂（实测：图标色 `#080808` @0.45 落在 `#f2f2f2` 上 = 3.14:1，`#a9b7c6` @0.45 落在 `#313335` 上 = 2.47:1 —— 两主题不一致，且与文字的 `fg_disabled` 不同步）。
- 库内**菜单家族**（`menus/draw.rs`、`forms/options.rs:124`、`navigation/menu.rs:184`）已经统一是「`fg_disabled` 换前景色」，而 `opacity` 只有 `Button` 一家。改菜单去跟 `Button` 只会制造新的不一致。

---

## 2. 对比度验收基线

按 §10 的写法（实测 → 给数字 → 定基线），**不写「应满足 WCAG AA」这类无法执行的空话**。

### 2.1 验收基线（取这两个数）

| 项 | 验收基线 |
| --- | --- |
| 禁用行文字对菜单面板底 | **浅 2.26:1 ／ 深 2.71:1**（宿主 `jetbrains.json`） |
| 库默认回退时（无宿主主题 / gallery） | **浅 2.32:1 ／ 深 2.40:1** |
| 禁用行**键帽文字**对键帽底（`surface`） | **浅 2.38:1 ／ 深 2.89:1**；库默认回退 浅 2.32:1 ／ 深 2.55:1（§3.5） |
| 允许偏差 | ±0.05（色值与合成算法不变则应当一模一样；超出即说明有人改了 token 或 `theme.rs:459` 的 0.6） |
| 附加判据 | 键帽文字与行文字**同档**：两者对比度差值 < 0.2（今天差 2.8 倍，改后 < 0.2） |

### 2.2 低于 3:1 —— 判定与理由

**接受，不算违规，也不在 S5 里改。**

- 这是 `fg_disabled` 的**固有值**，不是菜单引入的：库默认 2.32/2.40，宿主推导后 2.26/2.71，两者都在 2.2–2.8 区间。
- 既有基线已经判过一次：`editor-basics-visual-spec.md:471` 写「约 2.3:1 / 约 2.4:1 … WCAG 1.4.3 明确豁免『非活动组件』——属既定设计，不算违规」，`:484` 复述为结论 2。本文沿用该判定，**不与之冲突**。
- 补偿信号（让「禁用」不靠颜色单独传达，WCAG 1.4.1）：① 不画 hover 底；② 鼠标不变成 pointer；③ 键盘方向键跳过该行（`menus/model.rs:192-203`）；④ 点击无任何反应（`menus/menu.rs:227-229` 直接 return）。四个信号里有三个是非颜色的。
- 深色下文字落差只有 2.29:1 偏弱，**但这是 Darcula 正文 6.21:1 的连带结果**；若要改善，动的是主题而不是菜单。

### 2.3 后续单建议（**不在 S5 内**）

若要提亮禁用态，正确的落点是 `crates/settings/src/theme.rs:459` 的 `color.opacity(0.6)`：

- `themes/jetbrains.json` **没有** `disabledForeground` 一类的键（已核实：全文件无 `disabled` 相关键），所以宿主是从 `muted.foreground` 叠 0.6 推出来的。
- 库侧 VS Code 主题通道倒是映射了 `disabledForeground → fg_disabled`（`rust/lithe-ui/src/settings/vscode.rs:54`），但 workbench 走的是宿主自己的 `palette_for`，不走那条。
- **建议单（主题向，非 S5）**：把宿主的 `fg_disabled` 从「`muted.foreground` @0.6」改成主题里一个真实存在的键（例如 `popover.foreground` 的降权版，或新增主题键并补进 `jetbrains.json` 两条主题），并把浅色抬到 ≥3:1。**动之前先算清**：那会同时改按钮/表单/下拉/标签页的禁用色，属全库视觉变更，需要单独评审。

---

## 3. 禁用行的完整视觉规格（逐项）

逐项给结论。「现状」列 = 库里今天已经是什么样；「要做什么」= S5 实现同学的动作。

| # | 项 | 禁用态口径 | 现状 / 出处 | S5 要做什么 |
| --- | --- | --- | --- | --- |
| 3.1 | 行文字 | `colors.fg_disabled` | 已实现：`menus/draw.rs:21-25` → `:66 .text_color(fg)` | 无（宿主只调 `.disabled(true)`） |
| 3.2 | 行图标 | `colors.fg_disabled` | 库内图标（`.icon(IconName)`）已实现：`draw.rs:41-45`；**自画图标（`icon_element`）没有**，见 §4 | **宿主侧给色**（§4） |
| 3.3 | hover 底 | **不画** | 已实现：`draw.rs:37`（`.when(current && !item.disabled, …)`）、`:67` 同 | 无 |
| 3.4 | 光标 | **默认箭头**（不 `pointer`、也**不** `not-allowed`） | 已实现：`draw.rs:38` / `:68` 只在非禁用时 `.cursor_pointer()` | 无 |
| 3.5 | 键位胶囊（`keys`） | **照画**，键帽文字 + 「+」→ `fg_disabled`；键帽底/描边不变 | 现状恒 `fg_muted`（键帽）/`fg_subtle`（「+」），且 `draw.rs:96` 是死样式 | **开库**：`KbdCombo` 加 `.disabled(bool)`（默认关）+ `draw.rs` 传 `item.disabled`（§3.5） |
| 3.6 | 子菜单箭头 ▶ | `fg_disabled`（S5 实际碰不到） | 现状恒 `fg_subtle`：`draw.rs:100-106` | 无（可选项见 §3.6） |
| 3.7 | 勾选态 Check / Radio | `fg_disabled` | 已实现：`draw.rs:41-45`（`Check(true)`/`Radio(true)` 取 `fg`） | 无 |
| 3.8 | 键盘可达性 | 跳过 | 已实现：`model.rs:192-203`（`step` 只停非禁用行）、`menu.rs:227-229`（`choose` 对禁用行 return） | 无 |
| 3.9 | 无障碍 `aria-disabled` | **无**（库内菜单行不带 aria） | `draw.rs` 的 `row()` 没有任何 aria 属性 | 不做，见 §3.9 |

### 3.1 行文字 —— `fg_disabled`

`draw.rs:21-25` 已按 `item.disabled` 取 `fg_disabled` / `fg`，`:66` 落到行上。宿主零改动。

### 3.2 行图标 —— 分两条路

- **库内图标**（`MenuItem::icon(IconName)`）：`draw.rs:41-45` 给了 `if item.disabled { fg } else { colors.fg_muted }`，即禁用时是 `fg_disabled`。**已完成。**
- **自画图标**（`MenuItem::icon_element`）：`draw.rs:48-56` 直接把宿主给的闭包画进图标槽，不参与配色 —— 这正是 `model.rs:94-95` 注释警告的那件事。workbench **走的就是这条**（`menu_bar.rs:1903-1904`）。处理办法见 §4。

> 顺带核实（不是 S5 的活，别顺手改）：workbench 的自画图标默认画在 `colors.fg` 上（`primitives/icon.rs:506`：没给颜色就取主题 `fg`），而库内菜单图标是 `fg_muted`（`draw.rs:44`）—— **启用态下 IDEA 图标比库内图标更黑/更亮一档**。这是既有状态，改它会让整排菜单行的图标一起变，属另开一条。

### 3.3 hover —— 不响应

`draw.rs:37` / `:67`：只有 `current && !item.disabled` 才画 `colors.chrome_hover` 底；`draw.rs:145-164` 的 `on_mouse_move` / `on_mouse_down` 也只在非禁用时挂。所以鼠标划过禁用行：底不变、无点击、且不抢走相邻行的高亮。**已实现。**

（量级参考，与禁用行无关：宿主浅色 `chrome_hover` = `button.hover.background #d9e5f2`（`jetbrains.json:43`），对面板底 `#f2f2f2` 是 1.14:1；深色 `#3c3f41`（`:173`）对 `#313335` 是 1.20:1 —— 与既有基线「hover 洗色本来就低，口径一致，维持」的判断一致，`editor-basics-visual-spec.md:472`。）

### 3.4 光标 —— 默认箭头，不要 `not-allowed`

结论：**保持默认箭头**。

- 不设 `cursor_pointer`：`draw.rs:38` / `:68` 已经只在非禁用时设。
- **也不要加 `cursor_not_allowed`**：菜单是瞬时浮层，`Button` 那套 `not-allowed`（`button.rs:498`）不是菜单的惯例；而库里另外两处禁用行（下拉选项行 `forms/options.rs:139-141`、浮层链接行 `navigation/menu.rs:195-206`）也都是「不设 pointer」而已，没有 `not-allowed`。菜单跟它们保持一致。

### 3.5 键位胶囊（`keys`）—— **保留**，但键帽文字降到 `fg_disabled`

**结论（与架构师对齐后定稿，覆盖初稿的「禁用行不画胶囊」口径）：禁用行照旧画键位胶囊，但键帽文字与「+」连字符一起降到 `colors.fg_disabled`，与行文字、行图标同档。**

保留的理由（架构师提出，我认可）：「有没有快捷键」是**静态事实**，「此刻能不能用」是**动态事实** —— IDEA 与 VS Code 的禁用菜单项都照常显示快捷键，只是随行一起变淡。抹掉它会丢信息，也会让行宽在「可用/不可用」之间跳。

**实现口径（注意落点不在 `draw.rs:96`）：**

1. **改 `draw.rs:96` 单独一行是无效的**（已核实，这是最关键的坑）：那行的 `.text_color(colors.fg_subtle)` 包不住胶囊 —— `KbdCombo` 里每个键帽在 `typography/inline.rs:55` 硬取 `theme.colors.fg_muted`、底色 `:52` 硬取 `surface`、描边 `:51` 硬取 `border_strong`，非 Mac 平台的「+」在 `:120` 硬取 `fg_subtle`。**父级 `text_color` 对它们全是死样式**，所以改父级不会让胶囊变淡。
2. 真正要动的是 `typography/inline.rs`：**让 `cap()`（`:41-57`）接一个颜色/状态**，`KbdCombo` 加一个默认关闭的开关（建议 `.disabled(bool)`，与 `MenuItem::disabled` 同名同义；若排版层想要中性名字，`.muted(bool)` 亦可），开启时键帽文字取 `colors.fg_disabled`、`:120` 的「+」也取同一个值；关闭时与今天一字不差（`fg_muted` / `fg_subtle`）。
3. `menus/draw.rs:91-99` 把 `item.disabled` 传进去：`KbdCombo::new(&keys).disabled(item.disabled)`。同一处的 `.text_color(..)`（`:96`）顺手写成 `if item.disabled { colors.fg_disabled } else { colors.fg_subtle }` —— 它本身是死样式，改它只为让文件读起来自洽，免得下一个人又以为「键位的颜色在这一行」。
4. **键帽的底色（`surface`）与描边（`border_strong`）保持不变**：实测键帽底对面板底只有 1.12:1、描边对面板底 浅 1.50:1 / 深 1.70:1（`#c8c8c8` / `#555555` vs `#f2f2f2` / `#313335`）—— 形状本身很安静，真正抢眼的就是**文字**（今天 5.10 / 5.36:1）。所以只降文字即可，不必动壳。

**改完的数字（验收用）：**

| 键帽文字 | 今天（`fg_muted` 落在键帽底 `surface` 上） | 改后（`fg_disabled`） |
| --- | --- | --- |
| IntelliJ Light | 5.10:1 | **2.38:1**（合成 ≈`#a8a8a8` 落在 `#ffffff` 上） |
| Darcula | 5.36:1 | **2.89:1**（合成 ≈`#6e7175` 落在 `#2b2b2b` 上） |
| 库默认（gallery） | 浅 6.54:1 / 深 7.84:1 | 浅 **2.32:1** / 深 **2.55:1** |

改后与行文字（浅 2.26 / 深 2.71）落在同一档（差值 < 0.2），**「⌘S 比 Save 还清楚」的现象消失**，整行读作一档灰。

**影响面（数字经架构师复核更正，以此为准）**：`KbdCombo` 在 `rust/lithe-ui/src/` 下共 **6 处生产调用** —— `menus/draw.rs:97`、`settings/shortcuts.rs:94`、`navigation/palette/kinds.rs:83`、`editor/welcome.rs:410`、`editor/keybindings.rs:187`、`forms/hotkey.rs:101`；画廊另有 4 处（`pages/typography/text.rs:107/112`、`pages/theme/platform.rs:88`、`pages/interaction/keys.rs:88-89`）。**开关默认关闭 ⇒ 除菜单行以外的 5 处生产调用与画廊全部零变化**，PR 里必须点明，否则会被当成一次全仓改色来回归。
**哨兵**：`src/shell/tests.rs:178` 已经在测试里画了 `KbdCombo` —— 开关默认值一旦改坏既有输出，它是最便宜的警报，跑 `cargo test -p lithe-ui` 即可。

（另一条已核实的平台细节：`KbdCombo` 的「+」连字符只在**非 Mac** 平台画（`typography/inline.rs:108` `spelled = platform != Platform::Mac`）。所以本节第 2 点里「+」的降档对 macOS 无影响，但仍要一起改 —— Windows/Linux 上它会画。）

> 回退方案（不要当首选）：**禁用行不设 `keys`**（`menu_bar.rs:1906-1908` 加 `&& !disabled`）。代价被我初稿低估了 —— 会受影响的不止 `Find` 一条。架构师按 `menu_key()`（`menu_bar.rs:567-597`）逐条复核：在「无缓冲时置灰」这批动作里 `menu_key()` 有值的共有 **10 条**（见 `reports/menu-disable-arch.md` §2.5）：
> `GoToLine`（`:575`）、`Find`（`:580`）、`FindAndReplace`（`:581`）、`ToggleComment`（`:586`）、`DuplicateLine`（`:587`）、`DeleteLine`（`:588`）、`MoveLineUp`（`:589`）、`MoveLineDown`（`:590`）、`NextTab`（`:593`）、`PreviousTab`（`:594`）。
> 10 行在有/无缓冲之间宽度与信息量跳变，比给 `KbdCombo` 加一个默认关闭的开关更贵。**只在开关确实加不上的时候用它。**

### 3.6 子菜单箭头 ▶ —— 同 `fg_disabled`（S5 实际碰不到）

`draw.rs:100-106` 用 `IconName::ChevronRight` + `colors.fg_subtle`，不区分禁用。workbench 里带子菜单的行只有主题子菜单（`menu_bar.rs:1936-1941`），它不会被置灰（没有可选主题时整行不出现），所以 **S5 不需要动它**。

若将来出现「禁用的子菜单行」：箭头应与文字同落 `fg_disabled`（`draw.rs:104` 加 `if item.disabled { colors.fg_disabled } else { colors.fg_subtle }`）。禁用子菜单行本身也打不开：`draw.rs:145` 不挂鼠标处理、`menu.rs:227-229` 对禁用行直接 return，两条都盖住了。

### 3.7 勾选态 Check / Radio —— 已经是 `fg_disabled`

`draw.rs:41-45`：`Kind::Check(true)` → `IconName::Check` 取 `fg`；`Kind::Radio(true)` → `IconName::Dot` 取 `fg`；禁用时 `fg` 就是 `fg_disabled`（`draw.rs:21-25`）。**已完成，不用管。**

（workbench 的菜单行目前没用 `check` / `radio`，这条是给库口径背书。）

### 3.8 键盘 —— 已跳过，不用处理

`Menu::step`（`model.rs:192-203`）只停在非禁用行；`choose`（`menu.rs:227-229`）对禁用行 return。方向键不会停在置灰项上，回车也不会触发它。**已实现。**

### 3.9 无障碍 —— 缺口，不在本单

菜单行整体没有 aria（`draw.rs` 的 `row()` 无 `.role()` / `.aria_*`），所以置灰行对读屏没有「不可用」的信号。补它需要动库（给行加 `aria-disabled`），且要连带考虑启用行是否补 `role="menuitem"`。**记为后续单，S5 不做**（不阻塞本次置灰：置灰的四个非颜色信号已存在，见 §2.2）。

---

## 4. 自画图标元素（`icon_element`）的禁用色 —— 具体口径

### 4.1 先确认一件事：能不能给 IDEA 图标换色？

**能，且不用换 SVG、不用新增图标。**

- `Icon::render`（`primitives/icon.rs:501-543`）最后画的是 `svg().text_color(color)`，`color = self.color.unwrap_or(theme.colors.fg)`（`:506`）。
- gpui 0.3.8 把 SVG 渲成**纯 alpha 蒙版**再拿这个颜色去填：`elements/svg.rs:182-186`（`paint_svg(..., color, cx)`）→ `svg_renderer.rs:231-252`（`render_alpha_mask`，只取 `p.alpha()`）。
- 因此 **SVG 文件里自带的颜色完全不参与**（`assets/icons/idea/legacy/ui/save.svg:3` 的 `stroke="#6C707E"` 只是源文件痕迹，画出来不是这个色），`Icon::color()` 对 IDEA 图标一样生效。

> 顺带核实：workbench 菜单图标**今天**画的是 `colors.fg`（`idea_icon_mono` → `Icon::from_path` 不给颜色 → `icon.rs:506` 回落 `fg`），不是文件里那个 `#6C707E`。

### 4.2 结论：**不动库**（就图标而言），宿主按状态选色

> 本单唯一会碰 `rust/lithe-ui` 的地方是 §3.5 的 `typography::KbdCombo`；图标这条路**不碰库**。

理由：

1. 库那条 `Build = Rc<dyn Fn(&App) -> AnyElement>`（`model.rs:17`）拿不到行状态，要让它拿到就得改类型签名，牵动 `MenuItem::element` / `icon_element` 的全部调用方 —— 为一个颜色改公共闭包签名不划算。
2. **宿主在建行时本来就知道这一行禁不禁**：`build_menu(index, cx)`（`menu_bar.rs:1889`）只在菜单展开时被调用，且手上有 `&mut App`（`menu_bar.rs:1826-1827` / `:1693`），可以读实体/全局判断「有没有活动缓冲」。状态是可得的，不用库传。
3. 库侧注释本身就是这个意思：`model.rs:94-95` ——「自画的元素不知道这件事，要做禁用行的话，颜色得自己按状态给」。

### 4.3 实现口径（照抄即可）

`menu_bar.rs:1996-1999` 的 `icon_element` 加一个 `disabled` 参数，闭包里按状态取色：

```rust
/// 菜单行里的图标槽：本仓那张图，深浅与禁用色等画的时候再问主题。
///
/// 禁用行的颜色由**宿主**给：库那条路（`MenuItem::icon`）会在禁用时把图标画成
/// `fg_disabled`，自画的元素拿不到这个状态（见 `lithe_ui::menus::MenuItem::icon_element`
/// 的注释），所以这里自己按状态选。`Icon::color()` 对 IDEA 图标同样生效
/// （gpui 把 SVG 当 alpha 蒙版画，文件里的颜色不参与）。
fn icon_element(
    icon: &'static idea::IdeaIcon,
    disabled: bool,
) -> impl Fn(&App) -> AnyElement + 'static {
    move |cx: &App| {
        let theme = cx.theme();
        // 非禁用时显式给 `fg`，与 `Icon::render` 的回落值（icon.rs:506）一致：视觉零变化，
        // 只是把隐式行为写明。
        let color = if disabled {
            theme.colors.fg_disabled
        } else {
            theme.colors.fg
        };
        idea_icon_mono(icon, theme.is_dark())
            .color(color)
            .into_any_element()
    }
}
```

`menu_bar.rs:1902-1908` 的建行处（**`keys` 照设**，变淡由库侧的 `.disabled(..)` 负责，见 §3.5）：

```rust
let mut row = MenuRow::new(action.label()).disabled(disabled);
if let MenuIcon::Idea(icon) = action.icon() {
    row = row.icon_element(icon_element(icon, disabled));
}
if let Some(keys) = action.menu_key() {
    row = row.keys(keys);
}
```

注意：`icon_element` 的另一处调用是主题子菜单行（`menu_bar.rs:1989`），那行不会被置灰，传 `false` 即可。

### 4.4 两个「别顺手做」的提醒

- **别用 `opacity` 代替换色**（`.opacity(0.45)`）：数字见 §1.4，两主题落点不一致，且与库内其它禁用行不同步。
- **别顺手改图标尺寸**：`Icon::from_path` 默认档位是 `IconSize::Md` = 16px（`primitives/icon.rs:424` + `theme/tokens.rs:141`），而菜单图标槽是 `icon_size(IconSize::Sm)` = 14px（`menus/draw.rs:75` + `tokens.rs:140`）—— 差 2px，是既有状态。**S5 不要动**：一改就是全部菜单行的图标档位变化，要单独确认（可记为后续单，与 S1 图标补齐同批）。

---

## 5. 与既有组件的一致性（核实结果）

| 组件 | 禁用怎么表达 | 出处 |
| --- | --- | --- |
| **菜单行** | 文字/图标 → `fg_disabled`；不画 hover 底；不设 `cursor_pointer` | `menus/draw.rs:21-25 / 37-38 / 41-45 / 67-68` |
| 浮层选项行（下拉） | 文字 → `fg_disabled`；`highlighted && !disabled` 才画 `chrome_hover` | `forms/options.rs:124-141` |
| 浮层链接行（带图标 + 说明文字） | **主色与次要色一起**落 `fg_disabled`：`(fg, muted) = if disabled { (fg_disabled, fg_disabled) } else { (fg, fg_muted) }` | `navigation/menu.rs:184-188` ← **§3.5 「次要槽一起变淡」的库内先例** |
| 输入框 | 文字 → `fg_disabled`，底 → `sunken` | `forms/input.rs:136-146` |
| 下拉触发器文字 | → `fg_disabled` | `forms/select.rs:125-126` |
| 开关 / 复选 / 单选 / 筹码 / 标签页 | → `fg_disabled` | `forms/switch.rs:141`、`forms/check.rs:158`、`forms/radio.rs:124`、`forms/chips.rs:90`、`navigation/tabs.rs:367` |
| 菜单栏触发器（workbench 五态表） | `TriggerFace::Disabled` = `(fg_disabled, None, None)` | `menu_bar.rs:2035-2037`（测试） |
| **按钮**（唯一例外） | 整元素 `.opacity(0.45)` + `.cursor_not_allowed()` + aria 描述 | `buttons/button.rs:493-499` |

**结论**：库内禁用态的主流口径是「换前景色到 `fg_disabled` + 撤掉交互反馈」，`opacity` 只有 `Button` 一家。菜单行本来就在这条主流上，S5 只要**补齐自画图标的取色**、**让键帽文字跟着行一起降档**，就与全库一致，不需要创造新口径 —— 尤其 `navigation/menu.rs:184-188` 已经写了「主色与次要色在禁用时**一起**落 `fg_disabled`」，§3.5 的口径就是照它抄的。

另外一条必须守住的一致性（workbench 自己的规矩，`menu_bar.rs:1950-1954` 注释明文）：

> 「未接线」行用 `fg_subtle` **且保持可点**；「现在不可用」才用 `fg_disabled`。

S5 属于后者。**不要把两类行画成一个样**。

---

## 6. 实现清单（照做）

1. 给 workbench 的 `MenuItem::Action` 一行加 `disabled` 判定 —— 判定表以架构师的 `MenuAction::is_enabled(ctx)` 为准（`reports/menu-disable-arch.md` §2.2），主理人最初列的 8 条只是其中的「无缓冲」最小集。
2. `menu_bar.rs:1902-1908`：建行时 `MenuRow::new(...).disabled(disabled)`；`icon_element(icon, disabled)`；**`keys` 照设**（变淡交给库）。
3. `menu_bar.rs:1996-1999`：`icon_element` 加 `disabled` 参数并按 §4.3 取色（`fg_disabled` / `fg`，启用态保持 `fg` 不变）。
4. `menu_bar.rs:1989`（主题子菜单行）：`icon_element(..., false)`。
5. 状态变化时让**外壳**重画即可（`render_bar` 跑在外壳的 `render` 里；编辑窗格的 notify 已被 `workspace.rs:1054-1057` 的 `observe` 转发）—— **不要在 `MenuBar` 上挂 `cx.observe(&editor, ..)`**，那会让子组件攥着编辑窗格句柄（架构师 §3.2，与 2026-10-10 那篇笔记的决策八冲突）。
6. **库侧唯一一处（§3.5）**：`typography/inline.rs` 给 `KbdCombo` 加默认关闭的 `.disabled(bool)` —— 开启时键帽文字（`:55`）与「+」（`:120`）取 `colors.fg_disabled`，键帽底/描边不动；`menus/draw.rs:91-99` 传 `item.disabled`。**除菜单行以外的 5 处生产调用与画廊 4 处零变化**；哨兵是 `src/shell/tests.rs:178`。
7. **不动**：`IdeaIcon` 常量表、主题配色、`assets/icons/**`、`fg_disabled` 本身、`menu_key()` 这张表。

主理人最初列的那 8 行（已核实，免得实现同学再查）：

| 动作 | 图标（`menu_bar.rs`） | `menu_key()` | 禁用时要做 |
| --- | --- | --- | --- |
| Save | `FLOPPY_DISK_ICON`（`:496`） | 无 | 文字 + 图标置灰 |
| SaveAs | `Empty`（`:539`） | 无 | 只文字置灰 |
| SaveAll | `Empty`（`:539`） | 无 | 只文字置灰 |
| RevertFile | `REFRESH_ICON`（`:540`） | 无 | 文字 + 图标置灰 |
| CloseTab | `X_ICON`（`:497`） | 无 | 文字 + 图标置灰 |
| Undo | `ARROW_COUNTER_CLOCKWISE_ICON`（`:509`） | 无 | 文字 + 图标置灰 |
| Cut | `SCISSORS_ICON`（`:511`） | 无 | 文字 + 图标置灰 |
| Find | `MAGNIFYING_GLASS_ICON`（`:513`） | **有** `KEY_FIND`（`:580`） | 文字 + 图标置灰 **+ 键帽文字一起降档** |

（**`MenuIcon::Empty`** 的行不调 `icon_element`，图标槽留空 —— `menu_bar.rs:1903` 的 `if let MenuIcon::Idea(..)` 已经处理，行不会跳位。）

**但 §3.5 真正要验的是这 10 行**（架构师按 `is_enabled` + `menu_key()` 复核出来的、置灰且带键位胶囊的全集 —— 上面那张表只覆盖了其中 1 行）：

`GoToLine`（`menu_bar.rs:575`）、`Find`（`:580`）、`FindAndReplace`（`:581`）、`ToggleComment`（`:586`）、`DuplicateLine`（`:587`）、`DeleteLine`（`:588`）、`MoveLineUp`（`:589`）、`MoveLineDown`（`:590`）、`NextTab`（`:593`）、`PreviousTab`（`:594`）。

这 10 行在无缓冲（或标签页不足）时置灰，**键位胶囊必须随行降到 `fg_disabled`** —— 视觉验收请至少抽查 `GoToLine`、`Find`、`FindAndReplace`、`NextTab` 四条（前三条键位长、第四条是三键和弦，最能暴露「键帽比标签显眼」）。

---

## 7. 验收清单

**视觉（两主题各过一遍）**

- [ ] 禁用行文字 = `fg_disabled`：实测对面板底 **浅 2.26:1 / 深 2.71:1**（±0.05）。
- [ ] 禁用行图标与文字**同色**（都是 `fg_disabled`），不再出现「灰字 + 深色图标」。
- [ ] 鼠标划过禁用行：**无** hover 底（浅 `#d9e5f2` / 深 `#3c3f41` 都不出现）。
- [ ] 光标在禁用行上是**默认箭头**（既不是 pointer 也不是 not-allowed）。
- [ ] 禁用行的键位胶囊**还在**，但键帽文字与行文字**同档**：键帽文字对键帽底 **浅 2.38:1 / 深 2.89:1**，与行文字（2.26 / 2.71）差值 < 0.2 —— 不再出现「⌘F 比 Find 还清楚」。抽查 §6 列出的 **10 行**（至少 `GoToLine` / `Find` / `FindAndReplace` / `NextTab`）。
- [ ] 键帽的**底与描边没变**（仍是 `surface` + `border_strong`）—— 只降了文字。
- [ ] 除菜单行以外的 **5 处生产 `KbdCombo`**（设置快捷键清单 / 录制框 / 命令面板 / 欢迎页 / 键位表）与画廊 4 处**观感一字不变**（`.disabled(..)` 默认关闭）；`cargo test -p lithe-ui` 过，尤其 `src/shell/tests.rs:178` 那条哨兵。
- [ ] 启用行的图标色与改动前**逐像素一致**（非禁用分支显式给 `fg`，等于 `Icon::render` 的回落值）。
- [ ] 无活动缓冲时：Save / SaveAs / SaveAll / RevertFile / CloseTab / Undo / Cut / Find 全置灰；打开一个缓冲后全恢复。
- [ ] 「未接线」行仍是 `fg_subtle` 且**可点**，没被误置灰。

**交互**

- [ ] 方向键在菜单里跳过置灰行；回车/点击置灰行无任何反应。
- [ ] 菜单开着的时候打开/关闭缓冲，置灰状态**当场**跟着变（第 6.5 条的 notify）。

**可复现位置**

- 库的禁用行画廊已有两条：`examples/gallery/pages/menus/rows.rs:77`（`MenuItem::new("Revert").disabled(true)`）与 `:220-224`（`Forward`）；两条都**没带 keys**，所以「禁用行 + 键位胶囊」这个组合在画廊里至今没被展示过。**本单改了 `KbdCombo`，就请顺手在 `pages/menus/rows.rs` 的 File 菜单里补一条带 `keys` 的禁用行**（例如 `MenuItem::new("Save").keys("secondary-s").disabled(true)`），两主题各看一眼 —— 这是 §3.5 唯一的可视证据。
- 主题色卡里也有 `fg_disabled` 一块：`examples/gallery/pages/theme/mod.rs:49`（可用来肉眼确认该色在新主题下的样子）。

---

## 8. 不做的事（本单边界）

- ✅ **唯一允许的库改动**：`typography/inline.rs` 的 `KbdCombo` 加默认关闭的 `.disabled(bool)` + `menus/draw.rs:91-99` 传 `item.disabled`（§3.5）。除此以外 `rust/lithe-ui` 一律不动。
- ❌ 不新增图标、不碰 `IdeaIcon` 常量表（那是 S1/S3）。
- ❌ 不改主题配色、不改 `themes/jetbrains.json`、不改 `theme.rs:459` 的推导（§2.3 另开单）。
- ❌ 不给 `MenuItem` 加 `danger` / `tone` 通道（那是另一条，`editor-basics-visual-spec.md:301-307`）。
- ❌ 不补 aria（§3.9）。
- ❌ 不改图标档位 16px → 14px（§4.4）。

---

## 附录 A：色值与算法

- 对比度算法 = 库内同一套：`rust/lithe-ui/src/devtools/contrast.rs:14-34`（WCAG 2.x 相对亮度；`over()` 在 sRGB 里按 alpha 合成，背景当不透明）。本文所有数字按此公式手算，与 `contrast.rs` 一致。
- 未走 Rust 运行时验证（本次不跑应用/不截图，按主理人要求），但公式与色值全部来自代码里核实的常量。
- 合成色：`#6e6e6e` @0.6 over `#f2f2f2` ≈ `#a3a3a3`；`#9aa0a6` @0.6 over `#313335` ≈ `#707479`。
- 宿主其余相关值：`foreground` 浅 `#080808` / 深 `#a9b7c6`（`jetbrains.json:15 / 141`）；`surface` = `background` 浅 `#ffffff` / 深 `#2b2b2b`（`theme.rs:361-365` 把 `surface` 赋成 `background`，键帽底读的就是它）；`border` 浅 `#c8c8c8` / 深 `#555555`（`:16 / :142`，`border_strong` 同值，`theme.rs:449-452`）。

## 附录 B：未核实 / 有保留的地方

- **没有截图**（本单不跑应用）。「禁用行看起来淡到什么程度」是色值 + 对比度推出来的；主观是否「够淡」建议实现后用 gallery 的 menus 页（`examples/gallery/pages/menus/rows.rs:77`）肉眼确认一次。
- **16px 图标画在 14px 槽里的实际观感未核实**（几何数字已核实：`draw.rs:75` + `tokens.rs:140` vs `icon.rs:424` + `tokens.rs:141`），是否溢出/被裁没有截图证据。
- gpui 的 SVG alpha-mask 结论取自 `gpui-pre 0.3.8` 依赖源（`src/elements/svg.rs:182-186`、`src/svg_renderer.rs:231-252`；crate 在 `Cargo.toml` 里钉 `=0.3.8`，路径在 cargo registry 内，不写进仓库）。若将来升 gpui 版本，这条要复查。
- 「未接线行不用 `fg_disabled`」是 workbench 的既有决定（`menu_bar.rs:1950-1954` 注释），本文沿用，未重新评审。
