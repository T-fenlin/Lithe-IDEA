//! Lithe 的 UI 集成测试层：把**真实视图**渲染进无头窗口，断言它真的画出来了。
//!
//! 这是任务表 `D26` 的落点，也是本仓库**第一处**能在窗口里跑交互的测试
//! （此前 `#[cfg(test)]` 全是纯函数单测，注释多处写明"本 crate 拿不到 `TestAppContext`"）。
//! 形状照上游 `gpui-kit-0.7.0/tests/ui.rs` + `tests/common/mod.rs` 抄；
//! 判据与可用的断言接口见 `docs/gpui kit/0.7.0/docs/test.md`。
//!
//! ⚠️ 关于"要不要给生产视图加 `.test_support()`"：`TestSupportExt::test_support()` 对
//! 原生 div 只注册观察、不增加布局盒子，**启用与否都能调用**（上游 `test.md` 的说明）。
//! 本测试**不需要**它 —— 断言走的是 `EditorPane` 自己的公开访问器 + 窗口真的渲染成功，
//! 所以不必为了让测试好写而改动生产渲染树（那正是 `test.md` 明确劝阻的做法：
//! "不要仅为让视觉断言通过而添加 aria_label"。）

mod common;

use std::path::PathBuf;

// ⚠️ 这两个 trait 都必须显式引入，缺一个就编译不过（实测）：
// `AppContext` 提供 `cx.new(..)` 与 `cx.update_window(..)`（**不是**
// `TestAppContextExt` —— 那个在 0.7.0 里只提供 `wait_for`，本测试是同步的用不到）；
// `TestWindowExt` 提供 `window.render_frame(..)`。
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AppContext as _, TestAppContext, px, size};

/// 建一个临时文件并在测试结束时删掉它（断言失败也要删，所以用 `Drop` 而不是结尾手动 remove）。
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    /// 在系统临时目录下写一个内容确定的文件（文件名带 pid，避免并发测试互相覆盖）。
    fn new(name: &str, contents: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lithe-ui-test-{}-{name}", std::process::id()));
        std::fs::write(&path, contents).expect("写临时文件");
        Self { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 打开一个真实文件 → 视图真的渲染出来 → 缩进宽度真的传到了视图里。
///
/// 这条同时钉住三件事，而这三件事此前都**只有真机手测**：
/// 1. `EditorPane` 能在无头窗口里渲染（渲染树不 panic、不依赖真实窗口）；
/// 2. 打开文件后活动 buffer 是"可写"的（`active_buffer_is_readonly()` 为 `false`），
///    以及 `tab_size()` 的默认档就是 2；
/// 3. `set_tab_size(n)` 立刻被视图接受（D13 的状态栏缩进格读的就是同一个值）。
#[gpui_kit::test]
fn editor_pane_renders_a_real_file_and_reports_its_indent(cx: &mut TestAppContext) {
    let file = TempFile::new("indent.md", "# 标题\n\n正文\n");

    // 组件库得先初始化（主题、字体、组件注册），否则渲染期取主题会 panic。
    cx.update(gpui_kit::init);

    let (handle, pane) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| lithe_gpui_editor::EditorPane::new(None, window, cx))
    });

    // ① 打开文件（需要 `&mut Window`，所以写在 update_content 里）。
    let path = file.path.clone();
    common::update_content(handle, &pane, cx, |pane, window, cx| {
        pane.open(&path, window, cx);
    })
    .unwrap();

    // ② 真的画出一帧（首帧之前查 `window.find` 会 panic）。
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    // ③ 视图的公开真值：刚打开的文件是可写的，缩进是默认档 2。
    cx.update(|cx| {
        let pane = pane.read(cx);
        assert_eq!(
            pane.tab_size(),
            2,
            "新视图的缩进宽度应当是默认档 2（设置页 tabSize 的默认值）"
        );
        assert!(
            !pane.active_buffer_is_readonly(),
            "刚打开的普通 markdown 文件必须是可写的（只读格的判据取 Buffer::writable）"
        );
        assert_eq!(
            pane.active_buffer_name(),
            "lithe-ui-test-".to_string() + &std::process::id().to_string() + "-indent.md",
            "活动标签应当是刚打开的那个文件"
        );
    });

    // ④ 改缩进宽度：视图立刻接受（真机路径是设置变化 → 外壳 → set_tab_size）。
    common::update_content(handle, &pane, cx, |pane, _window, cx| {
        pane.set_tab_size(8, cx);
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            pane.read(cx).tab_size(),
            8,
            "set_tab_size 之后访问器必须返回新值（状态栏缩进格读的就是它）"
        );
    });
}

