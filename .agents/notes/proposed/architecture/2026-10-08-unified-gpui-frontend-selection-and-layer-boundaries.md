# Agent 笔记：统一 GPUI 前端的选型与分层边界

状态：提议中

## 先说结论

Lithe 现在有两套各自实现的界面（macOS 用 SwiftUI/AppKit，Windows 用 React/Tauri），
目标是换成**一套 Rust 界面代码**覆盖 macOS、Windows、Linux，鸿蒙 PC 排在后面。
本 Note 定的是这次替换的**选型与分层边界**，不是排期，也不是实现细节。

四句话记住结论：

1. **框架层只能锁 `gpui-pre`**（crates.io 上的 GPUI 快照，本文钉 `=0.3.8`）。它许可干净（Apache-2.0），
   但它是**一个人重新打包的 Zed 快照**、没有公开源码仓库、约每周把版本重基到新的 Zed 提交上。
   所以必须**把源码 vendor 进仓库、自己持有 Zed 提交号**，而不是把它当成一个普通依赖。
2. **整套前端从零重建**，不把既有 GPUI 分支（`feat/lithe-gpui-components`，445 个提交）当基线。
   组件库是 **`rust/lithe-ui`**（Ely GPUI Components 的 vendored fork 与 Lithe 自有组件
   **合成一个 crate**，MIT 派生）；界面层 **`rust/lithe-gpui`** 从零重写并 path 依赖它；
   外部发布的 `gpui-kit` **不引入**——
   不是"以后退场"，而是从一开始就不出现在依赖里。
3. **界面规格真源是 IDEA**（Community 版），不是某一端的现有实现；验收用
   `shared/platform-feature-matrix/` 的能力矩阵取 **Windows ∪ macOS 并集**。
4. **窗口根浮层从第一天就自建**（不要先接上游的 `Root` 再迁移）。它是整套自维护方案里
   唯一的结构性风险点，越早验越好。

## 问题

### 今天要解决什么

同一个用户可见的能力，现在要在两个前端各实现一遍、各测一遍、各修一遍。历史上已经做过两次
全量前端替换（Qt/C++ → Tauri → 现在的方向），所以"换一次"这件事本仓库有能力做，但很贵。

好消息是业务逻辑不用重写：`rust/lithe-core/` 已经是语言中立的 JSON 命令信封，
macOS 侧的 SwiftUI 今天就通过 C ABI（`lithe_core_execute_json_with_events`、
`lithe_core_cancel` 等，见 `rust/lithe-core/src/runtime/ffi.rs`）消费它。
所以真正的工作量在**界面层**，而不是整个应用。

### 为什么这次要重新定选型

之前那轮 GPUI 工作已经定过"用 GPUI Kit 统一三端"，那份决策笔记在
`feat/lithe-gpui-components` 等分支上（`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`，
**本分支不存在**，不能作为本分支的依据）。本次重新调研后，有四处结论与它不同，必须写下来：

1. **组件层不再是 `gpui-kit`，而是仓库内自维护的组件库**（该分支上后来也确实转向了 Ely fork，
   见 `.agents/notes/proposed/architecture/2026-10-07-host-ui-controls-move-to-own-component-library.md`）。
2. **从零重建，不复制该分支的实现**：那条线的代码是围绕 `gpui-kit` 长出来的
   （窗口根用上游 `Root`、主题走映射桥、组件调用散在两套词汇里），与本文骨架不同。
   我们继承它的**结论与教训**，不继承代码。
3. **界面规格真源是 IDEA，不是"以 macOS 前端为准"**。
4. **鸿蒙 PC 不能按"一套 GPUI 打四端"规划**（理由见下）。

### 调研发现的四个硬事实

这些是本次选型的证据基础，逐条都有一手来源：

| 事实 | 证据 |
| --- | --- |
| 官方 `gpui`（crates.io）**已实质停更**：0.2.0–0.2.2 都在 2025-10 发布，之后约 11.5 个月没有再发；Zed 本体已到 1.23.x 且主仓当天仍在提交 | crates.io `gpui`；Zed 主仓 `crates/gpui/Cargo.toml` 至今仍是 `version = "0.2.2"` |
| Zed **从未发布**它的编辑栈：`editor` / `ui` / `rope` / `theme` / `settings` 等各自声明 **GPL-3.0-or-later** 且 `publish.workspace = true` 挂在根 `publish = false` 之下 | 逐 crate 查 Zed 仓库的 `Cargo.toml`。⇒ **商用项目不能拿它们当库**，这正好解释了第三方组件库存在的价值 |
| 实际被消费的框架层是**第三方重新打包**：`gpui-pre` 的 crates.io 发布者是**单个个人账号**，`repository` 字段指向 Zed 主仓是装饰性的；其 manifest 里写明了映射（`0.3.7` = 某个 Zed 提交） | crates.io `gpui-pre` 的 owners 与 package 元数据；`gpui-pre` 在 GitHub 上**没有公开仓库** |
| 组件层 `gpui-kit` 本身是**活跃、公司背书、许可干净**的：Apache-2.0，约每周一版，贡献集中在两名开发者 | `longbridge/gpui-kit`：16,508 stars、当天仍有提交、近一月上百次提交 |

这四条合起来决定了分层：**风险在框架层（必须自己扛），不在组件层（可以自己写、也可以借）。**

## 提案

### 1. 四层边界与依赖方向

自下而上四层，**依赖只能向下**：

| 层 | 是什么 | 谁维护 | 依赖 |
| --- | --- | --- | --- |
| 框架层 | `gpui`（`gpui-pre` / `gpui-pre-platform`），负责窗口、GPU surface、输入、输入法、渲染 | **vendor 进仓库，我们持有提交号** | 无 |
| 组件库 | `rust/lithe-ui`：Ely fork 的组件（视觉语言与通用控件）**加上** Lithe 自有组件（项目树、窗口根宿主…），合成一个 crate | 我们（复用既有分支的 fork，不跟踪上游） | 框架层 |
| 界面层 | `rust/lithe-gpui/crates/*`：`app` / `workbench` / `editor` / `git` / `java` / `settings` / `terminal` / … | 我们（**从零重写**，不复用既有分支的宿主代码） | 组件库 + `lithe-core` |

**正确做法**：写业务界面时，新组件放 `lithe-ui`，能用的现成组件从 `lithe-gpui` 取。
**不要这样做**：不要把第一方代码加进 vendored 目录（下文"尺寸与缩放"一节说明为什么）。

### 2. 框架层：锁 `gpui-pre`，但必须自己扛

**决定**：使用 `gpui-pre` 这条版本线，**精确钉在 `=0.3.8`**，并把 Zed 提交号作为我们真正的版本锚点。
既有分支钉的是 `=0.3.7`；从零重建正好给了我们重选一次的机会，选 0.3.8 的原因是那一周的增量里
有我们要用的修复（Windows 剪贴板写入含内嵌 NUL 的字符串、`test-support` 可在任何构建里安全启用、
跨平台无头窗口 API 与 Linux 实现、依赖瘦身、Linux 基准内存泄漏与启动时间）。

| 项 | 值 |
| --- | --- |
| 钉版 | `gpui-pre = "=0.3.8"`、`gpui-pre-platform = "=0.3.8"` |
| 版本锚点（真身） | Zed `zed-rev 279fe070bb389b79652e52065b2f001edcc0b11b` |
| 回退配对 | 兼容核对若失败 → `=0.3.7`（Zed `1a28cff4b409169bac058bca40dfbfeb7621d19b`，有既有成功记录） |
| 别误读 | 两版的 `zed-version` 字段都写 `0.2.2`——那是 Zed 自己 `gpui` crate 的版本号，Zed 从不改它；`0.3.x` 是重新打包者自己的编号 |
| 上游节奏 | 约每周一发（0.3.4 在 09-07 … 0.3.8 在 10-05）⇒ 升级是常态，**钉版才是纪律** |

