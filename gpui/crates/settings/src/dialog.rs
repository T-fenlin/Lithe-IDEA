//! 设置对话框：**模态对话框**（不是 Sheet、不是页面）。
//!
//! ## 布局真源
//!
//! 逐值照 `windows/tauri/src/features/settings/components/settings-dialog.tsx:88-157`
//! 与 `07-settings-ui.md` §1.3 的度量表：
//!
//! ```text
//! v_flex 820×620（居中、遮罩、Esc 可关）
//! ├─ 头部 44          h-11 border-b bg-surface px-3：[齿轮] 设置 ………… [关闭]
//! ├─ h_flex flex-1
//! │  ├─ 分类栏 190    w-47.5 border-r bg-surface p-2：项高 32、图标 16、选中 bg-primary/65
//! │  └─ 内容区 flex-1 bg-background p-6 overflow-y：页标题 text-xl font-semibold mb-5 + 分组
//! └─ 底部             justify-between px-4 py-3：左「恢复默认设置…」(ghost) 右「完成」(primary)
//! ```
//!
//! ## 组件选型：**不用** `component::setting::Settings`，自绘行
//!
//! `07-settings-ui.md` §9.3 推荐先用 `setting::{Settings, SettingPage, SettingGroup, SettingItem,
//! SettingField}`，但读完 0.6.6 的实现后判定**换不出这个布局**，三条硬证据：
//!
//! 1. **它自带搜索框，且无法关闭**：`Settings::render_sidebar` 的 header 永远塞一个
//!    `Input::new(&search_input)`（`gpui-component-0.6.6/src/setting/settings.rs:148-153`），
//!    没有任何开关；而 Windows 真源**没有搜索框**（§1.4：132 条搜索索引在真实对话框里没有渲染出口）。
//!    带着一个真源里没有的输入框，等于凭空多一个入口。
//! 2. **它的左栏是可拖拽分栏**：`h_resizable` + `resizable_panel().size(250)`（同文件 `:416-422`，
//!    宽度范围 160–360）；Windows 是**固定 190px、无拖拽把手**。
//! 3. **它的页面外壳不一样**：页头是 `p_4 + border_b_1`（`setting/page.rs:180-186`），
//!    Windows 是 `p-6` + 无下边框 + `text-xl font-semibold` 标题；分组/行的内边距也各自不同
//!    （`page.rs:236` 的 `py_4` vs Windows 的 `gap-3 p-3`）。
//!
//! 所以这里**复用它的行内控件**（`Switch` / `NumberInput` / `Button`，以及它自己的下拉实现方式
//! —— `Button + dropdown_menu_with_anchor + PopupMenuItem`，见
//! `setting/fields/dropdown.rs:60-87`）与 `Dialog` 容器，**自绘**左栏与行（[`crate::row`]）。
//! 代价：没有"白送"的搜索过滤（真源也没有）与页级重置按钮（真源只在底部有一个全局的）。
//!
//! ## 打开时机（硬约束）
//!
//! 浮层只能在事件回调或任务里打开；`render` 阶段调 `window.open_dialog` 会 panic
//! （`shell/overlays.md:203-213`、`gpui/crates/app/src/main.rs:14-16`）。本模块的两个入口
//! ——活动栏「设置」的点击回调、`Ctrl+,` 的**全局 action**（[`init`]）——都在事件回调里；
//! `--open-settings` 走 `window.on_next_frame`（首帧之后、非 render 阶段）。

use gpui_kit::assets::IconName;
use gpui_kit::base::{Selectable as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{InputEvent, InputState, NumberInput};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, WindowExt as _};
use gpui_kit::{
    AbsoluteLength, Anchor, App, AppContext as _, Context, ElementId, Entity, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px, rems,
};

use lithe_gpui_shared::tr;

use crate::row::{ControlWidth, RowActivation, page_stack, page_title, settings_group, settings_row};
use crate::schema::{
    DISPLAY_LANGUAGES, Settings, UI_FONT_SIZE_DEFAULT, UI_FONT_SIZE_MAX, UI_FONT_SIZE_MIN,
    UI_FONT_SIZE_STEP,
};
use crate::store::{AppearanceMode, SettingsStore, store};
use crate::theme;

