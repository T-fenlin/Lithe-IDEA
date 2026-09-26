# 编辑器语法高亮：0.6.6 的真实机制与我们的缺口

> 目的：回答"为什么打开文件后没有着色，要让它亮起来需要动什么"。
> 全部结论只依据**本机 cargo registry 里已发布的 0.6.6 源码**与仓库现有代码，逐条给 `文件:行号`。
> 本轮**没有**改代码、**没有**动 `Cargo.toml`、**没有**跑 `cargo`（因此所有"构建代价"都是推断，见 §6）。

## 0. 证据基准

| 简写 | 路径 |
| --- | --- |
| `KIT` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-0.6.6\` |
| `CMP` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-component-0.6.6\` |
| `BASE` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-base-0.6.6\` |
| `REPO` | `D:\developmentProjects\rust\Lithe-IDEA\gpui\` |

---

## 1. 0.6.6 的高亮机制

### 1.1 一句话结论

高亮 = **「`EditorState::language(名字)`」+「该名字在 `LanguageRegistry` 里注册了 grammar」+「该 grammar 的 Cargo feature 被打开」**。
三者缺一个，编辑器就是**无着色但完全可用**的纯文本（不会报错、不会 panic）。

### 1.2 设置语言的方法：`.language(..)`（builder）与 `set_highlighter(..)`（运行时）

| 方法 | 定义处 | 语义 |
| --- | --- | --- |
| `EditorState::new(window, cx)` | `BASE\src\input\base\state.rs:9230` | 建 `LayoutMode::CodeEditor`，**默认没有语言**（doc 注释 `:9224-9225`："without one the text is shown unhighlighted"） |
| `.language(impl Into<SharedString>)` | `BASE\src\input\base\state.rs:9240`（doc `:9237-9239`） | 把语言名写进 `EditorLanguage`，并把缓存的 highlighter 置空（`:9247-9248`） |
| `.set_highlighter(name, cx)` | `BASE\src\input\base\state.rs:772`（doc `:771`："Set highlighter language for `LayoutMode::CodeEditor` mode"） | 运行期换语言；同样清缓存，然后 `refresh(cx)` |
| `.language_name() -> SharedString` | `BASE\src\input\base\state.rs:9254` | 读当前语言名（可用于断言/诊断） |
| `.refresh(cx)` | `BASE\src\input\base\state.rs:1275` | 置 `_pending_update = true`，让**下一次 render** 重跑高亮 |

文档里的说明：

* `REPO\docs\gpui-kit\0.6.6\zh-CN\component\editor.md:92`
  > 使用 `.language()` 指定语法高亮语言。应用需要启用对应的 Cargo feature，例如 `tree-sitter-rust` 或 `tree-sitter-markdown`；也可以使用 `tree-sitter-languages` 包含全部内置语法。
* 同文件 `:76-90` 的"基础用法"就是 `.language("rust")`（`:80`）。
* `REPO\docs\gpui-kit\0.6.6\zh-CN\base\primitives\editor.md:12-18`：Base 只读语言配置、**不加载解析器**；Component 在初始化时安装 `LanguageProvider`。

### 1.3 `LanguageConfig` / `set_language_config` / `LanguageProvider` 各自做什么

三个东西**都与"颜色"无关**，别混：

| 名字 | 定义处 | 职责 | 对高亮有没有用 |
| --- | --- | --- | --- |
| `LanguageConfig`（编辑规则） | `BASE\src\input\editor\language_config.rs:88-117`（默认值：3 组括号 + 5 组自动配对 + `auto_close_before`） | 自动补全括号、Enter 缩进规则、成对 Backspace | **没有**。它不解析、不上色 |
| `set_language_config(lang, cfg, cx)` | `BASE\src\input\editor\language.rs:79-87` | 覆盖**某个语言名**的编辑规则（应用级全局） | **没有** |
| `LanguageProvider`（trait） | `BASE\src\input\editor\language.rs:11-30` | 提供三件事：语言名归一化 `language_name`（`:14`）、默认编辑规则 `config`（`:20`）、**语法上下文提供者** `syntax_context_provider`（`:27`） | 间接：只有它决定 `auto_close` 的"我在字符串/注释里吗"。**它不是上色器** |
| `set_language_provider(provider, cx)` | `BASE\src\input\editor\language.rs:71-73` | Base 用法：自己装一个语言服务 | 间接（同上） |

Component 侧的 `LanguageProvider` 实现在 `CMP\src\input\language_config.rs:14-29`：
`language_name` 走 `LanguageRegistry::singleton().editing_language_name(name)`（`:16`），
`config` 只对 `["text", "json", "python"]` 有特例（`:34-38`、`:47-72`），其余一律 `LanguageConfig::default()`（`:70`）。

**结论：我们不需要 `set_language_config`，默认规则已够。** 理由：
1. 我们目前没有任何自定义缩进/配对需求；`LanguageConfig::default()` 已经给了 `()`、`[]`、`{}` + `"` + `'` 的自动配对与 `auto_close_before`（`BASE\src\input\editor\language_config.rs:88-117`）。
2. 我们连 `text` / `json` / `python` 这三个特例都不需要绕开。
3. 真要做语言级缩进，才需要它；那是另一件事，与"亮起来"无关。

