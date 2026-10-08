use crate::model::execute_core;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IdentityScope {
    #[default]
    Local,
    Global,
}

impl IdentityScope {
    pub fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
        }
    }

    pub const ALL: [IdentityScope; 2] = [IdentityScope::Local, IdentityScope::Global];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityField {
    Name,
    Email,
}

impl IdentityField {
    pub fn id(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdentitySetup {
    pub is_repository: bool,
    pub has_commits: bool,
    pub branch: Option<String>,
    pub scope: IdentityScope,
    pub configured_name: Option<String>,
    pub configured_email: Option<String>,
    pub effective_name: Option<String>,
    pub effective_email: Option<String>,
}

impl IdentitySetup {
    pub fn configured(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Name => self.configured_name.as_deref(),
            IdentityField::Email => self.configured_email.as_deref(),
        }
    }

    pub fn effective(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Name => self.effective_name.as_deref(),
            IdentityField::Email => self.effective_email.as_deref(),
        }
    }

    pub fn editable(&self, scope: IdentityScope) -> bool {
        scope == IdentityScope::Global || self.is_repository
    }
}

pub const IDENTITY_MAX_BYTES: usize = 1024;

pub fn validate_value(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() {
        return Err("empty");
    }
    if value.len() > IDENTITY_MAX_BYTES {
        return Err("too_long");
    }
    if value
        .chars()
        .any(|character| character.is_control() || character == '<' || character == '>')
    {
        return Err("invalid_character");
    }
    Ok(())
}

pub fn inspect_request(root: &str, scope: IdentityScope) -> serde_json::Value {
    serde_json::json!({ "root": root, "scope": scope.id() })
}

pub fn configure_request(
    root: &str,
    scope: IdentityScope,
    field: IdentityField,
    value: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "root": root,
        "scope": scope.id(),
        "key": field.id(),
        "value": value,
    })
}

#[allow(dead_code)]
pub fn parse_setup(data: &serde_json::Value) -> IdentitySetup {
    let text = |key: &str| {
        data.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };
    IdentitySetup {
        is_repository: data
            .get("isRepository")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        has_commits: data
            .get("hasCommits")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        branch: text("branch"),
        scope: match text("scope").as_deref() {
            Some("global") => IdentityScope::Global,
            _ => IdentityScope::Local,
        },
        configured_name: text("configuredName"),
        configured_email: text("configuredEmail"),
        effective_name: text("effectiveName"),
        effective_email: text("effectiveEmail"),
    }
}

pub fn inspect_identity(root: &str, scope: IdentityScope) -> Result<String, String> {
    match execute_core("git.repositorySetup", inspect_request(root, scope)) {
        Ok(data) => Ok(data.to_string()),
        Err(code) => Err(redact(&code, root)),
    }
}

pub fn configure_identity(
    root: &str,
    scope: IdentityScope,
    field: IdentityField,
    value: Option<&str>,
) -> Result<String, String> {
    if let Some(value) = value {
        if let Err(reason) = validate_value(value) {
            return Err(reason.to_string());
        }
    }
    let payload = configure_request(root, scope, field, value);
    match execute_core("git.configureIdentity", payload) {
        Ok(data) => Ok(data.to_string()),
        Err(code) => Err(redact(&code, root)),
    }
}

