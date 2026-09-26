# gpui 编辑器接 JDTLS 智能提示（代码补全）：可照抄的实现调研

> 目的：回答「让 gpui 编辑器真的弹出 JDTLS 补全菜单，最少要做哪几件事」。
> 全部结论只依据**本机 cargo registry 里已发布的 0.6.6 源码**、仓库现有代码、`shared/contracts/rust-core-api.md`
> 与 `rust/lithe-core/src/lsp/**` 的**实际实现**，逐条给 `文件:行号`。
> 本轮**没有**改任何代码、**没有**跑 `cargo`（未取 `gpui/target` 锁），因此所有"构建代价"都是推断，见 §7。

## 0. 证据基准（简写）

| 简写 | 绝对路径 |
| --- | --- |
| `BASE` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-base-0.6.6\` |
| `CMP` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-component-0.6.6\` |
| `KIT` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-0.6.6\` |
| `GPUI` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-pre-0.3.6\` |
| `CORE` | `rust/lithe-core/`（仓库相对） |
| `DOC` | `shared/contracts/rust-core-api.md`（仓库相对） |
| `REPO` | `gpui/`（仓库相对） |

所有仓库内路径都相对仓库根 `D:\developmentProjects\rust\Lithe-IDEA`。

### 0.1 ⚠️ 写就时的**工作区实际状态**（本调研不是从干净树出发的）

本调研进行期间，`gpui/crates/java/src/service.rs` **已有未提交的改动**（`git status` 显示 `M`，+225/−1），
内容正是本文 §6 第 1 步要做的事 —— 也就是说**第 1 步已经落地了，只是还没提交**。已存在的名字（请沿用，不要另起一套）：

| 已有 | 位置（写就时的行号） |
| --- | --- |
| `pub struct JavaCompletionItem { label, insert_text, kind, detail, documentation, sort_text, filter_text, text_edit }` | `service.rs:46-87` 一带 |
| `pub struct JavaTextEdit { start, end, new_text }` / `pub struct JavaPosition { line, utf16_column }` | 同上 |
| `pub fn JavaLanguageService::completion(&self, file_path, text, line, utf16_column) -> Result<Vec<JavaCompletionItem>, String>` | `service.rs:232-…` |
| `const COMPLETION_TIMEOUT: Duration = Duration::from_secs(15)` | `service.rs:38-42` |
| 诊断行 `S1_JAVA_COMPLETION uri=… line=… col=… items=… ms=…` | `service.rs` 的 `completion` 尾部 |
| `fn parse_completion_item` / `parse_text_edit` / `parse_position` / `plain_snippet` | `service.rs` 的解析段 |
| 单测 `completion_item_reads_the_normalized_fields` | `service.rs` 测试模块 |

**它自己声明但尚未接线的两点**（`service.rs` 的 `completion` 文档注释里写了）：
① "超时由编辑器回落到 Core 的轻量补全（`lsp.builtinCompletions`）" —— 回落还没写；
② `textEdit` 缺失会导致"插重复"（`Sys` + `System` → `SysSystem`）—— 已在结构里带上 `text_edit`，上游消费侧见 §3.3。

→ **因此本文的 §6 第 1 步只列"待验证"，第 2 步起才是真正没做的部分。**

**（本节写就后不久，主代理又推进了一步 —— 第 2 步也已落地：）**
`gpui/crates/editor/Cargo.toml` 新增 `lsp-types = { version = "0.97", features = ["proposed"] }` 与 `anyhow = "1"`；
新增 `gpui/crates/editor/src/completion.rs`（`JavaCompletionProvider`，含 `lsp.builtinCompletions` 回落、`filterText`/`sortText` 透传、`CompletionItemKind` 映射、`text_edit` 透传、纯函数 `triggers_on` 便于单测）；
`gpui/crates/java/src/lib.rs` 导出 `JavaCompletionItem` / `JavaPosition` / `JavaTextEdit`。
**它把本文 §6 第 2 步的每条要求都做到了**（我逐条核对过 `completion.rs` 的 `to_lsp_item`、`is_completion_trigger`、`builtin_completions`）。
**两条 review 性质的观察（都不阻塞）**：
1. `anyhow` **不需要**单独声明：`gpui` 已经把它重导出（`GPUI\src\gpui.rs:76` 的 `pub use anyhow;`、`:93` 的 `pub use anyhow::Result;`），而 `gpui-kit` 用 `pub use ::gpui::*` 全量转发（`KIT\src\lib.rs:95`）→ `gpui_kit::anyhow` / `gpui_kit::Result` 可直接用。多声明一份不会引入第二个 crate（锁里只有 `anyhow 1.0.104`），只是多一处需要跟着上游口径走的地方。
2. `lsp-types` 那句注释说"特性必须逐字一致"——**方向对但机制不对**：Cargo 的 feature 是**并集**，`gpui-component` 已经打开了 `proposed`（`CMP\Cargo.toml:304-306`），所以即使不写 `features` 也只有一个 `lsp_types`。（写全更清楚，建议保留。）

---

## 1. 结论速览

要看到补全菜单真的弹出，**最少 4 件事**；其中 3 件是上游"白送"的渲染层，1 件是我们唯一必须写的。

| # | 要接的事 | 一句话 | 是否"白送" | 落点 |
| --- | --- | --- | --- | --- |
| 1 | **菜单渲染 / 键盘 / Esc / 文档浮层 / 鼠标点选** | gpui-component 的 `Input` 在 `render` 里已经建好 `CompletionMenu` 浮层并把它挂进 `deferred`，只要 `Lsp::completion_menu` 的状态被填上就出现 | ✅ **白送**（`CMP\src\input\overlay.rs:189,221-228,374`；`CMP\src\input\popovers\completion_menu.rs:170-430`） | 无需落点 |
| 2 | **触发时机** | 上游**没有** `Ctrl+Space`、没有任何补全 action；触发完全由"每敲一个字符 → `EditorMode::on_text_typed` → `handle_completion_trigger`"驱动，`is_completion_trigger` 是我们唯一的闸门 | ✅ 链路白送，**判定要写** | 实现 `CompletionProvider::is_completion_trigger`（`BASE\src\input\editor\lsp\completions.rs:102`） |
| 3 | **JDTLS 补全数据** | Core **已经支持** `lsp.request{operation:"completion"}`（`CORE\src\lsp\interface\engine.rs:275,3913,3940`；`DOC:1336-1342`），我们只需在 `lithe-gpui-java` 上加一个 `complete(..)` 门面 | ✅ Core 白送；门面**已在工作区落地但未提交**（见 §0.1） | `gpui/crates/java/src/service.rs` |
| 4 | **把数据送进菜单** | 实现一个 `CompletionProvider`，`completions()` 里 `background_spawn` 调 Core，把结果映射成 `lsp_types::CompletionItem`，**并自己解掉 snippet 占位符**（`overlay.rs:174-194` 是逐字插入） | ❌ **这是本次唯一必须新写的实质代码** | 新模块 `gpui/crates/editor/src/completion.rs` + 接线 `editor_view.rs` |

**一句话**：渲染层全在，Core 也全在；缺的是"`JavaLanguageService::complete`"与"`JdtCompletionProvider`"这两个胶水层，以及一个 75ms 量级的防抖 + 千分之一的 `staleDocumentVersion` 静默处理。

**不属于本次最小集（但同一条链上顺手就能做）**：诊断（`publishDiagnostics` → 波浪线）、hover、语义高亮、code action。§6 把它们排成第 3/4 步。

---

## 2. 上游能力清单（逐条：能力 / trait / 签名 / 定义处 / 谁调用 / 我们要不要实现）

### 2.1 `Lsp` 结构体：能力的注册入口

`BASE\src\input\editor\lsp\mod.rs:39-69`（字段定义）、`:71-89`（`Default`，**全部为 `None`/空**）。

| 字段 | 类型 | 行 |
| --- | --- | --- |
| `completion_provider` | `Option<Rc<dyn CompletionProvider>>` | `:41` |
| `code_action_providers` | `Vec<Rc<dyn CodeActionProvider>>` | `:43` |
| `hover_provider` | `Option<Rc<dyn HoverProvider>>` | `:45` |
| `definition_provider` | `Option<Rc<dyn DefinitionProvider>>` | `:47` |
| `document_color_provider` | `Option<Rc<dyn DocumentColorProvider>>` | `:49` |
| `semantic_tokens_provider` | `Option<Rc<dyn DocumentRangeSemanticTokensProvider>>` | `:51` |
| `show_document` | `Option<ShowDocumentHandler>` | `:56` |
| `completion_menu` | `CompletionMenuOptions`（宿主可调外观） | `:59` |

访问入口（**`EditorState` 上，公开**）：`BASE\src\input\editor\mod.rs:180-187` 的 `lsp()` / `lsp_mut()`。

> ⚠️ **没有** `diagnostics_provider` 字段、**没有** `inlay_hint_provider` 字段、**没有** `signature_help_provider` 字段。
> 诊断走的是另一条路（见 §2.5），inlay hint / signature help 上游**完全没有**。

### 2.2 补全：`CompletionProvider`（唯一必须实现的 trait）

`BASE\src\input\editor\lsp\completions.rs:40-103`。逐方法：

```rust
// BASE\src\input\editor\lsp\completions.rs:48-55
fn completions(
    &self,
    text: &Rope,
    offset: usize,
    trigger: CompletionContext,
    window: &mut Window,
    cx: &mut App,
) -> Task<Result<CompletionResponse>>;          // anyhow::Result
```

```rust
// BASE\src\input\editor\lsp\completions.rs:102（必需）
fn is_completion_trigger(&self, offset: usize, new_text: &str, cx: &mut App) -> bool;
```

```rust
// BASE\src\input\editor\lsp\completions.rs:71-80（有默认实现：返回空数组）
fn inline_completion(&self, _rope: &Rope, _offset: usize, _trigger: InlineCompletionContext,
                     _window: &mut Window, _cx: &mut App) -> Task<Result<InlineCompletionResponse>>;
