# Agent 笔记：GPUI 语言支持与语法高亮的分层与插件边界

状态：提议中

## 先说结论

语法高亮（把源码染成关键字、字符串、注释等颜色）不从上游编辑器里"借"，而是作为**可插拔的语言
支持层**由我们自己实现。组件库 `lithe-ui` 只提供两样东西：**注册表**（语言支持插件往里注册语言）
与**高亮引擎**（tree-sitter 解析、执行高亮查询、把结果映射到主题配色）。组件库**不认识任何具体
语言**；Java、YAML 这些语言由语言支持插件声明，当前是 `rust/lithe-gpui/crates/languages`。

直接复用 Zed 的编辑器栈不可行：它的 `editor`、`language`、`theme`、`rope`、`multi_buffer` 等 crate
都是 **GPL-3.0-or-later**，而且 workspace 根声明 `publish = false`（不发布到 registry）；只有
`gpui` 是 Apache-2.0。对一个 Apache-2.0/MIT 基线的闭源产品，把 GPL 代码链进发布物会带来 GPL 义务。

开发者需要记住三件事：

1. **加一种语言** = 在语言支持插件里加一条声明（当前需要重新构建；将来装载源换成扩展目录里的
   动态库时，声明方式不变）。**不要**为了加语言去改组件库。
2. 编辑器是**按行**取色的，而 tree-sitter 必须**整篇**解析——所以高亮结果是按缓冲区缓存、按文本
   版本失效的，不能每次渲染重新解析。
3. 高亮查询里出现我们不认识的 capture 名（例如 `@spell`）就**留空**，不要硬塞到某个颜色上。

## 问题

迁移前，宿主的编辑器平面（`rust/lithe-gpui/crates/editor`，5256 行）整体建在 `gpui-kit` 的编辑器上：
`EditorState`、`Rope`、`Position`、`RopeExt`、`TabSize`、`GrammarConfig`、`LanguageRegistry`、
`HighlightTheme` 都由上游提供，语法高亮也是上游开的 7 个 tree-sitter grammar 加 XML 共 8 种。

迁移到组件库后出现两个缺口：

- 组件库**没有语法层**：`tree_sitter`、`LanguageRegistry`、`GrammarConfig`、`HighlightTheme` 在
  `rust/lithe-ui/src` 里的命中数**全是 0**，只有 `forms::{lex, code_highlights}` 一个简易词法器
  （`editor/syntax.rs:57,113` 调它）。
- 上游那套 `GrammarConfig`/`LanguageRegistry`/`HighlightTheme` 会随 `gpui-kit` 退场，需要一个替代物，
  否则编辑器迁移会**降级**（从 tree-sitter 掉回词法器）。

同时要求"语言支持可做成插件"，也就是：加语言不应该依赖重新发布组件库。

## 提案

### 分层

| 层 | 归属 | 说明 |
| --- | --- | --- |
| 编辑平面（缓冲区、选择、折叠、括号配对） | `rust/lithe-ui/src/editor/` | 组件库已有；本提案不改它的模型 |
| **语言支持（语法 + 高亮）** | `rust/lithe-ui/src/language/` | **只放接口与引擎**：注册表、`Language` 声明、tree-sitter 引擎、capture→主题槽映射 |
| **具体语言** | `rust/lithe-gpui/crates/languages` | 第一方语言支持插件：8 种语言的 grammar 与高亮查询 |
| 语法配色 | `rust/lithe-ui/src/theme/` 的 `Syntax` | 13 个颜色槽，是配色的**唯一真源** |
| 语义（补全、跳转、诊断、调试） | `rust/lithe-core` | 语言服务器与调试适配器都在后端，编辑器只做客户端 |

### 接口

- 声明：`Language { id, name, extensions, grammar: fn() -> tree_sitter::Language, highlights, line_comment }`。
  grammar 用函数指针，因为 grammar 的构造不是 `const`，而声明要能放进 `static`。
- 注册：`language::register(&'static [Language])`。**同批里重复 id 或重复扩展名直接 panic**——那是插件
  自身的错误，静默丢掉一种语言更难查；跨批重复注册只记一条警告（宿主与插件都可能尝试初始化）。