**判据（不可跳过）**：先按 0.3.8 做组件底座的类型兼容核对，失败才退 0.3.7。
不核对就开工，后面每个编译错误都会被误判成"我们写错了"。

**消费方式分两步——最终形态不变，只是不卡住骨架**：

1. **骨架阶段**：crates.io 精确钉版 `=0.3.8`，并**提交 `Cargo.lock`**（它记录了精确版本与校验和，
   起步阶段的可追溯性够用）。
2. **单独一片做 vendor**：把框架层改成本仓库内的路径依赖，并记录来源与许可。
   实际涉及约 16 个 crate（`gpui-pre` 与它的九个兄弟 crate，加 `-platform` 以及
   `-linux` / `-macos` / `-windows` / `-wgpu` / `-web`），是一件独立工程。
   **为什么最终仍要 vendor**：唯一发布者一旦撤包，精确钉版就取不到；而且我们无法在本地给上游打补丁。

**为什么必须精确钉版，不能写成宽松版本要求**：官方 crate 停更一年多，而 `gpui-pre` 是个人重新打包、
无公开仓库、约每周重基。写成 `gpui-pre = "0.3"` 这类宽松要求，等于把升级节奏交给别人——
下一次 `cargo update` 就会把你悄悄重基到新的 Zed 提交上。

**开发者要做什么**：

- 升级 `gpui-pre` 是一次**有意的重基动作**，不是顺手 `cargo update`；要单独一片、单独验证。
- 每条平台相关的硬约束要留档（例如：一旦绕过组件库直接依赖 gpui，macOS 上必须给
  `gpui-pre-platform` 打开 `font-kit`，否则窗口能开、**一个字都不渲染**）。
- 判断 API 的依据是**已发布版本的源码 + 提交的 `Cargo.lock`**，不是技能文档或在线文档
  （既有工作记录过一次"照抄文档直接编译失败"的教训）。

### 3. 组件层：从零重建，不引入 `gpui-kit`

**决定**：宿主界面从零重建；组件底座**复用既有分支已经改好的 Ely fork**，不再从上游搬一份原版；
`gpui-kit` **不进依赖列表**。

**为什么复用底座而不是重新 vendor 一遍**：那条分支上，这个路径的历史共 **39 个提交**——
其中 **5 个是更早的"手写组件库"**（后来被 Ely fork 整体替换，见 `c6da4f26`），
**Ely fork 自己占 34 个**。这些改动里大部分不是"宿主实现"，而是组件底座本身的成果，
且正好落在我们要的方向上：

| 类别 | 内容 | 处置 |
| --- | --- | --- |
| 机械改名 | 包名与标识符改成 Lithe 词汇、保留上游署名 | 直接复用（本来就要做） |
| **对齐 `gpui-kit` 的 API 重构** | 尺寸 3 档扩到 5 档 + `Sizable`、`IconButton` 并入 `Button`、`TextInput` → `InputState`、`Switch`/`Checkbox`/`Dialog` 构造改 builder、i18n 换成 `rust_i18n` 并对齐宿主 | **直接复用**——这正是"编码与约定对齐 `gpui-kit`"的既有成果 |
| 真 bug 修复 | 无障碍节点 id 撞车导致 panic、浮动面板缺"停靠回去 / 关闭"、隐藏的浮动面板仍渲染 | **必须复用**，否则重犯 |
| Dock 能力先加后撤 | 先加了一批能力，后又"撤掉没有消费方的能力"、只留外壳真正用的 | 复用**撤后**的状态（这是已辩论过的结论：不留死 API） |
| Lithe 侧新组件 | 项目树（`lists::project`） | **留在 `lithe-ui` 内**（2026-10-08 定）：组件库是一个 crate，Ely 派生与自有组件不拆家 |
| 画廊中文化 | 43 页演示的文案与翻译 | 与产品运行无关，独立取舍（它同时是回归样本） |

**底座维护口径（2026-10-08 定）**：`lithe-gpui` **保持"我们维护的 fork"**，不回退成纯上游拷贝。
理由是那 34 个提交里有**两处真 bug 修复**（无障碍节点 id 撞车导致的 panic、浮动面板缺"停靠回去 / 关闭"）
与**一批对齐 `gpui-kit` 的 API 重构**，回退等于把已经赚到的保证再丢一次。
代价必须显式承认并处理：底座不再"逐文件等于上游"，所以
① `UPSTREAM.md` 要携带**完整**改动清单（已补：34 条提交）；
② 尺寸守卫按"目录"而不是"crate 名"排除上游派生代码：把上游派生文件收进 `src/vendor/`
（守卫只扫 `src/` 下不属于 `vendor/` 的部分）。**这一步尚未执行**——它要把上游派生文件里的
`crate::` 引用整体改成 `crate::vendor::`，是独立一片，与本片的布局调整分开做。

**另一条值得记的事实**：Ely 自带 **659 项测试**，既有工作把它们跑绿过（`82bca6c0`）。
复用底座的收益不只是"组件数量"，还包括这套现成的回归网——这也是"从零手写"比不了的地方。

**一个必须记住的坑**：底座的来源说明文件（`UPSTREAM.md`）的改动记录**只有 2 条**，
而实际有 39 个提交改过它。⇒ **复用时不能只信那份说明**，要按 `git log` 自查一遍。
这也是"单一真源"要修的地方：那份清单要么补全，要么明确它只记重要改动。

**为什么从零可行**：仓库里已经证明过一件事——Ely 组件库**直接建在 `gpui` / `gpui_platform` 上，
不经过 `gpui-base` / `gpui-component`**，而且它覆盖的章节包含 `gpui-kit` 恰好缺的东西：
终端（`terminal/`）、数据表（`tables/`）、编辑器（`editor/`）、菜单（`menus/`）、浮层（`overlays/`）。
它的规模是 44 章，比手写两个迭代只到 13 章的现实产出高一个量级。

**底座的来源与合规规矩**（沿用既有做法）：

- **固定上游提交**，不跟上游 `main` 漂；上游仓库很年轻、迭代很快，跟踪它没有收益。
- **保留许可与署名**：MIT 要求保留版权声明与许可文本，上游源码文档注释里的项目名是署名，
  **不要为了"统一命名"清掉**。
- **用 Cargo 包重命名消化版本差异**：让上游源码里的 `use gpui::…` 解析到我们钉的 `gpui-pre`，
  这样上游源文件几乎不用改。
- **改动逐条留痕**在组件库自己的来源说明文件里（来源提交、改了什么、为什么）。
- **兼容性判据**：上游用到的 gpui 类型必须在我们钉的版本里都存在——既有方法是对类型逐项核对，
  再加一次 `cargo check --all-targets` 零错误零警告。这个核对**必须重做**，因为我们换了版本线。

**不引入 `gpui-kit` 换到什么**：少一层外部依赖；不需要一次"退场"工程；主题只有一个真源
（不必先接一层把上游主题映射进来的桥）；上游那两个已登记的通知缺陷根本不进入项目（见 §4）。

**代价**：宿主界面全部要自己写；组件底座的成熟度不再有外部背书（见"风险"）。

**API 与编码约定**：

- **编码与架构约定以 `gpui-kit` 的 Coding Guides / Design Guides 为准**——这两份指南本身已经是
  仓库现行规范的真源之一（例如尺寸守卫引用的就是它们）。
