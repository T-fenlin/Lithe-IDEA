# GPUI 重写计划：对抗性质询与设计树（GRILL）

> 本文是对 `docs/development/gpui-ui-windows-rewrite-prompt.md`（下称"提示词"）的**一轮对抗性质询**。
> 目的不是确认计划，而是把它压成一张设计树：每个决策分出它下面的子决策，标出**现在就能问的 FRONTIER** 与**必须等 frontier 答案才能问的 DOWNSTREAM**。
>
> **本文只增不改任何实现**。所有结论都带 `文件:行号` 证据；查不到的一律写"未找到"。
>
> 事实基准时刻：2026-09-24 20:31（工作区正在被并发推进，见 §0.3 的"活文档"警告）。

---

## 0. 先说三条会改变整棵树的前提事实

### 0.1 工作区状态与任务书里的描述**不一致**（任务书是过期的）

任务书说 `gpui/Cargo.toml`、`Cargo.lock`、`shell/Cargo.toml`、`shell/src/main.rs` 是"工作区删除（未提交）"。**实测不成立**：

```
git status --porcelain（2026-09-24 20:31:40 实测）
 M gpui/PLAN.md
 M gpui/README.md
 M gpui/UI-MAP.md
 M gpui/shell/src/bin/shell-probe.rs
 D gpui/shell/src/bin/shell_probe/bottom_panel.rs
 D gpui/shell/src/bin/shell_probe/files.rs
 M gpui/shell/src/bin/shell_probe/mod.rs
 D gpui/shell/src/bin/shell_probe/panels.rs
 D gpui/shell/src/bin/shell_probe/stats.rs
 M gpui/shell/src/bin/shell_probe/workspace.rs
?? gpui/BLOCKERS.md
?? gpui/shell/src/bin/shell_probe/shell/
```

四个 build 文件与 `gpui/UI-MAP-macos.md` 都已在工作区里（`gpui/Cargo.toml`、`gpui/Cargo.lock`、`gpui/shell/Cargo.toml`、`gpui/shell/src/main.rs`、`gpui/UI-MAP-macos.md` 用 `Get-ChildItem` 全部命中）。`gpui/BLOCKERS.md` 的 B2 条目自己写着"已从 HEAD 恢复"（`gpui/BLOCKERS.md` B2 行）。**并发写者已经把这些坑填了，任务书没跟上。**

### 0.2 提示词已经被"局部超越"了：`PLAN.md` 多出一节 §5，与提示词并列但口径不同

`gpui/PLAN.md:435-464` 新增了「§5 本轮进行中（2026-09-25：规格来源改回 Windows + 清空重写第 1 轮）」，其中：

- `PLAN.md:441-449` 的新分阶段表**取代了 `PLAN.md:108-122` 的旧表**；
- `PLAN.md:453` 把重试上限从 5 次改成 **3 次**，登记处改到新文件 `gpui/BLOCKERS.md`；
- `PLAN.md:454` 把构建串行化定成"**只有主代理跑构建**";
- `PLAN.md:455` 把底部窗口径定成"**用真 Bottom dock**"；
- `PLAN.md:457` 宣布新文档 `gpui/UI-MAP-WINDOWS.md` 与 `gpui/research/gpui-kit-0.6.6-api.md` 由并行子代理产出。

**这意味着：提示词里的若干"未决项"已经在下游被单方面拍过板了。**质询必须先问"以哪份为准"，否则后面所有问题的前提都是漂的。

### 0.3 这是一份**活的工作区**：我质询期间它正在被改

- 我第一次 `git status`（20:2x）只有 7 条；20:31 再查变成 10 条 + 2 个未跟踪项。
- `gpui/shell/src/bin/shell_probe/shell/mod.rs` 的 mtime 是 20:25:39，`workspace.rs` 是 20:29:30，`gpui/BLOCKERS.md` 是 20:25 前后。
- 后台冷构建已完成：`.artifacts/p0/cold-check.log` 末尾（mtime 20:29:25）是
  `Finished dev profile ... in 5m 58s`，且**带 2 个 warning**（`unused import: AppContext as _`、`unused variable: cx`，都指向 `shell_probe/workspace.rs:17` 与 `:29`）。

**结论：这份质询的目标在移动。**下面每个节点的"现状"我都标了实测时刻；答案落地时要重新核对一次。

### 0.4 规模对账：不是 8.8k 行，是 ≈5.3k 行

任务书说"被删掉的旧实现规模合计约 8.8k 行"。实测 `git diff HEAD --stat`：

```
 gpui/shell/src/bin/shell_probe/bottom_panel.rs | 2384 --------------
 gpui/shell/src/bin/shell_probe/panels.rs       | 1164 ------------
 gpui/shell/src/bin/shell_probe/files.rs        |  169 --
 gpui/shell/src/bin/shell_probe/stats.rs        |   65 -
 gpui/shell/src/bin/shell_probe/workspace.rs    | 1451 +------------
 10 files changed, 73 insertions(+), 5242 deletions(-)
```

`5242` 是"HEAD→工作区"的净删除行数；其中 `workspace.rs` 的 1451 里有一部分是被改写而不是全删。**用"5.2k 行、其中真正可复用的是数据层与外壳"来陈述，比"8.8k 行已跑通代码"更准确**——"已跑通"也是可疑的：`PLAN.md:361` 自述存在"用满 5 次尝试后按规则登记跳过"的未解决问题（提交行文字顶部被裁掉 1/4 行高）。

---

## 1. 设计树

图的读法：`[F]` = FRONTIER（现在就能问，前提已定）；`[D]` = DOWNSTREAM（要等 frontier 答案）。
每个节点给出：**决策 → 可选方案 → 推荐 + 理由 → 证据**。

```
ROOT  R0  清空重写这条路线本身                            [F]
│
├─ A  代码基线
│   ├─ A1 旧实现怎么处置（重写 / 恢复 / 混合）              [F]
│   │   ├─ A1a 恢复哪些、删哪些的边界（"可复用域逻辑"清单）  [D] ← 等 A1
│   │   ├─ A1b 新旧两套入口的命名与共存策略                 [D] ← 等 A1
│   │   └─ A1c 模块拆分口径（无状态渲染函数 vs 每模块一个 Entity） [D] ← 等 A1/A1b
│   └─ A2 工作落在哪个分支（当前 main，且本机没有 preview）  [F]
│
├─ B  规格来源与范围（Windows 前端）
│   ├─ B1 以哪份文档为准：提示词 vs PLAN.md §5              [F]
│   ├─ B2 Windows 前端的哪些层算规格（chrome / Monaco / zustand 本地态） [F]
│   │   ├─ B2a 24 种 buffer 类型与 pane 树要不要在本轮建模   [D] ← 等 B2
│   │   └─ B2b 侧栏/编辑区里"前端本地 store 拥有的事实"清单  [D] ← 等 B2
│   ├─ B3 视图层取舍：哪些允许"做得比 Windows 好"           [D] ← 等 B1/B2
│   └─ B4 玻璃层与窗口透明（默认是关的）                    [F]
│
├─ C  布局与窗口
│   ├─ C1 底部窗归属（center 的 v_split vs DockPlacement::Bottom）[F]
│   │   └─ C1a 右侧工具窗与底部窗的相互遮挡/宽度口径          [D] ← 等 C1
│   ├─ C2 窗口口径（Dodona 94% / Tauri 1200×800 / 最小尺寸）  [F]
│   └─ C3 自绘窗口三键与拖拽/缩放热区                        [D] ← 等 C2
│
├─ D  构建、验收与证据
│   ├─ D1 阶段完成的判据（截图 + 对照 vs "3 次写文档"）      [F]
│   │   ├─ D1a 无法合成鼠标事件时的交互验收清单              [D] ← 等 D1
│   │   └─ D1b 允许写"测试脚手架"（自触发浮层/强制态）到什么程度 [D] ← 等 D1/D1a
│   ├─ D2 "0 error 0 warning" 是否为硬门槛                   [F]
│   ├─ D3 3 次重试的计量口径（什么算一次）                   [F]
│   ├─ D4 证据目录与留存（`.artifacts/` 被忽略、旧证据已删） [F]
│   └─ D5 构建串行化下的并行策略                             [D] ← 等 D2/D3
│
├─ E  数据与后端
│   ├─ E1 第一版命令清单（提示词 `:49` 说 3 条、`:106` 说 4 条）[F]
│   │   ├─ E1a 游标生命周期（nextCursor / historyCursorClose）[D] ← 等 E1
│   │   ├─ E1b repositoryRoot 相对路径归一化                 [D] ← 等 E1
│   │   └─ E1c "非仓库"的两种行为如何表达（无专用错误码）     [D] ← 等 E1
│   ├─ E2 宿主↔Core 的调用形态（只有 JSON 字符串入口）        [F]
│   └─ E3 终端第一版到底做什么                              [F]
│       ├─ E3a 最小 ANSI 处理范围（SGR/CSI/OSC/CR/BS）       [D] ← 等 E3
│       ├─ E3b 尺寸同步（resize/cols/rows）                  [D] ← 等 E3
│       ├─ E3c 输入模型（一个输入行 vs 逐键进 PTY）与 Ctrl+C  [D] ← 等 E3/E3b
│       └─ E3d 上下文菜单/搜索/分屏是否本轮做                [D] ← 等 E3
│
├─ F  组件与图标
│   ├─ F1 图标集：默认 101 vs 全量 1830                      [F]
│   │   └─ F1a 缺字形登记表落在哪                            [D] ← 等 F1/B1
│   └─ F2 密度档与组件像素常量覆盖清单                       [D] ← 等 B2
│
├─ G  文档与命名
│   ├─ G1 `UI-MAP-WINDOWS.md` / `gpui-kit-0.6.6-api.md` 谁产出、什么形态 [F]
│   ├─ G2 `UI-MAP.md` §2/§3 的 macOS 口径怎么处理            [D] ← 等 G1
│   └─ G3 提示词与 PLAN.md §5 的合并                        [D] ← 等 B1
│
└─ H  协作与交付
    ├─ H1 本机无 `gh`、git 连不上 origin 时的流程            [F]
    ├─ H2 验收描述与 PR 描述怎么落盘（没有 PR 时）            [D] ← 等 H1/D1
    └─ H3 「每个文件同一时间只有一个写者」是否仍然执行         [D] ← 等 D5
```