/// 打开设置对话框（`Ctrl+,`）。**只能从事件回调或任务里调用。**
///
/// 用 `actions!` + `App::on_action` 注册成**全局 action**，而不是挂在某个元素的
/// `on_action` 上：gpui 的按键派发在没有焦点元素时只走"窗口根节点"这一条路径
/// （`gpui-pre-0.3.6/src/window.rs:5815` 的 `focus_node_id_in_rendered_frame(None)` →
/// `root_node_id()`），而工作台视图是 `Root` 的**子**节点、不在那条路径上，所以挂在
/// `ShellWorkspace` 的根元素上会有"启动后没点过任何地方时 `Ctrl+,` 不响应"的死角。
/// `App::on_action` 注册的处理器在 action 冒泡阶段的最后一定会被调用
/// （`gpui-pre-0.3.6/src/app.rs:2360-2377`），与焦点无关。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-,", OpenSettings, None)]);
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_settings_dialog(window, cx));
    });
}

gpui_kit::actions!(lithe_settings, [OpenSettings]);

/// 打开设置对话框。已经开着时（含确认子对话框）不叠第二层。
pub fn open_settings_dialog(window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        return;
    }
    let store = store(cx);
    let view = cx.new(|cx| SettingsDialog::new(store, window, cx));
    println!("S1_SETTINGS dialog_opened");

    window.open_dialog(cx, move |dialog, window, _cx| {
        // 820×620 用 rem 表达（`rems(P / 16.)`，1rem = 16px），再按窗口当前的 rem 基准求值：
        // `Dialog::width` / `margin_top` 只收 `Pixels`
        // （`gpui-component-0.6.6/src/dialog/dialog.rs:395,405`），而 gpui 没有
        // `impl From<Rems> for Pixels`，所以走 `AbsoluteLength::to_pixels(rem_size)`
        // （`gpui-pre-0.3.6/src/geometry.rs:3361`）。
        let rem = window.rem_size();
        let viewport = window.viewport_size();
        // 左右/下各留 16（Dialog 自己的边距也是 `spacing_tokens().lg` = 16，
        // `dialog/dialog.rs:528`）。
        let margin = rem;
        let width = rem_px(rem, DIALOG_WIDTH);
        let height = rem_px(rem, DIALOG_HEIGHT).min((viewport.height - margin * 2.).max(px(0.)));
        // 居中：Dialog 的 y 默认是视口的 1/10（`dialog/dialog.rs:529`），Windows 是垂直居中。
        let margin_top = ((viewport.height - height) / 2.).max(px(0.));

        dialog
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(width)
            .margin_top(margin_top)
            .h(height)
            // 弹层自身的内边距与间距归零：头部 / 内容 / 底部都由本视图自己排。
            .p_0()
            .content({
                let view = view.clone();
                move |content, _, _| content.p_0().gap_0().child(view.clone())
            })
    });
}

/// 把规格值（px）按**当前 rem 基准**求值：`rems(P / 16.)`。
///
/// 写成 `/ 4.` 是错的 —— gpui 的档位 helper 后缀 `N` = `N × 0.25rem`，而这里的 `P` 是**像素**，
/// 1rem = 16px（主题的 `font.size`，`gpui/themes/lithe-dark.json:9`）。
fn rem_px(rem: gpui_kit::Pixels, spec_px: f32) -> gpui_kit::Pixels {
    AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)
}

/// 同一个换算的 `Length` 形式（`Styled::w` / `Styled::h` 收 `Length`，由布局期按窗口 rem 求值）。
fn rem_length(spec_px: f32) -> gpui_kit::Length {
    rems(spec_px / 16.).into()
}

/// 对话框尺寸（`settings-dialog.tsx:118-119` 的 `h-[620px] w-[820px]`）。
const DIALOG_WIDTH: f32 = 820.;
const DIALOG_HEIGHT: f32 = 620.;
/// 左栏宽度（`settings-dialog.tsx:126` 的 `w-47.5` = 190px）。
const NAV_WIDTH: f32 = 190.;
/// 头部高度（`settings-dialog.tsx:120` 的 `h-11` = 44px）——正好在 gpui 档位上，用 `h_11()`。