- **组件 API 词汇参考 `gpui-kit`，但只对"新写的 Lithe 组件"生效**。组件底座既有的词汇
  （`new(id, label)`、`ControlSize`、`IconName`…）**不回头改**——既有评估已经算过这笔账：
  `pub fn new(` 出现在数百个文件里，且 `IconName` 会和既有 expui 图标体系撞车，收益比最差。
- 因此会有一段**"底座词汇 + 新组件词汇"并存期**。它必须写进编码指南，
  否则新老写法各长各的，下一个人每写一个组件都要重新裁决一次。

### 4. 窗口根浮层：从第一天就自建

**决定**：窗口根宿主由自建库提供；**不要先接上游的 `Root`**。

**这一层要承担什么**（参考既有分支上上游 `Root` 的使用位置，仅供理解职责，不是要改的代码）：
每窗口包一次内容、持一张有序浮层栈、**每帧**渲染在内容之上、对外给"开对话框 / 推通知 /
开抽屉"的挂载入口，并统一处理遮罩点击、Esc 关闭、焦点归还、层级提升。
既有分支上的参考位置是 `crates/app/src/main.rs`、`crates/workbench/src/workspace.rs`
（`Root::new(..)`、`window.push_notification(..)`）与 `crates/workbench/src/search_everywhere.rs`。

**好消息：组件与机制已经在了，缺的只是宿主这一层。**

| 需要的东西 | 自建库里已有的实体 |
| --- | --- |
| 层级原语 | `layout/stack.rs::ZStack`——首个孩子定尺寸、其余绝对定位覆盖 |
| 遮罩 / 焦点 | `primitives::{Backdrop, FocusScope, raise, take_focus, give_back}` |
| 对话框 | `overlays/dialog.rs::Dialog`（自带"先交还焦点再关闭"的出口）、`overlays/dialogs.rs` 的确认/提示框 |
| 浮层 / 抽屉 | `overlays/{Popover, HoverCard, Lightbox, Spotlight}`、`layout/sheet.rs`、`layout/floating.rs` |
| 通知 | `feedback/toast.rs::{Toaster, ToastViewport, Toast}`、`feedback/notification.rs` |

**两条不能违反的约束**：

1. **宿主必须是"每帧渲染"的实体（`Entity`）**。既有记录里踩过"绕开 `Root` 直接写裸 GPUI ⇒ 浮层失效"，
   而上游 0.7 的浮层是每帧统一渲染的。做成"一次性挂载"会坏掉动画与状态。
2. **不要半路引入第二套宿主**。两套宿主并存会双重渲染、互抢焦点。
   一个窗口只能有一个宿主，这一点在从零重建时是免费的，别在中期自己破坏它。

**附带收益**：上游两个已登记的缺陷在这一层顺手就不存在了——
通知**时长**在上游是硬编码且宿主无入口，而自建库的 `Toast` 把停留时长做成公开可参数化入口；
**溢出语义**上游只是"不再渲染"、被挤掉的条目计时照走，而自建库的 `Toaster` 是**永久移除最旧那条**，
与产品规格（macOS 侧 `WorkbenchNotificationFeatureModel` 的"超出上限丢最旧"）同义。
每一项都要补一条专项测试钉住，否则等于换了实现却没钉住规格。

### 5. 界面规格真源是 IDEA，验收取并集

**决定**：尺寸、布局、观感、交互行为的权威来源是 **IDEA Community**；
各端现有实现只是它的近似。允许做得比 IDEA 更好，但**必须在对应区域的"优化"栏写明理由**。

**开发者要做什么**：改共享控件（下拉、树行、工具栏图标、搜索框等）之前，先查对应 IDEA
Community 的 action / control / theme 源，并把来源路径与版本记进持有该控件的 Note。
这条 `develop-lithe` 里已有同向要求，本 Note 只是把"规格真源是谁"讲明。

**一个必须补的口径**：`shared/platform-feature-matrix/features/` 下的能力矩阵（当前 149 条）
**只有 macOS 与 Windows 两列**。所以"**IDEA 有、两端都还没有**"的能力今天在矩阵里表达不出来，
而按上文它恰恰是规格的一部分。要么给矩阵补一列规格来源，要么另立一份规格清单；
不补的话，"全部移植"就没有可验收的完整口径。

**并集而不是单端**：只按 macOS 复刻会直接丢掉一批**Windows 已实现、macOS 缺或部分**的能力
（例如 AI 对话历史 / 供应商 / 会话三条、远程工作区、Vim 模式、docker compose、大纲符号等）。
这些必须以并集进入规格。

### 6. 尺寸与缩放：唯一锚点是主题基准字号

**决定**：不给不同平台写不同的像素常量。

**机制**：GPUI 的 `px()` 是**逻辑像素**（与 DPI 无关，缩放已经自动生效）。
而 GPUI Kit 的 Coding Guides 明确：窗口根的渲染会把 `theme.font_size` 设为 rem 基准，
所以**主题基准字号就是整个应用缩放比例的参考长度**，其余尺寸都应写成相对档位
（`p_2()` / `gap_3()` / `text_sm()` …），让文字、间距、控件、图标**一起**响应。
指南原文同时警告：具名档位应共享一个相对基准，**而不是变成互不相关的像素常量**。

**正确做法**：要整体缩放大小时，改**主题基准字号这一处**；
档位外的值用 `rems(值 / 16.)` 并注明数字来源；确实需要像素值时用窗口的 rem 尺寸换算。

**不要这样做**：不要在组件里按平台写 `cfg` 分支给不同像素值——那会造出第二份真源，
并让尺寸守卫失去判据。真有**实测**证明某处平台不同时，落点是**一张带注释与来源的平台覆盖表**，
并且要写明"为什么这部分不该跟随缩放"。

**从零重建时要顺手立好的两条**：

- 尺寸守卫要**一开始就建立**，并且把第一方组件 crate 纳入扫描范围（这正是把它单独立 crate 的目的）。
  既有分支的守卫引用了一份已经不存在的规范来源路径与过期行号，重建时不要再抄那份引用。
- 守卫要求"新写的第一方组件放独立 crate，不要挂进被排除的目录"——从零重建正好不必再补救。

### 7. 编辑器：编辑平面可以借底座，模型层必须自建

**决定**：编辑器的**编辑平面**（高亮、行号、折叠、多光标、查找替换、输入法、装饰、
诊断/补全/悬停等 UI 接缝）用组件底座的编辑器组件；**模型层自建**。

**模型层要自己做的清单**（这些原本由 Monaco 承担，换掉就得重新赚回来）：
带版本号的增量编辑与外部替换、undo/redo 分支与分组、差异视图、注释与行移动类命令、
智能选区、图片粘贴、保存冲突与磁盘回读核对、只读策略，以及**真正的 LSP 客户端布线**
（"编辑器有 LSP 接缝"不等于"有 LSP 客户端"——两侧要分开验证）。

**WebView 兜底只能当降风险项**：内嵌 WebView 的方案目前只在 macOS 与 Windows 有受支持路径，
Linux 那条还没发布，且它画在 GPUI 之上、浮层盖不住它。所以
**"顶不住就内嵌 Monaco"只适用于 macOS/Windows，不能当作 Linux 的逃生舱。**

### 8. 平台：三端证据分级，鸿蒙只留缝不承诺