---

## 2. 逐个节点的质询

### R0 — 清空重写这条路线本身 【FRONTIER】

**决策**：要不要"清空 `shell_probe/` 下的实现再重写"？

**可选方案**
1. 严格照提示词的"阶段 0：清空旧实现 → 搭最小骨架"再往上长。
2. 不清空：在旧实现上把"macOS 度量"逐处换成 Windows 度量。
3. **混合**：把旧实现按"与规格无关 / 与规格强耦合"切开，只保数据层与启动骨架。

**推荐：3（混合）**，且把"清空"从"阶段 0 的动作"降级成"最终结果"。

**理由（三条独立证据）**
- 旧实现里**与视觉规格无关**的部分是纯资产：`git show HEAD:gpui/shell/src/bin/shell_probe/files.rs:113-149` 的 `load_files`、HEAD `bottom_panel.rs:1005-1033` 的 `execute_core` 信封封装、`bottom_panel.rs:1040-1048` 的 `resolve_repository_root`、`bottom_panel.rs:1331-1404` 的五条 git 命令编排。这些代码与"标题栏 40px 还是 34px"无关，重写它们只是重新踩坑。
- 旧实现里**与视觉规格强耦合**的部分，恰恰是 Windows 口径下明确作废的：`PLAN.md:334` 记录了"底部工具窗横跨整个工作区 = macOS 口径"，而 Windows 默认 `terminalWidthMode === "editor"`（`gpui/research/windows/03-git-and-bottom.md:91`，证据 `windows/tauri/src/features/terminal/stores/terminal.store.ts:29`）。这段必须重做，但它只是 `v_split` 的一处接线，不是 5.2k 行。
- 已经清掉了（`git status` 里 4 个 `D`），而且**代价立刻可见**：现在界面上只剩一行 `工作台根：<path>`（`gpui/shell/src/bin/shell_probe/workspace.rs:42-48`）。从 5.2k 行回到一行，再要长回"外壳 + 侧栏 + 编辑区 + Git 窗 + 状态栏"，是纯亏损的往返。

**如果维持"清空"**，必须回答：这次重写要解决的问题，有多少是**旧实现的实现质量**造成的、有多少是**旧实现的规格来源（macOS）**造成的？只有后者才需要重写。

---

### A1 — 旧实现怎么处置 【FRONTIER，依赖 R0】

**推荐答案**：**从 `HEAD` 恢复，然后按"删到干净"的顺序逆向削减**，而不是"从零长到完整"。

具体口径建议：
1. 先 `git checkout HEAD -- gpui/shell/src/bin/shell_probe/` 拿回 5.2k 行，立刻**提交一次**（或至少打 tag），使"能跑的旧基线"成为可回退的点。
2. 定一张"保留 / 立刻删 / 改写"三栏表：数据层与信封封装保留；macOS 度量常量、macOS 文案、macOS 区域结构立刻删；`v_split` 底部接线改写。
3. 旧实现的**视觉部分**不要逐行改，而是每区域整块替换（一个区域一个文件，正好是旧实现已有的模块划分）。

**副作用**：旧实现带 2 个 warning 的编译基线（`.artifacts/p0/cold-check.log`）。若 D2 定了"0 warning"硬门槛，恢复旧代码会立刻违反它——这两条决策互相咬，见 D2。

---

### A1a — "可复用域逻辑"的边界清单 【DOWNSTREAM，等 A1】

等 A1 定了"恢复+削减"，才需要逐文件划线。已知必须处理的边界点：

| 旧代码 | 为什么必须保留/改写 | 证据 |
| --- | --- | --- |
| `execute_core` 信封封装 | Core 只有 JSON 字符串入口，信封必须手拼；`operationId` 是取消与陈旧结果判定的键 | `bottom_panel.rs:1005-1033`（HEAD）；`rust/lithe-core/src/protocol/command.rs:12-26`；`rust/lithe-core/src/runtime/dispatcher.rs:66-70` |
| `resolve_repository_root` | `git.status.repositoryRoot` 可能是**相对工作区根**的路径，直接回传会按进程 CWD 解析 | HEAD `bottom_panel.rs:1040-1048`；`rust/lithe-core/src/git/mod.rs:6574`、`:6659-6670` |
| `git.historyPage` + `historyCursorClose` 配对 | 游标背后是活着的 `git log` 进程；会话上限 8、空闲 120s 回收 | HEAD `bottom_panel.rs:1380-1404`；`rust/lithe-core/src/git/history.rs:25-29`、`:711-746` |
| `load_files` / 树构建 | `workspace.snapshot` 的 `hiddenDirectoryNames` 默认不返回 `target`/`node_modules`/`dist` | HEAD `files.rs:113-149`；`rust/lithe-core/src/project/files.rs:15-30`、`:39-45` |
| `panels.rs` 的编辑器滚轮接线 | 编辑器元素不吃滚轮，必须宿主自己 `on_scroll_wheel` | `gpui/UI-MAP.md:124` |

---

### A1b — 新旧入口的命名与共存 【DOWNSTREAM，等 A1】

现状是**两个 bin**：`gpui/shell/Cargo.toml:7-9` 定义 `[[bin]] name = "lithe-gpui-shell", path = "src/main.rs"`，而 `gpui/shell/src/bin/shell-probe.rs:3` 是 `mod shell_probe; pub use shell_probe::main;`，编译出的 bin 名是 **`shell-probe`**（`.artifacts/p0/cold-check.log` 的 warning 原文：`` `lithe-gpui-shell` (bin "shell-probe") generated 2 warnings ``）。

提示词的落点决策写"**保留 bin 名 `shell-probe`**"（提示词 `:47`）。但 `src/main.rs` 是一个 347 行的旧冒烟宿主，`gpui/README.md:32` 与 `gpui/capture-screenshot.ps1:21` 都引用 `shell-probe`。这条要在 A1 之后明确：**两个 bin 并存、还是删掉 `src/main.rs`**。不明确就会一直有两份"启动路径"文档。

---

### A1c — 模块拆分口径 【DOWNSTREAM，等 A1/A1b】

已在推进的做法（`PLAN.md:454`，实测 `gpui/shell/src/bin/shell_probe/shell/mod.rs` 存在、1124 字节，mtime 20:25:39）是"每区域一个**无状态渲染函数**（`-> impl IntoElement`），状态集中在 `workspace.rs`"。这是对旧实现"每区域一个 `Entity`"的反转。

