//! 工作台骨架：标题栏 + 项目标签条 + 左右活动栏 + Dock 布局组装 + 状态栏 + 浮层三层。
//!
//! 布局决定（来自 gpui-kit 官方资料，不是照抄 Windows 的 DOM）：
//!
//! - 工作区交给 `DockArea`。官方 dock 文档写明 Dock 是"生产环境使用的布局基础"，
//!   提供可拖拽标签组、嵌套分割与左右底边缘停靠，正是 IDE 工作台需要的东西。
//! - 窗口骨架照官方示例 `examples/dock/src/main.rs:542-594`：
//!   `div().relative().size_full().flex().flex_col()` → TitleBar →
//!   `div().flex_1().min_h_0()` 包住 DockArea → StatusBar。
//! - 三块"窗口边框"区域（标题栏 / 左侧活动栏 / 状态栏）的**组成、度量、文案与颜色 token**
//!   以 macOS 端为准，出处逐条写在各段注释里（调研原文：
//!   `.artifacts/ui-map/02-workbench-layout.md` §2.1/2.3/2.6 与
//!   `.artifacts/ui-map/01-app-shell.md` §2.2-2.7）。**Dock 内部的面板组装与
//!   底部工具窗高度不在本文件本次范围**（那是别人在改的部分）。
//! - **浮层三层必须业务视图自己挂**：`Root::render` 里没有 Dialog / Sheet /
//!   Notification 三层，不挂就永远弹不出来，而且是**静默失败**（不报错、不 panic，
//!   屏幕上什么都没有）。本文件在渲染根部（最外层 `div()` 的**末尾**）挂这三层，
//!   见 `render` 的注释与 `.artifacts/ui-map/09-overlay-howto.md` §1。
//! - 右侧栏按 macOS 口径是**默认不占布局宽度**的：真机里它是
//!   `.overlay(alignment: .trailing)` 的 `rightHoverRegion` + 40pt 右活动栏
//!   （`Views/Workbench/WorkbenchView.swift:229-234`），停靠形态另由
//!   `isDockedSidebarVisible` 控制（:1531）。所以 `项目统计` / `大纲` 注册成
//!   `DockPlacement::Right` 的停靠区并**默认收起**，由右活动栏按钮切换。
//! - **命令面板是本项目用 gpui-kit 补的优化项，macOS 侧没有这个功能**。macOS 唯一接近的
//!   是 `SearchEverywhereView` 的 `actions` / `commands` 模式（输入 `/` 进入命令模式，
//!   `Views/Search/SearchEverywhereView.swift:18-26,120-141,267`），入口是隐形手势
//!   **双击 ⇧（阈值 0.35s）或 ⇧⌘O**（`Models/Keymap/LitheCommandCatalog.swift:32-41`）。
//!   补它的理由：目录里的 **13 条命令完全没有默认键**，用户没有任何键盘路径
//!   （`.artifacts/ui-map/01-app-shell.md` §3.4；`gpui/UI-MAP.md` §2.1 的"macOS 做得不好"一栏）。
//!   注：目录实际是 **39 条**（`LitheCommandCatalog.swift:13-66`：38 条 `command(..)` +
//!   显式构造的 `search-everywhere`），调研文档记的 41 条与当前源码不符，这里以源码为准。
//!   实现路径 = `window.open_dialog` + `Command`：这是唯一同时满足"居中 / 带输入框 /
//!   Esc 关闭 / 点外关闭 / 焦点自动归还"且有官方测试背书的路径
//!   （`.artifacts/ui-map/09-overlay-howto.md` §6.1），`Command` 本身只是普通流式
//!   `v_flex`（`command/state.rs:819-837`），**不能直接当浮层**，必须放进 Dialog。
//!
//! 结构：标题栏（项目图标 + 项目名 + 下拉 / 分支）→ 项目标签条（右端是命令面板入口按钮）
//! → [ 左侧活动栏（38pt 图标竖条） | Dock 区域（右侧 360pt 停靠区默认收起） |
//! 右活动栏（40pt）] → 状态栏（面包屑 + 右侧指标）→ 浮层三层（Dialog / Sheet /
//! Notification，命令面板走 Dialog 这一层）。

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::dock::{
    DockArea, DockLayout, DockPlacement, DockSkin, PanelId, panel_handle,
};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, IndexPath, Root, Selectable as _,
    StyledExt as _, TitleBar, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    KeyDownEvent, ModifiersChangedEvent, ParentElement as _, Render, SharedString, Styled as _,
    Window, div, px,
};

// 底部工具窗（`提交记录`）是自成一体的面板：它自己画头部与标签行，不复用
// `ShellPanel` 的 `PanelKind` 分支，所以单独一个模块（声明在 `mod.rs`）。
use super::bottom_panel::BottomPanel;

use super::files::{FILE_ROWS, build_tree, load_files};
use super::panels::{PanelKind, ShellPanel};

/// 工作台视图：标题栏 + 左右活动栏 + Dock 区域 + 状态栏，并挂上 Dialog / Sheet /
/// Notification 三层浮层（见 `Render` 实现的注释）。
pub(super) struct ShellWorkspace {
    dock_area: Entity<DockArea>,
    /// Dock 的外观句柄，留着以后改面板样式。
    _skin: Rc<DockSkin>,
    /// 命令面板实体。真机里它是**浮层**（双击 Shift 唤出），不是 Dock 标签页；
    /// 挂到 `Root` 浮层上的那一步见 `gpui/PLAN.md` 的 S4。这一阶段先把它建出来
    /// 但不放进 Dock，免得出现真机没有的标签。
    _command_panel: Entity<ShellPanel>,
    /// **真正在用的**命令面板交互状态，由工作台自己持有：浮层属于窗口，
    /// 不属于 Dock 标签页（`.artifacts/ui-map/09-overlay-howto.md` §6.3 Step 1）。
    /// 上面那个 `_command_panel` 是这台面板当初留在 Dock 里的占位实体，两者互不相干。
    palette: Entity<CommandState>,
    /// 根 `div()` 的焦点句柄。见 [`ShellWorkspace::on_root_modifiers_changed`] 的注释：
    /// gpui 的键盘 / 修饰键事件沿"焦点节点的祖先链"派发，焦点为空时**只派发到窗口根节点**
    /// （`gpui-pre-0.3.6/src/window.rs:6244-6252`），那时挂在工作台根 div 上的监听一个都收不到，
    /// 所以工作台开局先用这个句柄占住焦点。
    focus_handle: FocusHandle,
    /// 双击 ⇧ 的手势状态（350ms 阈值），判定算法对齐 macOS 的
    /// `DoubleShiftGestureRecognizer`（见 [`DoubleShiftGesture`]）。
    shift_gesture: DoubleShiftGesture,
    /// 项目名。macOS 取 `model.projectName`（`WorkbenchView.swift:680,696`），
    /// 探针的等价来源是工作区根目录名；标题栏左段与状态栏面包屑回退态都用它。
    project_name: SharedString,
    /// 当前分支名。macOS 取 `model.currentBranch`（`WorkbenchView.swift:718`）。
    ///
    /// 待接线：正式口径是 Core 的 `workspace.repositories` + `git.status`
    /// （`.artifacts/ui-map/02-workbench-layout.md` §4）；这里先读宿主真实的
    /// `.git/HEAD`，读不到就**不画**分支按钮（`None`），而不是写死一个假分支名。
    branch: Option<SharedString>,
    /// 状态栏右侧那一项：工作区快照的文件总数（`None` = Core 命令还没回来）。
    /// 这是探针为了验证 Core 数据接线额外加的项，macOS 状态栏里**没有**这一项
    /// （见 `render_status_bar` 的注释）。
    snapshot_files: Option<SharedString>,
    /// 右侧 dock 当前选中的那一页。右 dock **默认收起**，所以它同时表达
    /// "下次打开右 dock 时显示哪一页"。
    right_panel: RightPanel,
    /// 右 dock 两页的稳定标识。面板实体本身已经被移进 Dock 布局（拿不回来），
    /// 右活动栏按钮靠这两个 `PanelId` 调 `DockArea::select_panel` 把目标页切到前台。
    stats_panel_id: PanelId,
    outline_panel_id: PanelId,
}

/// 右侧 dock 的两页。
///
/// macOS 的右侧栏内容由 `rightSidebarContributions` 决定（`WorkbenchView.swift:1313-1335`），
/// 探针把它们做成了右 dock 的两个标签页：`大纲` 是真机就有的右侧工具窗，
/// `项目统计` 是这一阶段为验证 `DataTable` + 真实快照数据加的临时面板
/// （见 `gpui/PLAN.md`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RightPanel {
    /// `项目统计`。
    Stats,
    /// `大纲`。
    Outline,
}