- 查询：`language::by_extension` / `by_path` / `all`。**没有注册就返回"没有"**，由调用方走退化路径。
- 引擎：`Highlighter::new(&'static Language) -> Option<Self>`、`reparse(text)`、`spans_in(range)`。
  `spans_in` 负责裁剪到请求范围；同起点的 span **外层在前**，这样渲染时里层覆盖外层，与语法嵌套一致。
- 颜色：`Slot`（13 个，与 `theme::Syntax` 一一对应）。`Slot::from_capture(name)` 按名称**首段**判定，
  所以 `function.method`、`type.builtin`、`variable.parameter` 都能落到同一个槽；不认识的名字返回 `None`。

### 装载策略：先静态，再动态，接口先立

- **现在（静态，已落地）**：grammar 作为依赖编进二进制，与 `gpui-kit` 时代一致。加语言需要重新构建。
  维护者 2026-10-08 明确：**动态装载（L4）先不做，静态够用**。
- **将来（动态，暂缓）**：装载源换成 `extensions/` 目录里的语言支持包（动态库 + 查询 + 配置），并遵守
  仓库既有纪律：资源在**构建阶段**固定来源并签名，运行时只从已安装插件发现入口，不写 app bundle 或安装目录。
  触发条件是"确实需要第三方在不重新发版的前提下加语言"；没有这个需求前不做，以免 ABI 与打包成本
  拖住接口定型。
- 关键点：这次定的是**接口与注册表**，不是加载器。换成动态库时，`Language` 声明与注册表 API 不变，
  只替换"从哪里拿到 grammar"。**现在不做加载器，不会阻碍将来做**。

### 已知边界（不假装完整）

- Markdown 只注册了 block 语法与 block 查询；行内（强调、链接、行内代码）需要
  `tree_sitter_md::INLINE_LANGUAGE` + `HIGHLIGHT_QUERY_INLINE` 的第二次解析（术语上是 injections），
  等引擎支持注入后再补。
- XML 只注册了 XML 语法，DTD 是另一个 grammar，尚未注册。

### 静态语言支持什么时候才"看得见"

语言注册表与引擎已经能用（`lithe-gpui-languages` 的端到端测试证明 `class`/`1` 能落到正确槽位），
但**界面上还看不到**：产品编辑器 crate（`rust/lithe-gpui/crates/editor`，5256 行）整块建在
`gpui-kit` 的编辑器上，高亮由上游自己的 `GrammarConfig`/`LanguageRegistry` 完成。要让我们的注册表
生效，编辑器必须走组件库的渲染路径。

**这不是能按文件分片的改动**（2026-10-08 侦察后修正过先前的切法）：

- `gpui-kit` 的 `EditorState` 是该 crate 的中枢类型，被 **6 个文件**引用
  （`buffer.rs` 3 处、`code_actions.rs` 4、`completion.rs` 2、`diagnostics.rs` 2、`editor_view.rs` 6、`hover.rs` 2）；
- 产品 `Buffer` 的 `editor: Entity<EditorState>` 只被 `editor_view.rs` 用，但用了 **54 次**；
- 上游 `Editor` 部件的渲染落点只有 **1 处**（`editor_view.rs:2157`）。

所以换模型必然级联整个 crate：**要么整体换掉，要么别动**。只换那 1 处渲染点做不了——组件库的
编辑器要渲染它自己的模型。维护者的选择是**完整移植、一笔原子提交**（而不是先做一个"只读高亮预览"
的楔子，也不是继续让语言层停留在测试级）。

目标映射（侦察已确认）：`EditorState` → `CodeEditor`（`state.rs`，Entity + `EditorEvent`）；
`Rope`/`Position` → `Buffer` + `Selection`；provider → 库侧 `hints`/`hover`/`refactor`/`problems`；
诊断/inlay hints/code lens/ghost text/diff/git 标记/断点 → `CodeEditor` 的 `set_*`；
`LanguageRegistry`/`GrammarConfig` → **本笔记的注册表**（`crates/editor/src/languages.rs` 那 42 行
注册 XML 的文件随之删除）；`HighlightTheme` → `theme::Syntax`。

