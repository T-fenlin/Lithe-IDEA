# GPUI Kit 0.6.6「应用图标 + 资源系统」研究笔记

> 范围：只依据**本机已解压的已发布源码**（cargo registry）与**仓库内 0.6.6 文档镜像**。
> 证据规则：每条结论都带 `文件:行号`。凡是没在源码里核到的，一律写「未验证」并说明原因，不推测。
> 本笔记**不改任何代码**，只回答「怎么接线」。
>
> 路径简写（下文都用简写）：
> - `<REG>` = `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837`
> - `<REPO>` = `D:\developmentProjects\rust\Lithe-IDEA`
> - `<DOCS>` = `<REPO>\gpui\docs\gpui-kit\0.6.6\zh-CN`（151 页镜像，真源是 **0.6.6 默认**，不是 `versions/main`）

---

## 0. 前置事实：版本链、源码在哪、以及一条过期的任务前提

### 0.1 版本链（决定「gpui 本体」到底是哪个 crate）

| 层 | crate（包名） | 版本 | lib 名 | 证据 |
| --- | --- | --- | --- | --- |
| facade | `gpui-kit` | 0.6.6 | `gpui_kit` | `<REG>\gpui-kit-0.6.6\Cargo.toml:14-15,215-217` |
| GPUI 本体 | **`gpui-pre`**（**被重命名成 `gpui`**） | 0.3.6 | `gpui` | `<REG>\gpui-kit-0.6.6\Cargo.toml:354-356`（`[dependencies.gpui] version = "=0.3.6" package = "gpui-pre"`）、`<REG>\gpui-pre-0.3.6\Cargo.toml:82-83` |
| 平台后端 | `gpui-pre-windows` / `gpui-pre-platform` | 0.3.6 | `gpui_windows` / `gpui_platform` | `<REG>\gpui-kit-0.6.6\Cargo.toml:374-382`、`<REG>\gpui-pre-platform-0.3.6\src\gpui_platform.rs:63-69` |
| 组件层 | `gpui-component` / `gpui-base` | 0.6.6 | — | `<REG>\gpui-kit-0.6.6\Cargo.toml:358-363` |
| 资源层 | `gpui-kit-assets` | 0.6.6 | `gpui_kit_assets` | `<REG>\gpui-kit-0.6.6\Cargo.toml:365-367`、`<REG>\gpui-kit-assets-0.6.6\src\lib.rs:36` |

⚠️ 关键：**`use gpui::X` 里的 `gpui` 就是 `gpui-pre` 0.3.6**，本地 registry 目录名是 `gpui-pre-0.3.6`。搜索源码时不要找 `gpui-0.x`（不存在这个目录名）。
Lite 侧的锁定结果：`<REPO>\gpui\Cargo.lock` 里 `gpui-kit 0.6.6` / `gpui-pre 0.3.6` / `gpui-pre-windows 0.3.6` / `image 0.25.10` / `rust-embed 8.12.0` / `raw-window-handle 0.6.2` / `windows 0.62.2` / `embed-resource 3.0.11` 全部已在锁文件中。

**`gpui-pre-macos` / `gpui-pre-linux` 在本机 registry 里不存在**（列目录只看到 `gpui-pre-0.3.6`、`gpui-pre-windows-0.3.6`、`gpui-pre-platform-0.3.6` 等 60 个 `gpui-pre-*`，没有 macos/linux 后端）——因为本机只构建过 Windows 目标。所以 macOS / X11 的后端行为**无法在本机核对**，见 §7。

### 0.2 一条过期前提：`gpui/` 里**已经**有图片资源了

任务描述说「`gpui/` 目录里零个图片文件」——**这条已过期**。当前工作树里（分支 `feat/gpui-shell-rewrite`，工作树干净）已有一个提交 `9d9bcdaa chore(gpui): 把应用图标与品牌 logo 提取到 gpui/assets`，落了 8 个文件：

| 文件 | 字节 | 用途（据 `gpui/assets/README.md:29-33`） |
| --- | ---: | --- |
| `<REPO>\gpui\assets\icons\icon.ico` | 102 639 | exe 图标（PE 资源） |
| `<REPO>\gpui\assets\icons\32x32.png` | 2 219 | Windows 窗口与任务栏图标 |
| `<REPO>\gpui\assets\icons\64x64.png` | 5 917 | 同源导出备用 |
| `<REPO>\gpui\assets\icons\128x128.png` | 17 207 | 同源导出备用 |
| `<REPO>\gpui\assets\icons\128x128@2x.png` | 62 254 | 同源导出备用 |
| `<REPO>\gpui\assets\icons\icon.png` | 265 429 | macOS 打包备用 |
| `<REPO>\gpui\assets\icons\icon.icns` | 1 659 491 | macOS 打包图标 |
| `<REPO>\gpui\assets\images\logo.png` | 810 582 | 界面里的品牌 logo（欢迎页 / 标题栏项目菜单） |

`<REPO>\gpui\assets\README.md:9-11` 自己写着：「**本目录当前还没有任何 Rust/TS 代码引用**……接线（窗口图标、欢迎页 logo、标题栏项目菜单 logo）由后续任务完成」。
所以本任务的第一句话精确表述是：**图标资源已就位，缺的是接线代码**（原先的 `windows/tauri/**` 仍保留同一份副本，可作对照）。

`icon.ico` 的实际内容（我读了 ICO 目录，8 张图，全部 32bpp）：
16×16 / 20×20 / 24×24 / 32×32 / 48×48 / 64×64 / 128×128 / 256×256。
这一条很重要：旧前端的注释明确说过「Windows 11 任务栏会把**只有 256** 的 ICO 缩成一个空胶囊」，所以必须带小尺寸（`<REPO>\windows\tauri\src-tauri\src\host.rs:14-16`）——现有 `icon.ico` 满足该要求。

---

## 1. 结论速览

| 目标 | 推荐做法 | 一句话理由 | 风险 / 代价 |
| --- | --- | --- | --- |
| **Windows 任务栏 / 窗口图标 / 标题栏图标** | **build script 把 `gpui/assets/icons/icon.ico` 以资源 ID `1` 嵌进 `Lithe.exe`**（`embed-resource` 或 `winresource`，见 §5） | gpui 的 Windows 后端给窗口类设图标时，读的就是**主 exe 模块里的资源 ID 1**（`load_icon()`，`<REG>\gpui-pre-windows-0.3.6\src\platform.rs:1476-1490`），而且**只读这一个入口**；`WindowOptions.icon` 在 Windows 后端根本没被读 | 要加 `build.rs` + build-dependency（`embed-resource` 已在 lock，`winresource` 要新入 lock）；依赖 Windows SDK 的 `rc.exe`（本机有，见 §5.4）；资源 ID 必须是 1，用 tauri 那套默认 ID（32512）**不会生效** |
| **Windows 资源管理器里的 exe 图标** | 同上（同一个 ICO 资源即可同时满足） | Explorer 取 exe 里编号最小的 `RT_GROUP_ICON` | 同上 |
| **Linux/X11 窗口图标** | `WindowOptions { icon: Some(Arc<image::RgbaImage>), .. }` | 字段存在且注释写明是 X11 用的 | 需要 app 自己加 `image` 依赖解码 PNG（gpui 不重导出 `image`） |
| **macOS Dock / 应用图标** | 预期走 `.app` bundle 的 `icon.icns` + `Info.plist`（`CFBundleIconFile`） | `WindowOptions.icon` 注释是 X11 only，macOS 后端源码本机不可得 | **未验证**（本机无 `gpui-pre-macos`） |
| **界面里的品牌 logo（欢迎页 / 标题栏）** | `img("lithe/logo.png")` + 自定义 `AssetSource`（或直接 `include_bytes!`） | `AssetSource` 只服务 `img()`/`Icon`，**与窗口图标完全无关** | 见 §4：`with_assets` 只收**一个** source，必须写 wrapper（官方文档 `assets.md:103-136` 就是这段） |

