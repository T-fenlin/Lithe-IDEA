//! 提交身份（`user.name` / `user.email`）的**数据层**：读写 Git 的 local / global 作用域。
//!
//! 规格真源：`shared/contracts/git-repository-setup.md`（契约）+ `rust-core-api.md:169,171`
//! + Windows 前端 `windows/tauri/src/features/git/api/git-setup-api.ts` 与
//! `features/settings/components/git-identity-settings.tsx`。
//!
//! 本模块不渲染任何东西，也不认识 GPUI 的设置 crate：它是"宿主钩子"里
//! **宿主那一侧的实现**（接口定义在 `lithe-gpui-settings`，登记在 `ShellWorkspace::new`；
//! 依赖方向见 `crate::lib.rs` 与 `settings/src/dialog.rs` 的模块文档）。
//!
//! # 数据来源（逐条对契约）
//!
//! | 用途 | 命令 | payload | 响应 |
//! | --- | --- | --- | --- |
//! | 读当前仓库 / 作用域下的身份 | `git.repositorySetup` | `{ root, scope: "local" \| "global" }` | [`IdentitySetup`] |
//! | 写一个字段 | `git.configureIdentity` | `{ root, scope, key: "name" \| "email", value }` | 同上（写完即回读） |
//! | 清一个字段的覆盖 | `git.configureIdentity` | 同上，`value: null` | 同上 |
//!
//! `scope` 缺省是 `local`（`git/setup.rs:35-36` 的 `#[serde(default)]`），本侧恒显式传。
//!
//! # 三条硬约束（都在 Core 源码里核对过）
//!
//! 1. **`root` 必须是"存在的目录"，不是仓库根也行**：`validate_root` 只校验目录存在
//!    （`git/setup.rs:156`）。非仓库时 `inspect` 照常返回 `ok:true` + `isRepository:false`
//!    （`:177-195`），所以「不是仓库」**不是错误** —— 它与 `git.status` 的
//!    `repositoryRoot: null` 是同一类语义（见 `changes.rs` 硬约束 2）。
//! 2. **`local` 写入要求是仓库**：`configure_identity` 在非仓库 + `local` 时返回
//!    `Err(invalid_request)`（`git/setup.rs:241-246`）。所以界面在非仓库 + `local` 时
//!    必须**禁用**保存（真源同样禁，`git-identity-settings.tsx:129,136`），而不是等 Core 报错。
//! 3. **`global` 不需要是仓库**：`can_read_scope = is_repository || scope == Global`
//!    （`git/setup.rs:177`），写入路径也只对 `Local` 做仓库校验。
//!
//! # 校验与真源的分工
//!
//! Core 会拒绝空串 / 超过 1024 字节 / 含控制字符或尖括号的值（`git/setup.rs:224-236`）。
//! 本侧在**调用之前**再拦一遍（[`validate_value`]），理由有两条：
//!
//! 1. 真源的保存按钮在空值时就是禁用的（`git-identity-settings.tsx:137` 的 `!value.trim()`），
//!    本侧要在**同一个判据**上禁用按钮，所以这段校验是界面逻辑的输入，不只是防御；
//! 2. 让"点了保存却什么都没发生"不可能出现：界面上可点的按钮一定过得去 Core 的校验。
//!    两边的口径必须逐字一致（同样的 1024 字节、同样的控制字符与尖括号），否则会出现
//!    "按钮可点但 Core 必定报错"的死角。

use crate::model::execute_core;

/// 身份写入的作用域。字符串取值就是 Core 的 `scope` 枚举值
/// （`git/setup.rs:13-19` 的 `#[serde(rename_all = "camelCase")]` → `"local"` / `"global"`）。
///
/// 默认是 `Local`：Core 的 `GitSetupRequest` 给 `scope` 加了 `#[serde(default)]`
/// （`git/setup.rs:35-36`），本侧与那条缺省口径保持一致。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IdentityScope {
    /// 当前仓库（`--local`）。
    #[default]
    Local,
    /// 全局 Git 配置（`--global`）。
    Global,
}

impl IdentityScope {
    /// Core 的 `scope` 字面量。
    pub fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
        }
    }

    /// 真源下拉的两个取值（`git-identity-settings.tsx:98-99` 的 `local` / `global`）。
    pub const ALL: [IdentityScope; 2] = [IdentityScope::Local, IdentityScope::Global];
}

