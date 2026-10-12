# GPUI 界面层三端缺口与键位方案（陆井然）

> 界面设计师 · 界面布局 / 信息架构 / 交互流程 / 键位
> 基线 `3085f5d1`，分支 `feat/menu-bar-opaque`
> **本次未运行构建、未做实机验证。以下全部为静态代码分析，每条结论可指到 文件路径:行号 或矩阵 JSON id。**

---

## 结论

### 我负责的维度

GPUI 统一前端（`rust/lithe-gpui`）相对 **Windows 旧端**（`windows/tauri`）与 **macOS 端**（`macos/`），在**界面布局、信息架构、交互流程、键位**四类用户可见界面上的缺口、未接线入口的优先级、以及三端键位对齐方案。

### 四句话结论

1. **布局骨架不是缺口。** 标题栏、左右活动栏、Dock 三块工具窗、状态栏、窗口根浮层都在（`workspace.rs:3157-3204`、`title_bar.rs:28`、`activity_bar.rs:67`、`status_bar.rs:64`、`app/src/main.rs:709`）。缺口集中在**面板与视图的种类**、**编辑器内的视图开关**、**已有实现未接线**三类。

2. **44 个未接线入口里，28 条（63.6%）是界面层该补的**，其中 **7 条是纯接线**（实现已在仓里，只差一根线），是性价比最高的一批；另 16 条前置在编辑器内核、Core 命令或独立子系统，界面层补不了。

3. **键位问题的实质不是「macOS 上没有键位」，而是两件事**：① 三档预设在任何平台都退化成同一套（两档空表）；② **14 处字面 `ctrl-` 绑定里有 4 处在 macOS 上语义错位**（该是 ⌘ 却是 Control），而现有守卫测试扫不到它们。命令 id 命名空间（gpui 23 条 kebab 短名 / macOS 无前缀短名 / Windows 域前缀 camelCase）之间**无机械对应关系**，必须人工建映射表。

4. **界面层单维度总工作量 29.5 人日**；其中 **10.5 人日可立即开工**（无任何外部依赖），10.0 人日需编辑器内核配合，6.0 人日需外部真源或裁定，3.0 人日需多窗口路由。

### 需要主理人决策的两条

- **`KeymapPreset` 是否平台条件化**（§6.2）。「不做 macOS 档」的理由（⌘ 在 Windows 上无意义）成立，但结论下错了——正确做法是让档位平台条件化，而不是取消档位。两个方案的代价不同，需裁。
- **Eclipse 档补真源还是标为不可用**（§6.3）。现状是一条用户永远修不掉的红色告警。

---

## 证据与定位

### 1. 44 个未接线入口的计数（我自己数的）

用 PowerShell 逐行数 `menu_bar.rs` 的构造点，不是抄台账：

| 项 | 实测 | 台账 |
|---|---|---|
| `MENUS` 数组范围 | `:595-935` | 一致 |
| `MENUS` 内 `MenuItem::NotWired` | **42** | 42 ✅ |
| `MENUS` 内 `MenuItem::Action` | **51** | 52（台账 `:139` 记 52，**有 1 条差**） |
| `MENUS` 内 `MenuItem::Theme` | 1 | — |
| `MENUS` 内 `MenuItem::Separator` | 23 | — |
| `NOT_WIRED_OUTSIDE_MENUS`（`:172-178`） | **2** | 2 ✅ |
| 未接线合计 | **44** | 44 ✅ |

代码侧自证：`menu_bar.rs:2446-2451` 断言 `action_count() + not_wired_count() + 1 == SOURCE_MENU_ITEMS + LOCAL_ONLY_ITEMS`，即 `51 + 42 + 1 == 89 + 5`（`SOURCE_MENU_ITEMS = 89` 在 `:1920`，`LOCAL_ONLY_ITEMS = 5` 在 `:1921`）。**这条断言把 51 钉死了**，所以台账的 52 应是笔误（或把菜单外的 `NOT_WIRED_OUTSIDE_MENUS` 计进去了）。`MenuItem::Action` 全文件出现 63 次、`NotWired` 45 次，差额分别落在 `action_count()`/`not_wired_count()` 的 filter 与测试匹配臂上。

15 个 `MISSING_*` 分组（定义 `:79-152`，汇总 `:154`）命中数实测：`editor_features` 9、`editor_entry` 7、`lsp_requests` 7、`help_about` 5、`file_lifecycle` 2、`window_routing` 2、`terminal_split` 2、`subsystem` 2、`shortcuts_view` 2、`local_history` 1、`github_integration` 1、`project_lifecycle` 1、`updater` 1（MENUS 内合计 42），加菜单外 `clone_ui` 1、`new_project_scaffolding` 1 = **44**。与 `docs/development/gpui-gap-evidence-2026-10.md:146-162` 逐格一致。

### 2. 缺口表（29 行，见 §附录 A）

每行含「能力 / GPUI 现状 / Windows 端 / macOS 端 / 证据路径:行号」，GPUI 现状区分**未实现 / 未接线 / 已具备**三种成因。

### 3. 44 条未接线入口的分类与优先级（见 §附录 B）

判据：
- **界面层应补** = 缺失的工作主体是界面结构或交互（面板、入口、菜单项、视图、键位），且没有卡在界面层之外的硬前置。
- **非界面层** = 缺失的主体在别处（编辑器内核 / Core 命令 / 独立子系统）。

| 归类 | 条数 | 占比 |
|---|---|---|
| **界面层应补** | **28** | 63.6% |
| ├ P0（纯接线，实现已在） | 7 | 15.9% |
| ├ P1（界面层新增，范围小） | 11 | 25.0% |
| └ P2（依赖别处或范围大） | 10 | 22.7% |
| **非界面层** | **16** | 36.4% |

### 4. 三条界面层的独立发现

#### 发现一：快捷键一览的文案在撒谎（P0 里最划算的一条）

`Category::Keyboard` 登记了前置条件文案「前置条件：键位表与快捷键预设（现在只有固定绑定的少数快捷键）。」（`settings/src/dialog.rs:216` → `shared/locales/lithe.zh-CN.yml` 的 `settings.gpui.prerequisiteKeyboard`），且**不在** `Category::IMPLEMENTED`（`:163-171` 只列 General / Appearance / Project / Run / Editor / Terminal / Lsp）。