### 1.4 上色到底发生在哪条链上（这才是"没亮"的关键）

```
EditorState（LayoutMode::CodeEditor { language, highlighter, highlighter_factory }）   BASE\src\input\base\mode.rs:76-91
      │  code_editor() 里 highlighter_factory = None                                  BASE\src\input\base\mode.rs:82
      ▼
CMP 的 Editor 是 Input 的包装                                                          CMP\src\input\editor.rs:134-160（:136 Input::from_state）
      ▼
Input::render 里 ensure_highlighter_factory(tree-sitter 那个工厂)                      CMP\src\input\input.rs:495
      ▼
render 时若 _pending_update：mode.update_highlighter(force:false)                      BASE\src\input\base\state.rs:4156-4172
      ▼
工厂按**语言名**要 highlighter：factory(&language.name())                              BASE\src\input\base\mode.rs:290-293
      ▼
LanguageRegistry.has_parser(name)？没有 → 返回 None → 静默不上色                       CMP\src\highlighter\input_adapter.rs:22-28 + BASE\src\input\base\mode.rs:296-298
      ▼
paint 时 highlighter.styles(.., theme.highlight_theme)                                BASE\src\input\base\element.rs:1559-1562
```

两个"静默"点值得记住：

* 语言名没注册 → 工厂返回 `None` → 只是**不着色**，没有日志（`BASE\src\input\base\mode.rs:296-298`）。
* 名字注册了但 grammar 不可用 → `SyntaxHighlighter::new` 会 `tracing::warn!` 后回落到 `text`（`CMP\src\highlighter\highlighter.rs:334-345`）。

### 1.5 颜色从哪来（我们不用改，但要知道"亮"长什么样）

* `Input::render` 把 `cx.theme().highlight_theme` 塞进 `InputEditorStyle.highlight_styles`（`CMP\src\input\input.rs:510`），paint 时作为 `HighlightStyleResolver`（`BASE\src\input\base\element.rs:1559-1562`）。
* 语义名 → 颜色：`SyntaxColors::style`，41 个名字（`CMP\src\highlighter\registry.rs:15-57`、`:241-306`）。
* **我们的主题 JSON 里没有 `highlight` 段**（`REPO\themes\README.md:321-333` 明确写了"本次故意不写"）。
  `Theme::apply_config` 只在 JSON 带 `highlight` 时才替换 `highlight_theme`（`CMP\src\theme\schema.rs:1066-1073`）。

  > ⚠️ **2026-09 第二批主题改变了这条的适用范围**（本节其余内容与读数**不改**，它们描述的是当时的状态）：
  > `gpui/themes/` 里新加的 `gruvbox.json` / `jetbrains.json` / `nord.json` / `one.json` / `vscode.json`
  > **都带 `highlight` 段**（语法色映射表见 `gpui/themes/README.md` §7）；仍然不带的是第一批的
  > `lithe-{dark,light}.json`（理由见该文档 §6.1，行号引用已改为章节号）。
  > 另外 `crates/settings/src/theme.rs` 的 `apply_theme_by_name` 现在会在"目标主题没有 `highlight`"时
  > 按明暗把内置那一份 `highlight_theme` 复位（`stamp_builtin_highlight_if_absent`），§6.5 那条
  > "暗色下正文会不会是黑字"的坑因此不再是默认路径。

  ⚠️ **这里有一条需要先验证的坑（见 §6.5）**：
  * `Theme::default()` 的 `highlight_theme` 是 **`HighlightTheme::default_light()`**（`CMP\src\theme\mod.rs:679`），
    而它带 `editor.foreground = "#000000"`（`CMP\src\theme\default-theme.json:113`）。
  * `apply_theme_by_name` 的顺序是 **先 `Theme::change(mode)`（这一步把内置明/暗亮色刷对）
    再 `apply_config(&lithe主题)`**（`REPO\crates\settings\src\theme.rs:93-95`）；
    而 Lithe 的主题文件没有 `highlight` 段 → 这次 `apply_config` **不会**再改 `highlight_theme`
    → 留下来的是上一步刷对的**内置同明暗**配色（明色主题拿 `default-theme.json:112-213`，暗色拿 `:312` 起的 dark 段）。
  * 但**如果在 `Theme::change` 之前就有任何一次 Lithe 的 `apply_config`**，`highlight_theme` 就会是
    `default_light()`（黑字），暗色主题下正文会变成"近黑字 + 暗底"。

  → 结论：**"一开 feature 就会亮"预期成立**（拿到的是内置同明暗配色），
  但开工前值得花一分钟确认暗色下正文不是黑字；若发现是黑的，修法与 `README:332-333` 的说明一致：
  给 `REPO\themes\lithe-*.json` 补一个 `highlight` 段（Zed 兼容格式）即可。

