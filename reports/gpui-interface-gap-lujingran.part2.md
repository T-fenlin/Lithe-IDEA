# GPUI 界面层三端缺口盘点（第二部分：键位方案）

> 陆井然 · 界面布局 / 信息架构 / 交互流程 / 键位
> 基线 `3085f5d1`，分支 `feat/menu-bar-opaque`
> **本次未运行构建、未做实机按键验证。以下全部为静态代码分析。**

---

## 4. 键位：`keymap.rs:62` 的「不做 macOS 档」在 macOS 上到底会怎样

### 4.1 先纠正一个容易误读的地方

`crates/settings/src/keymap.rs:60-63` 写的是：

```rust
/// **不做 macOS 档**：⌘ 在 Windows 上没有意义。三个变体的对外 id 就是 serde 写进设置文件的
/// 字面量（`lithe` / `idea-classic` / `eclipse`）
```

**这不是「macOS 上没键位」。** 真正的机制在 `keymap.rs:20-26` 的模块文档里：键位串一律写 `secondary-`，gpui 的 `Keystroke::parse` 把它按平台归一化——macOS → `platform`（即 ⌘），其他 → `control`。所以 `DEFAULT_BINDINGS` 里那 9 条（`secondary-s`、`secondary-p`、`secondary-shift-p`…，`:352-407`）**在 macOS 上会正确变成 ⌘S、⌘P、⌘⇧P**。

被「不做」的是**预设档位**，不是键位本身。这一点必须说清楚，否则会误判优先级。

### 4.2 真正的后果（四条，逐条带证据）

#### 后果一：macOS 上选不到「参考产品自己的那套键位」

macOS 端的预设枚举有 `macOS` 这一档，且它的语义就是「命令自带的默认键位」：

```swift
// macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift:11-31
enum KeyboardShortcutPreset: String, CaseIterable, Sendable {
    case macOS = "macOS"
    case ideaClassic = "idea-classic"
    case eclipse = "eclipse"
    func bindings(for command: LitheCommandDefinition) -> [KeyboardShortcutBinding] {
        switch self {
        case .macOS: command.defaultBindings          // ← ⌘ 方言的默认值
        ...
```

而 gpui 侧的枚举只有三个变体（`keymap.rs:68-73`：`Lithe` / `IdeaClassic` / `Eclipse`），**没有 macOS 档**。macOS 端那一档是参考产品的键位基准，gpui 在 macOS 上提供不了它。

**影响**：macOS 用户拿不到与现有发行版一致的键位档位。功能上是「少一个选项」，不是「键位坏了」——因为 `Lithe` 档的默认值恰好也是 `secondary-` 语义（等价于 macOS 端的 `defaultBindings` 里的 ⌘ 系）。**所以这一条的严重度是「档位缺失」，不是「不可用」。**

#### 后果二：三档预设在 macOS 上退化成同一套

`preset_bindings()`（`keymap.rs:428-434`）的实际内容：

| 档位 | 表内容 | 行号 |
|---|---|---|
| `Lithe` | **空表**（按设计：它的键位就是默认值本身） | `:430` |
| `IdeaClassic` | **只有 1 条**：`toggle-terminal` → `alt-f12` | `:410-418` |
| `Eclipse` | **空表** | `:424` |

三档里两档是空的。`secondary-` 归一化后，`Lithe` 与 `Eclipse` 在**任何**平台上解析出的键位完全相同——用户切档看不到任何变化。`IdeaClassic` 也只多生效一条。

这不是 macOS 专属问题（Windows 上一样），但在 macOS 上更刺眼，因为参考产品那边这三档是**真的不同**（`LitheCommandCatalog.swift:39-124` 三张表分别有 24 / 26 / 17 条）。

#### 后果三：Eclipse 档会一直挂一条永远修不掉的告警

`preset_gap()`（`settings/src/dialog.rs:764-787`）的分母只数「能绑键的」命令（`is_bindable` 为真），分子数预设表没覆盖的：

```rust
.filter(|command| crate::keymap::is_bindable(**command))
.filter(|command| !rows.iter().any(|row| row.command == **command))
.count()
```

