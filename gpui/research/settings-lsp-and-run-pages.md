# 设置里的「LSP」与「运行配置」两页：可执行实现规格

> 目的：为设置对话框里**现在还是空态**的两页（`Category::Lsp` / `Category::Run`，
> `gpui/crates/settings/src/dialog.rs:642-647` 的 `empty_page`）查清
> 「哪一条前置条件已经满足、哪一条还不满足」，并给出可执行的实现规格。
>
> **只读调研**：本次没有改任何产品代码，只新增本文件。
> 允许的手段：`rg` / 读文件 / `git status` / `git log`。**没有跑 cargo、没有启动 Lithe。**
>
> 仓库外的证据路径（下文只写 crate 名）：
> `gpui-base-0.6.6` = `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-base-0.6.6`
> （不是 vendored 源码，是 cargo registry 里的上游 crate）。
>
> 本文把**事实**（有 `文件:行号` 的）与**推断**（我根据事实做的判断）分开写；
> 推断一律以「**推断**」开头。
>
> **⚠️ 行号快照**：本文件写作期间，工作区里**另一个代理正在接线「项目 · JDK 与 Maven」页**
> （`git status`：`gpui/crates/settings/{dialog,persistence,schema,store}.rs`、
> `gpui/crates/shared/{locales/*,src/i18n.rs}`、`gpui/tools/extract-locale.mjs` 均已修改，
> 另有未跟踪的 `gpui/crates/settings/src/project.rs`）。
> 因此 **`gpui/crates/settings/src/dialog.rs` 的行号正在变动**（我读到的版本里
> `content()` 的分支在 `:632-650`、`IMPLEMENTED` 在 `:248-253`；同一批改动落地后可能平移几十行）。
> 其余文件（`java/**`、`editor/**`、`workbench/**`、`rust/**`、`windows/**`）不受影响。
> 取行号时请以**符号名**（`Category::IMPLEMENTED` / `content()` / `prerequisite_key()` /
> `empty_page()`）为准，行号只作定位起点。

---

## 1. 结论速览

| 页 | 能不能做「有真值的版本」 | 一句话依据 |
| --- | --- | --- |
| **LSP** | **能，但只能有一个真控件**（`autoCompletion`）。另两项（`parameterHints` / `semanticTokens`）**都不能**做；JDTLS 运行时路径输入框**不建议做**（Windows 已把同类键退役）。 | `autoCompletion` 有真消费方：`gpui/crates/editor/src/completion.rs:225-241` 的触发判据 + `gpui/crates/editor/src/editor_view.rs:630-631,662` 的装载点。另两项缺的是**子系统**不是接线：上游 `Lsp` 结构体根本没有签名帮助接口（`gpui-base-0.6.6/src/input/editor/lsp/mod.rs:39-69`），语义高亮虽有上游 trait（同文件 `:51`）但 Java 侧一个方法都没实现（`gpui/crates/java/src/service.rs:196-628` 的公开方法清单里没有 `semantic_tokens`）。 |
| **运行配置** | **能，只读列表版**（列出 Core 识别到的可运行目标 + 如实标注「运行尚未接入」）。 | Core 的 `runConfig.generate` **一次调用**就从文件树产出带 `name` / `provider` / `execution` / `command` / `cwd` / `source` 的配置数组（`rust/lithe-core/src/execution/configuration.rs:460,654-659`；探测器表 `rust/lithe-core/src/execution/detectors/mod.rs:227-237`），而 gpui 侧只需 `core_json` + 一个后台任务（`gpui/crates/shared/src/core_client.rs:223`）。但 gpui **完全没有运行子系统**：无 run crate（`gpui/crates/` 只有 app/editor/explorer/git/java/settings/shared/terminal/workbench）、Run 工具窗是占位、Run 菜单为空、命令面板无 Run 命令。 |

**一句话总纲**：两页都不是"前置条件完全不满足"，但满足的程度不同 ——
LSP 页是「子系统已有、开关只差一个转发点」，运行配置页是「**数据**已有、**执行**完全没有」。
所以 LSP 页应该做成**真开关页**，运行配置页应该做成**只读事实页**且明确写出缺什么。

---

## 2. LSP 页

### 2.1 真源字段表（含标签键与实际 grep 到的行号）

真源组件：`windows/tauri/src/features/settings/components/macos-settings-panels.tsx:398-438`
的 `LspPanel()`（**内联**在 `macos-settings-panels.tsx` 里，**没有**独立的 `lsp-*.tsx` 文件
—— `windows/tauri/src/features/settings/components/` 下的文件清单里没有任何 lsp 命名的文件）。
分类表项：`windows/tauri/src/features/settings/components/settings-dialog.tsx:42`
（`{ id: "lsp", labelKey: "settings.tabs.lsp", icon: DatabaseIcon }`）。

| # | 设置键 | 标签键（真源） | 标签键在 `lithe.zh-CN.yml` 的行号 | 描述键 / 行号 | 控件 | 默认值 | 真源行号 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `autoCompletion` | `settings.mac.autoCompletion` | **3794** | `settings.mac.autoCompletionDescription` / **3796** | 开关 | `true` | `:406-415` |
| 2 | `parameterHints` | `settings.mac.parameterHints` | **3798** | **无描述** | 开关 | `true` | `:416-422` |
| 3 | `semanticTokens`（**标签键叫 `semanticHighlighting`**） | `settings.mac.semanticHighlighting` | **3800** | **无描述** | 开关 | `true` | `:423-429` |
| — | （分组标题） | `settings.mac.languageServices` | **3792** | — | `SettingsGroup` | — | `:405` |
| — | （分组标题） | `settings.mac.detectedServers` | **3802** | `settings.mac.detectedServersDescription` / **3804** | `SettingsGroup` | — | `:431-435` |

分类名键：`settings.tabs.lsp` = `lithe.zh-CN.yml:3670`（「LSP」），
gpui 已接（`gpui/crates/settings/src/dialog.rs:287`、`gpui/crates/shared/src/i18n.rs:320`）。

默认值的三处来源（互相一致）：

- `windows/tauri/src/features/settings/config/default-settings.ts:72` `semanticTokens: true`
- 同文件 `:153` `autoCompletion: true`、`:154` `parameterHints: true`
- 类型声明：`windows/tauri/src/features/settings/types/settings.types.ts:63`（`semanticTokens`）、
  `:152`（`autoCompletion`）、`:153`（`parameterHints`）

**⚠️ 真源里「已检测语言服务器」分组里没有任何列表、也没有任何输入框** ——
它只有一句 `<p>`（`:431-435`）。这一点与任务描述里"有没有『已检测语言服务器』列表、
有没有 JDTLS 运行时路径输入框"的答案是：**都没有**。

### 2.2 持久化位置（真源 vs gpui）

- **真源**：Tauri Store 的 `settings.json`（`windows/tauri/src/features/settings/lib/settings-persistence.ts:50`
  的 `load("settings.json", ..)`），键名就是上表的驼峰键名。
- **gpui**：`%APPDATA%\Lithe\settings.json`（`gpui/crates/settings/src/paths.rs:61-69`），
  可用 `LITHE_GPUI_SETTINGS_FILE` 覆盖（`:28,45-52`）。落盘走「原子写 + 300ms 防抖」
  （`gpui/crates/settings/src/persistence.rs:216-229`、`:34`）。
- **gpui 当前这三键一个都没有**：`gpui/crates/settings/src/schema.rs:106-205` 的 `Settings`
  全文（`theme` / `syncSystemTheme` / `autoThemeLight` / `autoThemeDark` / `uiFontSize` /
  `showStatusBar` / `displayLanguage` / `fontSize` / `tabSize` / `terminalDefaultShellId` /
  `confirmBeforeDiscard`）里没有 `autoCompletion` / `parameterHints` / `semanticTokens`。

### 2.3 消费方清单（逐项，`文件:行号`）

#### 2.3.1 `autoCompletion` —— **有真消费方**

事实：

1. 补全 provider 的装载点是 `gpui/crates/editor/src/editor_view.rs:630-631`
   （`(language == Some("java")).then(|| JavaCompletionProvider::new(...))`）与
   `:662` 的 `crate::completion::install(&mut state, completion_provider)`。
   **判据只有"这个文件是不是 `.java`"，没有任何设置参与。**
