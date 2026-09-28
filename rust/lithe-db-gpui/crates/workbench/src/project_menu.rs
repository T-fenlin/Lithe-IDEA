//! 标题栏「项目下拉」面板（Windows 规格 → gpui 落地）。
//!
//! 规格真源（一手源码，逐条核对过行号）：`lithe-db-gpui/research/windows/11-project-menu.md`
//! （下称「研究」），对应 Windows 前端
//! `windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx`（239 行）。
//!
//! # 面板结构（3 段 + 2 条分隔线）
//!
//! | 段 | 内容 | 行高 | 真源 |
//! | --- | --- | --- | --- |
//! | ① 动作 | 新建项目… / 打开… / 克隆仓库… | 32 | `title-project-menu.tsx:172-192` |
//! | — | 分隔线 | — | `:194` |
//! | ② 打开的项目 | 分组标题 + 每个已打开项目一行（当前项高亮 + 右侧勾） | 44（徽标 28） | `:195-214` |
//! | — | 分隔线 | — | `:216` |
//! | ③ 最近项目 | 分组标题 + 空态一行（v1 恒空） | 空态 `py-3` | `:217-235` |
//!
//! 面板度量：`w-96`(384) / `max-h min(32.5rem, 100vh-3rem)`(520) / `p-1.5`(6) / `rounded-md`(6.4)
//! —— `title-project-menu.tsx:170`。本侧用 `PopupMenu`，它的内边距是**硬编码 4**、行间距
//! **硬编码 2**、圆角取 `theme.radius`(6)（`gpui-component-0.6.6/src/menu/popup_menu.rs:1483,1484,1460`），
//! 三个值都没有 builder —— 这是已知且**未解**的 −2 / +2 / −0.4 偏差（研究 §5.3 已登记同一条）。
//!
//! # 项目徽标
//!
//! 无 `customIcon` 时取**首字母徽标**：名字按「非字母数字」切词 → 前 2 个词的第 1 个
//! **码点** → 大写 → 空则兜底 `LI`；配色 = `hash(name) % 5`（Java 式 31 进制、每步
//! `& 0x7fffffff`）。逐行照抄 `utils/title-project-menu-model.ts:14-31`，
//! 单测对齐 `title-project-menu-model.test.ts:75-81`（另加了研究里用 node 复算过的两组样本）。
//!
//! ⚠️ **5 个色值是写死的十六进制**（[`BADGE_TONES`]），不走 `cx.theme()`：真源本身就是
//! 常量数组 `bg-sky-600 / bg-emerald-600 / …`（`title-project-menu-model.ts:6-12`），
//! 与主题、语言、git 状态**无关**；gpui-kit 的 `ThemeColor` 里没有这 5 项
//! （`gpui-component-0.6.6/src/theme/theme_color.rs`）。这是本模块**唯一**允许写裸色值的地方，
//! 每个色值都对着 Tailwind v4 默认色板抄并有单测钉住。
//!
//! # 触发器：图标按维护者截图走徽标（与真源代码不一致）
//!
//! 真源触发器的图标**写死**应用 logo：`<img src="/logo.png" class="size-5 …">`
//! （`title-project-menu.tsx:155`），项目徽标只出现在**面板内每一行**（`:99`）。
//! 但维护者截图里的触发器显示的是**项目首字母徽标**（研究 §7 第 1 条把这条不一致登记为待拍板项）。
//! 本侧按截图做：触发器左侧画 20×20 的徽标（真源那个容器的尺寸，`:151-156`），**不画 logo**。
//! 其余触发器度量照真源：`h-6`(24)、`gap-1.5`(6)、`px-2`(8)、`max-w-56`(224)、
//! 文字 `truncate`、右侧 chevron 14×14 展开时旋转 180°（`:139-164`）。
//!
//! # 字号：路径 / 分组标题取 12px（`text_xs()`）
//!
//! 真源这两处的类名是 `ui-text-xs`，而**该类在仓库里未定义**
//! （`windows/tauri/src/styles/utilities.css:30-44` 只有 `-sm/-caption/-chrome/-base`，
//! 全仓库 `.css` grep 无 `.ui-text-xs`），所以真机实测是**继承父级的 13px**
//! （`ui/dropdown.tsx:753,782`）—— 13px 是"类名写错"的意外结果，不是设计意图（研究 §7 第 2 条）。
//! 本侧按**意图**取 12px：`text_xs()`（gpui 的档位 12；`--app-ui-font-size` 13 不在档位上，
//! 与 `crate::title_bar` 的 13→14、`crate::project_tabs` 同一口径）。
//! ⚠️ **唯一没有按 12px 落地的是两个分组标题**：见下。
//!
//! # 为什么不用 `Button::dropdown_menu_with_anchor`
//!
//! 研究 §5.3 推荐它（`DropdownMenu` → `Popover(appearance=false)` → `PopupMenu`），
//! 它的下限也确实更好（锚定、点外关闭、`Esc`、键盘导航、焦点归全都现成）。**但本侧不能用**：
//!
//! 它把开合状态放在 `DropdownMenuPopover` 内部的 `window.use_keyed_state`
//! （`gpui-component-0.6.6/src/menu/dropdown_menu.rs:118-150`，类型私有），
//! **没有"以编程方式展开"的入口** —— 本仓库已经在 `crates/settings/src/row.rs:29-31` 记过同一条
//! （"gpui 的 `Button::dropdown_menu` 没有'以编程方式展开'的入口"）。
//! 而本项目的工作站**锁屏**、鼠标/键盘注入到不了应用，验证必须有一条**启动态就把面板画出来**的
//! 通路（与 `crate::menu_bar` 的 `--menu-probe` 同一性质）：没有编程式开合，就截不到新帧、
//! 也拿不到"面板真的被画出来"的证据。
//!
//! 所以本模块自持一个 `open` 状态（[`ProjectMenu`]），把面板画成
//! `deferred(Positioner::side(触发器 bounds).placement(Bottom).align(Start).offset(4))`
//! —— 与真源的 `side="bottom" align="start" sideOffset=4` 一一对应（见 [`popup_for`] 的文档，
//! 那里也记了"为什么不用 `anchored()`"的实测理由）。
//! **点击回调与诊断入口调的是同一个** [`ProjectMenu::toggle`] / [`ProjectMenu::open_by_probe`]，
//! 被绕开的只有"操作系统把这次点击送进窗口"那一段。
//! `Esc` / `↑↓` / `Enter` / 点面板外关闭同样现成：它们由 `PopupMenu` 自己提供，
//! 本模块只负责在面板建出来时把焦点交给它（照 `AppMenu::build_popup_menu` 的做法，
//! `gpui-component-0.6.6/src/menu/app_menu_bar.rs:169-204`）。
//!
//! # ⚠️ 触发器必须是拖拽区的**兄弟节点**
//!
//! Windows 的命中测试取 `window_control_hitboxes` 里**第一个**命中项
//! （`gpui-pre-0.3.6/src/window.rs:1952-1956`，按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢 ——
//! 把触发器放进 `drag_region` 内部，点它就只会拖窗口。这一条与窗口三键、菜单栏是同一个坑
//! （`crate::title_bar` 模块头有完整说明），所以 [`crate::title_bar::title_bar`] 把本模块画出来的
//! 元素插在 `drag_region` **之前**，作为它的兄弟。触发器另外挂了一句
//! `window.prevent_default() + cx.stop_propagation()`（`AppMenuBar` 的同一句，
//! `app_menu_bar.rs:272-280`）作为第二道保险。
//!
//! # ⚠️ 接线时最容易再踩的两个实测坑（本轮都踩过、都有截图取证）
//!
//! 1. **`anchored()` 不要用于这张面板**。`deferred(anchored().anchor(Anchor::BottomLeft)…`
//!    （`crate::menu_bar` 的写法）取的是**锚点元素自己在布局里的静态位置**，本侧实测它把面板
//!    顶到了**窗口左上角**：面板的 `top` 落在触发器顶上，把触发器整个盖住（截图里只露出徽标
//!    的 1px 宽一条）。改用 `Positioner::side(触发器 bounds).placement(Bottom).align(Start)`
//!    —— 这正是 gpui-kit 自己给 Select / Combobox / DatePicker 用的那一支
//!    （`gpui-component-0.6.6/src/popover.rs:33-39`），摆位与真源
//!    `side="bottom" align="start" sideOffset=4` 一一对应（`title-project-menu.tsx:168-169`）。
//! 2. **量触发器的那个 `on_prepaint` 必须挂在"无内边距的包装层"上**。挂在触发器自己身上时
//!    它给的是**内容盒**（触发器有 `px_2`），于是面板整体右移 8 逻辑 px（实测：触发器盒子
//!    `x=487`，面板左边跑到 `497`）。挂在无 padding/margin/border 的包装 `div` 上，
//!    `bounds` 与触发器的视觉盒子逐像素相等 → 面板左边与触发器左边严格对齐（都是 `487`）。
//!    —— 这两条都在 `.artifacts/p8/NOTES.md` §4 有实测记录与截图。
//!
//! # 范围（B4 + B3 之后）
//!
//! | 面板里的东西 | 现状 | 说明 |
//! | --- | --- | --- |
//! | 三条动作行的**外观 / 文案 / 图标** | ✅ 画出来 | 三条**都可点**：「打开…」B4 起是**真接线**（换项目链路）；「克隆仓库…」B3 起、「新建项目…」B3 之后**都是占位项**（点了给"尚未接入：缺 X"，不是真做） |
//! | 「打开的项目」的当前项目行（高亮 + 勾） | ✅ | — |
//! | 「最近项目」的列表 / 空态 | ✅ B4 起是**真数据**（`lithe_db_gpui_settings::recent_projects`，落 `%APPDATA%\Lithe\recent-projects.json`；空态沿用 `noRecentProjects`） | 行**可点** → 换项目链路 |
//! | 切换到别的项目 | ✅ B4：走 `ShellWorkspace::request_open_project`（重建整个 `ShellWorkspace`） | |
//! | 打开…（系统目录对话框） | ✅ B4：`ShellWorkspace::open_project_picker`（gpui 自带的 `prompt_for_paths`） | |
//! | 新建项目… | ⚠️ **占位项**（B3 之后补） | 缺的是项目脚手架生成（真机是 `createNewDirectory` + 起终端跑 `npm create`）；能说成一句"缺什么"就与克隆仓库统一成占位项，用它自己那条能力组（[`crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING`]），见下 |
//! | 克隆仓库… | ⚠️ **占位项**（B3） | Core 的 `git.write` **已含** clone，缺的是 URL / 凭据 / 进度 UI —— 点了给那句话（[`crate::menu_bar::MISSING_CLONE_UI`]），不是"没有能力" |
//!
//! ## 为什么两条「缺能力」的动作行都是占位项
//!
//! 真源三条**恒可执行、不置灰**（`title-project-menu.tsx:172-192`），本侧两条缺东西。
//! 判据是**"能不能说清缺什么"**：能说成一句能力话的画成**可点的占位项**，说了等于没说的
//! 才是真禁用态 —— 维护者拍板（同一张面板里两种"不可用"表达方式属于不一致）。
//!
//! - 「克隆仓库…」（B3）：规格 `lithe-db-gpui/research/menu-and-open-project-plan.md` §B3 的注把它收进
//!   「按缺什么给提示」那张表（Q16 也是"先占位、不做最小版"），并且**要求那句话写成
//!   "Core 的 `git.write` 已含 clone，缺的是 URL / 凭据 / 进度 UI"** —— 能力在 Core 里，
//!   只是没有它外面那层界面。
//! - 「新建项目…」（B3 之后）：缺的是**项目脚手架生成**（真机是 `createNewDirectory` 建目录 +
//!   起终端跑 `npm create`，`windows/tauri/src/features/project-picker/new-project-content.tsx:239-277`）。
//!   它确实**不是**"能力已在、只缺 UI"，但"缺脚手架生成"本身就是一句诚实的能力话 ——
//!   于是与克隆仓库统一：可点、给提示，用**它自己那条**能力组
//!   （[`crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING`]，诊断里是
//!   `precondition=no_scaffolding`），**不借**克隆仓库那句。
//!
//! 两条的行为因此逐字一致：点了打一行
//! `S1_PROJECT_MENU action=<id> state=not_wired missing=<能力组 id>` ＋ 状态栏那句"尚未接入：缺 X"
//! （走菜单栏同一套 [`Missing`] 文案机制，见 [`crate::menu_bar::Missing`]）。
//!
//! ⚠️ **为什么不做最小版**（Q16，2026-09-26 维护者口径，仍然有效）：真机的新建项目是一条完整
//! 模态链路（选源 → 校验目标目录 → `createNewDirectory` → 起终端跑脚手架 → 打开项目），
//! 最小版只能做到"建一个空目录再打开" —— 那既不是真源行为，又把"缺脚手架"藏了起来，
//! 所以 v1 只占位，不假装做了。
//!
//! ⚠️ **真禁用态这一档没有消失**：动作行在外壳句柄丢了（窗口正在关）时仍走 `.disabled(true)`，
//! 面板里另有**非交互**行（分组标题 / 当前项目行 / 最近项目空态）同样用 `disabled(true)` 表达
//! "可画不可点" —— `disabled(true)` 的效果是不挂点击、不进键盘导航、前景 `muted_foreground`、
//! 无 hover 高亮（`menu_item.rs:115-133`），于是"不可用"在**语义与视觉上都能区分**
//! （不是只靠颜色）。逐条理由见 [`action_row`]。
//!
//! # 已知偏差（逐条给理由，都不是遗漏）
//!
//! | 偏差 | 真源 | 本侧 | 理由 |
//! | --- | --- | --- | --- |
//! | 面板内边距 / 行间距 / 圆角 | 6 / 0 / 6.4 | 4 / 2 / 6 | `PopupMenu` 三个值都硬编码且无 builder（见上） |
//! | 三条动作项 | 恒可执行、不置灰 | 三条**都可点**（B4 / B3 起），其中「新建项目…」「克隆仓库…」是**占位项**（点了给"尚未接入：缺 X"） | 本侧缺能力；把"缺什么"说成一句能力话，比留一个点不动的灰行更诚实（见 [`action_row`]） |
//! | 行内可用宽度 | 384 − 12(面板内距) − 16(行内距) ≈ 356 | 384 − 20(滚动条 16) − 8 − 16 ≈ 340 | `scrollable(true)` 的竖向滚动条**常驻**占 16 逻辑 px（`.artifacts/p8` 量到右侧 947..966 物理那一条），真源是 `overflow-y-auto`（无溢出时无条）。留着它是因为 `max_h` 就加在这个滚动容器上：**内容再高也撑不破 520**（12 条最近项目 ≈ 900 逻辑也照样被截住并滚动），换成"需要时才滚动"等于自己重算一遍内容高 |
//! | 项目行高 | `min-h-11`(44) | 量到 **52.8 逻辑**（44 只是下限，没被触发） | `min-h` 是**下限**：两行文字（14px 名 + 12px 路径，行高 ~1.4–1.5）+ `py-1.5`(12) 撑到 ~53。真源同样由内容撑高（13px 两行 ≈ 50）；差 ~3 是本侧 13→14 字号的既定口径（见 [`project_row`]） |
//! | 当前项目行 / 空态行 / 分组标题 | 可键盘高亮（`Menu.Item`） | `.disabled(true)`，跳过键盘导航 | 真源点当前项是 no-op 且**不关面板**（`:207-210`），而 `PopupMenuItem` 一旦可点，`confirm` 里**无条件** `dismiss`（`popup_menu.rs:875-884`）—— 只有 disabled 能表达"可画不可点" |
//! | 触发器展开态 | 无底色（只有 hover） | 无底色（同上） | 本模块自绘触发器，不接 `Popover::trigger` 的 `selected(is_open)`（那会给一个 `secondary_active` 蓝底，`button.rs:1245`） |
//! | 徽标配色 / 徽标字号 | 常量 5 色 / 10px | 同 | 真源即常量；10px 不在档位上，写 `rems(10./16.)` |
//!
//! 分组标题**没有**偏差：真源 `ui-text-xs` 未定义 → 真机实际 13px，本侧按意图取 12px
//! （`text_xs()`），见 [`group_label`]。
//!
//! 面板几何在 125% DPI 下**逐项量过**（`.artifacts/p8/geometry.ps1`，物理像素 = 逻辑 × 1.25）：
//! 面板宽 `487..966` = 480 物理 = **384 逻辑** ✓、面板左边 = 触发器左边 `487` ✓
//! （真源 `align="start"`）、面板上边 = 触发器下边 + 4 物理 ✓（真源 `sideOffset` 4）、
//! 触发器徽标 25 物理 = 20 逻辑 ✓、行内徽标 35 物理 = 28 逻辑 ✓、
//! 三条动作行的图标中心间距 42 物理 = 33.6 逻辑 ≈ 32 + gap 2 ✓。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{Align, ElementExt as _, Placement, Positioner, h_flex, v_flex};
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, ClickEvent, Context, DismissEvent, Entity,
    FocusHandle, Focusable as _, FontWeight, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Pixels, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, deferred, div, radians, rems, rgb,
};

