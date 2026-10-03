//! 跨 Feature 的稳定共享能力（App Shell 架构的 `shared` 层）。
//!
//! 依赖方向：**没有任何 feature 依赖它之外的 feature**；本 crate 反过来不依赖任何 feature。
//! 本 crate 只放"已有清晰名称 + 两个以上真实使用方"的能力，见《编码指南》「大型应用按业务
//! 能力组织 crate」：不为每个 helper 建 crate，也不把只被一处用到的代码提前共享。
//!
//! 当前模块：
//!
//! - [`core_client`]：`lithe-core` 的信封层（请求 JSON 拼装、响应解析、错误类型）。
//!   `explorer` 的 `workspace.snapshot` 与 `git` 的 6 条 `git.*` 曾经各抄一份，现已收敛到这里。
//! - [`i18n`]：`rust_i18n::t!` 的包装 + `{name}` 插值（前端文案语法是 `{count}`，
//!   而 `rust-i18n` 4.2 只认 `%{count}`）。
//! - [`rem_px`]：规格像素 → `Pixels` 的换算（只在固有方法只吃 `Pixels` 时用）。
//! - `design`：跨 Feature 的**设计规格常量**的共同 owner（同一个设计量不再让每个视图各写一份
//!   字面相同的数字）。**公开模块**：常量表本身就是一个命名空间（同 `icons` 的理由）。
//! - `document`：版本化 JSON 文档的通用原语（解析、未知键保留、原子写）。由设置文档与
//!   `.lithe/project.json` 两个真实使用方共享，**原样 `pub use` 到 crate 根**。
//! - `workspace_config`：`.lithe/` 的路径真源、项目身份、"默认不共享"守卫。
//!   **公开模块**（同 `icons` 的理由：`paths` 的成员表本身就是一个命名空间，
//!   摊平到 crate 根只会让十几个路径构造函数污染 `shared::*`）。
//!
//! 公开边界：`lib.rs` 是唯一的 import 路径，模块本身保持私有
//! （《编码指南》「内部重组时保持 public module path」的反面用法：这里是新建 crate，
//! 所以直接发布 `pub use`，不让 `shared::core_client::CoreRequest` 这类实现路径成为契约）。

mod core_client;
// 跨 Feature 的设计规格常量（同一个设计量只定义一次）。**公开模块**而不是 `pub use`：
// 调用点要写成 `lithe_gpui_shared::design::ROW_RADIUS`，常量表是命名空间，摊平到 crate 根
// 只会让 `ROW_RADIUS` / `TAB_MAX_WIDTH` 这类短名污染 `shared::*`。理由见 `src/design.rs`。
pub mod design;
mod document;
mod i18n;
// 图标资源路径清单（`lithe-gpui/assets/icons/idea/icons/**` 的 Rust 等价物）。**公开模块**而不是
// `pub use`：调用点要写成 `shared::icons::idea::GEAR_ICON`，常量表本身就是一个命名空间，
// 摊平到 crate 根只会让 79 个常量名污染 `shared::*`。理由与再生成命令见 `src/icons/mod.rs`。
pub mod icons;
// 语言高亮的启用状态（被卸载的语言扩展 → 这些语言不高亮）。
pub mod languages;
pub mod workspace_config;

// `rem_px` 用 `AbsoluteLength` / `rems` / `Pixels` 这三个类型，所以这里引 gpui-kit。
// 与 `i18n` 只取 `SharedString` 同一条口径：本 crate 不做任何 GPUI 渲染。
use gpui_kit::{AbsoluteLength, Pixels, rems};

// 国际化：**`i18n!` 必须出现在持有 `locales/` 的那个 crate 的根**。
//
// 为什么是 `shared` 而不是 `app`（`crates/app/src/main.rs`）：`rust_i18n::t!` 展开成
// `crate::_rust_i18n_try_translate(..)`（`rust-i18n-macro-4.2.2/src/tr.rs:438,454`），
// 而 `crate::_rust_i18n_try_translate` 由 `i18n!` 在**调用它的那个 crate** 里生成
// （`/src/lib.rs:402`）。也就是说 `t!` 只能在自己调过 `i18n!` 的 crate 里解析。
//
// 本仓库要的包装函数（先 `t!` 再按 `{name}` 替换）必须和 `t!` 同 crate，所以 locale 文件
// 与 loader 一并落在 `shared`。`crates/app/src/main.rs` 只做组合窗口与 Feature，不再持有
// 文案资源；这是对"locales 放 app/ 下"这一条布局的有意偏离，理由与验证记录在本 crate 的
// `src/i18n.rs` 模块文档里。
//
// 目录 `locales/` 相对 `CARGO_MANIFEST_DIR`（= `lithe-gpui/crates/shared`），文件是
// `lithe.zh-CN.yml` / `lithe.en.yml`（`_version: 2`），由
// `lithe-gpui/tools/extract-locale.mjs` 从 `crates/shared/locales/source/locale.ts` 生成。
rust_i18n::i18n!("locales", fallback = "en");

