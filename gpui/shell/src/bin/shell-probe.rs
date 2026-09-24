//! `shell-probe` bin 目标的 crate 根。
//!
//! Rust 2018+ 规则：`shell-probe.rs` 的子模块放在同名的 `shell_probe/` 目录里。
//! bin 目标的入口点必须能从 crate 根解析，所以这里再导出 `main`。
//! `shell_probe::main` 因此必须是 `pub fn`，否则报 `E0603` + `E0601`。

mod shell_probe;

pub use shell_probe::main;

// 国际化：`i18n!` 必须出现在本 crate 的**根**（bin 目标的根就是这个文件）——
// `rust_i18n::t!` 在哪个 crate 里展开，就找那个 crate 的 loader。
//
// 目录 `locales/` 相对 `CARGO_MANIFEST_DIR`（= `gpui/shell`），文件是
// `lithe.zh-CN.yml` / `lithe.en.yml`（`_version: 2`），由
// `gpui/tools/extract-locale.mjs` 从 `windows/tauri/src/i18n/locale.ts` 生成。
// 详见 `gpui/shell/locales/README.md` 与官方文档 `https://gpui-kit.com/zh-CN/docs/i18n.md`。
rust_i18n::i18n!("locales", fallback = "en");
