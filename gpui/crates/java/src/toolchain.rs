//! 「这台机器上用哪个 JDK 起 JDT LS」的**宿主注入点**：设置页覆盖值在语言服务侧的唯一消费入口。
//!
//! ## 为什么是"注册函数 + 进程级槽"，而不是让本 crate 依赖 `lithe-gpui-settings`
//!
//! 设置页（`lithe_gpui_settings::project`）把用户填的 JDK 主目录存进设置文件的
//! `javaHomePath`，而这条值真正要改的是**JDT LS 用哪个 JDK 起**
//! （[`crate::jdtls::resolve_runtime`]）。但依赖方向不允许 java → settings：
//! settings 的模块文档写死了它只向下依赖 `gpui-kit` + `lithe-gpui-shared`，
//! 反过来让 java 依赖 settings 就会把"设置文件的键与格式"灌进语言服务这一层。
//!
//! 所以照仓库里已有的宿主钩子口径（`lithe_gpui_settings::identity::GitIdentityHost`：
//! 低层 crate 定义钩子、`workbench` 在 `ShellWorkspace::new` 里登记实现）：
//! **本模块定义注入点，`workbench` 把设置文件里的值填进来**（登记点见
//! `gpui/crates/workbench/src/workspace.rs` 的 `register_java_toolchain`）。
//!
//! 与 Git 身份钩子唯一的差别是这里**没有回调**：设置侧不需要反过来调 java，
//! 所以槽里放的是纯数据（`GitIdentityHost` 放的是两个 `Box<dyn Fn>`，因为写身份那条路
//! 要 Core 命令、只有 `workbench` 拼得起来；这里没有任何"要宿主去做的事"）。
//!
//! ## 为什么是 `Mutex` 而不是 `thread_local!`
//!
//! `GitIdentityHost` 用 `thread_local!`（`settings/src/identity.rs:311-321`），因为它的调用方
//! 就在登记它的那条 UI 线程上。这里不行：槽的读者是 `JavaLanguageService::start_locked`
//! （`java/src/service.rs:661-663`），它跑在 gpui 的**后台执行器线程**上
//! （测试里还可以是自己起的 `std::thread`，见 `service.rs` 的真实 JDTLS 用例），
//! 与登记它的 `ShellWorkspace::new` 不是同一条线程 —— `thread_local!` 在那边读到的永远是
//! `None`，于是"设置里选了 JDK、语言服务照旧用自动发现的那个"这种假生效会原样重现。
//! 所以这里用进程级 `Mutex`（值是 `PathBuf`，天然 `Send + Sync`）。
//!
//! ## 三个设置键里为什么只有 JDK 那一个
//!
//! 设置页的三个键是 `javaHomePath` / `mavenExecutablePath` / `mavenJavaHomePath`
//! （键名逐字取自 Core 契约的 toolchain 载荷，`shared/contracts/rust-core-api.md:1625-1627`）。
//! 本侧**只有第一个有消费方**：
//!
//! - `javaHomePath` → JDT LS 的 JVM 用哪个 JDK 起（[`crate::jdtls::resolve_runtime`]）；
//! - `mavenExecutablePath` / `mavenJavaHomePath` → **没有任何地方执行 Maven**：
//!   `maven.scan` 是 Core 进程内的项目描述符解析（不发 `mvn` 子进程），右侧工具窗只呈现
//!   它的结论（`workbench/src/maven.rs` 的模块文档），gpui 侧也没有 `runConfig.*` 那条通路。
//!
//! 所以后两个键**故意不放进这个槽**：登记了没人读只会把"存了不生效"从设置页搬到这一层。
//! 它们仍无消费方这件事如实登记在汇报里；页面上的"尚未生效"标注属于设置 crate 的改动。
//!
//! ## 生效时机
//!
//! 槽在**每次** `resolve_runtime` 都重读，但 JDT LS 会话是"一个工作区一次"
//! （`EditorPane::prepare_java` 幂等，`editor/src/editor_view.rs:389-394`），所以改设置之后，
//! 新值在下一次**语言服务启动**时生效 —— 今天是"重启应用"（设置文件在启动时读一次，
//! `app/src/main.rs`）。运行中的会话不会中途换 JVM（换 JDK 必须重建 JDT 索引，
//! 静默换会让索引与 JVM 版本的对应关系断掉）。

use std::path::PathBuf;
use std::sync::Mutex;

/// 宿主要交给语言服务的 JDK 覆盖值。
///
/// `java_home` 是设置页 `javaHomePath` 的原文转成的路径。这里**不预先规范化**：
/// 页面的语义是"JDK 主目录，也允许直接填 `java` 可执行文件"
/// （`settings/src/project.rs:985-997` 的 `java_executable_in`），
/// 到底能不能用由 [`crate::jdtls::resolve_runtime`] 按同一套判据现场判
/// （它还要跑一次 `java -version` 过 JDT LS 的 ≥ 21 闸门）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaToolchainOverride {
    /// 用户在设置页填的 JDK 主目录（或 `java` 可执行文件）路径。
    pub java_home: PathBuf,
}

// 进程级唯一的那份覆盖值。`None` = 没登记（或登记了空值）⇒ 纯自动发现。
// 写成普通注释而不是 `///`：`static` 的内部文档在这里没有读者（注册函数与读取函数各自有文档）。
static OVERRIDE: Mutex<Option<JavaToolchainOverride>> = Mutex::new(None);

/// 登记（或清除）JDK 覆盖值。由 `ShellWorkspace::new` 在启动时调一次，设置变化时再调。
///
/// 传 `None` 等于"这台机器上没有选覆盖值" ⇒ 语言服务退回自动发现
/// （不是 panic、也不是"用一半"）。设置页的清空按钮走的就是这条。
pub fn set_java_toolchain_override(overridden: Option<JavaToolchainOverride>) {
    // 锁中毒不能变成"设置失效"：这份值只是路径文本，恢复到内部值继续用是安全的
    // （与 `java/src/service.rs` 里 `state` 锁的处理同一口径）。
    let mut slot = OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = overridden;
}

/// 当前登记的 JDK 覆盖值路径；`None` = 没有。每次调用都重读，所以拿到的是最新一次登记。
pub(crate) fn java_home_override() -> Option<PathBuf> {
    let slot = OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slot.as_ref().map(|overridden| overridden.java_home.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 清一次槽：本模块的用例会写全局状态，跑完必须还原成"没登记"，
    /// 否则同一个测试二进制里之后跑的用例会莫名其妙地带上一个覆盖值。
    struct Cleared;

    impl Drop for Cleared {
        fn drop(&mut self) {
            set_java_toolchain_override(None);
        }
    }

    /// 登记 → 读到；清除 → 读不到。**这是"设置里选的值真的传到了 java 侧"的最小证据**，
    /// 也是 `None`（清空/没登记）不被误当成"某个路径"的那条判据。
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
}