use lithe_db_gpui_shared::{tr, tr_args};
use lithe_db_gpui_settings::RecentProject;

use crate::menu_bar::Missing;
use crate::workspace::ShellWorkspace;

// ---------------------------------------------------------------------------
// 度量常量（规格值逐条来自 title-project-menu.tsx，见模块头表格）
// ---------------------------------------------------------------------------

/// 面板宽 384：`w-96`（`title-project-menu.tsx:170`）。
///
/// 384 不在 gpui 的固定 rem 档位表上（`gpui-pre-macros-0.3.6/src/styles.rs:926-1158`），而
/// `PopupMenu::min_w` / `max_w` 是**固有方法**、收 `impl Into<Pixels>`（它们遮蔽了 `Styled` 的
/// 同名方法），gpui 又**没有 `impl From<Rems> for Pixels`**（`crate::menu_bar` 的
/// `PANEL_MIN_WIDTH_SPEC` 记过同一条），所以要经 [`crate::rem_px`] 按**运行时** rem 基准换算一次。
const PANEL_WIDTH_SPEC: f32 = 384.;

/// 面板最大高 520：`max-h-[min(32.5rem,calc(100vh-3rem))]`（`title-project-menu.tsx:170`）。
///
/// 真源还有一半是"视口高 − 48"，gpui 的 builder 收不到视口，所以只表达 520 —— v1 面板只有
/// 5 行（约 250px），两半都够不着，这个简化在当前内容下不可见。
const PANEL_MAX_HEIGHT_SPEC: f32 = 520.;

/// 触发器最大宽 224：`max-w-56`（`title-project-menu.tsx:146`）。
const TRIGGER_MAX_WIDTH_SPEC: f32 = 224.;

/// 徽标字号 10：`text-[10px]`（`title-project-menu.tsx:62`）。档位外的值，写 `rems(10./16.)`。
const BADGE_FONT_SIZE_SPEC: f32 = 10.;

/// 面板与触发器之间的间隔 4：真源 `sideOffset` 的默认值（`ui/dropdown.tsx:735-737`）。
const PANEL_OFFSET_SPEC: f32 = 4.;

/// 面板与窗口边至少留 8：真源 `collisionPadding`（`ui/dropdown.tsx:737`）。
const PANEL_WINDOW_MARGIN_SPEC: f32 = 8.;

/// 圆角 6.4：真源的 `rounded-md` = `--radius × 0.8` = `8 × 0.8`（`styles/theme.css:7,134`）。
///
/// 6.4 不在 4px 网格上，gpui 的 `rounded_md()` 是 6（Tailwind 默认梯度），
/// 也不能从 `ThemeConfig.radius` 读（那是 `usize`，装不下 6.4）。与 `crate::project_tabs` 的
/// `TAB_RADIUS_SPEC`、`crate::menu_bar` 的 `ITEM_RADIUS_SPEC` 同一条。
/// `Styled::rounded` 收 `impl Into<AbsoluteLength>`，所以调用点写 `rems(ROW_RADIUS_SPEC / 16.)`。
const ROW_RADIUS_SPEC: f32 = 6.4;

/// 动作行高 32：`min-h-8`（`title-project-menu.tsx:174,181,188`）。
///
/// `PopupMenuItem::Item` 固定 `h(26)`（`popup_menu.rs:1309`），所以三条动作也必须走
/// `PopupMenuItem::element` 自绘，否则行高差 6。
const ACTION_ROW_HEIGHT_SPEC: f32 = 32.;

/// 项目行最小高 44：`min-h-11`（`title-project-menu.tsx:90-97`）。行高由"`py-1.5`×2 + 徽标 28"撑出。
const PROJECT_ROW_HEIGHT_SPEC: f32 = 44.;

/// 行内项目徽标尺寸 28：`ProjectBadge className="size-7"`（`title-project-menu.tsx:99`）。
const ROW_BADGE_SIZE_SPEC: f32 = 28.;

