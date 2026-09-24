//! `shell-probe` 二进制目标的根文件，只负责把模块树挂到 crate 根上。
//!
//! 原先约 700 行的单文件外壳已按"界面区域"拆成 `shell_probe/` 下的子模块，
//! 好让多个改动能并行落在不同文件里。真正的入口（`main`、`startup_window_bounds`、
//! 应用启动顺序）在 [`shell_probe`] 的 `mod.rs`；这里把它再导出到 crate 根，
//! 因为 bin 目标的入口点必须解析自 crate 根。
//!
//! 运行：`cargo run --bin shell-probe -- <workspace-root>`

mod shell_probe;

pub use shell_probe::main;