// :86-88 有默认实现，默认 300ms（常量在 :15）
fn inline_completion_debounce(&self) -> Duration;
// :90-97 有默认实现：Task::ready(Ok(false))
fn resolve_completions(&self, _completion_indices: Vec<usize>,
                       _completions: Rc<RefCell<Box<[Completion]>>>, _: &mut App) -> Task<Result<bool>>;
```

**谁调用**：

| 调用者 | 处 | 说明 |
| --- | --- | --- |
| `handle_completion_trigger` | `completions.rs:122-224`（`pub(crate)`） | 真正的编排：判定触发 → 取 query → `provider.completions(..)` → 填 `context_menu_content.completion` |
| `EditorMode::on_text_typed` | `BASE\src\input\editor\mod.rs:93-101` | 直接转调 `handle_completion_trigger` |
| `on_text_typed` 的调用点 | `BASE\src\input\base\state.rs:3822`（多行路径）、`:3933`（单行路径） | 都在 `replace_text_in_range` 内，条件是 `if !self.silent_replace_text` |
| `is_completion_trigger` 的调用点 | `completions.rs:144` | `handle_completion_trigger` 的第一道闸 |

### 2.3 补全菜单的默认选项与外观

```rust
// BASE\src\input\editor\lsp\completions.rs:22-37
pub struct CompletionMenuOptions { pub max_width: Pixels }
impl Default for CompletionMenuOptions { fn default() -> Self { Self { max_width: px(320.) } } }
```

- 宿主可改：`state.lsp_mut().completion_menu.max_width = px(420.)`（渲染时读取：`CMP\src\input\popovers\completion_menu.rs:383`）。
- 菜单最大高 `MAX_MENU_HEIGHT = px(240.)`（`completion_menu.rs:11`），与光标间距 `POPOVER_GAP = px(4.)`（`:12`）。
- 定位：`CompletionMenu::origin`（`completion_menu.rs:341-355`）用 `editor.cursor_layout()` 的 `cursor_bounds.origin` + `scroll_offset()` − `input_bounds().origin`，再左移 4px、下移 `line_height + 4px`。
- 选中态 `bg(cx.theme().tokens.accent)` + `text_color(cx.theme().accent_foreground)`（`:112-115`），`deprecated` 加删除线（`:110`、`:121`）。
- `detail` 用 `Label`（`:117-124`），`documentation` 走右侧（或窄窗下上下排列的）浮层 `render_markdown`（`:406-423`，`vertical_layout` 判据在 `:386-388`）。

### 2.4 行内补全（幽灵文本）：上游有，但 JDT 不提供

`CompletionProvider::inline_completion` 默认返回空数组（`completions.rs:79`），调度器 `schedule_inline_completion`（`:272-341`）在**每次** `handle_completion_trigger` 里都被无条件调用（`:139`），防抖 `inline_completion_debounce()`（默认 300ms，`:15,86-88`）。

- 需要 `InputExtras::inline_completion_item()`（`BASE\src\input\editor\mod.rs:244-246`）才渲染。
- **建议保持默认空实现**：JDT LS 不支持 `textDocument/inlineCompletion`，实现它没有数据源。

### 2.5 诊断：**没有宿主 provider trait**，只有"宿主自己往 `DiagnosticSet` 里塞"

| 名字 | 定义处 | 说明 |
| --- | --- | --- |
| `DiagnosticSet` | `BASE\src\input\editor\diagnostics.rs:186-262` | `SumTree<DiagnosticEntry>`，`new/reset/clear/push/extend/len/range/for_offset`（`:191-262`） |
| `DiagnosticEntry` | `diagnostics.rs:122-135` | `{ range: Range<usize>（字节）, diagnostic: Diagnostic }` |
| `Diagnostic` / `DiagnosticSeverity` | `diagnostics.rs:17-95` | `From<lsp_types::Diagnostic>` 已实现（`:54-74`），`from(lsp_types::DiagnosticSeverity)`（`:85-95`） |
| 挂载点 | `BASE\src\input\base\mode.rs:49`（字段）、`:86`、`:314-322`（`diagnostics()` / `diagnostics_mut()`） |
| 公开入口 | `BASE\src\input\base\state.rs:840-847` 的 `EditorState::diagnostics() -> Option<&DiagnosticSet>` / `diagnostics_mut()` |
| 渲染 | `BASE\src\input\base\element.rs:1509-1514`（取集合）、`:1602-1607`（`diagnostic_highlight_style`）、`:1637`（与 tree-sitter/语义/应用样式合并）——画在**正文高亮样式**里，不是独立元素 |
| 「弹出气泡」 | `BASE\src\input\editor\lsp\overlay.rs:111-118` 的 `present_diagnostic(..)`；`clear_diagnostic_popover`（`:120-124`）；组件侧渲染在 `CMP\src\input\popovers\diagnostic_popover.rs` |

⚠️ **三条必须知道的行为**：

1. **每次文本编辑都会清空诊断集合**：`state.rs:3551-3553`（多行）、`:3891`、`:3999` 都调 `diagnostics.reset(&self.text)`，而 `DiagnosticSet::reset` 的语义是"换 rope + `clear()`"（`diagnostics.rs:199-202`）。所以**不能**缓存诊断再重放，只能等 JDT 重新 `publishDiagnostics`。
2. **`DiagnosticSet` 自带一份 `Rope`**（`diagnostics.rs:187`），偏移换算用的是它（`:206-207`）→ 每次更新都要 `reset(&当前正文)` 后再 `extend(..)`。
3. 诊断**不归 `Lsp` 结构体管**，`Lsp::update`（`lsp/mod.rs:93-101`）只更新 document colors 与 semantic tokens。

### 2.6 语义高亮：有 provider trait，且 `Lsp::update` 自动刷新

```rust
// BASE\src\input\editor\lsp\semantic_tokens.rs:36-55
pub trait DocumentRangeSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend;
    fn semantic_tokens(&self, text: &Rope, range: Range<usize>,
                       window: &mut Window, cx: &mut App) -> Task<Result<SemanticTokens>>;
}
```

**谁调用**：`Lsp::update_semantic_tokens`（`semantic_tokens.rs:108`）← `Lsp::update`（`lsp/mod.rs:93-101`）← `EditorMode::refresh_language_features`（`BASE\src\input\editor\mod.rs:55-62`）← `state.rs:3554`（批量编辑后）、`:3913`（单行输入后）、`:4021`、`:4170`（render 的 `_pending_update` 分支）。渲染走 `InputExtras::semantic_token_styles`（`BASE\src\input\editor\mod.rs:221-228`）。

- 颜色按**类型名字符串**在 paint 时解析（`semantic_tokens.rs:29-33` 的文档注释），主题切换不需要重取。
- ⚠️ 上游这条链只发**区间**请求语义（trait 名就是 `DocumentRangeSemanticTokensProvider`），而 Core 只支持 **full**（`CORE\src\lsp\interface\engine.rs:307` + `DOC:1403-1410`：`textDocument/semanticTokens/full`，且"Range-only providers are not advertised as supporting this operation"）→ **要对上得自己把 full 结果按行切片**。这一步不属于本次最小集。

### 2.7 code action：有 provider trait，且 `Ctrl+.` 已绑好

```rust
// BASE\src\input\editor\lsp\code_actions.rs:8-34
pub trait CodeActionProvider {
    fn id(&self) -> SharedString;
    fn code_actions(&self, state: Entity<EditorState>, range: Range<usize>,
                    window: &mut Window, cx: &mut App) -> Task<Result<Vec<CodeAction>>>;
    fn perform_code_action(&self, state: Entity<EditorState>, action: CodeAction,
                           push_to_history: bool, window: &mut Window, cx: &mut App) -> Task<Result<()>>;
}
```

**谁调用**：`handle_code_action_trigger`（`code_actions.rs:53-109`）← `on_action_toggle_code_actions`（`:43-50`）← 注册在 `EditorMode::register_actions`（`BASE\src\input\editor\mod.rs:164-172`，动作 `ToggleCodeActions`）；键位 `cmd-.` / `ctrl-.`（`state.rs:292-294`）。菜单渲染在 `CMP\src\input\popovers\code_action_menu.rs`。

### 2.8 go-to-definition（**组件那条，我们不要用**）

```rust
// BASE\src\input\editor\lsp\definitions.rs:13-24
pub trait DefinitionProvider {
    fn definitions(&self, _text: &Rope, _offset: usize,
                   _window: &mut Window, _cx: &mut App) -> Task<Result<Vec<lsp_types::LocationLink>>>;
}
```

**谁调用**：`handle_hover_definition`（`definitions.rs:64-107`）← `handle_mouse_move`（`hover.rs:95-96`，条件是 `modifiers.secondary()`）；`handle_click_hover_definition`（`definitions.rs:129-154`）← `EditorMode::on_click`（`editor/mod.rs:76-84`）；`GoToDefinition` action（`definitions.rs:109-126`，`state.rs:117` 定义动作，**未绑任何按键**）。

> **明确不要装它。** 理由已在 `gpui/PLAN.md` §10.3 "冲突点" 记过：一旦装 `definition_provider`，`handle_click_hover_definition` 会提前 `return`（`state.rs:2259-2261` 附近），点击不再移动光标，而我们的 `Ctrl+单击` 正是靠"组件先移光标、我们冒泡时读 `cursor()`"（`REPO\crates\editor\src\editor_view.rs:775`）。两条路同时开会让 `Ctrl+单击` 失效。

### 2.9 hover：有 provider trait，鼠标移动触发（150ms 延时）

```rust
// BASE\src\input\editor\lsp\hover.rs:11-22
pub trait HoverProvider {
    fn hover(&self, _text: &Rope, _offset: usize,
             _window: &mut Window, _cx: &mut App) -> Task<Result<Option<lsp_types::Hover>>>;
}
```

**谁调用**：`handle_hover_popover`（`hover.rs:26-82`）← `handle_mouse_move`（`:84-102`）← `EditorMode::on_mouse_move`（`editor/mod.rs:139-147`）← `state.rs:2359`（`on_mouse_move` 监听器，注册在 `:4247`）。
延时：**首次** 150ms（`hover.rs:50-56`，`should_delay = hover_popover.is_none()`）；已经在同一个符号上则不重发（`:40-44`）。
清空：`alt` 修饰键移动（`:91-94`）、`clear_hover_state`（`:104-113`）、`state.rs:2353,3186`。
渲染：`InputExtras::hover_symbol_range`（`editor/mod.rs:238-242`）给正文加下划线，浮层在 `CMP\src\input\popovers\hover_popover.rs`。

### 2.10 inlay hint / signature help / document symbol / rename / formatting：**上游没有 provider trait**

`BASE\src\input\editor\lsp\` 下只有 7 个模块（`mod.rs:8-14`）：`code_actions` / `completions` / `definitions` / `document_colors` / `hover` / `overlay` / `semantic_tokens`。
`BASE\src\input\editor\` 下还有 `auto_close` / `decorations` / `diagnostics` / `highlighting` / `indent` / `language_config` / `language` / `mod` / `search`。

→ **inlay hint、signature help、rename、formatting、document symbol 在上游没有任何接口**（Core 侧有 `inlayHints`/`rename`/`formatting`/`foldingRanges`/`codeLens`，见 §4；但 gpui 渲染层只提供"折叠范围""诊断""语义色""补全""hover""code action""document color"这七种表现）。

### 2.11 `CMP\src\input\editor.rs` 有没有替我们装好？

**答案：渲染层全装好了，一个 provider 都没装。** 逐条：

| 它做了什么 | 处 |
| --- | --- |
| `Editor::render` → `Input::from_state(state)` + 字体/字号/行高/只读/tab_index/role/context_menu/on_paste | `CMP\src\input\editor.rs:134-160` |
| 高亮工厂：`state.ensure_highlighter_factory(crate::highlighter::input_highlighter_factory(), cx)` | `CMP\src\input\input.rs:495` |
| 高亮主题：`InputEditorStyle.highlight_styles = cx.theme().highlight_theme.clone()` | `CMP\src\input\input.rs:510` |
| **浮层总入口**：`state.render_overlays(window, cx)` | `CMP\src\input\input.rs:603` |
| `render_overlays` 里为 `EditorMode` **建 `CompletionMenu` 与 `CodeActionMenu`** | `CMP\src\input\overlay.rs:183-198`（`build_lsp`） |
| `render_overlays` 里**每帧装 action 路由** | `CMP\src\input\overlay.rs:390-395` → `M::install_action_handler` → `:137-160` |
| `sync_lsp` 里按 `revision` 把状态推给菜单 | `CMP\src\input\overlay.rs:200-274`（补全 `:209-229`、code action `:231-245`、hover `:249-258`、诊断 `:262-273`） |
| 把非空浮层放进 `floating` 列表 | `CMP\src\input\overlay.rs:371-386` |
| **provider 安装** | ❌ 一处都没有（`Lsp::default()` 全 `None`，`BASE\src\input\editor\lsp\mod.rs:71-89`） |
| 语言配置 / `LanguageProvider`（与补全无关，只管括号配对与缩进） | `CMP\src\input\language_config.rs:14-72`；注册在 `CMP\src\input\mod.rs:53-55` |

### 2.12 上游自带的**唯一**一个 `CompletionProvider` 参考实现

`CMP\src\inspector.rs:87`（`Rc::new(LspProvider {})`）→ `:104-107`（`state.lsp_mut().completion_provider = Some(lsp_provider.clone())`）→ `:632-634`（`is_completion_trigger` 恒 `true`）。
这是本机唯一一份可读的、真实的 `CompletionProvider` 实现（构造 `CompletionItem` 的完整字段写法见 `:604-621`）。**它是我们写 `JdtCompletionProvider` 的最佳模板。**

---

## 3. 触发与交互（决定"打字就有提示"能不能成立）

### 3.1 没有任何补全快捷键（这是本调研最反直觉的一条）

- `BASE\src\input\base\state.rs:71-119` 的 `actions!` 清单里**没有** `ShowCompletions` / `TriggerCompletion` 一类动作（只有 `ToggleCodeActions`、`GoToDefinition`、`Search`、`Replace` 等）。
- `state.rs:132-302` 的 `Input` 上下文键位里**没有** `ctrl-space`；唯一的 space 相关绑定是 `ctrl-cmd-space` → `ShowCharacterPalette`（`:242`）。
- `CMP` 侧全量 grep `ShowCompletions|TriggerCompletion` **零命中**。
- → **`Ctrl+Space` 目前不存在**。上游的补全只有"输入即触发"一条路。

### 3.2 唯一的触发链路（每次键盘输入/粘贴都会走）

```
键盘输入 / 粘贴
  └─ EditorState::replace_text_in_range            BASE\src\input\base\state.rs:3764 起（多行）/ :3827 起（单行）
       └─ if !self.silent_replace_text { M::on_text_typed(..) }   :3821 / :3932
            └─ EditorMode::on_text_typed                          BASE\src\input\editor\mod.rs:93-101
                 └─ handle_completion_trigger                     BASE\src\input\editor\lsp\completions.rs:122-224
                      ├─ :129-131 completion_inserting 时早退（接受补全期间不再请求）
                      ├─ :133-135 没有 completion_provider 时早退
                      ├─ :139 无条件调度行内补全（300ms 防抖）
                      ├─ :144-146 is_completion_trigger(..) 为 false 时早退   ← ★ 我们的闸门
                      ├─ :148-153 起始偏移 = 上次遗留的 trigger_start_offset，否则本次 range.end
                      ├─ :158-166 query = [起始偏移, 光标) 的文本
                      ├─ :177-180 CompletionContext{ TRIGGER_CHARACTER, trigger_character: Some(query) }
                      └─ :182-223 provider.completions(..) → 空则关菜单，非空则 items + open