`is_bindable` 由 `keymap_engine::register_bindable_resolver()`（`keymap_engine.rs:247-253`）注入，判据是 `managed_menu_action(command)` 有没有值——即**只有 9 条**（`keymap_engine.rs:67-88` 的薄表）。

于是：
- `IdeaClassic`：gap = 9 − 1 = **8**
- `Eclipse`：gap = 9 − 0 = **9**

设置页会显示告警「此预设不完整（8 / 9）」（`dialog.rs:2951-2968`）。而这两张表**永远补不满**——`keymap.rs:44-47` 自己写明 Eclipse 是「真源缺口」：「macOS 那两份 Eclipse 表是 ⌘ 方言，Windows 上不成立，而 Windows 端没有对应的 `Eclipse.xml` 真源」。

**影响**：用户切到 Eclipse 档就看到一条红色告警，且没有任何操作能让它消失。这是**已知缺陷的可见化**——按仓库「降级可见」的纪律可以接受，但应确认文案说的是「缺真源」而不是让用户以为是自己配错了。

#### 后果四：14 条命令根本无法绑键

`CommandId` 有 23 条（`shared/src/commands.rs:40-64`），但 `managed_menu_action` 只映射 9 条，其余 14 条没有可派发的 gpui `Action`，因此：

- 设置页的键位表里这 14 行**永久灰掉**（`dialog.rs:2923` 的 `bindable` 字段）；
- `preset_gap` 的分母把它们排除（`:784`），所以缺口数不虚高——这一处设计是对的；
- 但用户看到的是「23 行命令里 14 行改不了」。

这 14 条包含 `CloseTab`、`CloseOtherTabs`、`ReopenClosedTab`（这条**在**托管表里）、`OpenFile`、`SearchEverywhere`、`ToggleMaven`、`ToggleStatusBar`、`NewWindow` 等。**注意 `ToggleMaven` 与 `ToggleStatusBar` 明明有 `keybind_action` 之外的问题**：`keybind_action()`（`menu_bar.rs:477-535`）对它们返回 `None`（`:516-517`），所以它们是「有 CommandId、有菜单项、但没有可绑的 action」。

### 4.3 一个必须指出的隐性不一致

`install_actions` 里绑的是**字面 `ctrl-`**，不是 `secondary-`：

| 位置 | 绑定 | 是否被托管 |
|---|---|---|
| `crates/editor/src/lib.rs:31` | `ctrl-s` → `SaveBuffer` | **是**（会被重放剔除并换成 `secondary-s`） |
| `crates/editor/src/lib.rs:33` | `ctrl-w` → `CloseActiveTab` | 否（无 CommandId） |
| `crates/editor/src/lib.rs:37` | `ctrl-.` → `ShowCodeActions` | 否 |
| `crates/editor/src/lib.rs:40` | `cmd-.` → `ShowCodeActions` | 否（macOS 专属分支） |
| `crates/explorer/src/lib.rs:12-15` | `cmd-a`/`ctrl-a`、`cmd-c`/`ctrl-c` | 否（注释自述是「历史形态」） |
| `crates/git/src/lib.rs:31` | `ctrl-enter` → `CommitChanges` | 否 |
| `crates/settings/src/dialog.rs:52` | `ctrl-,` → `OpenSettings` | **是**（换成 `secondary-,`） |
| `crates/workbench/src/command_palette.rs:87-88` | `ctrl-shift-p`、`ctrl-m` | **是** |
| `crates/workbench/src/quick_open.rs:36` | `ctrl-p` | **是** |
| `crates/workbench/src/workspace.rs:2865` | `ctrl-o` → `OpenProjectFolder` | 否（无 CommandId） |
| `crates/workbench/src/replace_in_files.rs:39` | `ctrl-shift-r` | **是** |

`keymap.rs:349-351` 的注释说得很明白：「`ctrl-` 已按本模块的约定改写为 `secondary-`，**Windows 上两者等价**」。**但在 macOS 上两者不等价**：`ctrl-s` 是 Control+S，`secondary-s` 是 ⌘S。

