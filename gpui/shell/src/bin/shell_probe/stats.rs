//! 右侧「项目统计」面板的数据模型：`StatRow` 与 `StatsDelegate`。
//!
//! 委托直接用真实的工作区快照数据填充，不是为了验证组件而造的假数据。
//! 行数据由 [`super::files::load_files`] 产出，再由工作台写回面板自己的 `TableState`。

use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::{
    App, Context, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div, px,
};

/// 统计表的一行：顶层目录、文件数、占比。
pub(super) struct StatRow {
    pub(super) name: String,
    pub(super) files: usize,
    pub(super) share: f32,
}

/// `DataTable` 的委托。用真实快照数据填充，不是为了验证组件而造的假数据。
pub(super) struct StatsDelegate {
    pub(super) rows: Vec<StatRow>,
}

impl TableDelegate for StatsDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        3
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        // 三列总宽要 ≤ 右侧 Dock 的 240px，否则"占比"列会被面板右边裁掉。
        match col_ix {
            0 => Column::new("directory", "目录").width(px(96.)),
            1 => Column::new("files", "文件数").width(px(58.)).text_right(),
            _ => Column::new("share", "占比").width(px(54.)).text_right(),
        }
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let row = match self.rows.get(row_ix) {
            Some(row) => row,
            None => return div().size_full().into_any_element(),
        };

        let text = match col_ix {
            0 => row.name.clone(),
            1 => row.files.to_string(),
            _ => format!("{:.1}%", row.share * 100.0),
        };

        div()
            .size_full()
            .px_2()
            .child(SharedString::from(text))
            .into_any_element()
    }
}
