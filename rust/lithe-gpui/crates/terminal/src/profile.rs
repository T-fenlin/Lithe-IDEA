use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use gpui::SharedString;

#[derive(Clone)]
#[non_exhaustive]
pub struct TerminalProfile {
    name: SharedString,
    program: PathBuf,
    args: Vec<String>,
    working_directory: Option<PathBuf>,
}

impl TerminalProfile {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        let program = program.into();
        Self {
            name: SharedString::from(program.to_string_lossy().to_string()),
            program,
            args: Vec::new(),
            working_directory: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_working_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(directory.into());
        self
    }

    pub fn name(&self) -> &SharedString {
        &self.name
    }

    pub(crate) fn program(&self) -> &PathBuf {
        &self.program
    }

    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    pub(crate) fn working_directory(&self) -> Option<&PathBuf> {
        self.working_directory.as_ref()
    }
}

pub(crate) const SHELL_SYSTEM_DEFAULT: &str = "";

#[allow(dead_code)]
pub(crate) const SHELL_IDS: [&str; 4] = [SHELL_SYSTEM_DEFAULT, "powershell", "cmd", "wsl"];

pub(crate) fn profile_for_shell_id(id: &str) -> Option<TerminalProfile> {
    match id {
        "powershell" | "cmd" | "wsl" => Some(TerminalProfile::new(id)),
        _ => None,
    }
}

pub(crate) fn command_exists(program: &str) -> bool {
    for probe in ["where.exe", "which"] {
        let Ok(status) = Command::new(probe)
            .arg(program)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        else {
            continue;
        };
        if status.success() {
            return true;
        }
    }
    false
}

pub(crate) fn profiles() -> Vec<TerminalProfile> {
    detected_profiles().to_vec()
}

pub(crate) fn default_profile() -> Option<TerminalProfile> {
    detected_profiles().first().cloned()
}

fn detected_profiles() -> &'static [TerminalProfile] {
    static DETECTED: OnceLock<Vec<TerminalProfile>> = OnceLock::new();
    DETECTED.get_or_init(|| {
        let mut detected = Vec::new();
        for program in ["powershell", "cmd"] {
            if command_exists(program) {
                detected.push(TerminalProfile::new(program));
            }
        }
        detected
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_ids_map_to_programs() {
        for id in ["powershell", "cmd", "wsl"] {
            let profile = profile_for_shell_id(id).expect("这三个 id 必须映射出程序");
            assert_eq!(profile.program().to_string_lossy(), id);
        }
        assert!(profile_for_shell_id(SHELL_SYSTEM_DEFAULT).is_none());
        assert!(profile_for_shell_id("fish").is_none());
    }

    #[test]
    fn shell_id_table_matches_the_settings_whitelist() {
        assert_eq!(
            SHELL_IDS,
            ["", "powershell", "cmd", "wsl"],
            "与 lithe-gpui-settings 的 TERMINAL_SHELL_IDS 必须逐项一致"
        );
    }
}