---

## 2. grammar feature 清单（实际读的 `Cargo.toml`）

两份文件都读过，feature 名以**实际文本**为准：

* `KIT\Cargo.toml`（`gpui-kit` 0.6.6）
* `CMP\Cargo.toml`（`gpui-component` 0.6.6）

### 2.1 先排除两个误解

1. `KIT\Cargo.toml:46-49` 的 `default = ["component", "assets"]` —— **默认特性里没有任何 `tree-sitter-*`**。
2. `KIT\Cargo.toml:62-65` 的 `tree-sitter`（**没有后缀**）是一个**基础**特性，不是"全部语言"：
   它等于 `CMP\Cargo.toml:52-55` 的 `tree-sitter = ["dep:tree-sitter", "dep:tree-sitter-json"]`
   → **只带 tree-sitter 运行时 + JSON 语法**。
   "全部语言"叫 `tree-sitter-languages`（`KIT\Cargo.toml:138-141` → `CMP\Cargo.toml:128-164`）。

### 2.2 `tree-sitter-languages` 展开的 35 个语言特性

`CMP\Cargo.toml:128-164` 逐个列出（同样的 35 个名字在 `KIT\Cargo.toml:66-213` 一一转发）：

`astro`、`bash`、`c`、`cmake`、`cpp`、`csharp`、`css`、`diff`、`ejs`、`elixir`、`erb`、`go`、
`graphql`、`html`、`java`、`javascript`、`jsdoc`、`kotlin`、`lua`、`make`、`markdown`、`php`、`proto`、
`python`、`ruby`、`rust`、`scala`、`sql`、`svelte`、`swift`、`toml`、`tsx`、`typescript`、`yaml`、`zig`。

另外 `markdown-inline` **不在**这个列表里：它由 `CMP\Cargo.toml:177` 的
`tree-sitter-markdown-inline = ["tree-sitter-markdown"]` 提供（打开 markdown 就有了，
markdown 的注入语言里就包含它，`CMP\src\highlighter\languages.rs:264-265`）。
`gpui-kit` 侧的转发见 `KIT\Cargo.toml:154-157`。

### 2.3 关键：**Java 高亮要开哪个 feature**

* `gpui-kit` 侧：`KIT\Cargo.toml:122-125`
  ```toml
  tree-sitter-java = [
      "component",
      "gpui-component/tree-sitter-java",
  ]
  ```
* `gpui-component` 侧：`CMP\Cargo.toml:112-115` → `dep:tree-sitter-java`，依赖声明在 `CMP\Cargo.toml:435-437`（`tree-sitter-java = "0.23.5"`）。

→ **要 Java 高亮，在依赖 `gpui-kit` 的地方打开 `tree-sitter-java`（推荐）或 `tree-sitter-languages`（全量）。**

### 2.4 名字 ↔ 语言（`Language::from_name`，`CMP\src\highlighter\languages.rs:178-256`）

| 我们传的名字 | 命中的内置语言 | feature 门槛 |
| --- | --- | --- |
| `java` | Java | `tree-sitter-java`（`:210-211`） |
| `rust` / `rs` | Rust | `tree-sitter-rust`（`:234-235`） |
| `typescript` / `ts` | TypeScript | `tree-sitter-typescript`（`:248-249`） |
| `tsx` | Tsx | `tree-sitter-tsx`（`:246-247`） |
| `javascript` / `js` | JavaScript | `tree-sitter-javascript`（`:212-213`） |
| `json` / `jsonc` | Json | **无**（`:181`，`tree-sitter` 基础特性就够） |
| `markdown` / `md` / `mdx` | Markdown | `tree-sitter-markdown`（`:222-223`） |
| `yaml` / `yml` | Yaml | `tree-sitter-yaml`（`:250-251`） |
| `toml` | Toml | `tree-sitter-toml`（`:244-245`） |
| `kotlin` / `kt` / `kts` / `ktm` | Kotlin | `tree-sitter-kotlin`（`:216-217`） |
| `go` / `c` / `cpp` / `c++` / `csharp` / `cs` / `css` / `scss` / `html` / `php` / `python` / `py` / `ruby` / `rb` / `scala` / `sql` / `swift` / `bash` / `sh` / `lua` / `diff` / `proto` / `protobuf` / `make` / `makefile` / `elixir` / `ex` / `svelte` / `astro` / `zig` / `ejs` / `erb` / `graphql` / `jsdoc` | 各自语言 | 各自 feature（`:182-253`） |
| `text` / `plain` / `plaintext` | Plain（无 grammar） | 无（`:180`、`CMP\src\highlighter\languages.rs:373`） |

