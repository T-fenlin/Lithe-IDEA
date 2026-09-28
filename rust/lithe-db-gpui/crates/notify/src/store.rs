//! 通知中心的状态机：记录、去重、截断、未读水位线、角标。
//!
//! ## 为什么 store 不挂在 `ShellWorkspace` 上
//!
//! `.agents/notes/implemented/bug-fix/2026-09-27-exclude-write-must-be-read-back.md:73-77`
//! 否决「用通知报失败」时给了一条技术理由：异步结果回来时只保证**外壳实体**还活着
//! （换根会重建外壳），而对话框与通知要求窗口与 `Root` 就位。
//!
//! 通知中心是 `ShellWorkspace` 渲染的右工具窗，所以同一条理由对 store 直接成立：
//! **store 住在 `ShellWorkspace` 里，换根就一起没了**。这就是本 crate 独立成
//! `lithe-db-gpui-notify` 的第一条理由（第二条是依赖方向 —— `workbench` 依赖全部其它
//! crate，store 放 `workbench` 会让 `git` / `editor` / `java` 反向依赖而成环）。
//!
//! 调用方把 store 建成 `Entity<Store>` 挂在比外壳更长命的地方，用
//! [`Store::clear`] 对齐换根。
//!
//! ## 时钟由调用方传进来
//!
//! [`Store::record`] 的 `now_ms` 是参数而不是内部读的。这不是洁癖：
//! `write-stable-tests` 要求时间确定，而「4 秒后消失」「角标什么时候消」这类行为一旦
//! 内部直接读时钟就没法在单测里断言。调用方从 `Instant` 或系统时钟取一次传进来即可。
//!
//! ## 排序
//!
//! `entries` **始终是新→旧**。不排序：每次记录都插到最前面（或把命中的那条挪到最前面），
//! 所以数组顺序本身就是对的。排序会引入「同 seq 时谁在前」的比较问题，而 seq 是唯一的。

use gpui_kit::SharedString;

use crate::model::{Entry, EntryId, MAX_ENTRIES, Target};
use crate::severity::{Badge, Severity};

/// 一次 [`Store::record`] 的结果。给诊断行和测试用。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordOutcome {
    /// 新增了一条。
    Inserted,
    /// 命中去重键、原地替换并置顶。带替换后的累计次数。
    Replaced { occurrences: u32 },
}

/// 记一条通知的入参。
///
/// 字段私有 + builder：跨 crate 之后结构体字面量不再是合法构造方式
/// （《编码指南》「公共 API 设计」）。
#[derive(Clone, Debug)]
pub struct Input {
    code: SharedString,
    params: Vec<(SharedString, SharedString)>,
    severity: Severity,
    key: Option<SharedString>,
    target: Option<Target>,
}

impl Input {
    /// 一条 `severity` 档位、事件码为 `code` 的通知。`code` 同时是 `tr` 的键。
    pub fn new(code: impl Into<SharedString>, severity: Severity) -> Self {
        Self {
            code: code.into(),
            params: Vec::new(),
            severity,
            key: None,
            target: None,
        }
    }

    /// 一条来自 Core 稳定错误码的通知。
    ///
    /// `core_code` 是 `core_client::CoreError::Reported { code }` 里那个字符串形态。
    /// `cancelled` 会走 [`Severity::for_core_code`] 的 `None` 分支 —— 调用方应该用
    /// [`Input::for_core_code`] 的 `Option` 返回值先挡掉，不要指望这里。
    pub fn for_core_code(core_code: &str) -> Option<Self> {
        let severity = Severity::for_core_code(core_code)?;
        Some(
            Self::new(Severity::notification_code_for_core_code(core_code), severity)
                .param("coreCode", core_code),
        )
    }

    /// 事件码（同时是 `tr` 的键）。给诊断行用 —— `record` 之后 `input` 就被移进 store 了。
    pub fn code(&self) -> &SharedString {
        &self.code
    }