后果：被托管的那批在重放后**从 ctrl- 变成 ⌘-**（`keymap_engine.rs:229-232` 整批重建）；**没被托管的那批（`ctrl-w` 关标签、`ctrl-enter` 提交、`ctrl-o` 打开文件夹、`ctrl-a`/`ctrl-c` 树选择）在 macOS 上仍是 Control 系**，不是 macOS 用户预期的 ⌘ 系。

`explorer` 那两处是 `cmd-`/`ctrl-` 双写（`explorer/src/lib.rs:12-15`），**在 macOS 上 `cmd-a` 解析成 `modifiers.platform`，恰好可用**——所以它们碰巧对了。其余几处没有双写，在 macOS 上就是真错。

**这条是我这次盘点里最实质的 macOS 键位发现**：不是「缺 macOS 档」，而是**「14 处字面 `ctrl-` 绑定里，4 处在 macOS 上语义错位」**。`keymap.rs:514-544` 的 `no_keystroke_uses_the_cmd_dialect` 测试只检查**表里**的键位串（`DEFAULT_BINDINGS` / `IDEA_CLASSIC_BINDINGS` / `ECLIPSE_BINDINGS`），**不扫 `install_actions` 里的字面绑定**——所以这条不一致现在没有守卫。

---

## 5. 三端命令 id 命名空间对照

### 5.1 三套命名规律互不相同

| 端 | 真源 | 命名规律 | 示例 |
|---|---|---|---|
| **GPUI** | `shared/src/commands.rs:66-92` | kebab-case、无域前缀、23 条 | `open-settings`、`save-buffer`、`navigate-to-definition`、`toggle-terminal` |
| **Windows 旧端** | `windows/tauri/src/features/keymaps/defaults/default-keymaps.ts` | **域前缀 + camelCase** | `file.save`、`workbench.commandPalette`、`editor.goToLine`、`window.toggleMenuBar` |
| **macOS 端** | `macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift:129-182` | 无前缀、kebab-case、短名 | `save`、`settings`、`find-in-file`、`go-to-definition`、`toggle-terminal` |

### 5.2 逐条对照（各举 ≥10 条，指出可映射与不可映射）

「语义等价」= 三端都能表达同一件事；「gpui 可映射」= gpui 侧**已有**对应 `CommandId`。

| 语义 | GPUI id | Windows id | macOS id | gpui 可映射？ |
|---|---|---|---|---|
| 保存 | `save-buffer` | `file.save` | `save` | ✅ 已有，**未建映射** |
| 打开设置 | `open-settings` | `workbench.openSettings` | `settings` | ✅ 已有，**未建映射** |
| 转到定义 | `navigate-to-definition` | `editor.goToDefinition` | `go-to-definition` | ✅ 已有，**未建映射** |
| 切换终端 | `toggle-terminal` | `workbench.toggleTerminal` | `toggle-terminal` | ✅ **已映射**（唯一真源命中） |
| 切换 Maven | `toggle-maven` | （无） | `toggle-maven` | ⚠️ 两端同名但**参考表无键位**，故意不映射 |
| 命令面板 | `open-command-palette` | `workbench.commandPalette` | （无，用 search-everywhere） | ✅ 已有，**未建映射** |
| 快速打开 | `open-quick-open` | `file.quickOpen` | （无） | ✅ 已有，**未建映射** |
| 重开已关闭标签 | `reopen-closed-tab` | `file.reopenClosed` | （无） | ✅ 已有，**未建映射** |
| 关闭标签 | `close-tab` | `file.close` | （无） | ✅ 已有，**未建映射** |
| 文件内替换 | `open-replace-in-files` | `workbench.showFindReplace` | `replace-in-project` | ⚠️ 语义不完全对齐（一个在文件内、一个是全项目） |
| 转到行 | （无 CommandId，只有 `MenuAction::GoToLine`） | `editor.goToLine` | `go-to-line` | ❌ gpui 无 CommandId |
| 文件内查找 | （无） | `workbench.showFind` | `find-in-file` | ❌ gpui 无 CommandId |
| 后退 / 前进 | （无） | `navigation.goBack` / `goForward` | `navigate-back` / `navigate-forward` | ❌ 函数已实现但无 CommandId（见 part1 第 11 行） |
| 运行 / 调试 | （无） | `run.runContextConfiguration`、`debug.start` | `run`、`debug` | ❌ gpui 无 CommandId |
| 切换断点 | （无） | `debug.toggleBreakpoint` | `toggle-breakpoint` | ❌ gpui 无 CommandId |

