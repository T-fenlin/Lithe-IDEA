# s3 · GPUI 打包面清单：产物必须带哪些文件（构建发布层证据）

基线提交 `3085f5d1`。本文件回答一个对打包链路设计有决定作用的问题：
**GPUI 的 Windows/macOS 产物除了 exe 之外，还必须随包分发什么？**

检索口径：`rust/lithe-gpui/crates/**/*.rs` 实现代码（不含 `locales/**` 文案）。
本机实测项单独标注。

---

## 1. 资源解析的四类路径

### 1.1 编译期内嵌（打包不需要带源码树）

| 资源 | 机制 | 证据 |
|---|---|---|
| `assets/**`（315 个文件：图标、app-icon、README） | `rust-embed`，`#[folder = "$CARGO_MANIFEST_DIR/../../assets"]` | `crates/app/src/assets.rs:13` |
| `themes/*.json`（1 个） | `include_str!` | `crates/settings/src/theme.rs:16` |
| `extensions/themes/*.json`（5 个） | `include_bytes!` | `crates/settings/src/extensions.rs:30/40/50/60/77` |
| `extensions/fonts/**`（6 个：IBM Plex Mono、Source Code Pro 各 3 文件含 LICENSE） | `include_bytes!` | `crates/settings/src/extensions.rs:88-118` |
| `assets/fonts/*.ttf`（5 个 JetBrains Mono） | `include_bytes!` | `crates/settings/src/fonts.rs:12-24` |

`BUNDLED_EXTENSIONS` 的构成（`crates/settings/src/extensions.rs`）实测：
`Theme = 5`、`Font = 2`、`Languages = 1`，全部走 `include_bytes!`。
`BUILTIN_THEMES`（`theme.rs:14`）同样内嵌。

### 1.2 `repository_*_dir()`：名字骗人，**只在测试里用**

`crates/settings/src/paths.rs` 定义了五个返回源码树路径的函数：

```
:50   repository_themes_dir()            -> concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes")
:54   repository_theme_extensions_dir()  -> .../extensions/themes
:78   repository_icons_dir()             -> .../assets/icons
:82   repository_font_extensions_dir()   -> .../extensions/fonts
```
`crates/settings/src/fonts.rs:155` 另加 `repository_builtin_font_dir()`。

**逐条核对全部调用点**（检索 `repository_(icons|themes|builtin_font|theme_extensions|font_extensions)_dir`，
`crates/` 下共 17 处命中）：

| 调用点 | 是否生产路径 |
|---|---|
| `settings/src/paths.rs:257/258/264/265` | `#[cfg(test)]`（模块起点 `:139`） |
| `settings/src/theme.rs:829/834` | `#[cfg(test)]`（测试函数内） |
| `settings/src/theme.rs:928/929` | `#[cfg(test)]`（`every_theme_file_is_a_loadable_theme_set`） |
| `settings/src/extensions.rs:501` | `#[cfg(test)]`（`icons_packages_have_a_real_manifest`） |
| `settings/src/extensions.rs:536` | `#[cfg(test)]`（`theme_entries_match_the_repository_files`） |
| `settings/src/extensions.rs:555` | `#[cfg(test)]`（`font_entries_match_the_repository_files_and_ship_a_license`） |
| `settings/src/fonts.rs:165` | `#[cfg(test)]`（`every_builtin_font_file_is_embedded`） |

**17 处调用点全部位于 `#[cfg(test)]` 模块内，生产路径 0 处。**
这些函数的实际作用是**测试断言"内嵌表与源码树文件一一对应"**——正好证明内嵌是完整的。

### 1.3 运行时可写位置（打包不需要带，但必须保证可写）

| 用途 | 位置 | 证据 |
|---|---|---|
| 设置文件 | Win `%APPDATA%/Lithe/settings.json`；macOS `~/Library/Application Support/Lithe/settings.json`；Linux `$XDG_CONFIG_HOME/lithe` | `settings/src/paths.rs:98-137` |
| 主题/字体扩展安装目录 | 设置文件同级的 `extensions/` | `settings/src/paths.rs:61-96` |
| jdtls 索引缓存 | Win `%LOCALAPPDATA%/Lithe/cache`；macOS `~/Library/Caches/Lithe`；Linux `$XDG_CACHE_HOME/lithe` | `java/src/workspace.rs:52-74` |