```

**三条关键推论**：

1. **"打字就有提示"天然成立**，代价是**每敲一个字符一次完整 Core 往返**（`syncDocument` + `request`）。组件**不**给菜单补全做防抖（只给行内补全做，`completions.rs:15,291`），所以**防抖必须我们自己做**（`is_completion_trigger` 里做不行——它没有延时能力；要在 `completions()` 里 `background_executor().timer(..)`，或按 Windows 的做法在同步侧防抖，见 §3.5）。
2. **菜单不做前缀过滤**：`ContextMenuDelegate` 没有 filter 方法（`completion_menu.rs:24-167`），`query` 只用来给 label 的前 N 个字符上色（`completion_menu.rs:88-101`）。**所以 items 必须已经是筛选后的结果** —— 靠"每敲一个字符重发一次请求、让 JDT 自己按前缀筛"，或者我们自己按 `query`/`filterText` 过滤。**这是需要实测确认的一点（见 §7）。**
3. `trigger_character` 被填成了**整段 query 而不是单个触发字符**（`completions.rs:179`），所以我们不能靠 `CompletionContext` 判断"是不是刚敲了 `.`"；要判就判 `new_text`（`is_completion_trigger` 的第二参数就是本次输入文本）。

### 3.3 菜单的键盘、Esc、点选、关闭

| 交互 | 实现 | 处 |
| --- | --- | --- |
| `Up` / `Down` | `state.up` / `state.down` 先 `M::handle_context_menu_action`（菜单开时消费掉，不移动光标） | `BASE\src\input\base\movement.rs:312-318`、`:320-326`；动作绑定 `state.rs:176-177`；监听器 `state.rs:4205-4206` |
| `Enter` | 先 `M::handle_context_menu_action`，否则才插入换行 | `state.rs:1887-1890` |
| `Escape` | 先 `M::handle_context_menu_action`，否则才做其它逃逸处理 | `state.rs:2010-2013` |
| 路由到菜单 | `handle_action_for_context_menu` 按"补全菜单开 → `InputOverlayKind::Completion`"派给 `overlay_action_handler`；`handled && (Enter || Escape)` 时**由它自己再关一次** | `BASE\src\input\editor\lsp\completions.rs:238-269` |
| 菜单内的按键语义 | Enter=选中；Escape=隐藏；Up/Down=移动选中项（`cx.propagate()` 后处理） | `CMP\src\input\popovers\completion_menu.rs:243-290` |
| 处理器从哪来 | `OverlayMode::install_action_handler`（`EditorMode` 版本）把 `CompletionMenu::handle_action` 装进 `InputBaseState` | `CMP\src\input\overlay.rs:137-160`；每帧由 `render_overlays` 重装（`:390-395`） |
| 鼠标点到菜单外关闭 | `.on_mouse_down_out(.. hide)` | `completion_menu.rs:424-426` |
| 选中后插入 | `CompletionMenu::select_item` → `InputBaseState::insert_completion` | `completion_menu.rs:227-241` → `BASE\src\input\editor\lsp\overlay.rs:166-197` |
| 插入是**静默**的 | `replace_text_in_range_silent`（`silent_replace_text = true`）→ `on_text_typed` 被跳过 → **不会自我触发死循环** | `state.rs:3441-3451`；跳过的判据在 `:3821` / `:3932` |
| 插入后仍会发脏标记 | `emit_events` 未被静默关掉 → `cx.emit(InputEvent::Change)` 照发（`state.rs:3935-3937`）→ 现有脏标记/自动保存不受影响 | 同上 |

**`insert_completion` 的插入语义（**必须记住**）**：只认 `text_edit`（`Edit` / `InsertAndReplace`）或 `insert_text`，否则回落 `item.label`；再否则 `range = range.end..range.end`（纯追加）。**它逐字插入 `new_text`，没有任何 snippet 处理**（`overlay.rs:174-194`）。
→ JDTLS 的候选常带 `insertTextFormat: 2`（Snippet，形如 `greet(${1:name})`），**必须由我们先转成纯文本**，否则会把 `${1:name}` 原样打进源码。Core 已经有这条命令：`lsp.plainSnippet`（`DOC:944-946`，`CORE\src\lsp\lightweight\snippets.rs:9-18`，`{value}` → `{text}`）。

### 3.4 焦点与"菜单开时编辑器仍然吃键"的细节

- 菜单内容变化后只在**编辑器仍然聚焦**时写状态（`completions.rs:205-207`）；失焦则丢弃结果。
- 菜单打开时 `is_context_menu_open` 为真，`InputBaseState` 的"当前 key context 是否为 deepest"判定把它算进来（`state.rs:3163-3177`）。
- `hide_context_menu` 同时关补全与 code action 两个菜单（`completions.rs:226-231`）。

### 3.5 参考产品怎么做"自动触发"（只取命令序列与数值，不抄架构）

| 产品 | 触发方式 | 防抖 | 处 |
| --- | --- | --- | --- |
| Windows | Monaco `registerCompletionItemProvider` + `triggerCharacters: [".", ":", "<", '"', "'", "/", "@", "#"]`；`suggestOnTriggerCharacters: autoCompletion` 开关 | 文档同步 75ms | `windows/tauri/src/features/editor/engines/monaco/lsp-providers.ts:159-182`；`windows/tauri/src/features/editor/components/monaco-editor.tsx:931,2122`；`windows/tauri/src/features/editor/hooks/use-lsp-integration.ts:49,238-249` |
| macOS | 编辑器自身 `onCompletionRequested` 回调（`CodeEditorView` 在输入/命令时调 `requestLanguageCompletions()`），**请求前不做增量防抖**；结果与本地 builtin/Spring 候选**按 label 去重合并** | 无（同步靠 edit 事件层） | `macos/Sources/Lithe/Views/Editor/CodeEditorView.swift:771-773,2169,4013-4018`；`macos/Sources/Lithe/Models/AppModel/AppModel+LanguageEditing.swift:77-122`；`macos/Sources/Lithe/Application/Features/LanguageEditingCoordinator.swift:26-32` |
| 上游 gpui 行内补全 | 300ms | `BASE\src\input\editor\lsp\completions.rs:15` |
| Windows LSP 请求超时 | Core 自身死线 30s，本地兜底 30s + 5s 宽限；超时后主动 `lsp.cancelOperation` | `windows/tauri/src/platform/lsp-core-adapter.ts:33,37,1033-1086` |
| macOS 事件轮询 | 有在飞操作时 10ms，空闲时 50ms | `macos/Sources/LitheLanguageIntelligenceModule/Runtime/LanguageServerSession.swift:28-29,590-594` |

---

## 4. Core 契约（命令、字段、顺序、错误码、超时）

### 4.1 命令清单（`DOC:131-159`，实现见 `CORE\src\protocol\command.rs:362-368` 等）

| 命令 | 用途 | 关键来源 |
| --- | --- | --- |
| `lsp.startServer` | 起一个 Rust 拥有的进程/会话并开始初始化 | `DOC:136`；实现 `CORE\src\lsp\interface\engine.rs:1198-1202`（已在用） |
| `lsp.waitEvents` | 阻塞直到有事件或超时，然后**排空**同一批事件 | `DOC:151,1419-1422` |
| `lsp.pollEvents` | 排空事件（不阻塞） | `DOC:150` |
| `lsp.syncDocument` | `didOpen` / `didChange` | `DOC:143,1303-1310` |
| `lsp.closeDocument` | 关闭文档并清掉它的诊断 | `DOC:145` |
| `lsp.request` | 提交一个语义请求，返回不透明 `operationId` | `DOC:146,1336-1342` |
| `lsp.cancelOperation` | 取消一个在飞语义操作 | `DOC:149`；请求体 `{sessionId, operationId}`（`engine.rs:358-364`） |
| `lsp.plainSnippet` | snippet → 纯文本 | `DOC:132,944-946` |
| `lsp.applyTextEdits` | 按 UTF-16 范围套用编辑 | `DOC:131,938-942` |
| `lsp.clearDiagnostics` | 清掉一个会话的全部诊断 | `DOC:152` |
| `lsp.snapshot` | 诊断运行时快照（测试/控制面用） | `DOC:153` |
| `lsp.builtinCompletions` | **无进程**的当前文件标识符补全 | `DOC:133,1143-1151` |
| `lsp.stopServer` / `lsp.destroyServer` | 优雅关闭 / 摘掉已终止会话 | `DOC:142,154`（已在用） |

### 4.2 `lsp.request` 的实际字段（**这是最容易写错的一条**）

```rust
// CORE\src\lsp\interface\engine.rs:323-349 —— 注意 deny_unknown_fields
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticRequest {
    pub session_id: String,
    pub operation_id: Option<String>,      // operationId
    pub operation: LspSemanticOperation,   // completion / hover / definition / ...
    pub uri: Option<String>,
    pub virtual_uri: Option<String>,       // 只给 virtualDocument 用
    pub position: Option<LspPosition>,
    pub new_name: Option<String>,
    pub range: Option<LspRange>,
    pub diagnostics: Vec<LspClientDiagnostic>,
    pub completion_item: Option<Value>,
    pub code_action: Option<Value>,
    pub command: Option<Value>,
}
```

- `LspPosition` 的列字段名是 **`utf16Column`**（见已在用的 `REPO\crates\java\src\service.rs:157`；归一化输出侧见 `CORE\src\lsp\interface\client.rs:1247-1256`）。
- **`deny_unknown_fields`**：多写一个字段直接 `invalid_request`。
- **`operation` 支持的取值**（`engine.rs:270-321` 的 enum，serde `camelCase`；字符串映射在 `engine.rs:3938-3963`）：
  `completion`(✅ 我们要的) / `hover` / `definition` / `declaration` / `typeDefinition` / `references` / `implementation` / `javaSuperImplementation` / `rename` / `formatting` / `codeActions` / `resolveCompletion` / `resolveCodeAction` / `executeCommand` / `inlayHints` / `foldingRanges` / `semanticTokens` / `codeLens` / `virtualDocument` / `javaEntrypoints` / `javaTestItems` / `javaMainMethods`。
- 响应**同步**返回 `{operationId}`（`engine.rs:351-356` 的 `OperationResponse`）；结果通过 `requestCompleted` 事件回来（见 §4.5）。

### 4.3 补全结果的归一化形状（Core 已经替我们压平了）

`CORE\src\lsp\interface\client.rs:871-922` 的 `lsp_feature_result_for_method`：

```jsonc
// textDocument/completion →
{ "items": [ /* parse_completion_items，client.rs:924-935 */ ] }
```

每个 item 的字段（`client.rs:937-965`）：

| 字段 | 类型 | 备注 |
| --- | --- | --- |
| `label` | string | **必需**；缺了整条被丢弃（`:938`） |
| `insertText` | string | `insertText` → `textEdit.newText` → 回落 `label`（`:939-947`） |
| `insertTextFormat` | number | 默认 **1**（PlainText）（`:951`） |
| `kind` | number? | LSP 数字 kind（`:952`） |
| `detail` | string? | `:953` |
| `documentation` | string? | **已被压平成字符串**（`:954` + `completion_documentation` `:967-976`） |
| `sortText` / `filterText` | string? | `:955-956` |
| `textEdit` | object? | 归一化成 `{range:{start:{line,utf16Column},end:{...}}, newText}`（`client.rs:1233-1238`） |
| `additionalTextEdits` | array | `:958-962` |
| `data` | any | 原样透传，供 `resolveCompletion` 用（`:963`） |

> ⚠️ **两处要实测确认的坑（见 §7）**：
> 1. Core 只认 `textEdit.range`（`client.rs:1235`），而 LSP 的 `textEdit` 还可能是 **`InsertReplaceEdit`**（`{insert, replace, newText}`）。若 JDT 这么发，Core 会得到 `textEdit: null`，且 `insertText` 回落链也找不到 `newText` → 该条最终以 **label** 当插入文本。Windows 侧走同一份归一化，没有额外补偿（`lsp-core-adapter.ts:1306-1307` 只取 `normalized.items`）。
> 2. `insertTextFormat` 被 Core 默认成 1；即便 JDT 报 2，也没人转换 —— 转换是我们的事（`lsp.plainSnippet`）。

### 4.4 `lsp.syncDocument` 的确切语义（**决定版本号由谁管**）

`DOC:1303-1310` + `CORE\src\lsp\interface\engine.rs:1281-1360`：

- 请求体：`{ sessionId, uri, languageId, text?, contentChanges? }` —— **没有 `version` 字段**，**版本号由 Core 拥有并单调自增**（`engine.rs:1328-1335,1360`）。
- 首次同步发 `didOpen`（版本 1）；之后发 `didChange`（版本递增）。（`DOC:1304`；已在用的注释 `REPO\crates\java\src\session.rs:264-267`）
- 响应：`{ documentVersion, changed }`。
- **相同全文重复同步 → `changed: false`、版本不变、不发 LSP 通知**（`DOC:1309-1310`）。这给了我们一个免费优化：防抖后正文没变就不再有协议开销。
- 若服务端声明了增量同步且 `contentChanges` 带了 LSP range，则走 range 编辑、可以不传 `text`（`DOC:1305-1307`）。

### 4.5 **哪些请求必须先 `syncDocument`**（本调研最关键的一条）

**答案：任何带 `uri` 的 `lsp.request` 都必须先同步该文档。** 两处硬校验：

1. `engine.rs:1674-1691`（普通语义请求分支）：
   ```rust
   let document_is_open = state.client.open_documents.get(&uri)
       .is_some_and(|document| document.version > 0);
   let provider_owns_document = is_virtual_source_uri(&self.provider_id, &uri);
   if !document_is_open && !provider_owns_document {
       return Err(CoreError::new(ErrorCode::InvalidRequest,
           "The document is not open in the language server."));
   }
   ```
   → 没同步 = `invalid_request` + 这条消息。
2. `engine.rs:1500-1523`（带 `requestedDocumentVersion` 的 Java 导航/标记类请求）：版本缺失 = `invalid_request`（"The document is not synchronized with the language server."）；版本不符 = **`cancelled`**（"The document changed before the Java navigation request started."，details 带 `expectedVersion`/`currentVersion`）。

另外 `engine.rs:1526-1539`：服务端没声明该能力 → **`not_supported`**，details 是能力名（补全的能力名是 `completion`，`engine.rs:3940`）。JDTLS 一定会声明 `completionProvider`（`CORE\src\lsp\interface\client.rs:34-48` 的 client capabilities 里已开 `completion`），所以正常路径遇不到；但**"会话还没 ready"会先撞 `invalid_request`**（`engine.rs:1485-1490`："Language server is not ready."）。

### 4.6 陈旧结果的**自动取消**：Core 白送，我们不用自己写版本守卫

`engine.rs:1334-1335`（每次 `syncDocument` 之后）：

```rust
stale_cancellations = cancel_stale_document_requests_locked(self, &mut state, &uri, document_version);
```

`cancel_stale_document_requests_locked`（`engine.rs:4289-4346`）把**同一 URI、版本更旧、且方法在 `is_stale_sensitive_method` 名单里**的在飞请求全部取消，发 `$/cancelRequest`，并给该 `operationId` 推一条 `error.code = "staleDocumentVersion"` 的 `requestCompleted`。

**`is_stale_sensitive_method`（`engine.rs:4348-4360`）的名单里第一位就是 `textDocument/completion`。** 🎯

→ 结论：**"上一次补全请求还没回来，用户又敲了一个字符"这件事由 Core 自动解决**：新一次 `syncDocument` 会让旧请求以 `staleDocumentVersion` 终结。我们只需要把 `staleDocumentVersion` / `requestCancelled` 当成"静默丢弃"，不要弹错误提示。
（macOS 就是这么做的：`LanguageServerSession.swift:627-631` 把 `staleDocumentVersion` / `requestCancelled` 翻成 `CancellationError`，上层 `LanguageEditingCoordinator.swift:143` 对 `CancellationError` 不提示。）

### 4.7 诊断是**事件推送**，不是轮询请求

- LSP 侧：`textDocument/publishDiagnostics` → `CORE\src\lsp\interface\client.rs:343-377`。
  **两条丢弃规则**：① 文档不在 `open_documents` 里 → 丢（`:350-352`）；② 事件带 `version` 且与当前文档版本不符 → 丢（`:353-356`）。
- 事件形状：`LspRuntimeEvent`（`engine.rs:389-430`）：
  ```jsonc
  { "type": "diagnostics", "sequence": 7, "providerId": "java", "sessionId": "...",
    "uri": "file:///...", "version": 12, "diagnostics": [ /* LspClientDiagnostic */ ] }
  ```
- 事件类型清单（`DOC:1423-1425`）：`stateChanged` / `featuresChanged` / `diagnostics` / `requestCompleted` / `serverInfoChanged` / `log`（实现里还有 `semanticTokensRefresh`、Maven 相关字段，见 `LanguageServerSession.swift:649-651` 与 `engine.rs:415-429`）。
- 取事件：`lsp.pollEvents` 或 `lsp.waitEvents`，**排空语义**：`waitEvents` "waits until queued events exist or the timeout elapses, **then drains the same typed events**"（`DOC:1419-1422`）。
- 语义高亮刷新：`semanticTokensRefresh` 事件（`DOC:1411-1413`）。

> ⚠️ **最大的架构冲突**（也是 §6 第 3 步的核心难点）：
> 现有 `REPO\crates\java\src\session.rs:302-359` 的 `Session::request` 在**等自己的 `requestCompleted` 时直接调 `waitEvents` 并丢弃其它事件**（`:329-357`，只认 `log` 与自己的 `requestCompleted`）。
> `waitEvents` 是**排空**语义 → **两个并发消费者会互相偷事件**。所以在我们把会话改成"单一事件泵 + 按 `operationId` 分派"之前，**不能**同时开一个诊断订阅。这是"补全"能立刻做、而"诊断"必须晚一步做的原因。

### 4.8 超时、错误码与其它口径

| 项 | 值 / 字段 | 处 |
| --- | --- | --- |
| 握手超时 | `initializeTimeoutMilliseconds`（现用 90s） | `DOC:1197-1198`；`REPO\crates\java\src\session.rs:56` |
| 就绪空闲/绝对上限 | `serviceReadyIdleTimeoutMilliseconds`=45s、`serviceReadyAbsoluteTimeoutMilliseconds`=600s | `DOC:1205-1207`；`session.rs:57-58` |
| 普通语义请求超时 | `requestTimeoutMilliseconds`（现用 60s） | `DOC:1168-1169`；`session.rs:59,152` |
| 构建超时 | `javaBuildTimeoutMilliseconds`（默认 600000） | `DOC:1370-1371` |
| 关闭超时 | `shutdownTimeoutMilliseconds`（现用 2000ms） | `session.rs:157` |
| 信封级错误码（`snake_case`） | `invalid_request` / `not_supported` / `cancelled` / `timed_out` / `process_failed` / `parse_failed` / … | `CORE\src\protocol\error.rs:5-34` |
| **运行时**错误码（`requestCompleted.error.code`，`camelCase`，**另一套命名空间**） | `staleDocumentVersion` / `requestCancelled` / `requestTimeout` / `transportFailed` / `invalidServerResult` | `engine.rs:4319,1762,1805,1886,3427,3452,2601,1188` |
| 每个请求最多完成一次 | 晚到的响应在取消/超时之后被忽略 | `DOC:1425-1429` |
| 取消 | `lsp.cancelOperation {sessionId, operationId}` | `engine.rs:358-364` |
| `operationId` 唯一性 | 同一会话内重复挂起 → `invalid_request`（"The language-server operation ID is already pending."） | `engine.rs:1491-1498` |

---

## 5. 我们这一侧现在有什么、缺什么

### 5.1 可直接复用的（**不要重写**）

| 能力 | 函数 / 类型 | 处 | 复用方式 |
| --- | --- | --- | --- |
| 起 JDTLS + 等就绪 + 索引缓存 | `JavaLanguageService::prepare/ensure_session/start_locked` | `REPO\crates\java\src\service.rs:108-115,193-281` | 原样复用；已完成（`S1_JAVA_READY`） |
| 会话生命周期 | `Session::start/wait_ready/shutdown` | `REPO\crates\java\src\session.rs:113-191,197-262,369-424` | 原样复用 |
| **一次语义请求（含 operationId / 超时 / 错误翻层）** | `Session::request(&self, operation: &str, payload: Value, deadline: Instant) -> Result<Value, String>` | `session.rs:302-359` | **直接复用**，补全只是换 `operation = "completion"` |
| 文档同步（版本由 Core 管） | `Session::sync_document(&self, uri, language_id, text) -> Result<(i64, bool), String>` | `session.rs:268-295` | **直接复用** |
| 事件排空 | `Session::wait_events(&self, timeout) -> Result<Vec<Value>, String>` | `session.rs:427-443` | 复用；但**要做诊断订阅时得改造**（见 §4.7 的警告） |
| 文件路径 → `file://` URI | `service.rs::file_uri`（`&Path -> Result<String,String>`） | `service.rs:442-446` | **要提升为 `pub(crate)` 或在 crate 内复用**（现在是私有自由函数） |
| 目录 → `rootUri` | `service.rs::directory_uri` | `service.rs:449-453` | 已够 |
| 光标列口径换算（字节偏移 ↔ Core 的 `utf16Column`） | `navigation::core_position(&Rope, usize) -> (u64, usize)` / `editor_position(&Rope, usize, usize) -> Position` | `REPO\crates\editor\src\navigation.rs:232-253` | **直接复用**（补全请求要发 `utf16Column`，就是 `core_position`） |
| JDTLS 载荷 / JDK 发现 / 路径归一 | `jdtls::resolve`（`:98`）/ `resolve_runtime`（`:440`，`LITHE_JDTLS_JAVA` 覆盖在 `:40,442`）/ `normalize_path`（`:627`） | `REPO\crates\java\src\jdtls.rs` | 已够 |