**这张表就是判断「缺的是映射表」的证据**：上面前 9 行里，**7 行的 gpui `CommandId` 已经存在**，macOS / Windows 侧的键位串也已经在各自的真源里，缺的只是「`save-buffer` ↔ `save` ↔ `file.save`」这一列对照。`keymap.rs:36-47` 的自述完全吻合：

> 三端命令 id 命名空间互不相同：gpui 侧 23 个 id 里只有 `toggle-terminal`、`toggle-maven` 两个与 macOS 端同名，Windows 前端又是第三套……预设表因此**无法从任何一端机械移植**，只能人工建显式映射。

### 5.3 预设档位对照

| 档位 | GPUI | Windows 旧端 | macOS 端 |
|---|---|---|---|
| 默认 / 无 | `lithe` | `none` | `macOS` |
| IDEA Classic | `idea-classic`（1 条） | `jetbrains`（8 条 overrides + 3 条 disabled） | `idea-classic`（24 条） |
| Eclipse | `eclipse`（**空**） | （无） | `eclipse`（17 条）+ parent（26 条） |
| VS Code | （无） | `vscode`（5 条） | （无） |
| Sublime / Xcode / Atom / Emacs / Zed | （无） | 各有 5～10 条 | （无） |

Windows 旧端共 **8** 档（`keybinding-presets.ts:67-171`），macOS 端 **3** 档，GPUI **3** 档但两档是空表。三端**没有一档能对齐**。

---

## 6. 三端键位对齐方案（建议）

### 6.1 建一张显式的命令映射表，不要试图机械移植

**做什么**：新增一张三端命令 id 对照表，作为键位预设的唯一移植依据。落点建议 `crates/settings/src/keymap/cross_platform.rs`（或 `shared` 下，看架构师裁定——涉及 `settings → shared` 依赖方向）。

**表长什么样**（每行一条）：

```
语义键（稳定）      gpui CommandId          macOS id            Windows id
"save"              SaveBuffer              "save"              "file.save"
"open-settings"     OpenSettings            "settings"          "workbench.openSettings"
"go-to-definition"  NavigateToDefinition    "go-to-definition"  "editor.goToDefinition"
...（先覆盖现有 9 条托管命令 + macOS 表里的 24 条可对齐项）
```

**为什么必须是显式表**：三套命名规律（kebab 短名 / kebab 短名 / 域前缀 camelCase）之间没有任何可推导的规律，且**语义也有一对多**（gpui 的 `open-replace-in-files` 对应 macOS 的 `replace-in-project`，但 gpui 侧它其实是「在文件中替换」的入口，语义边界不同）。机械映射会在这些边界上静默出错。

**配套守卫**（沿用本仓「判据机器化」的习惯）：加一条测试，断言表里每个 gpui `CommandId` 都存在、每个 macOS / Windows id 都能在对方真源文件里找到字符串。这样真源改了、表没跟上，测试会红。

### 6.2 macOS 档的处理：加档，但加的是「平台条件档」

**建议**：给 `KeymapPreset` 增加 `MacOs` 变体（id `macos`），其 `preset_bindings` 在非 macOS 平台上返回空表并**在 UI 上标注「此档仅在 macOS 有效」**；在 macOS 上则按 §6.1 的映射表把 `LitheCommandCatalog` 的 `defaultBindings` 落进来。

**为什么不照 `keymap.rs:62` 的原意继续不做**：原注释的理由是「⌘ 在 Windows 上没有意义」——这个理由成立，但结论下错了。正确的做法是**让档位平台条件化**，而不是**取消档位**。macOS 是仓库自述的 reference product，它的键位档位是规格的一部分（选型笔记 §5：「界面规格真源是 IDEA……验收取 Windows ∪ macOS 并集」）。