| 平台 | 现状（证据分级） |
| --- | --- |
| Windows | **最强**：纯 Win32 + D3D11 + DirectComposition，文字走 DirectWrite，输入法真的实现了；已知缺口是**没有证据支持内嵌 WebView / 子窗口**，而现有 Windows 产品正是 React+Tauri |
| Linux | **可行但有边界**：Wayland 与 X11 都默认开，渲染走 wgpu → Vulkan 1.3（**没有证据支持无独显 / 软件渲染路径**），文字走 cosmic-text + fontconfig，输入法两条路都在但近一年多都修过；**中文输入法与字体回退是这一端最该先验的点** |
| macOS | 平台层本身齐（AppKit + Metal + CoreText），但**本仓库的 GPUI 代码从未在这上面构建过**（从零重建后依然如此） |
| 鸿蒙 PC | 见下 |

**鸿蒙的结论要写清楚，避免以后误传**：

- **Rust 核心上鸿蒙没问题**：`aarch64` / `armv7` / `x86_64-unknown-linux-ohos` 是 Rust 官方
  **Tier 2（带宿主工具）**目标；经 NAPI 从 ArkTS 调 Rust 也已经被验证过。
  本仓库的 `lithe-core` 本来就产出 `staticlib` / `cdylib`，所以"另一个前端经同一 C ABI 消费 Core"
  在接缝层面是既有能力。
- **但 GPUI 上鸿蒙不是"再编一个目标"**：GPU 渲染只是框架的一半，另一半是**平台层**
  （申请窗口、拿到绑定窗口的 GPU surface、收输入与输入法事件），这一层是按操作系统各写一份的。
  鸿蒙受认可的 UI 层是 **ArkTS/ArkUI**，原生 GPU 代码只能作为 **XComponent 里的客人**——
  也就是说 **ArkTS 薄壳省不掉**，而且还要为 GPUI 写一个鸿蒙后端。
- 全生态目前只有一个原型在做这件事，**规模极小**（几个提交、一个作者、刚建不久），
  且依赖一个个人 fork。

**决定**：鸿蒙排在后面，**不阻塞主线，也不承诺**。今天只守一条纪律：
**平台层保持可替换**，不要把它和某一端的写法绑死。这样将来要接鸿蒙时，
问题收窄成"给平台层加一个后端"，而不是"推翻整套界面代码"。

### 9. `lithe-core` 冻结与被阻塞的能力

**决定**：`rust/lithe-core/` 是唯一后端真源，**本方向默认不改**；它缺的能力由 `main` 侧切片补，
本方向只上报。

> **一次明确的例外（2026-10-08，维护者指示）**：`tree-sitter` 由 `"0.25"` 升到 `"0.26"`，
> 并配一处必需的代码适配（`rust/lithe-core/src/languages/mybatis.rs`：0.26 把 `Node::child()`
> 的索引从 `usize` 改成 `u32`，循环改成 `(0..node.child_count() as u32)`）。
>
> **为什么必须动 core**：`tree-sitter` 带 `links = "tree-sitter"`，同一个依赖图不允许两份同名
> 原生库；界面层的语法高亮需要 0.26 的语法包，而 core 钉着 0.25。Cargo **没有**"消费者覆盖
> 另一个 crate 的版本要求"的机制（`[patch]` 只能替换成满足原要求的版本），所以只要界面层与
> core 同图，这个版本就只能由 core 侧对齐。既有 GPUI 分支早已这么做，且 JS 侧 `windows/tauri`
> 的 `web-tree-sitter` 也在 `^0.26.9` 线上。
>
> **验证与未决**：core 编译通过、跑完 800 条（742 通过；58 条失败全为环境类——46 条 git 并行
> 租约冲突，单跑即通过；12 条 LSP 建目录被拒 `os error 5`）；受影响的 `languages` 过滤集
> 170 通过 / 1 失败（属 LSP 权限类）。**"58 条是既有的"未能建立基线**：把 core 退回 0.25 后
> `tree-sitter v0.25.10` 的构建脚本直接失败 ⇒ 这一点记 **unverified**。
> **待办**：这次改动落在本分支上，需**镜像回 `main`**（单独分支 + PR），否则下次从 main
> 同步 core 会冲突。除这一处外，"core 冻结、缺口走 main 侧补"的口径不变。

**为什么必须显式写出来**：有若干能力的前置在 Core 侧，界面无论怎么写都补不上。
这些项要么在界面上不画入口，要么画出**可解释的状态**——**不许画假按钮、不许点了没反应、
不许静默失败**（"降级可见"是本仓库既有纪律）。

需要登记的是"**等哪个 Core 命令**"，例如：扩展目录列举、Git 写入的排除模式、
Search Everywhere 的按类型分段与相关度、替换的**应用**命令、文件级复制/移动/删除。
每一项都要写下解锁条件，前端先交付不依赖它的部分。

**验收口径的连带要求**：某个共享 crate 现有的一批红测试正是上述 Core 缺口造成的，
在 main 侧修好之前**不会转绿**。所以该 crate 的验收只能写成"既有红只剩这些、且与本轮改动无关"，
**不得声称全绿**。

### 10. 命名与落点

- **命名定案（2026-10-08）**：组件库 = **`lithe-ui`**（`rust/lithe-ui`，独立 workspace）；
  界面层 = **`lithe-gpui`**（`rust/lithe-gpui`，宿主 crate 的落点，从零重写）。两者平级、
  各自独立 workspace。既有布局里"界面层目录"与"组件库 crate"同名，是这次混乱的根源。
- **起点用独立 workspace**：`rust/lithe-gpui/Cargo.toml` 自己当 workspace 根（成员取通配 `crates/*`），
  **不并入 `rust/`**。理由是分片与互不干扰：本分支上 macOS 前端仍通过 C ABI 链接 `lithe-core`
  的 staticlib，Core 的 release profile 仍按"体积受控、供别的进程链接的库"来定；而并入会强制把
  `rust/` 的 resolver 从 2 升到 3、改变 Core 的 feature 合并策略，并让 GPUI 的大依赖树污染
  Core 的 `Cargo.lock`。**合并是一次单独的、可独立验证的片**，不在起点自检里做。
- 组件库是**一个 crate**：Ely 派生内容与 Lithe 自有组件（项目树等）都在 `rust/lithe-ui` 里，不拆两个。
- 界面层的第一个宿主 crate 落地时，在 `rust/lithe-gpui/` 建 workspace 根（`members = ["crates/*"]`、
  `resolver = "3"`）。**在此之前那里只有一份 README**——空成员的 workspace 不合法
  （`cargo metadata` 直接报错），所以不留空壳结构。
- **它会被尺寸守卫扫描**，所以从第一天就必须是相对档位写法。
- 需要补一行所有权登记：仓库所有权与共享边界的 Note，以及 `develop-lithe` 的所有权表。

### 11. 从既有分支继承什么、不继承什么

从零重建**不等于把经验也丢掉**。既有分支最贵的产出不是代码，而是那份逐条踩过的坑
（`rust/lithe-gpui/BLOCKERS.md`）。重建前先按"是否与框架/组件库绑定"分类处置：

| 处置 | 条目类型 | 重建时要做的动作 |
| --- | --- | --- |
| **继承（与框架无关）** | Core 侧缺口、产品规格裁决、真源对齐问题 | 直接沿用结论：Core 缺口继续走 main 侧补；"无真源、不做"与"占位保持不动"这类裁决照旧 |
| **继承为风险（属 gpui 层）** | 文字渲染类：行盒小于字体升降部时裁掉字顶 | 新组件同样会遇到，写组件时按既有定案（不叠加裁剪、用省略号 + 最小行高） |
| **不再适用（属 `gpui-kit` 或那份宿主实现）** | 通知上限与时长无入口、按钮图标档位与盒子尺寸不匹配、活动栏图标被裁切 | 不要照搬：这些是上游组件的内部限制，自建组件自己定档位与入口 |
| **不继承代码，只继承教训** | 大工作区下项目树被行预算吃光（深度优先把预算耗在第一个大目录） | 重建时直接按广度优先建行，不要重犯 |
| **待查** | 编辑器断点列能否排在行号列右侧 | 取决于自建库编辑器是否开放 gutter 列宽，需实测后定 |