这三处**全部通过平台环境变量推导**，没有一个从 exe 所在目录推导。

### 1.4 必须随包分发的（exe 同级）

**语言服务器**，通过 `current_exe()` 定位：

```
java/src/jdtls.rs:275-296   bundled_roots(): exe_dir/LanguageServers/jdtls
java/src/jdtls.rs:582-585   exe_dir/LanguageServers/jdk
```

`bundled_roots()` 里还有一条兜底：从 exe 目录逐级向上找 `.artifacts/jdtls`（最多
`MAX_EXE_WALK_DEPTH` 层）——这是**开发期**便利，正式打包后靠
`exe 同级/LanguageServers/jdtls` 命中，与旧端 `build-windows.ps1:118/132` 的落位一致。

其余生产路径的 `current_exe()` 用法不影响打包内容：
`settings/src/restart.rs:7`（重启自身，`S1_SETTINGS restarting`）、
`workbench/src/spring.rs:315`（在 `#[cfg(test)]` 内，模块起点 `:172`）。

---

## 2. 打包面结论

**Windows 产物必须包含：**

```
Lithe.exe                              <- cargo build -p lithe-gpui-app --release
LanguageServers/jdtls/**               <- prepare-jdtls.ps1 产出（bin/jdtls.bat、config_win、plugins、lombok/lombok.jar）
LanguageServers/jdk/**                 <- prepare-jdk.ps1 产出（bin/java.exe、lib）
```

**不需要随包**：`themes/**`、`extensions/**`、`assets/**`、`assets/fonts/**`
——全部编译期内嵌（`rust-embed` / `include_str!` / `include_bytes!`）。

**签名范围**：`sign-windows.ps1:74-79` 的设计是「先签 exe，再签同级每个 `*.dll`」。
本机实测 `lithe-gpui-target/debug/` 顶层 **DLL 数量为 0**（38 个 DLL 都在 `deps/` 子目录，
不随产物分发），说明 GPUI 的 Windows 产物**很可能只有一个 exe 需要签**
（MSVC 目标动态链接 CRT，系统组件无需签名）。该结论需在 release 产物上复测确认。

**macOS 产物**（未在本环境验证，本工作区为 Windows 主机）：
按 `java/src/workspace.rs:57-65` 与 `paths.rs:108-117` 的 macOS 分支，
产物应为 `.app` bundle，`Contents/MacOS/Lithe` 为可执行文件，
`Contents/Resources` 下需放 `LanguageServers/{jdtls,jdk}`（由 `current_exe()` 的父目录
`Contents/MacOS/` 推得 `Contents/MacOS/LanguageServers`；**此落位需在 macOS 上实测确认**，
因为旧 macOS 端把语言服务器放在插件包内而非 app 同级）。

---

## 3. 与「打包只需 exe」判断的对照

**预测**：themes/fonts/icons/extensions 全内嵌；打包须带 exe + `LanguageServers/{jdtls,jdk}`；
`repository_*_dir()` 仅测试调用。
**实际**：逐条核对一致，17 处调用点全在 `cfg(test)`，生产路径 0 处。**无异常。**

---

## 4. 本步未覆盖 / 需在别的环境确认

1. **macOS 产物的 `LanguageServers` 落位未实测。** 上述推断来自 `current_exe().parent()`
   的代码逻辑，不是运行结果。标注为：**只读代码，未验证**。
2. **release profile 下的 DLL 情况未测。** 本机只有 debug 产物，`debug/` 顶层 0 个 DLL；
   release 下是否引入同目录 DLL 需复测。
3. **`assets/` 有 315 个文件但 `#[exclude]` 排除了 `fonts/**` 与 `README.md`**（`assets.rs:14-15`），
   字体另走 `fonts.rs` 的 include_bytes。两处边界未再深查是否有重叠或遗漏。
4. 未核对 `lithe-ui` 组件库自身的 `Assets` 源（`assets.rs:62` 回退到 `lithe_ui::Assets`）
   在打包时是否也全部内嵌——它同样是 `rust-embed`，但未逐个核对清单。
