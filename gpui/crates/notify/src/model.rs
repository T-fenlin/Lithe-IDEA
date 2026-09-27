//! 通知条目模型：通知中心存什么，以及怎么搜、怎么算未读。
//!
//! 本模块是纯数据 + 纯函数，**不做任何渲染、不碰 locale、不读时钟**。这三条约束是
//! 刻意的，见文末「为什么时钟由调用方传进来」。
//!
//! ## 存什么
//!
//! 一条通知存的是**稳定事件码 + 参数**，不是拼好的文案。依据是
//! `shared/contracts/application-boundary.md:227-228`：领域层返回稳定原因，用户可见文案
//! 归各产品展示层所有。所以 `code` 是 `tr` 的键、`params` 是插值参数，界面自己调
//! [`lithe_gpui_shared::i18n::tr_args`] 拼出这句话。
//!
//! ## 三个不要
//!
//! - **不要 `success` 档。** 旧 Windows 产品的角标本来就排除 success
//!   （`windows/tauri/src/features/notifications/components/notifications-trigger.tsx:24`），
//!   而成功不是事后还需要回看的信息，它只该占用现有的绿条
//!   （`gpui/crates/git/src/changes_view.rs:913`）。
//! - **不要逐条 `read: bool`。** 那是 `macos/Sources/Lithe/Application/Features/
//!   WorkbenchNotificationFeatureModel.swift:82-86` 的做法，而
//!   `gpui/docs/archive/ui-map-macos.md:96` 已经点名它的后果：通知中心一打开就
//!   `markAllNotificationsRead()`，未读红点形同虚设。逐条 boolean 换个写法照样复现 ——
//!   它没法表达「你正看着中心时新来的那条仍然是未读」。本模块用 [`Store::read_upto_seq`]
//!   水位线代替，见 [`crate::store`]。
//! - **不要在这里读时钟。** 原因见文末。
//!
//! ## 决策来源
//!
//! `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`

use gpui_kit::SharedString;

use crate::severity::Severity;

/// 历史上限。
///
/// 取 100 有依据：macOS 的 `WorkbenchNotificationFeatureModel.swift:4-8` 用的就是
/// `maximumHistoryCount = 100`，也是三个已有数字里最宽松的一个（Windows 旧产品
/// `notifications.store.ts:5` 是 20、gpui-kit 的 toast 是 `notification.rs:558` 的 10）。
/// 通知中心是唯一的记录面，比那两个临时面需要更长的回看窗口。
pub const MAX_ENTRIES: usize = 100;

/// 一条通知的稳定标识。
///
/// 去重替换时**不变**（同一个逻辑事件还是同一条），而 [`Entry::seq`] 每次记录都变 ——
/// 这两件事分开是「已读过的条目再次发生时应该重新变未读」的前提。
pub type EntryId = u64;

/// 可选的跳转目标。
///
/// **本轮没有任何生产者**，只有类型。留它的理由是：IDEA 的详情是「描述 + 一个跳转链接」，
/// 而链接要有真实目的地才有意义；gpui 现在一半功能没建
/// （`gpui/HANDOFF.md:265` 记着缺 Java 智能提示与项目模型），现在接线出来的多半是死链，
/// 而死链比没有链接更糟。先把字段占住，以后填目的地不用改模型、不用迁数据。
///
/// 两个变体都有出处：JetBrains 官方文档的原话是错误描述「is provided with a link that
/// opens another dialog or a tool window in which you can examine the problem in detail」。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// 打开某个文件（可带一行号）去看细节。
    File {
        /// 工作区相对路径，`/` 分隔（`shared/contracts/application-boundary.md` 的
        /// workspace-relative 口径）。**不要**塞绝对路径。
        path: SharedString,
        /// 一基行号（同一份契约的一基口径）。
        line: Option<u32>,
    },
    /// 切到某个工具窗。`id` 用 `RightToolWindowView::id()` 那套小写标识。
    ToolWindow {
        /// 工具窗标识。
        id: SharedString,
    },
}