**Java 的 query 是从 grammar crate 里取的**：`tree_sitter_java::HIGHLIGHTS_QUERY`（`CMP\src\highlighter\languages.rs:457-463`），
不是 `include_str!` 的本地 `.scm` —— 也就是"打开 feature 就必然有高亮查询"。

⚠️ 副作用提醒：有 5 个语言即使打开 feature 也**不会**上色，因为它们的 highlights query 是空串：
`swift`（`:500`）、`csharp`（`:516`）、`graphql`（`:518`）、`proto`（`:520`）、`cmake`（`:529`）。
（Java / Rust / TypeScript / Tsx / JSON / YAML / TOML / Markdown 都不在此列。）

---

## 3. 我们现在的状况

### 3.1 各 `Cargo.toml` 里的 feature 配置：**一个 grammar 都没开**

`gpui-kit = "0.6"` 出现在 8 个 crate，全都**没有**写 `features = [...]`，也就是吃默认
（`default = ["component", "assets"]`，`KIT\Cargo.toml:46-49`）：

| 文件 | 行 |
| --- | --- |
| `REPO\crates\app\Cargo.toml` | `:30` `gpui-kit = "0.6"` |
| `REPO\crates\editor\Cargo.toml` | `:8` `gpui-kit = "0.6"` |
| `REPO\crates\explorer\Cargo.toml` | `:8` |
| `REPO\crates\git\Cargo.toml` | `:8` |
| `REPO\crates\settings\Cargo.toml` | `:8` |
| `REPO\crates\shared\Cargo.toml` | `:10` |
| `REPO\crates\terminal\Cargo.toml` | `:8` |
| `REPO\crates\workbench\Cargo.toml` | `:8` |

锁文件也印证了：`REPO\Cargo.lock:2404-2441` 的 `gpui-component` 依赖清单里**没有** `tree-sitter`、也没有任何 `tree-sitter-*`；
`REPO\Cargo.lock:7591` 起的那几个 `tree-sitter` / `tree-sitter-java` 条目是 `lithe-core`（`rust/` 那个 crate）的，
不是 gpui 这条链的（`REPO\Cargo.lock:3857-3858` 挂在 `lithe-core` 下）。

### 3.2 `editor_view.rs` 打开文件时**没有**调 `.language(...)`

全仓库 `*.rs` 里 `EditorState::new` 只出现两处，两处都只写了 `.searchable(true)`：

| 调用点 | 行 | 现状 |
| --- | --- | --- |
| 磁盘文件 buffer | `REPO\crates\editor\src\editor_view.rs:266` | `EditorState::new(window, cx).searchable(true)` —— **无 `.language`** |
| JDT 虚拟源码 buffer | `REPO\crates\editor\src\editor_view.rs:615` | 同上 —— **无 `.language`** |

`grep -rn "\.language(" REPO\crates` 零命中（`language` 只作为注释/`LanguageConfig` 讨论出现）。
→ **这就是没着色的直接原因**（`BASE\src\input\base\state.rs:9224-9225`：不给语言就按未高亮显示）。

### 3.3 `buffer.rs` 里**没有**按扩展名判语言的地方

`REPO\crates\editor\src\buffer.rs:40-64` 的 `icon_for_file` 是唯一一处"按扩展名分类"的逻辑，
它只产出 `IconName`（图标），与语言/grammar 无关。

顺带确认：`REPO\crates\explorer\src\model.rs:272` 也有一个同名的 `icon_for_file`（explorer 自己那份），
同样只做图标。**没有任何模块做"扩展名 → 语言名"。**

---

## 4. 要改什么（可执行结论）

### 4.1 在哪加 feature（版本口径 + 代价）

**推荐：只在 `REPO\crates\editor\Cargo.toml:8` 改一行。**

```toml
gpui-kit = { version = "0.6", features = ["tree-sitter-java"] }
```

理由与口径：

* **版本口径不变**：`0.6` 这个口径（`REPO\crates\app\Cargo.toml:19-29` 的注释）保持不动，只加 feature；
  Cargo 的 feature 是**并集**，所以 `gpui-kit` 在整棵构建图里就带上了 `tree-sitter-java`（`KIT\Cargo.toml:122-125`
  → `gpui-component/tree-sitter-java` → `CMP\Cargo.toml:112-115`），
  而 `tree-sitter-java` 又隐含 `tree-sitter`（`CMP\Cargo.toml:112-113`）→ **JSON 也顺带能亮**（`CMP\src\highlighter\languages.rs:181`）。
* **写在哪一个 crate 都一样**：Cargo 会做 feature 并集，8 个 crate 里写任意一处，链接进最终二进制的都是同一份 `gpui-component`。
  写在 `editor` 里最贴"谁需要解释得清"；写在 `app` 里也可以（那时 `editor` 侧看不出来源）。
