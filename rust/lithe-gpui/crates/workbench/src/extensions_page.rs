//! 中央扩展页：扩展目录的浏览入口（Zed 式独立页）。
//!
//! ## 为什么是中央页而不是右侧工具窗
//!
//! 真机（已删除的 Windows/Tauri 产品）里右活动栏的拼图按钮调 `openExtensionsBuffer`
//! —— 扩展是一块**编辑器缓冲区**，占中央区；gpui 宿主早期把它做成右栏空态视图是
//! 记录在案的临时偏离（`right_tool_window.rs` 模块文档）。2026-09-29 起对齐真源：
//! 拼图项打开**中央扩展页**（`ShellWorkspace` 的中央列二选一切换），右工具窗只剩
//! 通知与 Maven。与编辑器缓冲区的差异：本页不是 buffer（编辑区的 buffer 模型
//! `pub(crate)` 且与文件绑定），而是外壳的中央页槽位 —— 与左栏双实体同一套
//! 「两实体二选一」的既有模式。
//!
//! ## 数据从哪来
//!
//! `lithe_gpui_extensions::load_catalog`（Core `plugins.listCatalog`，内置目录
//! fixture + 完整校验）。实体构造后 `cx.spawn` 拉一次并缓存（与 `maven.scan` 的
//! 懒扫同一理由：读数据排到派发栈之外）；v1 是**只读**目录 —— 安装 / 启停 /
//! 下载还没有数据源，**不画假按钮**（`07-settings-ui.md` §7.3-D 同一口径）。
//!
//! ## 文案
//!
//! 大部分键复用旧 Windows 前端留在 locale 里的 `extensions.*` 词汇（搜索占位、
//! 分类、作者、贡献项、内置徽标等）；「字体 / 调试 / LSP」三个分类与「加载失败」
//! 是 gpui 侧新增（理由见 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS`）。

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div,
};

use lithe_gpui_shared::{tr, tr_args};

/// 分类筛选条（顺序 = 界面顺序，按维护者点名顺序：主题 / 图标主题 / 字体 / 调试 /
/// LSP / 语言）。`None` = 「全部」。后一项是 `tr` 键；分类 id 与
/// `lithe_gpui_extensions::CATEGORIES` 一致（Core 从贡献项推导出的分类名）。
const FILTERS: [(Option<&'static str>, &'static str); 7] = [
    (None, "lithe.extensions.all"),
    (Some("themes"), "lithe.extensions.themes"),
    (Some("icons"), "lithe.extensions.icons"),
    (Some("fonts"), "lithe.extensions.category.fonts"),
    (Some("debug"), "lithe.extensions.category.debug"),
    (Some("lsp"), "lithe.extensions.category.lsp"),
    (Some("languages"), "lithe.extensions.languages"),
];

/// 分类 id → 界面标签。id（ 等）是目录契约的协议面，界面按  键显示，
/// 与筛选条共用同一批键 —— 否则同一件事会出现「图标」与「icons」两种写法。
fn category_label(category: &str) -> SharedString {
    match category {
        "themes" => tr("lithe.extensions.themes"),
        "icons" => tr("lithe.extensions.icons"),
        "fonts" => tr("lithe.extensions.category.fonts"),
        "debug" => tr("lithe.extensions.category.debug"),
        "lsp" => tr("lithe.extensions.category.lsp"),
        "languages" => tr("lithe.extensions.languages"),
        // 目录里出现未知分类时原样显示（比空串诚实：至少说明数据里有这一档）。
        other => SharedString::from(other.to_string()),
    }
}

/// 目录加载状态。
#[derive(Debug)]
enum CatalogStatus {
    /// 构造后后台任务还没回来。画「正在加载扩展」空态。
    Loading,
    /// 目录就绪。
    Loaded(Vec<Card>),
    /// 拉取或解析失败；附展示用的错误原文（目录是打包数据，失败只会是宿主版本
    /// 不匹配或打包损坏，如实呈现）。
    Failed(SharedString),
}

