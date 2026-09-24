//! 国际化包装：`rust_i18n::t!` + `{name}` 插值。
//!
//! ## 为什么需要这一层
//!
//! 项目的文案真源是 `windows/tauri/src/i18n/locale.ts`（经 `gpui/tools/extract-locale.mjs`
//! 生成 `crates/app/locales/lithe.{zh-CN,en}.yml`），它沿用前端语法：**`{count} 个空格`**
//! （`lithe.footer.spaces`）。
//!
//! 而 `rust-i18n` 4.2.2 的 `t!` 只把 **`%{...}`** 当占位符
//! （`rust-i18n-4.2.2/src/lib.rs:45-91` 的 `replace_with_args`），不认 `{...}`。
//! 直接用 `t!` + `args!` 会原样输出 `{count} 个空格`。
//!
//! 所以这一层做两件事：**先 `t!`，再按 `{name}` 替换**。locale 数据与前端保持逐字一致
//! （不为了迁就 `rust-i18n` 去改写生成出来的 yml —— 那份文件是自动生成的，改了会被覆盖）。
//!
//! ## locale loader 的位置约束（本仓库实测，决定了 `locales/` 放在本 crate）
//!
//! `rust_i18n::t!` 展开成 `crate::_rust_i18n_try_translate(..)`
//! （`rust-i18n-macro-4.2.2/src/tr.rs:438,454`），而 `crate::_rust_i18n_try_translate`
//! 由 `i18n!` 在**调用它的那个 crate** 里生成（`rust-i18n-4.2.2/src/lib.rs:402`）。
//! 所以 `t!` 只能在自己调过 `i18n!` 的 crate 里解析 —— 在别的 crate 里写
//! `rust_i18n::t!(..)` 会报 `E0433: cannot find _rust_i18n_t in crate`（本轮实测）。
//!
//! 本仓库要求 `tr` / `tr_args` 是 `shared` 对外的稳定包装，所以 `i18n!` 与 `locales/`
//! 一并落在 `shared`（`src/lib.rs`），`crates/app` 只组合窗口与 Feature。
//!
//! 附带结果：`rust-i18n` 的 `i18n!` 只会执行一次（`_RUST_I18N_BACKEND` 是 crate 内的
//! `LazyLock`），所以**不要**在 `app` 里再调一次 `i18n!` —— 那会生成第二份 backend，
//! 变成两个真相源。
//!
//! ## 返回值为什么是 `SharedString`
//!
//! 调用点绝大多数是 GPUI element 的 `.child(..)` / `.text(..)`，它们要 `IntoElement`，
//! 而 `String` 不是 `IntoElement`。这里直接给 `SharedString`，省掉每个调用点一次 `.into()`。

use gpui_kit::SharedString;

/// 取一条文案（`key` 形如 `lithe.footer.spaces`），不带插值。
pub fn tr(key: &str) -> SharedString {
    SharedString::from(rust_i18n::t!(key).to_string())
}

/// 取一条文案并替换 `{name}` 占位符。
///
/// ```ignore
/// // lithe.spaces.count = "{count} 个空格"
/// let label = tr_args("lithe.footer.spaces", &[("count", "4")]);
/// ```
///
/// 参数是 `&[(&str, &str)]`（名字 + 已格式化好的值）：locale 数据里的占位符沿用前端语法，
/// 而"数字怎么格式化"是调用方的事，本层不做复数 / 数字格式的推断。
pub fn tr_args(key: &str, args: &[(&str, &str)]) -> SharedString {
    let template = rust_i18n::t!(key).to_string();
    SharedString::from(interpolate(&template, args))
}

/// 把模板里的 `{name}` 逐个替换成对应值；没有匹配的占位符原样保留。
///
/// 纯函数（不碰全局 locale 表），所以可以直接单测 —— 见文件末尾的测试。
/// 顺序替换而不是一次扫描：参数个数是个位数，这样读起来最直白。
fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut text = template.to_string();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::interpolate;

    /// `{name}` 必须被替换：这是本层存在的唯一理由
    /// （`rust-i18n` 只认 `%{name}`，直接 `t!` 会漏出花括号原文）。
    #[test]
    fn replaces_brace_placeholders() {
        assert_eq!(interpolate("{count} 个空格", &[("count", "4")]), "4 个空格");
    }

    /// 多个占位符、以及同一个占位符出现两次都要替换干净。
    #[test]
    fn replaces_every_occurrence() {
        assert_eq!(
            interpolate("{a}-{b}-{a}", &[("a", "x"), ("b", "y")]),
            "x-y-x"
        );
    }

    /// 模板里没有的占位符原样保留：漏配参数时应当看得见，而不是被静默清空。
    #[test]
    fn keeps_unknown_placeholders() {
        assert_eq!(interpolate("{missing}", &[("count", "4")]), "{missing}");
    }
}
