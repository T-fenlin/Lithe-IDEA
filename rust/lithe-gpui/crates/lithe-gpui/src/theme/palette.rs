//! Ely 调色板移植。
//!
//! 版权与来源：色值逐项移植自 Ely GPUI Components
//! （https://github.com/ZacharyZhang-NY/Ely-GPUI-Components ，`src/theme/palette.rs`，
//! MIT OR Apache-2.0；2026-10-05 取自 main 分支）。该仓库的代码绑定 zed 上游
//! gpui，与本 crate 的 gpui-pre 0.3.7 不兼容，因此只移植设计值，不移植实现。
//!
//! 角色**命名**不照搬 Ely：语义角色名对齐 gpui-component 的 `ThemeColor`
//! （迁移友好），Ely 独有角色保留原名（surface/sunken/hover/active 等）。
//! 映射关系记录在各字段的行内注释（Ely 原名）与 CATALOG.md。
//!
//! Ely 未提供 hover/active 步进色的状态色（danger/success/warning/info/link），
//! 按 [`step`] 规则派生：light 调暗、dark 调亮。派生值非逐字移植。

use gpui_kit::{Hsla, rgb, rgba};

/// 代码 token 配色（第 17 章 Editor 使用）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Syntax {
    pub keyword: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub comment: Hsla,
    pub function: Hsla,
    pub type_name: Hsla,
    pub constant: Hsla,
    pub property: Hsla,
    pub tag: Hsla,
    pub attribute: Hsla,
    pub operator: Hsla,
    pub punctuation: Hsla,
    pub variable: Hsla,
}

/// 语义色全集。组件只读这里的角色，永远不写死颜色。
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThemeColors {
    /// 全透明色（Ghost 等静止态不占色用）。
    pub transparent: Hsla,
    /// Ely `bg`。
    pub background: Hsla,
    /// Ely `surface`。
    pub surface: Hsla,
    /// Ely `sunken`。
    pub sunken: Hsla,
    /// Ely `hover`。
    pub hover: Hsla,
    /// Ely `active`。
    pub active: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    /// Ely `fg`。
    pub foreground: Hsla,
    /// Ely `fg_muted`。
    pub muted_foreground: Hsla,
    /// Ely `fg_subtle`。
    pub subtle_foreground: Hsla,
    /// Ely `fg_disabled`。
    pub disabled_foreground: Hsla,
    /// Ely `sunken`：安静底色块。
    pub muted: Hsla,
    /// Ely `hover`：shadcn 语义的 accent（悬停填充）。
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    /// Ely `accent`（墨色主色）。Ely 的设计是单色 accent：主按钮就是墨色。
    pub primary: Hsla,
    /// Ely `accent_hover`。
    pub primary_hover: Hsla,
    /// 派生值（非 Ely 原值）。
    pub primary_active: Hsla,
    /// Ely `on_accent`。
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_hover: Hsla,
    pub secondary_active: Hsla,
    pub secondary_foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    /// Ely `overlay`。
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub danger: Hsla,
    pub danger_hover: Hsla,
    pub danger_active: Hsla,
    pub danger_foreground: Hsla,
    pub danger_subtle: Hsla,
    pub success: Hsla,
    pub success_hover: Hsla,
    pub success_active: Hsla,
    pub success_foreground: Hsla,
    pub success_subtle: Hsla,
    pub warning: Hsla,
    pub warning_hover: Hsla,
    pub warning_active: Hsla,
    pub warning_foreground: Hsla,
    pub warning_subtle: Hsla,
    pub info: Hsla,
    pub info_hover: Hsla,
    pub info_active: Hsla,
    pub info_foreground: Hsla,
    pub info_subtle: Hsla,
    pub link: Hsla,
    pub link_hover: Hsla,
    /// Ely `selection`（带透明度）。
    pub selection: Hsla,
    /// Ely `focus`。focus ring / 键盘焦点色。
    pub focus: Hsla,
    /// 输入框边框（shadcn 语义），取 Ely `border_strong`。
    pub input: Hsla,
    /// 文本光标。
    pub caret: Hsla,
    pub title_bar: Hsla,
    pub title_bar_foreground: Hsla,
    pub sidebar: Hsla,
    pub sidebar_border: Hsla,
    pub sidebar_foreground: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_accent_foreground: Hsla,
    pub sidebar_primary: Hsla,
    pub sidebar_primary_foreground: Hsla,
    pub tab: Hsla,
    pub tab_active: Hsla,
    pub tab_active_foreground: Hsla,
    pub tab_bar: Hsla,
    pub tab_foreground: Hsla,
    pub tooltip_bg: Hsla,
    pub tooltip_fg: Hsla,
    /// 遮罩层底色（带透明度）。
    pub backdrop: Hsla,
    /// 阴影基色。
    pub shadow: Hsla,
    /// Ely：媒体元素上的前景。
    pub on_media: Hsla,
    /// Ely：媒体元素底衬。
    pub media_backdrop: Hsla,
    /// Ely：流式文本微光。
    pub shimmer: Hsla,
    /// Ely：毛玻璃底。
    pub glass: Hsla,
    /// Ely：画布纸面。
    pub paper: Hsla,
    /// Ely：画布墨色。
    pub ink: Hsla,
    /// 图表 8 色（Ely `chart`，`HUE_NAMES` 顺序：Blue/Teal/Ochre/Rose/Violet/Green/Rust/Cyan）。
    pub chart: [Hsla; 8],
    /// 终端 ANSI 16 色（Ely `ansi`）。
    pub ansi: [Hsla; 16],
    /// 代码 token 配色。
    pub syntax: Syntax,
}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn ca(hex: u32) -> Hsla {
    rgba(hex).into()
}

