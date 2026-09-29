//! 「设置 → Git」页的**数据边界**：宿主钩子（[`GitIdentityHost`]）与它那几张纯数据表。
//!
//! ## 为什么需要宿主钩子
//!
//! 设置页要读写 Git 的提交身份（`user.name` / `user.email`），而 Core 的两条命令
//! （`git.repositorySetup` / `git.configureIdentity`）落在 `lithe-gpui-git`。
//! 依赖方向不允许 `settings → git`（那会成环：`workbench` 同时依赖两者，见
//! `crate::lib.rs` 的模块文档），也不能让设置 crate 自己去拼 Core 信封——那等于把
//! 同一件事做成两个真相源。
//!
//! 所以照仓库里**已有的同类口径**做：`lithe-gpui/crates/editor/src/editor_view.rs` 的
//! `TabMenuHostActions`（编辑区定义两个"只有外壳做得了"的回调，由 `ShellWorkspace::new`
//! 登记进去）。本模块是同一形状：
//!
//! ```text
//! settings  ──定义──▶ GitIdentityHost{ 工作区根, load, save }  ──登记──▶ Global(SettingsHook)
//!    ▲                                                                    ▲
//!    └──────────────── workbench::ShellWorkspace::new 用 lithe-gpui-git 的实现填进去 ──┘
//! ```
//!
//! **钩子没登记时**（测试宿主、其它宿主、或登记之前的任何时刻）：
//! [`git_identity_page_state`] 返回 `None`，Git 页退回**明确空态**
//! （现在的 `settings.gpui.prerequisiteGit` 那一页）。**不 panic、也不假装成功**。
//!
//! ## 为什么回调收/发 JSON 而不是 `lithe_gpui_git` 的类型
//!
//! 本 crate **不认识** `lithe-gpui-git`（那是依赖方向的红线）。若钩子的签名里出现
//! `lithe_gpui_git::IdentitySetup`，`workbench` 就必须把 git 的类型喂给 settings，
//! 而 settings 为了写出那个类型又得反向依赖 git —— 钩子就成了摆设。
//! 于是边界定在**线格式**上（键名逐字取自 Core 的 `GitSetupResponse`，
//! `rust/lithe-core/src/git/setup.rs:68-79`）：
//!
//! - `load(scope: &str) -> Task<()>`：scope 取 `"local"` / `"global"`；任务结束时调
//!   [`GitIdentityPage::deliver_load`]，`Some(json)` = Core 的响应原文，`None` = 失败
//!   （本侧的约定：宿主在这里已经打过 `S1_GIT_IDENTITY` 诊断，设置页只画"读失败"）。
//! - `save(scope, key, value) -> Task<()>`：`key` 取 `"name"` / `"email"`；
//!   `value: None` = **清除覆盖**（Core 的 `value: null`）；任务结束时调
//!   [`GitIdentityPage::deliver_save`]。
//!
//! 取 JSON 的**字符串**而不是 `serde_json::Value`：线格式跨 crate 边界时用文本，
//! 宿主与设置两侧可以各自独立单测自己的解析。
//!
//! ## 回调收 `AsyncWindowContext`，回写走 `WeakEntity::update`
//!
//! 宿主做三件事：**在 `cx.background_spawn` 上跑 Core 命令** → 把结果变成 JSON 文本 →
//! 用 `async_cx.update(move |window, cx| delegate(..))` 回前台把结果交出去。
//!
//! delegate 的签名是 `FnOnce(Option<IdentitySetup>, &mut Window, &mut App) + 'static`：
//! 设置侧自己那一层（[`GitIdentityPage::load`] / [`GitIdentityPage::save`]）用
//! `WeakEntity::update` 把 `App` 换成 `Context<SettingsDialog>` 再回写，
//! 窗口一路传下去只为一件事——`InputState::set_value` 要它。
//!
//! ⚠️ **设置侧不能自己起任务**：`Context::spawn_in` 的异步体拿到的是
//! `AsyncWindowContext`，`Context::spawn` 拿到的是 `AsyncApp`（只有 `update(&mut App)`，
//! 拿不到窗口）。所以"起任务"这件事交给宿主，设置侧把窗口与上下文一起交给它
//! （见 `GitIdentityPage::load` 的 `cx.spawn_in(..)` 包装）。
//!
//! 结果仍然经过**代次校验**：切换作用域 / 重新加载会让旧请求的结果作废
//! （真源 `git-identity-settings.tsx:34-61` 的 `generation` + `currentContext` 同一条口径）——
//! 那一层在**设置侧**（`SettingsDialog::git` 的 `generation`），宿主不参与。
//!
//! ## 为什么用线程局部而不是 `App::set_global`
//!
//! 登记发生在 `ShellWorkspace::new`（那时有 `App`）没问题，但**读取**发生在
//! `SettingsDialog::render`，而 `render` 只有 `&mut Context<Self>`——它**借不到 `App`**
//! 去查 `Global`（gpui 的 `Context` 不暴露 `try_global`，只有 `App` 有）。
//! 线程局部（[`std::cell::Cell`]）绕开这个死结，且**不引入任何异步**：
//! 这两个钩子只在 UI 线程读写（登记一次、渲染期读），值一旦登记就不再变
//! （工作区根在进程生命周期里是常量）。
//!
//! 代价是测试之间共享同一个线程局部的状态。本模块的测试因此只断言
//! **"没有登记时"**那一支（`None` → 空态），以及纯数据的解析与判据；
//! 需要"登记之后"的断言由 `lithe-gpui-workbench` 的端到端验证覆盖。

