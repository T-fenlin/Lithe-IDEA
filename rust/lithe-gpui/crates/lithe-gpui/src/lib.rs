//! Lithe 的自有 GPUI 组件库。
//!
//! 分层与依赖：
//! - 上游唯一依赖 [`gpui-kit`](https://gpui-kit.com)（关闭默认 feature），只使用其
//!   `gpui-base` 行为层；gpui 本体经本 crate 根再导出，调用方不需要直接依赖 gpui。
//! - 视觉语言（色板、圆角、密度刻度）移植自 Ely GPUI Components，语义角色命名对齐
//!   gpui-component 的 `Theme`，为宿主将来从 gpui-component 迁移铺路。
//! - 本 crate 是纯库：不依赖任何 lithe crate，图标与字体一律由调用方注入。
//!
//! 入口：
//! - [`init`]：注册主题 Global 并初始化 gpui-base，创建任何窗口前调用一次。
//!   它取代 `gpui_kit::init`（在关闭 component feature 的前提下后者只做 base 初始化）。
//! - [`ActiveTheme`]：`cx.theme()` 读取当前语义 token。
//! - [`open_window`]（gpui-kit 提供）：窗口首层自带 base `Root`。
//!
//! 组件目录与章节对账见本 crate 根目录的 `CATALOG.md`。

pub use gpui_kit::base;
pub use gpui_kit::*;

mod avatar;
mod badge;
mod button;
mod card;
mod checkbox;
mod code_block;
mod debug;
mod dock;
mod feedback;
mod git;
mod icon;
mod icon_button;
mod input;
mod kbd;
mod label;
mod link;
mod nav;
mod overlay;
mod progress;
mod radio;
mod separator;
mod severity;
mod shell;
mod sizing;
mod spinner;
mod switch;
mod tabs;
mod tag;
mod theme;
mod typography;

pub use avatar::Avatar;
pub use badge::{Badge, BadgeVariant};
pub use button::{Button, ButtonVariant};
pub use card::{Card, CardTitle, SectionHeader, Spacer};
pub use checkbox::Checkbox;
pub use code_block::CodeBlock;
pub use debug::{FrameList, StackFrame, Variable, VariableList};
pub use dock::{Dock, DockPanel, h_dock, resize_handle_appearance, v_dock};
pub use feedback::{Alert, Empty};
pub use git::{CommitList, CommitSummary, DiffLine, DiffLineKind, DiffStats, DiffView};
pub use gpui_kit::base::ResizablePanelEvent as DockPanelEvent;
pub use gpui_kit::base::ResizableState;
pub use icon::Icon;
pub use icon_button::IconButton;
pub use input::{Input, InputEvent, InputState};
pub use kbd::Kbd;
pub use label::Label;
pub use link::Link;
pub use nav::{Breadcrumb, Pagination};
pub use overlay::{
    Menu, MenuEntry, MenuItem, Notification, OverlayState, close_menu, open_menu, push_notification,
};
pub use progress::Progress;
pub use radio::Radio;
pub use separator::Separator;
pub use severity::Severity;
pub use shell::{Sidebar, StatusBar, TitleBar};
pub use sizing::{Sizable, Size};
pub use spinner::Spinner;
pub use switch::Switch;
pub use tabs::{Tab, Tabs};
pub use tag::Tag;
pub use theme::{ActiveTheme, Syntax, Theme, ThemeColors, ThemeMode, init};
pub use typography::{Code, Heading, HeadingLevel, Text};
