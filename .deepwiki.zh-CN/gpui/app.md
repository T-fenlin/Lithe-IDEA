# GPUI App Shell

`rust/gpui/crates/app/` 是**组合根**。它只有 5 个文件，一个二进制目标（`Lithe`），没有 `lib.rs`，也没有 `tests/`。它的全部职责在 `app/src/main.rs:1-6` 中被概括为：*“App Shell 只组合窗口与 Feature，不承载任何具体 Feature 逻辑”*。

| 文件 | 职责 |
| --- | --- |
| `main.rs` | CLI 解析、启动根解析、21 步启动顺序、窗口边界、设置 / 主题接线，以及所有校验探针驱动（1307 行） |
| `assets.rs` | `LitheAssets`：对 `gpui/assets/**` 的 `rust_embed` 表格，以及给 gpui-kit 的 `AllAssets` 回退的 `AssetSource`（436 行） |
| `build.rs` | 仅 Windows：编译 `lithe.rc`，将 `icon.ico` 嵌入为 PE 资源 **ID 1** |
| `lithe.rc` | 22 行的 `.rc`，说明为什么 ID 必须为 1，以及为什么不能加入 MANIFEST |

`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`（`:37`）—— release 版本运行在 GUI 子系统中，debug 版本保留控制台，以便 `S1_*` 诊断信息可见。

## 启动顺序

GPUI Kit 0.6.6 里只有一个合法启动顺序；乱序要么静默失败，要么直接 panic。这是该 crate 最重要的知识点。

| # | 步骤 | 位置 | 原因 |
| --- | --- | --- | --- |
| 0 | `parse_options()` → `resolve_launch_root(root)` | `:849-859` | 在任何 `App` 存在之前就需要失败并报错 |
| 1 | `lithe_gpui_settings::load()` → `locale` | `:891-894` | 配置文件是 UI 语言的“常规来源”；`--locale` 只是单次覆盖 |
| 2 | 解析 `--right-view` / `--left-view` id | `:866-887` | 在创建窗口前解析，避免启动状态被污染 |
| 3 | `gpui_kit::application().with_assets(LitheAssets).run(..)` | `:896-921` | `with_assets` 最多只可调用一次；第二次调用会覆盖之前的值。必须使用 `AllAssets`（1830 个 Lucide glyph），因为源码控制 UI 需要 `git-*` 图标 |
| 4 | `set_locale(&locale)` | `:966` | 必须在 `gpui_kit::init` 之前执行 |
| 5 | `gpui_kit::init(cx)` | `:967` | — |
| 6 | `Theme::change(Dark)` | `:972` | gpui-kit 默认是浅色；Lithe 的产品默认是深色。显式切换后，真正主题生效前的窗口框架也能看起来正常 |
| 7 | `Theme::set_scrollbar_mode(Always)` | `:975` | gpui-kit 默认滚动条会淡化；IDE 需要持久细条 |
| 8 | `init_store(cx, Init { loaded, theme_override })` | `:980-989` | 注册设置 `Entity` + `Global`。**并不应用主题**——见下文 |
| 9 | `watch_lithe_themes(cx)` | `:987` | 把 7 个内置主题写入 `<config>/themes/`，并启动 `ThemeRegistry::watch_dir`。**真正应用主题的地方在回调里**，因此 `S1_THEME applied=` 只会出现一次 |
| 10 | `install_actions(cx)` | `:988` | `Ctrl+,` |
| 11 | `set_shell_startup(cx, ..)` | `:998-1003` | 使用 `App::Global`，因为项目切换时会在 `window.replace_root` 里重建 `ShellWorkspace`，而 `main` 的局部变量无法被捕获。后果是：重建后的 shell 会重新应用 `--right-view` / `--left-view` |
| 12 | `install_open_project_action(cx)` | `:1008` | `Ctrl+O`。`App::on_action` 是**累积式**的，如果把 `ShellWorkspace::new` 注册在这里会堆叠处理器 |
| 13 | `menu_bar::install_key_actions(cx)` | `:1016` | 另外六个绑定，原因同样是“只在这里注册” |
| 14 | `startup_window_bounds(cx)` | `:1018` | — |
| 15 | `cx.spawn(.. cx.open_window(..))` | `:1020-1027` | 窗口是在**任务里打开**的，`run` 闭包会立即返回。`main` 中放一个 sleep loop 会饿死窗口创建任务（`:1245-1251`） |
| 16 | `ShellWorkspace::new(root, ..)` | `:1028-1037` | 组合根调用 |
| 17 | `store.attach_window(window, cx)` | `:1040-1042` | 系统外观监听器需要 `&mut Window` |
| 18 | 通过 `window.on_next_frame` 驱动的探针 | `:1043-1199` | 覆盖层只允许从事件回调或任务中打开。**例外**：会话探针在窗口创建后同步运行，因为无监督启动时帧回调根本不会触发（`session.rs:30-36`） |
| 19 | `cx.new(|cx| Root::new(workspace, window, cx))` | `:1201` | **`Root` 必须是窗口的第一层**——它承载对话框、sheet 和通知。任何在这之前打开 overlay 都会 panic |
| 20 | `--palette-keys` 分发 | `:1210-1238` | `dispatch_keystroke` 会同步调用 `Window::draw`，要求 root 已安装；此操作在 400 ms 后台计时器之后启动 |
| 21 | `return root_entity` | `:1239-1241` | 闭包必须返回 root view |