use std::path::PathBuf;
use std::rc::Rc;

/// 提交身份的作用域。取值就是 Core 的 `scope` 字面量
/// （`rust/lithe-core/src/git/setup.rs:13-19`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IdentityScope {
    /// 当前仓库（真源 `git.setup.local`）。
    #[default]
    Local,
    /// 全局 Git 配置（真源 `git.setup.global`）。
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

    /// 下拉顺序（真源 `git-identity-settings.tsx:98-99`：先 local 后 global）。
    pub const ALL: [IdentityScope; 2] = [IdentityScope::Local, IdentityScope::Global];

    /// 从宿主钩子收到的作用域字面量解析回来（不是这两个取值的都当 `local`：
    /// Core 的缺省作用域就是 `local`，见 `rust/lithe-core/src/git/setup.rs:35-36`）。
    pub fn from_id(id: &str) -> Self {
        match id {
            "global" => Self::Global,
            _ => Self::Local,
        }
    }
}

/// 要写哪一个字段。取值就是 Core 的 `key` 字面量。
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

    /// 界面上的标签文案键（真源 `git.setup.name` / `git.setup.email`）。
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Name => "lithe.git.setup.name",
            Self::Email => "lithe.git.setup.email",
        }
    }

    /// 界面元素 id 的**下标**。**不能把 `id()` 拼进要素元组**：`ElementId` 只实现了
    /// `From<(&'static str, u32)>` / `(&'static str, u64)` / `(&'static str, EntityId)`
    /// （`gpui-pre-0.3.6/src/elements/div.rs:779` 那组），`&str` 拼不进去。
    pub fn index(self) -> u32 {
        match self {
            Self::Name => 0,
            Self::Email => 1,
        }
    }

    /// 从宿主钩子收到的字段字面量解析回来（不是这两个取值的都当 `name`：
    /// 钩子的调用点只有这两个取值，解析失败时退回第一个比 panic 更安全）。
    pub fn from_id(id: &str) -> Self {
        match id {
            "email" => Self::Email,
            _ => Self::Name,
        }
    }
}

/// 一次读 / 写的结果（Core `GitSetupResponse`，`rust/lithe-core/src/git/setup.rs:68-79`）。
///
/// 键名与 Core 逐字相同（camelCase），所以本结构可以从宿主回的 JSON 直接反序列化。
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IdentitySetup {
    /// 这个目录是不是 Git 工作树。
    pub is_repository: bool,
    /// 有没有提交（unborn HEAD 不算失败）。
    pub has_commits: bool,
    /// 当前分支短名；非仓库或 unborn 时为 `None`。
    pub branch: Option<String>,
    /// 所选作用域里直接写着的 `user.name`（**不含** include；填输入框用它）。
    pub configured_name: Option<String>,
    /// 所选作用域里直接写着的 `user.email`。
    pub configured_email: Option<String>,
    /// 继承解析后**当前生效**的 `user.name`。
    pub effective_name: Option<String>,
    /// 继承解析后**当前生效**的 `user.email`。
    pub effective_email: Option<String>,
}

impl IdentitySetup {
    /// 兼容旧宿主的解析入口：`scope` 字段在 JSON 里也存在，但界面用的是**本地**的作用域
    /// （用户刚点的那一个），所以这里刻意不读它——Core 只是原样回显请求里的值。
    pub fn from_json(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

    /// 所选作用域里该字段写着的值。
    pub fn configured(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Name => self.configured_name.as_deref(),
            IdentityField::Email => self.configured_email.as_deref(),
        }
    }