但页面调度里它是**真页面**：`dialog.rs:1009` `Category::Keyboard => self.keyboard_page(cx)`，空态分支只给 `Category::Logs | Category::Updates`（`:1012`）。`keyboard_page`（`:2887-3038`）有预设下拉、可搜索键位表、录制、导入导出。

而且 `dialog.rs:3504-3521` 的测试 `every_category_is_implemented_or_declares_a_prerequisite` 断言「有前置 ⟺ 不在 IMPLEMENTED」——**这条不一致被测试固化了**：页面是真的，登记说它是空的。

于是两处菜单入口（`:870-874` 工具菜单、`:901-905` 帮助菜单）保持 NotWired，用户看到「缺快捷键一览界面」，而设置里明明有。

#### 发现二：右栏 Spring 面板不止「缺一个入口」

`RightToolWindowView::Spring` 的 `id()` / `from_id()` / `title()` / `icon()` 四个分支都已实现（`right_tool_window.rs:23-84`），扫描逻辑也在（`workspace.rs:2385-2393`）。但：

```rust
// right_tool_window.rs:47-53
pub const fn from_rail_index(index: usize) -> Option<Self> {
    match index {
        1 => Some(Self::Notifications),
        2 => Some(Self::Maven),
        _ => None,          // Spring 没有映射
    }
}
```

而 `right_activity_items()`（`workspace.rs:3245-3258`）只渲染 3 项：`EXTENSIONS_RAIL_IX=0` 插件、1 铃铛、2 Maven。**Spring 连图标都没进 rail。**

`docs/development/gpui-parity-plan-2026-10.md:134-154` 把它描述为「只差一个入口」，**低估了**——是「rail 项 + 映射 + 测试」三处，还要同步改 `right_tool_window.rs:501-511` 的断言（现有用例断言 `from_rail_index(3) == None`）。

#### 发现三：编辑器内查找是「静默无响应」，比 NotWired 更难诊断

`MenuAction::Find` / `FindAndReplace` 是 `MenuItem::Action`（`menu_bar.rs:673-674`），但 `keybind_action()` 对它们返回 `None`（`:503-506`），注释写明「`lithe_ui` 的编辑器有 FindWidget，但 `CodeEditor` 还没有接线」。

后果与 NotWired **不同**：菜单项画得出来、也点得动（走 `dispatch_editor_action`，`workspace.rs:1872-1881`），但没有可派发的 gpui action，所以**既没有键位胶囊、按下 Ctrl+F 也不会有反应**。

这与选型笔记 §9 的「降级可见」纪律不一致（「不许画假按钮、不许点了没反应」）。**建议**：这类「有 Action 但无 `keybind_action`」的项也应有可见的未接线性标注。

### 5. 键位：四条后果与三端对齐方案（见 §附录 C）

---

## 风险与条件

| # | 风险 / 条件 | 影响 | 触发条件 / 假设 | 可能推翻结论的证据 |
|---|---|---|---|---|
| R1 | **未跑实机，键位结论是静态推断** | §4.3 的「macOS 上 4 处 `ctrl-` 语义错位」是读码推断。`KeyBinding::new` 的解析在 gpui 内部，`ctrl-w` 在 macOS 上是否真的绑到 Control+W（而不是被 gpui 归一化掉）**未实机确认** | 假设 gpui 的 `Keystroke::parse` 对字面 `ctrl-` 不做平台改写。依据是 `keymap.rs:22-26` 明确把 `secondary` 称为「平台归一化开关」，暗示字面 `ctrl` 不是 | 在 macOS 上跑一次、按 Control+W 看是否关标签。若 gpui 也把字面 `ctrl-` 归一化了，R1 连同 §6.4 整条工作包作废 |
| R2 | **`MenuItem::Action` 计数与台账差 1**（51 vs 52） | 只影响台账准确性，不影响 44 这个总数（代码断言自证） | — | 台账若另有依据（如把菜单外 2 条计入 Action），则以台账为准 |
| R3 | **`crates/java` 的 LSP 请求面只做了关键词检索** | §附录 B 里 7 条 `lsp_requests` 归为「非界面层」，依据是 `signatureHelp` / `formatting` / `rename` / `references` / `implementation` 命中 0。**若这些请求实际存在于 Core（`rust/lithe-core`）而只是 `crates/java` 没封装**，结论要从「非界面层」改成「界面层 + 接线」 | 假设检索关键词覆盖了实际命名 | 闻本立核对 `rust/lithe-core/src/lsp/` 的请求面 |
| R4 | **编辑器分屏的实现路径未评估** | §附录 B 把「分屏编辑器」放 P0，但**没说清怎么实现**。`lithe-ui` 的 `layout/panes.rs:166` 有 `split()`，能否直接承担编辑区分屏未判断 | 假设需要先定架构 | 架构师判断 `layout/panes.rs` 是否够用；若够用则工作量从「新架构」降到「接线」 |
| R5 | **Windows 旧端也有假入口** | 追「分屏编辑器」时发现链路完整，但同文件的 `onToggleVim`（`use-menu-events-wrapper.ts:268-276`）注释自述「For now, we'll show a notification」——**只弹「即将推出」**。说明 `menu_bar.tsx` 有 `onClick` ≠ 功能可用 | — | 这说明「矩阵 Windows 列 implemented」对某些条目也不可信，与证据台账 §1.7 的「矩阵高估」是同一类问题 |
| R6 | **`KeymapPreset` 平台条件化会牵动计数断言** | `preset_ids_match_the_serde_form`、`categories_match_the_windows_list_without_ai` 一类测试要同步改 | 若选平台条件枚举方案 | 架构师可能反对在枚举里引入平台条件 |
| R7 | **矩阵无 gpui 列，本报告的条目无法登记** | `docs/development/gpui-parity-plan-2026-10.md:344-348` 把这个列为待确认。我的缺口表里带矩阵 id 的行（第 16、17、21、26 行）现在只能引用 macOS/Windows 列 | 未决项 | 主理人裁定登记方式；若新增 gpui 列，本报告的 29 行可直接迁移 |

