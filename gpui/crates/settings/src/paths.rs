//! 设置文件的路径解析。
//!
//! ## 为什么这里可以出现 `#[cfg(target_os = ...)]`
//!
//! 「应用数据目录在哪」是纯粹的平台基础设施问题，没有第二处可以安放：gpui / gpui-kit /
//! gpui-base **都没有**数据目录 helper（`grep -r 'data_dir\|APPDATA\|config_dir'` 在
//! `gpui-pre-0.3.6` / `gpui-pre-platform-0.3.6` / `gpui-base-0.6.6` / `gpui-component-0.6.6` /
//! `gpui-kit-0.6.6` 的 `src/` 里零命中），而为一个目录再引 `dirs` 依赖也不划算。
//! 所以平台分支集中在**本模块**，业务代码（`store.rs` / `dialog.rs`）里不出现
//! `#[cfg(target_os)]`。
//!
//! ## 三平台口径
//!
//! | 平台 | 路径 | 依据 |
//! | --- | --- | --- |
//! | Windows | `%APPDATA%\Lithe\settings.json` | 任务书指定；Tauri Store 在 Windows 上也落在 Roaming AppData |
//! | macOS | `~/Library/Application Support/Lithe/settings.json` | 任务书指定 |
//! | Linux | `$XDG_CONFIG_HOME/lithe/settings.json`，回落 `~/.config/lithe/settings.json` | 任务书指定 |
//!
//! ## 测试/验证用的覆盖
//!
//! `LITHE_GPUI_SETTINGS_FILE`（**完整文件路径**，不是目录）优先于一切平台推导：
//! `.artifacts/` 下的两轮机器验证都靠它把设置文件指到临时文件，不去污染真实用户目录。

use std::path::PathBuf;

/// 覆盖设置文件完整路径的环境变量名。
pub const SETTINGS_FILE_ENV: &str = "LITHE_GPUI_SETTINGS_FILE";

/// 应用数据目录名（Windows 的 `Lithe`）。
const APP_DIR_NAME: &str = "Lithe";
/// Linux 惯例的目录名（XDG 规范建议小写）。只在非 Windows/macOS 的构建里用到，
/// 所以常量本身也跟着 cfg 走，免得在 Windows 上留一条 dead_code 警告。
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const APP_DIR_NAME_LINUX: &str = "lithe";
/// 设置文件名，与 Windows 的 Tauri Store 文件名一致（`settings.json`，
/// `windows/tauri/src/features/settings/lib/settings-persistence.ts:50`）。
const SETTINGS_FILE_NAME: &str = "settings.json";
/// 最近项目文件名。与设置文件**同目录**，但**另一份文件**（决策 Q20：
/// `gpui/research/menu-and-open-project-plan.md:35`）。
const RECENT_PROJECTS_FILE_NAME: &str = "recent-projects.json";

/// 解析设置文件的完整路径。
///
/// 返回 `None` 表示**推导不出路径**（例如 Windows 上 `APPDATA` 缺失）。此时设置仍然可用，
/// 只是不落盘 —— 调用方要打一条诊断，而不是 panic 或退回到"当前工作目录"（那会把用户的
/// 设置写进任意目录）。
pub fn settings_file_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(SETTINGS_FILE_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    default_settings_file_path()
}

/// 只看环境变量覆盖，不做平台推导（测试与诊断用）。
pub fn settings_file_override() -> Option<PathBuf> {
    std::env::var_os(SETTINGS_FILE_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// 解析最近项目文件的完整路径：**设置文件的同目录兄弟文件**。
///
/// ```text
/// %APPDATA%\Lithe\settings.json  →  %APPDATA%\Lithe\recent-projects.json
/// C:\tmp\lithe-verify\settings.json  →  C:\tmp\lithe-verify\recent-projects.json
/// ```
///
/// 为什么跟着设置文件走（而不是各自推一遍目录）：[`SETTINGS_FILE_ENV`] 是这个仓库
/// **唯一**的"把设置数据挪到临时目录"的开关，机器验证脚本（`.artifacts/` 那几轮）就靠它。
/// 最近项目如果自己从 `APPDATA` 推导，验证时就会绕过开关、把真实用户目录里的
/// 最近项目列表读进来又写回去 —— 那正是覆盖开关要避免的事。
///
/// 返回 `None` 表示设置文件路径本身推导不出来（见 [`settings_file_path`]），
/// 此时最近项目也**不落盘**，调用方照样能拿到空列表（`recent_projects::load`）。
pub fn recent_projects_file_path() -> Option<PathBuf> {
    let settings = settings_file_path()?;
    // ⚠️ `SETTINGS_FILE_ENV` 按文档是**完整文件路径**；`with_file_name` 只替换最后一段
    // （`C:\tmp\dir\` 这种带尾分隔符的写法会得到 `C:\tmp\recent-projects.json`）。
    Some(settings.with_file_name(RECENT_PROJECTS_FILE_NAME))
}

#[cfg(target_os = "windows")]
fn default_settings_file_path() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(app_data)
            .join(APP_DIR_NAME)
            .join(SETTINGS_FILE_NAME),
    )
}