### 为什么 `init_store` 不应用主题

`ThemeRegistry::watch_dir` 是异步加载的。`init_store` 时，注册表里只有上游默认的 `Default Light` / `Default Dark`，因此此时校验主题 id 会误判断 `Lithe Dark` 缺失并静默替换——这确实曾经发生过（`settings/src/store.rs:165-187`, `settings/src/lib.rs:44-46`）。

## CLI

使用手写的 `parse_options()` 解析 `std::env::args()`，没有 clap。`Options` 在 `:98-235`，用法文本在 `:243-293`。只有三个 flag 属于产品功能，其余都是机器校验探针，且被明确标注（`:110-120`）。

| Flag | 语义 |
| --- | --- |
| `[<workspace-root>]` | 可选。为空则“打开最近存在的项目”，否则以当前目录作为工作区 —— 这是双击打开路径 |
| `--theme <id\|name>` | 单次启动覆盖，不写回。接受 `themes[].id` 或 `themes[].name` |
| `--locale <tag>` | 单次启动覆盖，不写回。gpui-kit 自带 `en` / `zh-CN` / `zh-HK` |
| `--open-settings`, `--open-palette` | 延迟到 `on_next_frame` 执行 |
| `--menu-probe <id> [action]`, `--menu-probe-delay`, `--menu-probe-open-ms` | 打开顶层菜单；`lithe.` 前缀是启发式规则，第二个参数不匹配则硬错误（参数迭代器是单向的） |
| `--theme-probe <index\|name>`, `--project-menu-probe`, `--branch-panel-probe` | — |
| `--right-view <id>`, `--left-view <id>` | `extensions`/`notifications`/`maven`/`spring`；`files`/`changes`/`search` |
| `--open-project-probe <dir>`, `--open-project-destination`, `--open-project-remember`, `--open-project-delay-ms` | 跑通“打开文件夹”链路 |
| `--appearance-revert <key>`, `--session-probe`, `--session-active`, `--session-hide-sidebar`, `--session-assert` | — |
| `--palette-keys <csv>`（可重复） | 用 `gpui_kit::Keystroke::parse` 解析；解析失败直接退出 |
| `--compact-menu-bar` | 菜单栏以“左上角图标 + 浮动胶囊”形式显示 |

`-h` 会返回用法，并以 `Err` 形式返回，`main` 进而将它打印到 stderr 并以 **2** 退出（`:849-855`）。