    /// 该字段当前生效的值。
    pub fn effective(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Name => self.effective_name.as_deref(),
            IdentityField::Email => self.effective_email.as_deref(),
        }
    }

    /// `local` 作用域下能不能编辑。判据逐字照真源
    /// `git-identity-settings.tsx:111,129,136` 的 `scope === "local" && !state.isRepository`：
    /// `global` 恒可编辑（Core 的 `can_read_scope` 也是这么判的，
    /// `rust/lithe-core/src/git/setup.rs:177`）。
    pub fn editable_in(&self, scope: IdentityScope) -> bool {
        scope == IdentityScope::Global || self.is_repository
    }

    /// 「保存」按钮可不可点：可编辑 + 值被 Core 的校验接受 + **与已保存的值不同**
    /// （真源 `git-identity-settings.tsx:133-139` 的四个禁用条件）。
    pub fn can_save(&self, scope: IdentityScope, field: IdentityField, draft: &str) -> bool {
        self.can_save_reason(scope, field, draft).is_none()
    }

    /// 为什么不能保存；`None` = 可以保存。**只有这一处判据**：
    /// 界面用 `can_save`（= `is_none()`），诊断行用这里的取值
    /// （`S1_GIT_IDENTITY ... result=blocked reason=…`），两者不会漂移。
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

    /// 「清除覆盖」按钮可不可点：该作用域里**确实写着**一个值
    /// （真源 `git-identity-settings.tsx:147` 的 `configured == null`）。
    pub fn can_clear(&self, field: IdentityField) -> bool {
        self.configured(field).is_some()
    }
}

/// 值的合法上限（**UTF-8 字节数**）。逐字照 Core 的 `MAX_IDENTITY_BYTES`
/// （`rust/lithe-core/src/git/setup.rs:10`）。
pub const IDENTITY_MAX_BYTES: usize = 1024;

/// 值与 Core 的校验逐条对齐（`rust/lithe-core/src/git/setup.rs:224-236`）：
/// 非空（trim 后）、不超过 1024 字节、不含控制字符与尖括号。
///
/// 界面用它**禁用保存按钮**，所以这段逻辑不能比 Core 更宽——否则会出现"按钮可点、
/// 点下去必定报错"的死角；也不能更严——否则用户明明能写的值会被挡在门外。
pub fn identity_value_is_valid(value: &str) -> bool {
    identity_value_rejection(value).is_none()
}

/// 值与 Core 的校验不符时的**稳定原因码**；`None` = 合法。
///
/// 原因码只进诊断行（`S1_GIT_IDENTITY ... reason=too_long`）与断言，
/// **不上界面**：界面上就是"保存按钮禁用"这一个事实，解释它需要新 locale 键，
/// 而真源在这一处也没有给用户任何解释（`git-identity-settings.tsx:133-139` 只是禁用）。
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