/// 宿主把通知 toast 的**同时可见条数上限**设成 3。
///
/// 判据是**真实赋值**而不是常量本身：走生产入口 `theme::apply_theme_by_name`
/// （它同时是启动、切主题、改设置三条路的路由），再读 `Theme` 的
/// `notification.max_items` —— 上游消费点 `grouped()` 每帧读的正是这个字段
/// （`gpui-component-0.7.0/src/notification.rs:783-787`），所以"字段是 3"就是
/// UI-MAP 里"最多 3 条"已接的**全部**机器可验证内容。
///
/// 分两段断言，各自守一条独立的失效：
/// 1. 设上之后是 3（常量被改错、或赋值被删掉都会失败）；
/// 2. 再切一次主题后**仍然是 3**（`Theme::change` + `apply_config` 若哪天开始回写
///    `notification`，这条会先炸 —— 那时"最多 3 条"会在每次切主题后静默退回 10）。
///
/// ⚠️ 范围说明：`lithe-gpui-settings` 这个 crate 没有 `gpui-kit` 的 `test-support`
/// dev-dependency（settings 的 `#[cfg(test)]` 全是纯函数单测，理由见 `menu_bar.rs` 里
/// "本 crate 拿不到 `TestAppContext`"那条），所以这条**路由真的跑起来**的测试放在
/// 唯一拥有 `TestAppContext` 的 UI 测试 crate（本文件）里。测试**不开窗口、不 sleep**。
///
/// ⚠️ 这条只证明"上限值被设上了"，**真实观感未经真机视觉验证**。
#[gpui_kit::test]
fn theme_application_caps_simultaneous_notification_toasts_at_three(cx: &mut TestAppContext) {
    use gpui_kit::component::Theme;
    use lithe_gpui_settings::theme::NOTIFICATION_MAX_ITEMS;

    // 组件库得先初始化：主题注册表在 `gpui_kit::init` 里建立。
    cx.update(gpui_kit::init);

    // 注册表里总会有主题（`gpui_component` 的 `ThemeRegistry::init` 会装内置那两份）。
    // 用注册表的键而不是写死名字：本仓库有两套主题（内置 IDEA 一组 + 捆绑扩展），
    // 写死哪个都会在主题改名时变成一条无关的失败。
    let name = cx.update(|cx| {
        gpui_kit::component::ThemeRegistry::global(cx)
            .themes()
            .keys()
            .next()
            .expect("组件库初始化后主题注册表必须非空")
            .clone()
    });

    // ① 走生产入口应用一次主题。
    let applied = cx.update(|cx| lithe_gpui_settings::theme::apply_theme_by_name(cx, &name));
    assert!(applied, "主题 {name} 应当在注册表里并能被应用");
    assert_eq!(
        cx.update(|cx| Theme::global(cx).notification.max_items),
        NOTIFICATION_MAX_ITEMS,
        "应用主题后 toast 的同时可见条数上限必须是真源 macOS 的 maximumVisibleCount（3），实测 {}",
        NOTIFICATION_MAX_ITEMS
    );

    // ② 再应用一次（= 用户切主题）：上限不能被主题配置冲掉。
    cx.update(|cx| {
        lithe_gpui_settings::theme::apply_theme_by_name(cx, &name);
    });
    assert_eq!(
        cx.update(|cx| Theme::global(cx).notification.max_items),
        NOTIFICATION_MAX_ITEMS,
        "切主题会重跑 apply_config，上限必须仍然生效（`notification` 不是主题文件字段）"
    );
}

