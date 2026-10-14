# S5 菜单动态 `disabled` 通道 —— 架构设计

> 作者：闻本立（editor-architect）｜状态：**设计稿，待主理人拍板**｜本文只做设计，不含实现代码。
> 所有结论都给了 `文件:行号`；凡未能在代码里核实的，一律标「未核实」。
> **配套文档**：`reports/menu-disable-visual.md`（颜思齐）—— 视觉口径以它为准，
> 本文 §2.4 / §2.5 已按其结论修订（v2）。
>
> **v3（2026-10-14）**：主理人拍板 —— ① `can_go_back` / `can_go_forward` 从 P1 **提入本批**
> （`EditorContext` 五字段 → **七字段**）；② `KbdCombo::disabled` 库改动**进本批**。
> 涉及 §0-3、§1.2-1.4、§2.2、§5、§7（新增 A19 / A20）、§8、§11。
>
> v3 里我自己抓到并修掉的一处：`GoBack` / `GoForward` **不能**只听 `can_go_*`，
> 必须与 `has_active_buffer` 相与 —— 历史在关掉标签页后仍残留，而 `go_back` / `go_forward`
> 都要先取「当前位置」（`current_entry` 以 `self.active?` 开头，`editor_view.rs:1720-1728`）。

---

## 0. 先说结论

1. **菜单表保持静态，只给每一行现算一个 `disabled` 标志。** 不裁剪行、不把 `MENUS` 改成函数。
   谓词写成 `MenuAction::is_enabled(&EditorContext) -> bool`，与 `label_key()` / `icon()` /
   `menu_key()` / `keybind_action()` / `command_id()` 这五张既有映射同处一 `impl`。
2. **上下文用「每帧现取一份标量快照 + 显式参数传递」**，不放 thread-local 全局。
   快照由 `ShellWorkspace::render` 在调用 `render_bar` 之前现取，一路传到 `build_menu`。
3. **本批只判六件事**：有没有活动缓冲 / 有几个标签页 / 活动缓冲是不是磁盘文件 /
   能不能写回 / **跳转历史能不能后退 / 能不能前进**。
   后两条是主理人 2026-10-14 拍板从 P1 提进来的（`navigation.rs:292/297` 代码现成），
   且**必须与 `has_active_buffer` 相与**——只听 `can_go_*` 会画出「亮着但点了没反应」的死行（§2.2、A19）。
   dirty、选区、剪贴板、查找条开关、调试会话、已关标签页栈、终端标签页数
   **全部降级为常亮**，理由见 §1.4。
4. **子菜单父项不置灰**（含 compact 面板里那九个顶层项）。空的分组沿用既有做法「整行不出现」。
5. **只改菜单，不改快捷键、不改命令面板。** 执行侧保留「no-op + 诊断」作双保险（裁决 #7）。
6. **库侧 `lithe-ui` 只有一处小改**：给 `KbdCombo` 加一个**默认关闭**的 `.disabled(bool)`
   开关（`typography/inline.rs`），菜单侧接线（`menus/draw.rs:91-99`）。
   原因是键帽颜色硬写在 `cap()` 里（`inline.rs:55` 取 `fg_muted`），
   父级 `text_color` 传不进去 ⇒ 禁用行的「⌘S」会比它自己的标签更显眼。
   **图标槽不动库**（宿主自画、宿主上色）。宿主 → 库 单向依赖不变，
   Core 契约与 `shared/contracts` 不动。
7. **禁用行的光标是默认箭头**，既不是 pointer 也不是 `not-allowed`
   （库已保证：`draw.rs:68` 只在非禁用行上 `cursor_pointer()`）。

---

## 1. 启用上下文模型

### 1.1 先纠正一处场景描述

主理人任务书里写的「刚启动停在欢迎页」**不是本批的实际触发场景**：欢迎页是一个**独立窗口**，
由 `lithe-gpui/crates/app/src/main.rs:736-741` 用 `WindowRoot::new(welcome, ...)` 建，
`welcome.rs` 里根本没有菜单栏（grep `menu_bar` 零命中）。

真正的触发场景是**工作区窗口里一个标签页都没打开**：刚打开一个还没开过文件的项目，
或把最后一个标签页关掉。此时 `EditorPane.buffers` 为空、`active == None`
（`editor_view.rs:1966-1975`：`buffers.is_empty()` 与 `active.is_none()` 严格等价）。

### 1.2 状态在代码里的真实位置

| 状态 | 现在在哪 | 能否在构建菜单时低成本取到 | 依据 |
| --- | --- | --- | --- |
| 有无活动缓冲 | `EditorPane.active: Option<usize>`（私有） | **能**，需新增 1 个 `pub fn` | `editor_view.rs:230`；`active_buffer_name()` 返回 `String`（`972`）能凑合但不能用，见下 |
| 标签页数量 | `EditorPane.buffers: Vec<Buffer>`（私有） | **能**，需新增 1 个 `pub fn` | `editor_view.rs:229` |
| 活动标签页下标 | 同 `active` | **能**，同上 | `editor_view.rs:230` |
| 活动缓冲是磁盘文件 | `EditorPane::active_disk_file()` **已公开** | 能，但它 `clone()` 一个 `PathBuf`（`1009-1018`） | 建议另加一个不克隆的 `has_active_disk_file()` |
| 活动缓冲能否写回 | `EditorPane::active_buffer_is_readonly()` **已公开** | 能，`&self` 纯读 | `editor_view.rs:990-995`（无活动缓冲时返回 `true`） |
| 活动缓冲是否 dirty | `Buffer.is_dirty`（`pub(crate)`） | **拿不到**，需新增访问器 | `buffer.rs:213`；跨 crate 不可见 |
| 是否有选区 | `CodeEditor`（`lithe-ui`）内部 | **拿不到且代价高**（要库侧新开 API） | 未核实具体 API 面 |
| 剪贴板是否有内容 | gpui 剪贴板 | 能取但**是同步平台调用**，不适合每帧 | 未核实 |
| 撤销栈是否为空 | `CodeEditor`（`lithe-ui`）内部 | **拿不到** | 未核实 |
| 查找条是否打开 | `EditorPane.find: Option<FindBar>`（私有） | 能，但本批**用不上**（见 §1.3） | `editor_view.rs:276` |
| 跳转历史是否为空 | `JumpHistory::can_go_back/forward`（`pub(crate)` + `#[allow(dead_code)]`） | **本批做**：提成 2 个 `pub fn` 并删 `#[allow(dead_code)]`（主理人 2026-10-14 已批准纳入） | `navigation.rs:292`、`297` |
| 已关闭标签页栈 | `EditorPane.closed: Vec<PathBuf>`（私有） | 需新增 1 个 `pub fn` | `editor_view.rs:250`；只在磁盘缓冲关闭时入栈（`1956-1960`） |
| 终端标签页数 | `TerminalPane` | **无公开 API** | `terminal/src/session.rs` 只有 `new_tab` / `close_active_tab -> bool`（`253`、`279`） |
| 调试会话状态 | `DebugPane` | 本批不判 | `workspace.rs:552` |

