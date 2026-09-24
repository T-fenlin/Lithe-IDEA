//! 跨 Feature 的稳定共享能力（App Shell 架构的 `shared` 层）。
//!
//! 依赖方向：**没有任何 feature 依赖它之外的 feature**；本 crate 反过来不依赖任何 feature。
//! 本 crate 只放"已有清晰名称 + 两个以上真实使用方"的能力，见《编码指南》「大型应用按业务
//! 能力组织 crate」：不为每个 helper 建 crate，也不把只被一处用到的代码提前共享。
//!
//! 当前两个模块：
//!
//! - [`core_client`]：`lithe-core` 的信封层（请求 JSON 拼装、响应解析、错误类型）。
//!   `explorer` 的 `workspace.snapshot` 与 `git` 的 6 条 `git.*` 曾经各抄一份，现已收敛到这里。
//! - [`i18n`]：`rust_i18n::t!` 的包装 + `{name}` 插值（前端文案语法是 `{count}`，
//!   而 `rust-i18n` 4.2 只认 `%{count}`）。
//!
//! 公开边界：`lib.rs` 是唯一的 import 路径，模块本身保持私有
//! （《编码指南》「内部重组时保持 public module path」的反面用法：这里是新建 crate，
//! 所以直接发布 `pub use`，不让 `shared::core_client::CoreRequest` 这类实现路径成为契约）。

mod core_client;
mod i18n;

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
// 目录 `locales/` 相对 `CARGO_MANIFEST_DIR`（= `gpui/crates/shared`），文件是
// `lithe.zh-CN.yml` / `lithe.en.yml`（`_version: 2`），由
// `gpui/tools/extract-locale.mjs` 从 `windows/tauri/src/i18n/locale.ts` 生成。
rust_i18n::i18n!("locales", fallback = "en");

pub use core_client::{CoreClient, CoreError, CoreRequest, DEFAULT_TIMEOUT_MILLIS, core_json};
pub use i18n::{tr, tr_args};