### 5.2 缺的能力（逐条）

| 缺什么 | 现状 | 没有它会怎样 |
| --- | --- | --- |
| **`JavaLanguageService::completion(..)`** | **已在工作区实现（未提交）**，见 §0.1；`definition(..)` 在 `service.rs:122-176` | —— （原先是第一个缺口） |
| **snippet → 纯文本** | Core 有 `lsp.plainSnippet`，工作区实现已接入（`service.rs` 的 `plain_snippet`） | 未转换时 `greet(${1:name})` 会被原样插进源码 |
| **超时后的轻量回落** | 工作区实现的文档注释声明要回落 `lsp.builtinCompletions`，**回落还没写** | JDTLS 冷启动/超时（15s）时用户什么都看不到，而 `lsp.builtinCompletions` 是**无进程**的（`DOC:133`） |
| **`CompletionProvider` 实现** | 不存在 | 菜单永不出现（`completions.rs:133-135` 直接 `return`） |
| **`lsp-types` 直接依赖** | `REPO\crates\editor\Cargo.toml` 没有；`gpui/Cargo.lock:4008-4011` 已有 `lsp-types 0.97.0`（由 `gpui-component` 传递引入，`CMP\Cargo.toml:304-306` 与 `BASE\Cargo.toml:94-96` 都是 `0.97.0` + `proposed`） | 无法实现 `CompletionProvider`（trait 签名里就是 `lsp_types::CompletionResponse`） |
| **防抖 / 取消策略** | 无 | 每敲一个字符一次完整往返；快速输入会堆积或卡顿 |
| **单一事件泵** | `Session::request` 自己消费 `waitEvents` | 一旦再加诊断订阅就互相偷事件（§4.7） |
| **`lsp.closeDocument`** | 从未调用 | 关标签后 Core 里文档仍开着、版本继续涨；诊断不清（`DOC:145`）；长期运行会泄漏 `open_documents` 条目 |
| **`publishDiagnostics` 订阅** | 不存在 | 没有波浪线；`DiagnosticSet` 一直是空 |
| **hover / code action / 语义高亮 provider** | 不存在 | 对应浮层永不出现 |