### LSP 的界面编排要重建，但**数据层复用**

产品编辑器今天不是自己接 core，而是把语言服务的结果适配进 `gpui-kit` `EditorState` **内置的 LSP
子系统**（`state.lsp_mut()`、`lsp.completion_provider`、`lsp.completion_menu.max_width`、
`Rc<dyn CompletionProvider>`）。删掉上游编辑器就删掉了这套子系统，所以补全/悬停/代码动作的
**编排**（何时弹、弹在哪、键盘选择、接受后如何落编辑）必须重建。

好消息是数据层不用动：`rust/lithe-gpui/crates/java`（`JavaLanguageService`，4000 行）
**只依赖 `shared` 与 `url`，完全不碰 `gpui-kit`**，已经暴露 `prepare` / `definition` / `hover` /
`completion` / `code_actions` / `diagnostics` / `workspace_diagnostics` / `sync_document`。
editor crate 现在只是把这些结果包成上游的 provider（`completion.rs:39 install(...)`、
`JavaCompletionProvider`）。

组件库这一侧提供的是**展示件**而不是子系统：`hints.rs` 有 `CompletionItem`/`CompletionMenu`
（`selected(usize)`、`on_pick`）与 `SignatureHelp`，`hover::HoverInfo`，`refactor::CodeActionMenu`；
但 `CodeEditor` **没有** completion/hover/action 钩子，`view.rs` 也不渲染这些弹层。浮动层的落点用
已经建好的门 G1 宿主（`lithe_ui::shell::{WindowRootExt::push_layer, open_rooted_window}`），
即"组件库给展示件、宿主负责编排与定位"。

**因此决策**：编排留在产品 crate（`rust/lithe-gpui/crates/editor`），数据仍走
`lithe-gpui-java`；若编排需要而组件库缺少的能力（例如光标锚定的弹层定位助手），按仓库既有口径
**补进 `lithe-ui` 或映射到最接近的 API**，不用空壳糊过去，也不把产品语义塞进通用组件库。

### L3 的前置缺口：组件库缺「LSP 请求接缝」（2026-10-08 判定，尚未开工）

按映射表动手前，子代理侦察后**停下来**（产品代码一行未改），我逐条复核确认：

- **组件库没有任何 provider 接缝**：`CompletionProvider|HoverProvider|CodeActionProvider` 在
  `rust/lithe-ui` 全仓命中 **0**；`CodeEditor` 唯一的扩展点是 `chat_field`。而产品每一处都打在
  gpui-kit 的私有 LSP 槽上：`completion.rs:31,47` 的 `lsp_mut().completion_provider` 与
  `.completion_menu.max_width`、`hover.rs:17` 的 `hover_provider`、`code_actions.rs:18` 的
  `code_action_providers`。库侧 `hints::{CompletionItem, CompletionMenu}`、`hover::HoverInfo`、
  `refactor::CodeActionMenu` 都是 `RenderOnce` **展示件**：构造时就要静态数据，没有异步取项、没有锚点。
- **读不到也设不了光标的字节偏移**：`CodeEditor` 公开面只有 `position()`（行列，从 1 起）与 `select()`，
  而 LSP 请求要的正是字节偏移 ⇒ `editor_view.rs` 的 `navigate_to_definition`(812)、
  `open_go_to_line`(1702)、`show_hover_at_caret`(1848)、`open_at`(536)、`apply_definition`(928)、
  `restore_entry`(1089)、`go_to_line_column`(1814) 全部落空；`apply_lsp_edits`（`code_actions.rs:153`）
  在库侧同样没有对应物。
- **视图度量是私有的**：`visible_row_range` / `line_height` / `scroll_offset` / `offset_at` 都是
  `pub(crate)`，而宿主的断点 gutter、滚轮与 alt+click 要用。