/// 一张卡片的**自有**数据。
///
/// 与 `notifications.rs` 的 `Row` 同一理由：`Entity::read` 的守卫借用状态与 `cx`，
/// 而建元素树要 `&mut Context`；先把要用的拷出来，借用在建树之前结束。
/// 目录是个位数条目，克隆可忽略。
#[derive(Debug, Clone)]
struct Card {
    id: String,
    name: SharedString,
    version: SharedString,
    description: Option<SharedString>,
    /// Core 推导出的分类（结构化，判据用）。
    categories: Vec<String>,
    /// 分类标签的展示串（`categories` 按 `" · "` 连接）。
    categories_label: SharedString,
    publisher: SharedString,
    built_in: bool,
    /// **可卸载**（入口点是 `bundled`）：卡片上出现卸载/重新安装按钮。
    ///
    /// v1 里只有捆绑主题扩展属于这一类；`builtIn`（宿主核心能力，如 IDEA 主题、
    /// 字体、Java 支持）不给卸载按钮 —— 它们没有卸载语义，画出来就是假控件。
    removable: bool,
    /// 这个包贡献了字体（详情里要写清「下次启动生效」—— 字体运行期摘不掉）。
    has_fonts: bool,
    /// 这个包贡献的是图标集时，它的**包名**（设置键 `icons` 的取值）；
    /// 卡片据此画「使用 / 当前」——图标主题与配色主题一样是"选一个生效"的东西。
    icons_package: Option<String>,
    /// 展开详情里的贡献项行（字体家族已去重）。
    contributions: Vec<SharedString>,
}

impl Card {
    fn from_snapshot(entry: &lithe_gpui_extensions::ExtensionSnapshot) -> Self {
        // 贡献项行**全部走 tr**：标签复用分类标签键（与筛选条同一词汇），格式走
        // extensions.contributionLine（标点归各语言自己管）。此前这里硬编码中文，
        // 英文界面下会漏中文 —— 那是本页第一版留下的缺口。
        let line = |category: &str, items: &str| -> SharedString {
            tr_args(
                "lithe.extensions.contributionLine",
                &[
                    ("label", category_label(category).as_ref()),
                    ("items", items),
                ],
            )
        };
        let mut contributions = Vec::new();
        if !entry.theme_names.is_empty() {
            contributions.push(line("themes", &entry.theme_names.join("、")));
        }
        if !entry.icons_names.is_empty() {
            contributions.push(line("icons", &entry.icons_names.join("、")));
        }
        if !entry.font_families.is_empty() {
            contributions.push(line("fonts", &entry.font_families.join("、")));
        }
        if !entry.debug_adapter_names.is_empty() {
            contributions.push(line("debug", &entry.debug_adapter_names.join("、")));
        }
        for support in &entry.language_supports {
            let extensions = support
                .file_extensions
                .iter()
                .map(|extension| format!(".{extension}"))
                .collect::<Vec<_>>()
                .join("、");
            contributions.push(line(
                "languages",
                &format!("{}（{extensions}）", support.display_name),
            ));
        }
        Self {
            id: entry.id.clone(),
            name: entry.display_name.clone().into(),
            version: entry.version.clone().into(),
            description: entry.description.clone().map(Into::into),
            categories: entry.categories.clone(),
            categories_label: entry
                .categories
                .iter()
                .map(|category| category_label(category).to_string())
                .collect::<Vec<_>>()
                .join(" · ")
                .into(),
            publisher: entry.publisher.clone().into(),
            built_in: entry.entrypoint_kind == "builtIn",
            removable: entry.entrypoint_kind == "bundled",
            has_fonts: !entry.font_families.is_empty(),
            icons_package: if entry.categories.iter().any(|item| item == "icons") {
                // 目录里的贡献路径形如 `icons/pierre/extension.json`，包名就是中间那段。
                entry
                    .icons_paths
                    .first()
                    .and_then(|path| path.split('/').nth(1))
                    .map(str::to_string)
            } else {
                None
            },
            contributions,
        }
    }
}