/// 触发器内徽标尺寸 20：真源图标容器 `size-5`（`title-project-menu.tsx:151-156`）。
///
/// 真源那个容器里装的是应用 logo（见模块头），本侧按维护者截图改画徽标，尺寸沿用容器。
const TRIGGER_BADGE_SIZE_SPEC: f32 = 20.;

// ---------------------------------------------------------------------------
// 项目徽标（首字母 + 5 色哈希）
// ---------------------------------------------------------------------------

/// 徽标色板，**顺序即下标**：`hash(name) % 5` 取值。
///
/// 真源 `utils/title-project-menu-model.ts:6-12` 是 `bg-sky-600` / `bg-emerald-600` /
/// `bg-orange-600` / `bg-violet-600` / `bg-rose-600`，这里抄 Tailwind v4 的默认色值
/// （研究 §2.3 的表格逐个核对过）。**不是主题 token**：真源这 5 个色与主题无关，切换主题不变。
const BADGE_TONES: [u32; 5] = [0x0284_c7, 0x0596_69, 0xea58_0c, 0x7c3a_ed, 0xe11d_48];

/// 徽标文字色：真源写死 `text-white`（`title-project-menu.tsx:62`）。
const BADGE_FOREGROUND: u32 = 0x00ff_ffff;

/// 徽标兜底字母：真源 `|| "LI"`（`title-project-menu-model.ts:21`）。
const BADGE_FALLBACK: &str = "LI";

/// 一个项目的首字母徽标：字母 + 色板下标。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectBadge {
    /// 1~2 个大写字母（首字母取**码点**，所以 emoji / 中文都安全）。
    initials: SharedString,
    /// [`BADGE_TONES`] 的下标（`0..5`）。
    tone_index: usize,
}

impl ProjectBadge {
    /// 徽标字母（1~2 个大写字符；切不出词时是 `LI`）。
    pub fn initials(&self) -> &SharedString {
        &self.initials
    }

    /// 色板下标（`0..5`）。
    pub fn tone_index(&self) -> usize {
        self.tone_index
    }

    /// 底色。真源是常量，不随主题变（理由见 [`BADGE_TONES`]）。
    pub fn tone(&self) -> Hsla {
        Hsla::from(rgb(BADGE_TONES[self.tone_index]))
    }

    /// 按名字算徽标。**逐行照抄** `utils/title-project-menu-model.ts:14-31`。
    pub fn for_name(name: &str) -> Self {
        // 按「非字母数字」切词（真源的 `/[^\p{L}\p{N}]+/u`；`char::is_alphanumeric` 就是
        // 同一组 Unicode 类别：`Alphabetic || Numeric`）。
        let words: Vec<&str> = name
            .split(|character: char| !character.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        // 前 2 个词各取第 1 个**码点**（`chars().next()` = 码点，不是 UTF-16 单元），再大写。
        let mut initials: String = words
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect();
        initials = initials.to_uppercase();
        if initials.is_empty() {
            initials = BADGE_FALLBACK.to_string();
        }

        // Java 式 31 进制哈希，种子 0，每步截成 31 位正整数 —— 同一个名字永远同一色。
        // 用 i64 而不是 i32：`value * 31` 在截断前最大约 6.6e10，i32 会溢出。
        let hash = name.chars().fold(0i64, |value: i64, character: char| {
            (value * 31 + character as i64) & 0x7fff_ffff
        });

        Self {
            initials: initials.into(),
            tone_index: (hash % BADGE_TONES.len() as i64) as usize,
        }
    }
}

/// 一枚徽标（彩色圆角方块 + 白字）。
///
/// ⚠️ **必须自绘**：`PopupMenuItem` 的 `icon` 槽只收 `Icon` 字形（`popup_menu.rs:40`），
/// 渲染时还被硬编码 `.xsmall()`（`:1172`），装不下彩色方块（研究 §5.4 第 3 行）。
fn badge_view(badge: &ProjectBadge, size: Pixels) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .size(size)
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .bg(badge.tone())
        .text_color(Hsla::from(rgb(BADGE_FOREGROUND)))
        .text_size(rems(BADGE_FONT_SIZE_SPEC / 16.))
        .font_weight(FontWeight::BOLD)
        .child(badge.initials.clone())
}

// ---------------------------------------------------------------------------
// 面板里的动作（段 ①）
// ---------------------------------------------------------------------------

/// 段 ① 的三条动作。顺序照真源 `title-project-menu.tsx:172,179,186`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelAction {
    /// 新建项目…（真源 `:172-178`，lucide `PlusIcon`）。
    NewProject,
    /// 打开…（真源 `:179-185`，`FolderOpenIcon`）。
    OpenFolder,
    /// 克隆仓库…（真源 `:186-192`，`GitBranchIcon`）。
    CloneRepository,
}

impl PanelAction {
    /// 面板里的顺序（= 渲染顺序 = 键盘顺序）。
    const ALL: [Self; 3] = [Self::NewProject, Self::OpenFolder, Self::CloneRepository];

    /// 图标：真源三个 lucide 字形 `plus` / `folder-open` / `git-branch` 在 gpui-kit 的资源目录里
    /// **都在**（`gpui-kit-assets-0.6.6/assets/icons/`），所以 `IconName` 直接用同名变体。
    fn icon(self) -> IconName {
        match self {
            Self::NewProject => IconName::Plus,
            Self::OpenFolder => IconName::FolderOpen,
            Self::CloneRepository => IconName::GitBranch,
        }
    }

    /// 文案键（真源既有键，**零新增**：`lithe.zh-CN.yml:3538,3544,3568`）。
    fn label_key(self) -> &'static str {
        match self {
            Self::NewProject => "lithe.titleProject.newProject",
            Self::OpenFolder => "lithe.titleProject.open",
            Self::CloneRepository => "lithe.titleProject.cloneRepository",
        }
    }

    /// 诊断行 `S1_PROJECT_MENU action=…` 的取值（可 grep 的契约）。
    fn id(self) -> &'static str {
        match self {
            Self::NewProject => "newProject",
            Self::OpenFolder => "open",
            Self::CloneRepository => "cloneRepository",
        }
    }

    /// 这条动作缺的**前置条件**（只进诊断，不是界面文案 —— 所以不走 i18n）。
    ///
    /// `None` = 这条**真的接线了**（面板里的「打开…」，B4）。诊断行因此能用同一个格式区分
    /// 三档（判据在 [`PanelAction::wire`]）：`state=wired precondition=none` /
    /// `state=not_wired precondition=…`（两条占位项：**可点**，但各自缺一种能力）/
    /// `state=disabled precondition=…`（真禁用态：今天三条动作都到不了这里，只有外壳句柄
    /// 丢了时才在 [`action_row`] 里按"画出来但不可点"处理）。
    fn precondition(self) -> Option<&'static str> {
        match self {
            Self::NewProject => Some("no_scaffolding"),
            // B4 接线之后这条不再缺前置条件：它落到 `ShellWorkspace::open_project_picker`
            // （gpui 自带的 `prompt_for_paths`，零新增依赖）—— 与「文件 → 打开文件夹」同一个函数。
            Self::OpenFolder => None,
            // ⚠️ B3 起这一条**可点**，但前置条件照旧登记：它缺的不是"没有 clone 能力"
            // （Core 的 `git.write` 已含），而是 URL / 凭据 / 进度 UI 这三块界面，
            // 所以诊断行里仍然看得到 `precondition=no_git_clone_call_site`。
            Self::CloneRepository => Some("no_git_clone_call_site"),
        }
    }

    /// 这条动作**点了之后落到哪**（「缺什么」能力组，或 `None` = 真禁用态）。
    ///
    /// ⚠️ 这是 B3 起新拆出来的一问：以前"能不能点"与"缺什么前置条件"是同一件事
    /// （`enabled() == precondition().is_none()`）。「克隆仓库…」变成占位项之后两者分家了 ——
    /// 它**可点**，但**仍然缺前置条件**（真做起来要 URL / 凭据 / 进度 UI）。
    /// 所以判据只留在这里一处，`enabled()` 由它派生，不会再漂移。
    ///
    /// ⚠️ 返回 `None` 的唯一含义是"这条**连缺什么都说不成一句能力话**"——B3 之后
    /// 三条动作都给出了能力组，所以这个 `None` 臂现在**只作为兜底存在**（新增动作时若
    /// 想不出能力句，它会挡着不让人偷偷把动作画成可点）。
    fn wire(self) -> Option<PanelWire> {
        match self {
            // 新建项目缺**项目脚手架生成**（真机是 `createNewDirectory` + 起终端跑 `npm create`）。
            // 这是"没有这条能力"，但它能被说成一句诚实的"缺什么"，所以与克隆仓库统一成占位项 ——
            // 而**不是**复用克隆仓库那句（缺的根本不是同一件事）。
            Self::NewProject => Some(PanelWire::NewProjectNotWired),
            Self::OpenFolder => Some(PanelWire::OpenFolder),
            Self::CloneRepository => Some(PanelWire::CloneNotWired),
        }
    }

    /// 这条动作今天**点不点得动**（真源三者恒可执行；本侧 B3 之后三条都可点，
    /// 只有"外壳句柄丢了"会在 [`action_row`] 里被降级成真禁用态）。
    fn enabled(self) -> bool {
        self.wire().is_some()
    }
}

/// 段 ① 一条动作行**点下去会发生什么**（[`PanelAction::wire`] 的返回值）。
///
/// 抽成枚举而不是两个 `bool`：`action_row` 里那段 `on_click` 要按它分流，
/// 而"漏掉一档"会在 [`PanelWire::missing`] 的 `match` 里变成编译错误（两个 bool 只会静默走进 else）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelWire {
    /// 已接线（B4）：调 `ShellWorkspace::open_project_picker`，与「文件 → 打开文件夹」/
    /// `Ctrl+O` **同一个函数**（Q12：这两条同义）。
    OpenFolder,
    /// 占位（B3 / Q16）：点「克隆仓库…」→ 外壳打诊断 ＋ 状态栏给
    /// 「尚未接入：Core 的 `git.write` 已含 clone，缺的是 URL / 凭据 / 进度 UI。」
    /// （[`crate::menu_bar::MISSING_CLONE_UI`]）。
    CloneNotWired,
    /// 占位（B3 之后）：点「新建项目…」→ 外壳打诊断 ＋ 状态栏给
    /// 「尚未接入：缺项目脚手架生成（真机是新建目录 + 起终端跑 `npm create`）。」
    /// （[`crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING`]）。
    ///
    /// 与 [`PanelWire::CloneNotWired`] 走**同一个**外壳入口
    /// （`ShellWorkspace::report_panel_action_not_wired`），差别只有传进去的能力组。
    NewProjectNotWired,
}