/// 一次读 / 写要让宿主做的两件事（见模块文档的「为什么回调收/发 JSON」）。
///
/// 两个回调**只负责启动**工作、不返回句柄：宿主内部把任务 `detach()` 掉
/// （gpui 的 `Task` 一 drop 就**取消**，见 `GitIdentityPage::load` 的注释），
/// 所以设置侧不需要（也不能）再管这条任务的生命周期。
pub struct GitIdentityHost {
    /// 传给 Core 的 `root`：**工作区根目录**（契约要求"存在的目录"，不必是仓库根，
    /// `shared/contracts/git-repository-setup.md:4`）。界面用它显示"正在配置哪个目录"。
    pub workspace_root: PathBuf,
    /// `git.repositorySetup` 一次：`scope` 取 `"local"` / `"global"`；
    /// 读成功交 `Some(Core 响应原文)`，失败交 `None`（宿主自己打 `S1_GIT_IDENTITY` 诊断）。
    ///
    /// `async_cx` 由设置侧的 `Context::spawn_in` 提供：宿主用它起后台任务并回前台投递。
    pub load: Box<
        dyn Fn(
            &str,
            Box<dyn FnOnce(Option<String>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static>,
            &mut gpui_kit::AsyncWindowContext,
        ),
    >,
    /// `git.configureIdentity` 一次：`key` 取 `"name"` / `"email"`，`value: None` = 清除覆盖。
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

// 窗口/进程级唯一的那份钩子（见模块文档的「为什么用线程局部」）。
// 写成普通注释而不是 `///`：`thread_local!` 是宏调用，rustdoc 不给它生成文档
// （会报 `unused_doc_comments`）。
thread_local! {
    static HOST: std::cell::RefCell<Option<Rc<GitIdentityHost>>> = const { std::cell::RefCell::new(None) };
}

/// 登记（或清除）身份钩子。由 `ShellWorkspace::new` 在启动时调一次。
///
/// 传 `None` 等于"这个宿主没有 Git 身份能力"，Git 页会**退回明确空态**
/// （不是 panic、也不是画一半的控件）。
pub fn set_git_identity_host(host: Option<GitIdentityHost>) {
    HOST.with(|slot| *slot.borrow_mut() = host.map(Rc::new));
}

/// Git 页渲染与操作需要的**全部**状态。`None` = 钩子没登记 → 空态。
///
/// 单独一个返回类型而不是直接暴露钩子：调用点（`SettingsDialog`）只关心
/// 「有没有」「工作区根是哪个」「怎么读写」，`Box<dyn Fn>` 的细节不外漏。
#[derive(Clone)]
pub struct GitIdentityPage {
    /// 工作区根（界面显示 + 传给 Core）。
    pub workspace_root: PathBuf,
    host: Rc<GitIdentityHost>,
}

/// 宿主登记的**工作区根**；没有宿主登记时 `None`。
///
/// ## 为什么这个访问器住在这里（而不是新建一份钩子）
///
/// `settings` crate 拿不到工作区根：它不依赖外壳，`SettingsDialog::new` 也没有根参数
/// （见 `crate::lib.rs` 的依赖方向）。全进程里**唯一**把根登记进来的地方就是
/// [`GitIdentityHost::workspace_root`]（`ShellWorkspace::new` 建视图时登记，
/// `lithe-gpui/crates/workbench/src/workspace.rs:771-774`），而「运行配置」页
/// （[`crate::run`]）要的正是同一个根（`runConfig.generate` 的 `root`）。
///
/// 所以这里把它抽成一个**中性名字**的访问器复用，而不是新加一份"只有设置页会读、
/// 却没有任何宿主会登记"的空钩子 —— 后者比依赖现有钩子更容易坏（页面会永远停在空态）。
/// 将来的退出路径：外壳愿意登记一个中性的根钩子时，只改这一个函数体。
///
/// 代价与边界：运行配置页的工作区根因此与「Git」页同源；宿主没登记时它退回
/// "打开项目后才能识别"的明确空态（[`crate::dialog`] 的 `Category::Run` 分支）。
pub fn host_workspace_root() -> Option<PathBuf> {
    git_identity_page().map(|page| page.workspace_root)
}

/// 取当前登记的原子钩子；没登记时 `None`。
pub fn git_identity_page() -> Option<GitIdentityPage> {
    HOST.with(|slot| {
        slot.borrow().as_ref().map(|host| GitIdentityPage {
            workspace_root: host.workspace_root.clone(),
            host: host.clone(),
        })
    })
}

impl GitIdentityPage {
    /// 读一次快照。`deliver` 在**前台的 `WeakEntity::update` 里**被调用一次：
    /// `(Some(快照), window, cx)` = 读到了，`(None, ..)` = 失败（宿主已打过诊断）。
    ///
    /// ⚠️ `deliver` 里**必须**自己做代次校验（切作用域 / 重新加载会让旧结果作废）；
    /// 本函数只保证"读回来的 JSON 已经解析成 [`IdentitySetup`]"。
    ///
    /// 泛型 `T` = 调用方视图的类型（`Context::spawn_in` 定义在 `Context<T>` 上）；
    /// 本函数不需要认识 `T`，只用它的上下文去起任务。
    pub fn load<T: 'static>(
        &self,
        scope: IdentityScope,
        deliver: impl FnOnce(Option<IdentitySetup>, &mut gpui_kit::Window, &mut gpui_kit::App)
        + 'static,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<T>,
    ) {
        let scope_id = scope.id().to_string();
        // 闭包必须是 `'static` 的，所以先把 `Rc` 克隆出来（借用 `self` 逃不出去）。
        let host = self.host.clone();
        // ⚠️ **必须 `detach()`，不能 `let _ =`**：gpui 的 `Task` 一 drop 就**取消**
        // （`gpui-pre-scheduler-0.3.6/src/executor.rs:389-391`：`If you drop a task it will
        // be cancelled immediately`）。这条在本轮实测踩到过：写成 `let _ =` 时
        // `S1_GIT_IDENTITY run=load` 出现了、`result=…` 永远不出现，界面卡在「正在检查
        // Git 仓库…」——正是"任务被丢掉"的样子。
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

    /// 写一个字段（`value: None` = 清除该作用域的覆盖）。
    ///
    /// ⚠️ 值必须由调用方先过 [`identity_value_is_valid`]：这里不再校验
    /// （"哪些值可写"是界面判据，宿主侧只负责转给 Core）。
    pub fn save<T: 'static>(
        &self,
        scope: IdentityScope,
        field: IdentityField,
        value: Option<String>,
        deliver: impl FnOnce(Option<IdentitySetup>, &mut gpui_kit::Window, &mut gpui_kit::App)
        + 'static,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::Context<T>,
    ) {
        let host = self.host.clone();
        // 理由同 [`Self::load`] 的注释：`Task` 一 drop 就取消，必须 `detach()`。
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

    /// **钩子没登记 → 空态**：这条是"HANDOFF 要求的降级分支"的机器可校验版本。
    ///
    /// 线程局部在同一测试进程里是共享的，所以这条测试**必须假设自己没登记过**：
    /// 文件里没有别的测试登记钩子（登记只发生在 `ShellWorkspace::new`，测试不构造它）。
    #[test]
    fn page_is_absent_until_a_host_registers_it() {
        set_git_identity_host(None);
        assert!(
            git_identity_page().is_none(),
            "没有宿主登记时必须返回 None（Git 页据此画明确空态）"
        );
        // 同一个钩子也是「运行配置」页拿到工作区根的唯一通道（见 `host_workspace_root` 的文档）：
        // 没登记时它必须同样是 `None`，页面据此画"打开项目后才能识别"的空态。
        assert!(
            host_workspace_root().is_none(),
            "没有宿主登记时工作区根必须是 None"
        );
    }

    /// 作用域 / 字段的字面量逐字对齐 Core 的枚举（`#[serde(rename_all = "camelCase")]`）。
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

    /// 响应解析：八个字段一一对应（键名 = Core 的 camelCase）。
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
        assert_eq!(setup.configured(IdentityField::Email), Some("dev@example.com"));
    }

    /// 缺字段 / 非法 JSON 都不 panic：缺字段回落默认，非法 JSON 返回 `None`。
    #[test]
    fn malformed_responses_never_panic() {
        let setup = IdentitySetup::from_json(r#"{"isRepository":true}"#).expect("缺字段要能解析");
        assert!(setup.is_repository);
        assert_eq!(setup.branch, None);
        assert!(IdentitySetup::from_json("not json").is_none());
        assert!(IdentitySetup::from_json("null").is_none());
    }

    /// **非仓库 + `local` 不可编辑**（真源与 Core 的同一条判据）；
    /// `global` 恒可编辑。
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

    /// 「保存」的四个禁用条件（真源 `git-identity-settings.tsx:133-139`）：
    /// 不可编辑 / 值非法 / 与已保存值相同 / 空草稿——逐个钉住。
    #[test]
    fn save_is_enabled_only_for_a_new_valid_value() {
        let setup = IdentitySetup {
            is_repository: true,
            configured_name: Some("Lithe Dev".to_string()),
            ..IdentitySetup::default()
        };
        // 新值 → 可保存。
        assert!(setup.can_save(IdentityScope::Local, IdentityField::Name, "Other"));
        // 与已保存值相同（含首尾空白差异）→ 不可保存。
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "Lithe Dev"));
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "  Lithe Dev  "));
        // 空 / 非法 → 不可保存。
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "   "));
        assert!(!setup.can_save(IdentityScope::Local, IdentityField::Name, "a<b"));
        // 不可编辑的作用域 → 不可保存，即使值本身合法。
        let no_repository = IdentitySetup::default();
        assert!(!no_repository.can_save(IdentityScope::Local, IdentityField::Name, "Other"));
        assert!(no_repository.can_save(IdentityScope::Global, IdentityField::Name, "Other"));
    }

    /// 「清除覆盖」只在**该作用域里确实写着值**时可点。
    #[test]
    fn clear_requires_a_configured_value() {
        let setup = IdentitySetup {
            configured_name: Some("Lithe Dev".to_string()),
            ..IdentitySetup::default()
        };
        assert!(setup.can_clear(IdentityField::Name));
        assert!(!setup.can_clear(IdentityField::Email));
    }

    /// 值的校验与 Core 的 `validate` 逐条对齐（`rust/lithe-core/src/git/setup.rs:224-236`）。
    ///
    /// ⚠️ 长度判据是**字节**：342 个汉字 = 1026 字节 > 1024，必须被拒；
    /// 1024 个 ASCII 字符正好合法（Core 用的是 `>` 而不是 `>=`）。
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
        // 原因码是诊断契约（`S1_GIT_IDENTITY … reason=`），逐条钉住。
        assert_eq!(identity_value_rejection("  "), Some("empty"));
        assert_eq!(identity_value_rejection("a<b"), Some("invalid_character"));
        assert_eq!(
            identity_value_rejection(&"汉".repeat(342)),
            Some("too_long")
        );
        assert_eq!(identity_value_rejection("ok"), None);
    }
}