**质询点**：无状态渲染函数在 gpui 里意味着每次 `render` 重建整棵子树、无法用 `cx.listener` 持有区域私有状态（例如侧栏滚动位置、引用树展开集合、筛选输入框的 `InputState`）。`InputState` 必须是 `Entity`（`gpui-base-0.6.6/src/input/base/state.rs:766` 的 `InputState::new(window, cx)`）。**所以"全部无状态"在第一个带输入框的区域（阶段 4 的筛选条）就会破功。**建议：定义"纯展示区域用无状态函数；持有 `InputState`/`TableState`/`TreeState` 的区域持有 `Entity`，由 `ShellWorkspace` 统一持有句柄"。

---

### A2 — 工作落在哪个分支 【FRONTIER】

**实测事实**
- 当前分支 `main`，HEAD `a9a709ab`。
- **本机不存在 `preview` 分支**：`git branch -a --list` 只有 `main` + `remotes/origin/main`；`.git/packed-refs` 里也没有 `preview`；`git reflog --all` 无 `preview`。
- `AGENTS.md` 的"每次进行功能开发…都单独从最新的 **preview** 分支创建新分支"因此**物理上无法执行**（连本地都没有 preview，且取不到 origin）。

**可选方案**
1. 继续在 `main` 上直接干（当前实际状态）。
2. 从当前 `main` HEAD 切一个本地功能分支（如 `feat/gpui-windows-shell`），本地提交，后续能取到 origin 时再 rebase 到真正的 `preview`。
3. 先停下，等维护者提供 origin 访问或 preview 内容。

**推荐：2。** 理由：方案 1 会让"清空重写"这种大范围改动直接堆在 `main` 上，且 `main` 是发布分支（`git log` 里 `main` 上有 `lithe 0.5.2`、`Merge pull request #836 ... homebrew-cask` 这类发布提交）。方案 2 的成本只是 `git switch -c`，却能立刻把"分支纪律"这条硬约束满足掉一半，并且让 `git diff main...HEAD` 成为天然的验收证据。方案 3 会阻塞整条线，而本机并没有"必须有 preview 才能写代码"的技术理由。

---

### B1 — 以哪份文档为准 【FRONTIER】

**现状（互相不一致，全部实测）**
- 提示词：`docs/development/gpui-ui-windows-rewrite-prompt.md`，125 行，落款"由上一轮对话整理"。
- `gpui/PLAN.md:435-464` 的 §5：**后写的**（mtime 20:26:13），且自述"取代 §2 的旧表"（`PLAN.md:443`）。
- `gpui/BLOCKERS.md`：**再后写的**（20:25 前后），且它把"重试上限 3 次"这条规则写成"维护者 2026-09-25 定，**替代** `PLAN.md` 里'同一问题最多试 5 次'的旧口径"（`gpui/BLOCKERS.md` 开头规则段）。

**推荐**：确立**单一真源层级**：`提示词`（决策与坑清单）→ `PLAN.md §5`（执行细分与进度）→ `BLOCKERS.md`（做不了的登记）。三者冲突时以**提示词为准**，除非 §5/BLOCKERS 明确写了"取代/替代"并且有维护者时间戳。**并立即把提示词里已被取代的两条（重试上限、底部窗口径）就地更新**，否则第三个读者还会踩。

---

### B2 — Windows 前端的哪些层算规格 【FRONTIER】

这条被严重低估。提示词把"规格来源 = `windows/tauri/src/`"当作一句话，但 Windows 前端是**三层不同性质的东西**：

| 层 | 例子 | 能否当规格 |
| --- | --- | --- |
| 呈现层（DOM/CSS） | 标题栏 40px、活动栏 38px、状态栏 24px | ✅ 提示词要的就是它 |
| 前端状态层（zustand） | buffer 列表、pane 树、pin/preview、项目标签条持久化 | ❌ **Core 里没有**，GPUI 侧要从零建 |
| 内容宿主（Monaco） | 行号、折叠、minimap、查找替换、诊断波浪线 | ⚠️ gpui-kit 有等价物但不等价（`gpui/README.md:113` 列了缺 minimap/overview ruler/sticky scroll/LSP 客户端） |

**证据**：`gpui/research/windows/02-editor-sidebar.md:17` 明确写"标签/窗格**状态不在 Core**：buffer 列表、激活项、pin、preview、窗格树全在渲染进程 zustand，工作区会话持久化到前端 storage（key `lithe-tab-sessions`）。Core 只提供文件读写、工作区快照/搜索、LSP。"

**推荐**：明确宣布"本轮的规格 = **呈现层**；前端状态层只在被某区域的静态外观需要时按最小子集重建（例如项目标签条需要一份 `projectTabs` 列表），不试图复刻 zustand store 的形状"。这一句不写下来，阶段 3（编辑区）就会变成"重新实现 buffer store"。

---

### B2a — 24 种 buffer 类型的 pane 树要不要在本轮建模 【DOWNSTREAM，等 B2】

`02-editor-sidebar.md:16` 记录标签是 buffer，内容类型有 24 种（editor/terminal/diff/image/pdf/database/webViewer/pr/issue…）。若 B2 只认呈现层，则本轮的编辑区就是"一条标签栏 + 一个编辑器"，`+`／分屏／拖出成新 pane 都不做（旧实现也正是这么登记的：`PLAN.md:252` 把 `+`/拆分/全屏列为"能力缺失而不是少画了按钮"）。

---

### B2b — "前端本地 store 拥有的事实"清单 【DOWNSTREAM，等 B2】

已知会在这轮反复撞到的：状态栏光标位置/编码/缩进（`gpui/research/windows/01-shell.md:441-444`）、项目准备状态（`:446`）、内存轮询（`:437`，走原生 Tauri 命令 `get_application_memory_usage`，**Core 里没有**）、帧率、Git 变更防抖刷新（`:440`）。这些在 GPUI 侧要么自建、要么状态栏留空位。

---

### B4 — 玻璃层与窗口透明：**默认是关的** 【FRONTIER】

提示词 `:56` 写："玻璃层 | Windows 端是 `color-mix` 半透明 + `backdrop-filter: none`（没有模糊）→ gpui 用**带 alpha 的颜色**直接画即可，**可一比一**"。

**这条是错的。** 实测证据链：

1. 玻璃 token 全部包在 `html:is(.platform-macos, .platform-windows):not([data-window-transparency="disabled"])` 里（`windows/tauri/src/styles/window-transparency.css:5`）。
2. 设置默认值是 **`windowTransparency: false`**（`windows/tauri/src/features/settings/config/default-settings.ts:106`），经 `applyWindowTransparency` 写成 `data-window-transparency="disabled"`（`windows/tauri/src/features/settings/lib/settings-effects.ts:30-37`）。
3. Tauri 窗口本身 **`"transparent": false`**（`windows/tauri/src-tauri/tauri.windows.conf.json`）。
4. 前端传给原生层的 `transparencyEnabled` 参数，**Rust 侧的命令签名根本不接收它**：JS 侧 `invoke("set_native_window_appearance", { themeType, transparencyEnabled })`（`settings-effects.ts:127`），Rust 侧 `pub fn set_native_window_appearance(window: WebviewWindow, theme_type: String)`（`windows/tauri/src-tauri/src/host.rs:513-515`）——只有一个参数，函数体只做 `window.set_theme(...)`。
5. 顺带：`gpui/research/windows/04-theme-and-components.md:369` 自己写着"**Rust/gpui 侧没有等价的「原生窗后透」API，这块必须重新设计或降级**"。这条调研结论与提示词 `:56` 直接冲突，而提示词要求"不要跳过第 3 节"却只读了坑清单，没读调研的这条。

**推荐**：
- **不在本轮做玻璃层**。默认外观下它就是"不透明的 `--surface`/`--background`"，与普通分层配色无异。阶段 1 用不透明色画，把 `--lithe-glass-*` 记进 FUTURE 清单。
- 如果维护者要求连"透明模式"也 1:1：技术上可行——gpui 的 `WindowOptions` 有 `window_background: WindowBackgroundAppearance`（`gpui-pre-0.3.6/src/platform.rs:2225-2226`），枚举含 `Transparent / Blurred / MicaBackdrop / MicaAltBackdrop`（`platform.rs:2449-2464`），默认是 `Opaque`（`platform.rs:2345-2364` 的 `Default` 用 `WindowBackgroundAppearance::default()`），而且**运行期可改**：`window.set_background_appearance(...)` 是 `pub`（`gpui-pre-0.3.6/src/window.rs:2901-2905`）。但这属于"用 gpui 平台能力做产品级新特性"，不是"照规格一比一"。