/// 左侧分类。**v1 只有两页**：其余 10 个分类对应的子系统在 gpui 侧不存在，
/// 做了也只是空壳；理由与前置条件写在 `gpui/PLAN.md` 的「阶段 8」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    /// 常规（`settings.tabs.general` = 常规）。
    General,
    /// 外观（`settings.tabs.appearance` = 外观）。
    Appearance,
}

impl Category {
    /// 渲染顺序（Windows 的 12 个分类里前两个就是这两个）。
    const ALL: [Category; 2] = [Category::General, Category::Appearance];

    /// 分类名文案键（Windows `settings-dialog.tsx:35-48` 的 `labelKey`）。
    fn label_key(self) -> &'static str {
        match self {
            Self::General => "lithe.settings.tabs.general",
            Self::Appearance => "lithe.settings.tabs.appearance",
        }
    }

    /// 分类图标。Windows 用 `GearSixIcon`（常规）/ 外观页在真源里没有分类图标，
    /// 这里取 Lucide 的 `palette`（`gpui-kit-assets-0.6.6/assets/palette.svg`，真实字形）。
    fn icon(self) -> IconName {
        match self {
            Self::General => IconName::Settings,
            Self::Appearance => IconName::Palette,
        }
    }
}

/// 设置对话框的内容视图。
pub struct SettingsDialog {
    store: Entity<SettingsStore>,
    category: Category,
    /// 「界面字体大小」的数字输入状态（`NumberInput` 是 Stateful 组件，状态由调用方持有）。
    font_size_input: Entity<InputState>,
    /// 订阅与观察（`store` 变了要重绘；输入框变了要写设置）。
    _subscriptions: Vec<Subscription>,
}