**代价与边界**：这会让 `KeymapPreset::ALL` 的成员依赖平台，`preset_ids_match_the_serde_form`（`keymap.rs` 测试）与 `categories_match_the_windows_list_without_ai` 一类的计数断言需要同步。**若架构师认为平台条件枚举代价太大**，退一步的方案是：保留三个变体，但把 `MacOs` 语义并进 `Lithe` 档（即 `Lithe` 档在 macOS 上就是 macOS 端的 `defaultBindings`），并在 UI 文案里说明。**这一条需要主理人裁。**

### 6.3 Eclipse 档：补齐真源，或明确标为「不可用」

现状是空表 + 一条永远修不掉的告警。两个选项：

- **选项 A（推荐）**：找 Eclipse 官方的 keymap 真源（Eclipse 的 `org.eclipse.ui` 默认键位绑定有公开文档与 CSV），按 §6.1 建表补上。macOS 端那两张表（`eclipseBindings` / `eclipseParentBindings`）本身就是从 Eclipse 的 macOS 绑定里整理出来的，可以作为交叉校验。
- **选项 B**：把 Eclipse 档从 `KeymapPreset::ALL` 里摘掉，或在 UI 上标成「暂不可用（缺真源）」并**不给缺口告警**——告警的前提是「用户能修」，这条修不了。

**不建议**保持现状：一条永久红色告警会让用户怀疑自己的配置。

### 6.4 把字面 `ctrl-` 绑定统一到 `secondary-`

**做什么**：把 §4.3 表里**非托管**的那几处字面绑定改写成 `secondary-`（`editor/src/lib.rs:33` 的 `ctrl-w`、`:37` 的 `ctrl-.`、`git/src/lib.rs:31` 的 `ctrl-enter`、`workbench/src/workspace.rs:2865` 的 `ctrl-o`），`explorer/src/lib.rs:12-15` 的双写收敛成 `secondary-a` / `secondary-c`。

**为什么**：macOS 上 `ctrl-w` 关标签、`ctrl-enter` 提交都不符合平台惯例；而 `secondary-` 在 Windows 上等价于 `ctrl-`（`keymap.rs:22-26`），**改动对 Windows 零影响**。

**配套守卫**：把 `no_keystroke_uses_the_cmd_dialect`（`keymap.rs:514-544`）的扫描范围从「三张预设表」扩到「全仓 `KeyBinding::new` 的字面串」，这样新增字面 `ctrl-` / `cmd-` 会被拦下。

### 6.5 键位冲突处理策略

现状与建议：

| 层 | 现状 | 建议 |
|---|---|---|
| 优先级 | 四层合成，第一层命中即止：用户覆盖 → 预设表 → 注册表默认值 → 未绑定（`keymap.rs:8-15`） | **保持**。这个口径与 macOS 端 `bindings(for:)` 一致（`LitheCommandCatalog.swift:24-32`），是对的 |
| 空列表语义 | 第 1、2 层空列表 = 显式「此预设下无键位」，不回填（`keymap.rs:17-18`）；`keymap_engine` 用「一条都不 add」实现真解绑（`keymap_engine.rs:32-37`、测试 `an_empty_override_really_unbinds_the_command`） | **保持**。这是正确且已被测试钉住的 |
| context 谓词 | 目录每行留 `context`，重绑时原样带回；今天 9 行都是 `None`（`keymap.rs:27-30`） | 编辑器键位带 `ElyEditor` context（`lithe-ui/src/editor/keys.rs:58`）。**建议**：一旦把编辑类命令纳入托管表，必须带上 context，否则 `secondary-z` 会在编辑区之外触发 |
| 冲突检测 | 设置页有「此预设不完整」告警（`dialog.rs:2951-2968`），但**没有「两条命令抢同一个键」的检测** | **建议新增**。macOS 端在启动时用 `precondition` 做全表冲突检测（`LitheCommandCatalog.swift:205-221` 的 `validated`），gpui 侧没有对等物。用户录一个已被占用的键会静默覆盖 |
| 平台方言 | 表内禁 `cmd-`（有测试 `no_keystroke_uses_the_cmd_dialect`） | **保持并扩范围**（见 §6.4） |

---

## 7. 工作量估算（界面层单维度）