### 5.3 依赖方向（遵守现有图，不能反向）

现状（`REPO\crates\*/Cargo.toml`）：

```
app ──► workbench ──► editor ──► java ──► shared
                └──► settings ──► shared
                └──► git / terminal / explorer ──► shared
```

- `lithe-gpui-editor` 已经依赖 `gpui-kit` + `lithe-gpui-shared` + `lithe-gpui-java` + `serde_json`（`REPO\crates\editor\Cargo.toml:16-24`）。
- `lithe-gpui-java` **只**依赖 `lithe-gpui-shared` + `serde_json` + `url`，**完全不依赖 gpui**（`REPO\crates\java\Cargo.toml`），这是有意的（`REPO\crates\java\src\lib.rs:8-24`："这个 crate 没有一行 LSP 协议、也没有一行 Java 语法"）。
- **因此**：`CompletionProvider` 实现只能放 `editor`（它同时看得见 gpui-kit 的 trait 与 java 的门面）；`java` 只返回自己的纯 DTO，**不要**把 `lsp_types` / gpui 引进 `java`，否则破坏上面那条边界。

---

## 6. 分步实现计划（每步 = 一个可独立验证的小交付）

> 纪律照 `gpui/HANDOFF.md` §1：`cargo build --bin Lithe` 重定向到日志再 grep、同一时刻只有一个 `cargo` 写者、诊断走 `S1_*`、界面文案走 `lithe_gpui_shared::tr`。
> ⚠️ **一个文档内的不一致要先拍板**：`HANDOFF.md:63-64` 要求"诊断走 stderr（`eprintln!`）"，但现有 `S1_JAVA_*` 全在 `REPO\crates\java\src\{service,session}.rs` 里用 `println!`（例：`session.rs:180,228,376`；`service.rs:110,148,168,209,220,231`），而 `S1_NAV_*` 也用 `println!`（`editor_view.rs:590,657`）。只有外壳层的 `S1_TAB_MENU` 用 `eprintln!`（`workbench\src\workspace.rs:706`）。
> **建议**：新增的 `S1_JAVA_COMPLETION*` 紧跟邻居用 `println!`（否则同一族诊断分居两个流更难读），验证脚本**同时**重定向 `.out` 与 `.err` 再 grep。这条要么在代码注释里写明，要么先问维护者。