impl PanelWire {
    /// 这一档缺的**能力组**（`None` = 真接线，不需要提示）。
    ///
    /// 抽在这里只为一件事：**"哪一档用哪句能力话"只有这一处真源** —— `on_click` 用它取文案，
    /// 单测也用它钉住"新建项目**没有**借克隆仓库那句"（两句说的根本不是同一件事）。
    /// 新增一档占位项时，这里的 `match` 会强制作者登记它的能力组。
    fn missing(self) -> Option<Missing> {
        match self {
            Self::OpenFolder => None,
            Self::CloneNotWired => Some(crate::menu_bar::MISSING_CLONE_UI),
            Self::NewProjectNotWired => Some(crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING),
        }
    }
}

/// 一条动作行：32 高、8 内距、8 间隔、16×16 图标、13→14px 文字。
///
/// 三条的落点见 [`PanelAction::wire`]：「打开…」**真接线**（B4：交给
/// `ShellWorkspace::open_project_picker`，与「文件 → 打开文件夹」/`Ctrl+O` **同一个执行点**）、
/// 「克隆仓库…」与「新建项目…」**占位**（B3 / B3 之后：交给
/// `ShellWorkspace::report_panel_action_not_wired`，给一句"尚未接入：缺 X"）。
///
/// ⚠️ **必须自绘**（`PopupMenuItem::element`）：`Item` 固定 26 高（`popup_menu.rs:1309`），
/// 撑不到真源的 32。
///
/// `mx_neg_2()` + `px_2()` 是抵消父级的 `.px(8)`（`popup_menu.rs:1230`）：
/// `MenuItemElement` 自己有 8 内距，行底色要铺满行宽就得先退回来。
///
/// # ⚠️ 三条都可点，那"不可用"怎么表达（B3 之后的口径）
///
/// **真源这三条恒可执行**（`title-project-menu.tsx:172-192`，三者都不置灰），本侧两条缺东西：
/// 新建缺脚手架生成、克隆缺 URL / 凭据 / 进度 UI。判据是**"能不能说清缺什么"**：
///
/// - 两条都能说成一句诚实的能力话 ⇒ **可点**，点了把那句话交给外壳（状态栏 ＋ `S1_PROJECT_MENU`
///   诊断），各用**自己**那条能力组（[`crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING`] /
///   [`crate::menu_bar::MISSING_CLONE_UI`]）—— 维护者拍板：同一张面板里两种"不可用"表达方式
///   属于不一致，所以「新建项目…」从真禁用态改成与克隆仓库一致的占位项。
/// - `.disabled(true)` 这一档**没有删除**，它现在只服务两类行：
///   ① 外壳句柄丢了（窗口正在关）的动作行 —— 那时"点了有反应"本来就做不到，画出来但不可点；
///   ② 面板里的**非交互**行（分组标题 / 当前项目行 / 最近项目空态，见本文件后面三处）。
///   `disabled(true)` 让 `PopupMenuItem` 不挂点击、不进键盘导航、前景走 `muted_foreground`、
///   没有 hover 高亮（`menu_item.rs:115-133`），于是"不可用"在**语义与视觉上都能区分**
///   （不是只靠颜色）。
///
/// 前置条件仍然可 grep：面板每次打开时打一行 `S1_PROJECT_MENU action=… state=… precondition=…`
/// （见 [`diagnose_actions`]）—— 两条占位项那一行是 `state=not_wired`（新建是
/// `precondition=no_scaffolding`、克隆是 `precondition=no_git_clone_call_site`），
/// 点下去还会追加一行同 id 的 `state=not_wired missing=<能力组 id>`（见
/// `ShellWorkspace::report_panel_action_not_wired`）。
///
/// `shell` 是**外壳句柄**（面板实体自己没有句柄，见 [`ProjectMenu::shell`]）：可点的三条
/// 都要回到外壳上执行 —— 被绕开的只有"点一下面板"这一段。
fn action_row(action: PanelAction, shell: Option<WeakEntity<ShellWorkspace>>) -> PopupMenuItem {
    // 有接线（真做或占位）**且**外壳句柄还在 ⇒ 这一行画成可点态；否则真禁用
    // （句柄丢了意味着窗口正在关，那时"点了有反应"本来就做不到）。
    let wire = action.wire().zip(shell);
    let clickable = wire.is_some();
    let item = PopupMenuItem::element(move |_window, cx| {
        // 禁用态前景：真源由 `data-disabled:opacity-50` + 继承色决定；gpui-kit 的
        // `MenuItemElement` 直接给 `muted_foreground`（`menu_item.rs:131-133`），
        // 所以图标与文字都显式取同一个 token（子元素的显式色会覆盖父级继承色）。
        // 可点的两条取正常前景色 —— 两者在界面上必须一眼可分。
        let color = if clickable {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        h_flex()
            .w_full()
            .h(rems(ACTION_ROW_HEIGHT_SPEC / 16.))
            .gap_2()
            .mx_neg_2()
            .px_2()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            // 图标 16×16：真源由 `[&_svg:not([class*='size-'])]:size-4` 决定
            // （`ui/dropdown.tsx:782`），`Icon` 侧显式 `.size_4()`（`Icon::xsmall()` 是 12）。
            .child(Icon::new(action.icon()).size_4().text_color(color))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .text_color(color)
                    .child(tr(action.label_key())),
            )
    });

    if !clickable {
        return item.disabled(true);
    }

    let Some((wire, shell)) = wire else {
        return item.disabled(true);
    };
    // 三条可点的动作各自回外壳执行（这里只**路由**：不实现换项目 / 不实现脚手架 / 不实现克隆 UI）。
    // 句柄在上面已经判过非空；`update` 返回 `Err` 只可能是"窗口正在关"，静默忽略。
    item.on_click(move |_event, window, cx| match wire.missing() {
        // 两条占位项走**同一个**外壳入口，差别只有能力组（[`PanelWire::missing`]）——
        // 于是"状态栏那句话 + `S1_PROJECT_MENU` 诊断行"的格式只有一处真源。
        Some(missing) => {
            let _ = shell.update(cx, |shell, cx| {
                shell.report_panel_action_not_wired(action.id(), missing, cx)
            });
        }
        // 真接线的那一条（B4）。
        None => {
            let _ = shell.update(cx, |shell, cx| shell.open_project_picker(window, cx));
        }
    })
}

// ---------------------------------------------------------------------------
// 项目行（段 ②）与空态行（段 ③）
// ---------------------------------------------------------------------------

/// 面板里要显示的一个项目（面板只需要这三样；真源 `ProjectTab` 还有别名 / 自定义图标等，见下）。
#[derive(Clone, Debug)]
pub(crate) struct ProjectEntry {
    /// 显示名（真源 `getProjectDisplayLabel(project)` 的结果，`title-project-menu.tsx:119`）。
    pub name: SharedString,
    /// 项目路径（真源 `project.path`，`:102`）。
    pub path: SharedString,
    /// 是不是**当前**项目（真源 `project.isActive`，`:112,206`）。
    pub active: bool,
}

/// 一行「打开的项目」：44 高、徽标 28、名 + 路径两行、当前项高亮 + 右侧 16px 勾。
///
/// ⚠️ **这四件都必须自绘**（研究 §5.4）：
///
/// 1. 徽标走不了 `icon` 槽（见 [`badge_view`]）；
/// 2. `PopupMenuItem` **没有 title/subtitle 字段**，两行文字只能自己在 element 里排；
/// 3. **没有"某项常态高亮"的 API**：`MenuItemElement::selected` 是 `pub(crate)`，
///    只由鼠标/键盘的 `selected_index` 驱动（`menu_item.rs:41-44`、`popup_menu.rs:1214`），
///    所以当前项那层 `bg-selected` 要自己画；
/// 4. 右侧勾：`PopupMenuItem::checked(true)` + `check_side(Side::Right)` 现成，但勾固定
///    `Icon::xsmall()`(12)，真源是 `size-4`(16)（`:211`）—— 自绘更稳。
///
/// ⚠️ **行高是"内容撑高"，不是固定 44**：真源 `min-h-11`(44) 是**下限**
/// （`title-project-menu.tsx:90-97`），实际高度 = `py-1.5`(12) + 内容。本侧内容 = 徽标 28 与
/// 两行文字（14px 名 + 12px 路径，行高 ~1.4–1.5）取大者 → 实测（`.artifacts/p8/geometry.ps1`，
/// 125% DPI）**52.8 逻辑**：44 这个下限**没有被触发**。真源同样由内容撑到约 50（13px 两行），
/// 差的 ~3 就是本侧"13 → 14 字号"的既定口径（`crate::workspace` 模块头），不是漏了行高。
fn project_row(entry: &ProjectEntry) -> PopupMenuItem {
    let badge = ProjectBadge::for_name(&entry.name);
    let name = entry.name.clone();
    let path = entry.path.clone();
    let active = entry.active;

    PopupMenuItem::element(move |window, cx| {
        let theme = cx.theme();
        // `badge_view` 收的是 `Pixels`（`Div::size` 之外还要传给自绘的容器），所以徽标尺寸
        // 按**当帧的** rem 基准换算一次（`crate::rem_px`）；写死 16 就是假 rem。
        let rem = window.rem_size();
        // 先取色再进闭包：`cx.theme()` 借 `cx`，而 `.when(..)` 的闭包也要用它。
        let accent = theme.accent;
        let foreground = theme.foreground;
        let muted = theme.muted_foreground;
        let primary = theme.primary;

        h_flex()
            .w_full()
            .min_h(rems(PROJECT_ROW_HEIGHT_SPEC / 16.))
            .gap_2p5()
            .mx_neg_2()
            .px_2()
            .py_1p5()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            // 当前项常态高亮：真源 `active && "bg-selected text-foreground"`（`:96`）。
            // gpui-kit 没有 `selected` 这个 token，语义最近的是 `accent`（`crate::project_tabs`
            // 的选中标签底、`crate::activity_bar` 同一取值）。
            .when(active, |this| this.bg(accent))
            .child(badge_view(&badge, crate::rem_px(rem, ROW_BADGE_SIZE_SPEC)))
            .child(
                // `min-w-0` 是 flex 子项能被压缩的前提（真源 `span.min-w-0.flex-1`，`:100`），
                // 右侧勾 `shrink-0`，路径因此是**尾部省略**（真源 `truncate`，`:102`：
                // 不做中间省略、没有 `title` 悬停提示）。
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(foreground)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(path.clone()),
                    ),
            )
            .when(active, |this| {
                this.child(Icon::new(IconName::Check).size_4().text_color(primary))
            })
    })
    // 当前项**可画不可点**：真源点它直接 `return`（`:207-210`，面板不关），而 `PopupMenuItem`
    // 只要可点，`confirm` 里就**无条件** `dismiss`（`popup_menu.rs:875-884`）。
    // `disabled(true)` 是唯一能表达"不进键盘导航、点了不关面板"的开关；它带来的
    // `muted_foreground` 被上面每个子元素自己的颜色覆盖，视觉上与真源一致。
    .disabled(true)
}