**风险总览**：①「资源 ID 必须是 1」是本任务最容易踩的坑（Tauri 的约定是 32512，见 §5.5）；②`gpui-pre` 已经往 exe 里嵌了一份 `RT_MANIFEST`（资源 ID 1，见 §5.2），所以自己的 `.rc` **绝不能再写 manifest**；③「两份 `.res` 不冲突」是推断（不同类型同 ID），没有实际链接验证（§7）。

---

## 2. `WindowOptions.icon` 的确切事实（源码证据）

### 2.1 字段存在，类型是 `Option<Arc<image::RgbaImage>>`，注释写「X11 only」

```rust
// <REG>\gpui-pre-0.3.6\src\platform.rs:2172-2243（节选）
2172: /// The variables that can be configured when creating a new window
2173: #[derive(Debug)]
2174: pub struct WindowOptions {
...
2238:     /// Icon image (X11 only)
2239:     pub icon: Option<Arc<image::RgbaImage>>,
2241:     /// Tab group name, ...
2242:     pub tabbing_identifier: Option<String>,
2243: }
```

- 类型是 **`Option<Arc<image::RgbaImage>>`**（`Arc` 来自 `std::sync::Arc`，见 `platform.rs:70-77` 的 `sync::Arc` 导入）——**不是** `Option<Arc<RenderImage>>`、也不是 `gpui::Image`。`RenderImage` 是另一个东西（`<REG>\gpui-pre-0.3.6\src\assets.rs:43-49`，BGRA、给 `img()` 渲染用）。
- 注释直译：「Icon image (**X11 only**)」。
- **没有 `#[non_exhaustive]`**：我在 `platform.rs` 全文 grep `non_exhaustive` **零命中**；结构体只有 `#[derive(Debug)]`（2173）。所以**结构体字面量 + `..Default::default()` 完全合法**，gpui 自己的例子就是这么写的。
- `WindowOptions` **没有** `Clone`/`Copy`（只 derive `Debug`），`Default` 是**手写 impl**（`platform.rs:2345-2371`），其中 `icon: None`（`platform.rs:2364`）。想复用一份基础配置只能「每次调用一个返回 `WindowOptions` 的函数」，不能 `.clone()`。

### 2.2 两种写法（都可用）

```rust
// 写法 A：从 Default 出发（gpui 官方例子：<REG>\gpui-pre-0.3.6\examples\hello_world.rs:101-105）
let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    icon: None,                       // 想设就换成 Some(Arc::new(rgba))
    ..Default::default()
};
```

```rust
// 写法 B：从 gpui-component 的 TitleBar 基座出发
//   （<REG>\gpui-component-0.6.6\src\title_bar.rs:67-91 提供 window_options()；
//     Lite 现在就是这一种：<REPO>\gpui\crates\app\src\main.rs:209-213）
use gpui_kit::component::TitleBar;
use gpui_kit::{px, size, WindowOptions};

let window_options = WindowOptions {
    window_bounds: Some(bounds),
    window_min_size: Some(size(px(1024.), px(680.))),
    icon: load_window_icon(),          // 新增这一行即可，字段名就是 `icon`
    ..TitleBar::window_options()
};
```

`TitleBar::window_options()` 的实现也证明「字面量 + `..Default::default()`」是官方承认的构造方式：
`<REG>\gpui-component-0.6.6\src\title_bar.rs:81-91`（`WindowOptions { titlebar: Some(..), app_owns_titlebar_drag: true, ..Default::default() }`）。
文档侧同一口径见 `<DOCS>\component\title-bar.md:92-118`。

### 2.3 这个值怎么流到平台后端，以及**为什么在 Windows 上填了没用**

```
WindowOptions.icon
  → <REG>\gpui-pre-0.3.6\src\window.rs:1519-1542（destructure WindowOptions，icon 在 1539）
  → <REG>\gpui-pre-0.3.6\src\window.rs:1564（塞进 WindowParams { ..., icon }）
  → <REG>\gpui-pre-0.3.6\src\platform.rs:2294-2296（WindowParams.icon，注释同样写 "x11 only"）
  → cx.platform.open_window(...)（window.rs:1549-1568）→ 平台后端
```

平台后端的核对结果：

- **Windows 后端完全不读 `params.icon`。** 在 `<REG>\gpui-pre-windows-0.3.6` 全 crate grep `icon|Icon` 共 26 处命中，逐条看都是别的东西：`GetModuleHandleW` 找的那套 `HICON`（下文 §5）、`IsIconic`（最小化判断）、任务对话框的 `TD_*_ICON`、以及资源管理器快捷方式。**没有任何一处引用 `params.icon`**。
- Windows 后端的窗口类图标来自**另一条路**：平台初始化时从 exe 资源加载（§5.1）。
- `WindowParams.icon` 上的 `dead_code` 抑制也只在 wayland 下加（`platform.rs:2294-2296` 的 `#[cfg_attr(feature = "wayland", allow(dead_code))]`），而 Windows 目标下 `focus` 之类字段反而显式标了 `target_os = "windows"`（`platform.rs:2285-2293`）——这进一步说明 `icon` 在 Windows 上不是被读的字段。
- Linux/X11：`gpui-pre-linux` 本机不存在，无法核对实现，只能采信注释（**未验证**）。
- macOS：同上，**未验证**（见 §7）。

**结论（对 Lithe 而言）**：`WindowOptions.icon` 在 Windows 上是一个**语法合法、语义无效**的字段。填了不会有编译错误、不会有运行时错误、也不会有任何视觉变化。

---

## 3. 一张 PNG 怎么变成 `Option<Arc<image::RgbaImage>>`

### 3.1 先排除两条「gpui 自带解码」的猜想

| 猜想 | 事实 | 证据 |
| --- | --- | --- |
| gpui 重导出了 `image`，可以直接 `gpui::image::...` | **没有**。`gpui-pre` 的 `src/` 里 `pub use image` / `extern crate image` **零命中**；`src/gpui.rs:89-168` 的 re-export 清单里也没有 `image`。`image` 只在内部被 `use image::...` 私有引用（如 `platform.rs:54-56`） | `<REG>\gpui-pre-0.3.6\src\gpui.rs:89-168`、`src\platform.rs:54-56` |
| 有 `gpui::Image::from_bytes()`，能直接当窗口图标 | `gpui::Image` 只是 **`{ format, bytes, id }` 的字节容器**（`platform.rs:2902-2911`），`Image::from_bytes(format, bytes)`（`platform.rs:2953-2959`）**不产生 `RgbaImage`**；它只能经 `use_render_image` / `get_render_image` 变成 `Arc<RenderImage>`（`platform.rs:2966-2986`），而 `RenderImage` **不是** `icon` 字段要的类型 | `<REG>\gpui-pre-0.3.6\src\platform.rs:2902-2911,2946-2986` |
| gpui 内部的解码函数可以复用 | `decode_static_image` / `decode_static_image_from_decoder` 是 **`pub(crate)`**（`platform.rs:2913`、`2923`），应用拿不到；而且它把 RGBA 的 R/B 交换成了 **BGRA**（`platform.rs:2932-2935`），就算拿得到也不是 `RgbaImage` 要的通道序 | `<REG>\gpui-pre-0.3.6\src\platform.rs:2913-2938` |

**所以：要用 `WindowOptions.icon`，必须由应用自己引入 `image` crate 解码 PNG。**

### 3.2 版本对齐（会不会类型不匹配？）