---

## 建议与验收

### 建议的先后顺序与依赖链

```
第一批（0.5 周，无依赖，感知最强） 5.0 人日
  W2 快捷键一览入口（0.5）→ W1 P0 七条纯接线（2.0）→ W3 Spring 面板（1.0）→ W4 标签页切换（1.5）
  一次构建可验完
       ↓
第二批（0.5 周，纯数据，可并行） 4.5 人日
  W9 字面 ctrl- 统一（1.0）→ W8 三端映射表（2.0）→ W12 键位冲突检测（1.5）
  W8 是 W10 的前置
       ↓
第三批（1 周，需编辑器内核配合） 10.0 人日
  W6 编辑器内查找接线（3.0）→ W5 文件生命周期（4.0）→ W7 终端分栏（3.0）
  W6 最该先做（菜单项已在但静默无响应）
       ↓
第四批（0.5 周，需裁定 / 外部真源） 6.0 人日
  W10 macOS 档（2.0，依赖 W8）→ W11 Eclipse 档（2.0）→ W14 帮助落地页（2.0）
       ↓
第五批（0.5 周） 4.0 人日
  W13 项目生命周期（3.0）→ W15 未接线可见性一致性（1.0）
```

### 工作量表（界面层单维度）

| # | 工作包 | 人日 | 依赖 | 完成标准 |
|---|---|---|---|---|
| W1 | P0 七条纯接线（切换注释、缩略图 / 行号 / 空白字符开关、后退 / 前进） | 2.0 | 无 | 6 个菜单项从 NotWired 变 Action；`MISSING_EDITOR_FEATURES` 从 9 降到 3；`S1_MENU notWired` 计数下降；矩阵同步 |
| W2 | 快捷键一览入口 + `Category::Keyboard` 登记修正 | 0.5 | 无 | 两处菜单项指向已有 `keyboard_page`；该分类进 `IMPLEMENTED` 并删前置文案；`dialog.rs:3504` 测试同步 |
| W3 | Spring 面板入口（rail 项 + 映射 + 测试） | 1.0 | 无 | 右栏出现第 4 个图标；点击渲染 `spring_panel.rs`；`right_tool_window.rs:501-511` 断言更新 |
| W4 | 标签页切换（上 / 下）+ 键位 | 1.5 | 无 | 两项变 Action；带 context 绑定；键盘可切 |
| W5 | 文件生命周期（新建标签页 / 新建文件 / 另存为 / 全部保存 / 还原文件） | 4.0 | **编辑器侧**：buffer 落盘到新路径 + 无标题 buffer 模型 | 五项接线；新建→编辑→保存→另存为走通；矩阵更新 |
| W6 | 编辑器内查找 / 替换接线 | 3.0 | `lithe-ui` 的 `FindWidget` 已在（`find.rs:60`），需接进 `EditorPane` | Ctrl+F 出查找条；`Find`/`FindAndReplace` 拿到 `keybind_action`；菜单显示键位胶囊 |
| W7 | 终端分栏 | 3.0 | `TerminalPane` 需支持分栏（当前只有一组页签，`session.rs:253-286`） | 两项接线；左右 / 上下分栏并各自独立会话 |
| W8 | 三端命令 id 映射表 + 守卫测试 | 2.0 | 无 | 表覆盖 9 条托管命令 + macOS 表可对齐项；守卫断言双向都能在真源找到 |
| W9 | 字面 `ctrl-` 统一到 `secondary-` + 扩范围守卫 | 1.0 | 无 | 4 处改完；`no_keystroke_uses_the_cmd_dialect` 扩到全仓字面串；Windows 行为不变 |
| W10 | macOS 档位 | 2.0 | **W8** + 主理人裁 §6.2 | macOS 上能选到与参考产品一致的档位；`KeymapPreset::ALL` 计数断言同步 |
| W11 | Eclipse 档补真源或标为不可用 | 2.0 | 外部真源 | 二选一落地；缺口告警消失或有「缺真源」说明 |
| W12 | 键位冲突检测 | 1.5 | 无 | 录制已占用键时给提示；有测试钉住 |
| W13 | 项目生命周期（关闭窗口 / 退出 / 关闭项目） | 3.0 | 多窗口句柄路由（`open_window_host` 已在） | 三项接线；多窗口下语义正确 |
| W14 | 帮助 / 关于落地页（文档、新特性、更新日志、报告问题、请求功能） | 2.0 | **非纯界面**：需外链 / 内容 | 五项接线或明确降级为「打开外链」 |
| W15 | 未接线可见性一致性（发现三） | 1.0 | 无 | `Find`/`FindAndReplace` 一类项在菜单上可见标注未接线 |
| | **合计** | **29.5** | | |

### 依赖关系里必须说清的两条

1. **W10 卡在裁定上，不卡在技术上。** §6.2 的两个方案代价不同，需先裁。裁定前 W10 不能开工，但 **W8 可以先做**——两个方案都要用它。

2. **构建链路没建好之前，W5～W7 的验收做不了。** 这三项改的是 `editor` 与 `terminal` 的运行时行为，验收靠启动日志 `S1_*` 打点（`menu_bar.rs:71-76` 的 `S1_MENU notWired`、`editor_view.rs:1090` 的 `S1_NAV_BACK`），必须**真的跑起来**。而 GPUI 端目前没有 macOS / Linux 构建入口（证据台账 §3），macOS 侧界面验收只能等构建链路（**与常通关核对**）。Windows 侧可验，但受 15 分钟/次构建与 `explorer.exe` 内存泄漏制约。

### 验收方式（每项都要能观测）

| 类别 | 验收方式 |
|---|---|
| 菜单项接线 | 启动日志 `S1_MENU notWired` 计数下降（`menu_bar.rs:71-76` 打点）；`menu_bar.rs:2446-2451` 的计数断言随 `MISSING_GROUPS` 变化而通过 |
| 面板 / 工具窗 | 启动日志 `S1_RIGHT_PANEL view=... visible=...`（`right_tool_window.rs:127-130`） |
| 编辑器内操作 | `S1_NAV_BACK` / `S1_NAV_FORWARD`（`editor_view.rs:1090`、`:1101`）等打点 |
| 键位 | `S1_KEYMAP snapshot total=... managed=... other=...`（`keymap_engine.rs:134-138`）确认快照时机正确；设置页键位表显示与实按一致 |
| 矩阵 | 每批功能同步 `shared/platform-feature-matrix/features/<id>.json`，跑 `node scripts/generate-platform-feature-matrix.mjs` + `./scripts/verify-platform-feature-matrix.sh` |
| 清理 | 验证结束后关闭 Lithe 进程并确认无残留（根 `AGENTS.md`） |