pub use core_client::{CoreClient, CoreError, CoreRequest, DEFAULT_TIMEOUT_MILLIS, core_json};
pub use document::{
    DOCUMENT_VERSION_KEY, Parsed, declared_version, known_keys, merge_document, parse,
    preserve_unknown, previous_object, read_document_text, save_document, save_json, tmp_path,
};
pub use i18n::{tr, tr_args};

/// 宿主版本（= 本 crate 的 `CARGO_PKG_VERSION`，全 gpui crate 版本一致发布）。
///
/// `plugins.listCatalog` 用它做目录兼容性校验：`shared/fixtures/plugins/official-v2.json`
/// 的 `hostVersion` 必须与这里保持一致，否则 Core 会以 `parse_failed` 拒绝整个目录。
pub const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 把**规格像素值**（规格文档里写的 px）按**运行时 rem 基准**求值成 `Pixels`。
///
/// ## 什么时候才走这里
///
/// 这是 `rems(P / 16.)` 的 `Pixels` 形式，**只在调用点必须交出 `Pixels`** 时用。
/// 原因是有两类槽吃不到 rem：
///
/// - 组件的**固有方法**会静默遮蔽 `Styled` 上的同名方法 —— `Button::rounded` 收
///   `impl Into<ButtonRounded>`、`Button`/`Input` 的 `Sizable::with_size` 收 `impl Into<Size>`，
///   而 `ButtonRounded` 与 `Size` **各自只实现了 `From<Pixels>`**，没有 `From<Rems>`
///   （`gpui-component-0.7.0/src/button/button.rs:29`、`:334` 与同版本 `src/sizing.rs:169`、`:183`）。
///   在这些地方写 `rems(..)` 会得到 `error[E0277]: the trait bound ... From<Rems> is not satisfied`。
/// - 少数 API 的签名直接就是 `Pixels`（例如 `ScrollWheelEvent::delta.pixel_delta(...)`、
///   `Dialog::width` / `margin_top`、`PopupMenu::min_w` / `max_w` / `max_h`、
///   `Positioner::offset` / `margin`）。
///
/// 能吃 `AbsoluteLength` / `Length` 的调用点（`Styled::w` / `h` / `rounded` / `margin_top` /
/// `text_size` / `line_height` …）**直接写 `rems(P / 16.)`**，由布局期按窗口 rem 求值，
/// 不必经过这里 —— 多绕一层只会多一次手写换算。
///
/// ## 两个容易写错的点
///
/// - `spec_px` 是**规格像素值**，不是 rem 数。`rem` base = 16px，所以除的是 **`16.`**：
///   档位 helper 的后缀 `N` 表示 `N × 0.25rem`（即 4px 一档），**不要照着写 `/ 4.`**。
/// - `rem` 必须是**运行时基准**（`window.rem_size()`，或等价的 `cx.theme().font_size`；
///   `Root::render` 每帧把它写进 `window.set_rem_size`）。写成 `to_pixels(px(16.))` 就是
///   **假 rem**：看起来合规，实际把基准写死，界面字号一放大就不再跟着缩放。
///
/// 本转换是确定性的（同样的 `rem` 与 `spec_px` 必然得到同一个 `Pixels`），但因为以 rem
/// 基准为入参、且只服务界面层，所以留在宿主侧的 `shared`，不进 `lithe-core`。
///
/// 收敛历史：这段换算原先在 `editor` / `git` / `settings` / `terminal` / `workbench`
/// 五个 crate 里各有一份逐字相同的实现（+ 三份重复单测），任何一处改口径都会分叉；
/// 现已收敛到 `shared` 这一份，各处只 `pub(crate) use` 转发。
pub fn rem_px(rem: Pixels, spec_px: f32) -> Pixels {
    AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)
}

#[cfg(test)]
mod tests {
    use super::rem_px;
    use gpui_kit::px;

    /// [`rem_px`] 必须**随传入的运行时 rem 基准缩放**，而不是把 16px 写死。
    ///
    /// 只断言"16px 基准下等于规格值"证明不了这一点 —— 那正是「假 rem」
    /// （`AbsoluteLength::from(rems(P / 16.)).to_pixels(px(16.))`）也会通过的断言。
    /// 所以这里额外钉住"基准翻倍 ⇒ 结果翻倍"。
    #[test]
    fn rem_px_scales_with_the_runtime_base() {
        assert_eq!(rem_px(px(16.), 26.), px(26.));
        // 基准翻倍 ⇒ 结果翻倍。
        assert_eq!(rem_px(px(32.), 26.), px(52.));
        // 20px 基准（125% DPI 的常见值）下是 20 / 16 倍。
        assert_eq!(rem_px(px(20.), 32.), px(40.));
        // 规格值可以不是整数（圆角这类由 `--radius × k` 推出的值）。
        assert_eq!(rem_px(px(16.), 1.6), px(1.6));
        // 大值同样按比例（标签/面板最大宽度这类规格值）。
        assert_eq!(rem_px(px(32.), 200.), px(400.));
    }
}
