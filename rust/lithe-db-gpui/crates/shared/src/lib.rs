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
mod document;
// 图标资源路径清单（`lithe-db-gpui/assets/ui-icons/idea/**` 的 Rust 等价物）。**公开模块**而不是
// `pub use`：调用点要写成 `shared::icons::idea::GEAR_ICON`，常量表本身就是一个命名空间，
// 摊平到 crate 根只会让 79 个常量名污染 `shared::*`。理由与再生成命令见 `src/icons/mod.rs`。
pub mod icons;
mod i18n;
pub mod workspace_config;

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
// 目录 `locales/` 相对 `CARGO_MANIFEST_DIR`（= `lithe-db-gpui/crates/shared`），文件是
// `lithe.zh-CN.yml` / `lithe.en.yml`（`_version: 2`），由
// `lithe-db-gpui/tools/extract-locale.mjs` 从 `windows/tauri/src/i18n/locale.ts` 生成。
rust_i18n::i18n!("locales", fallback = "en");

pub use core_client::{CoreClient, CoreError, CoreRequest, DEFAULT_TIMEOUT_MILLIS, core_json};
pub use document::{
    DOCUMENT_VERSION_KEY, Parsed, declared_version, known_keys, merge_document, parse,
    preserve_unknown, previous_object, read_document_text, save_document, save_json, tmp_path,
};
pub use i18n::{tr, tr_args};