/// 要写哪一个字段。字符串取值就是 Core 的 `key` 枚举值
/// （`git/setup.rs:40-45` 的 `#[serde(rename_all = "camelCase")]` → `"name"` / `"email"`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityField {
    Name,
    Email,
}

impl IdentityField {
    /// Core 的 `key` 字面量。
    pub fn id(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
        }
    }
}

/// 一次读 / 写的结果（Core `GitSetupResponse`，`git/setup.rs:68-79`）。
///
/// `configured_*` 是**所选作用域里直接写着的值**（不含 include），
/// `effective_*` 是 Git 正常继承解析出来的**当前生效值**。界面上两者都要显示：
/// 真源把前者填进输入框、把后者画在下面那行「当前生效值 / 尚未配置有效值」
/// （`git-identity-settings.tsx:47-48,153-157`）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdentitySetup {
    /// 这个目录是不是 Git 工作树（`rev-parse --is-inside-work-tree`）。
    pub is_repository: bool,
    /// 有没有提交（unborn HEAD 不算失败）。
    pub has_commits: bool,
    /// 当前分支短名；非仓库或 unborn 时为 `None`。
    pub branch: Option<String>,
    /// 回显的作用域（Core 原样返回请求里的 `scope`）。
    pub scope: IdentityScope,
    /// 所选作用域里写着的 `user.name`。
    pub configured_name: Option<String>,
    /// 所选作用域里写着的 `user.email`。
    pub configured_email: Option<String>,
    /// 继承解析后**当前生效**的 `user.name`。
    pub effective_name: Option<String>,
    /// 继承解析后**当前生效**的 `user.email`。
    pub effective_email: Option<String>,
}

impl IdentitySetup {
    /// 所选作用域里该字段写着的值（填输入框用）。
    pub fn configured(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Name => self.configured_name.as_deref(),
            IdentityField::Email => self.configured_email.as_deref(),
        }
    }

    /// 该字段当前生效的值（画「当前生效值」那一行用）。
    pub fn effective(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Name => self.effective_name.as_deref(),
            IdentityField::Email => self.effective_email.as_deref(),
        }
    }

    /// `local` 作用域下能不能编辑：真源判据是 `scope === "local" && !state.isRepository`
    /// （`git-identity-settings.tsx:111,129,136`）。`global` 恒可编辑。
    pub fn editable(&self, scope: IdentityScope) -> bool {
        scope == IdentityScope::Global || self.is_repository
    }
}

/// 值里最长的 UTF-8 字节数。逐字照 Core 的 `MAX_IDENTITY_BYTES`
/// （`git/setup.rs:10`）——**字节数不是字符数**，中文一个字符 3 字节。
pub const IDENTITY_MAX_BYTES: usize = 1024;

/// 值能不能被 Core 接受；不能时返回一句**用户看得懂**的原因。
///
/// Core 那边的原文是 `"Enter a nonempty Git identity without control characters or
/// angle brackets"`（`git/setup.rs:233`）。本侧不照抄英文原文（界面不许出现英文字面量），
/// 也不新造 locale 键：把这条判据用在**禁用保存按钮**上，所以界面上根本不会出现这句话，
/// 它只出现在诊断行里（`S1_GIT_IDENTITY ... result=invalid`）。
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

/// `git.repositorySetup` 的请求体（纯函数，单测钉住字段名与取值）。
pub fn inspect_request(root: &str, scope: IdentityScope) -> serde_json::Value {
    serde_json::json!({ "root": root, "scope": scope.id() })
}

/// `git.configureIdentity` 的请求体。`value: None` = **清除该作用域的覆盖**
/// （`git/setup.rs:56-64` 的 `Option<String>`，`None` 序列化成 JSON `null`）。
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

/// 解析 `GitSetupResponse`。
///
/// 缺字段**不报错**：Core 的响应字段都是必填的（`git/setup.rs:68-79` 没有 `Option` 包裹
/// 的 `#[serde(default)]`），但本侧按既有口径逐字段容错（同 `changes.rs::parse_changes`），
/// 免得将来 Core 加字段/改形状时整个页面变成错误态。
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