**要照抄的不是结论口号，而是"判据机器化"这个习惯**：既有分支把尺寸规范做成了守卫脚本，
把"哪些做不了"做成了登记册。重建时这两样都要有。

### 12. 窗口根宿主（门 G1）：机制与第一片

**决定**：自建窗口根宿主叫 **`WindowRoot`**，放在组件库 `rust/lithe-ui/src/shell/host/`；
**不叫 `Root`**——上游 gpui-base 的根类型同名，迁移期两套并存时同名会让"还剩几处旧根"grep 不出来。

**机制：不必重写浮层。** 组件库的 `primitives::raise(id, child).with_priority(n)` 已经是
Dialog / Toast / Menu / Popover / Spotlight / Lightbox 的**统一挂载机制**（排序与绘制交给 gpui 的
`Window::defer_draw`）。宿主只是"每帧渲染的实体 + 有序层栈 + `Entity<Toaster>` + 窗口级入口"。

**两条硬约束**（已写进代码注释）：

1. **浮层元素每帧重建**：`primitives::layer` 在 prepaint 时 `child.take()` 消费元素，
   把 `AnyElement` 缓存起来复用，第二帧必 panic ⇒ `Overlay::build` 是
   `Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>`。
2. **一个窗口一个宿主**，整片切换、不逐控件混用。

**窗口级入口返回 `bool`，不照抄上游的 `expect`**：上游 `WindowExt` 在窗口没挂根时直接 panic，
而本仓库多窗口路径确实存在没挂宿主的窗口（见下面发现 ①）。没挂宿主 / 没有浮层时返回 `false`，
调用方走降级表现（降级可见）。

**第一片已落地（2026-10-08）**：`src/shell/host/{mod,root,ext}.rs`（`WindowRoot`、`WindowRootExt`、
优先级档位），并把 `feedback::toast` 的同屏上限由 4 改为 **3**（与真源对齐——换了实现必须钉住规格）。
验证：`cargo check -p lithe-ui --all-targets` 通过。

**G1 侦察发现的三个既有问题（都还没修，按序登记）**：

1. **多窗口路径现在就缺窗口根**：`app/src/main.rs` 里"在新窗口打开工程"那条 `cx.open_window`
   返回的是裸 `ShellWorkspace`，**没有包 `Root`** ⇒ 那个窗口压根没有浮层宿主。
2. **尺寸守卫对组件库是"失明"，不是"按目录排除"**：`tools/check-ui-px.mjs` 只扫
   `rust/lithe-gpui/crates/*/src`，而 `rust/lithe-ui/` 是另一个 workspace、根本没进扫描范围；
   它排除的 crate 目录（`crates/lithe-gpui/`）也已不存在。守卫需要一次独立重做
   （建议把排除判据改成"由 `UPSTREAM.md` 声明的派生目录"）。
3. **rem 缩放锚点在组件库不存在**：`Theme` 没有 `font_size` 字段，全仓无人调 `set_rem_size`
   ⇒ "主题基准字号是唯一缩放锚点"这条规范目前**没有落点**，需要单独一片。

**已完成（2026-10-08，`d4423274`）**：窗口根整片切过来了——`app/src/main.rs` 三条路径、
`workbench/src/workspace.rs`（含 `replace_root_with_workspace` 的换根闭包）、
`search_everywhere.rs` 与 `tests/common/mod.rs` 的测试收口，以及全部经上游 `WindowExt` 的
对话框 / 通知调用点（约 60 处，改走 `WindowRootExt` 的同形入口）。

**G1 验收证据**：`cargo check --workspace` = **0 error / 3 条既有警告**；
`cargo test -p lithe-ui --features test-support` = **1558 passed / 0 failed**，其中宿主新增的
5 条行为测试全过（单根、Esc 关层、显式关闭只跑一次 `on_close`、关闭后焦点归还、
没挂宿主时**报告而不是 panic**）；两个 workspace 的 `cargo fmt --check` 均 exit 0。

**两个结构缺口已补（2026-10-08）**：

- **rem 缩放锚点**：组件库 `theme::Theme` 新增 `font_size: f32`（基准字号），`base_rem()` 由硬编码
  `px(16.0)` 改为读它；`tokens` 不再各自乘 `font_scale`（此前它与 `ZoomControl` 改窗口 rem 大小是
  **两套叠加的缩放**）；`Theme::update` 统一把 `font_size × font_scale` 标到所有窗口的 rem 大小，
  新增 `Theme::apply_rem` 作为唯一入口；`ZoomControl` 改为把百分比写进主题；`WindowRoot` 挂载时继承
  当前缩放。**并发现并接掉一处第二真源**：仓库里有**两个 `Theme`**——宿主自己的（`font_size: Pixels`）
  与组件库的，宿主的 UI 字号设置原先只写自己那份、不会流到库的锚点；现在 `settings::theme::
  apply_theme_font_size` 会把同一个值同步进组件库（带 `has_global` 守卫）。
  验收：`cargo test -p lithe-ui --features test-support` = 1558 passed / 0 failed；
  `cargo check --workspace --all-targets` = 0 error。
- **尺寸守卫对组件库失明**：守卫本来就是**红的**（自检抓到被排除的 `crates/lithe-gpui` 已不存在），
  不只是盲。已改为**按声明判定派生边界**（`DERIVED_ROOTS` 必须带 `UPSTREAM.md` 作证据），并把
  组件库里属于我们自己的目录纳入扫描（`FIRST_PARTY_IN_DERIVED`：`lithe-ui/src/shell/host`、
  `lithe-ui/src/lists/project`）。两条不变式任一不成立即判失败，防止覆盖静默失效。
  验收：`--selftest` 与实扫均 exit 0。

**仍未做（下一阶段）**：其余 9 个宿主 crate 的组件仍走 gpui-kit（组件重写批，剩余 369 行）。

规模（grep 实测）：`open_dialog` 19、`close_dialog` 23、`replace_root` 4、`Root::new` 3、
`push_notification` 1、`WindowExt` 引用 14 ⇒ 约 **60 处**，分布在 `workbench`(41)、`editor`(8)、
`settings`(7)、`git`(4)。

迁移形态：在 `WindowRootExt` 上给出与旧调用点**同形**的入口（`open_dialog(id, build)` /
`close_dialog()`），调用点只改 trait 导入与构造闭包。⚠️ 新旧两个 trait 的同名方法同时进作用域
会直接编译歧义，所以**只能在换根那一片里一次改完**，不能分批。
**组件重写批的第一块路障（2026-10-08 实测）**：宿主代码大量直接读 `cx.theme().X`，而这些取用口来自上游
gpui-component 的 Theme，与组件库的 Theme **不同名、不同形**。以最小的 `run` 试点即一次报出 22 个错：

- 缺 `mono_font_family`（组件库字段叫 `mono_family`）；
- 缺扁平的**颜色取用**：`primary` / `foreground` / `muted_foreground` / `border` / `danger` / `warning` /
  `accent`——组件库把它们收在 `colors: Palette` 里（`cx.theme().colors.primary` 之类）。

⇒ 组件重写批应当以「**主题取用对齐**」开场：给组件库 Theme 补上游同名的取用口（或在宿主侧改为
`colors.*` 取用），否则每个宿主 crate 一动手就先撞这一面。另：`base::h_flex/v_flex` → `layout::h_stack/v_stack`
与 `ButtonVariants::ghost/small` → `Button::variant(ButtonVariant::Ghost)/size(Size::Small)` 已确认是**直接对应**，
`Button::icon` 的元素形态已在组件库补上 `icon_element`。
#### 主题取用对齐：宿主名 → 组件库槽（改写片按此表执行）