### 1.3 字段表（本批真正要的）

```rust
// rust/lithe-gpui/crates/workbench/src/menu_bar.rs

/// 菜单行「此刻能不能点」的判定上下文。
///
/// 只装**标量**：它每帧被现取一次，然后按值传进菜单构建链。不放实体句柄、不放
/// `PathBuf`、不放任何需要 `&mut App` 的东西 —— 构建链跑在 `ShellWorkspace::render`
/// 里，那里不允许再 `update` 任何实体（见 §6 风险 R2）。
///
/// 不变量（invariants）：
///   1. `has_active_buffer == (tab_count > 0) == active_tab_index.is_some()`
///      —— 由 `EditorPane` 自己的 `close()` 保证（`editor_view.rs:1966-1975`）。
///   2. `has_active_disk_file == true` ⇒ `has_active_buffer == true`。
///   3. `active_writable` 只在 `has_active_buffer == true` 时有意义；否则恒 `false`。
///   4. `can_go_back` / `can_go_forward` 与 `has_active_buffer` **互不蕴含**，判定时必须
///      **两个都看**（见 §2.2 的 `GoBack` / `GoForward`）。原因是历史在关掉标签页后仍然
///      残留（`JumpHistory.entries` 只在导航时增长，`editor_view.rs:1955-1960` 的 `close()`
///      不清它），所以「有历史但没开文件」是真实可达状态；而 `go_back` / `go_forward`
///      第一句都是 `let Some(present) = self.current_entry(cx) else { return }`，
///      `current_entry` 又以 `let index = self.active?` 开头（`editor_view.rs:1720-1728`）
///      ⇒ 没有活动缓冲时两者必然 no-op。只听 `can_go_*` 会画出「亮着但点了没反应」的死行。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditorContext {
    /// 有没有活动缓冲（= 有没有打开的标签页）。
    pub has_active_buffer: bool,
    /// 活动缓冲是一个**磁盘**文件，且不是虚拟源路径（未命名 scratch 不算）。
    pub has_active_disk_file: bool,
    /// 活动缓冲能写回磁盘（非二进制 / 非有损转码 / 非缺失 / 非超限）。
    pub active_writable: bool,
    /// 打开的标签页数。
    pub tab_count: usize,
    /// 活动标签页下标；`None` 表示没有标签页。
    pub active_tab_index: Option<usize>,
    /// 跳转历史能不能后退（Go ▸ Back）。
    pub can_go_back: bool,
    /// 跳转历史能不能前进（Go ▸ Forward）。
    pub can_go_forward: bool,
}
```

构造与兜底：

```rust
impl EditorContext {
    /// 从编辑窗格现取一份。**只读**（五个 `&self` 取值器），不 notify、不触碰别的实体。
    pub fn capture(pane: &EditorPane) -> Self;

    /// 没有外壳时的兜底：**一律可用**，与改造前的行为完全一致。
    /// 刻意不做「默认全灰」——默认值若取全灰，任何一条没接上上下文的调用路径
    /// （测试、未来的第二入口）都会静默地把菜单全灰掉，那比不灰更难查。
    pub const fn permissive() -> Self;
}
```

### 1.4 明确不做的（降级为常亮）

| 候选字段 | 结论 | 理由 |
| --- | --- | --- |
| 缓冲是否 dirty（用于 Save / RevertFile） | **不做** | ① `Buffer::is_dirty` 是 `pub(crate)`（`buffer.rs:213`），要新开 API；② IDEA 与 VS Code 在「干净」时都**不**置灰 Save —— 置灰会让用户找不到这个动作；③ 裁决 #7 的 no-op 语义已经能安全兜住 |
| 撤销栈是否为空（Undo / Redo） | **不做** | 库侧 `CodeEditor` 未开 API（未核实其面）；无缓冲时已经由 `has_active_buffer` 覆盖，有缓冲时置灰撤销栈的收益远小于成本 |
| 是否有选区（Cut / Copy） | **不做** | 同上；且 Cut/Copy 在有缓冲时本来就是「合法 no-op」 |
| 剪贴板是否有内容（Paste） | **不做** | 读剪贴板是同步平台调用，放在每帧的菜单构建里不合适 |
| 查找条是否打开（Find / FindAndReplace） | **不做** | 语义不对：Find 的价值是「打开查找条」，与查找条当前是否打开无关 |
| 调试会话（Start / Stop Debugging） | **不做** | 依赖会话状态，另开单 |
| dirty 用于 SaveAll | **不做**（主理人裁为后续单） | 需要 `any_dirty()`；本批按 `tab_count > 0` 宽松判定 |
| 已关闭标签页栈（`ReopenClosedTab`） | **不做**（后续单） | 需要 `has_closed_tabs()`；本批常亮 |
| 终端标签页数（`CloseTerminalTab`） | **不做**（后续单） | `TerminalPane` 无 `tab_count()` 之类的公开 API（`session.rs` 只有 `new_tab` / `close_active_tab -> bool`） |

---

## 2. 静态表 → 动态表：方案与取舍

### 2.1 三个候选

| 方案 | 做法 | 优点 | 代价 | 结论 |
| --- | --- | --- | --- | --- |
| **(a) 静态表 + 每行现算 `disabled`** | `MENUS` 不动；新增 `MenuAction::is_enabled(&EditorContext) -> bool`；`build_menu` 里 `row.disabled(!action.is_enabled(&ctx))` | 表仍是 `&'static [MenuItem]`；行下标稳定；既有 15 个 `MENUS` 测试全不动；六个映射同处一 `impl`，加新变体时 `match` 穷尽性检查会编译报错 | 谓词集中在一处大 `match`（约 40 行） | **推荐** |
| (b) 表改成函数，按上下文裁剪/置灰 | `TopMenu.items: fn(&EditorContext) -> Vec<MenuItem>` | 表达力最强 | ① `MENUS` 不能再是 `static`，`action_count()`（`1030`）/ `not_wired_count()`（`1038`）/ `lookup_action()`（`1457`）与全部 `MENUS` 测试要改；② **裁剪会让行下标漂移**，而库的 `Open.levels` 存的就是下标（`menu.rs:53` `Level.at: Option<usize>`），键盘标记会错位；③ 「置灰」比「裁剪」信息量大 | 否决 |
| (c) 静态表 + 独立谓词表 | `const ENABLED: &[(MenuAction, fn(&EditorContext) -> bool)]` | 谓词可复用 | 两张表要手工对齐；`MenuAction` 已有五张映射都写成 `impl` 上的 `match`，再开一张表是第二个真源；没有穷尽性检查 | 否决 |