- **没有 LSP 交互的生命周期**：库侧 `EditorEvent` 只有 `Changed/Lens/Asked/ChatClosed/Unlock`，
  按键绑定里没有补全触发 ⇒ 触发、锚点跟随、键盘选择、接受、悬停布局、灯泡弹出、编辑应用都要从零做。

**决策**：把这三类能力（provider/请求接缝、光标字节读写、视图度量）作为**独立一片补进 `lithe-ui`**，
语义按上游对齐（不给上游独有 API 补别名，也不把产品语义塞进通用组件库）；这一片完成前不启动 L3 的
原子移植。硬做 L3 只剩两条错路——把 LSP 交互静默降级成静态菜单，或在组件库里塞一个 IDE 子系统——
两条都不接受。

**这一片已落地（2026-10-08，`rust/lithe-ui/src/editor/`，测试 1568 → 1580 passed）**：

- 光标字节偏移：`cursor_offset()`、`set_cursor_offset(offset, window, cx)`（越界与切字符经 `clamped_range`
  拉回、收单点光标、最小滚动、焦点交给编辑器——与上游 `set_cursor_position` 同口径，产品现有 4 处调用点依赖它）。
- LSP 批量编辑：`apply_edits(&[(Range<usize>, String)], cx)`（起点降序、落笔前再夹一次、重叠不 panic、
  一次算一个撤销步、光标映射到所在编辑的替换文本末尾；只读编辑器拒绝改动）。
- 视图度量与锚点：`visible_row_range()`、`line_height()`、`scroll_offset()`、`offset_at(...)`、
  `bounds_for_offset(offset, cx)`（只回答已布局且可见的位置）。`bounds_for_offset` 需要 `cx`：
  有 inlay hint 的行要按渲染后的列算，不给 `cx` 就是**假装正确**。
- 请求事件与数据推送：`EditorEvent::CompletionRequested(usize)` / `HoverRequested(usize)`；
  `ctrl-space` 显式触发、`auto_completion(bool)`（默认关，只在恰好敲进一个标识符字符后发）、
  悬停按标识符进入发一次（同一个符号只问一次）；`set_completions` / `clear_completions` /
  `completions` / `present_hover` / `hover`（库只存、只画，弹层与键盘导航留宿主）。

**L3 仍要履行的三件事**（写在代码 doc 里，也记在这里）：

1. **gutter 需要行号**：`visible_row_range()` 给的是**行表下标**，有折叠/ghost/lens 时与行号不等价；
   产品的断点 gutter 现在假设 `row + 1 == line`，会错位。组件库要补一个 O(1) 的"行下标 → 行号"查询
   （折叠体与越界返回"没有"，不要瞎猜），已作为本片的紧接补丁。
2. **补全/悬停的数据生命周期由宿主负责**：库不随文本变化自动清（`clear_completions` /
   `present_hover(None)` 由宿主调），也没有"指针离开符号"的事件——收层时机归宿主。
3. **弹层编排归宿主**：定位用 `bounds_for_offset`，浮层用门 G1 的
   `lithe_ui::shell::WindowRootExt::push_layer`。

### L3 第二次开工被拦：弹层键盘接管与软换行（2026-10-08，未改一行代码）

委托出去做移植的子代理在动手前停下并交了证据，我逐条复验成立：

- **宿主拿不到弹层需要的按键**：`keys.rs:54` 是 `pub(crate) const CONTEXT = "ElyEditor"`，
  `view.rs:423` 在编辑器根节点 `.key_context(CONTEXT)`，43 个 action 全绑给 `CodeEditor` 自己，
  `keys.rs:143` 的 `listen()` 是 `pub(crate)`。于是"宿主渲染补全弹层"在实现上不成立：编辑器持焦点时
  Up/Down 被库吃掉去移光标、Enter 被吃掉去换行，浮层收不到导航键；反过来让浮层抢焦点则打着字就断。
  补全是在用功能（`workbench/src/workspace.rs:939,954` 接设置开关），不能丢。