`icon` 的类型里写的是 gpui 视角下的 `image::RgbaImage`，即 `image 0.25.x` 的 `RgbaImage`（`<REG>\gpui-pre-0.3.6\Cargo.toml:342-361`：`version = "0.25.1"`，`features = [..., "png", ...]`，`default-features = false`）。
Lite 的 `gpui/Cargo.lock` 已经把 `image` 锁到 **0.25.10**，所以应用只要声明 `image = { version = "0.25", default-features = false, features = ["png"] }` 就会**统一到同一个 crate 实例**，类型完全一致。
**反例（会编译不过）**：应用声明 `image = "0.24"` 或 `image = "0.26"` → 两个不同版本的 `RgbaImage` 是两个不同类型。

### 3.3 最小可编译片段

```toml
# <REPO>\gpui\crates\app\Cargo.toml（新增；版本与 gpui-pre 的 image 0.25.1 归一到 lock 里的 0.25.10）
[dependencies]
image = { version = "0.25", default-features = false, features = ["png"] }
```

```rust
// <REPO>\gpui\crates\app\src\main.rs 片段
use std::sync::Arc;

use gpui_kit::component::TitleBar;
use gpui_kit::{WindowOptions, px, size};

/// 把编译期内嵌的 PNG 解码成 `WindowOptions.icon` 要的那个类型。
///
/// - `include_bytes!` 的路径是**相对当前源文件**的：本文件在 `gpui/crates/app/src/`，
///   资源在 `gpui/assets/`，所以是 `../../assets/...`。
/// - `image::load_from_memory_with_format` 见 `<REG>\image-0.25.10\src\images\dynimage.rs:1691`；
///   `DynamicImage::to_rgba8` 在同文件 `:304`。
/// - ⚠️ 必须是 **RGBA**（`to_rgba8()`），不能照抄 gpui 内部的 BGRA 交换（`platform.rs:2932-2935`）。
fn load_window_icon() -> Option<Arc<image::RgbaImage>> {
    const PNG: &[u8] = include_bytes!("../../assets/icons/32x32.png");
    let decoded = image::load_from_memory_with_format(PNG, image::ImageFormat::Png)
        .expect("assets/icons/32x32.png 必须可解码");
    Some(Arc::new(decoded.to_rgba8()))
}

fn window_options(bounds: gpui_kit::WindowBounds) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(bounds),
        window_min_size: Some(size(px(1024.), px(680.))),
        icon: load_window_icon(), // ← 只对 X11 有意义；Windows 上见 §2.3
        ..TitleBar::window_options()
    }
}
```

若不想引入 `image`（例如只想设纯色/占位图标），也可以直接造：

```rust
// 32×32 全不透明红：RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>
let raw = vec![255u8, 0, 0, 255].repeat(32 * 32);
let rgba = image::RgbaImage::from_raw(32, 32, raw).expect("长度必须 = w*h*4");
let icon = Some(std::sync::Arc::new(rgba));
```

**直接可用的入口**：`<REG>\gpui-pre-0.3.6\examples\image_loading.rs:37` 就是「编译期内嵌一张 PNG」的官方写法（`concat!(env!("CARGO_MANIFEST_DIR"), "/examples/image/app-icon.png")`），可作为路径写法参考。

---

## 4. 资源系统：`AssetSource` 全貌 + 能否叠加自定义资源目录

### 4.1 trait 的**完整方法签名**（0.6.6）

```rust
// <REG>\gpui-pre-0.3.6\src\assets.rs:12-19
12: /// A source of assets for this app to use.
13: pub trait AssetSource: 'static + Send + Sync {
14:     /// Load the given asset from the source path.
15:     fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>>;
16:
17:     /// List the assets at the given path.
18:     fn list(&self, path: &str) -> Result<Vec<SharedString>>;
19: }
```

- 只有 **2 个方法**，都有默认实现吗？**没有默认实现**，两者都必须写。
- `Result` 是 gpui 的 `anyhow::Result`（`<REG>\gpui-pre-0.3.6\src\gpui.rs:93`：`pub use anyhow::Result;`）。
- 给 `()` 的实现是**真实现**（返回「没有资源」），不是桩：`<REG>\gpui-pre-0.3.6\src\assets.rs:21-29`（`load` → `Ok(None)`，`list` → `Ok(vec![])`）。
- **0.6.6 里没有任何 `unimplemented!()` / `todo!()` 的 AssetSource 实现**：`gpui-kit-0.6.6` / `gpui-kit-assets-0.6.6` / `gpui-base-0.6.6` 三个 crate 全量 grep `unimplemented!|todo!(|unreachable!` 只命中 3 处 `unreachable!`，全在 dock 布局树与 markdown 解析里，**与 assets 无关**。
- `AssetSource` 在本机可见源码里的消费者有 3 类：SVG 渲染器（`<REG>\gpui-pre-0.3.6\src\svg_renderer.rs:93,124,257,309-315`）、`img()` 元素（`<REG>\gpui-pre-0.3.6\src\elements\img.rs:631,658`，`cx.asset_source().load(&path)`）、以及 gpui-component 的**原生菜单图标**（`<REG>\gpui-component-0.6.6\src\native_menu\mod.rs:32,229`、`native_menu\windows.rs:44,90`、`native_menu\macos.rs:55,91`）。
  ⚠️ 这意味着：一旦换成自定义 wrapper（§4.4），**菜单图标也走你的 `load`**，fallback 必须正确覆盖 `icons/**` 路径。
  `gpui-base` 完全不碰资源：`<REG>\gpui-base-0.6.6\src` 全文 grep `AssetSource|with_assets|WindowOptions|WindowParams` **零命中**。
  **`AssetSource` 完全不参与窗口/任务栏/exe 图标。**

### 4.2 `with_assets` 接受的类型

```rust
// <REG>\gpui-pre-0.3.6\src\app.rs:198-206
198:     /// Assigns the source of assets for the application.
199:     pub fn with_assets(self, asset_source: impl AssetSource) -> Self {
200:         let mut context_lock = self.0.borrow_mut();
201:         let asset_source = Arc::new(asset_source);
202:         context_lock.asset_source = asset_source.clone();
203:         context_lock.svg_renderer = SvgRenderer::new(asset_source);
204:         drop(context_lock);
205:         self
206:     }
```

- 参数是 **`impl AssetSource`**（静态分发，内部立刻 `Arc::new` 成 `Arc<dyn AssetSource>`，存进 `App`：`app.rs:797`、构造处 `app.rs:845,884`，读出口 `App::asset_source()` 在 `app.rs:2074-2075`）。
- **一次只能注册一个**：签名只收一个值；重复调用只是**覆盖**（没有任何断言/panic）。
- gpui-kit 层面**没有**再包一层 `with_assets`（`<REG>\gpui-kit-0.6.6\src\lib.rs` 全文没有 `with_assets`），应用直接调 `gpui_kit::application().with_assets(..)`。
- Lite 现状：`<REPO>\gpui\crates\app\src\main.rs:179-188` 已经是 `application().with_assets(gpui_kit::assets::AllAssets).run(..)`。

### 4.3 `gpui_kit::assets::AllAssets` 到底实现了什么

它**没有**实现 `RenderImage` 之类的东西（`RenderImage` 是 struct 不是 trait，`assets.rs:43`）；`AllAssets` 是一个 **`rust_embed::RustEmbed` 生成的、实现了 `AssetSource` 的空结构体**：