上游 `gpui-component` 的 `Theme` 是 `colors: ThemeColor` + **`Deref`**，所以宿主写的是扁平
`theme.foreground`（自动 deref 而来）。组件库的 `Palette` 命名不同，且**不打算为同一概念再平铺一份**
（那是第二真源）。逐名映射如下（依据是宿主**实际用法**，不是名字相似）：

| 宿主（gpui-kit）名 | 用到的次数 | 组件库的落点 | 依据 |
| --- | --- | --- | --- |
| `muted_foreground` / `foreground` | 182 / 73 | `colors.fg_muted` / `colors.fg` | 同名概念 |
| `border` | 58 | `colors.border`（强调处用 `border_strong`） | 同名概念 |
| `danger` / `success` / `warning` / `info` | 36/11/7/4 | 同名槽 `colors.danger` 等 | 同名概念 |
| `background` | 22 | `colors.bg` | 同名概念 |
| `accent` | 16 | `colors.accent` | 同名概念 |
| `primary` | 14 | `colors.selection`（作选中行/选中块底）或 `colors.accent`（品牌交互） | 按**用法**分：`bg(primary)`、`primary.opacity(0.28)` 都在画选中底 |
| `muted` | 5 | `colors.sunken` | 用法的"下沉面" |
| `popover` | 4 | `colors.overlay` | 浮层底 |
| `secondary` | 2 | `colors.surface` | 用法是"非活动标签底" |
| `list_hover` | 2 | `colors.hover` | 同一概念，库已按列表行命名 |
| `ring` | 1 | `colors.focus` | 焦点环 |
| `sidebar_accent` | 1 | `colors.selection` | 侧栏选中底 |
| `tab_bar` / `title_bar` | 13 / 2 | `colors.surface`（chrome 表面） | 两者都只作 `bg(…)`；库的 `shell::tabs`/`chrome` 画同一片表面 |
| `font_size` | 23 | `Theme::font_size`（`f32`）/ `font_size_px()` | 同一锚点，只是类型；`rem_px(base, x)` 用后者 |
| `mono_font_family` / `mono_font_size` | 11 / 0 | `Theme::mono_family` / `mono_font_size`（本轮补） | 字段名不同 / 宿主只写不读 |

⚠️ **unverified（需要在改写 `tab_bar`/`title_bar` 时核实）**：若宿主内置的 JetBrains 主题 JSON 给
`tab_bar`/`title_bar` 设了**与 surface 不同**的值，则映射到 `colors.surface` 会丢失那份差异，
届时组件库需要新增一个 chrome 表面槽。核对方式是读主题 JSON 的键与值，不要按名字猜。

**已落地的库侧增量（①a）**：`mono_font_size: f32` + `font_size_px()` / `mono_font_size_px()`
（纯换算，不新增真源）。颜色槽**一个都没加**——宿主所有颜色读点都能落到既有槽上，按上表改名即可。
### 13. `shared` 迁移完成（2026-10-08）与它暴露的三个缺口

**已落地**：`shared` 整片摘掉 gpui-kit（Cargo.toml 改依赖；`Empty*` 改走组件库 `EmptyState` 的槽位入口；
`IconName`/`Icon`/`Theme` 换库类型；`v_flex`→`v_stack`）。组件库侧新增
`EmptyState::{slots, title, media_element, content}` 与 `primitives::Icon` 的 `Styled`（均记入 UPSTREAM.md）。
验收（主会话独立复跑）：`cargo check --workspace --all-targets` = 0 error；
`cargo test -p lithe-ui --features test-support` = **1560 passed / 0 failed**；两个 workspace
`cargo fmt --check` = exit 0；px 守卫 = exit 0。

**① `app` 的资源源缺口（会静默画空白，下一片优先修）**：宿主 `LitheAssets` = 宿主 `assets/**` +
`GpuiKitFallbackIcons`（106 个名字），**没挂组件库的 `lithe_ui::Assets`（307 个图标）**；而
`lithe_ui::init` 只探一个 `icons/check.svg`（恰好被上游回落命中）就通过 ⇒ 组件库任一不在这 106 个名字里的
图标在宿主里**画不出来**（例如 `FileJson`）。修法是挂上组件库资源源，并让探针不再可能被巧合命中。

**② 图标名字缺 31 个**：宿主用到 73 个图标名，组件库图标集**缺 31 个**（如 `FolderClosed`）。
`explorer` 那一片必须先定"补名字 + 补 SVG 到组件库资产"还是"换名映射"。

**③ 过渡桥（临时，登记待删）**：`shared` 迁移后，宿主上游菜单槽位仍只收 `gpui_kit::component::Icon`
（`PopupMenuItem::icon(impl Into<gpui_kit::Icon>)`），元素进不去。为不停在"无等价物"，在
`crates/workbench/src/icon_kit.rs`（新）与 `crates/editor/src/editor_view.rs`、`crates/settings/src/dialog.rs`
各留一处**同名过渡桥**，把 IDEA 图标包成上游 `Icon`：调用点 0 改动、渲染与迁移前逐字节相同。
**菜单整片迁到 `lithe_ui::menus` 时，三处桥与 `shared` 的元素版一并收口、删桥。**

**两处可见变化（已登记，待真机或方案确认）**：

- `.json` 的图标由 `FileBraces` 变 **`Braces`**：组件库没有 `FileBraces`，而 `FileJson` 的 SVG 不在宿主
  资源源里（会画空白）。①修好后应改回 `FileJson` 或给库补 `FileBraces`。
- `shared::unavailable` 版面**归一化到组件库档位**（标题/说明尺寸与间距走库档位；说明不再叠
  `.opacity(0.72)`；随之失去读者的 `UNAVAILABLE_DESCRIPTION_OPACITY` 常量已删）。**未真机对照**；
  若不接受，需要给库的说明槽位补元素/透明度入口。

**`shared` 的 22 条既有红测试（与本片无关，但按 §9 口径必须登记）**：
`cargo test -p lithe-gpui-shared` **在起点即红**（50 passed / 22 failed）：6 条 `icons::file_icon::*` 失败于
数据文件 `assets/icons/idea/extension.json` 是 macOS 图标副本（旧清单期望 128 个定义与
`icons/files/*_dark.svg`），16 条 `workspace_config::*` 失败于 Core 拒绝 Git 写操作
（`Unsupported Git write operation`）。⇒ 该 crate 的验收只能写"**既有红只剩这些、且与本轮改动无关**"，
**不得声称全绿**。
#### 图标口径修正（维护者 2026-10-08）：应用图标只用**我们自己的**图标集

**规则**：应用图标一律取自本仓库自己的图标集（`rust/lithe-gpui/assets/icons/idea/`，实测 **302 个文件**，
含 `fileTypes/json.svg` 等）；**我们自己没有那个图标就空着**，**不用 UI 库（gpui-kit 或 lithe-ui）的图标来顶**。

**据此作废与修正本稿先前两处结论**：

- ❌ **作废 ①**（"把 `lithe_ui::Assets`（307 图标）挂进宿主资源源"）：不挂。
- ❌ **作废 ②**（"宿主 73 个图标名在库图标集里缺 31 个，需补名字或换名映射"）：判据换成"**我们自己的图标集里有没有**"，
  没有就空着；不再以库的图标集为参照。