fn redact(message: &str, root: &str) -> String {
    if root.is_empty() {
        return message.to_string();
    }
    let forward = root.replace('\\', "/");
    let backward = root.replace('/', "\\");
    let text = message
        .replace(root, "")
        .replace(&forward, "")
        .replace(&backward, "")
        .trim()
        .to_string();
    if text.is_empty() {
        "git identity failed".to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn requests_use_the_core_key_names() {
        assert_eq!(
            inspect_request("C:/ws", IdentityScope::Global),
            json!({ "root": "C:/ws", "scope": "global" })
        );

        let payload = configure_request(
            "C:/ws",
            IdentityScope::Local,
            IdentityField::Email,
            Some("a@b.c"),
        );
        assert_eq!(
            payload,
            json!({ "root": "C:/ws", "scope": "local", "key": "email", "value": "a@b.c" })
        );

        let payload = configure_request("C:/ws", IdentityScope::Local, IdentityField::Name, None);
        assert_eq!(payload.get("value"), Some(&serde_json::Value::Null));
    }

    #[test]
    fn scope_and_field_ids_match_core() {
        assert_eq!(IdentityScope::Local.id(), "local");
        assert_eq!(IdentityScope::Global.id(), "global");
        assert_eq!(IdentityField::Name.id(), "name");
        assert_eq!(IdentityField::Email.id(), "email");
        assert_eq!(
            IdentityScope::ALL,
            [IdentityScope::Local, IdentityScope::Global]
        );
        assert_eq!(IdentityScope::default(), IdentityScope::Local);
    }

    #[test]
    fn response_parses_every_field() {
        let setup = parse_setup(&json!({
            "isRepository": true,
            "hasCommits": false,
            "branch": "main",
            "scope": "global",
            "configuredName": "Lithe Dev",
            "configuredEmail": "dev@example.com",
            "effectiveName": "Inherited",
            "effectiveEmail": "inherited@example.com"
        }));
        assert!(setup.is_repository);
        assert!(!setup.has_commits);
        assert_eq!(setup.branch.as_deref(), Some("main"));
        assert_eq!(setup.scope, IdentityScope::Global);
        assert_eq!(setup.configured(IdentityField::Name), Some("Lithe Dev"));
        assert_eq!(
            setup.effective(IdentityField::Email),
            Some("inherited@example.com")
        );
    }

    #[test]
    fn non_repository_is_a_normal_state() {
        let setup = parse_setup(&json!({
            "isRepository": false,
            "hasCommits": false,
            "branch": null,
            "scope": "local",
            "configuredName": null,
            "configuredEmail": null,
            "effectiveName": "Machine User",
            "effectiveEmail": null
        }));
        assert!(!setup.is_repository);
        assert_eq!(setup.branch, None);
        assert_eq!(setup.configured(IdentityField::Name), None);
        assert_eq!(setup.effective(IdentityField::Name), Some("Machine User"));
        assert!(
            !setup.editable(IdentityScope::Local),
            "非仓库 + local 必须禁用"
        );
        assert!(
            setup.editable(IdentityScope::Global),
            "global 不需要仓库上下文"
        );
    }

    #[test]
    fn missing_fields_fall_back_per_field() {
        let setup = parse_setup(&json!({ "isRepository": "yes" }));
        assert_eq!(setup, IdentitySetup::default());
        assert!(!setup.editable(IdentityScope::Local));
    }

    #[test]
    fn value_validation_matches_core() {
        assert!(validate_value("Lithe Dev").is_ok());
        assert_eq!(validate_value("   "), Err("empty"));
        assert_eq!(validate_value("a<b"), Err("invalid_character"));
        assert_eq!(validate_value("a>b"), Err("invalid_character"));
        assert_eq!(validate_value("a\nb"), Err("invalid_character"));
        assert_eq!(
            validate_value(&"汉".repeat(342)),
            Err("too_long"),
            "1026 字节 > 1024"
        );
        assert!(validate_value(&"x".repeat(IDENTITY_MAX_BYTES)).is_ok());
    }

    #[test]
    fn paths_are_redacted_from_failure_text() {
        let text = redact(
            "process_failed: fatal: not a git repository (C:\\ws\\demo)",
            "C:\\ws\\demo",
        );
        assert!(!text.contains("C:\\ws\\demo"), "{text}");
        assert!(text.contains("process_failed"), "{text}");
        assert!(text.contains("not a git repository"), "{text}");
    }

    #[test]
    fn invalid_values_never_reach_core() {
        let error = configure_identity(
            "C:/definitely/not/a/workspace",
            IdentityScope::Global,
            IdentityField::Name,
            Some("   "),
        )
        .expect_err("空值必须被拒");
        assert_eq!(error, "empty");

        let error = configure_identity(
            "C:/definitely/not/a/workspace",
            IdentityScope::Global,
            IdentityField::Name,
            Some("a<b"),
        )
        .expect_err("尖括号必须被拒");
        assert_eq!(error, "invalid_character");
    }
}