2. "服务起得比文件晚"的补装路径同样是硬编码的：
   `editor_view.rs:396-426`（`prepare_java()`，`:405-422` 的循环里对每个 Java buffer
   `completion::install(state, Some(provider))`）。
3. `install` 的语义是「`None` 就什么都不做」：
   `gpui/crates/editor/src/completion.rs:79-89`（`:83-85` 的早退）。
   所以**不装 provider ⇒ 上游在 `completions.rs:133-135` 直接早退 ⇒ 菜单永不出现**。
4. provider 内部另有一个**纯函数**触发判据：
   `gpui/crates/editor/src/completion.rs:225-227`（`is_completion_trigger` →
   `triggers_on(new_text)`）与 `:236-241`（`triggers_on` 的判据：
   标识符字符 / `.` / `@` 才触发）。
5. 上游的调用点：`gpui-base-0.6.6/src/input/editor/lsp/completions.rs:122-146`
   （`:133-135` 取不到 provider 就返回；`:144` 才问 `is_completion_trigger`）。

**推断**：这里有两个可用的门控点，取舍如下 ——

| 门控点 | 改哪里 | 优点 | 缺点 |
| --- | --- | --- | --- |
| (a) **不装 provider** | `editor_view.rs:630-631` + `:405-422` 按设置传 `None` | 一行判据，连菜单宽度都不改 | 关掉之后**连"轻量兜底"也没了**（`completion.rs:210-217` 的 `builtin_completions` 是同一个 provider 的数据源）；将来做手动触发（`Ctrl+Space`）时也要重新判断 |
| (b) **门控 `is_completion_trigger`**（推荐） | `completion.rs:225-227` 读一个 `auto_completion: bool` 字段；由 `EditorPane` 在装载时把设置值传进 `JavaCompletionProvider::new`（`:103-112` 加一个参数）或后续 `set_auto_completion` 更新 | 语义最准：真源描述原文就是「显示活动语言服务器提供的**补全建议**」（`lithe.zh-CN.yml:3797`），关掉=**不再自动弹**，与"能不能手动要一次"解耦 | provider 仍装着，需要一处 `Rc<AtomicBool>` 或 `Cell` 才能被外壳运行时改动（provider 是 `Rc<dyn CompletionProvider>`，`is_completion_trigger` 收 `&self`） |

两条路都会**同时**关掉 JDTLS 补全与 Core 轻量兜底，这一点必须在描述里如实写
（真源的描述句只提"活动语言服务器"，照抄会让用户以为兜底还在）。

**转发缝已有现成模板**：`EditorPane::set_tab_size`（`editor_view.rs:345-379`）
+ 外壳订阅转发（`gpui/crates/workbench/src/workspace.rs:868-906`，
`:882-884` 启动时喂一次、`:891-905` 每次变化再喂）。
`autoCompletion` 应逐字照这一套：`SettingsStore` 加 setter → 外壳订阅 → `EditorPane::set_auto_completion`。

#### 2.3.2 `parameterHints` —— **没有消费方，且上游没有接口**

事实：

1. 上游 `Lsp` 结构体的 provider 字段**全部**列在
   `gpui-base-0.6.6/src/input/editor/lsp/mod.rs:39-69`：
   `completion_provider`（`:41`）、`code_action_providers`（`:43`）、`hover_provider`（`:45`）、
   `definition_provider`（`:47`）、`document_color_provider`（`:49`）、
   `semantic_tokens_provider`（`:51`）、`show_document`（`:56`）。
   **没有 `signature_help_provider`、没有 `parameter_hints` 之类的字段。**
2. 整个 `gpui-base-0.6.6` crate 的 `src/` 下 `signature` 只有 **2 处命中**，且都是文档里
   "函数签名"的意思，与 LSP 的 `textDocument/signatureHelp` 无关：
   `gpui-base-0.6.6/src/dock/dock_area.rs:185`、`.../dock/layout/builder.rs:22`。
   **`SignatureHelp` 字符串零命中。**
3. LSP 模块的文件清单也没有签名帮助这一层：
   `gpui-base-0.6.6/src/input/editor/lsp/mod.rs:8-14` = `code_actions` / `completions` /
   `definitions` / `document_colors` / `hover` / `overlay` / `semantic_tokens`。
4. gpui 侧全仓库搜 `parameterHints` / `SignatureHelp` / `signature_help` 只命中
   **文案键、文档与调研笔记**，没有一行实现（`gpui/crates/shared/locales/lithe.zh-CN.yml:3798,4338`、
   `gpui/research/editor-lsp-completion.md:754,790`、`gpui/research/windows/10-menu-bar.md:152`）。

**结论（事实级）**：`parameterHints` **没有消费方，也没有可挂载的上游接口**。
任务描述里"有的话装在哪；没有就明确说没有"的答案是：**明确没有**。
要做得先自建一个浮层（真源是 Monaco 的 `SignatureHelpTooltip`，
`gpui/UI-MAP-WINDOWS.md:815`），那是"新做一个编辑器浮层"，不是"接一个开关"。

#### 2.3.3 `semanticTokens` —— **上游有接口，Java 侧零实现**

事实：

1. 上游**有** trait 与挂载点：`DocumentRangeSemanticTokensProvider`
   （`gpui-base-0.6.6/src/input/editor/lsp/semantic_tokens.rs:36-55`：`legend()` + `semantic_tokens(..)`），
   挂载点 `Lsp.semantic_tokens_provider`（`mod.rs:51`）。
2. 上游在**每次文本变化**时都会拉一次：
   `mod.rs:92-101` 的 `Lsp::update` → `:100` `self.update_semantic_tokens(text, window, cx)`；
   真正的请求在 `semantic_tokens.rs:108-150`（`:114` 没 provider 直接返回）。
3. **Java 侧没有实现**：`gpui/crates/java/src/service.rs` 的公开方法只有
   `new`（`:198`）、`prepare`（`:213`）、`definition`（`:227`）、`completion`（`:298`）、
   `code_actions`（`:405`）、`diagnostics`（`:554`）、`sync_document`（`:597`）、`shutdown`（`:621`）。
   **没有 `semantic_tokens`。**
4. gpui 全仓库搜 `semantic` 只命中：文案键、上游注释、以及
   `gpui/crates/editor/src/navigation.rs:453` 一条**测试名**里的 "semantic path"
   （指"语义导航路径"，不是语义高亮）。**没有任何 `semantic_tokens_provider` 的赋值点。**

**推断**：Core 侧的数据源是齐的（`lsp.request` 支持
「full-document semantic tokens」，契约 `shared/contracts/rust-core-api.md:1341`），
缺的是三层：`JavaLanguageService::semantic_tokens` → `DocumentRangeSemanticTokensProvider` 适配器
→ `editor_view.rs` 的装载点。这是**一个独立特性**，不是设置页的一项。
另有性能前提要先量：上游每次击键都请求一次（`mod.rs:100`），
而本仓库已记录过补全往返"最坏前台等 15s"（`gpui/research/editor-lsp-completion.md:771`）。

#### 2.3.4 JDTLS / JDK 运行时路径 —— 发现顺序与"最小改动点"

**发现顺序（事实，逐行）**：

JDTLS 可执行文件 —— `gpui/crates/java/src/jdtls.rs:330-344` 的 `find_executable`，
候选按此顺序拼接，取**第一个存在的常规文件**（`:343`）：

1. `bundled_roots()`（`:350-368`）：`<exe 目录>/LanguageServers/jdtls`，
   再从 exe 目录向上最多 12 层（`MAX_EXE_WALK_DEPTH`，`:24`）找 `<dir>/.artifacts/jdtls`，
   并用 `is_root`（`:405-409`）过滤（要能看见 `jdtls.bat|.cmd|.exe|jdtls`，根目录或其 `bin/`）。
2. `PATH` 的每个目录（`:335-339`），候选名顺序 = `EXECUTABLE_NAMES`（`:22`：
   `jdtls.bat` → `jdtls.cmd` → `jdtls.exe` → `jdtls`）。