impl SettingsDialog {
    fn new(store: Entity<SettingsStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let value = store.read(cx).settings().ui_font_size;
        // 数字框的引擎配置放在 `InputState` 上：`+`/`-` 与上下键都按 `step` 走、
        // 越界文本在输入期间被容忍、失焦时收敛到范围（`gpui-base-0.6.6/src/input/base/state.rs:9032-9052`）。
        let font_size_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_ui_font_size(value))
                .step(UI_FONT_SIZE_STEP)
                .min(UI_FONT_SIZE_MIN)
                .max(UI_FONT_SIZE_MAX)
        });

        let mut subscriptions = Vec::new();
        // 设置变了 → 重绘（主题/字号/开关的显示都跟着走）。
        subscriptions.push(cx.observe(&store, |_, _, cx| cx.notify()));
        // 输入框变了 → 写设置。照 `setting/fields/number.rs:96-111` 的口径：
        // 解析不了的中间态（`""`、`"1."`）与越界值都**不写**设置，等下一次按键补全，
        // 免得"输入 18 的过程中先被夹成 10"。
        subscriptions.push(cx.subscribe_in(
            &font_size_input,
            window,
            |this: &mut Self, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(parsed) = input.read(cx).value().parse::<f64>() else {
                    return;
                };
                if !(UI_FONT_SIZE_MIN..=UI_FONT_SIZE_MAX).contains(&parsed) {
                    return;
                }
                let store = this.store.clone();
                store.update(cx, |store, cx| store.set_ui_font_size(parsed, cx));
            },
        ));

        Self {
            store,
            category: Category::General,
            font_size_input,
            _subscriptions: subscriptions,
        }
    }

    /// 头部 44px：齿轮 + 「设置」+ 关闭。`settings-dialog.tsx:120` + `ui/dialog.tsx:261-283`。
    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .h_11()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .text_color(cx.theme().foreground)
            .child(Icon::new(IconName::Settings).size_4())
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr("lithe.workbench.settings")),
            )
            .child(div().flex_1())
            .child(
                // 只有图标的按钮必须有 tooltip 与可读名称（设计指南「字体与图标」）。
                Button::new("settings-close")
                    .small()
                    .ghost()
                    .icon(IconName::Close)
                    .accessibility_label(tr("lithe.tabs.close"))
                    .tooltip(tr("lithe.tabs.close"))
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
    }

    /// 左栏：固定宽、无拖拽把手，选中的分类有**颜色 + 字重**两种可辨别信号。
    fn nav(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .id("settings-nav")
            .h_full()
            .w(rem_length(NAV_WIDTH))
            .flex_shrink_0()
            .gap_0p5()
            .p_2()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .aria_label(tr("lithe.settings.mac.categories"))
            .children(Category::ALL.iter().enumerate().map(|(index, category)| {
                let selected = *category == self.category;
                Button::new(("settings-category", index))
                    .ghost()
                    .selected(selected)
                    .w_full()
                    // 内容整体左对齐：`Button` 的内容层默认 `justify_center`
                    // （`button/button.rs:700-705`），所以要塞一个 `w_full` 的自定义内容，
                    // 而不是用 `.icon()/.label()`（那两者会被居中）。
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .items_center()
                            .gap_2p5()
                            .child(Icon::new(category.icon()).size_4())
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .when(selected, |this| this.font_weight(FontWeight::MEDIUM))
                                    .child(tr(category.label_key())),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.category = *category;
                        cx.notify();
                    }))
            }))
    }

    /// 右栏：页标题 + 分组，只有这一列滚动（滚动条贴住它自己的边缘）。
    fn content(&self, cx: &Context<Self>) -> impl IntoElement {
        let settings = self.store.read(cx).settings().clone();
        div()
            .id("settings-content-scroll")
            .flex_1()
            .min_w_0()
            .h_full()
            .p_6()
            .bg(cx.theme().background)
            .overflow_y_scrollbar()
            .child(page_stack().child(page_title(tr(self.category.label_key()))).children(
                match self.category {
                    Category::General => self.general_page(&settings, cx),
                    Category::Appearance => self.appearance_page(&settings, cx),
                },
            ))
    }

    /// 「常规」页：v1 只做「语言」一项。
    fn general_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.language"),
                vec![settings_row(
                    "settings-row-language",
                    tr("lithe.settings.mac.language"),
                    // ⚠️ **不照抄** Windows 的 `settings.mac.languageDescription`
                    // （「界面语言会立即生效。默认语言为英文。」）—— gpui 侧 `set_locale` 只在启动早期
                    // 调用一次，而界面里有构造期就 `tr()` 过的文案（活动栏项、状态栏），运行中换语言会
                    // 变成一半新一半旧。所以描述如实写"重启后生效"。这条是 gpui 侧新增的键
                    // （Windows 缺失，本侧补）。
                    Some(tr("lithe.settings.gpui.languageRestartDescription")),
                    self.dropdown(
                        "settings-language",
                        language_label(&settings.display_language),
                        DISPLAY_LANGUAGES
                            .iter()
                            .map(|tag| {
                                let value = SharedString::from(*tag);
                                (
                                    value.clone(),
                                    language_label(tag),
                                    settings.display_language == *tag,
                                )
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                // 只写设置（`set_display_language` 不等防抖、立即落盘），语言在
                                // **下次启动**生效。
                                //
                                // `restart.rs` 里有现成的"用相同参数重启自己"（`restart_application`），
                                // 本轮**故意不接**：为一次语言切换杀掉进程，会连带丢掉编辑器未保存内容
                                // 与终端会话，而 `PLAN.md` §8 的 v1 口径就是"只提示、由用户自己重启"。
                                // 要改成"选完立即重启"，在这里补一次
                                // `crate::restart::restart_application(cx)` 即可。
                                store.update(cx, |store, cx| {
                                    store.set_display_language(value.to_string(), cx);
                                });
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 「外观」页：v1 做**能立刻生效**的 4 项。
    fn appearance_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let mode = AppearanceMode::from_settings(settings);
        // 显示**当前生效**的主题（跟随系统时就是系统对应的那一支），而不是 Windows 的
        // `settings.theme`：同步模式下 Windows 会显示一个并非生效中的值
        // （`macos-settings-panels.tsx:141` 读 `settings.theme`，而 `:122` 写的是 `autoTheme*`）。
        // 这是有意的偏离，避免界面显示的值与实际生效的不一致。
        let applied_theme = self.store.read(cx).applied_theme();
        let themes = theme_choices(cx, applied_theme.clone());

        vec![
            settings_group(
                tr("lithe.settings.appearance.theme"),
                vec![
                    settings_row(
                        "settings-row-theme",
                        tr("lithe.settings.appearance.colorTheme"),
                        None,
                        self.dropdown(
                            "settings-theme",
                            applied_theme,
                            themes,
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    store.update(cx, |store, cx| store.set_theme(value, cx));
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                    settings_row(
                        "settings-row-appearance-mode",
                        tr("lithe.settings.mac.appearanceMode"),
                        Some(tr("lithe.settings.mac.appearanceDescription")),
                        self.dropdown(
                            "settings-appearance-mode",
                            mode_label(mode),
                            vec![
                                (
                                    SharedString::from(MODE_SYSTEM),
                                    tr("lithe.settings.mac.followSystem"),
                                    mode == AppearanceMode::System,
                                ),
                                (
                                    SharedString::from(MODE_LIGHT),
                                    tr("lithe.settings.mac.light"),
                                    mode == AppearanceMode::Light,
                                ),
                                (
                                    SharedString::from(MODE_DARK),
                                    tr("lithe.settings.mac.dark"),
                                    mode == AppearanceMode::Dark,
                                ),
                            ],
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    let mode = match value.as_ref() {
                                        MODE_LIGHT => AppearanceMode::Light,
                                        MODE_DARK => AppearanceMode::Dark,
                                        _ => AppearanceMode::System,
                                    };
                                    store.update(cx, |store, cx| {
                                        store.set_appearance_mode(mode, cx)
                                    });
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                ],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.appearance.typography"),
                vec![settings_row(
                    "settings-row-ui-font-size",
                    tr("lithe.settings.appearance.uiFontSize"),
                    // Windows 的 `settings.appearance.uiFontSizeDescription`
                    // （「以 0.5 像素为单位调整界面文本和图标缩放」）正好说明了步长与作用范围。
                    Some(tr("lithe.settings.appearance.uiFontSizeDescription")),
                    NumberInput::new(&self.font_size_input)
                        .w(ControlWidth::Number.length())
                        .into_any_element(),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.appearance.interface"),
                vec![settings_row(
                    "settings-row-status-bar",
                    tr("lithe.settings.appearance.showStatusBar"),
                    None,
                    {
                        let store = self.store.clone();
                        Switch::new("settings-show-status-bar")
                            .small()
                            .checked(settings.show_status_bar)
                            .on_change(move |checked, _, cx| {
                                let checked = *checked;
                                store.update(cx, |store, cx| {
                                    store.set_show_status_bar(checked, cx)
                                });
                            })
                            .into_any_element()
                    },
                    // 开关行支持"点整行 = 切换"（Windows `SettingsRow` 的整行激活）。
                    // 开关自己会吃掉点击并切换，所以行回调只在点标签/空白处触发，不会触发两次。
                    Some({
                        let activation: RowActivation = {
                            let store = self.store.clone();
                            let next = !settings.show_status_bar;
                            Box::new(move |_, _, cx| {
                                store.update(cx, |store, cx| {
                                    store.set_show_status_bar(next, cx)
                                });
                            })
                        };
                        activation
                    }),
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 底部：左「恢复默认设置…」+ 右「完成」。
    fn footer(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                // 会打开确认对话框的命令：文案末尾带省略号（设计指南「大小写、标点与符号」）。
                Button::new("settings-restore-defaults")
                    .ghost()
                    .label(tr("lithe.settings.gpui.restoreDefaultsOpen"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.confirm_restore_defaults(window, cx)
                    })),
            )
            .child(
                Button::new("settings-done")
                    .primary()
                    .label(tr("lithe.settings.mac.done"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        // 关闭前把防抖窗口里的最后一次改动落盘，避免"改了没生效就退出"。
                        let store = this.store.clone();
                        store.update(cx, |store, _| store.flush_pending());
                        window.close_dialog(cx);
                    })),
            )
    }

    /// 「恢复默认设置」的确认对话框。
    ///
    /// ⚠️ **有意不照抄 Windows 的文案**：真源是
    /// `settings.mac.restoreDefaultsConfirm` = 「⚠️确认恢复所有配置吗？」
    /// （`windows/tauri/src/i18n/locale.ts`，渲染点 `settings-dialog.tsx:99-107`），
    /// 而设计指南 `docs/design-guides.md:434` 明确点名"您确定要……吗"这类套话是反例。
    /// 所以按指南写成"标题=决策、正文=后果、按钮=结果词"：
    /// 标题「恢复默认设置？」、正文「所有设置都会回到默认值。」、按钮「取消」+「恢复默认设置」。
    /// 这三条里只有按钮文案沿用 Windows 原句，另两条是 gpui 侧新增键（Windows 缺失，本侧补）。
    fn confirm_restore_defaults(&self, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let input = self.font_size_input.clone();

        window.open_dialog(cx, move |dialog, _, cx| {
            let store = store.clone();
            let input = input.clone();
            dialog
                .title(tr("lithe.settings.gpui.restoreDefaultsTitle"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.settings.gpui.restoreDefaultsBody")),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("settings-restore-cancel")
                                .label(tr("lithe.ui.cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            // 破坏性结果用破坏性样式标记（设计指南「破坏性操作要区分可逆与不可逆」），
                            // 但样式不替代准确用词 —— 文案仍是结果词。
                            Button::new("settings-restore-confirm")
                                .danger()
                                .label(tr("lithe.settings.mac.restoreDefaults"))
                                .on_click(move |_, window, cx| {
                                    store.update(cx, |store, cx| store.restore_defaults(cx));
                                    // 数字框的文字不会自己跟着回落（设置层不回写控件），
                                    // 恢复默认后显式同步一次，免得界面显示一个已经不是当前的数值。
                                    input.update(cx, |input, cx| {
                                        input.set_value(
                                            SharedString::from(format_ui_font_size(
                                                UI_FONT_SIZE_DEFAULT,
                                            )),
                                            window,
                                            cx,
                                        );
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    /// 一个下拉：`Button` + `dropdown_menu_with_anchor`。
    ///
    /// 这就是 gpui-kit `setting::SettingField::dropdown` 自己的实现方式
    /// （`gpui-component-0.6.6/src/setting/fields/dropdown.rs:60-87`）：
    /// `Select` + `SelectState` 更适合"可搜索的长列表"，而这里最多十来个主题。
    fn dropdown(
        &self,
        id: impl Into<ElementId>,
        current_label: SharedString,
        options: Vec<(SharedString, SharedString, bool)>,
        on_select: Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>,
    ) -> gpui_kit::AnyElement {
        let on_select = std::rc::Rc::new(on_select);
        Button::new(id)
            .outline()
            .w(ControlWidth::Default.length())
            .justify_between()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(current_label),
            )
            .dropdown_caret(true)
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                options.iter().fold(menu, |menu, (value, label, checked)| {
                    let on_select = on_select.clone();
                    let value = value.clone();
                    menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(*checked)
                            .on_click(move |_, window, cx| {
                                on_select(value.clone(), window, cx);
                            }),
                    )
                })
                // 主题可能有十几个，允许滚动（Windows 的下拉也是可滚动列表）。
                .scrollable(true)
            })
            .into_any_element()
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ⚠️ 三个区域函数都收 `&Context<Self>`（不可变借用）：edition 2024 的 RPIT 会捕获
        // 签名里的生命周期，收 `&mut` 就会让"同一个表达式里连调两次"报 E0499
        // （同 `gpui/crates/workbench/src/lib.rs:63-74` 的结论）。
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.header(cx))
            .child(
                h_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(self.nav(cx))
                    .child(self.content(cx)),
            )
            .child(self.footer(cx))
    }
}

/// 外观模式下拉的取值常量（内部值，不是界面文案）。
const MODE_SYSTEM: &str = "system";
const MODE_LIGHT: &str = "light";
const MODE_DARK: &str = "dark";

/// 主题下拉的候选：注册表里全部主题名；**当前生效的名字不在列表里时补在第一位**
/// （照 Windows 的 `normalizedThemeOptions`，`macos-settings-panels.tsx:102-113`），
/// 否则用户在设置文件里写了一个已卸载的主题时，下拉会显示一个不在候选里的空值。
///
/// 返回 `(value, label, checked)` 三元组：value 是主题名（设置文件里存的就是它），
/// label 是显示名 —— gpui 的主题没有单独的展示名，两者都是 `themes[].name`。
fn theme_choices(cx: &App, applied: SharedString) -> Vec<(SharedString, SharedString, bool)> {
    let mut names = theme::theme_names(cx);
    if !names.iter().any(|name| name == applied.as_ref()) {
        names.insert(0, applied.to_string());
    }
    names
        .into_iter()
        .map(|name| {
            let label = SharedString::from(name.clone());
            let checked = name == applied.as_ref();
            (SharedString::from(name), label, checked)
        })
        .collect()
}

/// 语言下拉的显示名。
///
/// Windows 的两个选项是**硬编码**的 `English` / `简体中文`
/// （`macos-settings-panels.tsx:187-188`），**不在 i18n catalog 里**
/// （`gpui/crates/shared/locales/README.md` §7 第 5 条登记过这件事），死代码页签
/// `tabs/general-settings.tsx` 用的中文是「英语」/「简体中文」（`07-settings-ui.md` §3.1）。
/// 界面上不许出现字面量，所以这两条是 gpui 侧新增的键（Windows 缺失，本侧补），
/// 中文取死代码页签的「英语」/「简体中文」，英文按语言自称写。
fn language_label(tag: &str) -> SharedString {
    match tag {
        "en-US" => tr("lithe.settings.gpui.languageEnglish"),
        _ => tr("lithe.settings.gpui.languageChinese"),
    }
}

/// 外观模式的下拉显示名。
fn mode_label(mode: AppearanceMode) -> SharedString {
    match mode {
        AppearanceMode::System => tr("lithe.settings.mac.followSystem"),
        AppearanceMode::Light => tr("lithe.settings.mac.light"),
        AppearanceMode::Dark => tr("lithe.settings.mac.dark"),
    }
}

/// UI 字号显示文本（`13` / `16.5`，不补 `.00`）。
fn format_ui_font_size(value: f64) -> String {
    let normalized = crate::schema::normalize_ui_font_size(value);
    if normalized.fract() == 0.0 {
        format!("{}", normalized as i64)
    } else {
        format!("{normalized}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字号显示文本：整数不补 `.00`，半步长保留一位小数。
    #[test]
    fn font_size_labels_are_compact() {
        assert_eq!(format_ui_font_size(13.0), "13");
        assert_eq!(format_ui_font_size(16.5), "16.5");
        assert_eq!(format_ui_font_size(13.24), "13");
    }

    /// 分类清单必须正好是 v1 的两页（多一页就是空壳）。
    #[test]
    fn only_two_categories_are_exposed() {
        assert_eq!(
            Category::ALL,
            [Category::General, Category::Appearance],
            "v1 只做常规与外观两页"
        );
    }

    /// rem 换算：`rems(P / 16.)` 在 16px 基准下必须等于规格像素值。
    #[test]
    fn rem_conversion_matches_the_spec_pixels() {
        assert_eq!(rem_px(px(16.), DIALOG_WIDTH), px(820.));
        assert_eq!(rem_px(px(16.), DIALOG_HEIGHT), px(620.));
        assert_eq!(rem_px(px(16.), NAV_WIDTH), px(190.));
        // 基准字号翻倍（uiFontSize 26）时长度等比例放大：这就是用 rem 的意义。
        assert_eq!(rem_px(px(32.), NAV_WIDTH), px(380.));
    }
}
