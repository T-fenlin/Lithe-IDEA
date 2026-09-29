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
use gpui_kit::component::{ActiveTheme as _, Icon, Selectable as _, Sizable as _};
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
    (Some("icon-themes"), "lithe.extensions.iconThemes"),
    (Some("fonts"), "lithe.extensions.category.fonts"),
    (Some("debug"), "lithe.extensions.category.debug"),
    (Some("lsp"), "lithe.extensions.category.lsp"),
    (Some("languages"), "lithe.extensions.languages"),
];

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
    /// 展开详情里的贡献项行（字体家族已去重）。
    contributions: Vec<SharedString>,
}

impl Card {
    fn from_snapshot(entry: &lithe_gpui_extensions::ExtensionSnapshot) -> Self {
        let mut contributions = Vec::new();
        if !entry.theme_names.is_empty() {
            contributions.push(format!("主题：{}", entry.theme_names.join("、")).into());
        }
        if !entry.icon_theme_names.is_empty() {
            contributions.push(format!("图标主题：{}", entry.icon_theme_names.join("、")).into());
        }
        if !entry.font_families.is_empty() {
            contributions.push(format!("字体：{}", entry.font_families.join("、")).into());
        }
        if !entry.debug_adapter_names.is_empty() {
            contributions
                .push(format!("调试适配器：{}", entry.debug_adapter_names.join("、")).into());
        }
        for support in &entry.language_supports {
            let extensions = support
                .file_extensions
                .iter()
                .map(|extension| format!(".{extension}"))
                .collect::<Vec<_>>()
                .join("、");
            contributions.push(format!("语言：{}（{extensions}）", support.display_name).into());
        }
        Self {
            id: entry.id.clone(),
            name: entry.display_name.clone().into(),
            version: entry.version.clone().into(),
            description: entry.description.clone().map(Into::into),
            categories: entry.categories.clone(),
            categories_label: entry.categories.join(" · ").into(),
            publisher: entry.publisher.clone().into(),
            built_in: entry.entrypoint_kind == "builtIn",
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
/// （`"icon-themes"` 包含 `"themes"`，子串会把两档混在一起）。
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
    /// 盯住搜索框：查询一变就重画。必须持有（`Subscription` 是 RAII）。
    _search_subscription: Subscription,
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
            _search_subscription: search_subscription,
        }
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

    /// 一张扩展卡片：名称 + 版本 + 分类标签 + 「内置」徽标；点击展开详情。
    ///
    /// **行内没有嵌套可点元素** —— GPUI 的点击默认冒泡，嵌套按钮会在点它时连带
    /// 展开整行（`notifications.rs` 的同一条规矩）。
    fn card(&self, card: &Card, cx: &mut Context<Self>) -> AnyElement {
        let expanded = self.expanded.as_deref() == Some(card.id.as_str());
        let id = card.id.clone();

        let mut column = v_flex()
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
                    // v1 目录全部是内置能力；徽标是数据判据（`entrypoint.kind`），
                    // 不是硬编码 —— 将来接入 nativeBundle 时它自然消失。
                    .when(card.built_in, |this| {
                        this.child(
                            div()
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(tr("lithe.extensions.builtIn")),
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

/// 筛选项的稳定元素 id 片段（`None` = 全部）。
fn filter_key(category: Option<&str>) -> &'static str {
    match category {
        None => "all",
        Some("themes") => "themes",
        Some("icon-themes") => "icon-themes",
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
                        this.children(cards.iter().map(|card| self.card(card, cx)))
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

    /// 分类判据是**结构化**等值：`"icon-themes"` 包含子串 `"themes"`，
    /// 子串匹配会把两档混在一起 —— 这条钉住等值判据。
    #[test]
    fn category_match_is_exact_not_substring() {
        let card = sample_card();
        assert!(card_matches_category(&card, None));
        assert!(card_matches_category(&card, Some("themes")));
        assert!(!card_matches_category(&card, Some("icon-themes")));
        assert!(!card_matches_category(&card, Some("fonts")));
    }
}
