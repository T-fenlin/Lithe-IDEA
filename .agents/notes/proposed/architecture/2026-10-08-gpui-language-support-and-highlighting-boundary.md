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

按依赖顺序分四片，每片可独立编译验证：

| 子片 | 范围 | 依据 |
| --- | --- | --- |
| **L3a** | `buffer.rs`(339 行) + `languages.rs`(42 行)：文本模型与 LSP 位置换算换到 `lithe_ui::editor::Buffer` + `lithe_ui::language` | 其余文件都建在 `Rope`/`Position`/`EditorState` 上（分别 44/84/14 处） |
| **L3b** | `completion.rs`/`hover.rs`/`code_actions.rs`/`problems.rs`/`diagnostics.rs` → 库侧 `hints`/`hover`/`refactor`/`problems` | 库侧已有对等件，属重接而非新建 |
| **L3c** | `editor_view.rs`(2194 行)：编辑器部件换成库侧 `CodeEditor`/`view` | **高亮此时才在产品里可见**，因为高亮由渲染路径决定 |
| **L3d** | `navigation.rs`/`go_to_line.rs` 收尾，摘掉该 crate 的 `gpui-kit` | 收尾片 |

先做 L3a 的原因：它是其余三片共同的地基（位置与 rope 语义），而且**不需要**先动 2194 行的部件文件。

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