/// 搜索判据（与 `lithe_gpui_extensions::matches_query` 同一覆盖面：id、名称、
/// 发布者、描述与贡献项名称 —— 贡献项名称都在 `contributions` 行里）。
fn card_matches_query(card: &Card, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let contains = |text: &str| text.to_lowercase().contains(&query);
    contains(&card.id)
        || contains(card.name.as_ref())
        || contains(card.publisher.as_ref())
        || card
            .description
            .as_ref()
            .is_some_and(|text| contains(text.as_ref()))
        || card
            .contributions
            .iter()
            .any(|line| contains(line.as_ref()))
}

/// 分类判据：`None` = 全部；分类是结构化字段，**不能**用子串匹配
/// （`"icons"` 包含 `"themes"`，子串会把两档混在一起）。
fn card_matches_category(card: &Card, category: Option<&str>) -> bool {
    match category {
        None => true,
        Some(category) => card.categories.iter().any(|item| item == category),
    }
}

/// 中央扩展页。
///
/// 由 `ShellWorkspace` 在**首次打开时**创建并常驻持有（重开不丢搜索与筛选状态，
/// 与左栏双实体同一口径）。
pub struct ExtensionsPage {
    search: Entity<InputState>,
    /// 当前查询串。`Input` 没有 `on_change`，订阅 `InputEvent::Change` 存一份
    /// （与 `notifications.rs` 同一做法）。
    query: SharedString,
    /// 当前分类筛选；`None` = 全部。
    category: Option<&'static str>,
    /// 展开了详情的扩展（稳定包 id）。只展开一条。
    expanded: Option<String>,
    status: CatalogStatus,
    /// 设置存储：读 `uninstalledExtensions`（决定卡片画「卸载」还是「重新安装」）。
    ///
    /// `Option` 而不是必填：设置存储比本页更早建立（`main.rs` 的启动顺序），
    /// 但页面构造在**换根重建**时也可能跑，那时句柄一定在；拿不到就按"全部在装"
    /// 渲染（不画卸载按钮会撒谎，所以只影响状态判定，见 [`Self::is_uninstalled`]）。
    settings: Option<Entity<lithe_gpui_settings::SettingsStore>>,
    /// 盯住搜索框：查询一变就重画。必须持有（`Subscription` 是 RAII）。
    _search_subscription: Subscription,
    /// 盯住设置存储：卸载状态一变就重画（按钮与徽标跟着换）。
    _settings_subscription: Option<Subscription>,
}

