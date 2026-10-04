use std::path::PathBuf;
use std::rc::Rc;

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

    pub fn from_id(id: &str) -> Self {
        match id {
            "global" => Self::Global,
            _ => Self::Local,
        }
    }
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

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Name => "lithe.git.setup.name",
            Self::Email => "lithe.git.setup.email",
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::Name => 0,
            Self::Email => 1,
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "email" => Self::Email,
            _ => Self::Name,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IdentitySetup {
    pub is_repository: bool,
    pub has_commits: bool,
    pub branch: Option<String>,
    pub configured_name: Option<String>,
    pub configured_email: Option<String>,
    pub effective_name: Option<String>,
    pub effective_email: Option<String>,
}

impl IdentitySetup {
    pub fn from_json(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

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

    pub fn editable_in(&self, scope: IdentityScope) -> bool {
        scope == IdentityScope::Global || self.is_repository
    }

    pub fn can_save(&self, scope: IdentityScope, field: IdentityField, draft: &str) -> bool {
        self.can_save_reason(scope, field, draft).is_none()
    }

    pub fn can_save_reason(
        &self,
        scope: IdentityScope,
        field: IdentityField,
        draft: &str,
    ) -> Option<&'static str> {
        if !self.editable_in(scope) {
            return Some("not_a_repository");
        }
        if let Some(reason) = identity_value_rejection(draft) {
            return Some(reason);
        }
        if draft.trim() == self.configured(field).unwrap_or("").trim() {
            return Some("unchanged");
        }
        None
    }

    pub fn can_clear(&self, field: IdentityField) -> bool {
        self.configured(field).is_some()
    }
}

pub const IDENTITY_MAX_BYTES: usize = 1024;

pub fn identity_value_is_valid(value: &str) -> bool {
    identity_value_rejection(value).is_none()
}

pub fn identity_value_rejection(value: &str) -> Option<&'static str> {
    if value.trim().is_empty() {
        return Some("empty");
    }
    if value.len() > IDENTITY_MAX_BYTES {
        return Some("too_long");
    }
    if value
        .chars()
        .any(|character| character.is_control() || character == '<' || character == '>')
    {
        return Some("invalid_character");
    }
    None
}