**启动根解析分成两段以便测试**：纯规则 `choose_launch_root(explicit, recent, fallback)`（`:703-711`，4 个单测位于 `:1260-1307`）和 I/O 包装 `resolve_launch_root`（`:719-770`），后者读取 `recent-projects.json`，标记缺失项，持久化后再回退。`fallback_directory()` 永远不会退出（`:790-815`）。

## 窗口边界

`startup_window_bounds`（`:69-88`）：最小 1024×680，兜底 1280×800。没有可用 display 时就使用 `Windowed`；否则取 `display.visible_bounds()` 的 94%，并排除 taskbar 影响，再夹到最小尺寸和可见尺寸之间，并居中。代码注释中提到了两种被拒绝方案：硬编码 macOS 尺寸，以及 `WindowBounds::Maximized` / `zoom_window()`——在当前 GPUI 版本中，这两者只会设置“恢复尺寸”，并不真正最大化窗口。

## 资源

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../assets"]
#[exclude = "icon-themes/lithe/**"]  #[exclude = "icon-themes/pierre/**"]
#[exclude = "icon-themes/symbols/**"] #[exclude = "icon-themes/material/**"]
#[exclude = "fonts/**"]  #[exclude = "legacy-ide-icons/**"]
#[exclude = "gutter-icons/**"]  #[exclude = "database-icons/**"]
#[exclude = "feature-icons/**"]  #[exclude = "README.md"]
pub struct LitheAssets;                                        // assets.rs:110-122
```

测量到的资源包大小为 **269 个文件 / 3 200 771 bytes（约 3.05 MiB）** —— `icons/**` 7 个位图，`images/logo.png` 1 个，`ui-icons/**` 157 个 IntelliJ expui SVG，`icon-themes/idea/**` 104 个。收窄前它曾是 1203 个文件 / 8 567 891 B。

每一个 `#[exclude]` 都有原因，去掉它而不接线对应资源只会让二进制变大：

| 排除项 | 原因 |
| --- | --- |
| 4 个 icon-theme 包 | 933 个文件 / 大约 5.09 MiB，**不可达**：当前活动文件图标主题 id 是编译期常量 `ACTIVE_FILE_ICON_THEME = "idea"`，运行时没有任何枚举目录 |
| `fonts/`、`gutter-icons/`、`database-icons/`、`feature-icons/`、`legacy-ide-icons/` | 来自已删除前端的归档资源，尚未接线 |
| `README.md` | 文档，不是资源；改动它会令文件/字节计数和回归测试的固定值失真 |

`#[folder]` 必须是绝对字符串——因为 crate 在 `gpui/crates/app`，资源在 `gpui/assets`，而 `rust-embed` 的 `get()` 做了朴素的 `starts_with` 边界检查，带有 `..` 的路径会被它拦住（`assets.rs:95-100`）。

`AssetSource` 实现（`:199-227`）会先尝试 embed，再尝试 `AllAssets`，把 `Err` 统一折叠成 `Ok(None)`。原因是 `AllAssets::load` 在 miss 时返回 `Err`；如果用 `?`，那么“glyph 未找到”会直接变成硬错误。

五个回归测试保护这个资源包，包括 `embedded_range_stays_within_the_documented_envelope`（锁定 269 / 157 / 104 / 7 / 1 以及精确字节总数）和 `probe_paths_are_loadable_from_the_embedded_table`（断言**存在于 embed table**，而不仅仅是 `load` 成功），因为 Lucide 里也恰好有 `icons/settings.svg`，单独做 `load` 断言会被回退资源骗过，无法证明真实加载路径来自 embed（`:403-418`）。

## 参考资料

- `rust/gpui/crates/app/src/main.rs`
- `rust/gpui/crates/app/src/assets.rs`
- `rust/gpui/crates/app/build.rs`, `lithe.rc`, `Cargo.toml`
- `rust/gpui/crates/settings/src/lib.rs`, `store.rs`, `theme.rs`, `paths.rs`
- `rust/gpui/crates/workbench/src/workspace.rs`
- `rust/gpui/README.md`