* **不要写 `features = ["tree-sitter-languages"]`**（除非明确要 35 种语言）：它会拉进 35 个 grammar crate
  （`CMP\Cargo.toml:128-164`），是本机编译时间与产物体积的主要变量。
* **构建代价（推断，未实测）**：开 `tree-sitter-java` 会新增编译
  `tree-sitter`（0.26.13，`CMP\Cargo.toml:379-380`）+ `tree-sitter-java`（0.23.5，`CMP\Cargo.toml:435-437`）
  两个 crate；`tree-sitter` 本体的 C 代码很小，Java grammar 的 `parser.c` 是**几十万行的机器生成 C**，
  这是本次改动里唯一显著的一次性构建成本。产物体积增量主要是那份 `parser.c` 编译出的静态表（数百 KB 量级）。
  **我没有跑 cargo，所以这两条都是推断**（见 §6）。
* ⚠️ **锁文件会有一处可预期的变化**：`REPO\Cargo.lock:2404-2441` 的 `gpui-component` 依赖清单会多出 `tree-sitter` 等条目，
  并且可能新增一个 `tree-sitter` 0.26.x 的 `[[package]]` —— 因为现有锁里那颗是 `lithe-core` 要的 **0.25.10**
  （`REPO\Cargo.lock:7591` 起；`rust\lithe-core\Cargo.toml:26-27` 是 `tree-sitter = "0.25"`）。
  两个 semver 不同的大版本并存**在 Cargo 里是允许的**（C 依赖不是 Rust 类型，不构成"类型不兼容"），
  但这是本轮没能实测的构建细节之一。

### 4.2 语言名 ↔ 扩展名映射表（建议实现）

新增一个纯函数（放在 `REPO\crates\editor\src\buffer.rs`，与 `icon_for_file` 并列，便于单测），
输入 `name: &str`（文件名），输出 `Option<&'static str>`（**语言名**，不是图标）：

| 扩展名 | 语言名 | 需要的 feature | 备注 |
| --- | --- | --- | --- |
| `java` | `java` | `tree-sitter-java` | 截图里那种 Java 着色 |
| `rs` | `rust` | `tree-sitter-rust` | |
| `ts` / `mts` / `cts` | `typescript` | `tree-sitter-typescript` | |
| `tsx` | `tsx` | `tree-sitter-tsx` | |
| `js` / `jsx` / `mjs` / `cjs` | `javascript` | `tree-sitter-javascript` | |
| `json` / `jsonc` | `json` | **`tree-sitter` 基础特性**（`tree-sitter-java` 已隐含） | |
| `md` / `markdown` | `markdown` | `tree-sitter-markdown` | |
| `yml` / `yaml` | `yaml` | `tree-sitter-yaml` | |
| `toml` | `toml` | `tree-sitter-toml` | |
| `kt` / `kts` | `kotlin` | `tree-sitter-kotlin` | |
| `py` | `python` | `tree-sitter-python` | |
| `go` / `c` / `h` / `cc` / `cpp` / `hpp` / `cs` / `rb` / `php` / `scala` / `sql` / `css` / `html` / `sh` / `lua` | `go` / `c` / `c` / `cpp` / `cpp` / `cpp` / `csharp` / `ruby` / `php` / `scala` / `sql` / `css` / `html` / `bash` / `lua` | 各自 feature | 名字一律取 §2.4 表里**左侧那一列**，别自造（`c++`、`makefile`、`plaintext` 也能认，但没必要）。⚠️ `.h` 只有 `c` 可映射，上游**没有** `"h"` 这个名字（`CMP\src\highlighter\languages.rs:186-187` 只认 `"c"`） |
| `txt` / 其它 / 无扩展名 | **`None`**（不调 `.language`） | — | 见下 |

**未知扩展名与"开了 `.language` 但没开 feature"的行为完全一致**：不上色、不报错
（工厂返回 `None`，`BASE\src\input\base\mode.rs:296-298`；最坏情况也只是
`SyntaxHighlighter::new` 打一行 warn 后回落 `text`，`CMP\src\highlighter\highlighter.rs:334-345`）。

→ 所以**回落策略有两条，选一条并写清**：

* **(A) 推荐：未知扩展名 = 不调 `.language(..)`**（保持现状的"未高亮"，语义最诚实）。
* (B) 显式 `.language("text")`：`Plain` 的 config 是 `GrammarConfig::plain("text")`
  （`CMP\src\highlighter\languages.rs:373`），`has_parser` 为 false（`CMP\src\highlighter\registry.rs:574-585`），
  结果同样是"无 highlighter"。**等价，但多一次无意义的注册表查询**，且将来若真的注册了 `text` 的 parser 会悄悄变行为。

注意 (A) 的一个便宜好处：**没有 feature 的语言名也不该写进表里**，否则就是"假装支持"。
例如只开 `tree-sitter-java` 时，映射表最诚实的形态是只映射 `.java`(+`.json`)；
其余扩展名一律回落 (A)。若维护者要"看得见的 Java 高亮"优先，这条就够。

