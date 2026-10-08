use std::path::PathBuf;

pub const SETTINGS_FILE_ENV: &str = "LITHE_GPUI_SETTINGS_FILE";

pub const THEMES_DIR_ENV: &str = "LITHE_GPUI_THEMES_DIR";

pub const THEME_EXTENSIONS_DIR_ENV: &str = "LITHE_GPUI_THEME_EXTENSIONS_DIR";

pub const FONT_EXTENSIONS_DIR_ENV: &str = "LITHE_GPUI_FONT_EXTENSIONS_DIR";

const APP_DIR_NAME: &str = "Lithe";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const APP_DIR_NAME_LINUX: &str = "lithe";
const SETTINGS_FILE_NAME: &str = "settings.json";
const RECENT_PROJECTS_FILE_NAME: &str = "recent-projects.json";
const THEMES_DIRECTORY_NAME: &str = "themes";
const EXTENSIONS_DIRECTORY_NAME: &str = "extensions";
const THEME_EXTENSIONS_DIRECTORY_NAME: &str = "themes";
const FONT_EXTENSIONS_DIRECTORY_NAME: &str = "fonts";

pub fn settings_file_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(SETTINGS_FILE_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    default_settings_file_path()
}

pub fn settings_file_override() -> Option<PathBuf> {
    std::env::var_os(SETTINGS_FILE_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn recent_projects_file_path() -> Option<PathBuf> {
    let settings = settings_file_path()?;
    Some(settings.with_file_name(RECENT_PROJECTS_FILE_NAME))
}

pub fn themes_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(THEMES_DIR_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    Some(settings_file_path()?.parent()?.join(THEMES_DIRECTORY_NAME))
}

pub fn repository_themes_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes"))
}

pub fn repository_theme_extensions_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/themes"
    ))
}

pub fn extensions_dir() -> Option<PathBuf> {
    Some(
        settings_file_path()?
            .parent()?
            .join(EXTENSIONS_DIRECTORY_NAME),
    )
}

pub fn theme_extensions_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(THEME_EXTENSIONS_DIR_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    Some(extensions_dir()?.join(THEME_EXTENSIONS_DIRECTORY_NAME))
}

pub fn repository_icons_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icons"))
}

pub fn repository_font_extensions_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/fonts"
    ))
}

pub fn font_extensions_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(FONT_EXTENSIONS_DIR_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    Some(extensions_dir()?.join(FONT_EXTENSIONS_DIRECTORY_NAME))
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

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard(&'static str, Option<std::ffi::OsString>);

    impl EnvGuard {
        fn capture(name: &'static str) -> Self {
            Self(name, std::env::var_os(name))
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.1 {
                Some(value) => unsafe { std::env::set_var(self.0, value) },
                None => unsafe { std::env::remove_var(self.0) },
            }
        }
    }

    fn lock_env() -> (
        std::sync::MutexGuard<'static, ()>,
        EnvGuard,
        EnvGuard,
        EnvGuard,
    ) {
        let guard = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (
            guard,
            EnvGuard::capture(SETTINGS_FILE_ENV),
            EnvGuard::capture(THEMES_DIR_ENV),
            EnvGuard::capture("XDG_CONFIG_HOME"),
        )
    }

    #[test]
    fn env_override_wins_over_platform_defaults() {
        let (_guard, _settings, _themes, _xdg) = lock_env();

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

    #[test]
    fn empty_env_value_is_not_an_override() {
        let (_guard, _settings, _themes, _xdg) = lock_env();

        unsafe { std::env::set_var(SETTINGS_FILE_ENV, "") };
        assert_eq!(settings_file_override(), None);
    }

    #[test]
    fn recent_projects_file_sits_next_to_the_settings_file() {
        let (_guard, _settings, _themes, _xdg) = lock_env();

        unsafe { std::env::set_var(SETTINGS_FILE_ENV, r"C:\tmp\lithe-verify\settings.json") };
        assert_eq!(
            recent_projects_file_path(),
            Some(PathBuf::from(r"C:\tmp\lithe-verify\recent-projects.json"))
        );

        unsafe { std::env::remove_var(SETTINGS_FILE_ENV) };
        assert_eq!(
            recent_projects_file_path(),
            settings_file_path().map(|settings| settings.with_file_name("recent-projects.json"))
        );
    }

    #[test]
    fn themes_dir_env_override_wins() {
        let (_guard, _settings, _themes, _xdg) = lock_env();

        unsafe { std::env::set_var(THEMES_DIR_ENV, r"C:\tmp\lithe-themes-override") };
        assert_eq!(
            themes_dir(),
            Some(PathBuf::from(r"C:\tmp\lithe-themes-override"))
        );

        unsafe { std::env::set_var(THEMES_DIR_ENV, "") };
        unsafe { std::env::set_var(SETTINGS_FILE_ENV, r"C:\tmp\lithe-verify\settings.json") };
        assert_eq!(
            themes_dir(),
            Some(PathBuf::from(r"C:\tmp\lithe-verify\themes"))
        );
    }

    #[test]
    fn themes_dir_follows_the_settings_file_directory() {
        let (_guard, _settings, _themes, _xdg) = lock_env();

        unsafe { std::env::remove_var(THEMES_DIR_ENV) };
        unsafe { std::env::set_var(SETTINGS_FILE_ENV, r"C:\tmp\lithe-verify\settings.json") };
        assert_eq!(
            themes_dir(),
            Some(PathBuf::from(r"C:\tmp\lithe-verify\themes"))
        );

        unsafe { std::env::remove_var(SETTINGS_FILE_ENV) };
        assert_eq!(
            themes_dir(),
            settings_file_path()
                .and_then(|settings| settings.parent().map(|dir| dir.join("themes")))
        );
    }

    #[test]
    fn repository_themes_dir_points_at_the_repo_themes_folder() {
        let dir = repository_themes_dir();
        assert!(dir.ends_with("themes"), "{dir:?}");
        assert!(dir.join("jetbrains.json").exists(), "{dir:?}");
    }

    #[test]
    fn repository_theme_extensions_dir_points_at_the_repo_extensions_folder() {
        let dir = repository_theme_extensions_dir();
        assert!(dir.ends_with("themes"), "{dir:?}");
        assert!(
            dir.parent()
                .is_some_and(|parent| parent.ends_with("extensions")),
            "{dir:?}"
        );
        assert!(dir.join("lithe.json").exists(), "{dir:?}");
    }
}