### 第 1 步：Core 侧打通"补全"这一条路 —— **已在工作区实现，只差验证**（见 §0.1）

工作区里已有 `REPO\crates\java\src\service.rs` 的 `JavaLanguageService::completion(..)`（15s 死线、snippet 走 `lsp.plainSnippet`、`textEdit` 带范围）。
**这一步剩下来的是验证**，不需要再写实现：

**验证（命令行，不需要 UI）**：
```powershell
cd D:\developmentProjects\rust\Lithe-IDEA\gpui
$env:LITHE_JDTLS_JAVA = 'D:\ProgramData\java\openjdk-21\bin\java.exe'
cargo test -p lithe-gpui-java *> ..\.artifacts\pN\step1.log
Write-Output "exit=$LASTEXITCODE"
Select-String -Path ..\.artifacts\pN\step1.log -Pattern "test result|FAILED|^error"
```
先用 `cargo test -p lithe-gpui-java completion_item_reads_the_normalized_fields` 跑那条纯解析单测（无 IO、秒级）。
再补一条**选定式**真实 JDTLS 测试（照 `service.rs` 里 `real_jdtls_resolves_a_cross_file_definition` 的 `SMOKE_ENV` + `TempWorkspace` + `Shutdown` 三重写法，需要 `LITHE_GPUI_JDTLS_SMOKE=1`）：在 `App.java` 里 `greeter.` 之后求补全，断言 ① 返回非空；② 至少一条 label 含 `greet`；③ 该条的 `insert_text` **不含 `$`**（snippet 已还原）；④ `text_edit` 非 `None`（否则会插重复）。

**期望的 `S1_*` 行**：`S1_JAVA_READY elapsedMs=…` → `S1_JAVA_SYNC uri=file:///…/App.java version=1 changed=true bytes=…` → `S1_JAVA_COMPLETION uri=file:///…/App.java line=… col=… items=NN ms=…`。
（注意现有实现**没有**在 `S1_JAVA_COMPLETION` 里输出 `snippets=` 计数，§7.1 第 3 条因此要靠单测断言而不是日志。）

### 第 2 步：菜单真的弹出来（**本次唯一真正要写的实质代码，也是验收点**）

**交付 A**：`REPO\crates\editor\Cargo.toml` 加一行
```toml
# `CompletionProvider` 的签名里就是 `lsp_types::CompletionResponse`（gpui-base-0.6.6/src/input/editor/lsp/completions.rs:48-55），
# 而 gpui-kit 只转出了 `Position`（gpui-base-0.6.6/src/input/mod.rs:101）。版本与依赖树里那份一致（gpui/Cargo.lock:4008）。
lsp-types = "0.97.0"
```
（`gpui/Cargo.lock` 只会给 `lithe-gpui-editor` 的依赖清单加一项，**不会**新增 `[[package]]` 或下载——`lsp-types 0.97.0` 已在锁里。）

**交付 B**：新模块 `REPO\crates\editor\src\completion.rs`

```rust
//! JDTLS 补全的**表现侧**：把 `JavaLanguageService::completion` 接到
//! `gpui_kit::component::input::CompletionProvider` 上。
//! 本模块不做协议，只做四件事：列口径换算、DTO→`lsp_types` 映射、触发判定、轻量回落。

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::anyhow::Result;                    // = gpui::Result = anyhow::Result（gpui-pre-0.3.6/src/gpui.rs:76,93）
use gpui_kit::component::input::{CompletionProvider, Rope};
use gpui_kit::{App, SharedString, Task, Window};
use lithe_gpui_java::{JavaCompletionItem, JavaLanguageService};
use lsp_types::{CompletionContext, CompletionItem, CompletionResponse};

pub(crate) struct JdtCompletionProvider {
    /// 服务句柄。`Arc` 是必要的：每次请求都要把它 move 进后台任务（与 `EditorPane::java` 同口径）。
    service: Arc<JavaLanguageService>,
    /// 该 buffer 的绝对路径（`file_uri` 的输入）。一个 buffer 一个 provider，与"一个标签一个 EditorState"一致。
    path: PathBuf,
    /// 工作区根的字符串形式：轻量回落 `lsp.builtinCompletions` 要 `filePath`。
    file_path: SharedString,
}

impl CompletionProvider for JdtCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        // `Task` 必须 'static：不能把 `&Rope` 借进去，克隆正文（`Rope` 是持久化结构，克隆很便宜）。
        let (line, utf16_column) = crate::navigation::core_position(text, offset);
        let body = text.to_string();
        let service = self.service.clone();
        let path = self.path.clone();
        let file_path = self.file_path.clone();

        cx.background_spawn(async move {
            // ⚠️ 这里**没有**防抖：见下方"防抖怎么放"。Core 的 stale 自动取消（§4.6）已经
            // 保证"后来的请求赢"，慢的是往返本身。
            match service.completion(&path, &body, line as u32, utf16_column as u32) {
                Ok(items) if !items.is_empty() => {
                    Ok(CompletionResponse::Array(items.into_iter().map(to_completion_item).collect()))
                }
                // 服务不可用 / 超时 / 空结果 → 回落轻量补全，用户至少看到当前文件的标识符。
                // 与 macOS 的 provider 链同口径（LanguageFeatureProvider.swift:116-132 的 builtin 优先）。
                Ok(_) | Err(_) => Ok(builtin_fallback(&file_path, &body, line, utf16_column)),
            }
        })
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        // 只对"单个标识符字符或点"触发；倒退（不发 on_text_typed）、粘贴大段、换行都不触发。
        let mut chars = new_text.chars();
        matches!((chars.next(), chars.next()),
                 (Some(c), None) if c.is_alphanumeric() || c == '_' || c == '.')
    }
}

/// 轻量回落：`lsp.builtinCompletions`（无进程，`DOC:133-135,1143-1151`；
/// 请求 `{filePath, text, position{line,utf16Column}}`，响应 `{items:[{label, insertText, kind, detail, textEdit:{range,newText}}]}`，
/// 见 `rust/lithe-core/src/lsp/lightweight/symbols.rs:14-18,33-47`）。
fn builtin_fallback(file_path: &str, text: &str, line: u64, utf16_column: usize) -> CompletionResponse { /* … */ }

/// `JavaCompletionItem` → `lsp_types::CompletionItem`。
/// `insert_text` 已是纯文本（snippet 在 java 侧就还原了）→ **不要**设 `insert_text_format`。
/// `text_edit` 必须带上：上游按它的 range 替换已敲进去的前缀，缺了就退化成"在光标处再插一段"
/// （`BASE\src\input\editor\lsp\overlay.rs:166-197`），于是 `Sys` + `System` 会变成 `SysSystem`。
/// `JavaTextEdit` 的位置是 **0 基 / UTF-16 列**，与 `lsp_types::Position` 同口径，**直接搬，不要再换算**。
fn to_completion_item(item: JavaCompletionItem) -> CompletionItem { /* … */ }
```

四个实现细节（**都必须照做**）：

