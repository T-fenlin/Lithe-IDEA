use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaToolchainOverride {
    pub java_home: PathBuf,
}

static OVERRIDE: Mutex<Option<JavaToolchainOverride>> = Mutex::new(None);

pub fn set_java_toolchain_override(overridden: Option<JavaToolchainOverride>) {
    let mut slot = OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = overridden;
}

pub(crate) fn java_home_override() -> Option<PathBuf> {
    let slot = OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slot.as_ref().map(|overridden| overridden.java_home.clone())
}

static MAVEN_SETTINGS: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_maven_settings_override(settings: Option<PathBuf>) {
    let mut slot = MAVEN_SETTINGS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = settings;
}

pub(crate) fn maven_settings_override() -> Option<PathBuf> {
    let slot = MAVEN_SETTINGS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slot.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cleared;

    impl Drop for Cleared {
        fn drop(&mut self) {
            set_java_toolchain_override(None);
            set_maven_settings_override(None);
        }
    }

    #[test]
    fn the_registered_override_is_read_back_and_can_be_cleared() {
        let _cleared = Cleared;
        set_java_toolchain_override(None);
        assert_eq!(java_home_override(), None);

        let path = PathBuf::from("jdk-override");
        set_java_toolchain_override(Some(JavaToolchainOverride {
            java_home: path.clone(),
        }));
        assert_eq!(java_home_override(), Some(path));

        set_java_toolchain_override(None);
        assert_eq!(java_home_override(), None);
    }

    #[test]
    fn the_maven_settings_slot_is_independent_from_the_jdk_slot() {
        let _cleared = Cleared;
        set_java_toolchain_override(None);
        set_maven_settings_override(None);
        assert_eq!(maven_settings_override(), None);

        let settings = PathBuf::from("ci-settings.xml");
        set_maven_settings_override(Some(settings.clone()));
        assert_eq!(maven_settings_override(), Some(settings));
        assert_eq!(java_home_override(), None);

        set_maven_settings_override(None);
        assert_eq!(maven_settings_override(), None);
    }
}