### 4.3 在 `EditorState::new(...)` 链上补什么

只需要在**现有 builder 链上加一个 `.language(..)`**，其余什么都不用动：

* 磁盘文件：`REPO\crates\editor\src\editor_view.rs:266`
  ```rust
  let language = language_for_file(&name);              // 新增：§4.2 的映射
  let mut state = EditorState::new(window, cx).searchable(true);
  if let Some(language) = language {
      state = state.language(language);                  // ← 唯一必须补的一步
  }
  if !writable {
      state.set_readonly(true, cx);                      // 保持现状（:267-271）
  }
  ```
* JDT 虚拟源码：`REPO\crates\editor\src\editor_view.rs:615`
  用 `display_path` 推导出来的 `name`（`:607-612`，例如 `String.java`）走**同一个** `language_for_file`，
  这样 `jdt://` 反编译出来的 Java 源码也亮 —— **这是"顺手白拿"的一处，值得做**。
* **不要**调 `set_language_config`（理由见 §1.3）。
* **不要**动 `set_highlighter_factory` / `set_editor_style`：`Editor` 已经在 render 里做了
  （`CMP\src\input\editor.rs:136` → `CMP\src\input\input.rs:495`、`:510`），我们插手只会覆盖主题色。
* 顺序：`.language(..)` 与 `state.set_value(body.text, window, cx)`（`:274` / `:619`）**哪个先都行**。
  `set_value` 走 `replace_text` → `reset_lsp_state`，在 code editor 上会置 `_pending_update = true`
  （`BASE\src\input\base\state.rs:897-915` → `:1017-1022`），下一次 render 就会用当前语言名去要 highlighter
  （`BASE\src\input\base\mode.rs:293`）。建议**先 `.language(..)` 再 `set_value`**，读起来"语言在内容之前"。
* 落实位置就在**已有的 `cx.new(|cx| { ... })` 闭包内部**（`:260-273` / `:614-618`）：
  `name`（`:251-255`）在闭包外已经算好，闭包按值/借用捕获它即可；
  不要在 `cx.new` 之后再 `editor.update(..)` 补 `.language(..)` —— `.language` 是**消费 self 的 builder**
  （`BASE\src\input\base\state.rs:9240`），不是 `&mut self` 方法，`update` 里用不了它
  （那里只能用 `set_highlighter`，见 `:772`）。

### 4.4 切换语言后要不要重设 / 各 buffer 分支怎么照顾

`Buffer { text, writable }`（`REPO\crates\editor\src\buffer.rs:71-76`）的四种 `writable = false` 情形
（读不到 `:99-103`、超 2 MiB `:104-107`、二进制 `:113-115`、非 UTF-8 有损 `:126-129`）
和 `writable = true` 的正常文本：

| buffer 类型 | 要不要 `.language(..)` | 为什么 |
| --- | --- | --- |
| 正常文本（`writable = true`） | **要** | 这是本次需求 |
| 读不到 / 超限 / 二进制 / 非 UTF-8（`writable = false`） | **不要**（或都行，但没意义） | 正文是宿主补的说明文案（`notice`，`:139-145`，每行 `// ` 前缀），按扩展名判出来的语言（如 `.bin` → None）本来就不该着；**关键是别让它影响"不可写"语义**——`set_readonly` 与 `language` 完全正交（`BASE\src\input\base\state.rs:1065` vs `:9240`） |
| JDT 虚拟源码（只读，`REPO\crates\editor\src\editor_view.rs:593-642`） | **要**（用 `display_path` 推） | §4.3 |

**"切换语言后重设"这件事本身不需要我们做**：`EditorState` 的生命周期就是"一个 buffer 一个 `EditorState`"
（`REPO\crates\editor\src\editor_view.rs:259` 的注释、`REPO\crates\editor\src\buffer.rs:260-264`），
同一个 buffer 不会中途换语言；`.language(..)` 会把缓存的 highlighter 置空（`BASE\src\input\base\state.rs:9247-9248`），
`set_highlighter(..)` 则会额外 `refresh`（`:788`）。只要语言在**创建时**定好，就不存在"切完不生效"。

**唯一需要额外注意的性能分支**：`tree-sitter` 的解析是**有预算**的 —— 同步解析限 `2ms` / `256 KiB`，
超了就走 150ms 防抖的后台解析（`CMP\src\highlighter\input_adapter.rs:68-70`、`:75-80`、`:96-157`）。
我们这边 `MAX_EDITOR_BYTES = 2 MiB`（`REPO\crates\editor\src\buffer.rs:20`）的文件，
超过 256 KiB 的会走后台路径 → **打开大文件时正文先无着色、随后补上**，这是上游设计，不需要我们处理，
但"大文件打开瞬间没颜色"不要当成 bug。