```rust
// <REG>\gpui-kit-assets-0.6.6\src\native_assets.rs
 5: include!(concat!(env!("OUT_DIR"), "/default_assets.rs"));      // Assets（101 个默认图标）由 build.rs 生成
 7: /// Explicitly embed the complete Lucide and GPUI Kit icon catalog.
 8: #[derive(rust_embed::RustEmbed)]
 9: #[folder = "assets"]
10: #[include = "icons/**/*.svg"]
11: pub struct AllAssets;
...
13: macro_rules! impl_asset_source { ($name:ident) => {
17:         pub fn new(_endpoint: impl Into<SharedString>) -> Self { Self }   // 原生平台忽略 endpoint
22:         impl AssetSource for $name {
23:             fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
24:                 if path.is_empty() { return Ok(None); }
27:                 Self::get(path).map(|file| Some(file.data))
29:                     .ok_or_else(|| anyhow!("could not find asset at path \"{}\"", path))   // ⚠️ 未命中是 Err
30:             }
32:             fn list(&self, path: &str) -> Result<Vec<SharedString>> {
33:                 Ok(Self::iter().filter_map(|name| name.starts_with(path).then(|| name.into())).collect())
36:             }
37:         }
38:     };}
41: impl_asset_source!(Assets);
42: impl_asset_source!(AllAssets);
```

- 导出路径：`<REG>\gpui-kit-assets-0.6.6\src\lib.rs:36`（native）、`:39,43`（wasm 同名别名 `AllAssets`）；`gpui_kit` 侧再导出为 `gpui_kit::assets`（`<REG>\gpui-kit-0.6.6\src\lib.rs:144-145`）。
- **它只覆盖 `icons/**/*.svg`**（第 10 行的 `#[include]`），并且 `IconName` 的路径形如 `icons/accessibility.svg`（`<DOCS>\docs\assets.md:61-63`）。**PNG / ICO 不在它的目录里**，所以「注册 `AllAssets` 就能加载 `logo.png`」是**错的**。
- `icon_assets!(AppAssets, [Search, Check])` 宏（`<REG>\gpui-kit-assets-0.6.6\src\lib.rs:65-86`）也只能按**内置 `IconName` 的路径**返回静态字节（第 73 行比较 `path == IconName::X.path()`），未命中返回 `Ok(None)`（第 77 行）。它**不能**用来注册任意 PNG 路径。

### 4.4 能不能**同时**注册自己的资源目录？——能，但不能「注册两个」

必须自己写一个 struct 实现 `AssetSource`，内部做 fallback。**这不是我发明的**，gpui-kit 自己的 crate 例子与官方 0.6.6 文档写的就是这套：

- crate 例子：`<REG>\gpui-kit-assets-0.6.6\examples\extra_assets.rs:23-37`
- 文档（0.6.6 默认镜像）：`<DOCS>\docs\assets.md:90-136`（小节标题 **「自定义资源」**），其中 `:103-136` 给出 `#[derive(RustEmbed)] #[folder = "./assets"]` + `impl AssetSource` + 把 `ComponentAssets` 当 fallback 的完整代码；`:138-160` 说明随后 `gpui_kit::application().with_assets(Assets)`。

Lite 可直接照抄的形态（**注意 fallback 的 `Err` 语义**）：

```rust
use std::borrow::Cow;

use gpui_kit::assets::AllAssets;
use gpui_kit::{AssetSource, Result, SharedString};   // Result = anyhow::Result（gpui.rs:93）

/// Lithe 的资源源：先查自己的资源，再回退到 gpui-kit 的全量 Lucide 字形。
struct LitheAssets;

impl AssetSource for LitheAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // ① 自己的资源：编译期内嵌，路径名完全自定义（与 IconName 无关）
        if path == "lithe/logo.png" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/images/logo.png"
            ))));
        }

        // ② 回退到 gpui-kit 的内置资源。
        // ⚠️ AllAssets::load 在未命中时返回 **Err**（native_assets.rs:27-29），
        //    所以不能用 `?` 透传，否则「查一个不存在的字形」会变成硬错误而不是加载失败。
        match AllAssets.load(path) {
            Ok(found) => Ok(found),
            Err(_) => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = AllAssets.list(path)?;          // list 未命中是 Ok(vec![])，可以放心透传
        paths.extend(
            ["lithe/logo.png"]
                .into_iter()
                .filter(|name| name.starts_with(path))
                .map(Into::into),
        );
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
```

接线（替换 `<REPO>\gpui\crates\app\src\main.rs:187` 那一行）：

```rust
gpui_kit::application()
    .with_assets(LitheAssets)     // 原来是 gpui_kit::assets::AllAssets
    .run(move |cx| { /* ... */ });
```

几个必须知道的点：

1. **`AllAssets` 与「自定义目录」不能同时直接注册**，只能包装（上面这段）。
2. **`load` 的未命中约定不统一**：`AllAssets` 未命中是 `Err`（`native_assets.rs:27-29`），`icon_assets!` 未命中是 `Ok(None)`（`lib.rs:76-77`），`()` 是 `Ok(None)`（`assets.rs:22-24`）。写 fallback 链时必须区分。
3. **想嵌任意 PNG/ICO**：`AllAssets`/`icon_assets!` 都帮不上（只覆盖 `icons/**/*.svg`），要么 `include_bytes!`（最简单，见上面），要么自己 `#[derive(rust_embed::RustEmbed)]`（`rust-embed 8.12.0` 已在 `gpui/Cargo.lock`，与 `gpui-kit-assets` 用的同一版本，不会重复编译出两份）。
4. `list` 的意义：必须返回**合并、排序、去重**后的列表，官方文档 `assets.md:38-42` 明确说了这是它们实测的口径（例子 `extra_assets.rs:30-36` 同型）。
5. **它跟窗口图标没有任何关系**：`gpui-kit-assets` 的 README 与文档把 `Assets`/`AllAssets` 定义成「图标字体（Lucide SVG）资源源」；任务栏/exe 图标走 §5 的 PE 资源路径。

---

## 5. 可执行文件图标（`.ico`）方案 —— Windows 上唯一真正生效的路

### 5.1 gpui 的 Windows 后端**只从 exe 资源 ID 1 取窗口/任务栏图标**

```rust
// <REG>\gpui-pre-windows-0.3.6\src\platform.rs:1476-1490
1476: fn load_icon() -> Result<HICON> {
1477:     let module = unsafe { GetModuleHandleW(None).context("unable to get module handle")? };
1478:     let handle = unsafe {
1479:         LoadImageW(
1480:             Some(module.into()),
1481:             windows::core::PCWSTR(1 as _),          // ← MAKEINTRESOURCE(1)：资源 ID 必须是 1
1482:             IMAGE_ICON,
1483:             0, 0,
1484:             LR_DEFAULTSIZE | LR_SHARED,
1485:         )
1486:         .context("unable to load icon file")?
1487:     };
1488:     Ok(HICON(handle.0))
1489: }
```

调用链：

```rust
// <REG>\gpui-pre-windows-0.3.6\src\platform.rs:244-248（平台初始化时加载，失败就退化成空 HICON）
244:         let icon = if !headless { load_icon().unwrap_or_default() } else { HICON::default() };
// <REG>\gpui-pre-windows-0.3.6\src\platform.rs:288-303（塞进每次建窗的 CreationInfo）
290:             icon: self.icon,
// <REG>\gpui-pre-windows-0.3.6\src\window.rs:446-460（建窗口时取出并注册窗口类）
446:         let WindowCreationInfo { icon, .. } = creation_info;
460:         register_window_class(icon);
// <REG>\gpui-pre-windows-0.3.6\src\window.rs:1419-1433（窗口类的 hIcon 就是它）
1424:             hIcon: icon_handle,
```

补充事实（都影响「能不能绕过去」）：