/// 读一次身份快照，成功时返回 **Core 响应原文**（JSON 文本）。
///
/// 返回文本而不是 [`IdentitySetup`]：调用方是设置页的宿主钩子，钩子的签名里
/// **不能出现本 crate 的类型**（否则 `lithe-gpui-settings` 就得反向依赖 `lithe-gpui-git`，
/// 见 `settings/src/identity.rs` 的模块文档）。文本是 `Send` 的，也不会把
/// `gpui::SharedString` 那种非 `Send` 类型牵进 `background_spawn`。
///
/// **在后台线程上调用**（`lithe_core::execute_json` 是同步的）。
///
/// 失败返回一句可直接上屏的文本：Core 的 `code: message`（与 `changes.rs::write` 的
/// [`crate::changes::WriteOutcome::failure`] 同一口径），并把仓库根绝对路径剪掉
/// （Git 的 stderr 常带绝对路径；真源同样刻意不把原始 stderr 摊给用户）。
pub fn inspect_identity(root: &str, scope: IdentityScope) -> Result<String, String> {
    match execute_core("git.repositorySetup", inspect_request(root, scope)) {
        Ok(data) => Ok(data.to_string()),
        Err(code) => Err(redact(&code, root)),
    }
}

/// 写一个字段（`value: Some`）或清除该字段的覆盖（`value: None`），
/// 成功时返回**写完后的 Core 响应原文**（同 [`inspect_identity`] 的理由）。
///
/// 写前先跑 [`validate_value`]：空值 / 超长 / 非法字符**不发请求**，直接返回失败原因
/// （口径见模块文档的「校验与真源的分工」）。
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

/// 剪掉消息里的仓库根绝对路径（与 `changes.rs::redact` 同一口径与理由）。
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

    /// 请求体的**键名与取值**是 Core 的兼容面：`scope` / `key` 用小写枚举值，
    /// `value` 清除时必须是 JSON `null`（不是缺字段、也不是空串 —— 空串会被 Core 拒绝）。
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

        // 清除覆盖 = 显式 `null`。
        let payload =
            configure_request("C:/ws", IdentityScope::Local, IdentityField::Name, None);
        assert_eq!(payload.get("value"), Some(&serde_json::Value::Null));
    }

    /// 两个作用域与两个字段的字面量逐字对齐 Core 的 `#[serde(rename_all = "camelCase")]` 枚举。
    #[test]
    fn scope_and_field_ids_match_core() {
        assert_eq!(IdentityScope::Local.id(), "local");
        assert_eq!(IdentityScope::Global.id(), "global");
        assert_eq!(IdentityField::Name.id(), "name");
        assert_eq!(IdentityField::Email.id(), "email");
        assert_eq!(IdentityScope::ALL, [IdentityScope::Local, IdentityScope::Global]);
        assert_eq!(IdentityScope::default(), IdentityScope::Local);
    }

    /// 响应解析：八个字段一一对应；`scope` 回显成枚举。
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
        assert_eq!(setup.effective(IdentityField::Email), Some("inherited@example.com"));
    }

    /// **非仓库不是错误**：`isRepository:false` + 两个 `configured*` 为 null 照常解析，
    /// 且此时 `local` 不可编辑、`global` 仍可编辑（硬约束 1 / 2 / 3）。
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
        assert!(!setup.editable(IdentityScope::Local), "非仓库 + local 必须禁用");
        assert!(setup.editable(IdentityScope::Global), "global 不需要仓库上下文");
    }

    /// 缺字段 / 类型坏掉时**不 panic、不整体失败**，回落到各自的默认值。
    #[test]
    fn missing_fields_fall_back_per_field() {
        let setup = parse_setup(&json!({ "isRepository": "yes" }));
        assert_eq!(setup, IdentitySetup::default());
        assert!(!setup.editable(IdentityScope::Local));
    }

    /// 值的校验与 Core 的 `validate` 逐条对齐（`git/setup.rs:224-236`）。
    ///
    /// ⚠️ 长度判据是**字节**：`"汉".repeat(342)` 是 1026 字节，必须被拒。
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
        // 1024 字节正好合法（Core 用的是 `>`，不是 `>=`）。
        assert!(validate_value(&"x".repeat(IDENTITY_MAX_BYTES)).is_ok());
    }

    /// 错误文本要剪掉仓库根绝对路径（Git 的 stderr 常带它）。
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

    /// **非法值不发请求**：`configure_identity` 在值过不了校验时直接返回原因码，
    /// 连 Core 都不会被调（所以这条测试不需要真的 git 仓库就能跑）。
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