    /// 严重性档位。给诊断行用。
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// 插值参数，按加入顺序。给调用点与测试用（条目建成之后 `Entry::params()` 才是权威）。
    pub fn params(&self) -> &[(SharedString, SharedString)] {
        &self.params
    }

    /// 加一个插值参数。同名参数后加的覆盖先加的（`interpolate` 是顺序替换）。
    pub fn param(mut self, name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        let name = name.into();
        let value = value.into();
        self.params.retain(|(existing, _)| *existing != name);
        self.params.push((name, value));
        self
    }

    /// 设定去重键。同键的通知原地替换 + 置顶 + 次数加一，而不是新增一行。
    ///
    /// 哪些调用点该给键：会**反复**发生的同一件事（保存失败、索引失败、找不到目标）。
    /// 哪些不该给：本来就该每次都留痕的事。
    pub fn key(mut self, key: impl Into<SharedString>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// 设定跳转目标。**本轮无调用方**，随 [`Target`] 一起占位。
    pub fn target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }
}

/// 通知中心的状态。
///
/// 视图层把它建成 `Entity<Store>`，用 `cx.observe` + `cx.notify()` 驱动重绘
/// （范式照 `lithe-db-gpui/crates/workbench/src/workspace.rs:1447` 订阅 `project_menu` 的写法）。
#[derive(Debug)]
pub struct Store {
    /// 新→旧。
    entries: Vec<Entry>,
    /// 下一个可用序号。见 [`Store::record`] 里「单一计数器」的说明。
    next_seq: u64,
    /// 已读水位线：用户上次打开中心时见到的最大 `seq`。
    ///
    /// 用**序号**而不是「数组长度」或「下标」，是为了在删除、去重替换、容量截断之后
    /// 仍然成立。`Entry::is_unread` 判的就是 `entry.seq > read_upto_seq`。
    read_upto_seq: u64,
}