- 窗口类注册是 **`static ONCE: Once`**（`window.rs:1419-1421`）——**进程内第一次开窗时定死**，之后所有窗口共用同一个 `hIcon`。
- `WNDCLASSW` 里**只设了 `hIcon`，没有 `hIconSm`**（`window.rs:1422-1430`）——小图标由系统从 `hIcon` 缩放，所以 ICO 里**必须带小尺寸**（现有 `icon.ico` 带了 16/20/24/32，见 §0.2）。
- `WindowsPlatform.icon` 是 **`pub(crate)`**（`platform.rs:1266` 在 `pub(crate) struct WindowCreationInfo` 内），`WindowsPlatform` 本身虽是 `pub`（`platform.rs:42`）但 `icon` 字段 `pub(crate)` → **应用代码无法拿到/替换它**。
- 全 crate grep `WM_SETICON` / `SetClassLongPtr` / `SetClassLong`：**零命中** → gpui 没有任何运行期改图标的入口。
- 全 crate grep `pub fn ...icon`：除了私有的 `load_icon`，没有公开 API。

### 5.2 gpui 已经在 `Lithe.exe` 里放了一份资源（`RT_MANIFEST`，ID 也是 1）

```rust
// <REG>\gpui-pre-0.3.6\build.rs:3-23
 8:     if target_os == "windows" {
 9:         #[cfg(feature = "windows-manifest")]
10:         embed_resource();
...
16:     let resource_dir = std::path::Path::new("resources/windows");
17:     let manifest = resource_dir.join("gpui.manifest.xml");
18:     let rc_file = resource_dir.join("gpui.rc");
21:     embed_resource::compile(rc_file, embed_resource::ParamsIncludeDirs([resource_dir]))
22:         .manifest_required()
23:         .unwrap();
```

```rc
// <REG>\gpui-pre-0.3.6\resources\windows\gpui.rc:1-2
1: #define RT_MANIFEST 24
2: 1 RT_MANIFEST "gpui.manifest.xml"
```

- 该 feature **默认开着**：`<REG>\gpui-pre-0.3.6\Cargo.toml:58-63`（`default = ["font-kit","wayland","x11","windows-manifest"]`）、`:78`（`windows-manifest = ["dep:embed-resource"]`）。
- **实测证据（我在不构建的前提下直接读的已构建产物）**：`<REPO>\gpui\target\debug\Lithe.exe`（79 915 520 字节）里能按 ASCII 搜到 `requestedExecutionLevel`（偏移 79 623 465）、`dpiAware`（79 623 938）、`assemblyIdentity`（79 624 355）——**说明 gpui 的 manifest 确实被链进了 Lithe.exe**。
- 链路机制（**推断**，见 §7）：`gpui-pre` 是**只有 lib、没有 bin** 的 crate，`embed_resource::compile` 对它走的是「非 bin」分支，发的是 `cargo:rustc-link-search=native=…` + `cargo:rustc-link-lib=dylib=<rc 文件名主干>`（`<REG>\embed-resource-3.0.11\src\lib.rs:429-449`）；`rustc-link-lib` / `rustc-link-search` 会**沿依赖链向上传播**，所以最终 bin 也链接了那份资源。

**对本任务的影响（重要）**：我们自己的 `.rc` **只写 ICON，绝不要写 manifest**，否则会和 gpui 那份 ID 1 的 `RT_MANIFEST` 撞车。工具默认值正好帮我们避免这一点：`winresource` 的 `manifest` 默认是 `None`（`<REG>\winresource-0.1.31\lib.rs:292`），且只有在「同时设置了 FILETYPE 和 manifest」时才写 manifest 块（`:578-589`）。

### 5.3 三条可落地的 build-script 写法（选一条即可）

#### (a) `embed-resource`（**推荐**：它已经在 `gpui/Cargo.lock` 里）

`gpui-pre` 的 build-dependency 就是它，lock 已解析到 **3.0.11** → 加进应用的 `[build-dependencies]` **不会引入新的 crate 版本**。给应用 crate 自己用一个 `.rc`，让 `rc.exe` 处理 ID：

```rc
// 新增 <REPO>\gpui\crates\app\lithe.rc
// 资源 ID 必须是 1：gpui 的 load_icon() 硬编码 MAKEINTRESOURCE(1)
1 ICON "assets/icons/icon.ico"
```

```rust
// 新增 <REPO>\gpui\crates\app\build.rs
fn main() {
    #[cfg(windows)]
    {
        // 应用 crate 有 [[bin]]（<REPO>\gpui\crates\app\Cargo.toml:7-9），
        // 所以 compile() 会发 cargo:rustc-link-arg-bins=…（embed-resource/src/lib.rs:442-443）。
        embed_resource::compile("lithe.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
```

```toml
# 新增到 <REPO>\gpui\crates\app\Cargo.toml
[build-dependencies]
embed-resource = "3"
```

要点：`embed_resource::compile` 在「调用方 crate 有 bin」时发 `cargo:rustc-link-arg-bins=<out>`（`<REG>\embed-resource-3.0.11\src\lib.rs:442-443`）；`.rc` 里的相对路径由 `rc.exe` 以 CWD=**build script 的 CWD**（即 crate 根 `gpui/crates/app`）解析，所以写 `assets/icons/icon.ico` 时需要把 ico 放到该 crate 下或改用 `../../assets/icons/icon.ico`（**路径写法未验证**，见 §7）。
`manifest_optional()` / `manifest_required()` 只影响「rc 是否必须编出 manifest」的返回值判定（`:351` 等），我们的 `.rc` 没有 manifest，应选 `manifest_optional()`。

#### (b) `winresource`（默认 ID 恰好就是 1，代码最短）

```rust
// <REPO>\gpui\crates\app\build.rs
fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icons/icon.ico");   // 默认 nameID = "1"，见下
        res.compile().unwrap();
    }
}
```

- `set_icon` 的默认 ID **就是 `"1"`**：`<REG>\winresource-0.1.31\lib.rs:408-410`（`const DEFAULT_APPLICATION_ICON_ID: &str = "1"; ... self.set_icon_with_id(path, DEFAULT_APPLICATION_ICON_ID)`）——**与 gpui 的 `PCWSTR(1)` 完全吻合**。
- 生成的 `.rc` 形如 `1 ICON "<path>"`（`:570-577`）。
- MSVC 分支发 `cargo:rustc-link-arg=<out_dir>\resource.res`（`:732-789`，具体 `<REG>\winresource-0.1.31\lib.rs:787`）。
- 需要把 `winresource = "0.1"` 加进 `[build-dependencies]`：本机 registry 有 0.1.31，但**它不在 `gpui/Cargo.lock` 里** → 需要更新 lock（联网/走镜像解析）。

#### (c) `tauri-winres`（旧前端用的就是它，但**默认 ID 是错的**）

`tauri-build` 当年给 `LitHE.exe` 嵌图标用的是 **ID 32512**，不是 1：

```rust
// <REG>\tauri-build-2.6.3\src\lib.rs:668-669
668:     if window_icon_path.exists() {
669:       res.set_icon_with_id(&window_icon_path.display().to_string(), "32512");
// <REG>\tauri-winres-0.3.6\src\lib.rs:287-294（文档自述 32512 是误解了 IDI_APPLICATION）
287:     /// Add an icon with nameID `32512`.
294:     /// The reason for `32512` is that we misunderstood `IDI_APPLICATION` (`MAKEINTRESOURCE(32512)`)
304:     pub fn set_icon(&mut self, path: &str) -> &mut Self {
305:         self.set_icon_with_id(path, "32512")
```

→ 若沿用 `tauri-winres`，**必须**写 `set_icon_with_id(path, "1")`。否则资源管理器里 exe 图标看着是对的（Explorer 取编号最小的 `RT_GROUP_ICON`），但**窗口/任务栏图标会退回系统默认图标**（因为 gpui 只按 ID 1 找）。这是本任务最隐蔽的坑。