3. `search_roots(workspace_root)`（`:371-403`），顺序为：
   `JDTLS_HOME`（`:373`）→ `LITHE_JDTLS_ROOT`（`:376`）→ 对
   `LOCALAPPDATA` / `ProgramFiles` / `ProgramFiles(x86)` / `XDG_DATA_HOME` 各拼
   `jdtls`、`Eclipse JDT Language Server`、`Programs/jdtls`（`:382-389`）→
   对 `USERPROFILE`（回落 `HOME`）拼 `~/.jdtls`、`~/scoop/apps/jdtls/current`、`~/scoop/shims`（`:390-395`）
   → `<workspace_root>/.lithe/toolchains/jdtls`（`:396-401`）。

跑 JDTLS 的 JDK —— `jdtls.rs:440-495` 的 `resolve_runtime`，逐个候选跑 `java -version`
（`probe_java`，`:566-582`），第一个主版本 ≥ 21（`MINIMUM_JAVA_MAJOR`，`:74`）的胜出：

1. `LITHE_JDTLS_JAVA`（`:40,442-447`）；
2. `bundled_jdk_roots()`（`:448` → `:498-516`）：`<exe>/LanguageServers/jdk`、向上 12 层的 `.artifacts/jdk`；
3. `<jdtls_root>/jre`（`:449`）；
4. `JAVA_HOME`（`:450-454`）；
5. `PATH` 的每个目录（`:455-459`）；
6. `installed_jdk_roots()`（`:460` → `:519-563`）：`~/.jdks`、`~/.sdkman/candidates/java`、
   `~/scoop/apps`，`ProgramFiles`/`ProgramFiles(x86)`/`LOCALAPPDATA` 下的
   `Java`/`Eclipse Adoptium`/`Microsoft`/`Amazon Corretto`/`Zulu`/`BellSoft`/`Programs`，
   `ProgramData/java`，`XDG_DATA_HOME/jdks`，并各自再枚举一层子目录。

两个入口的调用点：`gpui/crates/java/src/service.rs:660-661`
（`jdtls::resolve(&self.workspace_root)?` 与 `jdtls::resolve_runtime(installation_root(&installation.executable))?`）。

**若维护者坚持要一个覆盖值：最小改动点（函数签名级，推断）**

三条路，从最小到最大：

- **(A) 不做新设置项，复用既有环境变量（零代码）**：
  `LITHE_JDTLS_ROOT`（`jdtls.rs:376`）与 `LITHE_JDTLS_JAVA`（`:40`）已经是"装到非常规位置"的入口，
  且 `:38-39` 的文档明确说它们**只覆盖推导、不是常规入口**。
  代价：进程级环境变量，运行中改不了，设置页里做不了输入框。
- **(B) 真做设置项（若维护者要）—— 需要改 4 处签名**：
  1. `gpui/crates/java/src/jdtls.rs:98`
     `pub(crate) fn resolve(workspace_root: &Path) -> Result<JdtlsInstallation, String>`
     → `resolve(workspace_root: &Path, root_override: Option<&Path>) -> Result<JdtlsInstallation, String>`；
     实现上把 `root_override` 插到 `find_executable`（`:330`）候选序列的**最前**，
     并且**仍然只通过 `resources_for_root`（`:160`）验收**（不允许绕过"缺件即报错"那条判据）。
  2. `gpui/crates/java/src/jdtls.rs:440`
     `pub(crate) fn resolve_runtime(jdtls_root: &Path) -> Result<JavaRuntime, String>`
     → `resolve_runtime(jdtls_root: &Path, java_override: Option<&Path>) -> Result<JavaRuntime, String>`；
     把 override 插在 `JAVA_EXECUTABLE_ENV`（`:442`）**之前或之后**要明确指定一个顺序
     （推断：放在 env 之后更稳，env 仍是"验证链路的最高优先级"）。
     仍然必须过 `probe_java` 的 ≥ 21 闸门（`:473,578-580`）。
  3. `gpui/crates/java/src/service.rs:198` `JavaLanguageService::new(workspace_root: PathBuf)`
     → 加一个字段 + `pub fn set_jdtls_overrides(&self, root: Option<PathBuf>, java: Option<PathBuf>)`
     （**不要**改 `new` 的签名：`editor_view.rs:393` 是唯一调用点，但它不该关心设置）。
     然后 `service.rs:660-661` 把 override 传进去。
  4. 外壳转发：`gpui/crates/workbench/src/workspace.rs:891-905` 那条 `cx.observe` 里加一路，
     并在 `:882-884` 的启动段补一次；但**目录变更要重启 JDTLS 会话**（`service.rs:621` 的 `shutdown`
     是唯一的关闭路径），这条语义在真源里不存在，属于新设计。
- **(C) 只读展示"检测到的 JDTLS / JDK"（推荐，见 2.4）**：不改发现逻辑，
  只把已经在跑的发现结果暴露出来。落点：
  `service.rs:739-742` 已经把 `(JdtlsInstallation, JavaRuntime)` 存进了
  `JavaLanguageService.installation`（字段 `:193`，**私有**），加一个只读访问器
  （如 `pub fn detected_toolchain(&self) -> Option<DetectedJavaToolchain>`）即可。

**⚠️ 强烈建议不要做输入框（事实级理由）**：Windows 真源**已经退役过一个同类键**
`jdtlsJavaHomePath`：`windows/tauri/src/features/settings/lib/settings-normalization.ts:550`
`delete (normalizedSettings as { jdtlsJavaHomePath?: unknown }).jdtlsJavaHomePath;`，
并且有守卫测试 `windows/tauri/src/features/settings/lib/settings-normalization.test.ts:26-33`
「retired JDTLS JDK setting normalization / discards the former user-configurable JDK home」。
本任务书里"设置里的 JDTLS 运行时路径输入框"这个前置假设，
在真源里是**被产品主动拿掉的东西**。
（另：gpui 侧现在还有一条独立的「项目 · JDK 与 Maven」页正在做 —— 见 §4.1 ——
JDK 路径的选择应该归那一页，避免同一个概念两个入口。）

### 2.4 每个字段的处置表

| 真源项 | 处置 | 理由 | 若做：消费者要改哪个文件的哪个函数 |
| --- | --- | --- | --- |
| `autoCompletion`（自动补全） | **✅ 做** | 唯一有真消费方的项（`completion.rs:225-241`）。 | ① `gpui/crates/settings/src/schema.rs`：`Settings` 加 `#[serde(rename = "autoCompletion")] pub auto_completion: bool`（默认 `true`）+ `Default` + `normalize()` 不需要（bool）；② `gpui/crates/settings/src/persistence.rs:144-198` 的逐键表加 `take(object, "autoCompletion", &mut settings.auto_completion, diagnostics)`；③ `gpui/crates/settings/src/store.rs` 加 `set_auto_completion`；④ `gpui/crates/editor/src/completion.rs:103-112` 的 `JavaCompletionProvider::new` 加 `auto_completion` 参数（或 `:92-99` 结构体加字段），`:225-227` 的 `is_completion_trigger` 读它；⑤ `gpui/crates/editor/src/editor_view.rs:630-631` 与 `:409-412` 两处传值 + 加 `pub fn set_auto_completion(&mut self, enabled: bool, cx)`（模板 `:345-379`）；⑥ `gpui/crates/workbench/src/workspace.rs:882-905` 转发；⑦ LSP 页新增 `lsp_page()`；⑧ `dialog.rs:248-253` 的 `IMPLEMENTED` + `:642-647` 的 `content()` 分支同时改（`every_category_is_implemented_or_declares_a_prerequisite`，`dialog.rs:1848-1863` 会守住一致性）。 |
| `parameterHints`（参数提示） | **❌ 不做，且页面上不画**（连置灰都不画） | 上游 `gpui-base-0.6.6` 没有签名帮助接口（`mod.rs:39-69` 的字段清单、全 crate `SignatureHelp` 零命中），也没有任何浮层可复用。画一个永远不生效的开关违反 `gpui/research/windows/07-settings-ui.md` §7.3-D 与 `dialog.rs:189-191` 的既有口径。 | — |
| `semanticTokens`（语义高亮） | **❌ 本批不做，且页面上不画** | 上游 trait 在（`semantic_tokens.rs:36-55`、挂载点 `mod.rs:51`），但 Java 侧零实现（`service.rs` 无 `semantic_tokens`），也没有任何装载点。这是"新做一个特性"，不是"接一个开关"。 | 若将来做：`service.rs` 加 `pub fn semantic_tokens(&self, file_path, text) -> Result<Vec<JavaSemanticToken>, String>`（包 `lsp.request{operation: semanticTokens}`）→ 新文件 `gpui/crates/editor/src/semantic_tokens.rs` 实现 `DocumentRangeSemanticTokensProvider` → `editor_view.rs:662` 附近装载。**这一项不该由设置页排期**。 |
| 「已检测语言服务器」分组 | **✅ 做，但改成一个真列表**（**推断**，需维护者拍板） | 真源这里是**静态文本**（`:431-435`），但 gpui 侧"检测到了什么"是**真有的事实**且**已经在算**：`service.rs:659-670` 每次启动都会解析 JDTLS 与 JDK，并打 `S1_JAVA_JDTLS executable=… version=… java=… javaVersion=… launcher=… config=…`。把它显示出来是"呈现已有事实"，不是假控件。 | ① `gpui/crates/java/src/service.rs:193` 的私有字段加只读访问器（如 `pub fn detected_toolchain(&self) -> Option<DetectedJavaToolchain>`，含 executable / version / java home / java version）；② gpui 侧需要一个"从设置对话框拿到 Java 服务"的通道 —— 依赖方向不允许 `settings → java`，照 `gpui/crates/settings/src/identity.rs:1-73` 的**宿主钩子**写法（线程局部 + `set_*_host`，由 `gpui/crates/workbench/src/workspace.rs:771-820` 登记）；③ `dialog.rs` 新增 `lsp_page()`。**注意**：启动 JDTLS 是重活，列表只能显示"已经起过的会话"或走 `background_spawn`，**不许在 render 里同步探测**。 |
| JDTLS 运行时路径输入框 | **❌ 不做** | ① 真源没有这个控件；② Windows 已退役同类键 `jdtlsJavaHomePath`（`settings-normalization.ts:550` + 测试 `:26-33`）；③ 既有的 `LITHE_JDTLS_ROOT` / `LITHE_JDTLS_JAVA` 已覆盖这个需求（`jdtls.rs:38-40`）；④ JDK/Maven 路径选择已在「项目 · JDK 与 Maven」页（§4.1）。 | 若维护者坚持：见 §2.3.4 (B) 的四处签名改动。 |

