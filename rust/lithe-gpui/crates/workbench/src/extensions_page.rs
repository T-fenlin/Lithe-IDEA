use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};

use lithe_gpui_shared::{tr, tr_args};

const FILTERS: [(Option<&'static str>, &'static str); 7] = [
    (None, "lithe.extensions.all"),
    (Some("themes"), "lithe.extensions.themes"),
    (Some("icons"), "lithe.extensions.icons"),
    (Some("fonts"), "lithe.extensions.category.fonts"),
    (Some("debug"), "lithe.extensions.category.debug"),
    (Some("lsp"), "lithe.extensions.category.lsp"),
    (Some("languages"), "lithe.extensions.languages"),
];

fn category_label(category: &str) -> SharedString {
    match category {
        "themes" => tr("lithe.extensions.themes"),
        "icons" => tr("lithe.extensions.icons"),
        "fonts" => tr("lithe.extensions.category.fonts"),
        "debug" => tr("lithe.extensions.category.debug"),
        "lsp" => tr("lithe.extensions.category.lsp"),
        "languages" => tr("lithe.extensions.languages"),
        other => SharedString::from(other.to_string()),
    }
}

#[derive(Debug)]
enum CatalogStatus {
    Loading,
    Loaded(Vec<Card>),
    Failed(SharedString),
}

#[derive(Debug, Clone)]
struct Card {
    id: String,
    name: SharedString,
    version: SharedString,
    description: Option<SharedString>,
    categories: Vec<String>,
    categories_label: SharedString,
    publisher: SharedString,
    built_in: bool,
    removable: bool,
    has_fonts: bool,
    has_languages: bool,
    icons_package: Option<String>,
    contributions: Vec<SharedString>,
}

impl Card {
    fn from_snapshot(entry: &lithe_gpui_extensions::ExtensionSnapshot) -> Self {
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
            has_languages: !entry.language_supports.is_empty(),
            icons_package: if entry.categories.iter().any(|item| item == "icons") {
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

fn card_matches_category(card: &Card, category: Option<&str>) -> bool {
    match category {
        None => true,
        Some(category) => card.categories.iter().any(|item| item == category),
    }
}

pub struct ExtensionsPage {
    search: Entity<InputState>,
    query: SharedString,
    category: Option<&'static str>,
    expanded: Option<String>,
    status: CatalogStatus,
    settings: Option<Entity<lithe_gpui_settings::SettingsStore>>,
    _search_subscription: Subscription,
    _settings_subscription: Option<Subscription>,
}

impl ExtensionsPage {
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

    fn use_icons(&mut self, package: &str, cx: &mut Context<Self>) {
        let Some(store) = self.settings.clone() else {
            eprintln!("S1_EXTENSIONS_PAGE action=use_no_settings package={package}");
            return;
        };
        store.update(cx, |store, cx| store.set_icons(package, cx));
    }

    fn set_uninstalled(&mut self, id: &str, uninstalled: bool, cx: &mut Context<Self>) {
        let Some(store) = self.settings.clone() else {
            eprintln!("S1_EXTENSIONS_PAGE action=no_settings id={id}");
            return;
        };
        store.update(cx, |store, cx| {
            store.set_extension_uninstalled(id, uninstalled, cx);
        });
    }

    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        self.expanded = if self.expanded.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_string())
        };
        cx.notify();
    }

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
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        rems(14. / 16.),
                    ))
                    .aria_label(tr("lithe.extensions.searchPlaceholder")),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .children(FILTERS.iter().map(|(category, key)| {
                        let selected = self.category == *category;
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
                            div()
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(card.categories_label.clone()),
                        )
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
            if card.removable && card.has_languages {
                details = details.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.extensions.languageUninstallHint")),
                );
            }
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

        column
            .id(ElementId::from(SharedString::from(format!(
                "lithe-extensions-row-{}",
                card.id
            ))))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(&id, cx)))
            .into_any_element()
    }

    fn empty_state(&self, cx: &App) -> AnyElement {
        let (title, failure): (SharedString, Option<SharedString>) = match &self.status {
            CatalogStatus::Loading => (tr("lithe.extensions.loadingExtension"), None),
            CatalogStatus::Failed(error) => {
                (tr("lithe.extensions.loadFailed"), Some(error.clone()))
            }
            CatalogStatus::Loaded(_) => (tr("lithe.extensions.noneFound"), None),
        };
        lithe_gpui_shared::empty_state::unavailable(
            Some(Icon::new(IconName::Puzzle).size(rems(
                lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.,
            ))),
            title,
            None,
            failure.map(|error| {
                div()
                    .w_full()
                    .px_3()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(error)
                    .into_any_element()
            }),
            cx,
        )
    }
}

pub(crate) fn active_icons(cx: &App) -> Option<String> {
    let store = lithe_gpui_settings::try_store(cx)?;
    Some(store.read(cx).icons())
}

fn filter_key(category: Option<&str>) -> &'static str {
    match category {
        None => "all",
        Some("themes") => "themes",
        Some("icons") => "icons",
        Some("fonts") => "fonts",
        Some("debug") => "debug",
        Some("lsp") => "lsp",
        Some("languages") => "languages",
        Some(_) => "other",
    }
}

impl Render for ExtensionsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cards = self.visible_cards();
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

    #[test]
    fn query_matches_contribution_names() {
        let card = sample_card();
        assert!(card_matches_query(&card, "gruvbox"));
        assert!(card_matches_query(&card, "内置主题"));
        assert!(card_matches_query(&card, "lithe team"));
        assert!(!card_matches_query(&card, "jetbrains mono"));
        assert!(card_matches_query(&card, "   "));
    }

    #[test]
    fn category_match_is_exact_not_substring() {
        let card = sample_card();
        assert!(card_matches_category(&card, None));
        assert!(card_matches_category(&card, Some("themes")));
        assert!(!card_matches_category(&card, Some("icons")));
        assert!(!card_matches_category(&card, Some("fonts")));
    }
}