---

### C1 — 底部窗归属 【FRONTIER】

**事实**：Windows 默认 `terminalWidthMode === "editor"`（`windows/tauri/src/features/terminal/stores/terminal.store.ts:29`），`BottomPane` 挂在**中央列**内（`main-layout.tsx:318-322`，见 `03-git-and-bottom.md:91`）；只有 `"full"` 时挂到 `lithe-workbench-glass` 下单列、横跨整个工作台、左右各留 4px（`main-layout.tsx:345-351`）。

**关键实测**：gpui-kit 的 `DockPlacement::Bottom` **只横跨 center 列**（`gpui-base-0.6.6/src/dock/dock_area.rs:1415-1431`，`gpui/UI-MAP.md:91`、`gpui/README.md:93`）。

**所以 Windows 默认口径与 gpui-kit 的 Bottom dock 天然一致**：底部窗宽度 = 中央列宽度 = 编辑器岛宽度，右侧工具窗是它的兄弟而不是被它盖住。**不会自相矛盾**——矛盾只存在于"上一版把底部窗放进 center 的 `v_split` 以横跨左栏+编辑区"（`PLAN.md:334`），那是 macOS 口径。

**推荐**：用 `DockPlacement::Bottom`，接受它只横跨 center 列；并**显式记一条差异**：Windows 的底部窗高度默认 320+4=324px、最小 200px、可拖（`03-git-and-bottom.md:274-275`），且**底部窗没有自己的标签条**，由活动栏/命令面板切换单值 `bottomPaneActiveTab`（`03-git-and-bottom.md:114-121`）——这与 gpui-kit Bottom dock 的"多面板页签"模型不同，需要决定是"Bottom dock 只放一个面板、页签由工作台自绘"还是"用 dock 的页签"。

**注意**：这条已在 `PLAN.md:455` 被单方面拍板为"用真 Bottom dock"，与提示词 `:51` 一致。所以它是 FRONTIER 但**答案已经写好，只需确认**。

---

### C2 — 窗口口径 【FRONTIER】

**三套并存的口径**

| 口径 | 值 | 证据 |
| --- | --- | --- |
| Windows Tauri 配置 | 1200×800，最小 720×480，`decorations:false`，`center:true` | `windows/tauri/src-tauri/tauri.windows.conf.json` |
| Windows 真机实测（历史） | 启动即最大化（`IsZoomed()=true`） | `PLAN.md:399` |
| 当前 gpui 实现 | 可见区 ×0.94、夹 1024×680、居中、`WindowBounds::Windowed` | `gpui/shell/src/bin/shell_probe/mod.rs:34-53`；依据 `Dodona/crates/dodona/src/main.rs:91-108` |

**推荐**：**保留 Dodona 口径**（现状）。理由：它是本机唯一被维护者确认"界面正常"的 gpui-kit 应用（`UI-MAP.md:101-106`），而且"窗口几何"不是 Windows 规格的一部分——Windows 规格是**窗口内部**的 40/24/38/4px。把 Tauri 的 1200×800 搬过来反而与"IDE 应该铺满"冲突。**但要把这条差异写进验收表**，否则"与真机并排对照"时第一眼就是"窗口大小不一样"。

---

### D1 — "阶段完成"的判据 【FRONTIER】

**计划里的两条判据是互斥的**
- 提示词 `:87`：验证 = `cargo check` 0 error 0 warning；每阶段留证据（DPI 感知截图 + 日志）。
- 提示词 `:151`（验收标准）：与 Windows 前端**并排对照**，尺寸与结构一致（逐项对照表 + 截图）。
- 提示词 `:77`（坑清单 11）：**同一个问题最多试 5 次**，超了写文档；而 `PLAN.md:453` 已改成 **3 次**。

**"3 次"到底指试什么？** 从 `BLOCKERS.md` 的规则段看，一次 = "一次 `cargo check`，或一次翻 gpui-kit 源码找 API 未果"。这在实践中会立刻出问题：**一次 `cargo check` 不是一次"尝试"**，它是编译整个依赖树后的一个错误列表；而 `PLAN.md:255`、`:312` 都记录了历史轮次"`cargo check` 共 5 次（用满上限）"——那 5 次里有 3 次是**并行子代理别的文件报错**（`bottom_panel.rs` 的图标名、`Map<Iter>..`），根本不属于"同一个问题"。

**推荐**：
1. 把"一次"定义为"**一个具体根因的一次修复尝试**"，而不是"一次构建"。构建次数不限，但同一根因连续 3 次没解决就进 `BLOCKERS.md`。
2. 把"阶段完成的判据"写成可判定列表，且**明确区分"代理可验证"与"只能维护者验证"**（见 D1a）。当前提示词 `:104-111` 的验收清单把这两类混在一起。
3. "并排对照"要求"逐项对照表"——**这张表现在不存在**（`UI-MAP-WINDOWS.md` 不存在，见 G1）。所以 D1 与 G1 互为前提。

---

### D1a — 无法合成鼠标事件时的交互验收 【DOWNSTREAM，等 D1】

**事实**：`PLAN.md:340` 记录"本环境无法合成鼠标事件（`SetCursorPos` 后光标停在 (0,0)、`GetForegroundWindow()` 返回 0），所以'点击打开'只能由维护者在本机验证"；`BLOCKERS.md` B5 把它固化成结论"所有点击/悬停/拖拽类交互只能由维护者在真机验证，代理只能验证'启动路径 + 静态布局 + 截图'"。

**能被解析验证的**：区域高度/宽度/间距/颜色（截图像素测量）、文案（`i18n/locale.ts` 逐字 grep）、真实数据是否到位（窗口内文字 + 写盘的原始 JSON）。

**不能被解析验证的**：悬停/选中/禁用三态、右键菜单、拖拽分栏、标签切换、命令面板唤起、终端输入。

**推荐**：验收表**每行标注验证方式**（`截图` / `日志` / `维护者实机`），并在阶段汇报里**显式声明本轮哪些行没被验证**。这比"假装阶段完成"便宜得多，也符合 `develop-lithe` 的"Report what changed, what was verified, and any remaining platform or test limitations"（`.agents/skills/develop-lithe/SKILL.md:266`）。

---

### D2 — "0 error 0 warning" 是否为硬门槛 【FRONTIER】

**事实**：当前基线就有 2 个 warning（`.artifacts/p0/cold-check.log` 末 10 行，指向 `shell_probe/workspace.rs:17` 与 `:29`）。而恢复旧实现（A1）会带回更多：`PLAN.md:318-320` 记录过 `PanelKind::Terminal` 变体未被构造、`workspace.rs` 的 `files_panel` 字段未读；`:224` 记录"残留 warning 与拆分前相同"。

**推荐**：把门槛拆成两条：
- **硬门槛**：`0 error`（不能交不能编译的东西）。
- **软门槛**：`0 warning`，且允许"本轮新增的 warning 必须为 0；历史遗留 warning 逐条登记到 `BLOCKERS.md` 或直接在注释里写明来由"。

理由：`gpui/shell/src/bin/shell_probe/mod.rs` 现在就在用 `with_assets`，而 `gpui/README.md:122` 明确说"宽松版本范围会让构建结果漂移"——依赖树的一点点变化就会引入新的 deprecated warning，把上游 warning 也算成自己的失败是错的。

---

### D3 — 3 次重试的计量口径 【FRONTIER】

见 D1 的推荐 1。另需明确：**并行子代理造成的构建失败算谁的次数**（`PLAN.md:255` 的第 2、3、5 次都属此类）。推荐：归"文件所有者"，且不计入被阻塞者的次数。

---

### D4 — 证据目录与留存 【FRONTIER】