### 2.5 空态现在为什么过时 / 新文案怎么写

现状（事实）：

- `gpui/crates/settings/src/dialog.rs:210-211` 把 Lsp 记为「空态」；
- `:302-313` 的 `prerequisite_key()` 返回 `lithe.settings.gpui.prerequisiteLsp`；
- 该键的值（`gpui/crates/shared/locales/lithe.zh-CN.yml:8668-8669`、
  `gpui/crates/shared/src/i18n.rs:362-365`）是：
  「前置条件：语言服务客户端（补全、参数提示、语义高亮）与 JDTLS 运行时路径设置。」

这句话现在**有一半是错的**：补全客户端**已经有了**（`editor_view.rs:630-631` +
`gpui/crates/editor/src/completion.rs` 全文件 + `gpui/crates/java/src/service.rs:298`）。
另一处已经过时的记录：`gpui/tools/extract-locale.mjs:158` 里这条键的理由写着
「真源那页的三个开关在 gpui 侧没有消费方；能真做的 JDTLS 运行时路径还没做成设置项
（`java/src/jdtls.rs:440-460` 现在是 env/JAVA_HOME/PATH 三级发现）」——
"三级发现"这个描述本身也不准（实际见 §2.3.4 的 6 级）。

**推断**：升级成实现页时，`prerequisite_key()` 要返回 `None`（`IMPLEMENTED` 表加 `Category::Lsp`），
并且**同时**清理：
- `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里 `prerequisiteLsp` 那条（或改成只描述仍未做的两项）；
- `i18n.rs:362-365` 的接线表项与两个 yml 的对应键（`lithe.zh-CN.yml:8668`、`lithe.en.yml` 同键）。
  **注意** `i18n.rs:766-772` 的 `SAME_IN_BOTH_LOCALES` 例外清单只影响"en 不能等于中文"那条断言，
  删键时要一起清。

---

## 3. 运行配置页

### 3.1 真源字段表

真源：`windows/tauri/src/features/settings/components/run-configuration-settings.tsx:11-114`
（**不是** Settings 体系的一个页签，而是独立组件，`gpui/research/windows/07-settings-ui.md:186-194` 已登记）。

**两种形态**（事实）：

| 形态 | 判据 | 真源行号 | 内容 |
| --- | --- | --- | --- |
| 无项目 | `!root` | `:50` | `settings.project.openProject`（`lithe.zh-CN.yml:3600`） |
| 加载中 | `state.root !== root \|\| state.isLoading` | `:51-54` | `settings.project.loading`（`:3614`） |
| **编辑某条配置** | `state.configurations` 找得到 `editingConfigurationId` | `:55-78` | 标题 = `configuration.name`（`:58`）+ `RunConfigurationEditor`（`:59-75`） |
| **列表态** | 否则 | `:79-113` | 说明 + 「重新识别」按钮 + 每条配置一个全宽 ghost 按钮 |

列表态的元素（事实）：`settings.run.description`（`:81`，键 `lithe.zh-CN.yml:3598`）→
错误提示 `role="alert"`（`:82-91`，`state.invalidMessage` / `state.saveError`）→
生成提示 `run.generatedEntries`（`:92-96`，键 `:3148`）→
按钮 `run.identifyAgain`（`:97-99`，键 `:3118`；生成中显示 `settings.project.loading`）→
配置按钮列表（`:100-111`，每条显示 `entry.name` + `ui.edit`（键 `:456`），
`disabled` 判据 = `state.status !== "ready" || state.isGenerating`）。

**编辑形态的字段表**（真源 `windows/tauri/src/features/run/components/run-configuration-editor.tsx:210-433`）：

| 区块 | 字段 | 标签键 | `lithe.zh-CN.yml` 行号 | 真源行号 |
| --- | --- | --- | --- | --- |
| 项目默认值（local） | 分组标题 | `run.projectDefaultsSection` + `run.saveScopeLocal` | 3220 / 3166 | `:214-217` |
| | 提示 | `run.saveScopeLocalHint` | 3170 | `:217` |
| | 「配置项目环境…」入口 | `settings.project.openSettings` | 3620 | `:219-223` |
| | Node 可执行文件 | `run.nodeExecutable` / `run.nodeExecutableHint` | 3196 / 3198 | `:224-242` |
| 配置事实 | `name` | （无键，直接渲染） | — | `:58` |
| | 类型 | `run.type` | 3176 | `:249-250` |
| | 生效来源 | `run.effectiveSource` + `run.source.{generated,project,local}` | 3178 / 3180,3182,3184 | `:251-252` |
| | main class | `run.mainClass` | 3186 | `:253-258` |
| 保存范围 | 分段按钮 | `run.saveScopeLocal` / `run.saveScopeProject` / `run.saveScopeLocalHint` / `run.saveScopeProjectHint` | 3166 / 3168 / 3170 / 3172 | `:262-283` |
| 覆盖项 | 分组标题 | `run.configurationOverridesSection` + `settings.project.overrides` | 3222 / 3622 | `:286-289` |
| | JDK 主目录 | `run.jdkHome` / `run.configurationOverrideHint` | 3188 / 3226 | `:290-314` |
| | Maven 可执行文件 | `run.mavenExecutable` | 3192 | `:315-339` |
| | Maven 用的 JDK | `run.mavenJdkHome` | 3200 | `:340-357` |
| | Maven 测试 | `run.mavenTests` / `...ProjectDefault` / `...Run` / `...Skip` / `...Hint` | 3204 / 3206 / 3208 / 3210 / 3212 | `:358-384` |
| | 程序参数 | `run.programArguments` | 3228 | `:387-395` |
| | VM 参数 | `run.vmArguments` | 3230 | `:396-404` |
| | 工作目录 | `run.workingDirectory` / `run.workingDirectoryHint` | 3238 / 3240 | `:405-414` |
| | 环境变量 | `run.environment` | 3232 | `:415-424` |
| 底部 | 取消 / 保存 | `run.cancel` / `ui.save` | 3246 / 452 | `:428-432` |
| 下拉里的兜底项 | 自动 / 当前 | `run.toolchainAuto` / `run.toolchainCurrent` / `run.toolchainProjectDefault` | 3214 / 3216 / 3224 | `:84-88,230-231,296-297` |

**数据从哪来（真源调用链，事实）**：

```
RunConfigurationSettings (run-configuration-settings.tsx:42-48)
  → ensureRunSettingsProject(useRunStore, root)            services/run-settings-project.ts:11-15
  → mavenLaunchContextForWorkspace(root)                   features/maven/stores/maven.store
