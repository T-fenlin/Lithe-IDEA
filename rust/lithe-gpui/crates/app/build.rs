//! 把 `lithe-gpui/assets/icons/icon.ico` 以资源 ID **1** 嵌进 `Lithe.exe`。
//!
//! Windows 上给窗口/任务栏设图标**唯一**可行的路就是嵌 exe 资源：gpui 的 Windows 后端
//! 用 `LoadImageW(GetModuleHandleW(None), PCWSTR(1), IMAGE_ICON, ...)` 取图标，而
//! `WindowOptions.icon` 是 X11-only。详细证据与被否方案见 `lithe.rc` 的注释和
//! `lithe-gpui/research/app-icon-and-assets.md`。
//!
//! 平台门控：只有 Windows 目标才声明 `embed-resource` 这个 build-dependency
//! （见 `Cargo.toml` 的 `[target.'cfg(windows)'.build-dependencies]`），所以非 Windows
//! 上这里没有任何活要干 —— 图标属于 Windows 的 exe 资源模型，macOS 走 `.app` bundle。

#[cfg(windows)]
fn main() {
    // `.rc` 本身与它引用的图标都要参与增量判断：换图标必须重新链接 exe。
    println!("cargo:rerun-if-changed=lithe.rc");
    println!("cargo:rerun-if-changed=../../assets/icons/icon.ico");

    // `manifest_optional()`：图标是装饰性资源，rc.exe "没尝试/非 Windows" 都算成功；
    // 只有**真的编译失败**才返回 `Err`。这种情况下必须让构建失败 —— 否则会静默产出一个
    // 没有图标的 exe，而"没图标"这件事在界面上一眼看不出来（窗口无边框，客户端里根本
    // 看不到窗口图标，只能去看任务栏）。
    embed_resource::compile("lithe.rc", embed_resource::NONE)
        .manifest_optional()
        // `CompilationResult` 没有实现 `Display`，这里转成字符串只为让 `expect` 能打印原因。
        .map_err(|reason| format!("{reason:?}"))
        .expect("嵌入 Windows 图标资源失败：找不到可用的 rc.exe，或 lithe.rc 编译不过");
}

#[cfg(not(windows))]
fn main() {}