**事实**
- `.artifacts/` 被仓库根 `.gitignore:9` 忽略 → **证据不进版本库、不随 PR 走**。
- 现有证据只剩 `.artifacts/p0/cold-check.log` 一个文件；`gpui/UI-MAP.md:35-37` 明确写"早期调试期的截图证据（`.artifacts/p1/*`）…已删除"，理由是"有**坏截图工具产出的误导性画面**"。
- 而验收标准要求"证据（DPI 感知窗口帧 + 整屏 + 与真机并排）落到 `.artifacts/`"（提示词 `:110`）。

**推荐**：明确"证据是**过程产物**，不进仓库；进仓库的是**结论**（写进 `PLAN.md`/`UI-MAP-WINDOWS.md` 的差异表 + 行号）"。理由是历史已证明截图会骗人（`PLAN.md:427`：坏截图工具让"内容只占 78%"看起来像"已铺满"，误导了整整两轮）。**只看图不看测量的验收方式是这条线最大的风险源。**

---

### E1 — 第一版命令清单 【FRONTIER】

**提示词自己矛盾**：`:49` 写"最小真实数据（`workspace.snapshot`、`git.status`、`git.historyPage`）"——3 条；`:106` 写"`workspace.snapshot`（项目树）、`git.status`/`git.historyPage`/`git.commitFiles`（Git 窗）"——4 条。

**四条命令全部真实存在**（`rust/lithe-core/src/protocol/command.rs:320`、`:401`、`:428`、`:432`），且 `git.commitFiles` 是**独立命令**，不是 `historyPage` 的字段（`GitHistoryPageResponse` 只有 `commits/nextCursor/nextOffset/hasMore`，`rust/lithe-core/src/protocol/contracts.rs:640-649`）。

**但四条命令做不出一个 Git 窗**：
- 引用树要 **`git.references`**（`command.rs:427`）；
- 游标要还，要 **`git.historyCursorClose`**（`command.rs:429`）；
- 提交详情要 **`git.commit`**（`command.rs:431`）。
旧实现用的正是这 5 条（HEAD `bottom_panel.rs:1278/1331/1366/1380/1398`），而提示词全文**没有出现** `git.references` 与 `git.historyCursorClose`。

**推荐**：第一版命令白名单定为 **6 条**：`workspace.snapshot`、`git.status`、`git.references`、`git.historyPage`、`git.historyCursorClose`、`git.commitFiles`（`git.commit` 留给"提交详情"进阶段 4 时再加）。

---

### E1a — 游标生命周期 【DOWNSTREAM，等 E1】

`nextCursor` 背后是**活着的 `git log` 子进程 + 背压通道**：会话上限 8、空闲 120s 回收、单流最多 5000 commit（`rust/lithe-core/src/git/history.rs:25-29`、`:711-746`、`:237-240`）。契约要求不用了就发 `git.historyCursorClose`（`shared/contracts/rust-core-api.md:890-897`）。**要定**：翻页时旧游标立刻关还是保持；刷新/切引用/切仓库时怎么关；窗口关闭时是否需要清理。

---

### E1b — `repositoryRoot` 相对路径归一化 【DOWNSTREAM，等 E1】

`git.status.repositoryRoot` 在工作区内的仓库返回的是**相对工作区根**的路径（`rust/lithe-core/src/git/mod.rs:6574` → `relative_or_absolute` `:6659-6670`；契约 `shared/contracts/rust-core-api.md:284-286`）。拿它当后续 `git.*` 的 `root` 会按**进程 CWD** 解析。旧实现专门写了 `resolve_repository_root`（HEAD `bottom_panel.rs:1040-1048`）。**这是必须照做的口径，不是可选项**，而提示词完全没提。

---

### E1c — "非仓库"的两种行为、且没有专用错误码 【DOWNSTREAM，等 E1】

- `git.status` 在非仓库下 **不报错**：`ok:true` + `repositoryRoot: null`（`rust/lithe-core/src/git/mod.rs:6536-6544`）。
- `git.historyPage` / `git.commitFiles` / `git.references` 在非仓库下 **`ok:false` + `process_failed`**（`git/history.rs:634-637`、`git/mod.rs:2029-2033`、`git/history.rs:154-159`）。
- `ErrorCode` 全 11 项里**没有**任何"不是仓库"的取值（`rust/lithe-core/src/protocol/error.rs:11-34`；`grep NotARepository|not_a_repository` → 空）。

**所以靠错误码区分"非仓库"是错的**。正确顺序是先 `git.status` 看 `repositoryRoot`（旧实现正是这样：HEAD `bottom_panel.rs:1354-1363`）。而 UI 文案上，提示词 `:378`（旧 PLAN 记录）用的是 macOS 原文「当前项目不是 Git 仓库」——**但本轮规格是 Windows**，Windows 的空态文案在 `03-git-and-bottom.md:141`（`GitRepositoryEmptyState`）。**这里有一个"规格来源改了但文案还没改"的残留**，需要专门确认。

---

### E2 — 宿主 ↔ Core 的调用形态 【FRONTIER】

**事实（这条最容易被想当然）**：`rust/lithe-core` 的 `mod git; mod project; mod protocol;` **都是私有模块**（`rust/lithe-core/src/lib.rs:11,16,17`），所以 `GitStatusRequest` / `GitHistoryPageRequest` / `WorkspaceSnapshotRequest` 等类型**在 crate 外不可命名**；`CoreRequest` 只 derive `Deserialize`（`command.rs:6-27`）、`CoreResponse` 只 derive `Serialize`（`contracts.rs:10-21`），两边都不能直接当收发结构。

**唯一公开入口**：`lithe_core::execute_json(request: &str) -> String`（`rust/lithe-core/src/lib.rs:25-27`），**同步阻塞**（`rust/lithe-core/Cargo.toml:11-30` 无 tokio/async-std；`grep 'async fn' rust/lithe-core/src` → 空）。

**推荐**：确认"宿主手拼 JSON + 自建信封封装 + `cx.background_spawn` 挪开 UI 线程"这条口径，并把它写成新实现的**第一条基础设施**（因为每个区域都要用）。旧实现的 `execute_core`（HEAD `bottom_panel.rs:1005-1033`）就是它，恢复即可。

**额外提醒**：Core 内部有**进程级全局状态**——搜索索引缓存（`rust/lithe-core/src/project/search_index.rs:95-100`）与 Git history 会话注册表（`rust/lithe-core/src/git/history.rs:31-33`）。`workspace.snapshot` 每次调用都会 `search_index::invalidate_root(&root)`（`rust/lithe-core/src/project/files.rs:162`），**刷新项目树会连带打掉搜索索引缓存**。这轮不接搜索，但要记下来。

---

### E3 — 终端第一版到底做什么 【FRONTIER】

提示词 `:50` 的口径是"直接调本地 `cmd` 与 `powershell`，第一版只做外壳：页签 + 流式显示 stdout/stderr + 一个发送命令的输入行；**不做完整 VT 模拟**"。

**四个必须当场纠正的事实**

1. **参考实现不是"四个 command"，是八个**，而且**传的不是字节流**：`windows/tauri/src-tauri/src/terminal.rs` 有 8 个 `#[tauri::command]`（`:85/:100/:105/:131/:142/:153/:164/:176`，注册于 `main.rs:130-137`）；输出是 `TerminalEvent::Output { data: Vec<u8> }`（`windows/tauri/crates/terminal/src/protocol.rs:56-68`）经 serde 变成**JSON 数字数组**（前端类型 `data: number[]`，`windows/tauri/src/features/terminal/types/terminal.types.ts:59-63`；接收侧 `Uint8Array.from(event.data)`，`use-terminal-connection.ts:160`；Rust 单测断言 `"data":[255,0,27]`，`protocol.rs:127-135`）。**不是 base64，也不是裸字节流。**
2. **Rust 侧对 ANSI 零清洗**：`connection.rs:478-487` 原样 `to_vec()` → `terminal.rs:116` 原样 `on_event.send(event)`。前端可读性 **100% 依赖 xterm.js**（`"@xterm/xterm": "^6.0.0"`，`windows/tauri/package.json:49`；唯一写入点 `use-terminal-connection.ts:176` 的 `terminal.write(bytes, …)`）。
3. **"不做 VT"在真实数据流上立刻卡住**：ConPTY 一定吐转义序列，把它们当普通文本打出来就是可见垃圾。仓库里有现成样本：`shared/fixtures/maven/dependency-tree-v1.json:4` 的 `"\u001b[36m[INFO]\u001b[0m …"`；TUI 探针里的 `\x1b[?1049h\x1b[2J\x1b[H`（`windows/tauri/crates/terminal/examples/tui_probe.rs:3-5`）。观感就是 `[36m[INFO][0m`、`[?1049h[2J[H` 直接出现在屏幕上；`\x1b]0;title\x07` 会让 `]0;…` 泄进正文（现有代码专门写正则拦这种泄漏：`windows/tauri/src/features/terminal/utils/terminal-title.ts:1-2`）。
4. **"照搬 macOS 的 `TerminalTransport`/`TerminalSession` 字段契约"（写在 `gpui/UI-MAP.md:144`）会丢掉尺寸同步**：macOS 契约里根本没有 rows/cols；`MacTerminalTransport.sizeChanged` 是空实现（`macos/Sources/Lithe/Platform/MacOS/Terminal/MacTerminalTransport.swift:398`），尺寸由 SwiftTerm 视图自己管；而 `nativeView: AnyObject`（`macos/Sources/LitheTerminalModule/Ports/TerminalTransport.swift:43`）本身就假设存在一个原生 VT 视图。Windows 侧相反，尺寸是显式命令：`terminal_resize`（`src-tauri/src/terminal.rs:142-151`）→ `connection.rs:550-559`。**GPUI 侧既没有原生 VT 视图可塞，也没有 resize 字段可搬。**