- ⚠️ **`GpuiKitFallbackIcons`（106 个 gpui-kit 图标名，挂在 `crates/app/src/assets.rs`）= "用 UI 库图标"的兜底机制**，
  按本口径应移除；移除前要确认它当前**是否还在替上游组件自身的内部图标（对话框关闭、勾选、微调钮等）兜底**——
  若是，则它属于"上游组件自己的渲染素材"，要么随组件被替换而消失，要么作为**限期拆除的过渡垫片**登记。
- ⚠️ **回归修正**：`shared/src/icons/file_icon.rs` 把 `.json` 映射成库的 `Braces`（理由"宿主资源源没有 `file-json.svg`"）
  属于**用库图标顶替我们的**，且**宿主的 `fileTypes/json.svg` 本就在**。应改回**我们自己的**图标。

**受影响规模（下一步的工作量证据）**：宿主里仍以**库的 `IconName::` 枚举**命名的调用点共 **127 处**
（workbench 84 / shared 17 / git 10 / explorer 6 / settings 5 / terminal 3 / app 1 / editor 1）。
各 crate 的迁移片都要把其中属于**应用图标**的改成我们自己的图标（`shared::icons::idea::*` 或我们的 SVG 路径），
我们没有的**留空**；只有**上游组件自身的内部装饰图标**才继续由上游绘制。
#### 图标口径的落地结果与第 5 步清单的拍板（2026-10-08）

**已落地**：`shared` 的 `lucide_fallback() -> IconName`（按扩展名返回**库**图标名，即"用库图标顶替"的机制本体）
改为 `fallback_icon() -> Option<AppIcon>`；新增 `AppIcon` **只装本仓路径**（装不下库的 `IconName` ⇒ 该行为在类型上
不可表达）；`FileIcon::lucide` → `FileIcon::own`，渲染三分支（清单图 → 我方兜底图 → **留空占同位**）。
`.json` 从库的 `Braces` **改回我们自己的** `icons/idea/fileTypes/json.svg`。`extension.json` 43 → 59 条定义、
悬空引用 10 → 0（根因：同步工具 `referencedDefinitions` 不数 `filenames`，误裁 maven/docker/class 等定义），
工具一并修正。`shared` 的 6 条图标红转绿（50/22 → **61/16**，余 16 条与起点同名、属 Core 拒 git 写的环境问题）。

**清单（专家只出数据）——库组件自身的内部装饰图标**：`Dialog` 关闭 X、`Toast`/消息条关闭 X 与严重度图标、
菜单 Check / 子菜单 ChevronRight / 下拉 ChevronDown / Ellipsis / Search、`DockPanel::plain` 的 Square、
分栏组头的 PanelRight/PanelBottom/X —— **今天全靠 `GpuiKitFallbackIcons` 命中**；而 `CircleAlert`、菜单单选 `Dot`、
`shell/status`、`tooltip`、`files` 等处若干图标**今天就已经空白**。若既不挂 `lithe_ui::Assets` 又去掉
`GpuiKitFallbackIcons`，上述会**静默**变空（`Icon` 的 bundled 分支跳过资源校验，不 panic）。

**拍板（依据：宿主资产自持 + 本仓自有真源 + 两个命名空间分离）**：

1. **应用图标**：只取 `assets/icons/idea/**`（已由 `AppIcon` 在类型上保证），缺失留空 —— 不动。
2. **库组件自身的内部装饰图标**：属本仓组件库**自己的**渲染素材（`rust/lithe-ui` 是本仓自有的 crate，非外部上游）
   ⇒ **挂 `lithe_ui::Assets`** 让库自己渲染。这不是"借库顶替宿主资产"——两者命名空间已由 `AppIcon` 隔开。
3. **`GpuiKitFallbackIcons`（外部 gpui-kit 的图标）**：在 2 落地后**摘掉**；当前保留为登记在案的限期垫片。
4. 把 `lithe_ui::init` 里**会被巧合命中的探针**换成真探针，否则"资源源已挂"无法自证。
5. 2/3 落地后复核清单里"今天就已空白"的那批是否被 `lithe_ui::Assets` 覆盖；仍未覆盖的按组件库规矩补其自有素材。
### 14. `app` 入口片：资产源换成组件库自带的 `lithe_ui::Assets`（2026-10-08）

**已落地**：

- `crates/app/src/assets.rs` 删掉上游 `gpui_kit::assets::icon_assets!(GpuiKitFallbackIcons, [106 个名字])`
  过渡垫片，回落源改为 **`lithe_ui::Assets`**（本仓组件库 crate 自带的资源源）——组件库组件自身的内部装饰
  图标（Dialog/Toast 的关闭 X、菜单 Check 与箭头、Severity 图标…）改由**本仓自有**素材提供，不再借外部
  gpui-kit 的图标名。`fallback_count()` 改名 `library_icon_count()`。
- 新增测试 `every_library_icon_the_host_draws_is_reachable`：逐个断言 `lithe_ui::primitives::IconName::ALL`
  的路径都能从 `LitheAssets` 取到非空字节 —— 比原来"106 个名字存在"更强，且覆盖到此前**今天就已空白**的那些。
- 入口 `gpui_kit::application()` → **`gpui_platform::application()`**（上游 gpui-kit 本身就是
  `pub use ::gpui_platform::application;`，见 `gpui-kit/src/lib.rs:168`）；app 新增与组件库**同一条**钉版依赖
  `gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8", features = ["font-kit"] }`。
- `tools/sync-gpui-kit-icons.mjs` **退役**（它维护的正是被拆掉的垫片名单）：改为显式 `console.error` + exit 1，
  不静默失效、不留陷阱。

**`app` 仍有 4 处 gpui-kit，归属已定（本片不动）**：

| 剩余 | 归属/前置 |
| --- | --- |
| `gpui_kit::init(cx)` | 要等**最后一个 gpui-kit 组件退场**（它初始化的是 gpui-kit 组件自身） |
| `Theme::change(ThemeMode::Dark, ..)` + `Theme::set_scrollbar_mode(..)` | **主题迁移片 ②**（宿主主题降为生产者；组件库没有 scrollbar mode 对等 API，需补库能力或映射） |
| `use gpui_kit::component::{Theme, ThemeMode, TitleBar}` 里的 `TitleBar::window_options()`（`main.rs:640,681`） | 标题栏/主题簇；**先查**组件库 `shell::chrome::TitleBar` 有无对等 API，无则按"补进库或映射最接近 API" |
| `gpui_kit::component::set_locale(&locale)` | 它服务的是 **gpui-kit 组件自己的字符串**（我们自己的 `tr()` 走本仓 i18n）；两者要一起退出，否则 gpui-kit 组件文案会掉回默认语言 |

**验收**：`cargo test -p lithe-gpui-app` = 11 passed / 0 failed；
`cargo check --workspace --all-targets` = 0 error；`cargo fmt --check` 与 px 守卫 = exit 0。
## 考虑过的备选方案

**备选一：复用 `feat/lithe-gpui-components` 分支的既有资产（445 个提交、13 个 crate、
已跑通的 Windows 构建、设置 8 页、项目树、Java 会话、通知）。** 表面上最省时间。
但那条线的代码是围绕 `gpui-kit` 长出来的：窗口根接的是上游 `Root`、主题走一层映射桥、
组件调用散在两套词汇里。而本文的骨架是"不引入 gpui-kit、浮层从第一天自建"。
继续用它等于**先把要拆的东西接上，再拆一遍**。
否决：宿主从零重建、只继承结论与教训；**但组件底座例外**——它本身是 Ely 的 fork，
按 §3 复用（含已对齐 `gpui-kit` 的 API 重构与真 bug 修复），因为它不属于"宿主实现"。