### 5.4 `rc.exe` 依赖与本机可用性

三个方案都要 Windows SDK 的资源编译器。本机实测：

- SDK 根：注册表 `HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots` → `KitsRoot10 = C:\Program Files (x86)\Windows Kits\10\`。
- `rc.exe` **不在 PATH**（`Get-Command rc.exe` 无结果），但**存在于版本子目录**：`C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe`（另有 arm64/x86 两份）。`llvm-rc.exe` **不存在**。
- 两个库都能自己找到它：
  - `winresource`：查注册表 + 扫 `bin\*\{x64,x86,arm64}`（`<REG>\winresource-0.1.31\lib.rs:794-851`），并支持 `RC_PATH` 环境变量覆盖（`:733`）。
  - `embed-resource`：`find_windows_10_kits_tool` 同样扫 `<KitsRoot10>\bin\*\x64\rc.exe`（`<REG>\embed-resource-3.0.11\src\windows_msvc.rs:58-71,125-152`），支持 `RC_<target>` / `RC` 环境变量（`src/lib.rs:641-645`）。
- 因此**本机不需要额外安装**（前提是 `rc.exe` 所在 SDK 版本仍在上面那个路径；如果将来 SDK 被换掉，用 `RC_PATH` 指过去）。

### 5.5 可对照的旧前端口径（Windows 前端当时怎么做的）

- 无边框窗口，所以显式设置图标：`<REPO>\windows\tauri\src-tauri\src\host.rs:295-304`（`decorations(false)` → `.icon(WINDOW_TASKBAR_ICON)` → 建窗后 `apply_window_taskbar_icon(&window)`）。
- 用的是 **32×32 PNG**，注释写明理由：`<REPO>\windows\tauri\src-tauri\src\host.rs:14-16`「Windows 11 taskbar downscales a 256-only ICO into an empty pill. Use the 32px asset after window creation so frameless windows keep a readable app icon.」
- 打包图标清单：`<REPO>\windows\tauri\src-tauri\tauri.conf.json:27`（`["icons/32x32.png","icons/128x128.png","icons/icon.ico"]`）。
- 旧前端还有**回归测试**直接读 `icon.ico` 校验内嵌尺寸（`host.rs:451`）与 32px 位图可见边界（`host.rs:471-480`）——gpui 侧若也要断言，可参考这两处的做法。
- ⚠️ gpui 侧**没有**等价于 Tauri `window.set_icon(..)` 的「建窗后设置图标」API（§5.1 的 grep 结论）→ 旧前端「建窗后补一张 32px」的兜底在 gpui 上**不成立**，只能靠 exe 资源。

---

## 6. 备选路线（都不是首选，且**未验证**）

### 6.1 运行期拿到 HWND 自己 `WM_SETICON`

可行性证据（源码层）：

- `Window` 实现了 `raw_window_handle::HasWindowHandle`：`<REG>\gpui-pre-0.3.6\src\window.rs:7343-7347`（`fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError>`），另见 `HasDisplayHandle`（`:7349-7355`）与 `PlatformWindow: HasWindowHandle + HasDisplayHandle`（`platform.rs:880`）。
- Win32 变体：`raw_window_handle::RawWindowHandle::Win32(Win32WindowHandle)`（`<REG>\raw-window-handle-0.6.2\src\lib.rs:175`），其字段 `pub hwnd: NonZeroIsize`（`<REG>\raw-window-handle-0.6.2\src\windows.rs:52-54`）。
- 依赖可用性：`raw-window-handle 0.6.2` 与 `windows 0.62.2` 都已在 `gpui/Cargo.lock`。

理论做法（**我没有编译/运行验证**）：用 `window.window_handle()` 取 `hwnd` → `SendMessageW(hwnd, WM_SETICON, ICON_BIG/ICON_SMALL, hicon)`；`hicon` 需自己从 ICO 字节造（例如 `CreateIconFromResourceEx`）或从磁盘 `LoadImageW`（会要求 ico 随程序分发）。
不推荐的理由：① 要新加两个直接依赖和一段 unsafe；② gpui 之后若自己管图标（改窗口类/响应 `WM_SETICON`）会互相打架；③ 窗口类 `hIcon` 仍是空/默认，某些系统路径（Alt-Tab、任务栏缩略图）可能走类图标而不是窗口图标——**未验证**。

### 6.2 macOS / Linux

- macOS：`WindowOptions.icon` 注释写 X11 only；`gpui-pre-macos` 源码本机不存在 → **无法核对**。业界通行做法是 `.app` bundle 的 `Info.plist` `CFBundleIconFile` + `Resources/icon.icns`（`gpui/assets/icons/icon.icns` 已经就位）。**未验证**。
- Linux：`WindowOptions.icon` 是 X11 (`_NET_WM_ICON`) 的路子，需要 `image` 解码（§3）。`gpui-pre-linux` 源码本机不存在 → **未验证**。

---

## 7. 未验证 / 不确定的部分（明确列出）

1. **macOS 与 Linux/X11 的后端实现**：`gpui-pre-macos` / `gpui-pre-linux` 不在本机 registry（本机只构建 Windows），`WindowOptions.icon` 的实际行为**只依据 doc 注释**（`platform.rs:2238`、`:2294`）。
2. **三种 build-script 方案都没有实际编译/链接验证**：本任务硬约束禁止 `cargo build/check`。特别是：
   - 「两份资源（gpui 的 `RT_MANIFEST` ID 1 + 我们的 `RT_GROUP_ICON`/`RT_ICON`/`RT_VERSION`）同时链入不冲突」是**推断**（Win32 资源按 `(type, id)` 索引，类型不同所以不冲突）——**没实测**。
   - 「`gpui-pre` 的 manifest 靠 `rustc-link-lib` 向上传播进 bin」是**推断**（依据 `<REG>\embed-resource-3.0.11\src\lib.rs:429-449` 与 Cargo 的传播语义）。**但「manifest 确实在 Lithe.exe 里」这一条是实测的**（§5.2 的字节搜索）。
3. **`.rc` 里 ICO 的相对路径**：`rc.exe` 解析相对路径的基准是 build script 的 CWD（crate 根），我按 `winresource` 传 `env!("CARGO_MANIFEST_DIR")` 作为 `/I` 的行为推断（`winresource lib.rs:754`、`embed-resource windows_msvc.rs:36`），但**没实测**「`../../assets/icons/icon.ico` 能不能被 `rc.exe` 接受」。稳妥做法是把 `icon.ico` 复制/软链到 `gpui/crates/app/` 下再用相对短路径（**这属于实现决策，本笔记不动手**）。
4. **`Lithe.exe` 里 `RT_MANIFEST` 的 ID 是 1**：这是从 `<REG>\gpui-pre-0.3.6\resources\windows\gpui.rc:2` 读出来的，不是从 exe 的资源目录里解析出来的（我没用 `LoadLibraryEx`/`EnumResourceNames` 之类的 API 读 exe 资源表，只做了字节搜索）。
5. **任务栏图标是否 100% 取窗口类 `hIcon`**（而不是 AppUserModelID 关联的快捷方式图标）：未验证，需要实机跑。相关工作（`app_id`、`set_app_identity`、JumpList/`set_dock_menus`）我只确认了 `WindowOptions.app_id` 字段存在（`platform.rs:2228-2229`），**没有**核对 gpui 在 Windows 上如何消费它。
6. **「gpui 没有建窗后设置图标的 API」**：结论基于对本地解压源码的 grep（`gpui-pre-0.3.6`、`gpui-pre-windows-0.3.6`、`gpui-kit-0.6.6` 里 `set_icon` / `WM_SETICON` / `SetClassLong*` 零命中；`gpui-base-0.6.6\src` 里 `WindowOptions|WindowParams` 也零命中；`gpui-component-0.6.6\src` 只把 `AssetSource` 用在原生菜单图标上，没有图标设置 API）。没有别的隐藏入口（如 `Platform` trait 上的方法）——`Platform`/`PlatformWindow` trait 的定义在 `platform.rs:161-260` / `:880+`，我没有逐行通读其全部方法。
7. **0.6.6 文档里没有任何「应用图标 / exe 图标 / 任务栏图标」章节**（见 §8 的检索记录）——所以「文档没写」这个结论是**检索结论**，不是「文档不完整」的判断。

---

## 8. 文档镜像检索记录（回答第 5 问）

- 镜像规模：`<DOCS>` 下 151 个文件（`Get-ChildItem -Recurse -File | Measure-Object` = 151），四个子目录 `base/`、`component/`、`docs/`、`shell/`。
- **有**一章与「资源/图标」相关：**`<DOCS>\docs\assets.md`，标题 `# Icon`（第 6 行）**，其 url front-matter 是 `/zh-CN/docs/assets.md`，description「为 GPUI Component 应用配置内置图标、自定义 SVG 与资源加载方式」。相关小节：
  - `## 共享名称与兼容性`（`:51`）、`## 使用默认内置资源`（`:65`，含 `with_assets(Assets)` 的 0.6.6 写法 `:79-84`）、**`## 自定义资源`（`:90`，`:103-136` 是自定义 `AssetSource` + fallback 的完整代码）**、`## 使用图标`（`:162`）、`## 单独嵌入 SVG 图标`（`:183`，`Icon::default().data(include_bytes!(...))`）。
  - 同页 `:19-49` 的 NOTE 讲了体积账（默认 101 个图标 44.28 KiB / `AllAssets` 1,830 个 731.45 KiB），与 Lite 现状里的注释一致。
  - 同一内容在组件页 `<DOCS>\component\icon.md`（`## 应用额外图标` 在 `:53`）里几乎重复。