impl ExtensionsPage {
    /// 建页并拉一次目录。
    ///
    /// Core 调用放在 `cx.spawn` 的任务里（不在点击派发栈里执行，判据与
    /// `ShellWorkspace::schedule_right_view_scan` 相同）：目录虽是打包数据，
    /// 但它走完整 JSON 信封与校验，不该挤占同一条输入消息。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.extensions.searchPlaceholder"))
        });
        let search_subscription = cx.subscribe_in(
            &search,
            window,
            |this: &mut Self, state: &Entity<InputState>, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.query = state.read(cx).value();
                    cx.notify();
                }
            },
        );
        let settings = lithe_gpui_settings::try_store(cx);
        let settings_subscription = settings
            .as_ref()
            .map(|store| cx.observe(store, |_, _, cx| cx.notify()));
        cx.spawn(async move |this, cx| {
            let status = match lithe_gpui_extensions::load_catalog() {
                Ok(snapshot) => CatalogStatus::Loaded(
                    snapshot
                        .extensions
                        .iter()
                        .map(Card::from_snapshot)
                        .collect(),
                ),
                Err(error) => CatalogStatus::Failed(SharedString::from(error)),
            };
            let _ = this.update(cx, |this, cx| {
                this.status = status;
                cx.notify();
            });
        })
        .detach();
        Self {
            search,
            query: SharedString::default(),
            category: None,
            expanded: None,
            status: CatalogStatus::Loading,
            settings,
            _search_subscription: search_subscription,
            _settings_subscription: settings_subscription,
        }
    }

    /// 切换文件图标主题（图标集卡片上的「使用」）。
    ///
    /// 落点是 `SettingsStore::set_icons`（它负责"认不出的包名不写入"），
    /// 本页只转交意图 —— 与卸载按钮同一条口径。
    fn use_icons(&mut self, package: &str, cx: &mut Context<Self>) {
        let Some(store) = self.settings.clone() else {
            eprintln!("S1_EXTENSIONS_PAGE action=use_no_settings package={package}");
            return;
        };
        store.update(cx, |store, cx| store.set_icons(package, cx));
        // 设置存储的订阅会让本页重绘（「当前」随之移动）。
    }

    /// 这个扩展此刻是不是"已卸载"（设置里的 `uninstalledExtensions`）。
    fn is_uninstalled(&self, id: &str, cx: &App) -> bool {
        self.settings.as_ref().is_some_and(|store| {
            store
                .read(cx)
                .uninstalled_extensions()
                .iter()
                .any(|value| value == id)
        })
    }

    /// 卸载 / 重新安装一个捆绑扩展（卡片上那个按钮的唯一动作）。
    ///
    /// 文件与设置记录的落点在 `SettingsStore::set_extension_uninstalled`（它负责
    /// "文件先动、清单后记"的顺序，理由见那里的文档）；本页只转交意图。
    fn set_uninstalled(&mut self, id: &str, uninstalled: bool, cx: &mut Context<Self>) {
        let Some(store) = self.settings.clone() else {
            eprintln!("S1_EXTENSIONS_PAGE action=no_settings id={id}");
            return;
        };
        store.update(cx, |store, cx| {
            store.set_extension_uninstalled(id, uninstalled, cx);
        });
        // 展开详情保持原样：卸载后卡片还在（只是状态变了），用户能看到「重新安装」。
    }

    /// 展开 / 折叠一条扩展的详情。
    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        self.expanded = if self.expanded.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_string())
        };
        cx.notify();
    }

    /// 筛选 + 搜索后的卡片数据（自有值）。
    ///
    /// 「筛选」与「搜索」是**与**的关系；判据是 [`card_matches_category`] 与
    /// [`card_matches_query`]（纯函数，单测在本模块）。
    fn visible_cards(&self) -> Vec<Card> {
        let CatalogStatus::Loaded(cards) = &self.status else {
            return Vec::new();
        };
        cards
            .iter()
            .filter(|card| card_matches_category(card, self.category))
            .filter(|card| card_matches_query(card, self.query.as_ref()))
            .cloned()
            .collect()
    }

    /// 顶部：图标 + 标题 + 可用数。
    fn header(&self, total: usize, cx: &App) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_8()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Icon::new(IconName::Puzzle)
                    .size_4()
                    .flex_shrink_0()
                    .text_color(cx.theme().primary),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.extensions.title")),
            )
            // 目录就绪才给计数；加载中 / 失败时数字没有意义。
            .when(total > 0, |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr_args(
                            "lithe.extensions.availableCount",
                            &[("count", &total.to_string())],
                        )),
                )
            })
            .into_any_element()
    }

    /// 搜索框 + 分类筛选条。
    fn toolbar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Input::new(&self.search)
                    .small()
                    .cleanable(true)
                    .flex_1()
                    .min_w_0()
                    .prefix(Icon::new(IconName::Search).text_color(cx.theme().muted_foreground))
                    .aria_label(tr("lithe.extensions.searchPlaceholder")),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .children(FILTERS.iter().map(|(category, key)| {
                        let selected = self.category == *category;
                        // 元素 id 用分类 key 拼接：七个按钮的 id 必须互不相同，
                        // 否则点击会串到别的筛选上（`notifications.rs` 同一条规矩）。
                        let id: SharedString =
                            format!("lithe-extensions-filter-{}", filter_key(*category)).into();
                        Button::new(id)
                            .ghost()
                            .small()
                            .selected(selected)
                            .toggled(selected)
                            .label(tr(key))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.category = *category;
                                cx.notify();
                            }))
                    })),
            )
            .into_any_element()
    }

    /// 一张扩展卡片：名称 + 版本 + 分类标签 + 状态徽标；点击展开详情。
    ///
    /// ⚠️ **行内的卸载/重装按钮必须 `stop_propagation`**：GPUI 的点击默认冒泡
    /// （`gpui-pre-0.3.6/src/app.rs:2383`），不阻断就会"点按钮同时把整行展开"
    /// （`terminal_view.rs` 的页签关闭按钮是同一处置）。
    fn card(&self, card: &Card, uninstalled: bool, cx: &mut Context<Self>) -> AnyElement {
        let expanded = self.expanded.as_deref() == Some(card.id.as_str());
        let id = card.id.clone();

        let mut column =
            v_flex()
                .w_full()
                .gap_1()
                .py_2()
                .px_3()
                .cursor_pointer()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .font_medium()
                                .text_color(cx.theme().foreground)
                                .child(card.name.clone()),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(card.version.clone()),
                        )
                        .child(
                            // 分类标签直接用 Core 推导出的稳定 id（`languages` 等），
                            // 与筛选条同一词汇，不另造一套展示名。
                            div()
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(card.categories_label.clone()),
                        )
                        // 状态徽标按**数据判据**决定（`entrypoint.kind` + 设置里的卸载清单），
                        // 不是硬编码：内置显示「内置」，捆绑未卸载显示「内置」（它确实随宿主
                        // 分发），已卸载显示「已卸载」。
                        .when(card.built_in || (card.removable && !uninstalled), |this| {
                            this.child(
                                div()
                                    .flex_shrink_0()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(tr("lithe.extensions.builtIn")),
                            )
                        })
                        .when(card.removable && uninstalled, |this| {
                            this.child(
                                div()
                                    .flex_shrink_0()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(tr("lithe.extensions.uninstalled")),
                            )
                        })
                        // 图标集：与配色主题一样"选一个生效" —— 当前那个显示「当前」（禁用），
                        // 其余显示「使用」。文案复用真源既有的 extensions.current / extensions.use。
                        .when_some(
                            card.icons_package.clone().filter(|_| !uninstalled),
                            |this, package| {
                                let is_current = package
                                    == crate::extensions_page::active_icons(cx).unwrap_or_default();
                                let label = if is_current {
                                    tr("lithe.extensions.current")
                                } else {
                                    tr("lithe.extensions.use")
                                };
                                let package_for_click = package.clone();
                                this.child(
                                    Button::new(SharedString::from(format!(
                                        "lithe-extensions-use-{}",
                                        card.id
                                    )))
                                    .outline()
                                    .small()
                                    .flex_shrink_0()
                                    .disabled(is_current)
                                    .label(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.use_icons(&package_for_click, cx);
                                    })),
                                )
                            },
                        )
                        // 卸载 / 重新安装：只有 `bundled` 入口点才有这个动作。
                        // 卸载是**可逆**的（重装从内嵌内容写回），所以用 outline 而不是 danger
                        // —— 危险样式留给不可逆操作。
                        .when(card.removable, |this| {
                            let label = if uninstalled {
                                tr("lithe.extensions.reinstall")
                            } else {
                                tr("lithe.extensions.uninstall")
                            };
                            let id_for_click = card.id.clone();
                            this.child(
                                Button::new(SharedString::from(format!(
                                    "lithe-extensions-toggle-{}",
                                    card.id
                                )))
                                .outline()
                                .small()
                                .flex_shrink_0()
                                .label(label)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        // 阻断冒泡：否则这一次点击会连带展开/折叠整行。
                                        cx.stop_propagation();
                                        this.set_uninstalled(&id_for_click, !uninstalled, cx);
                                    },
                                )),
                            )
                        }),
                );

        if expanded {
            let mut details = v_flex()
                .w_full()
                .gap_1()
                .pl_6()
                .pt_1()
                .border_l_1()
                .border_color(cx.theme().border);
            if let Some(description) = &card.description {
                details = details.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .child(description.clone()),
                );
            }
            details = details.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr_args(
                        "lithe.extensions.byPublisher",
                        &[("publisher", card.publisher.as_ref())],
                    )),
            );
            // 可卸载的字体包要写清生效时机：主题是热重载、字体要重启（见 fonts.rs 的模块文档）。
            if card.removable && card.has_fonts {
                details = details.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.extensions.fontRestartHint")),
                );
            }
            if !card.contributions.is_empty() {
                details = details
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.extensions.contributions")),
                    )
                    .children(card.contributions.iter().map(|line| {
                        div()
                            .w_full()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(line.clone())
                    }));
            }
            column = column.child(details);
        }

        // `.id(..)` 是 `on_click` 的前提（`StatefulInteractiveElement`），且必须用
        // 稳定包 id 而不是行号 —— 目录排序变化时点击不能串行。
        column
            .id(ElementId::from(SharedString::from(format!(
                "lithe-extensions-row-{}",
                card.id
            ))))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(&id, cx)))
            .into_any_element()
    }

    /// 列表区空态：加载中 / 失败 / 筛完没剩 三种文案。
    fn empty_state(&self, cx: &App) -> AnyElement {
        let (title, failure): (SharedString, Option<SharedString>) = match &self.status {
            CatalogStatus::Loading => (tr("lithe.extensions.loadingExtension"), None),
            CatalogStatus::Failed(error) => {
                (tr("lithe.extensions.loadFailed"), Some(error.clone()))
            }
            CatalogStatus::Loaded(_) => (tr("lithe.extensions.noneFound"), None),
        };
        v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .child(
                Empty::new()
                    // `Empty` 硬编码了虚线边框，产品界面统一关掉（其余空态同一招）。
                    .border_color(cx.theme().transparent)
                    .header(
                        EmptyHeader::new()
                            .media(EmptyMedia::new().child(Icon::new(IconName::Puzzle).size_8()))
                            .title(EmptyTitle::new().text_sm().child(title)),
                    ),
            )
            .children(failure.map(|error| {
                div()
                    .w_full()
                    .px_3()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(error)
            }))
            .into_any_element()
    }
}

