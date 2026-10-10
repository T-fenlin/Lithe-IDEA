# Lithe-UI 组件库文档（中文）

`lithe-ui` 是 Lithe 的 GPUI 组件库：窗口里的一切控件都来自这里，宿主（macOS 的 Swift 产品、Windows 的 Tauri 产品、Rust 的 `lithe-gpui` workbench）只负责装配与业务状态。

这套文档回答一个问题：**组件库里到底有哪些组件，怎么用。** 全部说明为中文，代码标识符、类型名保持英文原文。

## 目录

| 文件 | 内容 |
| --- | --- |
| [`components.zh-CN.md`](./components.zh-CN.md) | **总览清单**：`src/` 下 45 个模块的全部公开导出，每个组件一句话说明 + 最小可用片段 |
| [`components/01-基础与排版.md`](./components/01-基础与排版.md) | `primitives`、`typography`、`i18n`、`interaction`、`rendering`、`motion` |
| [`components/02-主题与设计token.md`](./components/02-主题与设计token.md) | `theme`：调色板、尺寸/圆角/字号 token、深浅模式、宿主如何接管配色 |
| [`components/03-布局与外壳.md`](./components/03-布局与外壳.md) | `layout`、`shell` |
| [`components/04-按钮.md`](./components/04-按钮.md) | `buttons` |
| [`components/05-表单.md`](./components/05-表单.md) | `forms`（含快捷键录制 `HotkeyInput`） |
| [`components/06-菜单与浮层.md`](./components/06-菜单与浮层.md) | `menus`、`overlays` |
| [`components/07-导航.md`](./components/07-导航.md) | `navigation`（含命令面板、Go to Line、标签页） |
| [`components/08-列表与数据展示.md`](./components/08-列表与数据展示.md) | `lists`、`data_display` |
| [`components/09-表格与图表.md`](./components/09-表格与图表.md) | `tables`、`charts` |
| [`components/10-编辑器与终端.md`](./components/10-编辑器与终端.md) | `editor`、`terminal`、`language`（含**快捷键编辑器** `KeybindingsEditor`） |
| [`components/11-设置与账户.md`](./components/11-设置与账户.md) | `settings`、`account`（含**快捷键列表** `KeyboardShortcutsList`） |
| [`components/12-工程与协作.md`](./components/12-工程与协作.md) | `git`、`project`、`files`、`collab`、`chat`、`messaging`、`agent` |
| [`components/13-调试与监控.md`](./components/13-调试与监控.md) | `debug`、`devtools`、`tooling`、`dashboard` |
| [`components/14-其余业务域.md`](./components/14-其余业务域.md) | `feedback`、`calendar`、`documents`、`onboarding`、`misc`、`media`、`mail`、`maps`、`canvas`、`finance`、`generative` |

想知道"某个组件在哪"，先查总览清单的模块表；想知道"这个组件怎么构造、有哪些链式方法"，再打开对应域的文件。

`src/` 下 45 个模块的归属（按域文件）：

| 域文件 | 覆盖的模块 |
| --- | --- |
| 01 | `primitives` `typography` `i18n` `interaction` `rendering` `motion` |
| 02 | `theme` |
| 03 | `layout` `shell` |
| 04 | `buttons` |
| 05 | `forms` |
| 06 | `menus` `overlays` |
| 07 | `navigation` |
| 08 | `lists` `data_display` |
| 09 | `tables` `charts` |
| 10 | `editor` `terminal` `language` |
| 11 | `settings` `account` |
| 12 | `git` `project` `files` `collab` `chat` `messaging` `agent` |
| 13 | `debug` `devtools` `tooling` `dashboard` |
| 14 | `feedback` `calendar` `documents` `onboarding` `misc` `media` `mail` `maps` `canvas` `finance` `generative` |

四十五个模块，一个不落。

## 三个必须先知道的约定

### 1. 一切样式来自主题 token，不得写死颜色与尺寸

组件不读硬编码色值，只读 `theme::Theme` 的全局（`src/theme/mod.rs` 的 `ActiveTheme::theme()`）：