/// 序号从 **1** 起，不是 0。
///
/// 因为未读判据是 `seq > read_upto_seq`，而 `read_upto_seq` 初始为 0：若第一条通知拿到
/// `seq = 0`，它一进中心就被判成已读，角标永远不亮。派生的 `Default` 给出 0，做不到这件事，
/// 所以手写。`record_all_is_unread` 守着这条。
impl Default for Store {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_seq: 1,
            read_upto_seq: 0,
        }
    }
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记一条通知。返回 [`RecordOutcome`]。
    ///
    /// ## 单一计数器
    ///
    /// `next_seq` 同时供给 [`Entry::id`] 与 [`Entry::seq`]。这省掉第二个计数器，代价是
    /// 「id 单调」而不是「id 连续」—— 而 id 只用于删除一个刚拿到的条目，不需要连续。
    ///
    /// ## 命中去重键时
    ///
    /// 原地替换 → 挪到最前 → `seq` 取新值（所以它**重新变成未读**，这正是「又发生了一次」
    /// 该有的行为）→ `created_at_ms` 与 `id` 不变。
    ///
    /// ## 容量
    ///
    /// 超过 [`MAX_ENTRIES`] 就丢最旧的一条（数组末尾，因为顺序是新→旧）。
    pub fn record(&mut self, input: Input, now_ms: u64) -> RecordOutcome {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);

        // 先用不可变借用找位置，再单独可变借用改内容 —— 两步分开是为了不把
        // `iter_mut` 的借用拖到「挪到最前」那一步，两个阶段会互相冲突。
        if let Some(key) = input.key.as_ref() {
            if let Some(position) = self.entries.iter().position(|entry| entry.key() == Some(key)) {
                self.entries[position].replace_with(
                    input.code,
                    input.params,
                    input.severity,
                    input.target,
                    now_ms,
                    seq,
                );
                let entry = self.entries.remove(position);
                self.entries.insert(0, entry);
                return RecordOutcome::Replaced {
                    occurrences: self.entries[0].occurrences(),
                };
            }
        }

        self.entries.insert(
            0,
            Entry::new(
                seq,
                input.code,
                input.params,
                input.severity,
                input.key,
                input.target,
                now_ms,
                seq,
            ),
        );
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
        RecordOutcome::Inserted
    }

    /// 清空，并把水位线一起归零。
    ///
    /// **换根必须走这里**，而且「清空」与「水位线归零」要是同一个动作 ——
    /// `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`
    /// 的风险一节点名了这个耦合：只清空不归零会让新项目的通知被上一条水位线判成已读。
    ///
    /// `next_seq` **不**归零：id 只要在本 store 生命周期内唯一就够了，归零会让
    /// 「界面还持有一个旧 id」这种情形变成静默删错条目。
    ///
    /// 返回被清掉的条数。
    pub fn clear(&mut self) -> usize {
        let removed = self.entries.len();
        self.entries.clear();
        self.read_upto_seq = 0;
        removed
    }

    /// 删掉一条。返回是否真的删掉了。
    pub fn remove(&mut self, id: EntryId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id() != id);
        self.entries.len() != before
    }

    /// 把水位线推到「当前已发出的最大序号」，即全部标为已读。
    ///
    /// 铃铛被点开时调。**不要**用「逐条把 `read` 打成 true」的实现 —— 那是
    /// `WorkbenchNotificationFeatureModel.swift:82-86` 的做法，
    /// `lithe-db-gpui/docs/archive/ui-map-macos.md:96` 已经点名它让未读红点形同虚设。
    pub fn mark_all_read(&mut self) {
        self.read_upto_seq = self.next_seq.wrapping_sub(1);
    }

    /// 全部条目，新→旧。
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 铃铛角标：**红点优先**。只要有一条未读错误就是红点，其次蓝点，无未读则不画。
    pub fn badge(&self) -> Badge {
        let mut has_error = false;
        let mut has_other = false;
        for entry in &self.entries {
            if !entry.is_unread(self.read_upto_seq) {
                continue;
            }
            match entry.severity() {
                Severity::Error => has_error = true,
                Severity::Info | Severity::Warning => has_other = true,
            }
        }
        if has_error {
            Badge::Error
        } else if has_other {
            Badge::Info
        } else {
            Badge::None
        }
    }

    /// 有没有未读。状态栏那条「点开中心」的入口用它决定要不要强调。
    pub fn has_unread(&self) -> bool {
        self.badge() != Badge::None
    }

    /// 未读条数。**只给状态栏 / tooltip 用**，铃铛角标不画数字（见 [`Badge`]）。
    pub fn unread_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.is_unread(self.read_upto_seq))
            .count()
    }

    /// 最新的那一条（`updated_at_ms` 最大的未读优先，否则取第一条）。
    ///
    /// 状态栏那条「点开中心」的入口显示的就是它 —— 那是「刚刚发生了一下」的唯一信号，
    /// 取代被删掉的 `status_notice` 那条 4 秒小字。
    pub fn latest(&self) -> Option<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_unread(self.read_upto_seq))
            .max_by_key(|entry| entry.seq())
            .or_else(|| self.entries.first())
    }

    /// 当前已见最大序号。给「换根时对齐」这类断言用。
    pub fn read_upto_seq(&self) -> u64 {
        self.read_upto_seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 单调递增的假时钟：每次 `record` 前进 10ms，让「最后更新时间」可断言。
    struct Clock(u64);

    impl Clock {
        fn next(&mut self) -> u64 {
            self.0 += 10;
            self.0
        }
    }

    fn input(code: &str, severity: Severity) -> Input {
        Input::new(code, severity)
    }

    /// 记录 → 新→旧。
    #[test]
    fn records_newest_first() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("a", Severity::Info), clock.next());
        store.record(input("b", Severity::Info), clock.next());
        store.record(input("c", Severity::Info), clock.next());

        let codes: Vec<_> = store.entries().iter().map(|e| e.code().to_string()).collect();
        assert_eq!(codes, vec!["c", "b", "a"]);
    }

    /// 同 key 反复发生：原地替换 + 置顶 + 次数加一，**不新增行**。
    ///
    /// 这是「通知量不被高频事件冲垮」的核心保证 —— 旧 Windows 产品有 269+ 个
    /// `toast.*` 调用点，逐条追加的话 100 条上限几秒就用完。
    #[test]
    fn same_key_replaces_in_place_and_moves_to_front() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("keep", Severity::Info).key("k"), clock.next());
        store.record(input("other", Severity::Info), clock.next());

        let first = store.record(input("hit-1", Severity::Error).key("k"), clock.next());
        assert_eq!(first, RecordOutcome::Replaced { occurrences: 2 });
        let second = store.record(input("hit-2", Severity::Warning).key("k"), clock.next());
        assert_eq!(second, RecordOutcome::Replaced { occurrences: 3 });

        assert_eq!(store.len(), 2, "同 key 不该新增行");
        let top = &store.entries()[0];
        assert_eq!(top.code().to_string(), "hit-2");
        assert_eq!(top.occurrences(), 3);
        assert_eq!(top.severity(), Severity::Warning, "档位要被替换成最新的");
    }

    /// 去重替换保留 `id` 与 `created_at_ms`，更新 `updated_at_ms`。
    #[test]
    fn replace_keeps_created_at_and_updates_updated_at() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("v1", Severity::Info).key("k"), clock.next());
        let before = store.entries()[0].clone();
        clock.next();
        store.record(input("v2", Severity::Info).key("k"), clock.next());
        let after = &store.entries()[0];

        assert_eq!(after.id(), before.id());
        assert_eq!(after.created_at_ms(), before.created_at_ms());
        assert!(after.updated_at_ms() > before.updated_at_ms());
    }

    /// 无 key 的两次记录是两条独立条目 —— 「本来就该每次留痕的事」不去重。
    #[test]
    fn no_key_always_appends() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("same", Severity::Info), clock.next());
        store.record(input("same", Severity::Info), clock.next());
        assert_eq!(store.len(), 2);
    }

    /// 容量截断：超过 100 条丢最旧的，且保新→旧。
    #[test]
    fn caps_at_max_entries_dropping_oldest() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        for i in 0..(MAX_ENTRIES + 5) {
            store.record(input(&format!("n{i}"), Severity::Info), clock.next());
        }
        assert_eq!(store.len(), MAX_ENTRIES);
        assert_eq!(store.entries()[0].code().to_string(), "n104");
        assert_eq!(
            store.entries()[MAX_ENTRIES - 1].code().to_string(),
            "n5"
        );
    }

    /// 角标：红点优先于蓝点。
    #[test]
    fn badge_prefers_error_over_info() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        assert_eq!(store.badge(), Badge::None, "空中心没有角标");

        store.record(input("i", Severity::Info), clock.next());
        assert_eq!(store.badge(), Badge::Info, "未读 info 是蓝点");

        store.record(input("w", Severity::Warning), clock.next());
        assert_eq!(store.badge(), Badge::Info, "warning 也是蓝点那一档");

        store.record(input("e", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error, "有未读 error 就是红点");
    }

    /// 回归：`next_seq` 必须从 **1** 起。
    ///
    /// 曾经用派生的 `Default`（0），而未读判据是 `seq > read_upto_seq`、`read_upto_seq`
    /// 初始 0 —— 于是**第一条通知一进中心就是已读**，角标永远不亮。这条守住那个 off-by-one。
    #[test]
    fn first_record_is_unread() {
        let mut store = Store::new();
        store.record(input("first", Severity::Error), 1);
        assert_eq!(
            store.badge(),
            Badge::Error,
            "第一条通知必须是未读，否则角标永远不亮"
        );
        assert_eq!(store.unread_count(), 1);
    }

    /// 已读水位线：`mark_all_read` 之后角标消失，但**之后**新来的那条仍是未读。
    ///
    /// 这条是水位线替代逐条 `read: bool` 的关键 —— 逐条 boolean 做不到「你正看着中心时
    /// 新来的那条仍然是未读」。
    #[test]
    fn mark_all_read_does_not_swallow_later_arrivals() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("before", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error);

        store.mark_all_read();
        assert_eq!(store.badge(), Badge::None, "推进水位线后没有未读");
        assert_eq!(store.unread_count(), 0);

        store.record(input("after", Severity::Error), clock.next());
        assert_eq!(
            store.badge(),
            Badge::Error,
            "打开中心之后才发生的事，必须还是未读"
        );
        assert_eq!(store.unread_count(), 1);
    }

    /// 删除条目不会让水位线失真：水位线判的是 seq，不是下标。
    #[test]
    fn removal_does_not_distort_the_watermark() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        let a = store.record(input("a", Severity::Error), clock.next());
        assert_eq!(a, RecordOutcome::Inserted);
        let a_id = store.entries()[0].id();
        store.record(input("b", Severity::Error), clock.next());
        store.mark_all_read();
        assert_eq!(store.badge(), Badge::None);

        assert!(store.remove(a_id));
        assert_eq!(store.len(), 1);
        assert_eq!(store.badge(), Badge::None, "删掉已读的条目不该冒出角标");

        store.record(input("c", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error, "新来的仍是未读");
    }

    /// 换根：`clear` 把条目和水位线**一起**清掉。
    ///
    /// `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md` 的
    /// 风险一节点名了这个耦合 —— 只清条目不归零水位线的话，新项目的通知会被上一条水位线
    /// 判成已读。
    #[test]
    fn clear_resets_entries_and_watermark_together() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("old", Severity::Error), clock.next());
        store.mark_all_read();
        assert_eq!(store.badge(), Badge::None);

        let removed = store.clear();
        assert_eq!(removed, 1);
        assert!(store.is_empty());
        assert_eq!(store.read_upto_seq(), 0, "水位线必须一起归零");

        // 换根之后新项目的通知必须是未读的。
        store.record(input("new-project", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error);
    }

    /// `clear` 之后 id 仍然唯一：id 单调递增、不归零。
    #[test]
    fn clear_does_not_recycle_ids() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("a", Severity::Info), clock.next());
        let first_id = store.entries()[0].id();
        store.clear();
        store.record(input("b", Severity::Info), clock.next());
        assert_ne!(store.entries()[0].id(), first_id);
    }

    /// Core 码进中心：`cancelled` 不进，其余带 `coreCode` 参数（于是可搜、可显示）。
    #[test]
    fn core_codes_map_through_and_cancelled_is_dropped() {
        assert!(
            Input::for_core_code("cancelled").is_none(),
            "cancelled 是用户自己取消的，不该进中心"
        );

        let input = Input::for_core_code("process_failed").expect("process_failed 有档位");
        let mut store = Store::new();
        store.record(input, 1);
        let entry = &store.entries()[0];
        assert_eq!(entry.severity(), Severity::Error);
        assert_eq!(entry.code().to_string(), "lithe.notifications.core.processFailed");
        assert!(
            entry.matches_query("process_failed"),
            "coreCode 参数必须可搜"
        );
    }

    /// `latest` 给状态栏那条入口用：优先未读里最新的，没有未读才退回最后一条。
    #[test]
    fn latest_prefers_newest_unread() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        assert!(store.latest().is_none());
        store.record(input("a", Severity::Info), clock.next());
        store.record(input("b", Severity::Info), clock.next());
        assert_eq!(store.latest().unwrap().code().to_string(), "b");

        store.mark_all_read();
        assert_eq!(
            store.latest().unwrap().code().to_string(),
            "b",
            "无未读时退回最后一条，让状态栏不留空白"
        );
    }
}