/// 一个**分组标题**：「打开的项目」/「最近项目」。
///
/// 真源 `DropdownMenuLabel className="px-2 pt-1.5 pb-1 font-normal ui-text-xs"`
/// （`title-project-menu.tsx:196-198,218-220`）→ `px-2`(8) / `pt-1.5`(6) / `pb-1`(4) /
/// `font-normal`（不设字重）/ 12px 灰字。
///
/// ⚠️ **第 7 处必须自绘的地方**（前六处见 [`action_row`] / [`project_row`] /
/// [`recent_empty_row`] / [`badge_view`]）：真源那个 `ui-text-xs` 类**在仓库里未定义**
/// （`windows/tauri/src/styles/utilities.css:30-44` 只有 `-sm/-caption/-chrome/-base`，
/// 全仓库 `.css` grep 无 `.ui-text-xs`），所以真机实测是**继承父级的 13px**
/// （`ui/dropdown.tsx:753,782`）—— 13px 是"类名写错"的意外结果，不是设计意图
/// （研究 §7 第 2 条）。本侧按**意图**取 **12px**：`text_xs()`（gpui 的档位只有 12/14/16，
/// 13 不在档位上，与 `crate::title_bar` / `crate::project_tabs` 同一条口径）。
///
/// 若走 `PopupMenu::label`，它会复用 `MenuItemElement` 的 `text_base`(**16**)
/// （`menu_item.rs:104-106`），比规格意图大一档且**没有覆盖点**（0.6.6 里也没有
/// `PopupMenuGroup` 类型）—— 这就是这处必须自绘的原因。
fn group_label(key: &'static str) -> PopupMenuItem {
    PopupMenuItem::element(move |_window, cx| {
        div()
            .w_full()
            .mx_neg_2()
            .px_2()
            .pt_1p5()
            .pb_1()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(tr(key))
    })
    // 分组标题是**非交互**标签（真源 `Menu.GroupLabel`，没有 onClick、没有折叠态，
    // `title-project-menu.tsx:195-198`）：disabled 让它不进键盘导航、点了也不关面板。
    .disabled(true)
}

/// 「最近项目」的**空态行**：`px-2 py-3 text-subtle-foreground ui-text-xs`（`:221-225`）。
///
/// 只在**一条最近项目都没有**时画（[`build_popup`] 的段③）。这不是"占位"：数据层
/// （`lithe_db_gpui_settings::recent_projects`）读不出文件时同样得到空列表，界面就该照真源
/// 说「没有最近项目」。
fn recent_empty_row() -> PopupMenuItem {
    PopupMenuItem::element(move |_window, cx| {
        div()
            .w_full()
            .mx_neg_2()
            .px_2()
            .py_3()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(tr("lithe.titleProject.noRecentProjects"))
    })
    // 空态是**非交互**文案（真源是普通 `div`，不是菜单项），所以 disabled：
    // 不进键盘导航，点了也不会收起面板。
    .disabled(true)
}

/// 一行「最近项目」：版式照 [`project_row`]（徽标 + 名 + 路径两行），但**可点** —— 点了就换到那个项目
/// （[`ShellWorkspace::request_open_project`]，与"选一个文件夹"之后的链路完全同一条）。
///
/// 与 [`project_row`] 的三处差别（都是有意的）：
///
/// 1. **不做"当前项"高亮、不画勾**：真源的最近项目行没有这两样（它是历史列表，当前项目
///    在上面的段②里已经标出来了）；
/// 2. **可点**：`on_click` 把路径交给外壳；面板的 `confirm` 会自己收起（`popup_menu.rs:875-884`），
///    不需要额外写关闭逻辑；
/// 3. `missing`（目录已失效）的行**照样可点**：路径探测在换项目链路里做（真源
///    `openRecentFolder` 也是先探测、再决定是标 `missing` 还是打开），而不是在这里
///    静默禁用 —— 用户点了之后会收到一句状态栏提示，比一个点不动的灰行更容易理解。
fn recent_row(entry: &RecentProject, shell: Option<WeakEntity<ShellWorkspace>>) -> PopupMenuItem {
    let badge = ProjectBadge::for_name(&entry.name);
    let name: SharedString = entry.name.clone().into();
    let path_text = entry.path.clone();
    let path = std::path::PathBuf::from(entry.path.clone());
    let item = PopupMenuItem::element(move |window, cx| {
        let theme = cx.theme();
        let foreground = theme.foreground;
        let muted = theme.muted_foreground;
        // 徽标尺寸按**当帧的** rem 基准换算（与 [`project_row`] 同一条，见 [`crate::rem_px`]）。
        let rem = window.rem_size();

        h_flex()
            .w_full()
            .min_h(rems(PROJECT_ROW_HEIGHT_SPEC / 16.))
            .gap_2p5()
            .mx_neg_2()
            .px_2()
            .py_1p5()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .child(badge_view(&badge, crate::rem_px(rem, ROW_BADGE_SIZE_SPEC)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(foreground)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(path_text.clone()),
                    ),
            )
    });

    match shell {
        Some(shell) => item.on_click(move |_event, window, cx| {
            // `clone`：`on_click` 收 `Fn`，路径可能被调用多次。
            let path = path.clone();
            let _ = shell.update(cx, |shell, cx| {
                shell.request_open_project(path.clone(), None, window, cx)
            });
        }),
        // 句柄没登记（窗口正在关）：画出来但不挂点击 —— 与 [`action_row`] 同一处置。
        None => item.disabled(true),
    }
}

// ---------------------------------------------------------------------------
// 面板
// ---------------------------------------------------------------------------

/// 造一个项目下拉面板。每次**打开**现建（收起后缓存被丢掉，见 [`ProjectMenu::popup`]）。
fn build_popup(
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    shell: Option<WeakEntity<ShellWorkspace>>,
    action_context: FocusHandle,
    window: &mut Window,
    cx: &mut App,
) -> Entity<PopupMenu> {
    let entries = entries.to_vec();
    let recent = recent.to_vec();
    PopupMenu::build(window, cx, move |menu, window, _cx| {
        // 面板三个尺寸都走 `PopupMenu` 的**固有** setter（只吃 `Pixels`，见
        // [`PANEL_WIDTH_SPEC`] 的说明），所以按**当帧的** rem 基准换算一次。
        let rem = window.rem_size();
        let mut menu = menu
            // 收起时把焦点还给外壳根元素（`PopupMenu::dismiss` 的 `action_context`，
            // `popup_menu.rs:1052-1072`），与 `crate::menu_bar` 同一做法。
            .action_context(action_context)
            .min_w(crate::rem_px(rem, PANEL_WIDTH_SPEC))
            .max_w(crate::rem_px(rem, PANEL_WIDTH_SPEC))
            // 真源 `max-h-[min(32.5rem,calc(100vh-3rem))]`（见 [`PANEL_MAX_HEIGHT_SPEC`]）。
            // `max_h` 只在 `scrollable(true)` 时生效（`popup_menu.rs:1488-1492`）。
            //
            // ⚠️ **`max_h` 加在"滚动容器本身上"，所以内容再高也撑不破 520**：`PopupMenu::render`
            // 里这两条是同一个 `when(self.scrollable, ..)` 分支里的兄弟调用
            // （v_flex().id("items").max_h(max_height).overflow_y_scroll()，`popup_menu.rs:1482-1492`），
            // 滚动容器的高度 ≤ 520、超出部分走滚动而不参与外层测量。按当前度量算：
            // 3 动作(3×32) + 2 分组标题(2×~25) + 1 当前行(~53) + 分隔线 2(~12) + 条目间距(~34)
            // + 面板内距(8) ≈ 265 逻辑；**真到 12 条最近项目**时 ≈ 265 + 12×53 ≈ 900 逻辑，
            // 仍然被这一层的 `max_h(520)` 截住并滚动（不是把面板撑到 900）。
            // 代价是滚动条**常驻** 16 逻辑 px（v1 里内容只有 ~265，缩略块占满整条轨道）：
            // 换成"需要时才滚动"就得自己算内容高（行数 × 行高 + 间距），而那正是
            // `PopupMenu` 已经用 `max_h` + `overflow_y_scroll` 做对的事，不重复发明。
            .max_h(crate::rem_px(rem, PANEL_MAX_HEIGHT_SPEC))
            .scrollable(true);

        // 段 ①：三条动作（「打开…」可点，另两条禁用，理由见 [`action_row`]）。
        for action in PanelAction::ALL {
            menu = menu.item(action_row(action, shell.clone()));
        }

        // 段 ②：「打开的项目」。`N === 0` 时段 2 **没有**空态占位（真源 `:199` 直接 map 空数组，
        // 只有段 3 有空态）。
        menu = menu
            .separator()
            .item(group_label("lithe.titleProject.openProjects"));
        for entry in &entries {
            menu = menu.item(project_row(entry));
        }

        // 段 ③：「最近项目」——真数据（B4），空列表时才是真源那个空态行。
        menu = menu
            .separator()
            .item(group_label("lithe.titleProject.recentProjects"));
        if recent.is_empty() {
            menu = menu.item(recent_empty_row());
        } else {
            for entry in &recent {
                menu = menu.item(recent_row(entry, shell.clone()));
            }
        }
        menu
    })
}