**备选二：长期锁定 `gpui-kit` 当组件层。** 它活跃、公司背书、许可干净、覆盖 Dock/表格/菜单/
对话框/输入/图表/无障碍。但它的组件 API 词汇与既有代码不同，锁定它就要回答"两套词汇怎么并存"；
而且它**不提供终端与差异视图**。否决：与自维护方向冲突，且缺件要另找来源。

**备选三：直接用官方 crates.io 上的 `gpui`（0.2.2）。** 许可干净、是官方。但它停更约 11.5 个月，
且今天的 Windows（D3D11）与 Linux（Wayland + wgpu）平台层是 Zed 后来才做的、从未发布到 crates.io。
否决：拿到的是一个一年前的旧架构。

**备选四：复用 Zed 的编辑器与组件（`editor`、`ui`、`rope`…）。** 表面上最省事。但逐个 crate 查过：
它们都是 **GPL-3.0-or-later** 且没有发布到 crates.io。即使忽略许可，它们还会拉进
`workspace`/`project`/`client` 等一大串依赖，并引入**第二个 GPUI 实例**（同名库、类型不互通）。
否决：许可直接出局。

**备选五：把 Ely 组件底座的 API 全面对齐 `gpui-kit`。** 调用写法会统一。但既有评估已经算过：
`pub fn new(` 出现在数百个文件里，改定义还要改每个调用点，`IconName` 还与既有 expui 图标体系撞车。
否决：改动量与收益比最差。改为**只对新组件采用 gpui-kit 的约定**。

**备选六：保留 WebView + Monaco 作为主编辑器。** 保住了既有投入，编辑器风险近乎归零。
但 Linux 的受支持路径还没发布，而且这与"摆脱 WebView、做原生 UI"的目标相反。
否决：仅保留为 **macOS/Windows 的降风险兜底**，不作 Linux 或鸿蒙的逃生舱。

**备选七：干脆统一到 Web 前端（macOS 也跑 Tauri/React）。** 界面代码复用最大。
但 macOS 会从原生 SwiftUI 退化成 WebView 应用，且仍然依赖 WebView。否决：体验退步。

**备选八：换用其它 GPUI 发行版或组件套件**（社区 fork、其它单维护者套件）。
逐个体检过：或停更、或有 yank、或只有几周历史、或没有 Dock/树/编辑器深度。
否决：风险不低于现状，收益不明确。

**备选九：完全从零手写组件库（连组件底座也不用）。** 产出可控、测试自己写。
但既有记录显示这条路两个迭代只覆盖到 13 章，而目标规模是 44 章，差距过大。
否决：vendored 组件底座已经解决过这个问题。

## 验收标准

本 Note 只定选型与边界，**不是排期**。它是否成立，取决于下面几个阶段门能不能过；
每道门都要有可复现的证据，不能只写"支持"。

1. **选型本身的可核验性**：本 Note 通过 `node scripts/verify-agent-notes.mjs`；
   四条硬事实（官方 gpui 停更、Zed 编辑栈许可、`gpui-pre` 发布者与无公开仓库、`gpui-kit` 活跃度）
   都能用 crates.io / GitHub 元数据在几分钟内复核。
2. **起点自检**：按 §2 钉的 `=0.3.8` 对组件底座做一次类型兼容核对
   （上游用到的 gpui 类型逐个存在）+ `cargo check --all-targets` 零错误零警告；
   核对失败则退到 `=0.3.7` 并记录失败点。
   这一步不做，后面每个编译错误都会被误判成"我们写错了"。
3. **门 G1（最高优先）：自建窗口根宿主从第一天就跑通。** 三件事（开对话框 / 开抽屉 / 推通知）
   都跑通，并证明：浮层在内容之上、遮罩与 Esc 关闭生效、焦点归还正确、动画不坏。
   **这是整套自维护方案唯一的结构性风险点，越早知道越好。**
4. **门 G2：编辑器对等。** 按 §7 的模型层清单逐项给实测结论
   （每项要么"已对等"要么"显式不支持"），不允许只写结论不给证据。
5. **门 G3：Linux 真机。** 构建 + 起窗口 + **中文输入法与字体回退** + 拖动分栏，
   并记录显卡/驱动前提（Vulkan 1.3）与无独显机器上的实际表现。
6. **门 G4：macOS 真机。** 从零重建后，macOS 上必须有一次真实构建与启动。
7. **能力矩阵逐条有结论**：149 条（或补充规格列之后的完整集合）每条要么"已对等"、
   要么"显式不支持"，不允许"看起来能用但走的是本地假实现"。
8. **被阻塞项登记完整**：每项写明解锁条件与界面上的降级表现。
9. **尺寸规范可机检**：尺寸守卫通过，且 `lithe-ui` 在扫描范围内。

## 风险

- **供应链**：框架层由一个人重新打包、无公开仓库、约每周重基。缓解手段只有
  vendor 源码 + 自己持有 Zed 提交号 + 把升级当成有意的重基动作。**这是本方案最大的外部不确定性。**
- **组件底座的上游极年轻**：Ely 上游仓库建于近期，属于"刚出现"的项目——虽然许可干净、
  迭代很快，但它的成熟度不构成保证。缓解：固定提交、改动留在我们自己的来源说明里，
  并保留一条触发条件——**当维护它的成本证明高于自建时，重新评估**。
- **窗口根宿主是唯一必须一次做对的结构性件**：它上面挂着所有浮层。
  从零重建的好处是不必迁移，坏处是**没有可抄的现成实现**，必须一次设计对。
- **编辑器模型层的工作量容易被低估**：Monaco 在本仓库承载过一整套已解决的问题，
  换实现等于重新赚回这些保证。
- **规格真源与验收矩阵不匹配**：真源是 IDEA，但矩阵只有两列。不补口径，
  "全部移植"就无法验收。
- **macOS 与 Linux 目前零构建证据**：这两端的结论今天都是"上游声称可用"，
  不是"我们验过"。不得对外声称已验证。
- **双词汇并存期会漂移**：组件底座的词汇与新组件的词汇同时存在。缓解：写进编码指南，
  并明确"新代码只走新库"。
- **鸿蒙是单原型赌注**：全生态只有一个规模极小的原型。已决定推后，但必须守住
  "平台层可替换"，否则将来要推翻界面代码。
- **并集口径会放大工作量**：取 Windows ∪ macOS 并集意味着要补一批 macOS 侧从未做过的能力，
  这部分不能按"照 macOS 抄一遍"估算。
- **复用组件底座 = 连它没写下来的改动一起继承**：底座的来源说明只记了 2 条改动，
  而实际有 39 个提交改过它。所以起点自检里必须包含"按 `git log` 把改动逐条过一遍"，
  不能只信那份说明文件——否则会继承一堆没人知道来历的行为差异。

## 适用范围

- `rust/`（框架层、组件底座与宿主的落点；其中 `rust/lithe-core/` 本方向不修改）
- `macos/`（界面规格的参考实现）
- `windows/tauri/`（现状对照，属于被替换对象）
- `shared/contracts/`（命令信封与跨端契约，属于兼容面，不得随界面改动而变）
- `shared/platform-feature-matrix/`（验收口径的能力矩阵）
- `.agents/notes/proposed/architecture/`
- `scripts/verify-agent-notes.mjs`
- `AGENTS.md`

> 说明：本 Note 描述的 `crates/lithe-gpui/`、`crates/lithe-ui/` 与框架层源码，当前只存在于
> `feat/lithe-gpui-components` 等工作分支上；本分支（自 `main` 新建）尚无任何 GPUI 代码。
> 因此上面"适用范围"只列本分支真实存在的路径，分支独有的既有决策笔记与登记册
> 按正文以纯文本注明出处，不建立链接。