### 2.2 推荐方案 (a) 的接口签名

```rust
impl MenuAction {
    /// 这一行在 `ctx` 下能不能点。**只影响菜单置灰**，不改变执行路径：
    /// 键位与命令面板仍会走到 `apply_menu_action`，那里保持「no-op + 诊断」（裁决 #7）。
    ///
    /// 与 `menu_key()`（这一行**有没有**快捷键）刻意解耦：前者是静态事实（绑没绑键），
    /// 后者是动态事实（此刻能不能用）。混进一张表要么让「有键但此刻不可用」画不出胶囊，
    /// 要么让人误以为胶囊在就是可按的。
    pub fn is_enabled(self, ctx: &EditorContext) -> bool {
        match self {
            // —— 只要有活动缓冲即可 ——
            Self::SaveAs
            | Self::CloseTab
            | Self::GoToLine
            | Self::GoToDefinition
            | Self::Undo
            | Self::Redo
            | Self::Cut
            | Self::Copy
            | Self::Paste
            | Self::SelectAll
            | Self::Find
            | Self::FindAndReplace
            | Self::ToggleComment
            | Self::QuickFix
            | Self::ShowHover
            | Self::DuplicateLine
            | Self::DeleteLine
            | Self::MoveLineUp
            | Self::MoveLineDown => ctx.has_active_buffer,

            // —— 有活动缓冲 **且** 能写回（二进制 / 有损转码的文件写不回去）——
            Self::Save => ctx.has_active_buffer && ctx.active_writable,

            // —— 必须是一个磁盘文件（未命名 scratch 不能还原、不能在磁盘行上打断点）——
            Self::RevertFile | Self::ToggleBreakpoint => ctx.has_active_disk_file,

            // —— 标签页数 ——
            Self::SaveAll | Self::CloseAllTabs | Self::CloseSavedTabs => ctx.tab_count > 0,
            Self::CloseOtherTabs | Self::NextTab | Self::PreviousTab => ctx.tab_count > 1,
            Self::CloseTabsToLeft => matches!(ctx.active_tab_index, Some(ix) if ix > 0),
            Self::CloseTabsToRight => matches!(ctx.active_tab_index, Some(ix) if ix + 1 < ctx.tab_count),

            // —— 跳转历史 ——
            // **两个条件都要**：只听 `can_go_*` 会画出「亮着但点了没反应」的死行。
            // 历史在关掉标签页后仍然残留，而 `go_back` / `go_forward` 都要先取「当前位置」，
            // `current_entry` 以 `self.active?` 开头（`editor_view.rs:1720-1728`）
            // ⇒ 没有活动缓冲时两者必然静默 return（`1636`、`1647`）。
            // `GoForward` 额外还要能落到目标缓冲，关了的话 `restore_entry` 会以
            // `reason=buffer-closed` 退出（`1664-1678`）——那属于执行侧的 no-op，
            // 本批不把它提前到置灰判定里（判它需要把整条历史搬到上下文里，太贵）。
            Self::GoBack => ctx.has_active_buffer && ctx.can_go_back,
            Self::GoForward => ctx.has_active_buffer && ctx.can_go_forward,

            // —— 其余一律可用：打开/新建类、界面开关、全局搜索、窗口、帮助 ——
            _ => true,
        }
    }
}
```

**不置灰的清单（必须保持可用）**：`NewTab`（`workspace.rs:1941-1945` 建 scratch，不需要活动缓冲）、
`NewWindow`、`NewFile`（`workspace.rs:1946-1948` 走对话框建文件）、`OpenFolder`、`OpenFile`、
`CommandPalette`、`QuickOpen`、`ReplaceInFiles`、`SearchEverywhere`，以及 View / Window / Help 全组
（都是界面开关或全局能力，与缓冲无关）。

**未接线行（`MenuItem::NotWired`）永不置灰** —— `menu_bar.rs:1951-1954` 已经写明理由：
「还没做」不是「现在不能用」，而且 `Menu::step()` 会跳过 `disabled` 行（`model.rs:198`），
置灰等于把它们的键盘可达性一起丢掉。本批必须守住这条。

### 2.3 `build_menu` 的改造骨架

```rust
// menu_bar.rs
fn build_menu(index: usize, ctx: EditorContext, cx: &mut App) -> Menu {
    ...
    MenuItem::Action(action) => {
        let action = *action;
        let disabled = !action.is_enabled(&ctx);       // ← 唯一的增量
        let row_handle = handle.clone();
        let mut row = MenuRow::new(action.label());
        if let MenuIcon::Idea(icon) = action.icon() {
            // 图标槽是宿主自画的，库不会替它上禁用色（model.rs:94-95 的注释），
            // 所以颜色必须宿主自己按状态给。
            row = row.icon_element(icon_element(icon, disabled));
        }
        if let Some(keys) = action.menu_key() {        // ← 键位胶囊照旧，与 disabled 无关
            row = row.keys(keys);
        }
        menu.item(
            row.disabled(disabled)                     // ← model.rs:107 已有
               .on_click(move |_window, cx| { ... }),  // ← 不用判：库已经不给禁用行挂点击
        )
    }
```

`on_click` 是否还要判？**不判。** 库侧两处已经拦住：绘制时不给禁用行挂 `on_mouse_down`
（`draw.rs:145` 的 `.when(!item.disabled, ...)`），键盘 `choose()` 里也有 `if item.disabled { return }`
（`menu.rs:227`）。宿主再判一次是第二个真源。断言层验「点了无副作用」即可（§7 A5）。

### 2.4 图标槽的禁用色

`idea_icon_mono()`（`shared/src/icons/mod.rs:159-162`）返回的是 `Icon::from_path`，
`color: None` ⇒ 渲染时落到 `theme.colors.fg`（`lithe-ui/src/primitives/icon.rs:506`），
**不会继承父行的 `text_color`**。所以宿主必须显式给色：