- **软换行完全没有**：`soft_wrap|word_wrap` 在整个 crate 里 **0 命中**，`Options` 没这个字段，
  `view.rs:379` 走 `uniform_list` 一行一行、`row.rs:98` 是 `whitespace_nowrap()`；而产品 `word_wrap`
  **默认 true**（`settings/src/schema.rs:288`）、设置页可切（`dialog.rs:1253-1262`）。照搬即静默降级。
- 两个较小的缺口：运行期设滚动位置（只有只读 `scroll_offset()`，`reveal` 是 `pub(crate)`）；
  以及宿主测试支撑（`TestSupportExt` 0 命中，workbench 两条 UI 测试要重写——这条属宿主侧工作）。

**决策（按"缺能力补进 lithe-ui、不允许静默降级"的既定口径）：先补库，不做可见降级。** 拆两片：

- **片 A（已落地，2026-10-08）**：把"弹层接管键盘"做成**通用**接缝——`CodeEditor::set_popup_active(bool, cx)`
  加 `EditorEvent::PopupKey(PopupKey)`（`Up/Down/PageUp/PageDown/Enter/Tab/Escape/Backspace` 在接管期间
  **库不执行自己的动作、改为上报**，其余按键与 IME 照常，编辑器保持焦点）；外加
  `set_scroll_offset(point, cx)`（夹到合法范围）。**没有引入真正的按键转发机制**：8 个既有 action 的
  处理里先看开关，命中就发事件后返回，未改 `listen()` 的可见性与 `CONTEXT`；仲裁点唯一
  （`report_popup_key` 未激活时直接返回、不发事件也不 notify），所以**关着时逐字节等于改动前**
  （测试 1581 → 1587 passed）。两处口径由实现拍板并写进 doc：**修饰形式不接管**（`shift-up`/`shift-tab`/
  `ctrl-backspace` 等仍归编辑器），`enter`/`escape` 的守卫**先于**原有的焦点检查与内联 chat 分支。
  顺带记一个后人会踩的坑：gpui 的 `UniformListScrollHandle::max_offset()` 返回的是**正值"可行进距离"**，
  合法滚动区间是 `-reach..0`（内容装得下时 `reach == 0`），按负值夹取会直接 panic。
  注意 `EditorEvent` 新增变体对**穷尽 match 的消费方是破坏性**的：本仓内消费点都在 lithe-ui，产品侧
  在 L3 里接这个事件时要补分支。
- **片 B（大，待排期）**：**软换行**。这是编辑器的布局特性（行表一行一行 → 需要按宽度折行的子行、
  影响度量、光标移动、折叠、gutter 与 `line_at_row` 的语义），规模明显大于片 A，必须独立成片并带自己的
  测试；片 B 落地前 L3 无法在不丢软换行的前提下收口。

另记两条**宿主侧**工作（不是库缺口）：产品用 `Position` 做 UTF-16 列换算，迁后要改用
`Buffer::point/offset`（口径是**字符列**不是 UTF-16 列，相关测试要跟着改）；代码动作当前在产品里
**用户不可达**（无触发键），迁后应做成宿主菜单入口，建议独立后续片。

**顺带修掉的真缺口（已落地）**：语言 id/扩展名/文件名保真度。产品的
`crates/editor/src/buffer.rs:15-44` 与设置侧 `lithe.projectLanguages` 用的是 `bash`（不是 `shell`）、
`json/jsonc`、`ktm`、`xsl/xslt/wsdl`，以及 `mvnw`/`gradlew`/`mvnw.cmd`/`gradlew.bat` 的无扩展名映射。
**id 一漂移不会报错，只会让"卸载项目语言包"再也关不掉高亮**。现在注册表有 `filenames` 字段与
`by_filename`/`by_path` 两级查找，并有保真度测试守着这 9 个 id 与 12 条路径映射。

## 考虑过的备选方案

1. **直接依赖 Zed 的编辑器与语言栈**。否决。一手证据（`gh api` 取 Zed main 的 crate manifest）：
   `crates/{editor,rope,text,language,lsp,project,ui,multi_buffer,theme}/Cargo.toml` 的 `license` 都是
   `GPL-3.0-or-later`，且 workspace 根 `publish = false`；`crates/rope` 在 crates.io 上不存在
   （registry 上的同名 crate 是 2014 年另一个 MIT 项目，已 yank）。附带的技术障碍：Zed 的编辑器绑定
   它自己那个 `gpui` commit，而我们精确钉 `gpui-pre =0.3.8`，两者不同源会出现两个 `gpui` 实例。