- 另有 `<DOCS>\component\title-bar.md:92-118`（`### 窗口配置`）讲 `WindowOptions` 该以 `TitleBar::window_options()` 为基座 —— 这是本任务要改的那段配置的唯一文档依据。
- **没有**任何一页讲「应用图标 / exe 图标 / 任务栏图标」。检索方式（对 `<DOCS>` 全目录 grep）：`应用图标|任务栏|window icon|\.ico|winres|winresource|embed.resource|set_window_icon|taskbar|桌面图标` —— 命中全部落在「组件图标 / SVG / `Icon`」语境，`.ico` 的匹配其实都来自 `.icon(...)` 方法调用，**没有任何一条是 exe 图标**。`docs/installation.md:46-52` 也只讲「依赖只加 `gpui-kit = "0.6"`」，没有打包/资源章节（全 69 行里没有 icon 相关段落）。
- 结论：**「给应用/窗口设图标」在 0.6.6 文档里是空白区**，唯一权威信息在源码（本笔记 §2/§5）。

---

## 9. 引用清单（文件:行号）

### 9.1 gpui / gpui-kit 源码（`<REG>`）

| 主题 | 位置 |
| --- | --- |
| `WindowOptions` 定义 / `icon` 字段 / 注释 X11 only | `gpui-pre-0.3.6\src\platform.rs:2172-2243`、`:2238-2239` |
| `WindowOptions` 无 `non_exhaustive`、仅 `Debug` | `gpui-pre-0.3.6\src\platform.rs:2173-2174`（全文 grep `non_exhaustive` 零命中） |
| `WindowOptions` 手写 `Default`（`icon: None`） | `gpui-pre-0.3.6\src\platform.rs:2345-2371`、`:2364` |
| `Arc` 来源（`std::sync::Arc`） | `gpui-pre-0.3.6\src\platform.rs:70-77` |
| `WindowParams.icon` | `gpui-pre-0.3.6\src\platform.rs:2294-2296` |
| `ImageFormat` / `gpui::Image` / `Image::from_bytes` | `gpui-pre-0.3.6\src\platform.rs:2825-2849`、`:2902-2911`、`:2953-2959` |
| gpui 内部解码 `pub(crate)` + BGRA 交换 | `gpui-pre-0.3.6\src\platform.rs:2913-2938`（交换在 `:2932-2935`） |
| `Image::use_render_image` / `get_render_image`（拿的是 `RenderImage`） | `gpui-pre-0.3.6\src\platform.rs:2966-2998` |
| `RenderImage` 定义 | `gpui-pre-0.3.6\src\assets.rs:43-49` |
| `AssetSource` trait 全签名 / `()` 实现 | `gpui-pre-0.3.6\src\assets.rs:12-19`、`:21-29` |
| `Application::with_assets` | `gpui-pre-0.3.6\src\app.rs:198-206` |
| `asset_source` 字段与读取口 | `gpui-pre-0.3.6\src\app.rs:797`、`:845`、`:884`、`:2074-2075` |
| `AssetSource` 的消费者（SVG / img） | `gpui-pre-0.3.6\src\svg_renderer.rs:93,124,257,309-315`、`gpui-pre-0.3.6\src\elements\img.rs:631,658` |
| `WindowOptions` → `WindowParams` 传递 | `gpui-pre-0.3.6\src\window.rs:1519-1542`（icon 在 `:1539`）、`:1549-1568`（`icon` 在 `:1564`） |
| `impl HasWindowHandle for Window` | `gpui-pre-0.3.6\src\window.rs:7343-7347`、`:7349-7355` |
| `PlatformWindow: HasWindowHandle + HasDisplayHandle` | `gpui-pre-0.3.6\src\platform.rs:880` |
| `pub use image` 不存在；re-export 清单 | `gpui-pre-0.3.6\src\gpui.rs:89-168`（`Result` 在 `:93`） |
| `image` 依赖与 features | `gpui-pre-0.3.6\Cargo.toml:342-361` |
| `gpui-pre` 默认 feature 含 `windows-manifest` | `gpui-pre-0.3.6\Cargo.toml:58-63`、`:78` |
| `gpui-pre` lib 名 = `gpui` | `gpui-pre-0.3.6\Cargo.toml:82-83` |
| gpui manifest 的 build script / .rc | `gpui-pre-0.3.6\build.rs:3-23`、`gpui-pre-0.3.6\resources\windows\gpui.rc:1-2` |
| `WindowOptions` 字面量示例 | `gpui-pre-0.3.6\examples\hello_world.rs:101-105`、`gpui-pre-0.3.6\examples\image_loading.rs:37` |
| Windows：`load_icon()`（资源 ID 1） | `gpui-pre-windows-0.3.6\src\platform.rs:1476-1490` |
| Windows：图标加载与降级 | `gpui-pre-windows-0.3.6\src\platform.rs:244-248`、`:288-303`、`:1265-1266` |
| Windows：窗口类 `hIcon` + `Once` | `gpui-pre-windows-0.3.6\src\window.rs:446-460`、`:1419-1433` |
| gpui-kit facade：`pub use ::gpui::*` / `assets` / `component` | `gpui-kit-0.6.6\src\lib.rs:95`、`:106`、`:108`、`:143`、`:144-145` |
| gpui-kit 依赖钉 `gpui-pre = 0.3.6` | `gpui-kit-0.6.6\Cargo.toml:354-356`、`:358-367`、`:374-382` |
| `TitleBar::window_options()` / `title_bar_options()` | `gpui-component-0.6.6\src\title_bar.rs:59-65`、`:67-91` |
| `AllAssets` 是 rust_embed + `AssetSource` | `gpui-kit-assets-0.6.6\src\native_assets.rs:8-11`、`:13-39`、`:41-42`（`load` 未命中 `Err` 在 `:27-29`） |
| `AllAssets` 导出 / wasm 同名 | `gpui-kit-assets-0.6.6\src\lib.rs:36`、`:39`、`:43` |
| `icon_assets!` 宏 | `gpui-kit-assets-0.6.6\src\lib.rs:65-86`（未命中 `Ok(None)` 在 `:76-77`） |
| fallback 组合的 crate 例子 | `gpui-kit-assets-0.6.6\examples\extra_assets.rs:23-37` |