```rust
fn icon_element(
    icon: &'static idea::IdeaIcon,
    disabled: bool,
) -> impl Fn(&App) -> AnyElement + 'static {
    move |cx: &App| {
        let theme = cx.theme();
        // 启用态**保持现状**（`fg`）：库自己那批自画图标用的是 `fg_muted`（draw.rs:44），
        // 两套口径统一是另一笔独立视觉变更，颜思齐已裁定不与本批混做（与 S1 图标补齐同批另开）。
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

`Icon::color()` 对宿主自画的 IDEA 图标**确实生效**，两条代码级证据（颜思齐另给了
gpui-pre `elements/svg.rs` / `svg_renderer.rs` 的底层证据，**我未核实**那层）：
① `Icon::render` 把颜色落在 `svg()` 的 `text_color`（`icon.rs:531`）；
② 库自己的行图标就是靠 `.color(lead_color)` 上色的（`draw.rs:41-56`），
而 `shared::icons::idea_icon_svg_px` 也用同一个机制（`icons/mod.rs:181-187`）。
所以**不需要换图、不需要新增图标**，文件里那个 `stroke="#6C707E"` 不参与。

调用点：`build_menu` 建 `Action` 行时传 `disabled`（`menu_bar.rs:1902-1908`），
主题子菜单那一行（`theme_menu`，`1989`）传 `false`。
宿主在建行时**本来就知道状态**，不用改库的 `Build` 签名 —— 这块**不动库**。

### 2.5 键位胶囊（`menu_key()`）的处理

- `menu_key()` / `keybind_action()` / `command_id()` **一行都不改**。
- 禁用行**照旧画胶囊**：快捷键的存在是静态事实，置灰不撤掉它（IDEA / VS Code 同口径）。

**有一处观感必须处理，而且落点不是我原先以为的地方（已由颜思齐核实、我复核确认）。**

我第一版写的是「`draw.rs:96` 把 `fg_subtle` 换成 `fg_disabled`」——**那条是死样式，改了没用**：

- 键帽不是 `draw.rs:96` 画的。`KbdCombo::render`（`lithe-ui/src/typography/inline.rs:99-127`）
  把每个键交给 `cap()`（`inline.rs:41-57`），而 `cap()` **硬取**主题色：
  描边 `border_strong`（`:51`）、底 `surface`（`:52`）、**文字 `fg_muted`（`:55`）**。
  父级 `text_color` 传不进去。
- `draw.rs:96` 的 `fg_subtle` 只影响**非 Mac 平台键间的「+」**（`inline.rs:107`、`:120`）
  ——在 macOS（本项目当前参考平台）上它根本不出现。

所以「禁用行上快捷键比标签还显眼」的**真凶是 `cap()` 的 `fg_muted`**，不是 `fg_subtle`：
宿主主题里 `fg_muted == fg_subtle == muted.foreground`
（`lithe-gpui/crates/settings/src/theme.rs:457-458`），而
`fg_disabled = muted.foreground @ α0.6`（`theme.rs:459`）——胶囊文字比行文字实一档。

**正确的落点（本批唯一允许的库改动，共 3 处）：**

1. `lithe-ui/src/typography/inline.rs`：`KbdCombo` 加一个**默认关闭**的开关
   `.disabled(bool)`（与 `MenuItem::disabled` 同名同义；排版层若要中性名字 `.muted(bool)` 亦可）。
   开启时 `cap()` 的文字（`:55`）改取 `colors.fg_disabled`、非 Mac 的「+」（`:120`）取同一个值；
   关闭时与今天**一字不差**。
2. `lithe-ui/src/menus/draw.rs:91-99`：`KbdCombo::new(&keys).disabled(item.disabled)`。
3. `draw.rs:96` 顺手写成 `if item.disabled { fg_disabled } else { fg_subtle }`
   ——它本身是死样式，改它只为让文件读起来自洽，别让人再以为「键位的颜色在这一行」。

**不要动**键帽底 `surface`（`:52`）与描边 `border_strong`（`:51`）：
颜思齐实测键帽底对面板底仅 1.12:1、描边 1.50/1.70:1，形状本身很安静，抢眼的只有文字。

**验收数字（取自颜思齐 `reports/menu-disable-visual.md` §3.5，非我实测）**：
键帽文字改后 浅 2.38:1 / 深 2.89:1，与禁用行文字 2.26 / 2.71 **同档（差 < 0.2）**。

**影响面（我复核过）**：`KbdCombo` 在 `rust/lithe-ui/src/` 下共 **6** 处生产调用 ——
`menus/draw.rs:97`、`settings/shortcuts.rs:94`、`navigation/palette/kinds.rs:83`、
`editor/welcome.rs:410`、`editor/keybindings.rs:187`、`forms/hotkey.rs:101`
（另有画廊 `pages/typography/text.rs:107/112`、`pages/theme/platform.rs:88`、
`pages/interaction/keys.rs:88-89`、`shell/tests.rs:178`）。开关默认关闭 ⇒ 其余调用点零变化，
**但 PR 里必须点明这一点**，否则会被当成一次全仓改色来回归。

**回退方案（不要当首选）**：禁用行直接不设 `keys`（`menu_bar.rs:1906-1908` 加 `&& !disabled`）。
代价被低估了——**会受影响的不止 `Find` 一条**。在「无缓冲时置灰」的这批动作里，
`menu_key()` 有值的共有 **10 条**：`GoToLine`（`menu_bar.rs:575`）、`Find`（`580`）、
`FindAndReplace`（`581`）、`ToggleComment`（`586`）、`DuplicateLine`（`587`）、
`DeleteLine`（`588`）、`MoveLineUp`（`589`）、`MoveLineDown`（`590`）、
`NextTab`（`593`）、`PreviousTab`（`594`）。10 行在有/无缓冲之间宽度与信息量跳变，
比改 `KbdCombo` 更贵。这条只在 `KbdCombo` 开关不好加时才用。

---

## 3. 构建时机与刷新

### 3.1 现状：菜单**本来就是每次重画现建**

- 顶层面板只在**开着**时才建：`menu_bar.rs:1826-1830`（`if open { build_menu(..) } else { Menu::new() }`）；
  compact 面板同理（`1640`、`1688-1697`）。
- 所以「每次展开时构建」就是当前行为，**不存在「构建一次缓存」这个选项**——缓存反而是要新增的东西。

### 3.2 打开期间状态变了怎么办

`ShellWorkspace` **已经**订阅了编辑窗格：

```rust
// workspace.rs:1054-1057
let editor_subscription = Some(cx.observe(&editor, |shell: &mut Self, _, cx| {
    shell.schedule_session_save(cx);
    cx.notify();
}));
```

而 `EditorPane` 在「变脏」时会 notify（`editor_view.rs:1815-1818`：脏位翻转即 `cx.notify()`），
开/关标签页、切标签页也会 notify。`ShellWorkspace` 一重画，就走到 `render_bar`
（`workspace.rs:3479`）⇒ 面板重建 ⇒ 禁用态跟着更新。

> **给实现同学的一条要点**（颜思齐也点到了这条）：菜单开着时缓冲状态变化要想让禁用态跟着变，
> 需要的是**外壳重画**，不是「`MenuBar` 自己被 notify」。因为 `MenuBar` 的重渲染是被
> `ShellWorkspace::render` 里的 `render_bar` 驱动的（`workspace.rs:3479`），
> 而编辑窗格的 notify 已经被转发成外壳的 notify（`workspace.rs:1054-1057` 那条 `observe`）。
> **所以这里不需要新增任何订阅。** 不要去写
> `cx.observe(&editor, |bar, _, cx| cx.notify())` 挂在 `MenuBar` 上——
> 那是让子组件攥着编辑窗格句柄，既违反 2026-10-10 那篇笔记的决策八，也不会更快。

**结论：不做缓存。** 代价评估：

| 项 | 估算 |
| --- | --- |
| 单次 `build_menu` | 一个顶层菜单 ~13-24 行；每行 = 一次 `tr()` + 一个 `Rc` 闭包；**无 I/O、无分配**（除 `has_active_disk_file` 若不新增访问器会 clone 一个 `PathBuf`） |
| 最坏情况（compact 面板） | 九个顶层菜单全建，约 90-180 行 |
| 量级 | 微秒级，远低于一帧预算 |

**代价（写在明处）**：面板每帧重建会让库的 `Open.levels` 走一次 `settle()`（`menu.rs:34-49`）。
若被标记的那一行刚好从「可用」变成「禁用」，`settle` 会清掉那一层的标记——
**这正是期望行为**（不可用就不该被指着），不是副作用。

**如果将来真的要缓存**：在 `MenuBar` 上放 `built: Option<(EditorContext, usize, Menu)>`，
键 = `(ctx, index)`，`ctx` 变了才重建。**本批不做**——缓存失效规则比重建更容易出错，
而现在连一次性能问题都还没观察到。

### 3.3 快照的传递路径（显式参数，不放全局）

```
ShellWorkspace::render                        workspace.rs:3474
  └─ ctx = self.menu_context(cx)              ← 新增：只读取一份快照
  └─ render_bar(&self.menu_bar, ctx, window, cx)   workspace.rs:3479
       └─ menu_bar(bar, ctx, window, cx)      menu_bar.rs:1534
            └─ merged_bar / above_bar / autohide_bar / compact_bar
                 └─ menu_row(bar, ctx, cx)    menu_bar.rs:1563
                      └─ menu_item(bar, index, ctx, cx)   menu_bar.rs:1823
                           └─ build_menu(index, ctx, cx)  menu_bar.rs:1889
                 └─ compact_menu(ctx, cx)     menu_bar.rs:1688