run.store.loadProject(root)  (stores/run.store.ts:432, :318, :391)
  → inspectRunConfiguration(root, checkFingerprint)        api/run-core-api.ts:43-45  → Core runConfig.inspect
  → generateRunConfiguration(root, paths, modulePaths, javaEntrypoints)
                                                           api/run-core-api.ts:62-73  → Core runConfig.generate
       paths 来自 host: listJavaSources(root)              api/run-host-api.ts:10-12
       javaEntrypoints 来自 JDT: discoverJavaEntrypoints   services/java-entrypoint-discovery.ts:41-68
  → resolveRunConfiguration(root, toolchainCandidates)     api/run-core-api.ts:75-80  → Core runConfig.resolve
       组装见 services/resolve-run-project.ts:39-96
  → discoverRunToolchains / resolveRunToolchains（**宿主 API**）
                                                           api/run-host-api.ts:38-45, :72-77
```

### 3.2 gpui 运行子系统现状

**事实清单**：

| 位置 | 现状 |
| --- | --- |
| crate 清单 | `gpui/crates/` 只有 `app` / `editor` / `explorer` / `git` / `java` / `settings` / `shared` / `terminal` / `workbench` —— **没有 run / launch / debug crate**。 |
| 底部工具窗 | `BottomPaneKind::Run` 与 `::Diagnostics` **存在但是占位**：`gpui/crates/workbench/src/workspace.rs:264-274`（注释「阶段 6 前用占位内容」）、`:297-305`（活动栏下标 3 = Run、5 = Diagnostics）、`:1856-1863`（画 `format!("{} 工具窗（未实现）", kind.label())`）。 |
| 活动栏 | Run 项有图标与标签（`workspace.rs:283` 的 `tr("lithe.workbench.run")`，键 `lithe.zh-CN.yml:3030`）。 |
| 顶部菜单 | 「运行」菜单**是空的**：`gpui/crates/workbench/src/menu_bar.rs:340-344`（`TopMenu { id: "run", label_key: "lithe.menu.run", items: &[] }`）。真源视图菜单里的「运行和调试」明确没做（`menu_bar.rs:360` 的注释）。 |
| 命令面板 | `CommandId` 枚举只有 11 条（`gpui/crates/workbench/src/command_palette.rs:187-210`：开关设置 / 主题 / 终端 / Maven / 状态栏 / 菜单栏 / 保存 / 命令面板 / 跳转定义）——**没有任何 Run 动作**。 |
| `run.*` 文案键 | gpui 只用了 3 条，而且是**终端**在用：`gpui/crates/terminal/src/constants.rs:33-37`（`lithe.run.running` / `succeeded` / `failed`，仪表盘状态用）。`lithe.run.*` 其余 100+ 条全部来自真源抽取，**在 gpui 侧零消费方**。 |
| 「运行配置」设置页 | `Category::Run` → `empty_page`（`gpui/crates/settings/src/dialog.rs:206-207`, `:642-647`），前置条件键 `lithe.settings.gpui.prerequisiteRun`（`:306`；文案 `lithe.zh-CN.yml:8664-8665`）。 |
| Core 命令使用 | gpui 全仓库（`gpui/`）搜 `runConfig` **零命中**（只有调研笔记与 PLAN 里提到）。已接的同类只有 `maven.scan`（`gpui/crates/workbench/src/maven.rs:93-118`）与 `spring.index`（`gpui/crates/workbench/src/spring.rs:101-102`）。 |

**推断**：「点绿三角把应用跑起来」需要**至少**四样现在都没有的东西：
启动计划消费、进程宿主（子进程 + stdout/stderr 流）、输出面板、停止/孤儿回收。
`gpui/research/java-spring-maven-inventory.md:602-620` 的 M4 已经把这个列为独立里程碑，
并明确「宿主进程能力 gpui 已有终端，可复用其 PTY 经验」。**这不是设置页范围内的事。**

### 3.3 Core 侧可直接用的事实

契约命令表：`shared/contracts/rust-core-api.md:162-168`。实现：`rust/lithe-core/src/execution/configuration.rs`。

#### 3.3.1 `runConfig.generate` —— **本页最小版本的唯一必需命令**

- 请求（`configuration.rs:39-51`，camelCase）：`{ root, paths?: string[], modulePaths?: string[], javaEntrypoints?: {...} }`。
- 响应（`configuration.rs:654-659`）：
  `{ generated: { version, generator: {fingerprint, inputs}, configurations: [...] },
     toolchainRequirements, entryCount, javaEntrypointsOrigin }`。
- **不写任何文件**：契约 `:1536-1538`「It returns generated configuration
  and toolchain requirement documents for the platform adapter to write atomically」。
- 探测**由 Core 自己走文件树**：`generate()` 先算 `maven_root`（`:469` → `project::maven_root`
  在 `rust/lithe-core/src/project/maven.rs:832-837` 把 `PathBuf::new()`（即工作区根自己）
  无条件放进候选集，所以 `root/pom.xml` 在 `paths` 为空时也找得到），
  再算 Java 入口（`:492-500`），然后才 `run_configurations_from_entrypoints`（`:501-505`）；
  再在 `detected_configurations`（`:902-930`）里调 `detectors::detect_all(root, maven_root)`
  （`detectors/mod.rs:242-256` → `scan::scan(root)` 有界遍历 `scan.rs:140-182`，
  深度 ≤ 6、目录数 ≤ 4000、`PRUNED` 剪枝 `:17-52`）。
- 因此**即使 `paths` 为空**，只要 `root/pom.xml` 可读，Maven 探测器就能工作。
- 探测器全集（`detectors/mod.rs:227-237`）与它们产出的 provider（**逐个核实过**）：

| 探测器 | provider | 依据文件 | 证据 |
| --- | --- | --- | --- |
| `npm::detect` | `npm.script` | `package.json` | `detectors/npm.rs:115,118,121` |
| `compose::detect` | `compose.service` / `compose.stack` | `docker-compose*.yml/yaml`、`compose.yml/yaml` | `detectors/compose.rs:13-14,39,59` |
| `procfile::detect` | `procfile.process` | `Procfile` | `detectors/procfile.rs:37` |
| `python::detect` | `python.script` / `python.django` / `python.flask` / `python.uvicorn` | `*.py`、`manage.py` 等 | `detectors/python.rs:45,54,93,111,157` |
| `cargo::detect` | `cargo.binary` | `Cargo.toml` | `detectors/cargo.rs:43` |
| `go::detect` | `go.main` / `go.command` | `go.mod` | `detectors/go.rs:14,29` |
| `gradle::detect` | `gradle.application` / `gradle.service` | `build.gradle(.kts)`、`gradlew` | `detectors/gradle.rs:36,48` |
| `make::detect` | `make.target` | `Makefile` / `makefile` / `GNUmakefile` | `detectors/make.rs:6,27,29` |
| `shell::detect_just` | `just.recipe` | `justfile` / `Justfile` / `.justfile` | `detectors/shell.rs:6,42,44` |
| `maven::detect`（**单独一路**） | `spring-boot.maven` / `quarkus.maven` / `micronaut.maven` | 声明式 reactor 里模块的 pom 应用的插件 | `detectors/maven.rs:16-20,31-48` |

**这一格就是"Core 能不能给出『可运行的 Maven 目标 / Spring Boot 应用』"的答案：能。**
`spring-boot-maven-plugin` → provider `spring-boot.maven`（`detectors/maven.rs:17`），
goal 是 `spring-boot:run`（`configuration.rs:2005`），参数属性名
`spring-boot.run.jvmArguments` / `.arguments` / `.main-class`（`:2006-2008`），
启动计划最终由 `maven.launchPlan` 生成（契约 `:1475-1494`，`-pl <module> -am`）。

**Java main class 不在这里**：`java.main` 只从 JDT 的 `lsp.request{operation:"javaEntrypoints"}`
来（契约 `:1343-1346`、`configuration.rs:492-495` 的 `java_entrypoints`），
Core 明确**不**从源码语法推断入口（契约 `:1360-1361`）。
没有 JDT 的那一口时，`generate` 会**沿用上一次生成的 Java 入口**（`:494` 的
`previous_java_entrypoints` → `:669-707` 读 `.lithe/run/generated.json`），
冷启动也不会把列表清空。**gpui 侧目前没有任何 `javaEntrypoints` 通路**
（`gpui/crates/java/src/service.rs` 只用 `definition` / `completion` / `codeAction` /
`virtualDocument` 一族，见 `:227/:298/:405` 与 `session.rs:332-336`）。

#### 3.3.2 其余可用的运行事实（本页可选，非必需）

| 命令 | 请求 / 响应要点 | 契约 | 本页是否需要 |
| --- | --- | --- | --- |
| `runConfig.inspect` | `{root, checkFingerprint?, localDocument?}` → `{status: "ready"\|"missing", generated, toolchainRequirements, localToolchains, toolchain, diagnostics, paths:{generated,configurations,local}}`（`configuration.rs:26-34, 448-456`）；**只读** | `:162`, `:1531-1546` | 可选：显示"文档陈旧"那类诊断 |
| `runConfig.resolve` | `{root, toolchainCandidates?, localDocument?}` → 生效配置 + 来源 + 队默认 + 结构化诊断 + 生效全局 toolchain + `localToolchains`（`configuration.rs:56-63`）；优先级 `local.json > configurations.json > generated.json` | `:164`, `:1548-1594` | 需要**磁盘上已有** `generated.json`（只有 `localDocument` 能在内存里覆写） |
| `runConfig.createLaunchPlan` | `{root, configurationId, currentFile?, classPath?, javaLaunch?, debugPort?, localDocument?, mavenContext?}` → toolchain 引用 + 参数数组 + 相对 cwd + 指纹 | `:168`, `:1658-1690` | ❌ 本页不做（这是"运行"那一步） |
| `runConfig.updateOptions` | `{root, scope, configurationId, workingDirectory, …}` → `{document}`（**纯变换，不写文件**；`:1612-1628`） | `:165` | 见 §3.4 的「保存用户选择」 |
| `runConfig.saveEditorChanges` | 同上 + `toolchain` → `{localDocument, projectDocument\|null, toolchainDocument\|null}`（`:1630-1646`） | `:166` | 同上 |
| `maven.scan` | `{root, paths?}` → reactor / 递归 modules / profiles / sourceRoots / hasWrapper（`:1450-1473`） | `:107` | 已经被 Maven 右栏用了（`gpui/crates/workbench/src/maven.rs:93`），本页不需要重复 |
| `maven.launchPlan` | `{root, context, module?, goals[]}` → toolchain 引用 + 参数数组 + 相对 reactor cwd + 指纹 | `:108`, `:1475-1494` | ❌ 间接：`createLaunchPlan` 会调它 |

**`.lithe` 文档层位置（事实）**：`configuration.rs:455` 的响应里直接写着
`{"generated": ".lithe/run/generated.json", "configurations": ".lithe/run/configurations.json",
"local": ".lithe/run/local.json"}`；工具链选择落在 `.lithe/toolchains/local.json`（契约 `:1635-1636`）。

### 3.4 最小有真值版本的规格

#### 界面结构（推断，按真源列表态对齐）

```text
页面标题：运行配置（settings.run.title，dialog.rs:285 已接）
正文：
  ├─ 说明        settings.run.description（lithe.zh-CN.yml:3598）
  ├─ 状态行      S1 风格的失败/空态文案（见下"不做部分的文案"）
  ├─ 「重新识别」  run.identifyAgain（:3118），点击 = 重跑下面的数据流
  └─ 列表        每条一行：name（主色/正文） + provider 或执行类别（弱化）
                 + source（弱化，`run.source.*` 三键之一）
