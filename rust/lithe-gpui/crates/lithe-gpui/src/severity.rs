//! 语义严重级别：Badge/Tag/Alert/日志行等共用的语义色映射。
//! 集中在这里，避免每个组件各自做一遍"变体 → 主题角色"的映射。

use gpui_kit::Hsla;

use crate::ThemeColors;

/// 语义严重级别。语义保持稀缺：Neutral 用于普通元数据。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Severity {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

impl ThemeColors {
    /// 语义色（文字/图标用）。
    pub fn severity(&self, severity: Severity) -> Hsla {
        match severity {
            Severity::Neutral => self.muted_foreground,
            Severity::Info => self.info,
            Severity::Success => self.success,
            Severity::Warning => self.warning,
            Severity::Danger => self.danger,
        }
    }

    /// 语义色的安静底（subtle 背景）。
    pub fn severity_subtle(&self, severity: Severity) -> Hsla {
        match severity {
            Severity::Neutral => self.muted,
            Severity::Info => self.info_subtle,
            Severity::Success => self.success_subtle,
            Severity::Warning => self.warning_subtle,
            Severity::Danger => self.danger_subtle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Severity;
    use crate::Theme;

    #[test]
    fn severities_map_to_distinct_colors_in_both_modes() {
        // 纯 Global 读数，不开窗口：每个级别在 light/dark 下都必须可区分。
        for dark in [false, true] {
            let theme = if dark {
                Theme::dark(false)
            } else {
                Theme::light(false)
            };
            let severity = |s| theme.severity(s);
            assert_ne!(severity(Severity::Info), severity(Severity::Success));
            assert_ne!(severity(Severity::Success), severity(Severity::Warning));
            assert_ne!(severity(Severity::Warning), severity(Severity::Danger));
            assert_ne!(severity(Severity::Neutral), severity(Severity::Info));

            let subtle = |s| theme.severity_subtle(s);
            assert_ne!(subtle(Severity::Neutral), subtle(Severity::Info));
            assert_ne!(subtle(Severity::Success), subtle(Severity::Danger));
        }
    }
}