```

调用点只有 `build_menu` 的两处（`menu_item`、`compact_menu`），**没有任何测试直接调它**
（`build_menu` 需要 `&mut App`，现有测试只迭代 `MENUS` 常量），改签名零风险。

### 3.4 为什么不用 thread-local 快照

| 维度 | 显式参数 | thread-local（如 `MENU_CONTEXT`） |
| --- | --- | --- |
| 多窗口 | 天然按窗隔离 | 多窗口时后渲染的窗会覆盖前一个，需要额外按窗分桶 |
| 顺序依赖 | 无（调用点必传） | 有：`render_bar` 之前必须有人 set 过 |
| 未初始化兜底 | 不存在这个问题 | 要定义「从来没人 set」时是灰还是亮 |
| 可测性 | `is_enabled` 是纯函数，单测直接构造 `EditorContext` | 需要 `with()` 包裹 |
| 与既有模式的贴合度 | `MenuBar` 已经从外壳拿 `pending` / `action_target` | 库里 `MENU_BAR` / `SHELL` 两个 thread-local 是**跨 crate 反向回调**用的，本例不需要 |

既有 thread-local（`command_palette.rs:19`、`menu_bar.rs:1484`）存在的理由是「库要回调宿主」，
而本例是「宿主把自己的数据递给自己的渲染函数」，用参数传就够了。

---

## 4. 子菜单与父项

**结论：父项不置灰。**

1. **置灰父项是行为改动，不只是视觉。** `settle()`（`menu.rs:34-49`，任务书里写的
   `fit_open_levels` 在代码里实际叫 `settle`）会清掉落在禁用行上的层级标记，
   已经展开的子菜单层级会被收起 ⇒ 禁用父项 = 把整组变成不可达。
2. **信息量。** 父项一灰，用户看不到「这组里还有几条能用」。Edit 在无缓冲时仍有
   `CommandPalette` 可用，灰掉它就是丢信息。
3. **compact 面板会整体失效。** compact 面板里九个顶层项**全是**子菜单（`menu_bar.rs:1688-1697`），
   父项一灰，汉堡按钮就变成装饰。
4. **无现实收益。** 九个顶层菜单里没有任何一个会出现「全部子项都不可用」——
   File 有 `NewTab/NewWindow/NewFile/OpenFolder/OpenFile`，Edit 有 `CommandPalette`，
   其余菜单全组都不依赖活动缓冲。
5. **IDEA 口径**：顶层/父级菜单项基本从不置灰。

**空分组怎么办？** 沿用 `MenuItem::Theme` 的既有做法——**整行不出现**（`menu_bar.rs:1936-1941`：
没有主题可选时那一行不进菜单），而不是置灰一个点不开的行。

---

## 5. 模块边界与依赖方向

```
lithe-core（不动）        shared/contracts（不动）
        ▲                        ▲
        │                        │
    lithe-ui ────────────────────┘        ← 可选 1 行：draw.rs:96
        ▲
        │   workbench → editor → lithe-ui → (lithe-core / shared)
   lithe-gpui/crates/editor                ← +5 个 pub fn 只读访问器
        ▲
   lithe-gpui/crates/workbench             ← 主体改动
        ▲
   lithe-gpui/crates/app（不动）
