//! gpui 通知中心的状态层。
//!
//! 这里是「消息发生过」的唯一落点：每记一条，外壳就弹一条**左下角 toast**（gpui-kit
//! `component::Notification`，5 秒自动消失 —— 2026-09-29 起这是通知唯一的即时反馈；
//! 曾废掉的角落 toast 复活，状态栏 4 秒小字与状态栏"最新一条"回显则彻底不在了），
//! 条目本身由活动栏铃铛 + 右工具窗「通知」长期留档。
//!
//! ## 分层
//!
//! - [`model`]：一条通知存什么（稳定事件码 + 参数，**不是**成品文案）、怎么搜。
//! - [`severity`]：三档严重性、11 个 Core 稳定错误码到档位的映射、铃铛角标的形状。
//! - [`store`]：记录 / 去重 / 截断 / 未读水位线 / 角标。
//!
//! ## 本 crate 不做什么
//!
//! - **不渲染。** 没有任何 `impl IntoElement`。视图在
//!   `lithe-gpui/crates/workbench/src/right_tool_window.rs`。
//! - **不拼文案。** 条目存 `tr` 的键与插值参数，由展示层调
//!   [`lithe_gpui_shared::i18n::tr_args`] 拼。依据是
//!   `shared/contracts/application-boundary.md:227-228`。
//! - **不读时钟。** [`Store::record`] 的 `now_ms` 由调用方传进来（见 `store` 的模块文档）。
//! - **不碰 Core。** 依赖图上压在 `lithe-gpui-shared` 之上、其余 crate 之下；
//!   Core 错误码以**字符串**形态进来（`shared::core_client::CoreError::Reported { code }`）。
//!
//! ## 为什么独立成一个 crate
//!
//! **依赖方向。** `workbench` 依赖全部其它 crate（editor / explorer / git / java /
//! settings / terminal），而其余每个只依赖 `shared`。通知中心的数据住在 `workbench` 的
//! `ShellWorkspace` 里，但 `git` 的写失败、`editor` 的保存失败都要往里记 —— store 放在
//! `workbench` 就得让它们反向依赖，成环。独立成 `lithe-gpui-notify`（压在 `shared` 之上、
//! 其余之下）之后，业务 crate 只需要依赖它，依赖图不变。
//!
//! ## store 归谁持有
//!
//! 由 `ShellWorkspace` 持有一个 `Entity<Store>`，并把**弱引用**注入给要发通知的
//! 业务实体（`EditorPane` 等）。不是全局槽 —— gpui 的 `Global` 只在 `App` 上有
//! `try_global` / `set_global`（`gpui-pre-0.3.6/src/app.rs:2134,2164`），而
//! `&mut Context<T>` 拿不到 `&mut App`，所以全局槽那条路走不通。
//!
//! 换根会重建 `ShellWorkspace`，于是**下一个项目的通知中心天然是空的**（新 store、
//! 水位线归零），不需要显式清空。代价是换根前到达的通知会跟着旧 store 一起没 —— 但那一刻
//! 旧 `EditorPane` 的异步任务本身也被 drop 了，原来那条 toast 同样发不出来，所以没有回归。
//!
//! 决策来源：
//!
//! `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`

// Note: 通知中心的边界与被否方案见 .agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md

mod core_bridge;
mod model;
mod severity;
mod store;

pub use core_bridge::{from_core_error, is_reportable};
pub use model::{matches_text, Entry, EntryId, Target, MAX_ENTRIES};
pub use severity::{Badge, Severity};
pub use store::{Input, RecordOutcome, Store};

/// 现在到 Unix 毫秒（`Store::record` 的 `now_ms`）。
///
/// 墙钟而不是单调时钟：通知时间戳要显示给用户。`SystemTime` 在 1970 之前返回 `Err`，
/// 退化成 0 —— 显示成一个时间戳，而不是让整帧 render 失败。
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    /// 墙钟不许回退：通知行按它算相对时间，回退会让「3 分钟前」显示成负的
    /// （`format_age` 里的 `saturating_sub` 会兜住，但那是兜底，不该是常态）。
    #[test]
    fn now_ms_does_not_go_backwards() {
        let first = super::now_ms();
        let second = super::now_ms();
        assert!(second >= first, "墙钟不许回退：{second} < {first}");
    }
}