**推荐（三选一，必须选一个并写下来）**
- **E3-A（推荐）**：第一版对齐 Windows 口径做**输出查看器**，但把 ANSI 处理降级为"最小清洗"（见 E3a），并且**显式声明这不是终端**（不能跑 vim/top/PSReadLine 行内重绘）。工作量最小且不会骗人。
- **E3-B**：引 `portable-pty` + `vte`/`alacritty_terminal` 做真 VT（`gpui/Cargo.lock` 现有 905 个包里**零命中** pty/VT 依赖，等于要新增一整条依赖链并冷构建）。这是"终端"的正解，但远超"第一版外壳"。
- **E3-C**：不做终端，把底部窗第一版只做 Git 窗，终端进 `BLOCKERS.md`。

---

### E3a — 最小 ANSI 处理范围 【DOWNSTREAM，等 E3】

若选 E3-A，最小集合是：**丢弃 SGR（`\x1b[…m`）+ 丢弃 CSI + 丢弃 OSC + 处理 `\r`/`\b`**。仓库里有三份现成参考可抄（都是 TS/正则，翻译成 Rust 即可）：
- SGR + OSC 跳过 + CR/BS 的**部分解释器**：`windows/tauri/src/features/run/utils/run-output-style.ts:92-149`（`parseAnsi`）、`:252-277`（`readCsi`/`skipOsc`）、`:279-314`（`applySgr`，含 256 色与真彩）；
- **序列边界扫描**（截断输出时不切断序列）：`windows/tauri/src/features/run/utils/output-timestamper.ts:202-238`；
- **Rust 侧的 CSI 剥离正则**（Core 里已有）：`rust/lithe-core/src/project/maven.rs:974-975`、`:1613-1614`。

**要定**：只丢不解析（观感=无颜色的日志），还是把 SGR 映射到 gpui 颜色（观感=有颜色的日志，工作量约 2 倍）。推荐**先"只丢不解析"**，把彩色留给后续。

---

### E3b — 尺寸同步 【DOWNSTREAM，等 E3】

必须做，否则 ConPTY 一直以创建时尺寸（默认 24×80，`windows/tauri/crates/terminal/src/protocol.rs:13-22`）断行。参考：前端 `getTerminalSize()` 读 xterm 的 `rows/cols`（`src/features/terminal/utils/terminal-protocol.ts:86-96`）→ `terminal_resize`。GPUI 侧的行列数必须**自己按字体度量算**（没有 xterm 的 fit addon）。

---

### E3c — 输入模型与 Ctrl+C 【DOWNSTREAM，等 E3/E3b】

提示词 `:50` 的"一个发送命令的输入行"意味着**整行提交**，而 Windows 侧是**逐键进 PTY**（`terminal.onData(write)`，`use-terminal-connection.ts:145`）。整行提交的后果：
- shell 侧的行编辑/补全/历史（方向键、Tab、Ctrl+R）**全部失效**；
- **Ctrl+C 断不了前台进程**（Windows 前端裸 Ctrl+C 是 passthrough 成 `0x03`，`src/features/terminal/utils/terminal-keyboard.ts:101`）。要保留中断能力，就得多一个按钮发 binary `[3]`（Rust 侧支持 `TerminalInput { kind:"binary", data }`，`crates/terminal/src/protocol.rs:34-48`）。
**要定**：第一版是否接受"Ctrl+C 不可用，只能关页签杀进程"。

---

### F1 — 图标集：默认 101 vs 全量 1830 【FRONTIER】

**这条计划的"硬约束"其实是个误解，一行代码能解。**

实测：
- `gpui-kit-assets-0.6.6/default-icons.txt` 恰好 **101** 行，**没有**任何 `git-*` 字形（与提示词 `:73` 一致）。
- 但同一个 crate 里有一份**全量 Lucide 目录**：`gpui-kit-assets-0.6.6/assets/icons/` 下 **1830 个 svg，合计 731 KB**，其中 git 字形齐全：`git-branch.svg`、`git-branch-plus.svg`、`git-branch-minus.svg`、`git-commit-horizontal.svg`、`git-commit-vertical.svg`、`git-merge.svg`、`git-merge-conflict.svg`、`git-compare.svg`、`git-compare-arrows.svg`、`git-fork.svg`、`git-graph.svg`、`git-pull-request*.svg`。
- 注册全量只需要换 asset source：`pub use native_assets::{AllAssets, Assets}`（`gpui-kit-assets-0.6.6/src/lib.rs:36`），`AllAssets` 的定义是 `#[derive(rust_embed::RustEmbed)] #[folder = "assets"] #[include = "icons/**/*.svg"]`（`src/native_assets.rs:9-12`）。即 `with_assets(gpui_kit::assets::Assets)` → `with_assets(gpui_kit::assets::AllAssets)`（现在写在 `gpui/shell/src/bin/shell_probe/mod.rs:73`）。
- 而且**不需要用 `component::IconName`**：`Icon::build` 接受任何 `impl IconNamed`，并且有 `impl<T: IconNamed> From<T> for Icon`（`gpui-component-0.6.6/src/icon.rs:41-45`），所以 `Icon::new(gpui_kit_assets::IconName::GitBranch)` 直接可用。`component::IconName` 只是一个"保留源码兼容"的 101 项子集枚举（`icon.rs:13-38`，宏生成自 `gpui-kit-assets-0.6.6/build.rs` 里的 `__component_icon_names!`）。
- 第三种更精细的路：`icon_assets!` 宏可以只嵌入选定字形并**与默认集组合**（`gpui-kit-assets-0.6.6/src/lib.rs:60-82` 的文档与宏体）。

**推荐**：**用 `AllAssets`**。理由：731 KB 对桌面 IDE 无意义，但它一次性消掉整条"缺字形→用近似→登记"的人工流程（`PLAN.md:293-296` 记录了旧实现为此把 `BookOpen` 当代 Git 日志图标、`ArrowDown` 代"更新所选"、`Replace` 代"与当前分支比较"…共 14 处代替）。若维护者坚持最小体积，就用 `icon_assets!` 精确补 git 字形（约 20 个 svg，几 KB），**但不要再用"近似字形 + 登记"那条路**。

**注意**：`00.2` 的 `PLAN.md:239` 与 `UI-MAP.md:48` 都把"只能用 101 个字形"写成硬规则——**这条规则要就地改掉**，否则后续子代理会一直被它误导。

---

### G1 — `UI-MAP-WINDOWS.md` / `gpui-kit-0.6.6-api.md` 【FRONTIER】

**事实**
- `gpui/UI-MAP-WINDOWS.md`：**不存在**。`git ls-tree -r HEAD` 里没有它，工作区没有它。而 `UI-MAP.md:11` 与 `PLAN.md:10-11` 都把它当"逐区域对应表"引用，`PLAN.md:14` 还把它写成"提示词里列为第 1 步交付物"。
- `gpui/research/gpui-kit-0.6.6-api.md`：**不存在**（`PLAN.md:457` 声称由子代理产出中）。
- 提示词路径漂移：已修（`PLAN.md:12` 与 `UI-MAP.md:11` 现在都写 `…-rewrite-prompt.md`，mtime 20:25–20:26）。
- `gpui/UI-MAP-macos.md`：已从 git 历史恢复（`BLOCKERS.md` B2）。