```

| 落点 | 改动 | 说明 |
| --- | --- | --- |
| `workbench/src/menu_bar.rs` | **主体**：`EditorContext` + `MenuAction::is_enabled` + `build_menu` 传参 + `icon_element` 带 `disabled` | 策略（谁能用）与它服务的 `MenuAction` 同处 |
| `workbench/src/workspace.rs` | **1 处**：`menu_context()` 取值 + `render_bar` 调用点加一个参数（`3479`） | 取数（状态在哪）归外壳 |
| `editor/src/editor_view.rs` | **+5 个 `pub fn`**：`tab_count()` / `active_tab_index()` / `has_active_disk_file()` / `can_go_back()` / `can_go_forward()` | 全是 `&self` 纯读，与既有 `soft_wrap_enabled()`（`394`）同风格；后两个是把 `navigation.rs:292/297` 从 `pub(crate)` 提上来，**要顺手删掉那两处 `#[allow(dead_code)]`** |
| `lithe-ui/src/typography/inline.rs` | **+1 个默认关闭的 builder**：`KbdCombo::disabled(bool)`（`:87-97` 加字段与 builder；`:55`、`:120` 取色） | 新增能力，**默认值即今天的行为** |
| `lithe-ui/src/menus/draw.rs` | **2 处接线**：`:97` 传 `.disabled(item.disabled)`；`:96` 顺手同步成 `fg_disabled`（死样式，只为自洽） | 不改依赖方向 |
| `lithe-core` / `shared/contracts` / `macos/` / `windows/` | **零** | 无跨端契约变化 |

**为什么不把 `EditorContext` 下沉到 `lithe-gpui-shared`？**
2026-10-10 那篇笔记（`.agents/notes/implemented/feature/2026-10-10-gpui-menu-bar-chrome-and-keymap-settings.md`
决策十）定过「两端都要用到的枚举放 `lithe-gpui-shared` 或 `settings::schema`」。
本例不适用：`EditorContext` ① 不持久化、② 不是跨端契约、③ **只有一个消费者**（`workbench::menu_bar`）。
下沉到 shared 反而会把「菜单能不能用」这条纯宿主策略变成一个跨端兼容面。

---

## 6. 风险清单

| # | 风险 | 影响 | 缓解 | 验证方式 |
| --- | --- | --- | --- | --- |
| R1 | 禁用行的键位胶囊比标签更显眼。真凶是 `cap()` 硬取 `fg_muted`（`typography/inline.rs:55`），**不是** `draw.rs:96`（那条是死样式，只影响非 Mac 的「+」）；宿主主题里 `fg_muted == fg_subtle`，`fg_disabled` 是它的 α0.6（`theme.rs:457-459`） | 观感错误：无缓冲时 File▸GoToLine 的「⌘G」比「转到行」还清楚 | `KbdCombo::disabled(bool)`（默认关）+ 菜单侧接线（§2.5）；**不要**动键帽底 `surface` 与描边 `border_strong` | 画廊 File 菜单补一条带 `keys` 的禁用行，亮/暗两套主题各看一眼（§7 A16） |
| R2 | 在 `ShellWorkspace::render` 里误用 `update`（不是 `read`）触碰别的实体 | gpui 运行时借用 panic：`cannot update X while it is already being updated`（`menu_bar.rs:1227-1241` 的注释、笔记决策八） | **设计上杜绝**：`menu_context()` 只读，签名收 `&self` / `&App`，不拿 `Context<Self>`；快照是 `Copy` 的标量，链路上没有任何地方需要 `update` | 起应用，无缓冲时反复开关九个菜单各 5 次 |
| R3 | 置灰判定与执行路径不一致（菜单灰了但 ⌘S 仍跑） | 用户困惑 | 本批接受，且**刻意保留** no-op + 诊断作双保险：即使 `is_enabled` 判错，也不会产生错误行为，只会少一层提示 | 断言 A6：⌘S 的诊断输出**不变**（验证没被顺手改坏） |
| R4 | 新增上下文后意外改变 `MENUS` 结构 | `SOURCE_MENU_ITEMS = 89`、`action_count()`、`not_wired_count()`、15 个 `MENUS` 测试全红 | 方案 (a) **不增删任何行**，这些数字天然不变；红就是守卫生效 | `cargo test -p lithe-gpui-workbench` |
| R5 | 有人把 `MenuItem::Action(MenuAction)` 改成带标志的 `Action(MenuAction, bool)` | 打破 `matches!(item, MenuItem::Action(_))`（`1034`、`2376`…）与 `label_key()`（`191-197`） | 明确**不推荐**这种形态；标志是运行期产物，不能进静态表 | code review |
| R6 | 键盘导航：某个菜单里**一个可用项都没有** | ↑↓ 无响应，看起来像卡住 | 现实中不发生（每个顶层菜单至少一条常亮项 + NotWired 行永不置灰）；但**要断言** | 断言 A11：无缓冲打开 Edit，`Menu::step` 的落点始终是 enabled 行 |
| R7 | 快照落后一帧 | 极端情况下菜单显示一次陈旧状态 | 快照在同一帧的 `render` 里现取，最坏延迟一帧；且菜单打开期间人不会同时改状态 | 断言 A7（打开一个文件后立刻重开菜单） |
| R8 | `has_active_buffer` 与 `tab_count` 两个字段可能互相矛盾 | 判定逻辑自相矛盾 | 由 `EditorPane::close()` 的不变量保证（`editor_view.rs:1966-1975`）；`EditorContext::capture` 里加 `debug_assert!` | 单测构造不变量 |
| R9 | 多窗口串味 | A 窗口的上下文用到 B 窗口的菜单上 | 显式参数传递 ⇒ 天然按窗隔离（这是选 A2 的理由之一） | 开两个项目窗，一边有文件一边没有，各查一次 File 菜单 |
| R10 | `has_active_disk_file` 若不新增访问器而复用 `active_disk_file()` | 每帧 clone 一个 `PathBuf` | 新增访问器（3 行），内部不 clone | review |
| R11 | 未接线行被误置灰 | 丢掉「未接入」提示的键盘可达性 | `build_menu` 的 `NotWired` 分支**不调用** `disabled()`；`is_enabled` 也不覆盖它 | 断言 A13 |

### 回归面（要跑的既有测试）

- `menu_bar.rs` 的 `mod tests`：`b2_actions_are_wired_and_only_bound_ones_carry_a_key`（`2265`）、
  `every_listed_item_is_wired_or_declares_what_is_missing`（`2490`）、`separators_are_between_items`（`2830`）、
  `theme_submenu_appears_exactly_once`（`2904`）等 —— 全部只依赖 `MENUS` 常量，方案 (a) 不动表 ⇒ 全绿。
- `keymap_engine.rs` 的 `NON_COMMAND` 副本 —— 不新增 `MenuAction` 变体 ⇒ 不受影响。
- `cargo test -p lithe-ui` —— 只有采用 §2.5 时才需要跑；重点是 `KbdCombo` 开关**默认值**
  不改变任何既有输出（`shell/tests.rs:178` 那条已经画了 `KbdCombo`，是最便宜的哨兵）。

---

## 7. 验收断言

**A. 纯单测（不需要 `App`，表驱动，与 `b2_actions_are_wired_...` 同风格）**