---

## 交接给主理人

### 需与其他成员核对的具体问题

| 与谁核对 | 核对什么 | 为什么会影响我的结论 |
|---|---|---|
| **闻本立**（架构师） | `rust/lithe-core/src/lsp/` 与 `crates/java` 里是否已有 `signatureHelp` / `formatting` / `rename` / `references` / `implementation` / `typeDefinition` 请求 | 我在 `crates/java` 检索这些关键词命中 0，据此把 7 条 `lsp_requests` 归为「非界面层」。**若 Core 已有而只是 java crate 没封装，这 7 条的归类要改**，界面层工作量 +7 条 |
| **闻本立** | `lithe-ui` 的 `layout/panes.rs:166` 的 `split()` 能否承担编辑区分屏 | 决定「分屏编辑器」是「新架构」还是「接线」，直接影响 W1 与工作量 |
| **闻本立** | `EditorPane` 接 `FindWidget` 的路径（发现三） | W6 的 3.0 人日估算依赖这个 |
| **颜思齐**（UI 设计师） | 菜单项「未接线」角标（`NOT_WIRED_BADGE_KEY`，`menu_bar.rs:41`）与发现三要新增的「有 Action 无 keybind_action」标注，视觉上怎么统一 | 我提出的是交互需求，具体形态归他 |
| **颜思齐** | 三端图标 / 字体的视觉条目（矩阵 5 条 `platform-specific` 里那 5 条） | 我在缺口表里只标了「与视觉相关的行」，未深入 token |
| **柯展之**（扩展工程师） | 右栏 `EXTENSIONS_RAIL_IX=0` 的扩展页与 `RightToolWindowView` 的关系 | W3 改 `right_activity_items()` 时会碰同一函数 |
| **常通关**（构建发布） | macOS / Linux 侧 GPUI 产物有无构建入口 | **W5～W7 的 macOS 验收完全依赖它**。没有入口则 macOS 侧只能写不能验 |
| **主理人** | §6.2 `KeymapPreset` 平台条件化方案二选一 | 决定 W10 的形态 |
| **主理人** | 矩阵如何登记 gpui 端（`gpui-parity-plan-2026-10.md:344-348` 的待确认项） | 决定我缺口表里 4 条带矩阵 id 的行怎么写 |

### 缺失资料

- **Eclipse 官方 keymap 真源**（Eclipse 的 `org.eclipse.ui` 默认键位绑定）。仓库里没有；macOS 端的两张表（`eclipseBindings` / `eclipseParentBindings`）是从 Eclipse 的 macOS 绑定整理来的，可作交叉校验但不是真源。
- **macOS 端 `LitheActionCatalog` 的完整分组清单**：我只确认了 `LitheActionGroup` 有 5 个成员（`LitheAction.swift:3-9`：Run / Navigation / Window / Project / History），未完整枚举各分组下的动作。

### 未覆盖内容（明确说明）

| 项 | 状态 |
|---|---|
| 实机按键验证 | **未执行**。所有键位结论均为静态读码，未在 macOS / Windows 上真按键 |
| 构建与启动日志 | **未执行**。未跑 `cargo build -p lithe-gpui-app`，`S1_MENU notWired` 实际计数未观测 |
| `crates/java` 的 LSP 请求面 | 只做关键词检索，**未逐行读实现**（见交接） |
| 编辑器分屏实现路径 | 只确认「全仓无 editor group」，**未评估** `layout/panes.rs` 能否承担（见交接） |
| Windows 旧端约 50 条菜单项的 handler | 只追了「分屏编辑器」一条（链路完整），**其余未逐个追**。已发现反例：`onToggleVim` 只弹「即将推出」 |
| macOS 端 `LitheActionCatalog` 完整分组 | 只确认分组枚举有 5 个成员，未枚举全部动作 |
| 三端同名预设档的逐条键位差异 | 只对照了档位名称与条数（3 / 8 / 3 档），**未逐条比对**键位串 |
| 视觉规格（配色、图标、token） | **不在我的职责范围**，归颜思齐 |

---

## 附录 A：缺口表（29 行）

「GPUI 现状」按任务要求区分三种成因：**未实现** = 检索不到任何实现入口；**未接线** = 实现已在，只缺入口 / 映射 / handler；**平台专属** = 刻意只属一端。