**推荐**：把 `UI-MAP-WINDOWS.md` 明确定义为**验收表的载体**（D1 要的"逐项对照表"就是它），内容以"区域 → Windows 出处行号 → px 值 → gpui-kit 实现 → 状态（未做/已做/差异）"为列。**不要**再写一份"六份调研的摘要"——六份调研（约 68 万字符）已经在仓库里，重复摘要是漂移的来源。`gpui-kit-0.6.6-api.md` 建议**不产出**：`gpui/README.md:40-109` 与 `UI-MAP.md:53-63` 已经承担了这个角色，再写第三份必然三处不一致（`gpui/README.md` 自己的第一句话就是"只讲怎么跑、已经验证过的接口事实和硬约束"）。

---

### H1 — 本机无 `gh`、git 连不上 origin 【FRONTIER】

**实测（我自己复核过，与 `BLOCKERS.md` B1 一致）**
- `Get-Command gh` → 无结果（`gh` 不在 PATH）。
- 我另外扫了 `C:\Program Files\GitHub CLI`、`%LOCALAPPDATA%\Programs\GitHub CLI`、`$env:ProgramFiles\GitHub CLI` → **全部不存在**。
- `git remote -v` → `origin https://github.com/T-fenlin/Lithe-IDEA.git`（存在，但取不到）。
- `git branch -a` → 只有 `main` / `origin/main` / `origin/HEAD -> origin/main`；**无 `preview`**。
- `AGENTS.md` 的开发流程第 1、2 条要求"从最新 preview 开分支"+"用 `gh` CLI 提 PR"。**两条都物理不可执行。**

**推荐**：本轮按"**本地分支 + 本地提交 + PR 描述草稿落盘**"走：
1. 从当前 `main` HEAD 开本地分支（见 A2），
2. 每阶段完成即本地提交，commit message 写清区域与像素依据，
3. 把 PR 描述按 `AGENTS.md` 第 3 条（"说清楚对应改动的功能点"）写成 `.artifacts/` 下的草稿（不进仓库）或直接在汇报里给全文，
4. **把 B1 作为唯一的流程阻塞项交给维护者**，不要反复尝试 `gh auth`/`git push`（那会消耗 D3 的尝试次数）。

---

### F2 — 密度档与"gpui-kit 写死像素"覆盖清单 【DOWNSTREAM，等 B2】

Windows 有两档密度（`:root[data-window-chrome-density="comfortable"]`，`windows/tauri/src/styles/theme.css:188-200`），本轮只做默认档是合理的。但 `04-theme-and-components.md:718` 有一节"gpui-kit 里**写死的像素常量**（密度映射必须逐个覆盖）"——如果哪天要支持 comfortable，这份清单是前提。本轮**明确宣布不支持密度切换**，把它记进 FUTURE。

---

## 3. FRONTIER 汇总（交给用户决策，共 12 条）

| # | 标题 | 一句话问题 | 我的推荐 |
| --- | --- | --- | --- |
| Q1 | 代码基线 | 清空重写，还是从 `HEAD` 恢复 5.2k 行再按 Windows 口径削减？ | 恢复 + 削减（"清空"当结果而不是动作） |
| Q2 | 分支策略 | 本机无 `gh`、无 `preview`、连不上 origin，怎么办？ | 从当前 `main` 开本地分支，本地提交，PR 描述落盘草稿 |
| Q3 | 真源层级 | 提示词 vs `PLAN.md §5` vs `BLOCKERS.md` 冲突时以谁为准？ | 提示词为准；已被取代的两条（3 次上限、Bottom dock）就地更新 |
| Q4 | 规格层次 | "规格来源=Windows 前端"是否包含 zustand 本地状态层与 Monaco 宿主层？ | 只认**呈现层**；状态层按最小子集重建 |
| Q5 | 玻璃层 | 透明/玻璃层本轮做不做？ | **不做**——Windows 默认 `windowTransparency: false` + `transparent: false`，外观就是不透明 |
| Q6 | 底部窗 | `DockPlacement::Bottom`（只横跨 center）是否自相矛盾？ | 不矛盾，与 Windows 默认 `"editor"` 一致；确认 `PLAN.md:455` 的拍板 |
| Q7 | 阶段判据 | 无鼠标事件注入时"阶段完成"靠什么判定？"3 次"指试什么？ | 判据 = 可判定列表 + 每行标验证方式；"一次"= 一个根因的一次修复，不是一次构建 |
| Q8 | warning 门槛 | "0 error 0 warning"是硬门槛吗？ | `0 error` 硬门槛；`0 warning` 只对**本轮新增** |
| Q9 | 命令白名单 | 3 条（`:49`）还是 4 条（`:106`）？ | 6 条：`workspace.snapshot` / `git.status` / `git.references` / `git.historyPage` / `git.historyCursorClose` / `git.commitFiles` |
| Q10 | 终端范围 | "不做 VT"具体做到哪一步？ | 做"输出查看器"+ 最小 ANSI 清洗（丢 SGR/CSI/OSC + 处理 CR/BS），并显式声明不是终端 |
| Q11 | 图标集 | 继续用 101 字形 + 近似代替，还是换 `AllAssets`/`icon_assets!`？ | 换 `AllAssets`（731 KB，解掉整条"近似 + 登记"流程） |
| Q12 | 文档 | `UI-MAP-WINDOWS.md` 谁产出、什么形态？`api.md` 要不要？ | `UI-MAP-WINDOWS.md` = 验收对照表；**不产出** `api.md` |

---

## 4. DOWNSTREAM（等 frontier 答案后才会变成可问）

- **A1a** 恢复后"保留/删/改写"的三栏边界（等 Q1）
- **A1b** 两个 bin（`lithe-gpui-shell` 与 `shell-probe`）的处置（等 Q1）
- **A1c** "全部无状态渲染函数"在第一个 `InputState` 处会破功，模块口径要收紧（等 Q1）
- **B2a** 24 种 buffer 类型与 pane 树是否建模（等 Q4）
- **B2b** 前端本地 store 事实清单（等 Q4）
- **B3** 哪些视图允许"做得比 Windows 好"（等 Q3/Q4）
- **C1a** 底部窗没有自己的页签条 → dock 页签 vs 工作台自绘（等 Q6）
- **C3** 自绘三键 + 8 个缩放热区 + 拖拽（等 Q5/Q6 的窗口口径确定后）
- **D1a** 交互验收清单（等 Q7）
- **D1b** 允许的"测试脚手架"程度（等 Q7，旧实现有先例：`PLAN.md:373` 曾临时在第 3 帧自动开对话框验证后删除）
- **D5** 构建串行下的并行策略（等 Q8/Q9）
- **E1a** 游标生命周期（等 Q9）
- **E1b** `repositoryRoot` 相对路径归一化（等 Q9）
- **E1c** "非仓库"的两种行为与空态文案（等 Q9；注意旧实现用的是 macOS 文案「当前项目不是 Git 仓库」，Windows 规格在 `03-git-and-bottom.md:141`）
- **E3a** 最小 ANSI 范围：只丢 vs 映射颜色（等 Q10）
- **E3b** 行列数怎么算（等 Q10）
- **E3c** 输入模型与 Ctrl+C（等 Q10）
- **E3d** 终端上下文菜单/搜索/分屏（等 Q10）
- **F1a** 缺字形登记表（等 Q11；若换 `AllAssets` 则大多数条目作废）
- **F2** 密度档覆盖清单（等 Q4）
- **G2** `UI-MAP.md` §2/§3 的 macOS 口径怎么处理（等 Q12）
- **G3** 提示词与 `PLAN.md §5` 合并成一份（等 Q3）
- **H2** 没有 PR 时"验收描述"落在哪（等 Q2/Q7）
- **H3** "同一文件同一时间只有一个写者"是否仍执行（等 D5；`PLAN.md:47` 有旧规则，`BLOCKERS.md` B3 已把它改成"构建单写者"）

---

## 5. 附：本轮新查出的、计划与文档里**不存在**的事实（汇总表）