/// 面板外层的定位：延迟绘制 + 摆在触发器的**正下方**（左对齐、间隔 4）。
///
/// ⚠️ **为什么不用 `anchored()` + `Anchor::BottomLeft`**（`crate::menu_bar` 用的那一种）：
/// 那条路取的是**锚点元素自己在布局里的静态位置**，本侧实测（`.artifacts/p8`）它把面板
/// 顶到了窗口左上角 —— 面板的 `top` 落在触发器**顶上**，把触发器整个盖住（截图里只露出徽标
/// 的 1px）。`Positioner::side(触发器 bounds)` 直接把"触发器下方"这件事说出来，
/// 与真源的 `side="bottom" align="start" sideOffset=4`（`title-project-menu.tsx:168-169`）
/// 一一对应，也正是 gpui-kit 自己给 Select / Combobox / DatePicker 用的那一支
/// （`gpui-component-0.6.6/src/popover.rs:33-39` 的 `dropdown_positioner`）。
///
/// 三个度量各自对着真源：`offset` 4 = `sideOffset` 默认值（`ui/dropdown.tsx:735-737`）、
/// `margin` 8 = `collisionPadding`（同处）、`Align::Start` = `align="start"`。
/// `occlude()` 不可省：不遮挡的话面板下面的标题栏拖拽区会继续吃鼠标事件
/// （`crate::menu_bar` 的同一处结论）。
///
/// `rem` 是**运行时** rem 基准（调用方从 `window.rem_size()` 取）：`Positioner::offset` /
/// `margin` 是**固有方法**、只吃 `Pixels`，所以两个规格值经 [`crate::rem_px`] 换算。
fn popup_for(
    popup: Entity<PopupMenu>,
    trigger_bounds: Bounds<Pixels>,
    rem: Pixels,
) -> impl IntoElement {
    deferred(
        Positioner::side(trigger_bounds)
            .placement(Placement::Bottom)
            .align(Align::Start)
            .offset(crate::rem_px(rem, PANEL_OFFSET_SPEC))
            .margin(crate::rem_px(rem, PANEL_WINDOW_MARGIN_SPEC))
            .occlude()
            .child(popup),
    )
    .with_priority(1)
}

// ---------------------------------------------------------------------------
// 诊断（`S1_PROJECT_MENU_*`）
// ---------------------------------------------------------------------------

/// 开合状态那一行诊断。`opened=false` 在构造期打（启动证据），`opened=true` 在面板
/// **真的被画出来**的那一帧打 —— 后者才是"面板出现"的机器证据，而不是"状态位被置了"。
///
/// `recent` 是**真数据**的条数（B4 起；以前是写死的常量 0）。
fn diagnose(opened: bool, current: &str, recent: usize) {
    eprintln!("S1_PROJECT_MENU opened={opened} recent={recent} current={current}");
}

/// 面板结构那行诊断：把"三段 / 三动作 / 行高"这些只能从像素里量出来的事实写成可 grep 的契约。
///
/// 它是**实现值**（本侧画成多少），不是量出来的像素；截图量出来的那一份在验收报告里对照。
fn diagnose_structure(projects: usize, recent: usize) {
    eprintln!(
        "S1_PROJECT_MENU panel_w={} max_h={} sections=3 actions={} actions_disabled={} \
         open_projects={} recent={} action_h={} row_h={} badge={}",
        PANEL_WIDTH_SPEC as i32,
        PANEL_MAX_HEIGHT_SPEC as i32,
        PanelAction::ALL.len(),
        // 画出来但**不可点**的动作数：B3 之后是 **0** —— 三条都可点（两条占位项"点了给提示"
        // 在 `enabled()` 口径里算可点，理由见 [`PanelAction::wire`]）。这个数就是"外观项"与
        // "点了有反应"之间的差额，写在诊断里免得只靠注释（它非零时说明有人把某条动作
        // 悄悄改回了真禁用态）。
        PanelAction::ALL
            .iter()
            .filter(|action| !action.enabled())
            .count(),
        projects,
        recent,
        ACTION_ROW_HEIGHT_SPEC as i32,
        PROJECT_ROW_HEIGHT_SPEC as i32,
        ROW_BADGE_SIZE_SPEC as i32,
    );
}

/// 三条动作各自的状态，**面板每次打开时**打三行。
///
/// 这样"哪条能点、缺什么"可 grep（B3 之后三档：`state=wired precondition=none`（真做）/
/// `state=not_wired precondition=…`（占位，点了给提示）/ `state=disabled precondition=…`
/// （真禁用态））—— 用户不会看到一个点下去只有日志、还把面板关掉的假入口。
///
/// ⚠️ `state` 的取值域就这三个，判据与 [`PanelAction::wire`] 同源（一个 `match` 分流，
/// 不写第二处 if）；今天三条动作分别落在 `wired` / `not_wired` / `not_wired`。
fn diagnose_actions() {
    for action in PanelAction::ALL {
        let state = match action.wire() {
            Some(PanelWire::OpenFolder) => "wired",
            Some(PanelWire::CloneNotWired | PanelWire::NewProjectNotWired) => "not_wired",
            None => "disabled",
        };
        eprintln!(
            "S1_PROJECT_MENU action={} state={} precondition={}",
            action.id(),
            state,
            action.precondition().unwrap_or("none")
        );
    }
}

/// 逐条打最近项目（**列表顺序 = 界面顺序**，即 `pinned` 优先 + `lastOpenedAt` 降序）。
///
/// 为什么值得一条一条打：验收线要证明"打开两个文件夹 → 下拉里两条、最近的在最前"，
/// 而截图只能证明"画出来了"、证明不了顺序与 `missing` 标记。这个行是可 grep 的契约：
/// `S1_PROJECT_MENU recent_project index=0 path=… missing=false pinned=false`。
fn diagnose_recent(recent: &[RecentProject]) {
    for (index, entry) in recent.iter().enumerate() {
        eprintln!(
            "S1_PROJECT_MENU recent_project index={index} path={} missing={} pinned={} lastOpenedAt={}",
            entry.path, entry.missing, entry.pinned, entry.last_opened_at
        );
    }
}

// ---------------------------------------------------------------------------
// 状态
// ---------------------------------------------------------------------------

/// 标题栏项目下拉的状态：开合 + 面板实体 + 收起订阅。
///
/// ⚠️ **为什么是 `Entity` 而不是一个无状态渲染函数**（其余区域文件都是后者）：面板的开关要
/// **跨越帧**存在，而且验证需要一条"启动态就把面板画出来"的通路（[`ProjectMenu::open_by_probe`]）——
/// 那要求开合状态有一个**可以从外面拿到句柄去改**的落点。理由与 `crate::menu_bar::MenuBar`
/// 完全相同，只是这里还多一条：`Button::dropdown_menu` 没有编程式展开入口（见模块头）。
///
/// 字段可见性一律私有：跨 crate 之后结构体字面量不再是合法构造方式，只经 [`ProjectMenu::new`]。
pub struct ProjectMenu {
    /// 面板是否展开（点击触发器与诊断入口写的是**同一个**字段）。
    open: bool,
    /// 打开时建出来的面板实体。收起来就丢掉（下次打开重建，与 `crate::menu_bar` 同一条口径：
    /// 重建一张 5 行的面板代价极小，缓存它反而要引入"什么时候该重建"的额外状态）。
    popup: Option<Entity<PopupMenu>>,
    /// 触发器的**窗口内边界**（上一帧 prepaint 量到的；`None` = 还没量过）。
    ///
    /// 面板的摆位要用它：`Positioner::side(..)` 收的就是触发器的 bounds，
    /// 而不是"锚点元素在布局里的静态位置"（后者的实测结果见 [`popup_for`] 的文档）。
    /// 用 `Rc<Cell<..>>` 而不是普通字段：量它的 `on_prepaint` 回调要 `'static`，
    /// 而面板的渲染在同一个实体的 `render` 里读它。
    trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// 面板的 `DismissEvent` 订阅（`Esc` / 点面板外 / 选中某项都会发出来）。
    ///
    /// 必须被持有：`Subscription` 一 drop 就取消（gpui 的 RAII 语义）。
    _dismiss_subscription: Option<Subscription>,
    /// 收起时焦点回到它：外壳根元素的焦点句柄。
    ///
    /// 面板内部收起（`Esc` / 选中项）由 `PopupMenu::dismiss` 自己归还；**点触发器收起**这条
    /// 走路不了它（那时是我们主动丢掉面板实体），所以 [`ProjectMenu::toggle`] 显式归还一次。
    action_context: FocusHandle,
    /// 外壳句柄：面板里**可点**的那些行（「打开…」动作行与最近项目行）要落到外壳的能力上
    /// （换项目 / 记最近 / 路径失效标记），而面板实体不是外壳、也拿不到它。
    ///
    /// 为什么是 `Option`：建面板实体时 `ShellWorkspace` 还没构造完（`cx.new` 的闭包里没有
    /// 外壳句柄），所以由 [`ProjectMenu::set_shell`] 在外壳建好之后补登记。
    /// 为什么是 `WeakEntity`：与 [`PROJECT_MENU`] 同一条理由 —— 这里**不能**吊住整个外壳，
    /// 否则 `replace_root` 丢掉旧外壳时引用计数归不了零（换项目就变成泄漏）。
    shell: Option<WeakEntity<ShellWorkspace>>,
}

impl ProjectMenu {
    /// 建状态并在**构造期**打一行 `opened=false` 的诊断（启动证据）。
    ///
    /// `current` 只用于这行诊断；面板里的项目名每帧由 [`render`] 从外壳拿（真值只有一个来源）。
    /// `recent_count` 同理：构造期的最近项目条数（外壳刚读出来的那一份）。
    pub fn new(
        current: &str,
        recent_count: usize,
        action_context: FocusHandle,
        cx: &mut App,
    ) -> Entity<Self> {
        diagnose(false, current, recent_count);
        cx.new(|_| Self {
            open: false,
            popup: None,
            trigger_bounds: Rc::new(Cell::new(None)),
            _dismiss_subscription: None,
            action_context,
            shell: None,
        })
    }

    /// 登记外壳句柄（[`crate::workspace::ShellWorkspace::new`] 在 `Self` 建好之后调一次）。
    ///
    /// 与 [`set_project_menu`] 分开：那个登记的是"面板句柄"（给诊断入口用），
    /// 这个登记的是"面板能回到哪儿去执行"（给面板里的行用）。两条都在 `ShellWorkspace::new`
    /// 里调用，理由见 [`ProjectMenu::shell`] 的字段文档。
    pub(crate) fn set_shell(&mut self, shell: WeakEntity<ShellWorkspace>) {
        self.shell = Some(shell);
    }