### 9.2 工具与依赖（`<REG>`）

| 主题 | 位置 |
| --- | --- |
| `winresource::set_icon` 默认 ID = `"1"` | `winresource-0.1.31\lib.rs:408-410`、`:457` |
| winresource 生成 `N ICON "path"` | `winresource-0.1.31\lib.rs:570-577` |
| winresource `manifest` 默认 `None` / FILETYPE 默认 1 | `winresource-0.1.31\lib.rs:292`、`:225`、`:578-589` |
| winresource MSVC 分支发 link-arg | `winresource-0.1.31\lib.rs:732-789`（`:787`） |
| winresource 找 `rc.exe` / `RC_PATH` | `winresource-0.1.31\lib.rs:794-851`、`:733` |
| `tauri-winres` 默认 ID 32512（并自述是误解 IDI_APPLICATION） | `tauri-winres-0.3.6\src\lib.rs:287-294`、`:304-305`、`:355` |
| `tauri-build` 当年用的 ID | `tauri-build-2.6.3\src\lib.rs:668-669` |
| embed-resource：有 bin → `rustc-link-arg-bins`；无 bin → link-search + link-lib | `embed-resource-3.0.11\src\lib.rs:429-451`（`:442-443`、`:447-448`） |
| embed-resource：`compile_for` / `compile_for_everything` | `embed-resource-3.0.11\src\lib.rs:468-483`、`:554-565` |
| embed-resource：MSVC 编 `.lib` + 找 `rc.exe` + `RC_<target>`/`RC` | `embed-resource-3.0.11\src\windows_msvc.rs:25-47`、`:58-71`、`:125-152`、`src\lib.rs:641-645` |
| image API（`load_from_memory_with_format` / `to_rgba8` / 根导出） | `image-0.25.10\src\images\dynimage.rs:1691`、`:304`、`image-0.25.10\src\lib.rs:148`、`:166` |
| raw-window-handle Win32 变体与字段 | `raw-window-handle-0.6.2\src\lib.rs:175`、`src\windows.rs:52-54` |

### 9.3 仓库内文档镜像（`<DOCS>`）

| 主题 | 位置 |
| --- | --- |
| 资源与图标页（`# Icon`） | `<DOCS>\docs\assets.md:1-207`（`:65` 默认资源、`:90` 自定义资源、`:103-136` fallback 代码、`:183` Icon::data） |
| 依赖与 feature 说明 | `<DOCS>\docs\getting-started.md:19-24` |
| 窗口配置（`TitleBar::window_options()` 基座） | `<DOCS>\component\title-bar.md:92-118` |
| 组件图标 API（与 assets.md 重复） | `<DOCS>\component\icon.md:8`、`:20-42`、`:53`、`:118`、`:329` |
| 安装（无打包/图标章节） | `<DOCS>\docs\installation.md:1-69` |

### 9.4 Lite 仓库现状（`<REPO>`）

| 主题 | 位置 |
| --- | --- |
| 当前启动顺序与 `with_assets(AllAssets)` | `gpui\crates\app\src\main.rs:179-188` |
| 当前 `WindowOptions` 字面量（要加 `icon` 的那处） | `gpui\crates\app\src\main.rs:209-213` |
| 应用依赖只有 gpui-kit | `gpui\crates\app\Cargo.toml:23` |
| bin 目标名 `Lithe`（无 build.rs） | `gpui\crates\app\Cargo.toml:7-9` |
| workspace 成员 `crates/*` | `gpui\Cargo.toml:8-10` |
| 图标/logo 资源已就位 + 「尚无代码引用」 | `gpui\assets\README.md:1-11`、`:29-33`、`:90-118` |
| 旧前端设图标 / 无边框 / 32px 理由 | `windows\tauri\src-tauri\src\host.rs:14-22`、`:295-304` |
| 旧前端 icon.ico 回归测试 | `windows\tauri\src-tauri\src\host.rs:451`、`:471-480` |
| 旧前端打包图标清单 | `windows\tauri\src-tauri\tauri.conf.json:27` |
| lock 里已有的相关版本 | `gpui\Cargo.lock`（`gpui-pre 0.3.6` / `gpui-pre-windows 0.3.6` / `image 0.25.10` / `rust-embed 8.12.0` / `raw-window-handle 0.6.2` / `windows 0.62.2` / `embed-resource 3.0.11`） |

### 9.5 实测记录（本次研究执行过的只读命令）

| 结论 | 命令/方法 |
| --- | --- |
| `gpui/assets/**` 已提交、工作树干净 | `git status --porcelain`（空）、`git ls-files gpui/assets`、`git log --oneline -3 -- gpui/assets` → `9d9bcdaa` |
| `icon.ico` 含 16/20/24/32/48/64/128/256 共 8 张 32bpp | 读 ICO 头 + 目录项（`ICONDIR`/`ICONDIRENTRY`） |
| `logo.png` 是 1024×1024 / 810 582 B | 读 PNG IHDR |
| gpui 的 manifest 已在 `Lithe.exe` 里 | 对 `gpui\target\debug\Lithe.exe`（79 915 520 B）做 ASCII 搜索：`requestedExecutionLevel`@79 623 465、`dpiAware`@79 623 938、`assemblyIdentity`@79 624 355；`lithe.ico` 未命中（预期，尚未接线） |
| 本机有 `rc.exe`（不在 PATH） | `HKLM\...\Installed Roots` → `KitsRoot10=C:\Program Files (x86)\Windows Kits\10\`；`...\bin\10.0.26100.0\x64\rc.exe` 存在；`Get-Command rc.exe`/`llvm-rc.exe` 无结果 |
| 三个 kit crate 没有 `unimplemented!()` 的 AssetSource | 对 `gpui-kit-0.6.6`、`gpui-kit-assets-0.6.6`、`gpui-base-0.6.6` grep `unimplemented!\|todo!(\|unreachable!` → 仅 3 处 `unreachable!`（dock 布局树、markdown），与 assets 无关 |

---

## 10. 给接线任务的最短行动清单（不含代码改动，仅指路）

1. **Windows（真正要解决的那条）**：在 `gpui/crates/app/` 加 `build.rs` + 一个 `.rc`（`1 ICON "…/assets/icons/icon.ico"`），build-dependency 选 `embed-resource = "3"`（已在 lock）或 `winresource = "0.1"`（默认 ID 就是 1）。**不要写 manifest**。
2. **`WindowOptions`**：Windows 上**不必**加 `icon:`（加了无效，见 §2.3）；若为将来 Linux 预留，加 `icon: Some(Arc<image::RgbaImage>)` 并同时加 `image` 依赖（§3.3）。
3. **验证**：`cargo build` 后检查 `Lithe.exe` 是否带 `RT_GROUP_ICON`/`RT_ICON`（资源查看器或 `LoadImageW(module, 1, IMAGE_ICON, …)` 探针），再实机看任务栏；两处都需要**实际运行**才能确认（本笔记无法覆盖，见 §7）。
4. **资源系统**：如果这轮只做窗口/exe 图标，**不要动 `with_assets`**（`AllAssets` 现状是对的）；等做欢迎页/标题栏 logo 时再按 §4.4 换成 wrapper。