| 事实 | 证据 | 对计划的冲击 |
| --- | --- | --- |
| 全量 Lucide 有 1830 字形、git 字形齐全、`AllAssets` 一行注册 | `gpui-kit-assets-0.6.6/assets/icons/`（1830 svg / 731 KB）、`src/native_assets.rs:9-12`、`src/lib.rs:36` | 推翻"缺字形只能近似替代+登记"这条硬规则 |
| `component::IconName` 只是 101 项兼容子集，`Icon` 接受任意 `IconNamed` | `gpui-component-0.6.6/src/icon.rs:13-45` | 同上 |
| gpui 支持 `Transparent / Blurred / Mica / MicaAlt` 窗口背景，且可运行期切换 | `gpui-pre-0.3.6/src/platform.rs:2225-2226`、`:2449-2464`、`window.rs:2901-2905` | 玻璃层不是"没有 API"，而是"Windows 默认关着" |
| Windows 默认 `windowTransparency: false`、Tauri `transparent: false` | `settings/config/default-settings.ts:106`、`tauri.windows.conf.json` | 推翻提示词 `:56` 的"可一比一" |
| 前端的 `transparencyEnabled` 参数 Rust 侧根本不接收 | `settings-effects.ts:127` vs `host.rs:513-515` | 该特性在 Windows 上可能从未真正生效过 |
| Core 的类型全在私有模块，宿主只能手拼 JSON 字符串 | `rust/lithe-core/src/lib.rs:11,16,17,25`；`command.rs:6-27`；`contracts.rs:10-21` | 决定宿主要自带信封层（E2） |
| `git.historyPage` 的游标背后是活的 `git log` 进程（上限 8 / 空闲 120s） | `git/history.rs:25-29,711-746` | 必须配 `historyCursorClose`（E1a） |
| `git.status.repositoryRoot` 可能是相对路径 | `git/mod.rs:6574,6659-6670` | 必须归一化（E1b） |
| "非仓库"没有专用错误码，两条命令行为还不一致 | `error.rs:11-34`；`git/mod.rs:6536-6544` vs `history.rs:634-637` | 必须先用 `git.status` 探（E1c） |
| 参考终端是 8 个 command + JSON 数字数组通道，Rust 侧零 ANSI 清洗 | `src-tauri/src/terminal.rs:85-181`、`protocol.rs:56-68`、`connection.rs:478-487` | 推翻提示词 `:50` 的"四个 command / 字节流 / 照它" |
| macOS `TerminalTransport` 没有 rows/cols，`sizeChanged` 是空实现 | `TerminalTransport.swift:37-61`、`MacTerminalTransport.swift:398` | 推翻 `UI-MAP.md:144` 的"照搬字段契约" |
| `workspace.snapshot` 每次都会清掉该 root 的搜索索引缓存 | `project/files.rs:162` → `search_index.rs:95-100` | 影响后续接搜索（E2 提醒） |
| `gpui/BLOCKERS.md` 已成为重试上限（3 次）的登记处 | `gpui/BLOCKERS.md` 规则段；`PLAN.md:453` | 提示词 `:77` 的"5 次"已作废（Q3） |
| 当前分支是 `main`，本机无 `preview`，无 `gh` | `git branch -a`；`Get-Command gh` 无结果；扫三个安装目录均不存在 | `AGENTS.md` 流程物理不可执行（Q2/H1） |
| 玻璃/透明相关 CSS 与 `.lithe-glass-*` 只在 `windowTransparency` 开启时生效 | `window-transparency.css:5,21-22` | 见 Q5 |

---

*本文只新增 `gpui/GRILL.md` 一个文件，未修改任何实现或其他文档。*

---

## 6. 唯一的 FRONTIER 问题清单（原样交给用户决策）

见 §3 的 12 行表。下面是可直接提问的形式：

1. **代码基线**：清空重写（现状：界面已回到一行「工作台根：…」，`shell_probe/workspace.rs:42-48`），还是 `git checkout HEAD -- gpui/shell/src/bin/shell_probe/` 恢复 5242 行旧实现、提交一次、再按 Windows 口径逐区域整块替换？→ **推荐后者**（"清空"当结果，不当动作）。
2. **分支策略**：本机 `gh` 不存在（扫过 PATH、`C:\Program Files\GitHub CLI`、`%LOCALAPPDATA%\Programs\GitHub CLI`）、`git ls-remote` 无凭证、**本地连 `preview` 分支都没有**（`git branch -a` 只有 `main`/`origin/main`）。→ **推荐**：从当前 `main` HEAD 开本地功能分支 + 本地提交 + PR 描述落 `BLOCKERS.md`/汇报里；不要反复尝试 `gh auth`（会吃掉 3 次上限）。
3. **真源层级**：提示词 / `PLAN.md §5`（20:26 写，声称取代 §2）/ `BLOCKERS.md`（20:25 写，声称"替代 5 次上限"）三者冲突时以谁为准？→ **推荐**：提示词为准，另两份只在写了"取代/替代"的那几条上覆盖，并**立刻回头把提示词里被覆盖的两条改掉**。
4. **规格层次**："规格来源 = `windows/tauri/src/`"是否包含 **zustand 本地状态层**与 **Monaco 宿主层**？→ **推荐**：只认**呈现层**（证据：`02-editor-sidebar.md:17` 明说 buffer/pane/pin/preview 全在前端 zustand，Core 只提供文件读写/快照/搜索/LSP）；状态层按最小子集重建。
5. **玻璃层**：不做 / 做不透明近似 / 真做透明（`AllAssets` 无关，走 `WindowBackgroundAppearance`）。→ **推荐不做**（Windows 默认关着，见 §2 的 B4）。
6. **底部窗**：确认用 `DockPlacement::Bottom`（只横跨 center 列），并决定"页签由 dock 提供还是工作台自绘"——Windows 的底部窗**没有自己的标签条**（`03-git-and-bottom.md:114-121`）。→ **推荐**：Bottom dock 只承载一个面板，切换由活动栏/命令面板驱动。
7. **阶段判据 + "3 次"**：阶段完成 = `cargo check` 0 error 0 warning + DPI 截图 + 并排对照；但交互无法验证（`PLAN.md:340`、`BLOCKERS.md` B5）。→ **推荐**：判据改成"可判定列表 + 每行标注验证方式（截图/日志/维护者实机）"；**"一次"= 一个根因的一次修复尝试，不是一次 `cargo check`**（否则并行子代理的其他文件报错会替别人吃掉次数，`PLAN.md:255` 的第 2/3/5 次正是如此）。
8. **warning 门槛**：`0 error 0 warning` 是否硬指标？→ **推荐**：`0 error` 硬；`0 warning` 只要求"本轮新增为 0，历史遗留逐条登记"。
9. **命令白名单**：`:49` 的 3 条 还是 `:106` 的 4 条？→ **推荐 6 条**：`workspace.snapshot`、`git.status`、`git.references`、`git.historyPage`、`git.historyCursorClose`、`git.commitFiles`（`git.commit` 留到"提交详情"时加）。
10. **终端范围**：不做 VT 的具体边界。→ **推荐**：定位为**输出查看器** + 最小 ANSI 处理（丢 SGR/CSI/OSC、处理 CR/BS；现成参考 `run-output-style.ts:92-149`、`output-timestamper.ts:202-238`、`rust/lithe-core/src/project/maven.rs:974-975`）；显式声明"不能跑 vim/top/PSReadLine 行内重绘"，并在验收表里写明"Ctrl+C 不可用"（或用按钮发 binary `[3]`）。
11. **图标集**：101 + 近似 vs `AllAssets` / `icon_assets!`。→ **推荐 `AllAssets`**（731 KB；同时把 `UI-MAP.md:48`、`PLAN.md:239` 的"只能用 101 字形"规则改掉）。
12. **文档**：`UI-MAP-WINDOWS.md` 是否 = 验收对照表？`gpui-kit-0.6.6-api.md` 要不要？→ **推荐**：前者就是 D1 要的"逐项对照表"（列：区域 / Windows 出处行号 / px 值 / gpui-kit 实现 / 状态），**不要**再摘一遍六份调研；后者**不产出**（`gpui/README.md:40-109` 与 `UI-MAP.md:53-63` 已经在做同一件事，第三份必然三处漂移）。