/// 宿主把通知 toast 的**内衬**设成真源右下 overlay 的 padding：右 **52** / 底 **38**（规格像素）。
///
/// 真源 `WorkbenchView.swift:517` 的 `.overlay(alignment: .bottomTrailing)` + `:553` 的
/// `.padding(.trailing, rightActivityBarWidth + 12)`（40 + 12）+ `:554` 的 `.padding(.bottom, 38)`。
/// 其中那个 40 现在是与宿主两条 rail **共用的 owner**
/// `lithe_gpui_shared::design::ACTIVITY_RAIL_WIDTH_SPEC`（`BLOCKERS.md` B14 落地后；
/// `activity_bar.rs` 的单测从 rail 侧钉同一个 owner）。
/// 上游消费点每帧读 `Theme.notification.margins`（`gpui-component-0.7.0/src/notification.rs`
/// `:1016-1017`，`:1062` 对 `Anchor::BottomRight` 只取 `bottom` / `right`），所以"字段是这两个值"
/// 就是"内衬已对齐"的**全部**机器可验证内容。
///
/// 三段断言，各自守一条独立的失效：
/// 1. 规格数字本身（常量被改成别的数会失败）；
/// 2. 走生产入口 `theme::apply_theme_by_name` 之后，两条边等于**按当刻 rem 基准**换算的规格值
///    —— 赋值点被删掉时会红，因为上游默认是 `px(16.)`（`notification.rs:556`），
///    与 52 / 38 不同（同时钉住"走的是 rem 换算、不是写死 `px(...)`"）；
/// 3. 再应用一次主题后仍然成立（`apply_config` 不会把它冲回上游默认）。
///
/// 期望值用 `rem_px(Theme::global(cx).font_size, 规格)` 现算而不是写死 `px(52.)`：宿主是在
/// 应用主题那一刻用 `cx.theme().font_size` 换算的，而该换算点排在 `apply_config` **之后**
/// （基准此刻已定），所以两边用的是同一个基准，断言是确定的、不是巧合。
///
/// ⚠️ 确定性：不开窗口、不 sleep、无后台任务（同本文件另一条通知测试）。这条只证明"内衬值被设上
/// 了且切主题后仍在"，**真实观感未经真机视觉验证**；`margins` 是 `Pixels` 字段，改「界面字体大小」
/// 后不会重新换算（默认档 uiFontSize = 13 ⇒ 基准 16px，两条路等价），这条也不覆盖。
#[gpui_kit::test]
fn theme_application_sets_notification_toast_insets_from_the_macos_source(cx: &mut TestAppContext) {
    use gpui_kit::component::Theme;
    use lithe_gpui_settings::theme::{
        NOTIFICATION_MARGIN_BOTTOM_SPEC, NOTIFICATION_MARGIN_RIGHT_SPEC,
    };
    use lithe_gpui_shared::design;
    use lithe_gpui_shared::rem_px;

    cx.update(gpui_kit::init);

    // ① 规格数字：右 = 真源 `rightActivityBarWidth(40) + 12`，底 = 真源写死的 38。
    assert_eq!(
        NOTIFICATION_MARGIN_RIGHT_SPEC, 52.,
        "右内衬必须等于真源 `WorkbenchView.swift:553` 的 `rightActivityBarWidth(40) + 12`"
    );
    // ①' 而且这个 52 必须**派生自活动栏 rail 宽的 owner**（`BLOCKERS.md` B14 落地后，
    // 宿主 rail 宽与通知内衬共用 `shared::design::ACTIVITY_RAIL_WIDTH_SPEC`）——
    // owner 一改，这条与 `activity_bar` 的单测会一起红。
    assert_eq!(
        NOTIFICATION_MARGIN_RIGHT_SPEC,
        design::ACTIVITY_RAIL_WIDTH_SPEC + 12.,
        "右内衬必须派生自 rail 宽 owner，而不是在本侧另写一个孤立的 52"
    );
    assert_eq!(
        NOTIFICATION_MARGIN_BOTTOM_SPEC, 38.,
        "底内衬必须等于真源 `WorkbenchView.swift:554` 的 `.padding(.bottom, 38)`"
    );

    // 与另一条通知测试同一取法：用注册表的键而不是写死主题名。
    let name = cx.update(|cx| {
        gpui_kit::component::ThemeRegistry::global(cx)
            .themes()
            .keys()
            .next()
            .expect("组件库初始化后主题注册表必须非空")
            .clone()
    });

    // 读一次当前 `(rem 基准, 内衬)`。基准由宿主的换算点决定，所以两边同源。
    let insets = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            let theme = Theme::global(cx);
            (theme.font_size, theme.notification.margins.clone())
        })
    };

    // ② 走生产入口应用一次主题。
    let applied = cx.update(|cx| lithe_gpui_settings::theme::apply_theme_by_name(cx, &name));
    assert!(applied, "主题 {name} 应当在注册表里并能被应用");
    let (rem, margins) = insets(cx);
    assert_eq!(
        margins.right,
        rem_px(rem, NOTIFICATION_MARGIN_RIGHT_SPEC),
        "应用主题后 toast 距右边缘的内衬应当是 {NOTIFICATION_MARGIN_RIGHT_SPEC} 规格像素（当刻基准 {rem:?}）；\
         上游默认是 px(16.)，实测 {:?} —— 排在 rem 换算之前的赋值点被删掉、或基准取错都会在这里红",
        margins.right
    );
    assert_eq!(
        margins.bottom,
        rem_px(rem, NOTIFICATION_MARGIN_BOTTOM_SPEC),
        "应用主题后 toast 距底边缘的内衬应当是 {NOTIFICATION_MARGIN_BOTTOM_SPEC} 规格像素（当刻基准 {rem:?}）；实测 {:?}",
        margins.bottom
    );

    // ③ 再应用一次（= 用户切主题）：apply_config 不能把内衬冲回上游默认。
    cx.update(|cx| {
        lithe_gpui_settings::theme::apply_theme_by_name(cx, &name);
    });
    let (rem, margins) = insets(cx);
    assert_eq!(
        margins.right,
        rem_px(rem, NOTIFICATION_MARGIN_RIGHT_SPEC),
        "切主题会重跑 apply_config，右内衬必须仍然生效（`notification` 不是主题文件字段）"
    );
    assert_eq!(
        margins.bottom,
        rem_px(rem, NOTIFICATION_MARGIN_BOTTOM_SPEC),
        "切主题会重跑 apply_config，底内衬必须仍然生效（`notification` 不是主题文件字段）"
    );
}