```rust
let colors = &cx.theme().colors;   // 语义色：bg / surface / title_bar / hover / active / border / fg / fg_muted …
let height = cx.theme().control_height(Size::Small); // 尺寸：20 / 24 / 28 / 32（随密度浮动）
let radius = cx.theme().radius(Radius::Md);          // 圆角：4 / 6 / 8 / 12
let size   = cx.theme().text_size(TextSize::Sm);     // 字号：11 / 12 / 13 / 14 / 16 …
let shadow = cx.theme().elevation(Elevation::Floating);
```

`Palette` 的全部字段见 `src/theme/palette.rs:112-154`（`bg`、`surface`、`tab_bar`、`title_bar`、`status_bar`、`sunken`、`overlay`、`hover`、`active`、`border`、`border_strong`、`fg`、`fg_muted`、`fg_subtle`、`fg_disabled`、`accent`、`accent_hover`、`on_accent`、`primary`、`on_primary`、`focus`、`link`、`selection`、`success`/`warning`/`danger`/`info` 及其 `*_subtle`、`backdrop`、`glass`、`shadow`、`tooltip_bg`、`tooltip_fg`、`paper`、`ink`）。深浅两套默认值在 `Palette::light()`（`:195`）与 `Palette::dark()`（`:271`）。

> **chrome 槽位**：`tab_bar` / `title_bar` / `status_bar` 是"窗口外壳"专用的三个槽，与页面底色 `bg` 不是一回事，不要互相顶替（见 `palette.rs:96-111` 的说明）。

### 2. 用一个组件的三步

1. 宿主启动时调一次 `lithe_ui::init(cx)`（`src/lib.rs:58`）：注册字体、装好主题全局、绑定 Tab / Shift-Tab、表单、编辑器、终端、文档的按键。资源必须挂上：`Application::with_assets(lithe_ui::Assets)`，否则启动即 panic。
2. 需要受控文本时用 `InputState` 实体：`let state = cx.new(|cx| InputState::new(cx));`，再把它交给 `Input::new(&state)` 这类组件。
3. 组件是 `RenderOnce`（一次性渲染）或 `Render`（实体视图）两种形态之一：`RenderOnce` 的像 `Button::new("id")` 直接放进元素树；`Render` 的像 `CodeEditor::new(text, window, cx)` 先建实体再 `.clone()` 进树。

### 3. 改了组件要同步画廊

画廊是组件的可视清单与证据来源（`examples/gallery/`）：

- 页面：`examples/gallery/pages/<域>/`，新页面要在 `pages/mod.rs` 的 `ALL` 里注册；
- 故事清单：`examples/gallery/stories.json`；
- 文案：`examples/gallery/i18n.rs`；
- 截图与交互证据：`capture.rs` / `probe.rs` / `script.rs` / `step.rs`。

跑画廊（target 目录必须放在工作区外）：

```bash
cd D:/developmentProjects/rust/Lithe-IDEA/rust/lithe-ui
CARGO_TARGET_DIR=D:/developmentProjects/rust/lithe-ui-target cargo build --example gallery
```

## 文案与图标

- 用户可见文案走 `rust_i18n`：`locales/ui.yml`，组件里用 `tr(key)`，不要写死中文/英文字符串。
- 图标走 `primitives::IconName` + `assets/icons/`，用 `Icon::new(IconName::Check)`；宿主自己的图标集用 `MenuItem::icon_element(...)` / `Button::icon_element(...)` 传入。
- 文件图标主题：`primitives::IconTheme`（`src/primitives/files.rs`）。

## 文档维护约定

- 组件名、方法名、参数、token 名必须能在 `src/` 里 grep 到，不臆造；
- 纯数据模型（`Task`、`Event`、`Commit` 这类）在清单里标注为「数据模型」，与可渲染组件分开；
- 每个域文件开头是"本域速查表"，正文按「构造 → 链式方法 → 事件回调 → 典型组合」展开；
- 组件改动后同步更新本目录对应文件与画廊。