- **A1** `EditorContext` 的不变量：`has_active_buffer == (tab_count > 0) == active_tab_index.is_some()`，
  且 `has_active_disk_file ⇒ has_active_buffer`；`permissive()` 下所有 `MenuAction::is_enabled` 均为 `true`。
- **A2** 表驱动：`&[(MenuAction, EditorContext, bool)]`，至少覆盖
  `Save / SaveAs / SaveAll / RevertFile / CloseTab / CloseOtherTabs / CloseTabsToLeft / CloseTabsToRight /
  NextTab / ToggleBreakpoint / Undo / Find / GoBack / GoForward / CommandPalette / NewFile / OpenFolder`
  在「空」「1 个磁盘标签」「1 个未命名 scratch」「3 个标签且活动在 ix=0 / ix=2」五种上下文下的取值。
- **A19（七字段版新增）** 构造「**有跳转历史但没有活动缓冲**」（`can_go_back == true`、
  `has_active_buffer == false`）的上下文，断言 `GoBack` / `GoForward` **都置灰**。
  这是本批最容易写错的一条：历史在关掉标签页后仍然残留，而 `go_back` / `go_forward`
  都要先取「当前位置」（`current_entry` 以 `self.active?` 开头，`editor_view.rs:1720-1728`），
  没活动缓冲时必然静默 return（`1636`、`1647`）。只判 `can_go_*` 会留下「亮着但点了没反应」的死行。
- **A20** 反向：打开一个文件、跳一次定义（`JumpHistory` 入栈）后，`GoBack` 变可用、`GoForward`
  仍置灰；按一次 `GoBack` 后两者反过来。

**B. 集成 / 交互（起应用，肉眼 + 诊断日志）**

- **A3** 关掉最后一个标签页后打开 **File**：`Save / SaveAs / SaveAll / RevertFile / CloseTab /
  CloseOtherTabs / CloseAllTabs / CloseSavedTabs / CloseTabsToLeft / CloseTabsToRight` 十行置灰；
  `NewTab / NewWindow / NewFile / OpenFolder / OpenFile` 五行可用。
- **A4** 同状态打开 **Edit**：`Undo / Redo / Cut / Copy / Paste / SelectAll / Find / FindAndReplace /
  ToggleComment / DuplicateLine / DeleteLine / MoveLineUp / MoveLineDown` 全置灰；`CommandPalette` 可用。
- **A5** 同状态点 **File ▸ Save**：菜单不关、不产生 `S1_MENU_RUN`、不产生 `S1_EDITOR_SAVE`
  （**点击无副作用**），且该行没有 hover 底、光标不是 pointer。
- **A6** 同状态按 `secondary-s`（⌘S / Ctrl+S）：**仍**输出
  `S1_EDITOR_SAVE active=false reason=no-active-buffer`（`editor_view.rs:1071`）——
  本批不改执行路径，这条是防「顺手改坏」的钉子。
- **A7** 打开一个磁盘文件后重开 File：`Save / SaveAs / RevertFile / CloseTab` 变为可用；
  `Run ▸ ToggleBreakpoint` 变为可用。
- **A8** 只 `New Tab`（未命名 scratch）：`Save` **可用**（`writable = true`，`save_active` 会转另存为，
  见 `editor_view.rs:1082-1086`）；`RevertFile` 与 `ToggleBreakpoint` **仍置灰**（没有磁盘路径）。
- **A9** 只开 1 个标签页：`NextTab / PreviousTab / CloseOtherTabs` 置灰；
  `CloseAllTabs / CloseSavedTabs / CloseTab` 可用。
- **A10** 开 3 个标签页、活动在 ix=0：`CloseTabsToLeft` 置灰、`CloseTabsToRight` 可用；
  切到 ix=2 后两者反过来。
- **A11** 无缓冲打开 Edit，用 ↓ 从第一行走到 `CommandPalette`：标记行始终是 enabled 行
  （`Menu::step` 已跳过禁用行，`model.rs:198`），不出现「走不动」。
- **A12** 禁用行的文字色 = `fg_disabled`（`draw.rs:21-25`）、**图标槽**色也 = `fg_disabled`
  （§2.4，这是宿主自画槽必须自己做的事）、无 hover 底（`draw.rs:67`）、光标非 pointer（`draw.rs:68`）。
- **A13** 未接线行（File ▸ Show Local History、Edit ▸ Format Document 等）**在任何上下文下都不置灰**，
  点击仍走 `push_not_wired`。
- **A14** compact 模式（`MenuBarMode::Compact`）下，面板里九个顶层子菜单项**全部不置灰**（§4）。
- **A15** 守卫：`action_count()` / `not_wired_count()` / `SOURCE_MENU_ITEMS = 89` 三个数字与改造前一致。

**C. 库改动的回归（只在采纳 §2.5 时）**

- **A16** 画廊 `rust/lithe-ui/examples/gallery/pages/menus/rows.rs` 补一条
  **带 `keys` 的禁用行**（如 `MenuItem::new("Save").keys("secondary-s").disabled(true)`），
  亮/暗两套主题各看一眼 —— 现有的两条禁用行（`:77`、`:220-224`）**都没带 `keys`**，
  「禁用行 + 键位胶囊」这个组合画廊里从来没展示过，光看代码验不出来。
  （这条是颜思齐提的要求，owner 在实现侧，不是本设计。）
- **A17** `KbdCombo` 的其余 5 处生产调用点（`settings/shortcuts.rs:94`、
  `navigation/palette/kinds.rs:83`、`editor/welcome.rs:410`、
  `editor/keybindings.rs:187`、`forms/hotkey.rs:101`）**视觉零变化** —— 开关默认关闭，
  逐处 diff 截图确认（PR 里要点明，避免被当成全仓改色来回归）。
- **A18** 禁用行光标 = **默认箭头**：不是 pointer、也**不要** `not-allowed`
  （`not-allowed` 只有 Button 一家；与下拉选项行、浮层链接行一致）。

---

## 8. 本批范围：做 / 不做

**做（本批已定，主理人 2026-10-14 拍板）**
- `EditorContext` **七个字段**（§1.3）+ `MenuAction::is_enabled`（§2.2）+ `build_menu` 传参（§3.3）
- `editor` crate **五个**只读访问器：`tab_count()` / `active_tab_index()` / `has_active_disk_file()`
  / `can_go_back()` / `can_go_forward()`（后两个从 P1 提入本批，需删 `#[allow(dead_code)]`）
- 图标槽禁用色（§2.4，**不动库**）
- 键位胶囊禁用色：`KbdCombo::disabled(bool)`（默认关）+ 菜单侧 2 处接线（§2.5，**动库的最小面**，已批准进本批）
- A1-A20 断言（A19 / A20 是七字段版新增的跳转历史两条，见 §7）