pub struct GitIdentityHost {
    pub workspace_root: PathBuf,
    pub load: Box<
        dyn Fn(
            &str,
            Box<dyn FnOnce(Option<String>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static>,
            &mut gpui_kit::AsyncWindowContext,
        ),
    >,
    pub save: Box<
        dyn Fn(
            &str,
            &str,
            Option<String>,
            Box<dyn FnOnce(Option<String>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static>,
            &mut gpui_kit::AsyncWindowContext,
        ),
    >,
}

thread_local! {
    static HOST: std::cell::RefCell<Option<Rc<GitIdentityHost>>> = const { std::cell::RefCell::new(None) };
}

pub fn set_git_identity_host(host: Option<GitIdentityHost>) {
    HOST.with(|slot| *slot.borrow_mut() = host.map(Rc::new));
}

#[derive(Clone)]
pub struct GitIdentityPage {
    pub workspace_root: PathBuf,
    host: Rc<GitIdentityHost>,
}

pub fn host_workspace_root() -> Option<PathBuf> {
    git_identity_page().map(|page| page.workspace_root)
}

pub fn git_identity_page() -> Option<GitIdentityPage> {
    HOST.with(|slot| {
        slot.borrow().as_ref().map(|host| GitIdentityPage {
            workspace_root: host.workspace_root.clone(),
            host: host.clone(),
        })
    })
}

impl GitIdentityPage {
    pub fn load<T: 'static>(
        &self,
        scope: IdentityScope,
        deliver: impl FnOnce(Option<IdentitySetup>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<T>,
    ) {
        let scope_id = scope.id().to_string();
        let host = self.host.clone();
        cx.spawn_in(&*window, async move |_view, async_cx| {
            (host.load)(
                &scope_id,
                Box::new(move |json, window, cx| {
                    deliver(
                        json.and_then(|text| IdentitySetup::from_json(&text)),
                        window,
                        cx,
                    );
                }),
                async_cx,
            );
        })
        .detach();
    }

    pub fn save<T: 'static>(
        &self,
        scope: IdentityScope,
        field: IdentityField,
        value: Option<String>,
        deliver: impl FnOnce(Option<IdentitySetup>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<T>,
    ) {
        let host = self.host.clone();
        cx.spawn_in(&*window, async move |_view, async_cx| {
            (host.save)(
                scope.id(),
                field.id(),
                value,
                Box::new(move |json, window, cx| {
                    deliver(
                        json.and_then(|text| IdentitySetup::from_json(&text)),
                        window,
                        cx,
                    );
                }),
                async_cx,
            );
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_is_absent_until_a_host_registers_it() {
        set_git_identity_host(None);
        assert!(
            git_identity_page().is_none(),
            "没有宿主登记时必须返回 None（Git 页据此画明确空态）"
        );
        assert!(
            host_workspace_root().is_none(),
            "没有宿主登记时工作区根必须是 None"
        );
    }

    #[test]
    fn scope_and_field_ids_match_core() {
        assert_eq!(IdentityScope::Local.id(), "local");
        assert_eq!(IdentityScope::Global.id(), "global");
        assert_eq!(IdentityField::Name.id(), "name");
        assert_eq!(IdentityField::Email.id(), "email");
        assert_eq!(IdentityScope::default(), IdentityScope::Local);
        assert_eq!(
            IdentityScope::ALL,
            [IdentityScope::Local, IdentityScope::Global]
        );
        assert_eq!(IdentityField::Name.label_key(), "lithe.git.setup.name");
        assert_eq!(IdentityField::Email.label_key(), "lithe.git.setup.email");
    }

    #[test]
    fn response_parses_every_field() {
        let setup = IdentitySetup::from_json(
            r#"{"isRepository":true,"hasCommits":false,"branch":"main","scope":"global",
                "configuredName":"Lithe Dev","configuredEmail":"dev@example.com",
                "effectiveName":"Inherited","effectiveEmail":"inherited@example.com"}"#,
        )
        .expect("合法响应必须能解析");
        assert!(setup.is_repository);
        assert!(!setup.has_commits);
        assert_eq!(setup.branch.as_deref(), Some("main"));
        assert_eq!(setup.configured(IdentityField::Name), Some("Lithe Dev"));
        assert_eq!(
            setup.effective(IdentityField::Email),
            Some("inherited@example.com")
        );
        assert_eq!(
            setup.configured(IdentityField::Email),
            Some("dev@example.com")
        );
    }

    #[test]
    fn malformed_responses_never_panic() {
        let setup = IdentitySetup::from_json(r#"{"isRepository":true}"#).expect("缺字段要能解析");
        assert!(setup.is_repository);
        assert_eq!(setup.branch, None);
        assert!(IdentitySetup::from_json("not json").is_none());
        assert!(IdentitySetup::from_json("null").is_none());
    }

    #[test]
    fn local_scope_requires_a_repository() {
        let setup = IdentitySetup::default();
        assert!(!setup.editable_in(IdentityScope::Local));
        assert!(setup.editable_in(IdentityScope::Global));

        let repository = IdentitySetup {
            is_repository: true,
            ..IdentitySetup::default()
        };
        assert!(repository.editable_in(IdentityScope::Local));
    }

    #[test]
    fn save_is_enabled_only_for_a_new_valid_value() {
        let setup = IdentitySetup {
            is_repository: true,
            configured_name: Some("Lithe Dev".to_string()),
            ..IdentitySetup::default()
        };
        assert!(setup.can_save(IdentityScope::Local, IdentityField::Name, "Other"));
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "Lithe Dev"));
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "  Lithe Dev  "));
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "   "));
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "a<b"));
        let no_repository = IdentitySetup::default();
        assert!(!no_repository.can_save(IdentityScope::Local, IdentityField::Name, "Other"));
        assert!(no_repository.can_save(IdentityScope::Global, IdentityField::Name, "Other"));
    }

    #[test]
    fn clear_requires_a_configured_value() {
        let setup = IdentitySetup {
            configured_name: Some("Lithe Dev".to_string()),
            ..IdentitySetup::default()
        };
        assert!(setup.can_clear(IdentityField::Name));
        assert!(!setup.can_clear(IdentityField::Email));
    }

    #[test]
    fn value_validation_matches_core() {
        assert!(identity_value_is_valid("Lithe Dev"));
        assert!(identity_value_is_valid("dev@example.com"));
        assert!(!identity_value_is_valid("   "));
        assert!(!identity_value_is_valid("a<b"));
        assert!(!identity_value_is_valid("a>b"));
        assert!(!identity_value_is_valid("a\nb"));
        assert!(!identity_value_is_valid(&"汉".repeat(342)));
        assert!(identity_value_is_valid(&"x".repeat(IDENTITY_MAX_BYTES)));
        assert_eq!(identity_value_rejection("  "), Some("empty"));
        assert_eq!(identity_value_rejection("a<b"), Some("invalid_character"));
        assert_eq!(
            identity_value_rejection(&"汉".repeat(342)),
            Some("too_long")
        );
        assert_eq!(identity_value_rejection("ok"), None);
    }
}