| # | 能力 | GPUI 现状 | Windows 端 | macOS 端 | 证据（路径:行号 / 矩阵 id） |
|---|---|---|---|---|---|
| 1 | 主窗口布局骨架（标题栏 + 左右活动栏 + Dock 三工具窗 + 状态栏） | **已具备** | implemented | implemented | `rust/lithe-gpui/crates/workbench/src/workspace.rs:3157-3204`、`title_bar.rs:28`、`activity_bar.rs:67`、`status_bar.rs:64` |
| 2 | 窗口根浮层宿主（对话框 / 通知 / 抽屉挂载） | **已具备** `WindowRoot` | 有（Tauri 原生） | 有 | `crates/app/src/main.rs:709`、`rust/lithe-ui/src/shell/host/` |
| 3 | 左侧活动栏项数 | 8 项 | 6～7 项 | 5 个 `SidebarDestination` | `workspace.rs:3220-3234` / `sidebar-pane-selector.tsx:115-235` / `AppModelSupportTypes.swift:105-111` |
| 4 | 底部工具窗种类 | 5 种（Terminal / Git / Run / Diagnostics / Debug） | 有 bottom-pane | 11 个 `ToolWindow` | `workspace.rs:310-349` / `bottom-pane.tsx` / `WorkbenchFeatureModel.swift:11-23` |
| 5 | 右栏工具窗 | 3 种（Notifications / Maven / Spring），**Spring 无 rail 映射** | Notifications + Maven | maven / agent 右停靠 | `right_tool_window.rs:23-53`、`workspace.rs:3245-3258` / `main-layout.tsx:345-354` / `WorkbenchFeatureModel.swift:59-64` |
| 6 | 编辑器分屏 | **未实现**（全仓无 editor group，检索 `split_editor\|EditorGroup\|editor_group` 只命中菜单文案） | **implemented**（链路已核实到底） | 有（`SplitHandleView`） | `menu_bar.rs:742-746` / `window-menu-bar.tsx:373-375` → `use-menu-events.ts:50` → `use-menu-events-wrapper.ts:265-267` → `pane-command-actions.ts:53` / `SplitHandleView.swift:10` |
| 7 | 编辑器内查找 / 查找替换 | **未接线**（`lithe-ui` 有 `FindWidget`，`CodeEditor` 没接） | 有 | 有 | `rust/lithe-ui/src/editor/find.rs:60`、`menu_bar.rs:503-506` |
| 8 | 缩略图开关 | **未接线**（`minimap()` builder 已在） | 有 | 有 | `rust/lithe-ui/src/editor/state.rs:325`、`menu_bar.rs:747-751` |
| 9 | 行号开关 | **未接线**（`line_numbers()` 已在） | 有 | 有 | `rust/lithe-ui/src/editor/state.rs:292`、`menu_bar.rs:753-757` |
| 10 | 空白字符开关 | **未接线**（`show_whitespace()` 已在） | 有 | 有 | `rust/lithe-ui/src/editor/state.rs:303`、`menu_bar.rs:758-762` |
| 11 | 跳转历史前进 / 后退 | **未接线**（`go_back`/`go_forward` 已实现但零调用点，带 `#[allow(dead_code)]`） | 有 | 有 | `crates/editor/src/editor_view.rs:1082-1102`、`menu_bar.rs:781-790` |
| 12 | 大纲 / 结构面板 | **未实现**（检索 `outline` 只命中文案键与 `ButtonVariant::Outline`） | 有 | 有 | `right_tool_window.rs:549` 断言 `from_id("outline") == None` |
| 13 | 快捷键一览 | **未接线**（设置页 Keyboard 分类已是完整页，两处菜单入口仍标 NotWired） | 有 | 有 | `settings/src/dialog.rs:1009`、`:2887-3038`；`menu_bar.rs:870-874`、`:901-905` |
| 14 | 文件编码 / 行尾符切换 | **未实现**（状态栏 `UTF-8` 是硬编码字符串） | 有 | 有 | `workspace.rs:2324` |
| 15 | 项目树新建 / 重命名 / 删除 | **未实现**（`explorer` 只有 Open / 复制路径 / 刷新） | 有 | 有 | `explorer/src/explorer_view.rs:152-176`、`menu_bar.rs:606-610` |
| 16 | 项目树多选与批量文件操作 | **未实现** | missing | implemented | 矩阵 `workspace-tree-batch-files` |
| 17 | Markdown 预览目录 | **未实现** | missing | implemented | 矩阵 `markdown-preview-outline` |
| 18 | 本地历史 | **未实现**（检索 `local-history` 命中 0） | 有 feature | 有 `LocalHistoryView` | `menu_bar.rs:635-639` / `macos/Sources/Lithe/Views/History/LocalHistoryView.swift:4` |
| 19 | 终端分栏 | **未实现**（`TerminalPane` 只有一组页签） | 有 `terminal.split` / `splitDown` | 有 | `terminal/src/session.rs:253-286`、`menu_bar.rs:831-840` / `default-keymaps.ts:142,148` |
| 20 | 多窗口与关闭窗口 / 退出 | **部分未接线**（`open_window_host` 已有，关闭 / 退出无入口） | 有 | 有（系统菜单栏） | `app/src/main.rs:684-718`、`menu_bar.rs:642-646`、`:654-658` |
| 21 | 应用菜单栏形态 | 自绘菜单栏 4 档 | 自绘紧凑菜单栏 | **平台专属**：系统菜单栏 `CommandGroup` | `menu_bar.rs:966-1012` / `window-menu-bar.tsx` / `LitheApp.swift:487-534`、矩阵 `workbench-compact-menu-bar` |
| 22 | 设置页分类数 | 11 类（7 类 `IMPLEMENTED`） | 14 类 | 14 类 | `settings/src/dialog.rs:149-171` / `settings-dialog.tsx:36-51` / `AppModelSupportTypes.swift:5-19` |
| 23 | 命令面板分类 | **2 类**（Settings / View） | **15 类** | **5 类** | `command_palette.rs:54-66` / `command-registry.ts` 的 `category:`（实测 15 个不同取值） / `LitheAction.swift:3-9` |
| 24 | 顶部账户 / 头像槽 | **空槽**（占位 `w_7` 空 div） | 有 | 有 | `title_bar.rs:215-217` |
| 25 | 状态栏项目 | 项目名 + 分支 + 行列 + UTF-8 + 空格数 + 锁 + 校验 | 有 | 有 | `workspace.rs:2235-2336` |
| 26 | 窗口标题显示活动文件 | **未实现** | implemented | platform-specific | 矩阵 `window-title-active-file` |
| 27 | 帮助 / 关于入口 | **未实现**（5 条 NotWired） | 有 | 有（系统菜单） | `menu_bar.rs:896-932` / `LitheApp.swift:525-534` |
| 28 | 更新器入口 | **未实现** | 有 | 有 | `menu_bar.rs:928-932` / `LitheApp.swift:526-529` |
| 29 | GitHub 集成入口 | **未实现** | 有入口但硬编码 `github: false` | 有 | `menu_bar.rs:735-739` / `backend-capabilities.ts:104` |

**读法**：第 1、2 行说明**布局骨架不是缺口**。缺口集中在**面板与视图的种类**（第 4、5、12、19 行）、**编辑器内的视图开关**（第 8、9、10 行）、**未接线的已有实现**（第 7、11、13 行）三类。

**信息架构的附带发现**：命令面板分类数是三端差距最大的一处（第 23 行：2 / 15 / 5）。gpui 只有两类而命令表里有 23 条命令——用户在一个只有两个分类的面板里找 23 条命令，分类等于没起作用。这属于信息架构而非「缺功能」，但直接影响「好找」。