人日为**界面层单维度**的估算，不含内核 / Core / 构建链路的改动。构建 15 分钟/次、每天 3～4 次有效构建的约束已计入（每个工作包按「成批写完再一起验」估）。

| # | 工作包 | 人日 | 依赖 | 完成标准 |
|---|---|---|---|---|
| W1 | P0 七条纯接线（切换注释、缩略图 / 行号 / 空白字符开关、后退 / 前进） | 2.0 | 无（`lithe-ui` 的 builder 与 `go_back`/`go_forward` 都已在） | 6 个菜单项从 NotWired 变 Action；`MISSING_EDITOR_FEATURES` 从 9 降到 3；启动日志 `S1_MENU notWired` 计数下降；矩阵同步 |
| W2 | 快捷键一览入口接线（含 `Category::Keyboard` 登记修正） | 0.5 | 无 | 两处菜单项指向已有的 `keyboard_page`；`Category::Keyboard` 进 `IMPLEMENTED` 并删前置文案；`dialog.rs:3504` 的测试同步改 |
| W3 | Spring 面板入口（rail 项 + 映射 + 测试） | 1.0 | 无 | 右栏出现第 4 个图标；点击渲染 `spring_panel.rs`；`right_tool_window.rs:501-511` 断言更新 |
| W4 | P1 标签页切换（上 / 下一个）+ 键位 | 1.5 | 无 | 两个菜单项变 Action；`ctrl-pageup` / `ctrl-pagedown` 或 `secondary-alt-left/right` 绑定；带 context |
| W5 | P1 文件生命周期（新建标签页 / 新建文件 / 另存为 / 全部保存 / 还原文件） | 4.0 | 需 buffer 落盘到新路径与无标题 buffer 模型（**编辑器侧配合**） | 五个菜单项接线；新建→编辑→保存→另存为链路走通；矩阵 `file-lifecycle` 一类条目更新 |
| W6 | 编辑器内查找 / 替换接线 | 3.0 | `lithe-ui` 的 `FindWidget` 已在（`find.rs:60`），需接进 `EditorPane` | Ctrl+F 出查找条；`MenuAction::Find` / `FindAndReplace` 拿到 `keybind_action`；菜单显示键位胶囊 |
| W7 | 终端分栏 | 3.0 | 需 `TerminalPane` 支持分栏布局（当前只有一组页签，`session.rs:253-286`） | 终端菜单两项接线；能左右 / 上下分栏并各自独立会话 |
| W8 | 三端命令 id 映射表 + 守卫测试 | 2.0 | 无（纯数据 + 测试） | 表覆盖 9 条托管命令 + macOS 表可对齐项；守卫测试断言双向都能在真源里找到 |
| W9 | 字面 `ctrl-` 统一到 `secondary-` + 扩范围守卫 | 1.0 | 无 | 4 处改完；`no_keystroke_uses_the_cmd_dialect` 扩到全仓字面串；Windows 行为不变 |
| W10 | macOS 档位（平台条件档或并入 `Lithe` 档） | 2.0 | **依赖 W8**；且需主理人裁「平台条件枚举 vs 并入 Lithe」（§6.2） | macOS 上能选到与参考产品一致的键位档；`KeymapPreset::ALL` 相关计数断言同步 |
| W11 | Eclipse 档补真源或标为不可用 | 2.0 | 需外部真源（Eclipse 官方 keymap） | 二选一落地；缺口告警消失或有明确「缺真源」说明 |
| W12 | 键位冲突检测 | 1.5 | 无 | 设置页在用户录制已被占用的键时给出提示；有测试钉住 |
| W13 | P2 项目生命周期（关闭窗口 / 退出 / 关闭项目） | 3.0 | 多窗口句柄路由（`open_window_host` 已在，`main.rs:684-718`） | 三项接线；多窗口下关闭语义正确 |
| W14 | P2 帮助 / 关于落地页（文档、新特性、更新日志、报告问题、请求功能） | 2.0 | 需外链 / 落地页内容（**非纯界面**） | 五项接线或明确降级为「打开外链」 |
| W15 | 未接线入口的可见性一致性（§3.3：Action 但无 `keybind_action` 的项也要有未接线性标注） | 1.0 | 无 | `Find` / `FindAndReplace` 一类项在菜单上可见地标注未接线，不再静默无响应 |
| | **合计** | **29.5** | | |