/// 一条通知。
///
/// 字段私有 + 构造器：跨 crate 之后结构体字面量不再是合法构造方式
/// （《编码指南》「公共 API 设计」，`gpui/crates/workbench/src/activity_bar.rs:172-174`
/// 是同一条理由的先例）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    id: EntryId,
    code: SharedString,
    params: Vec<(SharedString, SharedString)>,
    severity: Severity,
    /// 去重键。`None` = 每次记录都新增一条。
    key: Option<SharedString>,
    /// 这个事件被记录过几次（含首次）。
    occurrences: u32,
    created_at_ms: u64,
    updated_at_ms: u64,
    /// 单调递增的「最后一次被记录」序号。驱动新→旧排序与未读判定，见 [`Entry::is_unread`]。
    seq: u64,
    target: Option<Target>,
}

impl Entry {
    pub(crate) fn new(
        id: EntryId,
        code: SharedString,
        params: Vec<(SharedString, SharedString)>,
        severity: Severity,
        key: Option<SharedString>,
        target: Option<Target>,
        now_ms: u64,
        seq: u64,
    ) -> Self {
        Self {
            id,
            code,
            params,
            severity,
            key,
            occurrences: 1,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            seq,
            target,
        }
    }

    /// 稳定标识，去重替换时不变。
    pub fn id(&self) -> EntryId {
        self.id
    }

    /// 事件的稳定码，同时是 `tr` 的键。**不是**成品文案。
    pub fn code(&self) -> &SharedString {
        &self.code
    }

    /// 插值参数，按传入顺序保留（决定文案里的占位符替换顺序，不做排序 ——
    /// 排序会让 `{a} {b}` 这种依赖顺序的文案在两次运行间产生不同结果）。
    pub fn params(&self) -> &[(SharedString, SharedString)] {
        &self.params
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// 去重键；`None` 表示这条不参与去重。
    pub fn key(&self) -> Option<&SharedString> {
        self.key.as_ref()
    }

    /// 这个事件累计出现过几次。列表里**不画**「×N」（右工具窗现在只有约 350px 宽），
    /// 但字段留着 —— 以后想画不用改模型、不用迁数据。
    pub fn occurrences(&self) -> u32 {
        self.occurrences
    }

    /// 首次记录时间（毫秒）。去重替换**不改**它 —— 这条逻辑事件还是从第一次开始。
    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    /// 最后一次记录时间（毫秒），去重替换时更新。
    pub fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    /// 最后一次被记录的序号。
    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    /// 设定跳转目标。**本轮无调用方**，随 [`Target`] 一起占位。
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    /// 是否未读：`seq` 超过水位线即为未读。
    ///
    /// 这就是替代逐条 `read: bool` 的那个判据。参数 `read_upto_seq` 是「上次打开中心时
    /// 已见的最大序号」。因为判的是**序号**而不是「数组下标」或「数组长度」，删除条目、
    /// 去重替换、容量截断都不会让水位线失真；而且「你正看着中心时新来的那条」一定
    /// `seq > read_upto_seq`，仍然是未读。
    pub fn is_unread(&self, read_upto_seq: u64) -> bool {
        self.seq > read_upto_seq
    }

    /// 按 key 命中时原地替换内容。`id` / `created_at_ms` 保持不变。
    pub(crate) fn replace_with(
        &mut self,
        code: SharedString,
        params: Vec<(SharedString, SharedString)>,
        severity: Severity,
        target: Option<Target>,
        now_ms: u64,
        seq: u64,
    ) {
        self.code = code;
        self.params = params;
        self.severity = severity;
        self.target = target;
        self.updated_at_ms = now_ms;
        self.seq = seq;
        self.occurrences = self.occurrences.saturating_add(1);
    }

    /// 搜索：**只**看稳定码与参数值，不看渲染后的文案。
    ///
    /// 稳定码按 ASCII 大小写不敏感匹配（键与 Core 的 11 个错误码都是 ASCII 标识符，
    /// 照 `core_client.rs:261` 那个 `invalid_request` 判据的口径）；
    /// **参数值原样匹配，不折叠大小写** —— 折叠会在含中文或大小写敏感的路径上产生意外，
    /// 而 11 个错误码那点 ASCII 大小写需求由码自己那一路满足了。
    ///
    /// 渲染后文案的那一路由界面自己判（见 [`matches_text`]），因为只有界面知道拼出来的
    /// 那句话是什么。空查询命中一切。
    pub fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        if contains_ascii_ci(&self.code, query) {
            return true;
        }
        self.params.iter().any(|(_, value)| contains(value, query))
    }
}