---

## 附录 B：44 条未接线入口的全量分类

### B.1 界面层应补（28 条）

#### P0 —— 不补就会让用户觉得「这软件没做完」（7 条）

判据：**实现已经在仓里，只差一根线**。

| # | 入口 | 行号 | 缺什么 | 不补的观感后果 |
|---|---|---|---|---|
| 1 | 切换注释 | `menu_bar.rs:676-680` | `lithe-ui` 的 `toggle_comment`（`editor/edit.rs:334`）与键位 `secondary-/`（`editor/keys.rs:103`）都在，只差菜单 action | 编辑菜单里一个常用项点了弹「尚未接入」，而 Ctrl+/ 其实按得出来——**菜单与实际能力不一致，比纯缺失更伤信任** |
| 2 | 分屏编辑器 | `:742-746` | 无 editor group 模型（**需先定架构**） | 视图菜单第一项就是灰的 |
| 3 | 缩略图开关 | `:747-751` | `minimap()`（`editor/state.rs:325`）已在，宿主没开 | 视图菜单里三个开关连成一片「尚未接入」 |
| 4 | 行号开关 | `:753-757` | `line_numbers()`（`editor/state.rs:292`） | 同上 |
| 5 | 空白字符开关 | `:758-762` | `show_whitespace()`（`editor/state.rs:303`） | 同上 |
| 6 | 跳转历史后退 | `:781-785` | `go_back` 已实现（`editor_view.rs:1083`），带 `#[allow(dead_code)]`，无 action、无键位 | 「转到」菜单里两个箭头都点不动，是最像「半成品」的一处 |
| 7 | 跳转历史前进 | `:786-790` | `go_forward` 已实现（`editor_view.rs:1094`） | 同上 |

#### P1 —— 用户会明确抱怨（11 条）

| # | 入口 | 行号 | 缺什么 | 不补的观感后果 |
|---|---|---|---|---|
| 8 | 快捷键一览（工具菜单） | `:870-874` | **页面已存在**（`dialog.rs:1009` → `:2887-3038`），只差菜单项指向它 | 「工具 → 键盘快捷键」说「缺快捷键一览界面」，而设置里其实就有——**文案在撒谎** |
| 9 | 快捷键一览（帮助菜单） | `:901-905` | 同上 | 同一功能两个入口都是死的 |
| 10 | 另存为 | `:620-624` | 需 buffer 写盘到新路径 | 文件菜单里保存下面连着三项灰 |
| 11 | 全部保存 | `:625-629` | 需遍历 buffers 落盘 | 同上 |
| 12 | 还原文件 | `:630-634` | 需从磁盘重读 | 同上 |
| 13 | 下一个标签页 | `:814-818` | 无实现（检索 `NextTab\|next_buffer\|activate_next` 命中 0） | 多文件时只能鼠标点标签 |
| 14 | 上一个标签页 | `:819-823` | 同上 | 同上 |
| 15 | 新建标签页 | `:600-604` | 需无标题 buffer + 命名 + 落盘 | 文件菜单第一项就是灰的 |
| 16 | 新建文件 | `:606-610` | 同上 | 同上 |
| 17 | 终端分栏（右） | `:831-835` | `TerminalPane` 只有页签，无分栏布局 | 终端菜单四项里两项灰 |
| 18 | 终端分栏（下） | `:836-840` | 同上 | 同上 |

#### P2 —— 属界面层但依赖别处或范围较大（10 条）

| # | 入口 | 行号 | 缺什么 | 不补的观感后果 |
|---|---|---|---|---|
| 19 | 关闭窗口 | `:642-646` | 多窗口句柄路由（`open_window_host` 已在，`main.rs:684-718`） | 文件菜单里关闭项灰着 |
| 20 | 退出 | `:654-658` | 同上 | 只能用标题栏 X |
| 21 | 关闭项目 | `:613-617` | 项目生命周期（销毁项目窗口 + 落盘列表） | 无法从菜单切回欢迎页 |
| 22 | 克隆仓库 | `:173`（菜单外） | URL / 凭据 / 进度 UI（Core 的 `git.write` 已含 clone） | 项目下拉三项里一项是占位 |
| 23 | 新建项目 | `:175-178`（菜单外） | 脚手架生成 | 项目下拉第一项就是占位 |
| 24 | 文档 | `:896-900` | 落地页 / 外链 | 帮助菜单整段灰 |
| 25 | 新特性 | `:906-910` | 同上 | 同上 |
| 26 | 更新日志 | `:911-915` | 同上 | 同上 |
| 27 | 报告问题 | `:917-921` | 同上 | 同上 |
| 28 | 请求功能 | `:922-926` | 同上 | 同上 |

### B.2 非界面层（16 条）

| # | 入口 | 行号 | 分组 | 前置在哪 |
|---|---|---|---|---|
| 29 | 参数提示 | `:682-686` | `lsp_requests` | Java 服务侧无 `signatureHelp`（检索命中 0） |
| 30 | 格式化文档 | `:709-713` | `lsp_requests` | 无 `formatting` 请求 |
| 31 | 格式化选区 | `:714-718` | `lsp_requests` | 同上 |
| 32 | 转到实现 | `:793-797` | `lsp_requests` | 无 `implementation` 请求 |
| 33 | 转到类型定义 | `:798-802` | `lsp_requests` | 无 `typeDefinition` |
| 34 | 转到引用 | `:803-807` | `lsp_requests` | 无 `references` |
| 35 | 重命名符号 | `:808-812` | `lsp_requests` | 无 `rename` / `prepareRename` |
| 36 | 复制行 | `:689-693` | `editor_features` | `lithe-ui` 编辑器无 duplicate-line（命中 0） |
| 37 | 删除行 | `:694-698` | `editor_features` | 同上 |
| 38 | 上移行 | `:699-703` | `editor_features` | 同上 |
| 39 | 下移行 | `:704-708` | `editor_features` | 同上 |
| 40 | 显示本地历史 | `:635-639` | `local_history` | 缺本地历史存储（内容仓库） |
| 41 | GitHub | `:735-739` | `github_integration` | 缺 GitHub 集成三条链 |
| 42 | 检查更新 | `:928-932` | `updater` | 缺更新器 |
| 43 | 数据库 | `:857-861` | `subsystem` | 缺整个数据库子系统 |
| 44 | Web Inspector | `:863-867` | `subsystem` | 缺整个调试检查器子系统 |