```

**列表里能显示什么**：`generated.configurations[]` 的
`name` / `provider` / `execution` / `category` / `command`(+`args`) / `cwd` / `source` / `disabled`。
**不要**显示 `javaHomePath` / `mavenExecutablePath` / `mavenSkipTests` 之类 ——
它们在 `generated` 层不存在（那是 `local` / `project` 层与 `extensions` 里的东西）。

#### 数据来源（推断，推荐方案）

```text
SettingsDialog（Lsp/Run 页）— 打开这一页时懒跑一次，不在构造期、不在 render 里同步跑
  └─ cx.spawn_in(..) → background_spawn(async move {
        // ① 可见文件清单（同一个 Core 命令，explorer 已在用）
        let files = core_json("workspace.snapshot", json!({"root": root}))?.data.files;
        // ② 一次生成：Core 自己走文件树 + 用 paths 推 Maven 根与 Java 生态
        let generated = core_json("runConfig.generate", json!({
            "root": root, "paths": files, "modulePaths": []
        }))?;
        Ok(generated.data.generated.configurations)
     })
  └─ 回前台：设置侧的 generation 代次校验（照 dialog.rs:1008-1015 的 Git 页写法）
```

要点与理由：

1. **命令用 `runConfig.generate`，不用 `resolve`**：`resolve` 读的是磁盘上的
   `.lithe/run/*.json`，而 gpui 侧没有任何东西写过它们；`generate` 直接返回配置数组，
   **不需要先做文件写入适配器**。（这是"最小"的关键。Windows 之所以走 generate→resolve，
   是因为它还要合并用户保存过的 `local.json` / 团队 `configurations.json`。）
2. **`core_json` 已经在 settings crate 的依赖里**：`gpui/crates/settings/Cargo.toml:9`
   依赖 `lithe-gpui-shared`，而 `core_json` 就在 `gpui/crates/shared/src/core_client.rs:223`。
   默认超时 120s（`:37`），足够（Windows 侧给 `generate` 60s，`run-core-api.ts:71`）。
   **所以这一页不需要宿主钩子**（与 Git 页不同：`git.*` 住在 `lithe-gpui-git`，
   `settings` 不许依赖它，见 `gpui/crates/settings/src/identity.rs:1-19`；
   `runConfig.*` 是纯 Core 命令，没有归属 crate）。
   **推断**：如果维护者更倾向"外壳注册钩子"的统一形状，也可以照 Git 页做，但那是多一层间接。
3. **别在 render 里跑**：`workspace.snapshot` 与 `runConfig.generate` 都是同步读盘/遍历
   （`runConfig.generate` 会 `scan(root)`，深度 6）。gpui 侧已有两条明确教训：
   `gpui/crates/workbench/src/workspace.rs:1455-1462`（Maven/Spring 懒扫的理由）与
   `dialog.rs:602-604`（Git 页"不在构造期读"）。
4. **`paths` 传全部可见文件而不是只传 `*.java`**：`maven_root`（`maven.rs:832-850`）
   只看路径的**祖先链**，传全量文件能让嵌套 reactor 的 pom 被发现；
   而 `inferred_maven_module_paths`（`configuration.rs:932-958`）自己会过滤 `.java`。
   `workspace.snapshot` 的 `files` 在 explorer 侧被 `RENDER_LIMIT = 5000` 截断
   （`gpui/crates/explorer/src/model.rs:35,258`）—— **推断**：这一页应自己设一个上限并在
   超过时打诊断，不要静默用一份被截断的清单（截断本身不影响 Maven 根的选择，
   只影响极深目录的 Java 生态判定）。

#### 持久化位置

- **v1 不持久化任何东西**（推荐）。这一页只读 `runConfig.generate` 的结果，
  不写设置文件、不写 `.lithe`。
- 若要做「记住选中哪条配置」：那是 `runConfig.updateOptions`
  （`scope: "local"`，返回 `document` 字符串）+ 一个把
  `.lithe/run/local.json` 原子写下去的**新适配器**（gpui 侧目前没有；
  可复用 `gpui/crates/settings/src/persistence.rs:216-229` 的"临时文件 + rename"写法）。
  **推断**：这应排在"运行"落地之后，否则是"存了没人读"。

#### 不做部分的文案（必须有，否则等于骗人）

- **不画运行按钮**（连置灰也不画）—— 判据与 `dialog.rs:189-191`、`07-settings-ui.md` §7.3-D 一致。
- 需要**一条新的 gpui-only 文案键**，如实说明"列表是真的、运行还没接"，例如
  `lithe.settings.gpui.runNotWired` = 「上面列出的启动目标由 Core 从项目文件识别得到；
  点击启动尚未接入（没有运行面板与进程宿主）。」
  新增一个键要同时改 **4 处**（同族既有键的既有形状）：
  1. `gpui/crates/shared/locales/lithe.zh-CN.yml`（zh 值）；
  2. `gpui/crates/shared/locales/lithe.en.yml`（en 值，**不能等于中文**，
     `i18n.rs:701-718` 会断言）；
  3. `gpui/crates/shared/src/i18n.rs` 的接线表（`:158-680` 的 `WIRED`，`lithe-tools/extract-locale.mjs`
     的 `GPUI_ONLY_KEYS` 需要一条理由，照 `:158` LSP 那条的写法）；
  4. `gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS`（否则下次抽取会把这个键删掉）。
- 空态（`generate` 返回 0 条配置）应复用真源已有的 `run.noRunnableActionsFound`
  （`lithe.zh-CN.yml:3270`）或 `run.noMatchingActions`（`:3268`）——**这两个键都存在**，
  但**推断**：它们原本属于"扫描项目操作"那一族（真源 `run.scanningProjectActions` / `run.filterActions`），
  用之前要确认语义合适；更稳的是新增一条同族键并如实写"这是 Maven/npm/Compose 等
  构建文件里没找到可运行项"。

### 3.5 必须先补什么

**如果只要"只读列表版"：什么都不用先补。** 依赖项全都在：

| 依赖 | 状态 | 证据 |
| --- | --- | --- |
| Core `runConfig.generate` | ✅ 已实现并有集成测试 | `rust/lithe-core/src/execution/configuration.rs:460`；测试 `rust/lithe-core/src/tests/run_configuration.rs:58-69` 等 100+ 处 |
| Core `workspace.snapshot` | ✅ 已被 explorer 消费 | `gpui/crates/explorer/src/model.rs:236-261` |
| gpui `core_json` | ✅ | `gpui/crates/shared/src/core_client.rs:223`；调用样例 `gpui/crates/workbench/src/maven.rs:94` |
| 设置页可起后台任务 | ✅ | `gpui/crates/settings/src/identity.rs:52-56` 的说明 + `dialog.rs:1004-1022` 的 Git 页实现 |
| 文案键 | ✅ 真源键全在 yml 里（grep 行号见 §3.1 表） | `gpui/crates/shared/locales/lithe.zh-CN.yml` |

**如果要"能保存用户选择"或"能运行"，必须先补**（推断，按依赖顺序）：

1. **`javaEntrypoints` 通路**（可选但显著提升真值）：`gpui/crates/java/src/service.rs`
   加 `pub fn java_entrypoints(&self) -> Result<JavaEntrypoints, String>`
   （包 `lsp.request{operation:"javaEntrypoints"}`，`session.rs:325-370` 的 `request` 已有），
   再作为 `runConfig.generate` 的 `javaEntrypoints` 传进去。**没有它，`java.main` 类配置永远不会出现**
   （只会出现 Maven 框架服务 / npm / compose 等）。**禁止**在 gpui 里自己扫 `main` 方法 ——
   `scripts/verify-java-semantic-ownership.mjs:11-18` 会把本地 main 扫描器判为违规
   （`gpui/research/java-spring-maven-inventory.md:611-613`）。
2. **`.lithe/run/*.json` 的写适配器**（保存选择 / 让 `resolve` 有东西可读）：
   契约 `:1614-1615`「The platform adapter selects the target project or local file
   and performs the atomic write」→ gpui 侧要新增一个写入器（新 crate 或 `workbench` 的一个模块）。
3. **运行子系统**（真正做到"运行"）：进程宿主 + stdout/stderr 流 + 输出面板 + 停止/孤儿回收
   + `runConfig.createLaunchPlan` 的消费 + `mavenContext` 组装（`maven.scan` 已经有数据）。
   里程碑划分见 `gpui/research/java-spring-maven-inventory.md:602-620` 的 M4 与
   `gpui/research/windows/05-terminal-run-debug.md:210-211`。

---

## 4. 实现顺序与写域

### 4.1 先做哪页

**推断：先做「运行配置」页，再做「LSP」页。** 三条理由：

1. **运行配置页不需要动 `java/**`、不需要动 `editor/**`**：它只动 `gpui/crates/settings/**`
   一个 crate（数据层 + 一页 UI），爆炸半径最小、可独立验证。
2. **LSP 页会与「项目 · JDK 与 Maven」页抢写域**：工作区里**已经有**一个正在落地的
   「项目 · JDK 与 Maven」页（`git status` 显示 `?? gpui/crates/settings/src/project.rs`；
   `gpui/crates/settings/src/lib.rs:47,63-66` 已 `pub mod project` 并 re-export
   `discover` / `EffectiveToolchain` / `Overrides` 一族；写作期间
   `gpui/crates/settings/src/schema.rs:232-241` 已经出现
   `java_home_path` / `maven_executable_path` / `maven_java_home_path` 三个新键，
   `persistence.rs` / `store.rs` / 两个 yml / `i18n.rs` / `extract-locale.mjs` 也都在改）。
   它要改的 `schema.rs` + `persistence.rs` + `store.rs` + `dialog.rs` ——
   **正是 LSP 页的 `autoCompletion` 要改的四个文件**。两页并行会撞车；
3. LSP 页的 `autoCompletion` 是"加了就要保证生效"的开关（跨 `settings`/`editor`/`workbench`
   三个 crate + 三处 schema/persistence 守卫），需要一个安静的工作区。

### 4.2 动哪些文件

**运行配置页（最小只读版，推断的最小集合）**

| 文件 | 改动 |
| --- | --- |
| `gpui/crates/settings/src/run.rs`（**新建**） | 数据层：`core_json("workspace.snapshot")` → `core_json("runConfig.generate")` → 纯函数解析成 `RunProjectView { configurations: Vec<RunConfigurationView> }`（照 `gpui/crates/workbench/src/maven.rs:120-229` 的解析写法与 `:231-342` 的单测写法）；诊断行 `S1_SETTINGS_RUN ...`（照 `gpui/crates/settings/src/project.rs:41-45` 的 `PROJECT_DIAGNOSTIC_TAG` 形状） |
| `gpui/crates/settings/src/dialog.rs` | ① 新增 `run_page()`（照 `:1195-1460` 的 `git_page` 的"钩子/数据缺失就退回 `empty_page`"形状，但**不**需要钩子）；② `Category::IMPLEMENTED`（`:248-253`）加 `Category::Run`；③ `:302-313` 的 `prerequisite_key()` 把 `Self::Run` 改成 `None`；④ `:642-647` 的 `content()` 分支加 `Category::Run => self.run_page(...)`；⑤ 页状态（`generation` / `busy` / `error` / `configurations`）照 `GitPageState`（`:341-372`）加一个 `RunPageState`；⑥ 单测：`every_category_is_implemented_or_declares_a_prerequisite`（`:1848-1863`）会自动覆盖一致性，另加一条"没有数据源时不画假控件"的断言 |
| `gpui/crates/settings/src/lib.rs` | `pub mod run;` + re-export（`:43-52, 63-66` 的既有形状） |
| `gpui/crates/shared/locales/lithe.zh-CN.yml` / `lithe.en.yml` | 新增 `lithe.settings.gpui.runNotWired`（+ 可能的"没找到可运行项"键） |
| `gpui/crates/shared/src/i18n.rs` | 把新键补进 `:158` 起的 `WIRED` 表 |
| `gpui/tools/extract-locale.mjs` | 在 `GPUI_ONLY_KEYS` 里登记新键 + 理由（照 `:158` LSP 那条的写法） |

**LSP 页（只做 `autoCompletion` + 只读"已检测语言服务器"）**

| 文件 | 改动 |
| --- | --- |
| `gpui/crates/settings/src/schema.rs` | `Settings` 加 `auto_completion: bool`（`#[serde(rename = "autoCompletion")]`，默认 `true`）+ `Default`（`:207-223`）+ `defaults_match_the_windows_truth` 测试（`:348-362`） |
| `gpui/crates/settings/src/persistence.rs` | `settings_from_object`（`:144-198`）加一行 `take(...)`；`every_key_survives_a_round_trip`（`:440`）要能覆盖新字段（该测试用"每个字段都不是默认值"的设置跑往返，`auto_completion` 的"非默认值"是 `false`） |
| `gpui/crates/settings/src/store.rs` | `set_auto_completion`（照 `set_show_status_bar`，`:314`） |
| `gpui/crates/editor/src/completion.rs` | `JavaCompletionProvider`（`:92-99`）加字段；`new`（`:103-112`）加参数；`is_completion_trigger`（`:225-227`）读它 |
| `gpui/crates/editor/src/editor_view.rs` | `:630-631` 与 `:409-412` 两处传当前值；新增 `pub fn set_auto_completion(&mut self, enabled: bool, cx: &mut Context<Self>)`（模板 `:345-379`） |
| `gpui/crates/workbench/src/workspace.rs` | `:882-884` 启动喂一次、`:891-905` 订阅里转发一次 |
| `gpui/crates/settings/src/dialog.rs` | 新增 `lsp_page()` + `IMPLEMENTED` + `prerequisite_key` + `content()` 四处（同运行页） |
| `gpui/crates/java/src/service.rs` | **仅当**要做"已检测语言服务器"列表：加只读访问器（`installation` 字段在 `:193`，赋值在 `:739-742`） |
| `gpui/crates/settings/src/lsp.rs`（新建，**仅当**走宿主钩子） | 照 `identity.rs:275-342` 的 `Host` / `page` / `set_*_host` 三件套 |
| locale 四处（同上） | 若新增"检测到的 JDTLS/JDK"文案键 |

**⚠️ 明确的禁止项**：LSP 页**不要**新增 `parameterHints` / `semanticTokens` / `jdtlsJavaHomePath`
三个 schema 键 —— 那正是本仓库反复禁止的"存了没用 / 画了不生效"
（`gpui/crates/settings/src/schema.rs:20-25`、`gpui/crates/settings/src/dialog.rs:189-191`、
`gpui/research/windows/07-settings-ui.md` §7.3-D）。

---

## 5. 未确认项 + 下一步确认办法

| # | 未确认点 | 为什么重要 | 怎么确认 |
| --- | --- | --- | --- |
| 1 | `runConfig.generate` 在**本仓库根**（真实的多语言仓库）上返回哪些配置、耗时多少 | 决定列表形态与是否需要分页/上限；也决定空态文案 | 起一次 Lithe（**另一个代理负责**）或写一个选定式 Rust 测试，按 `rust/lithe-core/src/tests/run_configuration.rs:152-160` 的既有形状，对 `root = 仓库根` 调 `runConfig.generate { paths: [] }`，打印 `entryCount` / 各 `provider` / 毫秒数。**不要**在本任务里跑 cargo。 |
| 2 | `workspace.snapshot` 的 5000 条截断是否会影响本仓库的 Maven 根选择 | 影响 `paths` 要不要分块或改用别的清单 | 同上：打印 `files.len()` 与是否 `>= 5000`（`RENDER_LIMIT`，`gpui/crates/explorer/src/model.rs:35`） |
| 3 | gpui 的 Java 服务有没有可能已经具备 `javaEntrypoints` 的隐藏通路 | 决定 `java.main` 配置能不能出现在列表里 | 全文搜 `java_entrypoints` / `javaEntrypoints` / `resolveMainClass` 在 `gpui/crates/**`（本次已搜，**零命中**）；若确认没有，则需要新增（§3.5 第 1 项） |
| 4 | 「已检测语言服务器」列表是否值得做（真源只有一句静态文本） | 决定 LSP 页是"1 个开关"还是"1 个开关 + 1 段事实" | 这是**产品取舍**，需要维护者拍板。可先做只有 `autoCompletion` 的版本，把列表留成后续增量。 |
| 5 | 「项目 · JDK 与 Maven」页落地后，其 `schema.rs` 是否已经占用了工具链路径键名 | 决定运行配置页能不能直接读 `Overrides` 显示"生效值" | **写作期间已观察到**：`gpui/crates/settings/src/schema.rs:232-241` 新增了 `java_home_path` / `maven_executable_path` / `maven_java_home_path`（`Overrides` 在 `project.rs:290-300`）。落地后确认键名与 `project::discover(overrides)` 的入参形状是否稳定，再把运行页的"生效值"接上。 |
| 6 | 真源 `run.noRunnableActionsFound` 的语义适配性 | 决定空态是复用还是新增键 | 读真源 `features/run/**` 里这两个键的渲染点（本次未读）；或直接新增同族键更稳 |
| 7 | `autoCompletion` 关掉后，"轻量兜底"（`completion.rs:210-217` 的 `builtin_completions`）也一起关是否可接受 | 影响描述文案与是否要拆两个开关（真源只有一个） | **推断**：可接受（真源只有一个开关，且描述句说"补全建议"），但描述句要改成"不自动弹出补全菜单"以免用户以为兜底还在。需要维护者确认措辞。 |

---

## 附：本文引用的关键事实速查

```text
LSP 页真源            windows/tauri/src/features/settings/components/macos-settings-panels.tsx:398-438
LSP 分类表项          windows/tauri/src/features/settings/components/settings-dialog.tsx:42
三键默认值            windows/tauri/src/features/settings/config/default-settings.ts:72,153,154
退役的 JDTLS JDK 键   windows/tauri/src/features/settings/lib/settings-normalization.ts:550
                      windows/tauri/src/features/settings/lib/settings-normalization.test.ts:26-33
补全装载点            gpui/crates/editor/src/editor_view.rs:630-631,662（+ 补装 :396-426）
补全触发判据          gpui/crates/editor/src/completion.rs:225-227,236-241
上游补全钩子          gpui-base-0.6.6/src/input/editor/lsp/completions.rs:122-146
上游 provider 清单    gpui-base-0.6.6/src/input/editor/lsp/mod.rs:39-69（无签名帮助）
上游语义高亮 trait    gpui-base-0.6.6/src/input/editor/lsp/semantic_tokens.rs:36-55；挂载 :mod.rs:51
JDTLS 发现顺序        gpui/crates/java/src/jdtls.rs:330-344,350-368,371-403
JDK 发现顺序          gpui/crates/java/src/jdtls.rs:440-495,498-563
两处调用点            gpui/crates/java/src/service.rs:196-204,659-670,739-742
设置 schema（gpui）   gpui/crates/settings/src/schema.rs:106-223
逐键容错表            gpui/crates/settings/src/persistence.rs:144-198
空态与分类表          gpui/crates/settings/src/dialog.rs:194-333,632-650,1487-1530,1848-1863
git 页宿主钩子模板    gpui/crates/settings/src/identity.rs:1-73,275-342
                      gpui/crates/workbench/src/workspace.rs:764-820
外壳转发模板          gpui/crates/workbench/src/workspace.rs:868-906
Run 页真源            windows/tauri/src/features/settings/components/run-configuration-settings.tsx:11-114
Run 编辑器真源        windows/tauri/src/features/run/components/run-configuration-editor.tsx:210-433
Run host/core API     windows/tauri/src/features/run/api/run-host-api.ts:10-134
                      windows/tauri/src/features/run/api/run-core-api.ts:43-127
Run 组装              windows/tauri/src/features/run/services/resolve-run-project.ts:39-96
gpui Run 占位         gpui/crates/workbench/src/workspace.rs:264-305,1856-1863
gpui Run 菜单为空     gpui/crates/workbench/src/menu_bar.rs:340-344
gpui 命令面板无 Run   gpui/crates/workbench/src/command_palette.rs:187-210
Core 生成             rust/lithe-core/src/execution/configuration.rs:39-51,460,654-659
Core 探测器全表       rust/lithe-core/src/execution/detectors/mod.rs:227-237
Maven 框架探测器      rust/lithe-core/src/execution/detectors/maven.rs:16-20,31-48
扫描边界              rust/lithe-core/src/execution/detectors/scan.rs:10-52,140-182
契约                  shared/contracts/rust-core-api.md:107-112,162-168,1450-1494,1531-1690
Core 数据模型         shared/contracts/run-configuration-v2.schema.json:54-94
gpui Core 客户端      gpui/crates/shared/src/core_client.rs:100-129,223
```
