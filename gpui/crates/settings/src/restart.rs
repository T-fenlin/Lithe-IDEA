//! 重启应用（切换「显示语言」后要用）。
//!
//! ## 为什么语言切换要重启，而不是假装"立即生效"
//!
//! gpui 侧 `set_locale` 只在启动早期调一次，而界面里有**构造期就 `tr()` 过的文案**
//! （活动栏项、状态栏文案），运行中换语言只会有一半文案变、一半保持旧语言 —— 那比"重启后生效"
//! 更容易让人误解。所以「语言」这一项采取**真的重启**：把设置写进文件 → 用**相同参数**重新拉起
//! 自己 → 退出当前进程。这样对用户是"选完语言，界面就是新语言"，且没有任何半生效状态。
//!
//! ⚠️ **本轮未接线**：改「显示语言」只写设置、**不重启**（`dialog.rs` 的语言行里写了原因）。
//! 不接的理由：为一次语言切换杀掉进程，会连带丢掉编辑器未保存内容与终端会话；`PLAN.md` §8
//! 的 v1 口径是"只提示、由用户自己重启"。要改成"选完立即重启"，在 `dialog.rs` 的语言行里补一次
//! [`restart_application`] 调用即可（`store.set_display_language` 已经是立即写，新进程读得到）。
//!
//! ## 重启的实现要点
//!
//! - `std::env::current_exe()` + `std::env::args_os().skip(1)`：**逐字沿用本次启动的参数**
//!   （工作区根、`--theme` / `--locale` 覆盖、`--open-settings` 都要跟着过去，否则"重启后
//!   跑的不是同一个东西"）。
//! - 新进程继承父进程的环境变量，所以 `LITHE_GPUI_SETTINGS_FILE` 也照样生效。
//! - 重启前设置必须**已经落盘**（`SettingsStore::set_display_language` 走立即写，不等 300ms 防抖），
//!   否则新进程读到的还是旧语言。
//! - `App::quit()`（`gpui-pre-0.3.6/src/app.rs:1111`）在当前 effect 周期结束后退出。
//! - 拉起失败**不退出**：报一行诊断，让用户继续用当前语言（语言已经写进文件，下次启动会生效）。

use std::ffi::OsString;
use std::process::Command;

use gpui_kit::App;

/// 用相同参数重新拉起 Lithe；成功后退出当前进程。
///
/// 返回 `true` = 已请求退出（新进程已 spawn）。
pub fn restart_application(cx: &App) -> bool {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            // 拿不到自身路径时不退出：设置已经落盘，下次手动启动就是新语言。
            eprintln!("S1_SETTINGS restart_failed error=current_exe:{error}");
            return false;
        }
    };

    // 参数逐字沿用（工作区根 + 任何覆盖开关）。
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    match Command::new(&exe).args(&args).spawn() {
        Ok(child) => {
            println!(
                "S1_SETTINGS restarting pid={} exe={} args={}",
                child.id(),
                exe.display(),
                args.len()
            );
            cx.quit();
            true
        }
        Err(error) => {
            eprintln!(
                "S1_SETTINGS restart_failed exe={} error={error}",
                exe.display()
            );
            false
        }
    }
}