1. **`insertTextFormat` 不要设**：`JavaCompletionItem::insert_text` 已过 `lsp.plainSnippet`，把 `CompletionItem.insert_text_format` 留空/`PlainText` 即可，否则上游也不处理它（`overlay.rs:174-194` 只认 `text_edit` / `insert_text`）。
2. **`textEdit` 的列口径不要二次换算**：Core 归一化后就是 `{line, utf16Column}`（`client.rs:1247-1256`），`lsp_types::Position{line, character}` 恰好同口径；再套一次 `navigation::editor_position`（那是"UTF-16 → 字符列"的换算，`navigation.rs:249-253`）会把范围改错。
3. **`staleDocumentVersion` / `requestCancelled` 要静默**：`Session::request` 把它们翻成 `Err("staleDocumentVersion@request：…")`（`session.rs:338-352`）。在 `completions()` 里**不要再往上抛**——`handle_completion_trigger` 只在 `.ok()` 后处理（`completions.rs:186-191`），空数组会自动关菜单（`:193-201`），正是"新输入取代旧请求"该有的表现。当前 `service.rs` 的 `completion` 是直接 `?` 上抛的，所以**要么在 `service.rs` 里识别这两个前缀返回 `Ok(vec![])`，要么在 provider 里按错误文本识别**；推荐前者（只有 java 侧认识运行时错误码）。
4. **防抖怎么放**：`CompletionProvider::completions` 里只有 `&mut App`。可行的三条，**选一条并写进注释**：
   - (a) 用 `gpui` 的 `BackgroundExecutor`（`BASE\src\input\editor\lsp\completions.rs:287-291` 用的是 `cx.background_executor().clone()`，那里 `cx` 是 `&mut Context<..>`；`&mut App` 上取它的确切路径**我未核对**，见 §7.1 第 6 条）；
   - (b) 不做延时，靠"`syncDocument` 返回 `changed == false` 就直接返回空"（`DOC:1309-1310` 保证重复全文不发通知）+ Core 的自动 stale 取消（§4.6）；
   - (c) 在 `EditorPane` 里自己维护"最后一次输入时间戳"，在 `is_completion_trigger` 里按时间间隔闸门（纯逻辑、可单测，但会让快速输入时"少一次请求"而不是"晚一次"）。
   **建议 (b)**：不引入新 API，且行为最容易解释。

**交付 C**：接线（`REPO\crates\editor\src\editor_view.rs`）

- 新增私有方法（`EditorPane` 已有 `java: Option<Arc<JavaLanguageService>>` 字段，`editor_view.rs:281`）：
  ```rust
  /// 给一个 java buffer 装上 JDTLS 补全 provider。幂等（重复装只是换一份 `Rc`）。
  fn install_completion(&mut self, index: usize, cx: &mut Context<Self>) {
      let Some(service) = self.java.clone() else { return };
      let Some(buffer) = self.buffers.get(index) else { return };
      if !navigation::is_java_source(&buffer.path.to_string_lossy()) { return; }
      let provider: Rc<dyn CompletionProvider> = Rc::new(JdtCompletionProvider::new(service, buffer.path.clone()));
      buffer.editor.update(cx, |state, cx| {
          state.lsp_mut().completion_provider = Some(provider);
          cx.notify();
      });
      println!("S1_JAVA_COMPLETION_INSTALLED path={}", buffer.path.display());
  }
  ```
  （`is_java_source` 现在是 `navigation.rs:171-175` 的私有函数，需提成 `pub(crate)`。）
- `open()`（`editor_view.rs:427-515`）：在 `self.buffers.push(..)` + `self.active = Some(..)` **之后**（`:510-513`）调 `self.install_completion(self.buffers.len() - 1, cx);`。
- `prepare_java()`（`editor_view.rs:375-390`）：`self.java = Some(service)` 之后立刻给**已打开**的 buffer 全装一遍。**这条不能省**：`prepare_java` 由 `workbench\src\workspace.rs:654` 在 `EditorPane::new`（`:650`）之后调，但 `open` 的时机不受它约束（`:657-661` 的 `on_open` 闭包可以在服务就绪前触发），不补这一步就会出现"先开文件、后起服务 → 菜单永不出现"。

**验证（交互级）**：
```powershell
cd D:\developmentProjects\rust\Lithe-IDEA\gpui
cargo build --bin Lithe *> ..\.artifacts\pN\build.log
Write-Output "exit=$LASTEXITCODE"
Select-String -Path ..\.artifacts\pN\build.log -Pattern "^error|error\[|Finished"
```
1. 启动并**同时**收集两个流：`Lithe.exe *> ..\.artifacts\pN\comp.out.log 2> ..\.artifacts\pN\comp.err.log`（`LITHE_JDTLS_JAVA` 指向 JDK 21；理由见 §7.1 第 10 条的两流不一致）
2. 打开一个 `.java`（本调研用的样本可以是 `App.java` + `Greeter.java`，与 `service.rs` 的选定式测试同源）
3. 在方法体里敲 `gre`
4. 截图：`gpui\capture-screenshot.ps1 -Pid <pid>` → 期望光标下方出现 ≤320×240 **逻辑** px 的浮层，首项高亮（125% DPI：截图像素 = 逻辑 × 1.25，`HANDOFF.md:73`）
5. 按 `Down` → 高亮下移；按 `Esc` → 浮层消失且**光标不动**；再敲 `gre` → `Enter` → 正文出现 `greet`，且**不是** `gregreet`（这条同时验证 `textEdit` 带上了）

**期望的 `S1_*` 行**（按顺序）：
```
S1_JAVA_COMPLETION_INSTALLED path=D:\...\App.java      ← 第 2 步新增
S1_JAVA_SYNC uri=file:///.../App.java version=1 changed=true bytes=...
S1_JAVA_COMPLETION uri=file:///.../App.java line=6 col=30 items=37 ms=118
```
敲第二个字符后应能看到 `version=2`（版本递增由 Core 管，§4.4），以及（若发生取代）一条被静默掉的 `staleDocumentVersion` —— 它**不该**变成界面上的错误提示。

### 第 3 步：把会话改成"单一事件泵"（**做诊断/多请求并发的硬前提**）

**为什么必须**：§4.7 的警告 —— `waitEvents` 是排空语义，`Session::request` 现在自己在消费事件，两个消费者会互相偷。

**交付**：`REPO\crates\java\src\session.rs` 内改造

```rust
/// 事件泵：一个会话**唯一**的 `waitEvents` 消费者。
/// `request` 不再自己读事件，只等自己的 oneshot。
struct EventPump { /* background task 句柄 + pending: Mutex<HashMap<String, Sender<Result<Value,String>>>> */ }

impl Session {
    /// 起事件泵（`wait_ready` 之后、返回 Ready 之前调一次）。
    fn start_pump(&self, cx_like: /* 见下 */);
    /// 按 operationId 等结果；泵把 `diagnostics` / `log` / `stateChanged` 分派到各自的 sink。
    fn request_async(&self, operation: &str, payload: Value, deadline: Instant) -> ...;
}
```

**注意两个既有约定**（`gpui/PLAN.md` §15.1 已经踩过）：
- gpui 的 `Task` 一 drop 就取消 → 泵的任务必须**被持有**（`Session` 里多一个字段）。
- `Session` 目前是**纯 std**（`session.rs` 只 `use std::sync::Mutex`），`JavaLanguageService` 的方法都是**阻塞**的、由调用方放进 `cx.background_spawn`（`service.rs:81` 的文档）。所以事件泵有两条路：
  - **(A) 保持纯 std**：泵是一个 `std::thread`，`Session` drop 时用 `JoinHandle` + 一个 `AtomicBool` 收尾（或干脆不 join，靠 `stopServer` 让 `waitEvents` 提前返回）。
  - **(B) 引入 gpui**：`java` crate 加 `gpui-kit` 依赖，泵用 `cx.background_spawn`。**不推荐**：会破坏 `java` 无 UI 依赖的边界（`REPO\crates\java\Cargo.toml`），也会让 `service.rs:550-644` 那些不依赖 gpui 的单测变重。
  - **推荐 (A)**，并在 `java/src/lib.rs` 的模块文档里写清"泵是 std 线程，与 gpui 无关"。

**验证**：`cargo test -p lithe-gpui-java`（现有 4 条单测 + 新增"两个并发请求各自拿到自己的结果、且不影响第三个订阅者"的桩测）；再用第 4 步做端到端。

### 第 4 步：诊断（波浪线）—— 顺手就能做

**交付**：`REPO\crates\editor\src\completion.rs` 旁边加 `diagnostics` 侧：
- 在事件泵里把 `type == "diagnostics"` 的事件按 `uri` 分派；
- 回到前台（`cx.spawn_in` / `update_in`）后：
  ```rust
  use gpui_kit::base::input::{Diagnostic, DiagnosticSet};   // 走 gpui_kit::base（= gpui_base），component 没有转出 DiagnosticSet
  editor.update(cx, |state, cx| {
      if let Some(set) = state.diagnostics_mut() {
          set.reset(state.text());          // ⚠️ 必须用**当前**正文重置 rope（BASE\src\input\editor\diagnostics.rs:199-202）
          set.extend(items.iter().cloned().map(Diagnostic::from));
      }
      cx.notify();
  });
  ```
- **不要**缓存诊断再在编辑后重放：`state.rs:3551-3553` 每次编辑都会 `reset` 清空（§2.5 第 1 条）。波浪线在打字瞬间消失、JDT 重新 publish 后回来，这是预期行为。

**验证**：在 Java 文件里写一个引用了未定义符号的行 → 等 JDT 报错 → 截图对比该行区域的**非灰像素**与基线（`research/editor-syntax-highlighting.md` §5.2 的 `non_gray_pixels` 脚本可直接复用）；期望 `S1_JAVA_DIAGNOSTICS uri=… version=… count=N`。

### 第 5 步（可选，按用户要求再排）：`Ctrl+Space` 手动触发

**上游没有**，所以要自建；但上游给了两个公开钩子：