/// 派生步进色：`toward_light` 为 true 时朝白提亮（dark 模式），否则朝黑压暗
/// （light 模式）。`t` 是步进幅度（0.06 = hover，0.12 = active）。
fn step(base: Hsla, toward_light: bool, t: f32) -> Hsla {
    Hsla {
        l: if toward_light {
            base.l + (1.0 - base.l) * t
        } else {
            base.l * (1.0 - t)
        },
        ..base
    }
}

const HOVER_T: f32 = 0.06;
const ACTIVE_T: f32 = 0.12;
const LINK_HOVER_T: f32 = 0.08;

impl ThemeColors {
    pub(crate) fn for_mode(mode: super::ThemeMode, high_contrast: bool) -> Self {
        match mode {
            super::ThemeMode::Light => Self::light(high_contrast),
            super::ThemeMode::Dark => Self::dark(high_contrast),
        }
    }

    pub fn light(high_contrast: bool) -> Self {
        let mut colors = Self {
            transparent: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.0,
            },
            background: c(0xfcfaf7),
            surface: c(0xffffff),
            sunken: c(0xf6f4f1),
            hover: c(0xf2f0ed),
            active: c(0xe9e7e5),
            border: c(0xe1dfdd),
            border_strong: c(0xcccac8),
            foreground: c(0x181613),
            muted_foreground: c(0x605d5a),
            subtle_foreground: c(0x696764),
            disabled_foreground: c(0xacaaa8),
            muted: c(0xf6f4f1),
            accent: c(0xf2f0ed),
            accent_foreground: c(0x181613),
            primary: c(0x181613),
            primary_hover: c(0x302d2b),
            primary_active: step(c(0x302d2b), false, HOVER_T),
            primary_foreground: c(0xfcfaf7),
            secondary: c(0xf6f4f1),
            secondary_hover: c(0xf2f0ed),
            secondary_active: c(0xe9e7e5),
            secondary_foreground: c(0x181613),
            card: c(0xffffff),
            card_foreground: c(0x181613),
            popover: c(0xffffff),
            popover_foreground: c(0x181613),
            danger: c(0xba3e38),
            danger_hover: step(c(0xba3e38), false, HOVER_T),
            danger_active: step(c(0xba3e38), false, ACTIVE_T),
            danger_foreground: c(0xfcfaf7),
            danger_subtle: c(0xffefec),
            success: c(0x267b4c),
            success_hover: step(c(0x267b4c), false, HOVER_T),
            success_active: step(c(0x267b4c), false, ACTIVE_T),
            success_foreground: c(0xfcfaf7),
            success_subtle: c(0xe7f9ec),
            warning: c(0xa25f12),
            warning_hover: step(c(0xa25f12), false, HOVER_T),
            warning_active: step(c(0xa25f12), false, ACTIVE_T),
            warning_foreground: c(0xfcfaf7),
            warning_subtle: c(0xfef2dd),
            info: c(0x2e69b2),
            info_hover: step(c(0x2e69b2), false, HOVER_T),
            info_active: step(c(0x2e69b2), false, ACTIVE_T),
            info_foreground: c(0xfcfaf7),
            info_subtle: c(0xebf4ff),
            link: c(0x2863ab),
            link_hover: step(c(0x2863ab), false, LINK_HOVER_T),
            selection: ca(0x3772bb33),
            focus: c(0x3772bb),
            input: c(0xcccac8),
            caret: c(0x181613),
            title_bar: c(0xfcfaf7),
            title_bar_foreground: c(0x181613),
            sidebar: c(0xf6f4f1),
            sidebar_border: c(0xe1dfdd),
            sidebar_foreground: c(0x181613),
            sidebar_accent: c(0xf2f0ed),
            sidebar_accent_foreground: c(0x181613),
            sidebar_primary: c(0x181613),
            sidebar_primary_foreground: c(0xfcfaf7),
            tab: c(0xf6f4f1),
            tab_active: c(0xffffff),
            tab_active_foreground: c(0x181613),
            tab_bar: c(0xfcfaf7),
            tab_foreground: c(0x605d5a),
            tooltip_bg: c(0x181613),
            tooltip_fg: c(0xfcfaf7),
            backdrop: ca(0x18161352),
            shadow: c(0x181613),
            on_media: c(0xffffff),
            media_backdrop: ca(0x0e0d0bf8),
            shimmer: ca(0xffffffb3),
            glass: ca(0xfcfaf7b8),
            paper: c(0xffffff),
            ink: c(0x181613),
            chart: [
                c(0x5182c1),
                c(0x009589),
                c(0xaa732b),
                c(0xb8636b),
                c(0x8572bb),
                c(0x539156),
                c(0xb66946),
                c(0x138db1),
            ],
            ansi: [
                c(0x181613),
                c(0xa34942),
                c(0x287c42),
                c(0x8c5f00),
                c(0x346aac),
                c(0x875093),
                c(0x007c84),
                c(0xacaaa8),
                c(0x605d5a),
                c(0xbd6158),
                c(0x439458),
                c(0xa57710),
                c(0x4c82c6),
                c(0xa067ac),
                c(0x00959c),
                c(0xcccac8),
            ],
            syntax: Syntax {
                keyword: c(0x714ca6),
                string: c(0x317a45),
                number: c(0xa3591b),
                comment: c(0x696764),
                function: c(0x2863ab),
                type_name: c(0x1e7777),
                constant: c(0xaa5529),
                property: c(0xa14759),
                tag: c(0xa14759),
                attribute: c(0x9f5d12),
                operator: c(0x605d5a),
                punctuation: c(0x605d5a),
                variable: c(0x181613),
            },
        };
        if high_contrast {
            colors.muted_foreground = c(0x44423f);
            colors.subtle_foreground = c(0x575552);
            colors.border = c(0xa6a4a2);
            colors.border_strong = c(0x82807d);
            colors.glass = colors.background;
        }
        colors
    }

    pub fn dark(high_contrast: bool) -> Self {
        let mut colors = Self {
            transparent: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.0,
            },
            background: c(0x131110),
            surface: c(0x1b1917),
            sunken: c(0x0e0d0b),
            hover: c(0x252422),
            active: c(0x2c2b29),
            border: c(0x2c2b29),
            border_strong: c(0x413f3d),
            foreground: c(0xf3f1f0),
            muted_foreground: c(0xafadab),
            subtle_foreground: c(0x94928f),
            disabled_foreground: c(0x5c5a58),
            muted: c(0x0e0d0b),
            accent: c(0x252422),
            accent_foreground: c(0xf3f1f0),
            primary: c(0xf3f1f0),
            primary_hover: c(0xd9d7d5),
            primary_active: step(c(0xd9d7d5), false, HOVER_T),
            primary_foreground: c(0x131110),
            secondary: c(0x0e0d0b),
            secondary_hover: c(0x252422),
            secondary_active: c(0x2c2b29),
            secondary_foreground: c(0xf3f1f0),
            card: c(0x1b1917),
            card_foreground: c(0xf3f1f0),
            popover: c(0x201f1d),
            popover_foreground: c(0xf3f1f0),
            danger: c(0xe5756e),
            danger_hover: step(c(0xe5756e), true, HOVER_T),
            danger_active: step(c(0xe5756e), true, ACTIVE_T),
            danger_foreground: c(0x131110),
            danger_subtle: c(0x3a1d1b),
            success: c(0x6bbc89),
            success_hover: step(c(0x6bbc89), true, HOVER_T),
            success_active: step(c(0x6bbc89), true, ACTIVE_T),
            success_foreground: c(0x131110),
            success_subtle: c(0x182c1f),
            warning: c(0xe8b45e),
            warning_hover: step(c(0xe8b45e), true, HOVER_T),
            warning_active: step(c(0xe8b45e), true, ACTIVE_T),
            warning_foreground: c(0x131110),
            warning_subtle: c(0x342611),
            info: c(0x77abec),
            info_hover: step(c(0x77abec), true, HOVER_T),
            info_active: step(c(0x77abec), true, ACTIVE_T),
            info_foreground: c(0x131110),
            info_subtle: c(0x19273a),
            link: c(0x7eb1f3),
            link_hover: step(c(0x7eb1f3), true, LINK_HOVER_T),
            selection: ca(0x69a1e84d),
            focus: c(0x69a1e8),
            input: c(0x413f3d),
            caret: c(0xf3f1f0),
            title_bar: c(0x131110),
            title_bar_foreground: c(0xf3f1f0),
            sidebar: c(0x0e0d0b),
            sidebar_border: c(0x2c2b29),
            sidebar_foreground: c(0xf3f1f0),
            sidebar_accent: c(0x252422),
            sidebar_accent_foreground: c(0xf3f1f0),
            sidebar_primary: c(0xf3f1f0),
            sidebar_primary_foreground: c(0x131110),
            tab: c(0x0e0d0b),
            tab_active: c(0x1b1917),
            tab_active_foreground: c(0xf3f1f0),
            tab_bar: c(0x131110),
            tab_foreground: c(0xafadab),
            tooltip_bg: c(0x373533),
            tooltip_fg: c(0xf3f1f0),
            backdrop: ca(0x00000080),
            shadow: c(0x000000),
            on_media: c(0xffffff),
            media_backdrop: ca(0x000000f8),
            shimmer: ca(0xffffff10),
            glass: ca(0x131110b8),
            paper: c(0xf3f1f0),
            ink: c(0x181613),
            chart: [
                c(0x79a7e2),
                c(0x4eb9ad),
                c(0xcd995c),
                c(0xdc8a90),
                c(0xa998dd),
                c(0x7cb57d),
                c(0xd9906f),
                c(0x56b2d4),
            ],
            ansi: [
                c(0x2c2b29),
                c(0xe1897f),
                c(0x6fb880),
                c(0xc99d4e),
                c(0x74a7e8),
                c(0xc38ecf),
                c(0x35b9c0),
                c(0xafadab),
                c(0x5c5a58),
                c(0xfda297),
                c(0x89d298),
                c(0xe3b667),
                c(0x8dc1ff),
                c(0xdda7ea),
                c(0x56d3da),
                c(0xf3f1f0),
            ],
            syntax: Syntax {
                keyword: c(0xbba3e8),
                string: c(0x8ac596),
                number: c(0xeaab78),
                comment: c(0x94928f),
                function: c(0x85b6e9),
                type_name: c(0x84c9c8),
                constant: c(0xeda382),
                property: c(0xe29fa9),
                tag: c(0xe29fa9),
                attribute: c(0xe1b671),
                operator: c(0xafadab),
                punctuation: c(0xafadab),
                variable: c(0xf3f1f0),
            },
        };
        if high_contrast {
            colors.muted_foreground = c(0xcfcdcb);
            colors.subtle_foreground = c(0xafadab);
            colors.border = c(0x6a6966);
            colors.border_strong = c(0x858380);
            colors.glass = colors.background;
        }
        colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_backgrounds_match_the_ely_port() {
        // 锚定移植值：防止后续重构悄悄改动色板基准。
        assert_eq!(ThemeColors::light(false).background, c(0xfcfaf7));
        assert_eq!(ThemeColors::dark(false).background, c(0x131110));
    }

    #[test]
    fn high_contrast_strengthens_borders() {
        // 移植自 Ely 的 palette 测试。
        let (base, hc) = (ThemeColors::light(false), ThemeColors::light(true));
        assert!(hc.border.l < base.border.l);
        let (base, hc) = (ThemeColors::dark(false), ThemeColors::dark(true));
        assert!(hc.border.l > base.border.l);
    }

    #[test]
    fn derived_steps_move_toward_the_expected_extreme() {
        // light 模式 hover/active 朝黑压暗，dark 模式朝白提亮。
        let light = ThemeColors::light(false);
        assert!(light.danger_hover.l < light.danger.l);
        assert!(light.danger_active.l < light.danger_hover.l);
        let dark = ThemeColors::dark(false);
        assert!(dark.danger_hover.l > dark.danger.l);
        assert!(dark.danger_active.l > dark.danger_hover.l);
    }
}