/// 文案侧的搜索判据：子串命中，**原样**、不折叠大小写。
///
/// 单独抽成函数是为了让「为什么不折叠」这个决定落在代码里而不是只写在文档里。
pub fn matches_text(text: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    contains(text, query)
}

/// 原样子串。
fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// ASCII 大小写不敏感子串；非 ASCII 字符按原样逐字节比。
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let haystack = haystack.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    haystack.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(code: &str, params: &[(&str, &str)]) -> Entry {
        entry_with_seq(code, params, 1)
    }

    /// 显式指定 `seq` —— 未读判据是按序号而不是按数组位置，所以「哪条更新」必须
    /// 体现在数据里而不是靠数组下标。
    fn entry_with_seq(code: &str, params: &[(&str, &str)], seq: u64) -> Entry {
        Entry::new(
            seq,
            code.into(),
            params
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            Severity::Info,
            None,
            None,
            0,
            seq,
        )
    }

    /// 未读判据是**序号**而不是下标：删掉前面的条目不会让后面的条目变成已读。
    /// 这是水位线替代逐条 boolean 的核心好处，必须守住。
    #[test]
    fn unread_is_decided_by_seq_not_by_position() {
        let older = entry_with_seq("lithe.notifications.demo.old", &[], 1);
        let newer = entry_with_seq("lithe.notifications.demo.new", &[], 2);
        assert!(older.is_unread(0));
        assert!(newer.is_unread(0));
        // 水位线推到 1：只有 seq=2 的那条仍算未读。
        assert!(!older.is_unread(1));
        assert!(newer.is_unread(1));
    }

    /// 稳定码要能 ASCII 大小写不敏感命中：`Invalid_Request` 与 `invalid_request`
    /// 指的是同一件事。照 `core_client.rs:261` 的判据口径。
    #[test]
    fn code_matches_ascii_case_insensitively() {
        let item = entry("invalid_request", &[]);
        assert!(item.matches_query("INVALID_REQUEST"));
        assert!(item.matches_query("Invalid_Request"));
        assert!(item.matches_query("request"));
    }

    /// 参数值**原样**匹配，不折叠大小写。这是有意的：折叠会在路径上产生意外，
    /// 而 ASCII 大小写那点需求已经由稳定码那一路满足了。
    #[test]
    fn param_values_match_verbatim() {
        let item = entry("lithe.notifications.demo", &[("path", "/repo/POM.XML")]);
        assert!(item.matches_query("POM.XML"));
        assert!(!item.matches_query("pom.xml"));
    }

    /// 参数**名**不参与搜索：用户找的是值（路径、行号），不是「哪个参数」。
    #[test]
    fn param_names_are_not_searchable() {
        let item = entry("lithe.notifications.demo", &[("path", "value")]);
        assert!(item.matches_query("value"));
        assert!(!item.matches_query("path"));
    }

    /// 空查询命中一切 —— 搜索框清空时列表要立刻回到全量。
    #[test]
    fn empty_query_matches_everything() {
        let item = entry("lithe.notifications.demo", &[]);
        assert!(item.matches_query(""));
        assert!(matches_text("任意文案", ""));
    }

    /// 文案侧只做原样子串，理由与参数值一致。
    #[test]
    fn text_matches_verbatim() {
        assert!(matches_text("打开失败", "失败"));
        assert!(!matches_text("打开失败", "失败：/repo"));
    }

    /// 去重替换必须保住 `id` 与 `created_at_ms`（同一个逻辑事件），更新 `updated_at_ms`
    /// 与次数，并拿到新的 `seq`（所以它会重新变成未读）。
    #[test]
    fn replace_keeps_identity_but_refreshes_recency() {
        let mut item = entry("lithe.notifications.demo", &[]);
        let id = item.id();
        let created = item.created_at_ms();

        item.replace_with(
            "lithe.notifications.demo2".into(),
            vec![],
            Severity::Error,
            None,
            500,
            9,
        );

        assert_eq!(item.id(), id, "id 必须不变，否则界面持有的 id 会失效");
        assert_eq!(item.created_at_ms(), created, "首次时间不该被替换改掉");
        assert_eq!(item.updated_at_ms(), 500);
        assert_eq!(item.occurrences(), 2);
        assert_eq!(item.seq(), 9);
        assert_eq!(item.severity(), Severity::Error);
        assert!(item.is_unread(8), "重新发生的事件必须重新变未读");
    }
}