#[cfg(target_os = "macos")]
fn default_settings_file_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support")
            .join(APP_DIR_NAME)
            .join(SETTINGS_FILE_NAME),
    )
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_settings_file_path() -> Option<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        if !config_home.is_empty() {
            return Some(
                PathBuf::from(config_home)
                    .join(APP_DIR_NAME_LINUX)
                    .join(SETTINGS_FILE_NAME),
            );
        }
    }
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join(APP_DIR_NAME_LINUX)
            .join(SETTINGS_FILE_NAME),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 环境变量覆盖是验证链路的硬依赖：它必须**逐字**返回完整的文件路径。
    ///
    /// ⚠️ `set_var` / `remove_var` 是进程级状态，而 `cargo test` 默认并行跑同一个二进制里的
    /// 测试，所以本模块的测试统一在一个锁下串行执行，并在结束时还原（照
    /// `gpui/crates/shared/src/i18n.rs:266-280` 的写法）。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard(Option<std::ffi::OsString>);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.0 {
                // SAFETY: 由 ENV_LOCK 保证同一时刻只有一个测试在改环境变量。
                Some(value) => unsafe { std::env::set_var(SETTINGS_FILE_ENV, value) },
                None => unsafe { std::env::remove_var(SETTINGS_FILE_ENV) },
            }
        }
    }

    #[test]
    fn env_override_wins_over_platform_defaults() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _restore = EnvGuard(std::env::var_os(SETTINGS_FILE_ENV));

        // SAFETY: 见 ENV_LOCK 的说明。
        unsafe { std::env::set_var(SETTINGS_FILE_ENV, r"C:\tmp\lithe-test-settings.json") };
        assert_eq!(
            settings_file_path(),
            Some(PathBuf::from(r"C:\tmp\lithe-test-settings.json"))
        );
        assert_eq!(
            settings_file_override(),
            Some(PathBuf::from(r"C:\tmp\lithe-test-settings.json"))
        );
    }

    /// 空字符串不算覆盖（有些 shell 会导出空值），否则会得到一个"空路径"设置文件。
    #[test]
    fn empty_env_value_is_not_an_override() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _restore = EnvGuard(std::env::var_os(SETTINGS_FILE_ENV));

        // SAFETY: 见 ENV_LOCK 的说明。
        unsafe { std::env::set_var(SETTINGS_FILE_ENV, "") };
        assert_eq!(settings_file_override(), None);
    }

    /// 最近项目文件是设置文件的**同目录兄弟文件** —— 机器验证脚本靠 `SETTINGS_FILE_ENV`
    /// 把两份文件一起挪进临时目录；这条测试守的是"最近项目不许绕过那个开关"。
    #[test]
    fn recent_projects_file_sits_next_to_the_settings_file() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _restore = EnvGuard(std::env::var_os(SETTINGS_FILE_ENV));

        // SAFETY: 见 ENV_LOCK 的说明。
        unsafe {
            std::env::set_var(
                SETTINGS_FILE_ENV,
                r"C:\tmp\lithe-verify\settings.json",
            )
        };
        assert_eq!(
            recent_projects_file_path(),
            Some(PathBuf::from(r"C:\tmp\lithe-verify\recent-projects.json"))
        );

        // 没有覆盖时也必须是兄弟关系（平台默认目录不在这里断言，只断言相对关系与文件名）。
        // SAFETY: 见 ENV_LOCK 的说明。
        unsafe { std::env::remove_var(SETTINGS_FILE_ENV) };
        assert_eq!(
            recent_projects_file_path(),
            settings_file_path().map(|settings| settings.with_file_name("recent-projects.json"))
        );
    }
}