    /// 面板是否展开。
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 点触发器：开 → 关，关 → 开。
    ///
    /// 关的那一半要显式把面板实体与订阅丢掉：下一次开时会重建（见 [`ProjectMenu::popup`]）。
    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            self.popup = None;
            self._dismiss_subscription = None;
            // 焦点在面板上，而面板实体马上被丢掉 → 不还焦点的话它会凭空消失。
            window.focus(&self.action_context, cx);
        } else {
            self.open = true;
        }
        cx.notify();
    }

    /// 面板自己收起来了（`Esc` / 点面板外 / 选中某一项）：只清状态，焦点已由 `PopupMenu` 归还。
    fn handle_dismiss(
        &mut self,
        _popup: &Entity<PopupMenu>,
        _event: &DismissEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open = false;
        self.popup = None;
        self._dismiss_subscription = None;
        cx.notify();
    }

    /// **验证/诊断入口**：把面板打开（`--project-menu-probe`）。
    ///
    /// 这不是产品能力：它与"点一下触发器"落到**同一个** `open` 字段、同一段渲染代码，
    /// 唯一被绕开的是"操作系统把这次点击送进窗口"那一段。存在的理由与
    /// `crate::menu_bar::MenuBar::open_by_id` 完全相同：本机工作站**锁屏**，
    /// 鼠标/键盘注入到不了应用，没有这个入口就截不到"面板真的画出来了"那一帧。
    pub fn open_by_probe(&mut self, cx: &mut Context<Self>) {
        eprintln!("S1_PROJECT_MENU_PROBE open=true");
        self.open = true;
        cx.notify();
    }
}

thread_local! {
    /// 当前窗口的项目下拉句柄（`--project-menu-probe` 要在窗口之外拿到它）。
    ///
    /// 与 `crate::menu_bar` 的 `MENU_BAR` 同一套做法与同一条理由：渲染期在
    /// `ShellWorkspace::render` 里，那里 `cx.entity()` 是**外壳**的句柄而不是本模块的。
    /// 用 `WeakEntity` 而不是强引用：窗口关掉之后这里不能吊住整个视图。
    static PROJECT_MENU: RefCell<Option<WeakEntity<ProjectMenu>>> = const { RefCell::new(None) };
}

/// 登记句柄（[`crate::workspace::ShellWorkspace::new`] 调用一次）。
pub(crate) fn set_project_menu(menu: WeakEntity<ProjectMenu>) {
    PROJECT_MENU.with(|slot| *slot.borrow_mut() = Some(menu));
}

/// 取当前窗口的项目下拉句柄（没有就报错，不静默）。
///
/// `pub` 是给 `--project-menu-probe` 用的（诊断入口，见 [`ProjectMenu::open_by_probe`]）。
pub fn handle() -> WeakEntity<ProjectMenu> {
    PROJECT_MENU.with(|slot| slot.borrow().clone()).expect(
        "项目下拉句柄没有登记：`ShellWorkspace::new` 必须在 `render` 之前调 `set_project_menu`",
    )
}

// ---------------------------------------------------------------------------
// 渲染
// ---------------------------------------------------------------------------

/// 触发器：20×20 徽标 + 项目名（`truncate`）+ 右侧 chevron。
///
/// 度量照真源 `title-project-menu.tsx:139-164`：`h-6`(24) / `gap-1.5`(6) / `px-2`(8) /
/// `max-w-56`(224) / chevron 14×14 展开时旋转 180°。
///
/// ⚠️ **自绘 `div` 而不是 gpui-kit 的 `Button`**：`Button` 的 `Small` 档给的正是 24/padding 8
/// （`button.rs:629-632`），但它的内容层是内部 `h_flex`（`gap_1` + `justify_center`，
/// `button.rs:698-711`），**没有公开的覆盖点**；而真源要 `gap-1.5` + 左对齐。
/// `crate::menu_bar` 的顶级项也是自绘（同一类理由），这里沿用。
///
/// ⚠️ **展开时不给底色**：真源是 `ghost`，只有 hover 态（`ui/button.tsx:16`）。
/// 若走 gpui-kit 的 `Popover::trigger`，它会替我们 `selected(is_open)`，
/// 那在 ghost 上是一个 `secondary_active` 蓝底（`button.rs:1245`）—— 与真源不符，所以不接。
///
/// `rem` 是**运行时** rem 基准（调用方从 `window.rem_size()` 取）：`max_w` 走 `Styled`（直接写
/// `rems`），但徽标尺寸要交给 [`badge_view`]、它收 `Pixels`，所以那一个经 [`crate::rem_px`]。
fn trigger(
    entry: &ProjectEntry,
    open: bool,
    handle: WeakEntity<ProjectMenu>,
    rem: Pixels,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let badge = ProjectBadge::for_name(&entry.name);
    let name = entry.name.clone();
    // 未悬停前景 = ghost 的 `text-subtle-foreground`；gpui-kit 没有 `subtle` token，
    // 语义最近的是 `muted_foreground`（与 `crate::title_bar` / `crate::menu_bar` 同一取值）。
    // 先按值取出来再进闭包：`cx.theme()` 借 `cx`，而 `.hover(..)` 的闭包要 `'static`。
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;

    div()
        .id("lithe-title-project-trigger")
        .flex()
        .flex_shrink_0()
        .items_center()
        .h_6()
        .px_2()
        .gap_1p5()
        .max_w(rems(TRIGGER_MAX_WIDTH_SPEC / 16.))
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .text_sm()
        .text_color(idle)
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        // 点触发器**不能**拖窗口：它本身不在 `Drag` 命中区里（是 `drag_region` 的兄弟，
        // 见模块头），这两行是与 `AppMenuBar` 相同的第二道保险（`app_menu_bar.rs:272-280`）。
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
            let _ = handle.update(cx, |menu, cx| menu.toggle(window, cx));
        })
        .child(badge_view(&badge, crate::rem_px(rem, TRIGGER_BADGE_SIZE_SPEC)))
        // 项目名 `min-w-0 truncate`（真源 `:157`）：显示的是项目名本身，
        // `项目：{project}` 只是无障碍名（`:147`）。
        .child(div().min_w_0().truncate().child(name))
        // chevron 14×14：真源 `size-3.5`（`:158-163`）。
        // 展开时**旋转 180°**：真源是 `rotate-180`，gpui 的 `Div` 没有变换
        // （`Popover` 文档注明只有图片/SVG 能带 `TransformationMatrix`），
        // 而 `Icon` 正是 SVG —— 用 `Icon::rotate`（`gpui-component-0.6.6/src/icon.rs:161-167`），
        // 几何上与真源逐像素等价（chevron 上下对称）。
        .child(
            Icon::new(match open {
                true => IconName::ChevronUp,
                false => IconName::ChevronDown,
            })
            .small()
            .text_color(idle)
            .rotate(match open {
                true => radians(std::f32::consts::PI),
                false => radians(0.),
            }),
        )
        .aria_label(tr_args(
            "lithe.titleProject.trigger",
            &[("project", entry.name.as_ref())],
        ))
}

/// 画触发器 + （展开时的）面板，并返回 `AnyElement`。
///
/// 收 `&Entity<ProjectMenu>` 而不是从 [`PROJECT_MENU`] 现取：外壳持有这个 `Entity` 才算
/// 它活着（见 `ShellWorkspace::project_menu` 字段的说明）。
///
/// `recent` 是外壳刚读出来的最近项目（**真数据**，B4 起），顺序就是界面顺序。
///
/// 返回 `None` = 一个项目都没有（那时标题栏里只留拖拽区）—— `ShellWorkspace` 恒有 ≥1 个项目，
/// 所以这只在当前没有工作区时发生。
pub(crate) fn render(
    menu: &Entity<ProjectMenu>,
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if entries.is_empty() {
        return None;
    }
    menu.downgrade()
        .update(cx, |menu, cx| project_menu(menu, entries, recent, window, cx))
        .ok()
}