```rust
// BASE\src\input\editor\lsp\overlay.rs:67-84（pub，可从 crate 外调）
pub fn present_completion_items(&mut self, trigger_start_offset: usize, query: impl Into<String>,
                                items: Vec<lsp_types::CompletionItem>, cx: &mut Context<Self>);
```
注意 **`handle_completion_trigger` 是 `pub(crate)`**（`completions.rs:122`），crate 外调不到 → 手动触发要自己走一遍"取 query → `service.complete` → `present_completion_items`"，与第 2 步有一段重复逻辑。落点：`REPO\crates\editor\src\lib.rs:185` 的 `install_actions` 加 `KeyBinding::new("ctrl-space", TriggerCompletion, None)`（先 grep 全依赖树确认 `ctrl-space` 没被占用——`HANDOFF.md` 里 `ctrl-s`/`f12`/`ctrl-w` 的确认口径就是这个）。

### 第 6 步（可选）：hover / code action / 语义高亮

同一条链，各自只需再实现一个 trait（§2.6–2.9 的签名），因为渲染层已经在收 `hover_provider` / `code_action_providers` / `semantic_tokens_provider`（`Lsp` 字段，`BASE\src\input\editor\lsp\mod.rs:43-51`）。语义高亮要额外把 Core 的 **full** 结果切成区间（§2.6 的警告）。

---

## 7. 风险与未知

### 7.1 必须实测才能确认的（我**没能确认**，逐条给"怎么确认"）

| # | 未确认点 | 为什么重要 | 怎么确认 |
| --- | --- | --- | --- |
| 1 | **JDTLS `textDocument/completion` 是否返回前缀已筛选的列表** | 菜单不做过滤（`completion_menu.rs:24-167` 无 filter；`query` 只用于上色 `:88-101`）。若 JDT 返回全量，用户会看到几百条无关候选 | 第 1 步的选定式测试里打印 `items` 的 label 前 20 条，比较"光标在 `gre` 之后"与"光标在 `g` 之后"两次的条数；若相同则必须自建过滤（按 `filterText`/label `starts_with(query)`，query 用 `completion_menu_state().query`） |
| 2 | **JDT 的 `textEdit` 是不是 `InsertReplaceEdit`** | Core 只读 `textEdit.range`（`client.rs:1233-1238`）→ `InsertReplaceEdit` 会变成 `textEdit: null` + `insertText` 回落 label（§4.3 的坑）；而 `textEdit` 缺失会让上游退化成"在光标处再插一段"（`overlay.rs:174-194`），表现为 `Sys` + `System` → `SysSystem` | 第 1 步的选定式测试里断言 `text_edit.is_some()`；若大量为 `None`，就在 `service.rs` 里对缺失条目**按 query 起止自造 `textEdit`**（编辑器侧能算出 query 的起止：`handle_completion_trigger` 已经把 `trigger_start_offset` 放进 `CompletionMenuState`，`completion_menu.rs:305-310`）并记诊断；**不要**去改 Core（只读区） |
| 3 | **`insertTextFormat == 2` 的真实比例** | 决定 `lsp.plainSnippet` 的额外往返次数（**每条一次** Core 进程内调用，见 `service.rs` 的 `plain_snippet`） | 现有实现**没有**输出 snippet 计数（`S1_JAVA_COMPLETION` 只有 `items=` 与 `ms=`），所以要看 `ms=` 的量级：若 `items=200` 时 `ms` 明显偏大，就该批量/惰性转换（只在候选真的被选中时转 `insertText` 是不够的——`textEdit.newText` 也得转） |
| 4 | **每敲一个字符一次完整往返的主观响应** | 一次 `syncDocument` + 一次 `request` + 若干次 `waitEvents` 都在持 `state` 锁的情况下**同步**跑（`service.rs:136-139` 一带）。工作区实现把**应用侧**死线设成 15s（`COMPLETION_TIMEOUT`），而 Core 侧的 `requestTimeoutMilliseconds` 仍是 60s（`session.rs:59,152`）→ 最坏情况前台会等 15s | 第 2 步交互验证时**连续快速敲 10 个字符**，观察浮层是否跟得上、以及 `S1_JAVA_COMPLETION` 的 `items=`/`ms=` 与 `staleDocumentVersion` 出现次数；若卡，就在 provider 里加防抖或选 (b)（§6 第 2 步的"防抖怎么放"） |
| 5 | **`state` 锁的争用** | `definition()` 全程持锁（`service.rs:136-139`），`completion()` 现在**同口径**；于是 `F12` 与补全互相阻塞，`shutdown()`（`service.rs:179-186`）也可能被一次 15s 的补全挡住 | 交互验证时"一边打字一边按 `F12`"；若要松开，把锁粒度改成"只保护会话查找 + 文档同步"，请求用 `operationId` 而不是靠锁串行化（这正是第 3 步事件泵要解决的问题） |
| 6 | **`&mut App` 上取 `BackgroundExecutor` 的确切写法**（只在选"(a) 延时防抖"时才需要） | §6 第 2 步的代码里我写了 `cx.background_spawn(..)`（这个是确定的：`GPUI\src\app.rs:3073` 的 `AppContext::background_spawn`），但**没有**核对 `&mut App` 上 `background_executor()` 的可见性 | 查 `GPUI\src\app.rs` 与 `GPUI\src\gpui.rs:237` 的 `background_spawn` 定义处；`BASE\src\input\editor\lsp\completions.rs:287-291` 用的是 `cx.background_executor().clone()`，那里 `cx: &mut Context<..>`。**选 (b) 就绕开了这条。** |
| 7 | **`lsp.plainSnippet` 的实际输出** | 工作区实现已接入它（`service.rs` 的 `plain_snippet`），调用形态 `core_json("lsp.plainSnippet", json!({"value": s}))` 与 `DOC:944-946` / `CORE\src\lsp\lightweight\snippets.rs:9-18` 一致，但**没有实测**过 | 已有一条单测 `completion_item_reads_the_normalized_fields`（`service.rs` 测试模块）覆盖 `"greet(${1:name})"` → `"greet(name)"`；跑它即可（不需要 JDTLS） |
| 8 | **`lsp.closeDocument` 的调用时机对诊断的影响** | 关标签后 Core 仍认为文档开着（我们现在从不调），版本会继续涨 | 第 4 步做诊断时顺手接：`EditorPane::close`（`editor_view.rs:1141`）里对 java buffer 调一次；`DOC:145` 说它会清诊断 |
| 9 | **本机 JDK / JDTLS 载荷是否仍可用** | 第 1 步的选定式测试需要 JDK 21+；PATH 上的 java 是 1.8（`PLAN.md` §10.4 第 3 条），必须显式给 `LITHE_JDTLS_JAVA` | `.artifacts/jdtls` 存在（实测 79,830,573 字节）；`D:\ProgramData\java\openjdk-21\bin\java.exe` 需再确认一次存在性 |
| 10 | **`.out` vs `.err`** | §6 开头记的 `println!` / `eprintln!` 不一致 | 验证脚本两个流都重定向再 grep（`.artifacts/p2/verify-jdt.ps1` 的既有做法可直接照抄） |

### 7.2 结构性风险（需要维护者拍板的）

1. **事件泵的所有权**：第 3 步会往 `lithe-gpui-java` 里加一个 `std::thread`。这与 `PLAN.md` §10.4 第 6 条"窗口关闭时 `EditorPane::drop` **同步**调 `lsp.stopServer`"（`editor_view.rs:2142-2148`，`Drop` 在 `:2143` 调 `service.shutdown()`）需要一起设计收尾顺序，否则会留一个卡在 `waitEvents` 上的线程。**建议**：泵线程只做 `waitEvents`，`stopServer` 之后会话进终态、`waitEvents` 立刻返回空，线程自然退出——但这条**要实测**。
2. **snippet 的"正确处理"是插入后跳到第一个占位符**（真机行为）。本计划只做"还原成纯文本"（与 macOS 的 `LanguageServerSnippet.plainText` 一致，`LanguageEditingCoordinator.swift:45`），**不**做占位符跳转。若维护者要完整行为，需要改上游 `insert_completion`（`overlay.rs:166-197`）——那是 vendored crates 里没有的权限（我们没有 patch registry 的机制）。
3. **`Ctrl+Space` 与真源的一致性**：Windows 的真源是 Monaco 的 `triggerCharacters` + `suggestOnTriggerCharacters` 开关（`monaco-editor.tsx:931,2122`），**没有 `Ctrl+Space`**；macOS 也没有看到 `Ctrl+Space` 绑定（`CodeEditorView.swift:2382,3893,3988,4013` 都是编辑器回调）。所以第 5 步严格说是"新增一个真源没有的入口"，建议**先不做**，或者问维护者。
4. **`gpui/PLAN.md` §10.2 第 1 条**的"组件 `definition_provider` 与会话的冲突"在这里**不适用**（我们只用 `completion_provider`，它不参与 `handle_click_hover_definition` 的 `return`），所以补全与 `F12`/`Ctrl+单击`可以共存。**这一点我核对过**：`handle_click_hover_definition`（`definitions.rs:129-154`）只看 `hover_definition`，不看 `completion_provider`；`handle_completion_trigger`（`completions.rs:122-146`）只在 `completion_inserting` 时早退。

### 7.3 与设置页的关系（`HANDOFF.md` §4 队列第 2 项的 LSP 页）

真源 LSP 页的三个开关 `autoCompletion` / `parameterHints` / `semanticTokens` 目前在 gpui 侧**没有消费方**，所以照 §7.3-D 的口径"不画假控件"（`PLAN.md` §15.5 第 4 条）。
本调研之后，**`autoCompletion` 第一次有了真实消费方** —— 它可以接到第 2 步的 `is_completion_trigger` 上（关掉就恒 `false`）。要落这一项时按 `PLAN.md` §14.2 的口径改**两处**：`settings/src/schema.rs`（字段 + `Default` + 序列化键名测试）与消费方（`store.rs` 的 setter + 编辑器转发）。`persistence.rs` **不需要改** —— 键表已由 schema 派生，坏键回落路径自动覆盖新字段，守卫测试 `any_single_broken_key_leaves_every_other_key_intact` 会照出没进去的字段。
`parameterHints`（签名帮助）与 `semanticTokens` 仍无消费方：前者上游**完全没有**接口（§2.10），后者要先把 Core 的 full 结果切区间（§2.6）。