/// 当前生效的图标主题包名（设置键 `icons`）；读不到设置时为 `None`。
///
/// 图标集卡片的「当前 / 使用」两态靠它判定 —— 判据来自设置存储（`icons` 是它的键），
/// 不是"页面自己记一个选中项"：那样会出现"设置里换过、页面还显示旧的那个"。
pub(crate) fn active_icons(cx: &App) -> Option<String> {
    let store = lithe_gpui_settings::try_store(cx)?;
    Some(store.read(cx).icons())
}

/// 筛选项的稳定元素 id 片段（`None` = 全部）。
fn filter_key(category: Option<&str>) -> &'static str {
    match category {
        None => "all",
        Some("themes") => "themes",
        Some("icons") => "icons",
        Some("fonts") => "fonts",
        Some("debug") => "debug",
        Some("lsp") => "lsp",
        Some("languages") => "languages",
        // `FILTERS` 之外的分类不会出现在筛选条上；兜底值保证 id 仍然可拼且稳定。
        Some(_) => "other",
    }
}

impl Render for ExtensionsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 先把要画的数据拷成自有值（`visible_cards` 返回 `Vec<Card>`），借用在建树
        // 之前结束 —— 之后 `toolbar` / `card` 都要 `&mut Context`。
        let cards = self.visible_cards();
        // 卸载清单也先取出来（`is_uninstalled` 要读设置实体，不能在闭包里再借一次）。
        let uninstalled: Vec<String> = self
            .settings
            .as_ref()
            .map(|store| store.read(cx).uninstalled_extensions().to_vec())
            .unwrap_or_default();
        let total = match &self.status {
            CatalogStatus::Loaded(loaded) => loaded.len(),
            _ => 0,
        };

        v_flex()
            .size_full()
            .min_w_0()
            .bg(cx.theme().background)
            .child(self.header(total, cx))
            // 与通知面板同一教训（`notifications.rs` 的 `Layout` 注释）：目录加载中 /
            // 失败时也要画工具条 —— 否则加载完成前用户连筛选都点不了，失败时更是
            // 整页只剩空态。空态只占列表区。
            .child(self.toolbar(cx))
            .child(
                v_flex()
                    .id("lithe-extensions-list")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .when(cards.is_empty(), |this| this.child(self.empty_state(cx)))
                    .when(!cards.is_empty(), |this| {
                        this.children(cards.iter().map(|card| {
                            let is_uninstalled = uninstalled.iter().any(|value| value == &card.id);
                            self.card(card, is_uninstalled, cx)
                        }))
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{Card, FILTERS, card_matches_category, card_matches_query, filter_key};

    /// 筛选条的元素 id 片段必须互不相同 —— `ElementId` 撞了点击会串到别的筛选上。
    #[test]
    fn filter_ids_are_distinct() {
        let mut ids: Vec<&str> = FILTERS
            .iter()
            .map(|(category, _)| filter_key(*category))
            .collect();
        let unique = {
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted
        };
        ids.sort_unstable();
        assert_eq!(unique, ids, "筛选条元素 id 不能重复");
    }

    /// 七个筛选键（含新增的字体 / 调试 / LSP）都要真的解析得出。
    #[test]
    fn every_filter_label_resolves() {
        for (_, key) in FILTERS {
            assert_ne!(lithe_gpui_shared::tr(key), key, "{key} 未在 locale 中定义");
        }
    }

    fn sample_card() -> Card {
        let snapshot =
            lithe_gpui_extensions::CatalogSnapshot::from_response(lithe_gpui_extensions_sample())
                .expect("样本响应应可解析");
        Card::from_snapshot(&snapshot.extensions[0])
    }

    /// 一个最小快照（`ExtensionSnapshot` 由 Core 响应形状驱动）。
    fn lithe_gpui_extensions_sample() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": 2,
            "hostVersion": "0.0.1",
            "pluginAPIVersion": 1,
            "plugins": [{
                "id": "lithe.builtinThemes",
                "displayName": "Lithe 内置主题包",
                "description": "随宿主内置的七组界面与编辑器主题",
                "version": "0.0.1",
                "apiVersion": 1,
                "hostCompatibility": {"minimum": "0.0.1"},
                "vendor": {"id": "lithe", "displayName": "Lithe Team", "signatureRequirement": "sameTeamAsHost"},
                "entrypoint": {"kind": "builtIn", "targetName": "lithe-gpui-settings"},
                "moduleIDs": ["lithe.theme.builtin"],
                "categories": ["themes"],
                "themes": [{"name": "Gruvbox", "path": "themes/gruvbox.json"}]
            }]
        })
    }

    /// 搜索覆盖贡献项名称：搜 "gruvbox"（只出现在贡献项里）要能命中主题包。
    #[test]
    fn query_matches_contribution_names() {
        let card = sample_card();
        assert!(card_matches_query(&card, "gruvbox"));
        assert!(card_matches_query(&card, "内置主题"));
        assert!(card_matches_query(&card, "lithe team"));
        assert!(!card_matches_query(&card, "jetbrains mono"));
        assert!(card_matches_query(&card, "   "));
    }

    /// 分类判据是**结构化**等值：`"icons"` 包含子串 `"themes"`，
    /// 子串匹配会把两档混在一起 —— 这条钉住等值判据。
    #[test]
    fn category_match_is_exact_not_substring() {
        let card = sample_card();
        assert!(card_matches_category(&card, None));
        assert!(card_matches_category(&card, Some("themes")));
        assert!(!card_matches_category(&card, Some("icons")));
        assert!(!card_matches_category(&card, Some("fonts")));
    }
}