---

## 附录 C：键位方案

### C.1 「不做 macOS 档」到底会怎样

**先纠正误读**：`keymap.rs:60-63` 的「不做 macOS 档」**不是「macOS 上没键位」**。机制在 `keymap.rs:20-26`：键位串一律写 `secondary-`，gpui 的 `Keystroke::parse` 按平台归一化——macOS → `platform`（⌘），其他 → `control`。所以 `DEFAULT_BINDINGS` 那 9 条（`:352-407`）**在 macOS 上会正确变成 ⌘S、⌘P、⌘⇧P**。被「不做」的是**预设档位**，不是键位本身。

四条真实后果：

**后果一：macOS 上选不到参考产品自己的那套档位。** macOS 端的预设枚举有 `macOS` 档，语义就是「命令自带的默认键位」（`LitheCommandCatalog.swift:11-31`，`case .macOS: command.defaultBindings`）。gpui 侧只有 `Lithe` / `IdeaClassic` / `Eclipse`（`keymap.rs:68-73`），没有 macOS 档。**严重度是「档位缺失」，不是「不可用」**——因为 `Lithe` 档的默认值恰好也是 `secondary-` 语义。

**后果二：三档预设在任何平台都退化成同一套。**

| 档位 | 表内容 | 行号 |
|---|---|---|
| `Lithe` | **空表**（按设计） | `:430` |
| `IdeaClassic` | **只 1 条**：`toggle-terminal` → `alt-f12` | `:410-418` |
| `Eclipse` | **空表** | `:424` |

两档是空的 ⇒ 切档看不到变化。这不是 macOS 专属，但在 macOS 上更刺眼，因为参考产品那边三档**真的不同**（`LitheCommandCatalog.swift:39-124` 三张表分别 24 / 26 / 17 条）。

**后果三：Eclipse 档会一直挂一条永远修不掉的告警。** `preset_gap()`（`dialog.rs:764-787`）分母只数「能绑键的」命令，`is_bindable` 由 `keymap_engine.rs:247-253` 注入、判据是 `managed_menu_action` 有没有值——**只有 9 条**（`keymap_engine.rs:67-88`）。于是 `IdeaClassic` gap = 8、`Eclipse` gap = 9，设置页显示「此预设不完整」（`dialog.rs:2951-2968`）。而这两张表**永远补不满**——`keymap.rs:44-47` 自述 Eclipse 是「真源缺口」。

**后果四：14 条命令根本无法绑键。** `CommandId` 有 23 条（`shared/src/commands.rs:40-64`），`managed_menu_action` 只映射 9 条，其余 14 条没有可派发的 gpui `Action`，设置页那 14 行**永久灰掉**（`dialog.rs:2923`）。`preset_gap` 的分母把它们排除（`:784`）——这一处设计是对的，缺口数不虚高。

### C.2 我这次最实质的 macOS 键位发现

`install_actions` 里绑的是**字面 `ctrl-`**，不是 `secondary-`：

| 位置 | 绑定 | 是否被托管（会被重放改写） |
|---|---|---|
| `crates/editor/src/lib.rs:31` | `ctrl-s` → `SaveBuffer` | **是** → 变 `secondary-s` |
| `crates/editor/src/lib.rs:33` | `ctrl-w` → `CloseActiveTab` | **否** |
| `crates/editor/src/lib.rs:37` | `ctrl-.` → `ShowCodeActions` | **否** |
| `crates/editor/src/lib.rs:40` | `cmd-.` → `ShowCodeActions` | 否（macOS 专属分支） |
| `crates/explorer/src/lib.rs:12-15` | `cmd-a`/`ctrl-a`、`cmd-c`/`ctrl-c` | 否（注释自述「历史形态」） |
| `crates/git/src/lib.rs:31` | `ctrl-enter` → `CommitChanges` | **否** |
| `crates/settings/src/dialog.rs:52` | `ctrl-,` → `OpenSettings` | **是** → 变 `secondary-,` |
| `crates/workbench/src/command_palette.rs:87-88` | `ctrl-shift-p`、`ctrl-m` | **是** |
| `crates/workbench/src/quick_open.rs:36` | `ctrl-p` | **是** |
| `crates/workbench/src/workspace.rs:2865` | `ctrl-o` → `OpenProjectFolder` | **否** |
| `crates/workbench/src/replace_in_files.rs:39` | `ctrl-shift-r` | **是** |

`keymap.rs:349-351` 注释说「`ctrl-` 已按本模块的约定改写为 `secondary-`，**Windows 上两者等价**」。**但 macOS 上不等价**：`ctrl-s` 是 Control+S，`secondary-s` 是 ⌘S。

后果：被托管的那批重放后**从 ctrl- 变成 ⌘-**（`keymap_engine.rs:229-232` 整批重建）；**没被托管的那批（`ctrl-w` 关标签、`ctrl-enter` 提交、`ctrl-o` 打开文件夹）在 macOS 上仍是 Control 系**，不是 macOS 用户预期的 ⌘ 系。

`explorer` 那两处是 `cmd-`/`ctrl-` 双写，**在 macOS 上 `cmd-a` 解析成 `modifiers.platform`，恰好可用**——所以它们碰巧对了。其余几处没有双写，在 macOS 上就是真错。

**`keymap.rs:514-544` 的 `no_keystroke_uses_the_cmd_dialect` 测试只扫三张预设表，不扫 `install_actions` 的字面绑定**——所以这条不一致现在**没有守卫**。

### C.3 三端命令 id 命名空间对照

| 端 | 真源 | 命名规律 | 示例 |
|---|---|---|---|
| **GPUI** | `shared/src/commands.rs:66-92` | kebab-case、无域前缀、23 条 | `open-settings`、`save-buffer`、`navigate-to-definition` |
| **Windows 旧端** | `keymaps/defaults/default-keymaps.ts` | **域前缀 + camelCase** | `file.save`、`workbench.commandPalette`、`editor.goToLine` |
| **macOS 端** | `LitheCommandCatalog.swift:129-182` | 无前缀、kebab-case、短名 | `save`、`settings`、`find-in-file` |