/// 渲染主体（拿到 `&mut Context<ProjectMenu>` 的那一层，理由同 `crate::menu_bar::render_bar`）。
fn project_menu(
    menu: &mut ProjectMenu,
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    window: &mut Window,
    cx: &mut Context<ProjectMenu>,
) -> AnyElement {
    let open = menu.open;

    // 展开的第一帧才建面板：同时打诊断、订阅收起事件、把焦点交给面板自己。
    // ⚠️ 顺序有讲究：`subscribe_in` 与 `focus` 都必须在**面板真的被建出来**的那一次做，
    // 而面板由本实体的 `popup` 字段缓存，所以这段一帧只跑一次（照 `AppMenu::build_popup_menu`，
    // `app_menu_bar.rs:169-204`）。
    if open && menu.popup.is_none() {
        let popup = build_popup(
            entries,
            recent,
            menu.shell.clone(),
            menu.action_context.clone(),
            window,
            cx,
        );
        menu._dismiss_subscription =
            Some(cx.subscribe_in(&popup, window, ProjectMenu::handle_dismiss));
        // 焦点交给面板：`Esc` / `↑↓` / `Enter` 的 `key_context` 在它身上
        // （`popup_menu.rs:21-30,1467-1475`），焦点不在它身上这些键就不响。
        let focus = popup.read(cx).focus_handle(cx);
        if !focus.contains_focused(window, cx) {
            focus.focus(window, cx);
        }
        menu.popup = Some(popup);
        // 面板真的画出来了 —— 这一行才是"面板出现"的证据（不是"状态位被置了"）。
        diagnose(true, entries[0].name.as_ref(), recent.len());
        diagnose_structure(entries.len(), recent.len());
        diagnose_actions();
        diagnose_recent(recent);
    }

    // 触发器与面板的位置/尺寸都要按**当帧的** rem 基准换算（`project_menu` 收的是 `&mut Window`，
    // 取基元只读、不影响后面的 `on_prepaint` 闭包）。
    let rem = window.rem_size();
    let trigger = trigger(&entries[0], open, handle(), rem, cx).into_any_element();
    // 面板要等触发器量过一次才画：位置是"触发器下方"，没量到就没法摆
    // （只影响第 1 帧，`on_prepaint` 里已经主动补了一帧）。
    let panel = match (menu.popup.clone(), menu.trigger_bounds.get()) {
        (Some(popup), Some(trigger_bounds)) => Some(popup_for(popup, trigger_bounds, rem)),
        _ => None,
    };

    // ⚠️ 量触发器盒子的是**这一层**（无内边距的包装），不是触发器本身：
    // 实测（`.artifacts/p8`）`on_prepaint` 打在带 `px_2` 的触发器上时给的是它的
    // **内容盒**，面板因此会整体右移 8 逻辑 px（触发器左 487 物理 → 面板左 497 物理）。
    // 包装层没有 padding / margin / border，`bounds` 与触发器的视觉盒子逐像素相等，
    // 面板左边因此与触发器左边严格对齐（真源 `align="start"`）。
    let bounds_cell = menu.trigger_bounds.clone();
    div()
        .id("lithe-title-project-menu")
        .relative()
        .flex_shrink_0()
        // 第一次量到时主动要一帧：面板的位置读的是**上一帧**的 bounds，不补一帧的话
        // 它会先画在错误的位置上（`Popup` 用同一招，`gpui-base-0.6.6/src/popup.rs:110-125`）。
        .on_prepaint(move |bounds, window, _cx| {
            let first = bounds_cell.get().is_none();
            bounds_cell.set(Some(bounds));
            if first {
                window.request_animation_frame();
            }
        })
        .child(trigger)
        .children(panel)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        BADGE_FALLBACK, BADGE_TONES, PANEL_MAX_HEIGHT_SPEC, PANEL_WIDTH_SPEC, PanelAction,
        PanelWire, ProjectBadge,
    };

    /// 徽标字母对三个**在真源侧核对过**的样本必须逐个命中（研究 §2.2-5）：
    /// `jzwg` / `qzjk` 是研究里用 node 复算过的，`Lithe-IDEA` 是同一算法在真源笔记里给出的第三个样本。
    #[test]
    fn badge_initials_match_the_windows_algorithm() {
        assert_eq!(ProjectBadge::for_name("jzwg").initials().as_ref(), "J");
        assert_eq!(ProjectBadge::for_name("qzjk").initials().as_ref(), "Q");
        assert_eq!(
            ProjectBadge::for_name("Lithe-IDEA").initials().as_ref(),
            "LI"
        );
        // 真源单测（`title-project-menu-model.test.ts:75-81`）的三条：多词取前两个词的首码点、
        // 单个中文词取一个字、切不出词时兜底 `LI`。
        assert_eq!(
            ProjectBadge::for_name("Lithe-IDEA-issue-35-ci")
                .initials()
                .as_ref(),
            "LI"
        );
        assert_eq!(ProjectBadge::for_name("文档项目").initials().as_ref(), "文");
        assert_eq!(
            ProjectBadge::for_name("").initials().as_ref(),
            BADGE_FALLBACK
        );
        assert_eq!(
            ProjectBadge::for_name("---").initials().as_ref(),
            BADGE_FALLBACK
        );
    }

    /// 配色 = `hash % 5`。三个样本的下标与真源截图 / node 复算一致，且**同一个名字永远同一色**。
    #[test]
    fn badge_tone_index_is_the_hashed_palette_slot() {
        // jzwg → sky(0)、qzjk → violet(3)、Lithe-IDEA → emerald(1)。
        assert_eq!(ProjectBadge::for_name("jzwg").tone_index(), 0);
        assert_eq!(ProjectBadge::for_name("qzjk").tone_index(), 3);
        assert_eq!(ProjectBadge::for_name("Lithe-IDEA").tone_index(), 1);
        // 确定性：同一个名字算两次必须完全一样（真源 `:75-81` 断言的就是这一点）。
        assert_eq!(
            ProjectBadge::for_name("Lithe-IDEA-issue-35-ci"),
            ProjectBadge::for_name("Lithe-IDEA-issue-35-ci")
        );
    }

    /// 色板必须与真源的 5 个 Tailwind 常量逐个相等（研究 §2.3）——
    /// 这几个值是**写死的**（不走主题），所以需要一条测试钉住。
    #[test]
    fn badge_palette_is_the_tailwind_palette() {
        assert_eq!(
            BADGE_TONES,
            [0x0284_c7, 0x0596_69, 0xea58_0c, 0x7c3a_ed, 0xe11d_48],
            "sky-600 / emerald-600 / orange-600 / violet-600 / rose-600"
        );
    }

    /// 5 个色板下标都要能被取到（哈希不能偏科到只命中一两个颜色）。
    #[test]
    fn every_tone_slot_is_reachable() {
        let mut seen = [false; BADGE_TONES.len()];
        for index in 0..2_000 {
            seen[ProjectBadge::for_name(&format!("project-{index}")).tone_index()] = true;
        }
        assert!(
            seen.iter().all(|hit| *hit),
            "有颜色永远取不到：{seen:?}"
        );
    }

    /// 段 ① 的结构：**正好 3 条**动作、顺序照真源，且文案键 / 诊断 id 都不重复。
    ///
    /// 这条守的是"面板三段与真源可比对"这件事 —— 少一条或多一条，肉眼很难发现。
    #[test]
    fn action_rows_match_the_source_order() {
        assert_eq!(PanelAction::ALL.len(), 3);
        assert_eq!(
            PanelAction::ALL.iter().map(|a| a.id()).collect::<Vec<_>>(),
            vec!["newProject", "open", "cloneRepository"]
        );
        let keys: Vec<&str> = PanelAction::ALL.iter().map(|a| a.label_key()).collect();
        assert_eq!(
            keys,
            vec![
                "lithe.titleProject.newProject",
                "lithe.titleProject.open",
                "lithe.titleProject.cloneRepository"
            ]
        );
        // 三段动作各自的状态必须说清楚（B3 之后：一条真接线、两条占位项，后两条各缺一种能力），
        // 免得诊断行说不清"哪条能用、缺的是哪条能力"。
        let mut preconditions: Vec<&str> = PanelAction::ALL
            .iter()
            .map(|action| action.precondition().unwrap_or("none"))
            .collect();
        preconditions.sort_unstable();
        preconditions.dedup();
        assert_eq!(
            preconditions,
            vec!["no_git_clone_call_site", "no_scaffolding", "none"],
            "三条动作的前置条件必须各说各的（真接线的那条是 none）"
        );
    }

    /// B3 之后的动作表状态：**三条都可点** —— 「打开…」真接线，另两条是占位项
    /// （点了给"尚未接入：缺 X"），且两条的 `precondition()` 各给一个具体理由
    /// （诊断行靠它说清"缺什么"）。
    ///
    /// 这条守的是一类回归：
    ///
    /// 1. 把某条动作的 `precondition` 从 `Some` 改成 `None`（或反过来）会让诊断说谎；
    /// 2. 把两条占位项中的任何一条悄悄改回真禁用态（`wire()` → `None`）会让面板出现
    ///    "点了没反应"的入口；反过来把「打开…」也降级成占位项则是假装配不上真能力 ——
    ///    两者都被 B3 与维护者口径禁止；
    /// 3. **文案**：两条占位项必须各用**自己**那条能力组，尤其不许让「新建项目…」借
    ///    「克隆仓库…」那句（"Core 的 `git.write` 已含 clone"与脚手架生成无关）。
    #[test]
    fn every_action_row_is_clickable_and_says_what_is_missing() {
        let clickable: Vec<PanelAction> = PanelAction::ALL
            .into_iter()
            .filter(|action| action.enabled())
            .collect();
        assert_eq!(
            clickable,
            vec![
                PanelAction::NewProject,
                PanelAction::OpenFolder,
                PanelAction::CloneRepository
            ],
            "三条都可点：「打开…」真接线，新建 / 克隆是占位项（点了给提示）"
        );
        assert_eq!(
            PanelAction::OpenFolder.precondition(),
            None,
            "真接线的那条不该再报前置条件"
        );
        assert_eq!(
            PanelAction::OpenFolder.wire(),
            Some(PanelWire::OpenFolder)
        );
        assert_eq!(
            PanelAction::CloneRepository.wire(),
            Some(PanelWire::CloneNotWired)
        );
        assert_eq!(
            PanelAction::NewProject.wire(),
            Some(PanelWire::NewProjectNotWired),
            "「新建项目…」缺脚手架生成 —— 那句话说得出来，所以与克隆仓库一致做成占位项"
        );
        for action in [PanelAction::NewProject, PanelAction::CloneRepository] {
            assert!(
                action.precondition().is_some(),
                "{action:?} 没真的做出来就必须说清缺什么"
            );
        }
        assert_eq!(
            PanelAction::NewProject.precondition(),
            Some("no_scaffolding"),
            "占位项保留它**自己**那条前置条件，不复用克隆仓库的"
        );
        assert_eq!(
            PanelAction::CloneRepository.precondition(),
            Some("no_git_clone_call_site"),
            "占位项也要留前置条件：那句话说的是'缺 URL / 凭据 / 进度 UI'，不是'什么都有'"
        );
        // 「哪一种占位项用哪句能力话」只有一处真源（[`PanelWire::missing`]）：
        // 新建项目必须拿到**它自己**那条能力组，且与克隆仓库那句不同。
        assert_eq!(
            PanelWire::NewProjectNotWired.missing(),
            Some(crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING)
        );
        assert_eq!(
            PanelWire::CloneNotWired.missing(),
            Some(crate::menu_bar::MISSING_CLONE_UI)
        );
        assert_ne!(
            PanelWire::NewProjectNotWired.missing(),
            PanelWire::CloneNotWired.missing(),
            "两条占位项缺的不是同一件事，不许共用同一句能力话"
        );
        assert_eq!(
            PanelWire::OpenFolder.missing(),
            None,
            "真接线的那条不该带能力提示"
        );
    }

    /// 最近项目行的**可点性**只取决于"外壳句柄登没登记"这一件事 —— 没有句柄时必须能安全地
    /// 画出来（走 disabled 分支），而不是 panic 或假装能点。
    ///
    /// 这里只做结构判据（本 crate 拿不到 `TestAppContext`，理由见 `menu_bar.rs` 里那两处说明）：
    /// `recent_row` 收 `None` 时能构造出 item 就算过；真正的点击行为（换根 + 记最近）
    /// 由 `.artifacts/p22` 的实机 `S1_*` 日志覆盖。
    #[test]
    fn recent_rows_render_without_a_shell_handle() {
        use lithe_db_gpui_settings::RecentProject;

        let entry = RecentProject {
            name: "alpha".to_string(),
            path: r"D:\proj\alpha".to_string(),
            last_opened_at: 1_700_000_000_000,
            pinned: false,
            // `missing = true` 的行**照样可点**（路径探测在换项目链路里做，见 `recent_row` 文档）。
            missing: true,
            open_in_new_window: None,
        };
        let _item = super::recent_row(&entry, None);
    }

    /// 面板度量：384 宽 / 520 最大高（真源 `w-96` / `max-h-[min(32.5rem,…)]`）。
    ///
    /// 这两个值是渲染期经 [`crate::rem_px`]（按当帧 `window.rem_size()` 基准）换算的，所以这里
    /// 钉的是规格常量本身；换算函数自身的"随基准缩放"判据在 `crate::tests` 的
    /// `rem_px_scales_with_the_runtime_base` 里。
    #[test]
    fn panel_width_and_max_height_match_the_source() {
        assert_eq!(PANEL_WIDTH_SPEC, 384.);
        assert_eq!(PANEL_MAX_HEIGHT_SPEC, 520.);
    }
}