impl ShellWorkspace {
    pub(super) fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 项目名与分支名要在 `root` 被移进后台任务之前取出来。
        let project_name: SharedString = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string())
            .into();
        let branch = read_current_branch(&root);

        let (dock_area, skin) = DockSkin::dock_area("lithe-workbench", Some(1), window, cx);

        // 先建编辑区，再建文件面板：文件面板要拿到编辑区的弱引用来响应点击。
        // （两个面板都由 Dock 持有，互相强引用会形成环。）
        let editor_panel = cx.new(|cx| ShellPanel::editor(window, cx, root.clone()));
        // 启动时先打开一个**真实文件**，让编辑区一上来就能看到真内容；
        // 之后点文件树里的文件会走同一个 `open_document` 换文档。
        let initial = "rust/lithe-core/src/lib.rs";
        if root.join(initial).is_file() {
            editor_panel.update(cx, |panel, cx| panel.open_document(initial, window, cx));
        }
        let files_panel = cx.new(|cx| ShellPanel::files(window, cx, editor_panel.downgrade()));
        let bottom_panel = cx.new(|cx| BottomPanel::new(window, cx, root.clone()));
        // 真机右侧工具窗是 `大纲`；`项目统计` 是这一阶段为了验证 `DataTable`
        // 拿真实快照数据渲染出来的临时面板（见 `gpui/PLAN.md`）。
        let outline_panel = cx.new(|cx| {
            ShellPanel::placeholder("OutlinePanel", "大纲", PanelKind::Outline, window, cx)
        });
        let stats_panel = cx.new(|cx| {
            ShellPanel::placeholder("StatsPanel", "项目统计", PanelKind::Stats, window, cx)
        });
        // `stats_panel` 下面会被移进 Dock 布局，所以先拿它内部 `TableState` 的弱引用，
        // 之后异步加载完才能把行数据写回**被渲染的那个** `TableState`。
        let stats_weak = stats_panel.read(cx).stats.downgrade();
        let command_panel = cx.new(|cx| {
            ShellPanel::placeholder("CommandPanel", "命令面板", PanelKind::Command, window, cx)
        });
        // 命令面板的交互状态归工作台。这里只建实体：浮层在 `open_command_palette` 里
        // 用 `window.open_dialog` 打开（每打开一次重新构造 Dialog 与 `Command`，
        // 但状态只有这一份，见 `root.rs:64,73,262`）。
        let palette = cx.new(|cx| CommandState::new(window, cx));

        // 布局：中心区上半行（文件列表 | 编辑器），下半行是**横跨中心区**的底部工具窗；
        // 右侧的 `项目统计` / `大纲` 不再是中心区的常驻第三列，而是独立的右侧停靠区
        // （见下面 `set_dock`）。
        //
        // 这与真机一致：Windows 的 `提交记录` 工具窗横跨整个窗口宽度
        // （`docs/assets/screenshots/windows-git-log-tool-window.png` 里它从活动栏右边
        // 一直延伸到窗口右边缘，左侧的浏览器面板在它上面、不在它左边）。
        // 早先的版本把工具窗放在中列的上下分割里，导致它只占编辑器那一段宽度，
        // 与真机不符。
        let top_row = DockLayout::h_split()
            .child(
                // 左栏默认 320pt（`Views/Workbench/WorkbenchView.swift` 的布局常量）。
                DockLayout::tabs().panel_view(panel_handle(files_panel.clone()), cx),
                Some(px(320.)),
            )
            .child(
                DockLayout::tabs().panel_view(panel_handle(editor_panel), cx),
                None,
            );

        let layout = DockLayout::v_split()
            .child(top_row, None)
            .child(
                // 底部工具窗：macOS 的下区最小高 260pt。
                DockLayout::tabs().panel_view(panel_handle(bottom_panel), cx),
                Some(px(260.)),
            );

        dock_area.update(cx, |area, cx| {
            area.set_center(layout, window, cx);
        });

        // 右侧栏改成 macOS 口径：面板仍用 `set_dock(DockPlacement::Right, ..)` 注册、
        // 尺寸 360（macOS 右侧停靠工具窗的默认宽，最小 300、最大 520，
        // `.artifacts/ui-map/02-workbench-layout.md` §1.4 表 ←
        // `Services/Workbench/WorkbenchLayoutStore.swift:4-6`）。
        //
        // macOS 的右侧栏**默认不占布局宽度**：浮层形态是
        // `.overlay(alignment: .trailing)` + `workspaceArea.padding(.trailing, 40)`
        // （`WorkbenchView.swift:229-234`），停靠形态也要 `isDockedSidebarVisible`
        // 为真才出现（:1531）。所以这里建完立刻收起来：dock 关闭时不占宽度，
        // 编辑区宽度与"没有右侧栏"完全一致。
        let stats_panel_id = PanelId::from(stats_panel.entity_id());
        let outline_panel_id = PanelId::from(outline_panel.entity_id());
        let right_dock = DockLayout::tabs()
            .panel_view(panel_handle(stats_panel), cx)
            .panel_view(panel_handle(outline_panel), cx);
        dock_area.update(cx, |area, cx| {
            area.set_dock(DockPlacement::Right, right_dock, window, cx);
            area.set_dock_size(DockPlacement::Right, px(360.), window, cx);
            // 新建的 dock 默认是 open + collapsible
            // （`gpui-base-0.6.6/src/dock/dock_placement.rs:121-128`），
            // 所以"默认收起"要显式切一次；`collapsible` 保持 true，
            // 右活动栏按钮和 dock 自带的收起按钮才能再把它打开
            // （`toggle_dock` 对不可折叠且已打开的 dock 会直接返回，
            // `gpui-base-0.6.6/src/dock/dock_area.rs:301-320`）。
            area.set_dock_collapsible(DockPlacement::Right, true, window, cx);
            area.toggle_dock(DockPlacement::Right, window, cx);
        });

        // 真实数据：后台跑 Core 命令，回到前台更新面板。
        let files_weak = files_panel.downgrade();
        let workspace_weak = cx.entity().downgrade();
        cx.spawn(async move |_this, cx| {
            let loaded = cx
                .background_spawn(async move { load_files(&root, FILE_ROWS) })
                .await;
            let _ = workspace_weak.update(cx, |workspace, cx| {
                workspace.snapshot_files = match &loaded {
                    Ok((_, total, _)) => Some(format!("{total} 个文件").into()),
                    Err(error) => Some(format!("快照失败：{error}").into()),
                };
                cx.notify();
            });
            let _ = files_weak.update(cx, |panel, cx| {
                if let Ok((rows, total, stats)) = loaded {
                    let items = build_tree(&rows);
                    panel.tree.update(cx, |state, cx| {
                        state.set_items(items, cx);
                    });
                    panel.title = format!("项目 · {total}").into();
                    // 统计表在右侧 Dock 里，是独立的面板实体：
                    // 必须更新它自己持有的 `TableState`，而不是文件面板的那个。
                    let _ = stats_weak.update(cx, |state, cx| {
                        state.delegate_mut().rows = stats;
                        // `refresh` 是 gpui-component 文档里"行/列变了"之后的入口。
                        state.refresh(cx);
                        cx.notify();
                    });
                }
                cx.notify();
            });
        })
        .detach();

        // 根 div 必须自己先占住焦点，理由见 `focus_handle` 字段与
        // `on_root_modifiers_changed` 的注释：焦点为空时键事件只派发到窗口根节点，
        // 挂在工作台根 div 上的双击 ⇧ 监听一个都收不到。
        // 句柄由 `render` 里根 div 的 `track_focus` 认领，否则
        // `focus_node_id_in_rendered_frame`（`window.rs:6244-6252`）找不到对应节点。
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        let workspace = Self {
            dock_area,
            _skin: skin,
            _command_panel: command_panel,
            palette,
            focus_handle,
            shift_gesture: DoubleShiftGesture::new(),
            project_name,
            branch,
            snapshot_files: None,
            // 默认选 `项目统计`（右 dock 里的第一个标签页），但 dock 本身是收起的，
            // 所以启动时窗口右侧不占宽度。
            right_panel: RightPanel::Stats,
            stats_panel_id,
            outline_panel_id,
        };

        workspace
    }

    /// 右活动栏按钮的行为：切换右侧 dock 的显示，并把目标页选到前台。
    ///
    /// 语义照 macOS 的 `rightSidebarContributions`：点某个贡献项就展开对应的右栏内容
    /// （`WorkbenchView.swift:1313-1335`）。差别在触发方式：macOS 是 hover 型
    /// （悬停即打开、移出 60ms 后关闭，:1323-1335,1398-1409），gpui 侧没有这套延迟
    /// 状态机（`.artifacts/ui-map/02-workbench-layout.md` §6 第 7 条建议做成显式状态机），
    /// 所以本阶段用点击切换。
    ///
    /// 同一个按钮再点一次 = 收起右侧栏（`is_dock_open` 为真且选中的就是它）。
    fn toggle_right_dock(
        &mut self,
        panel: RightPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let placement = DockPlacement::Right;
        let panel_id = match panel {
            RightPanel::Stats => self.stats_panel_id,
            RightPanel::Outline => self.outline_panel_id,
        };
        // 判断要放在 `update` 外面：闭包里不能再借 `self`。
        let collapse = self.right_panel == panel && self.dock_area.read(cx).is_dock_open(placement);
        self.dock_area.update(cx, |area, cx| {
            if collapse {
                area.toggle_dock(placement, window, cx);
            } else {
                // `select_panel` 是"把已在 dock 里的某一页显示出来"的入口
                // （`gpui-base-0.6.6/src/dock/dock_area.rs:550-577`），
                // 它不会移动标签位置，正好用于在 `项目统计` / `大纲` 之间切换。
                area.select_panel(panel_id, window, cx);
                if !area.is_dock_open(placement) {
                    area.toggle_dock(placement, window, cx);
                }
            }
        });
        self.right_panel = panel;
        cx.notify();
    }

    /// 打开命令面板浮层。
    ///
    /// 路径 = `window.open_dialog` + `Command`，理由与证据：
    ///
    /// - 打开入口是 `WindowExt::open_dialog(&mut self, cx: &mut App, build)`
    ///   （`gpui-component-0.6.6/src/window_ext.rs:30-32,141-148`），内部走 `Root::update`
    ///   —— 所以窗口根必须是 `Root`（`mod.rs` 已经包了），否则那里直接 panic
    ///   （`root.rs:160-163`）；`build` 是 `Fn(Dialog, &mut Window, &mut App) -> Dialog`，
    ///   **每次打开都会重新构造**，所以对话框里的东西必须是现建的（`root.rs:64,73,262`）。
    /// - **不挂浮层三层就静默不显示**：`render` 里已经挂了 `Root::render_dialog_layer` 等三层
    ///   （见 `render` 的注释与 `.artifacts/ui-map/09-overlay-howto.md` §1）。
    /// - **Esc 关闭 / 点外关闭 / 焦点自动归还**全部由 Dialog 自带，这里一行都不用写：
    ///   键绑定 `escape → Cancel` / `enter → Confirm`（`gpui-base-0.6.6/src/dialog.rs:89-94`）、
    ///   遮罩 `on_any_mouse_down → on_cancel → request_close(false)`（`:590-625`）、
    ///   焦点记录与还原（`root.rs:301-310,333-363`）；焦点陷阱也是自带的（`dialog.rs:552-553`）。
    /// - `Command` 只是普通流式 `v_flex`（`command/state.rs:819-837`），**必须**放进 Dialog 当内容。
    fn open_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 已经开着就不要再叠一层。双击 ⇧ 的监听在浮层打开期间照样会收到事件——浮层是根
        // div 的子元素（见 `render`），仍在焦点节点的祖先链上。
        // `has_active_dialog` 是 `Root` 的真实状态（`window_ext.rs:55-56`），比另存一个 bool 可靠：
        // Esc / 点外 / 回车三条关闭路径都不用我们维护。
        if window.has_active_dialog(cx) {
            return;
        }

        let palette = self.palette.clone();
        // 每次打开都从空查询开始：上次留下的过滤词会让面板"看起来少了几条命令"。
        palette.update(cx, |state, cx| state.set_query("", window, cx));

        // `on_confirm` 只给 `&mut App`（`command/command.rs:157-163`），拿不回 `Context<Self>`，
        // 所以用弱引用回到工作台（与上面加载快照的写法一致）。
        let workspace = cx.entity().downgrade();
        // Dialog 的构造闭包要吃掉一份 `palette`（下面对它取引用），
        // 而下面 focus 搜索框那次 `window.defer` 还要用原来的实体，所以这里再克隆一份。
        let dialog_palette = palette.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            // 闭包是 `Fn`（每打开一次都要重新构造一遍），所以内部要移动的东西现克隆。
            let workspace = workspace.clone();
            // `on_ok` 的闭包要吃掉一份实体，这里也现克隆一份。
            let ok_palette = dialog_palette.clone();
            dialog
                // 宽度不由 macOS 定（macOS 没有命令面板）：`SearchEverywhereView` 的 860
                // 是"搜索一切"（含文件/类/符号结果）的宽度（`SearchEverywhereView.swift:192`），
                // 纯命令列表不需要那么宽，取 09 号调研建议的 560
                // （`.artifacts/ui-map/09-overlay-howto.md` §6.3）。
                .w(px(560.))
                // 顶部偏移取 macOS `SearchEverywhereView` 的 84（`:195`）。
                // Dialog 默认是"视口高 / 10"（`dialog/dialog.rs:529`），会随窗口高度漂；
                // 水平方向 Dialog 自己居中（`dialog/dialog.rs:534`），不用管。
                .margin_top(px(84.))
                // 命令面板不要右上角关闭按钮：macOS 的 Search Everywhere 只有 Esc 与点外关。
                .close_button(false)
                // 点外关 / Esc 关保持 Dialog 默认（都为 true，`dialog/dialog.rs:173-186`），显式写一遍当文档。
                .overlay_closable(true)
                .keyboard(true)
                // 回车要不要关面板，由这里说了算：`Dialog` 的 `Confirm` 会从 `Command`
                // 那个节点冒泡到 Dialog 宿主节点（`gpui-base-0.6.6/src/dialog.rs:574-587`），
                // 只有当前高亮的是一条**可执行**命令时才让它关。
                // （没有匹配项时 `CommandState` 不会去 confirm，但冒泡照样会到 Dialog，
                // 不加这道判断就会出现"回车把空面板关掉"。）
                // 判定读 `CommandState::selected_index`（`command/state.rs:231`，未过滤模型坐标）
                // 再查同一张表，与 `on_confirm` 用的是同一份映射，两边不会漂。
                .on_ok(move |_, _, cx| {
                    ok_palette
                        .read(cx)
                        .selected_index()
                        .and_then(palette_command_at)
                        .is_some_and(|command| command.enabled)
                })
                .child({
                    let mut command = Command::new(&dialog_palette)
                        // 已经在 Dialog 自己的框里，不要再套一层（`command/command.rs:200-207`）。
                        .bordered(false)
                        // 列表最大高 = macOS 的 560（`SearchEverywhereView.swift:193`）
                        // 减去搜索框 44（`:282`），≈516；默认只有 `rems(18.75)`=300（`command.rs:195`）。
                        .max_h(px(516.))
                        .placeholder(PALETTE_PLACEHOLDER)
                        .empty(|_, _, cx| {
                            div()
                                .py_6()
                                .w_full()
                                .text_center()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(PALETTE_EMPTY)
                        })
                        .on_confirm(move |index, window, cx| {
                            let Some(command) = palette_command_at(index) else {
                                return;
                            };
                            // 回调里 `window` 是现成的（`state.rs:599-603` 用 `window.defer` 把它传了进来），
                            // 而 `WeakEntity::update` 只给 `(&mut Self, &mut Context<Self>)`、
                            // 没有 window（`gpui-pre-0.3.6/src/entity_map.rs:795-806`），
                            // 所以把 `window` 直接捕获进这个闭包，Dock 的切换才拿得到它。
                            let _ = workspace.update(cx, |workspace, cx| {
                                workspace.run_palette_command(command.id, window, cx);
                            });
                        });
                    for group in palette_groups() {
                        command = command.group(group);
                    }
                    command
                })
        });

        // `Root::open_dialog` 只 focus 它自己分配的句柄（`root.rs:309-310`），
        // 真正把键盘交给搜索框还得自己再 focus 一次（`CommandState::focus`，`command/state.rs:281`）。
        // 此刻元素还没入树（要等下一帧重画），所以用 `window.defer` 押到本次效果周期末尾。
        // ⚠️ 这一步**没有官方用例**：`09-overlay-howto.md` §6.4 第 5 条要求"浮层一出现就能直接打字"
        // 必须实测；若发现丢字，就改成在 Dialog 的 `on_ok` 之前手动点一次输入框。
        window.defer(cx, move |window, cx| {
            palette.update(cx, |state, cx| state.focus(window, cx));
        });
    }

    /// 执行命令面板里确认的那条命令。`IndexPath` 已经由 [`palette_command_at`] 换成稳定 id。
    ///
    /// **本轮真正生效的只有两条**：右侧「大纲」与「项目统计」的显示 / 隐藏切换
    /// （走已有的 [`ShellWorkspace::toggle_right_dock`]：`select_panel` + `toggle_dock`，
    /// `gpui-base-0.6.6/src/dock/dock_area.rs:301-320,558-577`）。其余表项一律
    /// `enabled: false`，原因是**它们真的还不能生效**，逐条写在 [`PALETTE_GROUPS`] 上面。
    /// 「关闭编辑器标签」这类命令也一并登记在那里：编辑区把当前文档放在 `ShellPanel::document`
    /// （`panels.rs:373`），该字段对工作台不可见，本轮拿不到编辑区状态，所以不做。
    fn run_palette_command(
        &mut self,
        command: PaletteCommandId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            PaletteCommandId::Outline => self.toggle_right_dock(RightPanel::Outline, window, cx),
            PaletteCommandId::Stats => self.toggle_right_dock(RightPanel::Stats, window, cx),
            // 待接线：这 7 条对应 [`PALETTE_GROUPS`] 里 disabled 的表项，
            // `CommandState::confirm` 对禁用项直接返回（`command/state.rs:584-590`），
            // 正常操作路径下走不到这里；等某个面板接上以后把表项改成 `enabled: true` 再填实现。
            PaletteCommandId::ToggleTerminal
            | PaletteCommandId::ToggleProblems
            | PaletteCommandId::ToggleMaven
            | PaletteCommandId::ToggleGitLog
            | PaletteCommandId::ToggleRun
            | PaletteCommandId::ToggleTests
            | PaletteCommandId::ToggleDebug => {}
        }
        // 这里不 `cx.notify()`：`toggle_right_dock` 自己会 notify，待接线分支没有可重画的东西。
    }

    /// 根元素上的**按键**监听：只做一件事 —— 把"带 Shift 的输入"从双击 ⇧ 手势里排除。
    ///
    /// macOS 的判定规则是"两次**独立**的 Shift 轻点"（`DoubleShiftGestureRecognizer`，
    /// `Platform/MacOS/UI/MacShortcutDetector.swift:256-317`）：按下 Shift 期间还有别的键
    /// （`Shift+A` 这种）会让这次按下作废（`:267-270`，`handleKeyDown`）。
    ///
    /// **为什么 Shift 的按下 / 抬起不看这个回调**：这一层平台就不发 Shift 的键事件。
    /// Windows 平台的 `handle_key_event` 对 `VK_SHIFT` / `VK_CONTROL` / `VK_MENU` / `VK_LWIN`
    /// 等修饰键直接返回 `ModifiersChanged`，不构造 `KeyDownEvent` / `KeyUpEvent`
    /// （`gpui-pre-windows-0.3.6/src/events.rs:1504-1518`），所以 `on_key_down` /
    /// `on_key_up` 里永远看不到 `key == "shift"`（两者确实存在：
    /// `gpui-pre-0.3.6/src/elements/div.rs:1127-1160`，但拿不到 Shift）。
    /// 因此双击 ⇧ 只能由 `on_modifiers_changed` 驱动。
    fn on_root_key_down(&mut self, _: &KeyDownEvent, _: &mut Window, _: &mut Context<Self>) {
        self.shift_gesture.handle_key_down();
    }

    /// 根元素上的**修饰键变化**监听：识别双击 ⇧（阈值 350ms）后打开命令面板。
    ///
    /// 依据：macOS 的快捷键真源里 `search-everywhere` 的绑定之一是 `.doubleTap(.shift)`
    /// （`Models/Keymap/LitheCommandCatalog.swift:32-41`），阈值 0.35s
    /// （`Views/App/KeyboardShortcutRecorderView.swift:72`、`MacShortcutDetector.swift:155`）；
    /// 判定算法逐条对齐 [`DoubleShiftGesture`]。
    ///
    /// 监听挂在**根 `div()`** 上：修饰键事件沿"焦点节点的祖先链"冒泡
    /// （`gpui-pre-0.3.6/src/window.rs:6073-6091`），根 div 是所有区域的祖先；
    /// `on_modifiers_changed` 是 `InteractiveElement` trait 上的方法
    /// （`gpui-pre-0.3.6/src/elements/div.rs:1176-1182` → `Interactivity::on_modifiers_changed`，
    /// `div.rs:557-562`，所以 import 里要 `InteractiveElement as _`）。
    fn on_root_modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // macOS 的 `hasOtherModifiers` 只算 ⌘⌃⌥ 与功能键，不含 ⇪
        // （`MacShortcutDetector.swift:217-224`）；gpui 的 `platform` 就是 ⌘/Win。
        let modifiers = event.modifiers;
        let has_other_modifiers =
            modifiers.control || modifiers.alt || modifiers.platform || modifiers.function;
        if self
            .shift_gesture
            .handle_modifiers_changed(modifiers.shift, has_other_modifiers)
        {
            self.open_command_palette(window, cx);
        }
    }

    /// 标题栏（macOS `topBar`，`WorkbenchView.swift:669-767`）。
    ///
    /// 度量：高 40（`Metrics.toolbarHeight`，`Theme/LitheTheme.swift:416`）、
    /// `HStack(spacing: 9)`（:670）、左内边距 76（给 macOS 红绿灯让位，:756）、
    /// 右内边距 10（:757）。`TitleBar` 自带 34 高与平台相关的左内边距
    /// （macOS 80 / 其它 12，`gpui-component-0.6.6/src/title_bar.rs:15-19`），
    /// 两者都按规格覆盖；覆盖后不再区分平台，符合 `gpui/UI-MAP.md` §1.1.5
    /// "界面结构三端统一，平台差异交给 gpui 平台层"。
    ///
    /// 组成（macOS 是"左段两个切换按钮 + `Spacer(minLength: 22)` + 右段运行控件"）：
    ///
    /// - 左段 1 项目切换按钮：`LitheLogo(size: 24)` + 项目名 13pt semibold +
    ///   `chevron.down` 9pt，高 32、圆角 6、水平内边距 8、间距 8（:677-695）；
    /// - 左段 2 分支切换按钮：VCS 图标 14 + 分支名 12.5pt medium +
    ///   `chevron.down` 8pt bold，高 32、圆角 6、内边距 9、间距 7（:711-732）；
    /// - 右段：运行配置选择器 + 运行/调试/停止 + `UpdateControl` + 壁纸选择（:743-753）。
    ///
    /// 两个切换按钮在 macOS 里是打开 popover 的 `Button`，但探针还没有这两个 popover
    /// （`ProjectSwitcherPopover` / `BranchSwitcherPopover`），所以只画静态外观，
    /// 不做"点了没反应"的假按钮。右段**待接线**：那些元素都要运行配置、执行状态、
    /// 更新与壁纸数据源，探针都没有，本阶段不渲染（宁可缺，不画假的）。
    // 这几个绘制助手只读主题（`cx.theme()`），所以收 `&App` 而不是
    // `&mut Context<Self>`：Rust 2024 下返回的 `impl IntoElement` 会捕获入参生命周期，
    // 拿 `&mut` 会和 `render` 里后续的 `cx.theme()` 冲突（E0502）。
    fn render_title_bar(&self, cx: &App) -> impl IntoElement {
        let project_switcher = h_flex()
            .gap(px(8.))
            .px(px(8.))
            .h(px(32.))
            .rounded(px(6.))
            .child(
                // 默认图标集里没有 Lithe 的 logo 字形（`LitheLogo` 是应用自绘的矢量
                // 标志），暂用 `LayoutDashboard` 代替；配色用 macOS `accent` 对应的
                // gpui token `theme.primary`（`gpui/UI-MAP.md` §1.2 的 token 映射）。
                Icon::new(IconName::LayoutDashboard)
                    .w(px(24.))
                    .h(px(24.))
                    .text_color(cx.theme().primary),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(self.project_name.clone()),
            )
            .child(
                Icon::new(IconName::ChevronDown)
                    .w(px(9.))
                    .h(px(9.))
                    .text_color(cx.theme().muted_foreground),
            );

        let branch_switcher = self.branch.as_ref().map(|branch| {
            h_flex()
                .gap(px(7.))
                .px(px(9.))
                .h(px(32.))
                .rounded(px(6.))
                .child(
                    // 默认图标集里没有 git 分支字形（macOS 用 IntelliJ 资源
                    // `toolwindows/toolWindowVcs.svg`），暂用 `Network` 代替。
                    Icon::new(IconName::Network)
                        .w(px(14.))
                        .h(px(14.))
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_medium()
                        .text_color(cx.theme().foreground)
                        .child(branch.clone()),
                )
                .child(
                    Icon::new(IconName::ChevronDown)
                        .w(px(8.))
                        .h(px(8.))
                        .text_color(cx.theme().muted_foreground),
                )
        });

        TitleBar::new()
            // macOS 的 `topBar` 是 40pt（`Theme/LitheTheme.swift` 的 `Metrics.toolbarHeight`），
            // gpui-kit 的 `TitleBar` 默认 34px，按真机覆盖（`TitleBar` 实现了 `Styled`，
            // 且内部先 `.h(34)` 再 `refine_style`，所以外部能改掉）。
            .h(px(40.))
            // 右侧 10pt 是 macOS 的 `topBar` 内边距（`WorkbenchView.swift:758`）。
            .pr(px(10.))
            // ⚠️ **不要**在这里写 `.pl(px(76.))`：macOS 左侧那 76pt 是给系统交通灯留的位置
            // （`WorkbenchView.swift:756`），而 `TitleBar` 内部本来就按平台给了这个前导内边距
            // （`gpui-component-0.6.6/src/title_bar.rs:16-19,336`：macOS 80 / 其它平台 12，
            // 再被 `refine_style` 覆盖）。写死 76 会在 Windows 上留下一段 76pt 的空白，
            // 因为 Windows 的窗口按钮在右侧、左侧没有交通灯。
            // 平台差异交给组件的这一层处理，符合"不写平台业务分支"的约束。
            .child(
                h_flex()
                    // `TitleBar` 内部那行是 `justify_between` 的 flex 行，子元素默认按内容
                    // 宽度排版；不给 `w_full()` 的话下面的 `Spacer` 撑不开，右段会贴到
                    // 项目名右边而不是窗口右边。
                    .w_full()
                    .gap(px(9.))
                    .child(project_switcher)
                    // `Spacer(minLength: 22)`（`WorkbenchView.swift:741`）：本阶段右段是空的，
                    // 由它吃掉全部剩余宽度。
                    .children(branch_switcher)
                    .child(div().flex_1()),
            )
    }

    /// 左侧活动栏（macOS `activityBar`，`WorkbenchView.swift:1170-1264`）。
    ///
    /// 度量 `ActivityBarMetrics`（`WorkbenchView.swift:9-18`）：宽 38、按钮 30×30、
    /// 按钮间距 4、上下边距 4；背景 `LitheTheme.titlebar`（有壁纸时透明，:1261），
    /// 对应 gpui 的 `theme.title_bar`（活动栏与标题栏、状态栏同色，见
    /// `.artifacts/ui-map/02-workbench-layout.md` §2.7）。
    ///
    /// 上段按钮顺序照 `SidebarDestination.allCases`
    /// （`Models/AppModel/AppModelSupportTypes.swift:90-134`）：项目 / 更改 / 拉取请求 /
    /// 搜索 / 数据库连接；中文取 `macos/Resources/zh-Hans.lproj/Localizable.strings:7,323,414,321,1343`。
    /// 每项 30×30、圆角 4、激活底色 `subtleSelection`、激活前景 `primaryText` 否则
    /// `secondaryText`（:1194-1207）；图标在有 IntelliJ 资源时 18pt、否则 SF Symbol 16pt
    /// （:1183-1193）—— 探针走的是"没有资源"那一支，所以图标 16pt。
    ///
    /// `pullRequests` 在 macOS **恒不可用**（`LitheFeatureAvailability.githubPullRequests = false`），
    /// 按钮是禁用态、提示固定为"拉取请求集成正在开发中"（:1206,1211；中文见
    /// `Localizable.strings:412`）。图标也是"暂时用最接近的代替"：
    /// 默认图标集里没有 `slider.horizontal.3`（更改）、`arrow.triangle.pull`（拉取请求）、
    /// 数据库圆柱（`cylinder.split.1x2`）三个字形，分别用 `Network` / `Github` /
    /// `HardDrive` 代替（缺字形登记待补进 `docs/DESIGN.md` §九，本阶段只允许改本文件）。
    ///
    /// 下段**待接线**：macOS 是固定 292pt 高的"工具视口"，装模块贡献按钮 + 末尾固定的
    /// 齿轮（`general/gear.svg` → `model.showSettings()`，:1225-1258），条目少时贴底。
    /// 探针没有模块贡献列表，所以只保留贴底的齿轮（`settings` 就是齿轮字形，无需代替）。
    fn render_activity_rail(&self, cx: &App) -> impl IntoElement {
        v_flex()
            // 真机活动栏 38pt（`WorkbenchView.swift:12` 的 `ActivityBarMetrics.width`）；
            // 必须 h_full 才会撑满行高、图标落到顶部，否则它会按内容高度收缩并被行居中。
            .w(px(38.))
            .h_full()
            .min_h_0()
            .p(px(4.))
            .gap(px(4.))
            .items_center()
            .bg(cx.theme().title_bar)
            .child(activity_button(
                cx,
                "rail-project",
                IconName::Folder,
                "项目",
                true,
            ))
            .child(activity_button(
                cx,
                "rail-changes",
                IconName::Network,
                "更改",
                false,
            ))
            // 恒不可用：macOS 里这一项 disabled，提示语固定
            // （`WorkbenchView.swift:1206,1211`）。
            .child(
                activity_button(cx, "rail-pull-requests", IconName::Github, "拉取请求", false)
                    .disabled(true)
                    .tooltip("拉取请求集成正在开发中"),
            )
            .child(activity_button(
                cx,
                "rail-search",
                IconName::Search,
                "搜索",
                false,
            ))
            .child(activity_button(
                cx,
                "rail-database",
                IconName::HardDrive,
                "数据库连接",
                false,
            ))
            // 把齿轮压到底部：macOS 靠固定 292pt 的工具视口 + `alignment: .bottom` 实现。
            .child(div().flex_1())
            .child(activity_button(
                cx,
                "rail-settings",
                IconName::Settings,
                "设置",
                false,
            ))
    }

    /// 右侧活动栏（macOS `pluginActivityBar`，`WorkbenchView.swift:1266-1343`）。
    ///
    /// 度量：宽 40（`WorkbenchLayoutMetrics.rightActivityBarWidth`，:7）、上内边距 4、
    /// 背景 `LitheTheme.titlebar`（:1340-1342 → gpui 的 `theme.title_bar`）；
    /// 按钮与左栏同规格（30×30、圆角 4、行悬停底色，§2.4"行为同左栏"），
    /// 所以直接复用 [`activity_button`]。
    ///
    /// 为什么它在 gpui 里是行的**真实子元素**而不是 overlay：macOS 是
    /// `workspaceArea.padding(.trailing, 40)` + `.overlay(alignment: .trailing)`
    /// （:229-234），那 40pt 本来就占布局宽度，浮层只是画在这 40pt 上。
    /// 把它做成行尾子元素，占位与观感等价，也更符合 gpui 的排版模型。
    ///
    /// 内容：`rightSidebarContributions` 在本探针里就是右 dock 的两页
    /// （`项目统计` / `大纲`）。macOS 同一行还有通知铃铛与插件按钮（:1268-1311），
    /// 探针没有通知中心与插件贡献源，**待接线**，不画假按钮。
    ///
    /// 图标代替（默认图标集只有 `gpui-kit-assets-0.6.6/default-icons.txt` 那 101 个字形）：
    /// 默认图标集里没有 macOS 用的 SF `list.bullet.indent`（大纲），暂用语义最近的
    /// `GalleryVerticalEnd`（一叠条目 ≈ 目录）代替；`项目统计` 是探针临时加的面板
    /// （macOS 没有这个面板，也就没有原始字形），暂用 `ChartPie`（统计图表）代替。
    fn render_right_activity_rail(&self, cx: &Context<Self>) -> impl IntoElement {
        // 选中态取自 dock 的**真实开合状态**而不是另存一个 bool：用户从 dock 自带的
        // 收起按钮（`DockSkin` 默认画）关掉右侧栏后，活动栏的选中态要跟着灭掉。
        let right_dock_open = self.dock_area.read(cx).is_dock_open(DockPlacement::Right);
        v_flex()
            // 真机右活动栏 40pt（`WorkbenchView.swift:7`）；必须 h_full 才会撑满行高，
            // 否则按内容高度收缩并被行居中（左栏踩过同一个坑）。
            .w(px(40.))
            .h_full()
            .min_h_0()
            .pt(px(4.))
            .gap(px(4.))
            .items_center()
            .bg(cx.theme().title_bar)
            .child(
                activity_button(
                    cx,
                    "right-rail-stats",
                    IconName::ChartPie,
                    "项目统计",
                    right_dock_open && self.right_panel == RightPanel::Stats,
                )
                .on_click(cx.listener(|workspace, _, window, cx| {
                    workspace.toggle_right_dock(RightPanel::Stats, window, cx);
                })),
            )
            .child(
                activity_button(
                    cx,
                    "right-rail-outline",
                    IconName::GalleryVerticalEnd,
                    "大纲",
                    right_dock_open && self.right_panel == RightPanel::Outline,
                )
                .on_click(cx.listener(|workspace, _, window, cx| {
                    workspace.toggle_right_dock(RightPanel::Outline, window, cx);
                })),
            )
    }

    /// 状态栏（macOS `statusBar`，`WorkbenchView.swift:1650-1789`）。
    ///
    /// 容器：高 24（`Metrics.statusBarHeight`，`LitheTheme.swift:418`）、水平内边距 9、
    /// 左右两段间距 10、字体 `smallFont`(12)、前景 `secondaryText`、背景 `titlebar`
    /// （:1651-1664）。`StatusBar` 自带 `text_xs`(12) + `muted_foreground` 前景 +
    /// `tokens.status_bar` 背景（`gpui-component-0.6.6/src/status_bar.rs:86-96`），
    /// 与规格同一口径，所以只覆盖高度、内边距与间距。
    ///
    /// 左段是 `editorBreadcrumbs`（:1667-1701），右段是 `detailedStatusItems`
    /// （:1748-1768）。右段顺序：光标位置 → `UTF-8` → `\(tabWidth) spaces` → 只读锁 →
    /// 项目准备状态 → 内存 → 帧率 → Git 状态。
    /// macOS 还有一档 `compactStatusItems`（间距 10、去掉编码/缩进/锁）由
    /// `ViewThatFits` 在窗口变窄时自动切换（:1655-1658,1770-1778）；gpui 侧没有
    /// "按可用宽度二选一"的对应物，**待接线**：等接入分栏/测量能力后按可用宽度
    /// 切换这两档。
    fn render_status_bar(&self, cx: &App) -> impl IntoElement {
        // ---- 左段：面包屑 ----
        //
        // 待接线：macOS 左段把活动文档的相对路径按 `/` 拆分，目录段 `secondaryText`、
        // 末段是 `LitheIcon(kind:, size: 12)` + `primaryText`，段间是 7pt semibold 的
        // `chevron.right`（:1672-1692,1742-1746）。探针读不到活动文档：编辑区把当前
        // 文档放在 `ShellPanel::document`（`panels.rs:108`），该字段是 `panels` 模块私有、
        // 工作台取不到（本阶段只允许改本文件）。所以这里渲染 macOS 的
        // **无活动文档**分支：`folder` 图标 13 + 项目名（:1693-1698）。
        let breadcrumbs = h_flex()
            .gap(px(5.))
            .child(
                Icon::new(IconName::Folder)
                    .w(px(13.))
                    .h(px(13.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(self.project_name.clone());

        // ---- 右段 1：光标行列 ----
        // macOS 是 `EditorCaretPositionLabel`：没有活动文档时文案就是 `1:1`，
        // 等宽数字、点击打开"跳转到行"（`WorkbenchStatusViews.swift:6-21`）。
        // 待接线：`EditorChromeModel` 的光标位置与 Go to Line 对话框都还没接。
        // gpui 没有"只让数字等宽"的开关（SwiftUI 的 `monospacedDigit`），
        // 用主题的等宽字体族近似（`Theme::mono_font_family`，`theme/mod.rs:141`）。
        let caret_position = div()
            .font_family(cx.theme().mono_font_family.clone())
            .child(SharedString::from("1:1"));

        // ---- 右段 4：只读锁 ----
        // macOS 是 28×28 的 `litheIconButton()`，图标 `lock.fill` / `lock.open`，
        // 只读时禁用、提示 "Read-only document" / "Save"（:1753-1762）。
        // 没有活动文档 ⇒ 不是只读 ⇒ 画"解锁"那一支（`lock.open`）。
        // 默认图标集里没有 lock 字形，暂用 `Eye`（只读 = 只能看）代替。
        // 待接线：点击应保存活动文档（现在没有文档也没有保存动作，先不接点击）。
        let read_only_lock = Icon::new(IconName::Eye)
            .w(px(13.))
            .h(px(13.))
            .text_color(cx.theme().muted_foreground);

        // ---- 右段 5：Java 项目准备状态 ----
        // macOS `ProjectPreparationStatusView(compact: true)`：11pt 文字 + 状态图标，
        // 文案取 `phaseTitle(_:)`（`ProjectPreparationStatusView.swift:35-40,81-90`）。
        // 待接线：探针还没有 languageTooling 的项目准备快照，这里先按"已就绪"这个
        // 真实文案占位；接上快照后要照 macOS 的规则决定显示/隐藏 —— compact 模式下
        // `status == "ready"` 时整项是**不渲染**的（`ProjectPreparationStatusView.swift:27`），
        // 失败时换成 "Java 项目准备失败"。图标同理：`checkmark.circle` 用最近字形
        // `CircleCheck` 代替。
        let preparation = h_flex()
            .gap(px(6.))
            .text_size(px(11.))
            .child(
                Icon::new(IconName::CircleCheck)
                    .w(px(12.))
                    .h(px(12.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(SharedString::from("Java 项目模型已就绪"));

        // ---- 右段 6：内存 ----
        // macOS `MemoryUsageStatusView`：图标 `memorychip` + `"Total \(totalText)"` +
        // `·` + `"Lithe \(litheText)"`，间距 4、等宽数字（`WorkbenchStatusViews.swift:31-41`），
        // 文案键是插值键（zh-Hans 目录里只有 `"Total" = "总计"`，`Localizable.strings:1269`）。
        // 占位值 `—` 不是编的：`MemoryUsageMonitor.formatted(nil)` 返回的就是 `—`
        // （`Services/Monitoring/MemoryUsageMonitor.swift:176,280-281`），即"还没采样"的
        // 真实显示；未采样时 `totalText` 走的是 `formatted(0)`（:175,179），
        // 这里也统一按"未采样"的 `—` 占位。
        // 图标：默认图标集里 `memory-stick` 与 SF `memorychip` 是同一语义。
        // 待接线：`MemoryUsageMonitor` 的真实采样。
        let memory = h_flex()
            .gap(px(4.))
            .font_family(cx.theme().mono_font_family.clone())
            .child(
                Icon::new(IconName::MemoryStick)
                    .w(px(13.))
                    .h(px(13.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(SharedString::from("总计 —"))
            .child(SharedString::from("·"))
            .child(SharedString::from("Lithe —"));

        // ---- 右段 7：帧率 ----
        // macOS `FrameRateStatusView`：图标 `speedometer` + `framesPerSecondText`，
        // 文案就是 `"\(framesPerSecond) FPS"`，监视器未跑时是 `0 FPS`
        // （`WorkbenchStatusViews.swift:138-150`；`Services/Monitoring/FrameRateMonitor.swift:35-38`）。
        // 默认图标集里没有 `speedometer`，"帧"字面对应的 `Frame` 是最接近的字形。
        // 待接线：`FrameRateMonitor` 的真实采样。
        let frame_rate = h_flex()
            .gap(px(4.))
            .font_family(cx.theme().mono_font_family.clone())
            .child(
                Icon::new(IconName::Frame)
                    .w(px(13.))
                    .h(px(13.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(SharedString::from("0 FPS"));

        // ---- 右段 8：Git 状态 ----
        // macOS `gitStatus`（:1780-1789）：`HStack(spacing: 7)`，末尾恒有一个
        // `checkmark.circle.fill` 用 `LitheTheme.success`；文案 `"No changes"` /
        // `"\(n) changes"`，中文原文 `"No changes" = "没有更改"`（`Localizable.strings:1399`）。
        // 探针还没跑 `git.status`（`02-workbench-layout.md` §4），所以先按"没有更改"
        // 这一真实文案占位。待接线后换成真实计数，并补上"references 工具窗可见时
        // 先显示 `\(n) usages` + `scope` 图标"（:1782-1784）。
        // 图标：默认图标集里没有 `checkmark.circle.fill`，用 `CircleCheck` 代替。
        let git_status = h_flex()
            .gap(px(7.))
            .child(SharedString::from("没有更改"))
            .child(
                Icon::new(IconName::CircleCheck)
                    .w(px(13.))
                    .h(px(13.))
                    .text_color(cx.theme().success),
            );

        let right = h_flex()
            // `detailedStatusItems` 的间距是 14（:1749）；`StatusBar` 自带的是 8。
            .gap(px(14.))
            .child(caret_position)
            .child(SharedString::from("UTF-8"))
            // macOS 是 `"\(settings.tabWidth) spaces"`，默认 `tabWidth = 4`
            // （`Models/Settings/AppSettings.swift:161`），中文原文 `"4 spaces" = "4 个空格"`
            // （`Localizable.strings:1125`）。待接线：读真实设置值。
            .child(SharedString::from("4 个空格"))
            .child(read_only_lock)
            .child(preparation)
            .child(memory)
            .child(frame_rate)
            .child(git_status)
            // 探针额外项：Core `workspace.snapshot` 返回的文件总数
            // （`shell_probe/files.rs:94` 的 `total`）。macOS 状态栏里**没有**这一项，
            // 保留它是为了验证 Core 数据接线；面板头部不再显示数量（见 `panels.rs:493`）。
            .children(self.snapshot_files.clone());

        StatusBar::new()
            .h(px(24.))
            .px(px(9.))
            .gap(px(10.))
            .left(breadcrumbs)
            .right(right)
    }
}

/// 活动栏按钮：30×30、圆角 4、图标 16pt（macOS `WorkbenchView.swift:1194-1207`：
/// 按钮 30×30、`litheRowHover(cornerRadius: 4, activeBackground: subtleSelection)`；
/// 图标在有 IntelliJ 资源时 18pt、否则 SF Symbol 16pt，:1183-1193）。
///
/// `selected` 用 gpui-kit 的选中态表达 macOS 的 `subtleSelection` 底色；
/// 激活项前景在 macOS 是 `primaryText`（其余 `secondaryText`，:1207），
/// gpui-kit `ghost` 的悬停/选中前景不是这个口径，所以激活项显式覆盖一次。
/// `tooltip` 同时当无障碍标签用（macOS 的 `help` 与 `accessibilityLabel` 都是
/// `destination.title`，:1208-1213）。
fn activity_button(
    cx: &App,
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    selected: bool,
) -> Button {
    let button = Button::new(id)
        .ghost()
        .icon(icon)
        // 图标按钮不进 Tab 焦点序（macOS 里活动栏按钮也不参与键盘焦点环）。
        .tab_stop(false)
        .w(px(30.))
        .h(px(30.))
        .rounded(px(4.))
        .tooltip(tooltip)
        .accessibility_label(tooltip)
        .selected(selected);
    if selected {
        button.text_color(cx.theme().foreground)
    } else {
        button
    }
}

/// 读工作区当前分支：宿主本地读取 `<root>/.git/HEAD`，**不是** Core 契约。
///
/// 待接线：macOS 的分支名来自 Git 会话（`model.currentBranch`，
/// `WorkbenchView.swift:718`），正式实现应走 Core 的 `workspace.repositories`
/// 与 `git.status`（`.artifacts/ui-map/02-workbench-layout.md` §4）。这里先读宿主
/// 真实的 `.git/HEAD` 让标题栏显示真分支；读不到（不是 Git 仓库、worktree 的
/// `.git` 是文件、detached HEAD）就返回 `None`，标题栏不画分支按钮，
/// 而不是写死一个假分支名。
fn read_current_branch(root: &Path) -> Option<SharedString> {
    let head = std::fs::read_to_string(root.join(".git/HEAD")).ok()?;
    let reference = head.trim().strip_prefix("ref: refs/heads/")?;
    (!reference.is_empty()).then(|| SharedString::from(reference.to_string()))
}

// ============================ 命令面板（浮层）============================
//
// 这一块是**本项目用 gpui-kit 补的优化项**：macOS 没有命令面板，最接近的是
// `SearchEverywhereView` 的 `actions` / `commands` 模式（输入 `/` 进入命令模式，
// `SearchEverywhereView.swift:18-26,120-141,267`），唤出靠隐形手势（双击 ⇧ / ⇧⌘O）。
// 补它的动机是 macOS 命令目录里 **13 条没有任何默认键**
// （`.artifacts/ui-map/01-app-shell.md` §3.4）。理由与完整来源见文件头注释与
// `.artifacts/ui-map/09-overlay-howto.md`。

/// 命令面板搜索框的占位文案。macOS 原文 `"Type / to see commands" = "输入 / 查看命令"`
/// （`SearchEverywhereView.swift:267`；`macos/Resources/zh-Hans.lproj/Localizable.strings:1127`）。
const PALETTE_PLACEHOLDER: &str = "输入 / 查看命令";

/// 命令面板"没有匹配"的文案。macOS 原文 `"No matching commands" = "没有匹配的命令"`
/// （`SearchEverywhereView.swift:75-77` 的 `emptyResultsMessage` 的 `.commands` 分支；
/// `Localizable.strings:41`）。
const PALETTE_EMPTY: &str = "没有匹配的命令";

/// 双击 ⇧ 手势识别器：macOS `DoubleShiftGestureRecognizer`
/// （`Platform/MacOS/UI/MacShortcutDetector.swift:256-317`）的 Rust 等价物，逐条对齐它的语义：
///
/// 1. 只认"按下沿 / 抬起沿"，按下时必须没有别的修饰键（`:283-287`）；
/// 2. 按下期间只要出现别的修饰键，这次按下就不再算独立轻点（`:289-295`）；
/// 3. **轻点算在抬起时**：抬起时若这次按下是独立轻点且此刻没有别的修饰键，
///    才和上一次独立轻点比时间差（`:297-312`）；
/// 4. 与 ⇧ 无关的修饰键变化，只清掉待配对的轻点（`:314-316`）；
/// 5. 按下任意普通键 → 作废（`handleKeyDown`，`:267-270`），这就是 `Shift+A` 不会误触的原因。
///
/// 阈值 0.35s 是 macOS 的真源值（`KeyboardShortcutRecorderView.swift:72`、
/// `MacShortcutDetector.swift:155`）。
struct DoubleShiftGesture {
    /// 两次独立轻点的最大间隔（350ms）。
    threshold: Duration,
    /// ⇧ 当前是否按着（对应 macOS 的 `shiftWasDown`）。
    shift_is_down: bool,
    /// 这一次 ⇧ 按下是否还是"独立轻点"（对应 `currentPressIsStandalone`）。
    current_press_is_standalone: bool,
    /// 上一次独立轻点的时间戳（对应 `lastStandaloneTap`）。
    last_standalone_tap: Option<Instant>,
}

impl DoubleShiftGesture {
    fn new() -> Self {
        Self {
            threshold: Duration::from_millis(350),
            shift_is_down: false,
            current_press_is_standalone: false,
            last_standalone_tap: None,
        }
    }

    /// 任何普通键按下都作废当前手势：`Shift+A` 这类"带 Shift 的输入"不是独立 ⇧ 轻点
    /// （`MacShortcutDetector.swift:267-270`）。
    fn handle_key_down(&mut self) {
        self.current_press_is_standalone = false;
        self.last_standalone_tap = None;
    }

    /// 修饰键变化。返回 `true` 表示识别出"双击 ⇧"。参数语义与 macOS 一致：
    /// `is_shift_down` = ⇧ 当前是否按下，`has_other_modifiers` = 是否带 ⌘⌃⌥ 或功能键。
    fn handle_modifiers_changed(
        &mut self,
        is_shift_down: bool,
        has_other_modifiers: bool,
    ) -> bool {
        // 按下沿：这次按下是不是独立轻点，要到抬起时才算数。
        if is_shift_down && !self.shift_is_down {
            self.current_press_is_standalone = !has_other_modifiers;
            self.shift_is_down = true;
            return false;
        }

        // 还按着 ⇧：一旦带上别的修饰键，这次按下就作废。
        if is_shift_down {
            if has_other_modifiers {
                self.current_press_is_standalone = false;
                self.last_standalone_tap = None;
            }
            return false;
        }

        // 抬起沿：这里才是"记一次轻点"的地方。
        if self.shift_is_down {
            self.shift_is_down = false;
            let is_standalone = self.current_press_is_standalone && !has_other_modifiers;
            self.current_press_is_standalone = false;
            if !is_standalone {
                self.last_standalone_tap = None;
                return false;
            }
            let now = Instant::now();
            if let Some(last) = self.last_standalone_tap {
                if now.duration_since(last) < self.threshold {
                    // 配对成功：清掉，免得三次 ⇧ 连点触发两次。
                    self.last_standalone_tap = None;
                    return true;
                }
            }
            self.last_standalone_tap = Some(now);
            return false;
        }

        // 与 ⇧ 无关的修饰键变化：只清掉待配对的轻点，避免"⇧ … ⌘ … ⇧"被算成双击。
        if has_other_modifiers {
            self.last_standalone_tap = None;
        }
        false
    }
}

/// 命令面板里的一条命令。
struct PaletteCommand {
    /// 稳定 id：`on_confirm` 只给 `IndexPath`（`command/command.rs:157-163`），靠它换回是哪个命令。
    id: PaletteCommandId,
    /// 行内标题，取 macOS 的中文原文（目录里的命令取 `Localizable.strings` 的译文，
    /// 与 `LitheCommandCatalog.swift` 的英文标题一一对应），**不自己编**。
    title: &'static str,
    /// 额外搜索词：英文标题 + 命令 id（`CommandItem::keywords`，`command/item.rs:79-88`），
    /// 这样输入 `toggle-terminal` 或 `Toggle Terminal` 也能命中中文标题的命令。
    keywords: &'static [&'static str],
    /// 行首图标。只能用默认图标集：`mod.rs` 注册的是 `gpui_kit::assets::Assets`，
    /// 它只嵌入 `gpui-kit-assets-0.6.6/default-icons.txt` 的 101 个字形，缺字形用最接近的代替。
    icon: IconName,
    /// 本轮是否能真正生效。`false` 的项走 gpui-kit 的禁用态（**不是假按钮**）：
    /// 行文字降为 `muted_foreground`（`command/state.rs:749`）、键盘上下选跳过它（`:534`）、
    /// 点击 / 回车确认被忽略（`:584-590`）。
    enabled: bool,
}

/// 命令面板里的一条命令的稳定 id。[`ShellWorkspace::run_palette_command`] 按它分发。
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaletteCommandId {
    /// 右侧「大纲」工具窗（macOS `rightSidebarContributions` 的一项，`WorkbenchView.swift:1313-1335`）。
    Outline,
    /// 右侧「项目统计」工具窗（**探针临时面板**，macOS 没有它）。
    Stats,
    /// `toggle-terminal`（`LitheCommandCatalog.swift:56`）。
    ToggleTerminal,
    /// `toggle-problems`（:57）。
    ToggleProblems,
    /// `toggle-maven`（:58）。
    ToggleMaven,
    /// `toggle-git-log`（:59）。
    ToggleGitLog,
    /// `toggle-run`（:60）。
    ToggleRun,
    /// `toggle-tests`（:61）。
    ToggleTests,
    /// `toggle-debug`（:62）。
    ToggleDebug,
}

/// 命令面板里的一个分组（组标题 + 组内命令）。
///
/// 组标题取 macOS `LitheActionGroup` 的中文原文（`Models/LitheAction.swift` 的
/// `LitheActionGroup`：`.run/.navigation/.window/.project/.history`；
/// `Localizable.strings:408,1028,1029,7,281`），例如 `.window` → `"Window"` → 「窗口」。
/// 与 macOS 的 Search Everywhere 一致：那里的分组标题也是 `LocalizedStringKey(group.rawValue)`
/// （`SearchEverywhereView.swift:477`）。
/// **不用 Windows 端的 `commandPalette.categories.*`（File/Git/View/…）**：本次界面规格是
/// macOS 源码，Windows 只作旁证（`gpui/UI-MAP.md` §1 开头的"工作方式"）。
struct PaletteGroup {
    label: &'static str,
    commands: &'static [PaletteCommand],
}

// ---------------------------------------------------------------------------
// 待接线清单（本轮**不列进面板**的 macOS 命令，按目录里的作用域分；id 与中文标题都取自
// `Models/Keymap/LitheCommandCatalog.swift` 与 `macos/Resources/zh-Hans.lproj/Localizable.strings`）
//
// `.project`（`LitheCommandCatalog.swift:14-19`）：
//   `open-project` 打开项目（:986）、`save` 保存（:265）、`close-project` 关闭项目（:266）、
//   `settings` 设置（:2）、`rebuild-java-index` Java：重建索引（:1347）、
//   `reveal-in-finder` 在 Finder 中显示（:1026）
//   —— 缺口：本探针没有设置窗口（`settings` 要 `LitheWindowID.settings` 那个独立浮窗，
//      `Views/App/LitheApp.swift:679-746`）、没有 Java 索引后端、没有"当前活动文档"这个状态
//      （编辑区把文档放在 `ShellPanel::document`，`panels.rs:373`，工作台读不到），
//      换工作区根要重建整棵 Dock 布局，本轮不做。因此「关闭编辑器标签」这类命令也一并搁置。
// `.run`（:21-30）：`run` 运行（:408）、`debug` 调试（:635）、`stop-run` 停止运行（:919）、
//   `stop-debug` 停止调试（:921）、`debug-resume` 调试：继续（:923）、`debug-step-over` 调试：步过（:925）、
//   `debug-step-into` 调试：步入（:927）、`debug-step-out` 调试：步出（:929）、
//   `toggle-breakpoint` 切换行断点（:1012）、`view-breakpoints` 查看断点（:931）
//   —— 缺口：没有运行配置、执行状态与断点存储（`docs` 里的运行/调试契约都还没接线）。
// `.navigation`（:32-54）：`search-everywhere` 全局搜索（:63）、`navigate-back` 后退（:46）、
//   `navigate-forward` 前进（:48）、`find-in-file` 在文件中查找（:1016）、`find-next` 查找下一个（:275）、
//   `find-previous` 查找上一个（:276）、`replace-in-file` 在文件中替换（:1018）、`go-to-line` 跳转到行（:1020）、
//   `go-to-definition` 跳转到定义（:50）、`go-to-implementation` 跳转到实现（:278）、
//   `find-usages` 查找调用位置（:279）、`search-in-project` 在项目中查找（:57）、
//   `replace-in-project` 在项目中替换（:58）、`spring-endpoints` Spring 接口（:52）
//   —— 缺口：没有编辑区历史栈 / 查找状态 / 符号索引 / 项目级搜索面板，也没有语言智能后端。
//   `search-everywhere` 本身很特殊：它的作用就是"打开这个面板"，面板里再列它等于自己调自己，
//   所以不列（入口是双击 ⇧ 与项目标签条右端的按钮）。
// `.history`（:64-65）：`local-history` 本地历史（:680）、`project-local-history` 项目本地历史（:681）
//   —— 缺口：没有本地历史存储。
// ---------------------------------------------------------------------------

/// 命令面板的分组表。**只列"能真正生效"的命令 + 同一作用域里被同一类缺口挡住的目录命令**，
/// 其余按上面的"待接线清单"登记在注释里，不列成整屏灰项。
///
/// 为什么只有「窗口」一组：其余四个作用域在本探针里连数据源都没有（见上面的清单），
/// 全做成禁用项只会淹没真正能用的两条。分组表是数据驱动的（`&[PaletteGroup]`），
/// 某个作用域接上数据后加一个 [`PaletteGroup`] 即可，`on_confirm` 的坐标映射
/// （[`palette_command_at`]）不用改。
static PALETTE_GROUPS: &[PaletteGroup] = &[PaletteGroup {
    label: "窗口",
    commands: &[
        // 前两条是本轮**真正生效**的命令：它们就是右侧停靠区 `项目统计` / `大纲` 的开关
        // （等同右活动栏的两个按钮，`toggle_right_dock`）。
        // `大纲` 是 macOS 右侧栏贡献项（`WorkbenchView.swift:1313-1335`，标题来自模块贡献），
        // 本工程沿用它作为右 dock 第二页的标题（见 `render_right_activity_rail`）；
        // `项目统计` 是探针为了验证 `DataTable` 拿真实快照数据加的临时面板（`gpui/PLAN.md`），
        // **macOS 没有这个面板**，它出现在这里只是因为它确实能生效。
        PaletteCommand {
            id: PaletteCommandId::Outline,
            title: "大纲",
            keywords: &["outline"],
            // 与右活动栏同一字形（默认图标集没有 macOS 用的 SF `list.bullet.indent`）。
            icon: IconName::GalleryVerticalEnd,
            enabled: true,
        },
        PaletteCommand {
            id: PaletteCommandId::Stats,
            title: "项目统计",
            keywords: &["stats"],
            // 与右活动栏同一字形（默认图标集没有对应字形，用统计图表代替）。
            icon: IconName::ChartPie,
            enabled: true,
        },
        // 下面 7 条是 macOS `.window` 组的**全部**目录命令（`LitheCommandCatalog.swift:56-62`）。
        // 它们的处境正是补命令面板的理由：macOS 里这 7 条**一个默认键都没有**
        // （`.artifacts/ui-map/01-app-shell.md` §3.4），只能靠鼠标在 UI 里找。
        // 但探针里它们都还不能真正生效，所以一律 disabled：
        //   - 终端：gpui-kit **没有**终端模拟器 / PTY（`gpui/UI-MAP.md` §1.3 的能力缺口）；
        //   - 问题窗口：没有 languageTooling 诊断快照；
        //   - Maven：没有 Maven 面板与索引；
        //   - Git 日志：底部工具窗就是它，但 0.6.6 **没有**运行时面板可见性 API
        //     （`Panel::visible()` 只是只读查询，`gpui-base-0.6.6/src/dock/panel.rs:28,103`，
        //     唯一的写入口 `set_visible` 在 `test_support.rs:94` 是 `pub(crate)`；
        //     `DockArea::remove_panel` 又不返回面板视图，`dock_area.rs:468-476`），
        //     而底部工具窗位于 center 的 `v_split` 里，重新加回会改动其它区域；
        //   - 运行 / 测试 / 调试：没有对应的面板与执行状态。
        PaletteCommand {
            id: PaletteCommandId::ToggleTerminal,
            title: "切换终端",
            keywords: &["Toggle Terminal", "toggle-terminal"],
            icon: IconName::SquareTerminal,
            enabled: false,
        },
        PaletteCommand {
            id: PaletteCommandId::ToggleProblems,
            title: "切换问题窗口",
            keywords: &["Toggle Problems", "toggle-problems"],
            icon: IconName::TriangleAlert,
            enabled: false,
        },
        PaletteCommand {
            id: PaletteCommandId::ToggleMaven,
            title: "切换 Maven",
            keywords: &["Toggle Maven", "toggle-maven"],
            // 默认图标集里没有 Maven 字形，用"构建"语义的 `Building2` 代替。
            icon: IconName::Building2,
            enabled: false,
        },
        PaletteCommand {
            id: PaletteCommandId::ToggleGitLog,
            title: "切换 Git 日志",
            keywords: &["Toggle Git Log", "toggle-git-log"],
            // 与底部工具窗头部同一字形（`bottom_panel.rs` 用 `BookOpen` 代 Git 字形）。
            icon: IconName::BookOpen,
            enabled: false,
        },
        PaletteCommand {
            id: PaletteCommandId::ToggleRun,
            title: "切换运行窗口",
            keywords: &["Toggle Run", "toggle-run"],
            icon: IconName::Play,
            enabled: false,
        },
        PaletteCommand {
            id: PaletteCommandId::ToggleTests,
            title: "切换测试窗口",
            keywords: &["Toggle Tests", "toggle-tests"],
            // 默认图标集里没有测试字形，用"通过"语义的 `CircleCheck` 代替。
            icon: IconName::CircleCheck,
            enabled: false,
        },
        PaletteCommand {
            id: PaletteCommandId::ToggleDebug,
            title: "切换调试窗口",
            keywords: &["Toggle Debug", "toggle-debug"],
            // 默认图标集里没有 bug / 调试字形，用"检查"语义的 `Inspector` 代替。
            icon: IconName::Inspector,
            enabled: false,
        },
    ],
}];

/// 把 [`PALETTE_GROUPS`] 渲染成 `Command` 的条目。
///
/// `Command` 是 `RenderOnce`（`command/command.rs:57,240`）且 Dialog 的构造闭包是 `Fn`，
/// 每个效果周期都会重建一次，所以这里现算即可，不做缓存（`CommandItem::child` 那种
/// "可能被多次调用、必须无副作用"的约束这里也天然满足，`command/item.rs:90-104`）。
fn palette_groups() -> impl Iterator<Item = CommandGroup> {
    PALETTE_GROUPS.iter().map(|group| {
        let mut rendered = CommandGroup::new().label(group.label);
        for command in group.commands {
            rendered = rendered.item(
                CommandItem::new()
                    .label(command.title)
                    // `IconName` 是 `Clone` 但不是 `Copy`（`component/icon.rs:17-20`）。
                    .icon(command.icon.clone())
                    .keywords(command.keywords.iter().copied())
                    .disabled(!command.enabled),
            );
        }
        rendered
    })
}

/// 把 `Command::on_confirm` 给的 `IndexPath` 换回命令表里的那条命令。
///
/// 坐标语义以源码为准（`command/state.rs:369-404`）：显式分组用**组序号 + 组内序号**，
/// 面板里只有分组、没有散装条目，所以 `section` 就是 [`PALETTE_GROUPS`] 的下标；
/// **本地过滤不改变这两个坐标**（被过滤掉的条目不进 `matched`，但 `item_ix` 仍是原下标，
/// `command/command.rs:140-144`），且被整组过滤掉的组照样占用自己的组序号
/// （`state.rs:370-382`，`group_ix += 1` 在 `visible.is_empty()` 判断之前）。
fn palette_command_at(index: IndexPath) -> Option<&'static PaletteCommand> {
    PALETTE_GROUPS.get(index.section)?.commands.get(index.row)
}

impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 诊断输出保留，便于一眼看出视口与缩放的对应关系。
        let viewport = window.viewport_size();
        let scale = window.scale_factor();
        println!(
            "S1_VIEWPORT viewport={:?}x{:?} scale={}",
            viewport.width, viewport.height, scale
        );

        // ---- 浮层三层（Dialog / Sheet / Notification）----
        //
        // 这三层**必须由业务视图自己挂**：`Root::render` 只内联了 tooltip、原生菜单
        // 兜底与触摸选择三层，Dialog / Sheet / Notification 是静态方法、不在 `Root::render`
        // 里（`gpui-component-0.6.6/src/root.rs:185-295`，`:586-602` 的 render 没有它们）。
        // 不挂的后果是**静默失败**：`window.open_dialog` 不报错、不 panic，屏幕上什么都不
        // 出现（源码注释原话见 `dialog/dialog.rs:287-289`；调研原文
        // `.artifacts/ui-map/09-overlay-howto.md` §1 与 `gpui/UI-MAP.md` §1.3）。
        // 官方形态照 `gpui-kit-0.6.6/tests/overlays.rs:20-22,84-86`。
        //
        // 自检：挂层之后 `window.open_dialog(cx, |dialog, _, _| ...)`（以及
        // `open_sheet` / `push_notification`）才会真正显示。命令面板是本阶段**唯一**的触发者，
        // 入口有两个：项目标签条右端的按钮（鼠标验证）与双击 ⇧（`on_root_modifiers_changed`）。
        //
        // 三个 `let` 必须放在下面那些 `render_*` **之前**：它们要求 `&mut App`，
        // 而 `render_*` 返回的 `impl IntoElement` 会持有 `cx` 的共享借用（E0502）。
        // 三个层函数的返回类型是 `use<>`（不捕获任何生命周期），所以借用在语句结束就还回去了。
        let dialogs = Root::render_dialog_layer(window, cx);
        let sheets = Root::render_sheet_layer(window, cx);
        let notifications = Root::render_notification_layer(window, cx);

        let title_bar = self.render_title_bar(cx);
        let activity_rail = self.render_activity_rail(cx);
        let right_activity_rail = self.render_right_activity_rail(cx);
        let status_bar = self.render_status_bar(cx);

        div()
            .relative()
            // ✅ **根视图用 `size_full()`**（= 父容器 100%），与参考应用 Dodona 一致
            // （`Dodona/crates/dodona/src/app/mod.rs:153-157`）。
            //
            // ⚠️ 曾经写成 `.w(viewport_size().width / scale_factor())`，那是**错的**：在 125% 缩放的
            // 机器上 `viewport_size()` 报的是 DPI 无关的客户区尺寸（1440×864），再除以 1.25 得到
            // 1152×691 —— 于是内容只画到窗口的 **78%**，右侧与底部留出大片空白（维护者一直说的
            // "没占满"）。真相是：`px()` 就是那套 DPI 无关单位，`viewport_size()` 已经是它，
            // **再除一次 scale_factor 才是错的**。
            //
            // 这条结论第一次没查出来，是因为当时的截图工具是 DPI 不感知的：`PrintWindow` 只抓回
            // 虚拟化尺寸（1458×819），恰好裁掉了空白，让内容看起来"铺满了"。
            // 现在 `gpui/capture-screenshot.ps1` 会先声明 per-monitor-v2，抓到真实 1823×1024，
            // 证据：`.artifacts/p1/dpi-aware-window.png`（修前，内容 78%）与修后的对比。
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(title_bar)
            // 项目标签条：macOS 的 `projectTabBar` 是 `tabHeight(34) + 4 = 38`
            // （`WorkbenchView.swift:589`；`LitheTheme.swift:415`），背景 `toolHeader`
            // （:590），标签里是 `folder.fill` 图标 + 项目名（:601-615）。
            // **标签的选中态/下划线/关闭按钮不属本次范围，保持现状**；
            // 这里只把写死的 "Lithe-IDEA" 与 `PanelLeft` 换成真实项目名与规格里的
            // `folder` 字形（默认图标集里没有 `folder.fill`，`Folder` 是同一语义）。
            .child(
                h_flex()
                    .h(px(38.))
                    .px_2()
                    .gap_2()
                    .bg(cx.theme().tab_bar)
                    .child(
                        h_flex()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(cx.theme().tab_active)
                            .child(Icon::new(IconName::Folder))
                            .child(self.project_name.clone())
                            .child(Icon::new(IconName::Close)),
                    )
                    // 命令面板的**可见入口**（浮层的另一个入口是双击 ⇧，见
                    // `on_root_modifiers_changed`）。macOS 没有命令面板，它的 Search Everywhere
                    // 只有隐形手势（双击 ⇧ / ⇧⌘O，`LitheCommandCatalog.swift:32-41`），
                    // 这里额外给鼠标留一个入口，方便维护者不用键盘也能验证浮层。
                    // 图标与提示文案取 macOS 的中文原文（`"Search Everywhere" = "全局搜索"`，
                    // `Localizable.strings:63`；用 `Search` 字形与左活动栏的「搜索」一致）。
                    .child(div().flex_1())
                    .child(
                        Button::new("open-command-palette")
                            .ghost()
                            .icon(IconName::Search)
                            .tab_stop(false)
                            // macOS 图标按钮 28×28、圆角 5（`Theme/LitheTheme.swift:460-497`）。
                            .w(px(28.))
                            .h(px(28.))
                            .rounded(px(5.))
                            .tooltip("全局搜索")
                            .accessibility_label("全局搜索")
                            .on_click(cx.listener(|workspace, _, window, cx| {
                                workspace.open_command_palette(window, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(activity_rail)
                    // DockArea 需要整行高度：包裹层给 `h_full()`。
                    // 少了它，Dock 区域会因为拿不到确定高度而完全不渲染
                    // （这是加活动栏时引入的回归）。
                    .child(div().flex_1().h_full().min_h_0().child(self.dock_area.clone()))
                    // 右活动栏放在行尾：macOS 的 `rightHoverRegion` 是
                    // `HStack { 右侧栏; pluginActivityBar }` 贴窗口右缘（:1349-1389），
                    // 所以停靠的右侧栏在活动栏**左边**。gpui 的
                    // `DockPlacement::Right` 画在 DockArea 自己的右缘，
                    // 也就是这条活动栏的左边，顺序与真机一致。
                    .child(right_activity_rail),
            )
            .child(status_bar)
            // 双击 ⇧ 的按键 / 修饰键监听挂在**根元素**上：根 div 是所有区域（含浮层，
            // 三层是它的子元素）的祖先，事件冒泡一定经过它。
            // 两个 listener 的语义、平台依据与"为什么要 `track_focus`"见
            // `on_root_key_down` / `on_root_modifiers_changed` 的注释。
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_root_key_down))
            .on_modifiers_changed(cx.listener(Self::on_root_modifiers_changed))
            // 三层浮层挂在根元素**末尾**：它们都是 `absolute`，靠后画的才盖在内容上面。
            .children(dialogs)
            .children(sheets)
            .children(notifications)
    }
}