逐条对照（各 ≥10 条）：

| 语义 | GPUI id | Windows id | macOS id | gpui 可映射？ |
|---|---|---|---|---|
| 保存 | `save-buffer` | `file.save` | `save` | ✅ 已有，**未建映射** |
| 打开设置 | `open-settings` | `workbench.openSettings` | `settings` | ✅ 已有，**未建映射** |
| 转到定义 | `navigate-to-definition` | `editor.goToDefinition` | `go-to-definition` | ✅ 已有，**未建映射** |
| 切换终端 | `toggle-terminal` | `workbench.toggleTerminal` | `toggle-terminal` | ✅ **已映射**（唯一真源命中） |
| 切换 Maven | `toggle-maven` | （无） | `toggle-maven` | ⚠️ 同名但参考表无键位，故意不映射 |
| 命令面板 | `open-command-palette` | `workbench.commandPalette` | （无） | ✅ 已有，**未建映射** |
| 快速打开 | `open-quick-open` | `file.quickOpen` | （无） | ✅ 已有，**未建映射** |
| 重开已关闭标签 | `reopen-closed-tab` | `file.reopenClosed` | （无） | ✅ 已有，**未建映射** |
| 关闭标签 | `close-tab` | `file.close` | （无） | ✅ 已有，**未建映射** |
| 文件内替换 | `open-replace-in-files` | `workbench.showFindReplace` | `replace-in-project` | ⚠️ 语义不完全对齐 |
| 转到行 | （无 CommandId，只有 `MenuAction::GoToLine`） | `editor.goToLine` | `go-to-line` | ❌ gpui 无 CommandId |
| 文件内查找 | （无） | `workbench.showFind` | `find-in-file` | ❌ gpui 无 CommandId |
| 后退 / 前进 | （无） | `navigation.goBack` / `goForward` | `navigate-back` / `navigate-forward` | ❌ 函数已实现但无 CommandId |
| 运行 / 调试 | （无） | `run.runContextConfiguration`、`debug.start` | `run`、`debug` | ❌ gpui 无 CommandId |
| 切换断点 | （无） | `debug.toggleBreakpoint` | `toggle-breakpoint` | ❌ gpui 无 CommandId |

**前 9 行里 7 行的 gpui `CommandId` 已经存在**，macOS / Windows 侧键位串也已在各自真源里——缺的只是那一列对照。`keymap.rs:36-47` 的自述完全吻合：「只能人工建显式映射」。

**预设档位对照**：GPUI 3 档（两档空表） / Windows 8 档（`keybinding-presets.ts:67-171`） / macOS 3 档。三端**没有一档能对齐**。

### C.4 对齐方案

**（1）建显式映射表，不要机械移植。** 落点建议 `crates/settings/src/keymap/cross_platform.rs`（或 `shared`，看架构师裁定）。每行一条语义键 → 三个 id。**必须是显式表**：三套命名规律间无可推导规律，且语义有一对多（`open-replace-in-files` ↔ `replace-in-project` 边界不同）。配套守卫：断言表里每个 gpui `CommandId` 存在、每个 macOS / Windows id 能在对方真源里找到字符串。

**（2）macOS 档：加平台条件档。** 建议给 `KeymapPreset` 加 `MacOs` 变体，在非 macOS 上返回空表并在 UI 标注「此档仅在 macOS 有效」；在 macOS 上按映射表把 `LitheCommandCatalog` 的 `defaultBindings` 落进来。**退一步方案**（若架构师认为平台条件枚举代价太大）：保留三变体，把 macOS 语义并进 `Lithe` 档，UI 文案说明。**需主理人裁。**

**（3）Eclipse 档：补真源或标为不可用。** 选项 A（推荐）：找 Eclipse 官方 keymap 真源补上，用 macOS 端两张表交叉校验。选项 B：从 `ALL` 摘掉或标「暂不可用（缺真源）」并**不给缺口告警**——告警的前提是用户能修。**不建议保持现状**：永久红色告警会让用户怀疑自己的配置。

**（4）字面 `ctrl-` 统一到 `secondary-`。** 改 `editor/src/lib.rs:33`、`:37`、`git/src/lib.rs:31`、`workspace.rs:2865`，`explorer/src/lib.rs:12-15` 的双写收敛。**对 Windows 零影响**（`secondary-` 在 Windows 上等价 `ctrl-`）。配套：把 `no_keystroke_uses_the_cmd_dialect` 的扫描范围从三张表扩到全仓字面串。

**（5）键位冲突处理策略。**

| 层 | 现状 | 建议 |
|---|---|---|
| 优先级 | 四层合成，第一层命中即止（`keymap.rs:8-15`） | **保持**。与 macOS 端 `bindings(for:)` 同口径（`LitheCommandCatalog.swift:24-32`） |
| 空列表语义 | 第 1、2 层空列表 = 显式「此预设下无键位」（`keymap.rs:17-18`）；`keymap_engine` 用「一条都不 add」实现真解绑（`:32-37`，测试 `an_empty_override_really_unbinds_the_command`） | **保持**。正确且已被测试钉住 |
| context 谓词 | 目录每行留 `context`；今天 9 行都是 `None`（`keymap.rs:27-30`） | 编辑器键位带 `ElyEditor` context（`lithe-ui/src/editor/keys.rs:58`）。**编辑类命令纳入托管表时必须带 context**，否则 `secondary-z` 会在编辑区外触发 |
| 冲突检测 | 有「预设不完整」告警（`dialog.rs:2951-2968`），**无「两命令抢同一键」检测** | **建议新增**。macOS 端启动时用 `precondition` 做全表冲突检测（`LitheCommandCatalog.swift:205-221` 的 `validated`），gpui 无对等物。用户录已占用的键会静默覆盖 |
| 平台方言 | 表内禁 `cmd-`（测试 `no_keystroke_uses_the_cmd_dialect`） | **保持并扩范围** |