2. **一上来就做动态库装载**。推迟。动态装载要做 ABI 与版本校验、跨平台打包、崩溃隔离；而现在
   `gpui-kit` 提供的高亮本身就是静态的，没有"必须今天动态"的压力。先立接口是更低风险的一步。
3. **按名字把 capture 硬映射到最接近的颜色**（例如把 `@spell` 塞给 `comment`）。否决。那会让界面
   看起来"覆盖了"，实际上把没有语言支持的证据抹掉，违背"缺则留空"。
4. **让组件库内置 grammar**（`lithe-ui` 直接依赖 8 个 grammar crate）。否决。组件库是通用组件层，
   内置语言会让每个使用者承担体积与构建代价，也与"语言支持是插件"的目标相悖。
5. **继续用 `gpui-kit` 的 `GrammarConfig`/`LanguageRegistry`**。只能过渡。它随 `gpui-kit` 一起退场，
   而且它把"语言"硬编码在组件库里，正好是我们要避免的形状。

## 验收标准

- 组件库里**没有**任何具体语言的名字：`rust/lithe-ui/src` 里不出现 `tree_sitter_java` 之类的依赖。
- 加一种语言只改语言支持插件（`crates/languages`）与一处 `register`，不改组件库引擎。
- `Highlighter::new` 失败（grammar 或查询装不上）时返回 `None`，调用方退回词法器，**不 panic**。
- 编辑器渲染路径在没有任何语言注册时，行为与迁移前**逐行一致**。
- 语言列表与 `gpui-kit` 时代打开的 8 种一一对应，编辑器迁移后不降级。

## 风险

- **性能**：整篇解析发生在渲染路径附近。缓解：按 `Buffer` 缓存、按文本版本失效；解析失败时不重试
  同一份文本。
- **原生库冲突**：`tree-sitter` 带 `links = "tree-sitter"`，同一依赖图里只能有一份。缓解：组件库与
  `lithe-core` 都钉 `0.26`，grammar 版本与本仓 lock 中既有那份一致。
- **插件重复注册**：宿主与插件都可能调用注册。缓解：跨批重复只警告；同批冲突 panic。
- **Markdown/XML 的覆盖不完整**已在模块文档中写明，不当作已完成。

## 验证

```bash
# 组件库：注册表、capture 映射、配色、未声明扩展名留空
cd rust/lithe-ui && cargo test -p lithe-ui --features test-support

# 语言支持插件：真实 grammar 端到端（`class` → 关键字槽、`1` → 数字槽）
cd rust/lithe-gpui && cargo test -p lithe-gpui-languages

# 宿主整体
cd rust/lithe-gpui && cargo check --workspace --all-targets

# 笔记格式
./scripts/verify-agent-notes.sh
```

**"装了但装不上 → 退回词法器"这条分支的覆盖是分开的**，不要误以为有一条测试同时覆盖它：
组件库**没有** grammar 依赖（`tree-sitter` 只提供引擎），因此在组件库里造不出"声明了语言但 grammar
/查询装不上"的输入。现在的覆盖是两半互补——语言支持插件里用真 Java grammar + 故意写坏的查询，断言
`has_highlighter() == false`、`highlight_spans()` 为空且连问两次不 panic；组件库里断言"没有可用高亮器"
这一同一分支给出的颜色与逐行词法器**逐字节相同**。

## 适用范围

- `rust/lithe-ui/src/language/`（接口与引擎）
- `rust/lithe-gpui/crates/languages/`（第一方语言支持插件）
- `rust/lithe-ui/src/editor/`（高亮缓存与取色入口）
- `rust/lithe-gpui/crates/editor/`（尚未迁移：仍建在 `gpui-kit` 的编辑器上）
- `rust/lithe-gpui/extensions/`（将来动态装载语言支持包的位置）