---

## 5. 验证方式（机器可判定的证据）

### 5.1 结论：用**同一段 Java 源码、同一次打开**做开 feature 前后的彩色像素对比

思路（不依赖"我看截图觉得有颜色"）：

1. 固定输入：`REPO\gpui\` 下放一个**内容固定**的 Java 样本（例如 `class HighlightProbe { /* 注释 */ private static final String S = "x"; int n = 42; }`），
   让关键词、字符串、注释、数字四种颜色都出现。
2. 固定视图状态：窗口**最大化到同一尺寸**、编辑器标签只有一个、光标在 `1:1`、滚动到顶、主题固定（`theme` 用默认的深色，
   避免与系统外观联动：`REPO\crates\settings\src\schema.rs:77-93` 的 `syncSystemTheme` 默认 `false`）。
   → 两次截图**除 feature 外没有任何变量**。
3. 两次截图：
   * **B（before）**：当前 `feat/gpui-shell-rewrite` 现状（无 grammar feature、无 `.language`）。
   * **A（after）**：加 `tree-sitter-java` + `.language("java")` 之后。
4. 只在**正文区**统计，且**只看两件事，不看绝对颜色**（绝对颜色会随主题变）：
   * **`distinct_colors`（正文区内出现过的不同 RGB 值个数）**：B 应该是个位数（抗锯齿产生的灰阶），
     A 会跳到几十~上百（每种语法色 + 抗锯齿都有多个明度）。
   * **`non_gray_pixels`（`max(R,G,B) - min(R,G,B) > 24` 的像素数）**：B ≈ 0（正文是单色前景 + 灰阶抗锯齿），
     A 会显著 > 0。

判据：**A 的 `non_gray_pixels` 至少是 B 的 20 倍，且 A 的 `distinct_colors` ≥ 20**，就足以机器判定"真的亮了"。

### 5.2 具体坐标与脚本

本机 125% DPI、窗口截图 1823×1024、编辑器正文区约 `x 480..1094`。**裁剪时留出安全余量**：

* 建议裁剪 `x = 476..1100`、`y = 120..900`（避开标签栏与状态栏，正文区左右各留 4px 余量）。
* 若截图是"含标题栏的整窗"，先确认正文区 y 起点；可以先用 B 图跑一次脚本，把 `non_gray_pixels` 的
  逐行分布打出来，肉眼确认裁剪框落在代码上。

PowerShell（`System.Drawing`，只读 PNG，不改任何东西）：

```powershell
param([string]$Png, [int]$X=476, [int]$Y=120, [int]$W=624, [int]$H=780)
Add-Type -AssemblyName System.Drawing
$bmp = [System.Drawing.Bitmap]::FromFile((Resolve-Path $Png))
$colors = New-Object 'System.Collections.Generic.HashSet[int]'
$nonGray = 0; $total = 0
for ($y = $Y; $y -lt [Math]::Min($Y+$H, $bmp.Height); $y++) {
  for ($x = $X; $x -lt [Math]::Min($X+$W, $bmp.Width); $x++) {
    $c = $bmp.GetPixel($x, $y); $total++
    [void]$colors.Add(($c.R -shl 16) -bor ($c.G -shl 8) -bor $c.B)
    $mx = [Math]::Max($c.R, [Math]::Max($c.G, $c.B)); $mn = [Math]::Min($c.R, [Math]::Min($c.G, $c.B))
    if ($mx - $mn -gt 24) { $nonGray++ }
  }
}
$bmp.Dispose()
"$Png  distinct=$($colors.Count)  non_gray=$nonGray  total=$total  ratio=$([Math]::Round($nonGray/$total,4))"
```

（若沙盒把 `Add-Type` 限成 ConstrainedLanguage，就换 macOS/CI 侧同样的裁剪统计，
或退一步用现成的像素统计工具；**判据不变**。）

### 5.3 更硬的旁证（不依赖像素）

* **行为旁证 1**：`EditorState::language_name()`（`BASE\src\input\base\state.rs:9254`）现在会返回 `"java"`。
  可以在打开文件后打一行既有风格的探针日志（如 `S1_EDITOR_LANG name=java`），
  grep 日志即可断言"语言确实设上了"——但它**不能**证明"亮"（feature 没开时它照样返回 `java`）。
* **行为旁证 2（能证明 parser 真的在跑）**：代码折叠。`folding` 默认 `true`（`BASE\src\input\base\mode.rs:85`），
  折线候选**只来自 tree-sitter**（`BASE\src\input\base\state.rs:3623-3639` 的 `update_fold_candidates`）。
  所以：**gutter 出现折叠箭头 ⟺ grammar 已加载并成功解析**。
  这是一条零成本、机器可判定的"有 grammar"证据（可以数 gutter 区域的箭头像素，或直接看截图）。
  → 推荐组合：**"折叠箭头出现"证明 grammar 生效，"彩色像素计数"证明高亮渲染生效**。
* 单测层面：`REPO\crates\editor\src\buffer.rs:338-448` 已有 `read_body` 的测试模块，
  新增的 `language_for_file` 可以照同一风格加确定性单测（纯字符串映射，无 IO、无 GPUI 依赖），
  这是最便宜、最稳的回归网。

---

## 6. 未确认的点

1. **构建时间与产物体积的实测值**：本轮禁止跑 cargo（也禁止为试 feature 而编译），
   §4.1 里"几十万行 C / 数百 KB"是**推断**，没有实测。
2. **锁文件的确切变化**：预期 `gpui-component` 的依赖清单会增加 `tree-sitter` 等项，
   并可能出现 `tree-sitter` 0.26.x 与现有 0.25.10 **并存**（`REPO\Cargo.lock:7591` 起那颗来自 `lithe-core`）。
   "两份 grammar 版本并存会不会解析失败/提升"**没有实测**（Cargo 允许并存，但 35 语言的
   `tree-sitter-languages` 组合下是否存在某个 grammar 与本机锁冲突，我**没有**逐一核对）。
3. **feature 组合之间的冲突**：我只逐条读了 `KIT\Cargo.toml` / `CMP\Cargo.toml` 的 feature 定义，
   没有做"同时打开多个 `tree-sitter-*` 是否冲突"的实验；从定义看它们彼此独立、都只 `dep:` +
   隐含 `tree-sitter`（`CMP\Cargo.toml:56-233`），所以**预期**无冲突。
4. **`.language(..)` 的确切落点与性能**：我建议的是 `editor_view.rs:266`（及 `:615`），
   但**没有**验证 2 MiB 上限文件走 150ms 后台解析时（`CMP\src\highlighter\input_adapter.rs:96-157`）
   与我们的自动保存（150ms 防抖，`REPO\crates\editor\src\editor_view.rs:109`）叠加后的主观响应；
   两者都在后台/不同阶段，**预期**不冲突，但未实测。
5. **Lithe 主题要不要补 `highlight` 段**：不补也能亮（§1.5），但"亮成什么颜色"取决于上游内置调色板，
   与 Windows 真机那套 18 色回落规则（`REPO\themes\README.md:326-328` 引
   `windows/tauri/src/extensions/themes/syntax-token-colors.ts:3-44`）**不一致**。
   要不要对齐 Windows 是产品决策，本轮**未确认**。
   另外 §1.5 那条"暗色下会不会是黑字"的坑，我**没有**实际观察过一次渲染结果：
   结论是从 `REPO\crates\settings\src\theme.rs:93-95` 的调用顺序 + `CMP\src\theme\schema.rs:1066-1073`
   推出来的，**缺一次截图确认**。
   最便宜的确认方式：开 feature 后看正文区里"非灰像素"的颜色，若普通标识符的像素是 `#000000` 附近
   （而不是主题前景色），就说明命中了这条，需要补 `highlight` 段。

   > ⚠️ **2026-09 现状（本条的"未确认"只剩 Lithe 两族）**：第二批的
   > `gruvbox.json` / `jetbrains.json` / `nord.json` / `one.json` / `vscode.json` 都写了 `highlight`，
   > 映射规则见 `gpui/themes/README.md` §7；Lithe 仍不写（理由同该文档 §6.1，那条 18 色回落没有变）。
   > "黑字"那条已在代码侧收口：`crates/settings/src/theme.rs` 的 `apply_theme_by_name` 在目标主题没有
   > `highlight` 时按明暗复位内置那一份（`stamp_builtin_highlight_if_absent` + `S1_THEME highlight=builtin`
   > 探针），所以"切换到 Lithe 会留着上一个主题的语法色"不会再发生。**实机验证点**：先切到一个新主题
   > 再切回 Lithe，日志里应出现 `S1_THEME highlight=builtin theme=Lithe Dark …`，且正文不是黑字。
   > **同时也要知道**：
   > `highlight` 段是**整段替换**而不是按字段合并，所以主题文件里没写的字段（`editor.invisible`、
   > status 色等）会走各自的 `unwrap_or` 回落——逐项见 `gpui/themes/README.md` §7.4。
6. **是否也应给 `.json` / `.md` 等开 feature**：本轮只回答了"Java 要开什么"，
   多语言的 feature 组合取舍（体积 vs 覆盖）留给维护者定；映射表（§4.2）里我按"没有 feature 就不写进表"
   的原则给了最小形态，但没有替维护者决定要开几种语言。
7. **`explorer` 侧的同名 `icon_for_file`**（`REPO\crates\explorer\src\model.rs:272`）：本轮确认了它与语言无关，
   但**没有**评估未来要不要把"扩展名 → 语言"抽到 `shared` 供两处复用（这属于实现期的设计选择）。