**明确不做（后续单，主理人已裁）**
- dirty 用于 Save / RevertFile；`any_dirty()` ⇒ `SaveAll`（本批宽松判 `tab_count > 0`）
- 选区（Cut/Copy）；剪贴板（Paste）；撤销栈（Undo/Redo）；查找条开关；调试会话（Start/Stop Debugging）
- `has_closed_tabs()` ⇒ `ReopenClosedTab`；终端 `tab_count()` ⇒ `CloseTerminalTab`
- **快捷键也跟着禁用** —— 见 §9
- **命令面板也跟着禁用** —— 见 §9

**明确不改**
- `MENUS` 静态表的任何一行（不增删、不排序、不改 `Separator`）
- `menu_key()` / `keybind_action()` / `command_id()` / `label_key()` / `icon()`
- `MenuItem::NotWired` 的置灰策略
- 库的 `MenuItem::disabled()` 实现（`model.rs:107` 已够用）
- 键帽的**底**（`surface`）与**描边**（`border_strong`）—— 形状已经很安静，不动（`inline.rs:51-52`）
- 启用态图标的颜色（保持 `fg`，**不**顺手统一到 `fg_muted`）—— 与 S1 图标补齐同批另开（颜思齐裁定）
- 新增禁用 token（沿用 `fg_disabled`，不新增）

---

## 9. 为什么本批不扩散到快捷键与命令面板

| 通道 | 现状入口 | 结论 | 理由 |
| --- | --- | --- | --- |
| 快捷键 | `install_key_actions`（`menu_bar.rs:258`）→ `push_menu_key` → `bar.push_run` | **不禁用** | ① 这条路径根本不经过 `build_menu`，要拦就得在 `push_run` 里再判一次上下文，而 `push_run` 是 `&mut Context<MenuBar>`，拿不到编辑窗格；② 会牵动 `keymap_engine` 的快照与 `NON_COMMAND` 守卫；③ 「键位按下去 no-op + 诊断」本来就是裁决 #7 的原义 |
| 命令面板 | `visible_commands(ActionFlags)`（`workspace.rs:501`） | **不禁用** | 命令面板的语义是「**裁剪**不该出现的行」，不是「置灰」；`ActionFlags`（`343-348`）是另一套真源，两套混在一起会让「命令面板里没有」与「菜单里灰了」互相打架 |

**双保险**：执行侧（`save_active` / `save_as` / `revert_active` / `close_active` …）在开头就判
`self.active` 并打诊断（`editor_view.rs:1070`、`1094`、`1258`、`2002`）。
所以即使 `is_enabled` 判错，**最坏也只是「该灰的没灰」或「不该灰的灰了」，不会执行出错误行为**。
这是本批可以只做视觉层的前提。

**后续单（登记，不占本批）**：把 `is_enabled` 也接到 `push_run` 上 —— 需要 `MenuBar`
能拿到上下文，可行的形态是外壳在 `push_run` 之前把最新 `EditorContext` 塞进 `MenuBar`
的一个字段（`MenuBar` 已经是外壳持有的子组件，这条边是「外壳 → 子组件」，不违反「子组件不回戳外壳」）。

---

## 10. 与既有文档的关系

- `reports/editor-basics-remaining-design.md:519`：S5 的定义 —— 本设计是其展开，不改定义。
- `reports/editor-basics-remaining-design.md:500`：裁决 #7「no-op + 诊断；菜单置灰列为后续」——
  本批**追加**置灰，但**不撤回** no-op（§9 双保险）。
- `reports/editor-basics-interaction-spec.md:469-484`：F3/F4 的展开与「后续（另开单）」——
  本设计正是那一条后续单。
- `reports/menu-disable-visual.md`（颜思齐）：**配套文档，视觉口径以它为准**。
  本文 §2.4 / §2.5 的取值已经按它 §3.5 / §4.3 的结论修订过
  （我第一版把键帽颜色归到 `draw.rs:96` 是错的，实际落点是 `typography/inline.rs:55`）。
- `.agents/notes/implemented/feature/2026-10-10-gpui-menu-bar-chrome-and-keymap-settings.md`：
  决策八（子组件不回戳外壳、高频事件不触发外壳重画）与决策十（枚举落点）——本设计两条都遵守：
  上下文是**外壳读自己**、按值下发，不新增任何 notify。

**Note 义务**：本设计落地时应**更新**上面那篇 2026-10-10 的 Note（在「落地状态」表里补一行、
在「适用范围」里补 `menu_bar.rs` 已列 / `editor_view.rs` 新增），而不是新建一篇流水账
（`.agents/skills/agent-notes/SKILL.md` 的「优先更新已有 Note」）。

---

## 11. 未核实清单

- gpui 是否**每帧无条件**重建视图树（我由「`render_bar` 在 `ShellWorkspace::render` 里、
  且 `workspace.rs:1054` 的 observe 会 `cx.notify()`」推出「编辑窗格一变外壳就重画」，
  没有实测帧循环）。
- `CodeEditor`（`lithe-ui`）到底有没有可公开读取的选区 / 撤销栈 API —— 只确认了本批不需要，
  没有穷尽枚举。
- `TerminalPane` 是否另有非 `pub fn` 的标签页计数入口 —— 只 grep 了 `pub fn`，得到「无公开 API」。
- gpui-pre 把 SVG 渲成纯 alpha 蒙版再拿 `text_color` 填色（颜思齐给了
  `elements/svg.rs:182-186` / `svg_renderer.rs:231-252`）——**本机没有 gpui-pre 源码，我未核实**；
  我只给了两条代码级旁证（`icon.rs:531`、`draw.rs:41-56`）。
  **验收办法（主理人给的，比读源码便宜）**：实现跑起来后，同一张图、同一个 `Icon`，
  给它 `.color()` 看变不变色 —— 一眼就知道 `Icon::color()` 对 `Icon::from_path` 的图生效与否。
  **若不生效**（即这条方案走不通），不要就地改方案，回头找我和颜思齐重定图标禁用态的画法；
  备选有两条：① 换 `idea_icon_svg_px`（`icons/mod.rs:177-187`，它自己就是 `.text_color(...)` 那条路）；
  ② 按 `is_dark` 另选一张已降饱和的图。**先按现有方案走，把结果告诉我们。**
- `draw.rs:96` / `KbdCombo` 改色后的实际对比度数字 —— 全部取自颜思齐
  `reports/menu-disable-visual.md` §3.5，**我未实测**。
- 另见 §11 其余三条（gpui 帧循环、`CodeEditor` 选区 API 面、`TerminalPane` 非公开入口）。