**纯界面层可立即开工的（W1～W4、W8、W9、W12、W15）= 10.5 人日**，都不依赖别处。
**需编辑器内核配合的（W5、W6、W7）= 10.0 人日。**
**需外部真源或裁定的（W10、W11、W14）= 6.0 人日。**
**需多窗口路由的（W13）= 3.0 人日。**

### 建议的先后顺序与依赖链

```
第一批（0.5 周，无依赖，感知最强）
  W2（0.5）→ W1（2.0）→ W3（1.0）→ W4（1.5）
  合计 5.0 人日；一次构建可验完
       ↓
第二批（0.5 周，纯数据，可并行）
  W9（1.0）→ W8（2.0）→ W12（1.5）
  合计 4.5 人日；W8 是 W10 的前置
       ↓
第三批（1 周，需内核配合）
  W6（3.0）→ W5（4.0）→ W7（3.0）
  合计 10.0 人日；W6 最该先做（菜单项已在但静默无响应）
       ↓
第四批（0.5 周，需裁定 / 外部真源）
  W10（2.0，依赖 W8）→ W11（2.0）→ W14（2.0）
  合计 6.0 人日
       ↓
第五批（0.5 周）
  W13（3.0）→ W15（1.0）
  合计 4.0 人日
```

### 依赖关系里必须说清的两条

1. **W10（macOS 档）卡在裁定上，不卡在技术上**。§6.2 的两个方案（平台条件枚举 / 并入 `Lithe` 档）代价不同，需主理人先裁。在裁定前 W10 无法开工，但**W8 的映射表可以先做**——两个方案都要用它。
2. **构建链路没建好之前，W5～W7 的验收做不了**。这三项改的是 `editor` 与 `terminal` crate 的运行时行为，验收方式是启动日志的 `S1_*` 打点（`menu_bar.rs:71-76` 的 `S1_MENU notWired`、`editor_view.rs:1090` 的 `S1_NAV_BACK` 等），都需要**真的跑起来**。而 GPUI 端目前没有 macOS / Linux 的构建入口（矩阵与证据台账 §3），所以 macOS 侧的界面验收只能等构建链路（**与常通关核对**）。Windows 侧可验，但受 15 分钟/次构建与 `explorer.exe` 内存泄漏的制约。

---

## 8. 未覆盖与未验证

| 项 | 状态 |
|---|---|
| 实机按键验证 | **未执行**。所有键位结论均来自静态读码，未在 macOS / Windows 上真的按过键 |
| 构建与启动日志 | **未执行**。未跑 `cargo build -p lithe-gpui-app`，`S1_MENU notWired` 的实际计数未观测 |
| `crates/java` 的 LSP 请求面 | 只做了关键词检索（`signatureHelp` / `formatting` / `rename` / `references` / `implementation` 命中 0），**未逐行读实现**。这项归闻本立，我给的只是检索结论 |
| 编辑器分屏的实现路径 | 只确认了「全仓无 editor group」，**未评估** `lithe-ui` 的 `layout/panes.rs`（有 `split()`，`:166`）能否直接承担编辑区分屏——这需要架构师判断 |
| Windows 旧端菜单项的 handler 是否真接线 | **部分核实**。逐个追了「分屏编辑器」一条，链路完整（`window-menu-bar.tsx:373-375` → `use-menu-events.ts:50` → `use-menu-events-wrapper.ts:265-267` → `pane-command-actions.ts:53` 的 `splitActiveEditorGroup`）；**其余约 50 条未逐个追**。发现的反例：同文件的 `onToggleVim`（`:268-276`）注释自述「For now, we'll show a notification」，只是弹「即将推出」——即旧端菜单里**也有假入口**，`menu_bar.tsx` 有 `onClick` 不等于功能可用 |
| macOS 端 `LitheActionCatalog` 的完整分组 | 只读了 `LitheActionGroup` 的存在与部分用法，**未完整枚举**分组清单 |
| 三端键位预设的逐条差异 | 只对照了档位名称与条数，**未逐条比对**同名档位里的键位串差异 |
