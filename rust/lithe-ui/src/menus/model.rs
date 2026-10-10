use std::{ops::Range, rc::Rc};

use gpui::{AnyElement, App, Rems, SharedString, Window};

use crate::{forms::Run, navigation::fuzzy, primitives::IconName};

/// What a row does when chosen.
#[derive(Clone)]
pub(super) enum Kind {
    Run,
    Check(bool),
    Radio(bool),
    Sub(Menu),
}

/// 调用方自画的一行内容（见 [`MenuItem::element`]）。
pub(super) type Build = Rc<dyn Fn(&App) -> AnyElement>;

/// One row of a menu. Its keys are shown, not bound.
#[derive(Clone)]
pub struct MenuItem {
    pub(super) label: SharedString,
    pub(super) icon: Option<IconName>,
    pub(super) keys: Option<SharedString>,
    pub(super) kind: Kind,
    pub(super) disabled: bool,
    pub(super) on_click: Option<Run>,
    pub(super) hits: Vec<Range<usize>>,
    /// 整行由调用方画时的内容；给了它，`label` / `icon` / `keys` 都不画。
    pub(super) element: Option<Build>,
}

impl MenuItem {
    fn with(label: impl Into<SharedString>, kind: Kind) -> Self {
        Self {
            label: label.into(),
            icon: None,
            keys: None,
            kind,
            disabled: false,
            on_click: None,
            hits: Vec::new(),
            element: None,
        }
    }

    pub fn new(label: impl Into<SharedString>) -> Self {
        Self::with(label, Kind::Run)
    }

    /// 一行由调用方整行自画：徽标 + 两行文字这种一句话说不清的项。
    ///
    /// 行外壳（最小高度、内边距、圆角、hover 底、整行可点）仍由库给，`build` 只画内容。
    /// 富行没有 `label`，所以不参与 [`Menu`] 的字母过滤（[`super::SearchableMenu`] 那类）；
    /// [`Self::disabled`] 与 [`Self::on_click`] 照常生效。
    ///
    /// `build` 只拿得到 `&App`——绘制期没有 `&mut Window`——所以尺寸请用 [`Rems`] 表达，
    /// 别去问窗口的 `rem_size()`。
    pub fn element(build: impl Fn(&App) -> AnyElement + 'static) -> Self {
        Self {
            element: Some(Rc::new(build)),
            ..Self::with("", Kind::Run)
        }
    }

    /// A row with a check; the owner flips `checked` in `on_click`.
    pub fn check(label: impl Into<SharedString>, checked: bool) -> Self {
        Self::with(label, Kind::Check(checked))
    }

    /// One of a set; the owner moves `selected` in `on_click`.
    pub fn radio(label: impl Into<SharedString>, selected: bool) -> Self {
        Self::with(label, Kind::Radio(selected))
    }

    /// A row that opens `menu` beside it.
    pub fn submenu(label: impl Into<SharedString>, menu: Menu) -> Self {
        Self::with(label, Kind::Sub(menu))
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Its chord at the row's end, in gpui key syntax.
    pub fn keys(mut self, keys: impl Into<SharedString>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

#[derive(Clone)]
pub(super) enum Entry {
    Item(MenuItem),
    Separator,
    Heading(SharedString),
}

/// Rows, separators and titled groups. A row can open a submenu.
#[derive(Clone, Default)]
pub struct Menu {
    pub(super) entries: Vec<Entry>,
    /// 面板定宽；`None` 用主题的菜单宽度。
    pub(super) width: Option<Rems>,
    /// 面板最大高度；给了它整个面板滚动。
    pub(super) max_height: Option<Rems>,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// 一串行，按给的顺序接在后面。
    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.entries.extend(items.into_iter().map(Entry::Item));
        self
    }

    /// 面板定宽（最小、最大都锁到这个宽度）。不给就用主题的菜单宽度。
    ///
    /// 定宽面板是给「行里有富内容」的菜单用的：徽标 + 路径那种行，宽度随内容变会让
    /// 每一帧的面板宽度都不同。
    pub fn width(mut self, width: impl Into<Rems>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// 面板最大高度；超出的部分滚动。不给就不限高。
    pub fn max_height(mut self, height: impl Into<Rems>) -> Self {
        self.max_height = Some(height.into());
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    /// A titled run of rows, set off from the rows before it.
    pub fn group(
        mut self,
        title: impl Into<SharedString>,
        items: impl IntoIterator<Item = MenuItem>,
    ) -> Self {
        if !self.entries.is_empty() {
            self.entries.push(Entry::Separator);
        }
        self.entries.push(Entry::Heading(title.into()));
        self.entries.extend(items.into_iter().map(Entry::Item));
        self
    }

    pub(super) fn item_at(&self, ix: usize) -> Option<&MenuItem> {
        match self.entries.get(ix) {
            Some(Entry::Item(item)) => Some(item),
            _ => None,
        }
    }

    /// The next enabled row from `from`, stepping by `by` and wrapping.
    pub(super) fn step(&self, from: Option<usize>, by: isize) -> Option<usize> {
        let count = self.entries.len() as isize;
        let mut at = from.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
        for _ in 0..count {
            at = (at + by).rem_euclid(count);
            if self.item_at(at as usize).is_some_and(|item| !item.disabled) {
                return Some(at as usize);
            }
        }
        None
    }

    /// The rows whose labels fit `query`, best first, with the letters that matched. All of it for an empty query.
    pub(super) fn filtered(&self, query: &str) -> Menu {
        if query.is_empty() {
            return self.clone();
        }
        let mut found: Vec<(i32, MenuItem)> = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Item(item) => fuzzy(query, &item.label).map(|fit| {
                    let item = MenuItem {
                        hits: fit.hits,
                        ..item.clone()
                    };
                    (fit.score, item)
                }),
                _ => None,
            })
            .collect();
        found.sort_by(|(a, _), (b, _)| b.cmp(a));
        Menu {
            entries: found
                .into_iter()
                .map(|(_, item)| Entry::Item(item))
                .collect(),
            ..self.clone()
        }
    }
}
