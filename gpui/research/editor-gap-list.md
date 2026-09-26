# 编辑区实现缺口清单（对照 `windows/02-editor-sidebar.md`）

本文回答一个问题：**规格 `gpui/research/windows/02-editor-sidebar.md`（926 行，Windows 前端逆向规格）里的编辑区条目，gpui 侧实现到哪了？**

基线：分支 `feat/gpui-shell-rewrite`。判定方法：逐条读规格 §1.2/§1.3/§2.1–§2.3/§3.1/§3.2/§5.1/§5.2/§6.1/§6.3，逐条在 `gpui/crates/**` 找实现，每条带 `文件:行号`。

⚠️ 规格 §6.6「重写落地建议」**不是缺口清单**：那是"从零搭骨架"的建议（先搭 `h_resizable` + `Sidebar` + `DockArea`），骨架早就搭完了。判断缺口必须逐条比代码。

---

## 0. 一句话结论

已落地的只有四块：**标签栏（切换 / 脏点 / 关闭 / 右键 9 项）、空态、正文编辑器、24px 状态栏外框**。

规格里的**分屏、面包屑、三种横幅、24 种 buffer 渲染器、标签 pin/预览/拖拽、几乎全部可拖拽分隔条**都未实现。其中一部分是 **gpui-kit 本身没有这个能力**（第 4 节），另一部分是**纯本侧缺口**（第 3 节）——两者必须分开看，否则会去实现上游根本没有的东西。

---

## 1. 已实现

| 条目 | 证据 |
| --- | --- |
| 脏标记圆点 | `editor/src/editor_view.rs:2343-2349`（真值 `:819-821`） |
| 标签单击切换 | `editor_view.rs:2027` |
| ←→ 导航按钮 | `editor_view.rs:2239-2300` |
| 标签右键菜单 9 项 + 1 分隔线 | `editor_view.rs:2119-2223` |
| 空态（图标+标题+描述+右键「此处无任何内容」） | `editor_view.rs:2532-2597` |
| 查找栏（组件内建） | `editor_view.rs:772`、`editor/src/lib.rs:223-228` |
| 状态栏 24px 外框 / 光标真值 / 编码 UTF-8 | `workbench/src/status_bar.rs:213`、`workspace.rs:2034-2037`、`:2043` |
| 点文件树在编辑器打开 | `workspace.rs:956-960` ← `explorer/src/explorer_view.rs:670` |

## 2. 部分实现

| 条目 | 现状 | 差在哪 |
| --- | --- | --- |
| 关闭按钮可见性 | `editor_view.rs:2351-2367` | 只做"活动常显 + 非活动悬停"一档；规格是三档（`always`/`active`/`hover` + 固定恒显），没有 `tabCloseButtonVisibility` 设置项（grep 零命中） |
| 标签无障碍 | `editor_view.rs:2407` | 只有 `aria_label`；无 `role="tablist"`/`role="tab"`、无 `aria-live` 播报 |
| 横向滚轮 | 组件内建 `overflow_x_scroll` | 无 ctrl/meta 豁免、无开关（真源 `horizontalTabScroll`） |
| 标签右键菜单 | 9/14 项 | 缺 固定/取消固定、重命名（终端）、向右拆分、向下拆分、锁定编辑器组（自查表 `editor_view.rs:1653-1658`） |
| 只读锁 | `workspace.rs:2053-2058` | 只画图标，文字为空；无只读/可写切换 |
| 缩进项 | `workspace.rs:2044-2052` | 硬编码 2 |
| 文件路径项 | `workspace.rs:2009-2010` | 只有"项目名 + 图标"，不可点 |
| 侧栏高亮同步 | 切标签已实现 | 外壳 → 侧栏 `activePath` 回传未接 |

## 3. 未实现（纯本侧缺口，上游有能力或本来就要自绘）

**标签栏**：中键关闭、双击把预览转正、预览态斜体、pin、拖拽重排、拖出到其它窗格/底部面板、标签自动滚入视野（`scrollIntoView`）、标签键盘（←→ / Enter / Space / Delete）、`maxOpenTabs` 上限、终端标签重命名。

**编辑区宿主**：面包屑全系列（路径分段 / 目录下拉 / 符号面包屑 / MoreHorizontal 菜单 / 可见性设置）、外部冲突横幅、大文件「仍然启用」、恢复中 Spinner / 恢复失败重试卡片、跳转行:列 chip（状态栏 `status_bar.rs:68-69` 自己写着未实现）。

**buffer 类型**：规格说 24 种（editor/terminal/diff/image/pdf/database/webViewer/pr/issue…）；本侧 `Buffer` **没有 kind 枚举**（`editor/src/buffer.rs:280-343`），`open()` 只有"磁盘文件"一条路（`editor_view.rs:715-845`）⇒ **24 种里实现了 1 种**。

**窗格 / 分屏 / 底部面板**：分屏、分隔条拖拽、双击重置均分、拖放五区、窗格全屏、窗格锁定全未实现。侧栏是固定宽 `.w_80().flex_shrink_0()`（`workspace.rs:3236,3283-3292`）；底部 4px 热区是空 div（`:3318`）；主菜单「拆分编辑器」是 `NotWired`（`menu_bar.rs:1191-1197`）。
> ⚠️ 规格 §6.1 #2 判定"可一比一"的 `gpui_kit::base::resizable` **全仓未使用**：`h_resizable(` / `resizable_panel(` / `ResizablePanelGroup` 唯一命中是 `settings/src/dialog.rs:26` 的**注释**（代码已漂移）。

**状态栏 footer**：内存项写死 `"总计 0.0 MB · Lithe 0.0 MB"`（`workspace.rs:2059-2062`，**假数据**）、git 变更数空占位（`:2063-2065`）、项目准备状态 grep 零命中。

**侧栏强耦合项**：自动定位活动文件 `reveal_item`（API 现成 `explorer/src/lib.rs:97-99`，缺"当前活动文件"输入）、拖文件到编辑区（`explorer/src/lib.rs:88`）、树选中双底色（`:100-103`）。

**i18n**：`editor/src/buffer.rs:120`、`:129` 两处硬编码中文（locale 无对应键，`shared/src/i18n.rs:69` 已登记为"故意不走 i18n"）。§5.2 的 `editor.externalConflict` / `restoreLoadFailed` / `largeFileServicesDisabled` / `enableLargeFileServices` / `keepEditorVersion` / `loadDiskVersion` / `symbolPath` / `goToLineColumn` / `preview` / `findInFile` / `aiInlineEdit` 与 `panes.*`（yml `562-612`）**全部零引用**（因为功能没做）。

## 4. 上游 gpui-kit 无能力（不是本侧遗漏）

| 以为有 | 实际 | 规格证据 |
| --- | --- | --- |
| 标签脏点 / pin | 任何 crate 都没有 | `02:897` |
| 标签拖拽重排 | `TabBar` 无 `on_drag`；唯一参考实现在 dock `tab_panel.rs:150-581` | `02:898` |
| `role="tablist"` / live region | `tab/` 全目录 grep `Role::` 零命中 | 本次实测 |
| 标签高度 / 间距 | 组件写死标签高 28→可用 36、间距写死（`tab.rs:39,66`） | `editor/src/lib.rs:140-147` 已登记 |
| `TabBar::track_scroll` 自动露出选中标签 | 不会，必须自己 `scroll_to_item` | `02:756` |
| `BreadcrumbItem` 下拉 | 不实现 `ParentElement`，无内建下拉 | `02:900` |
| 公开的通用 `.tooltip()` 扩展 | 没有（`ManagedTooltipExt` 是 `pub(crate)`） | `02:902` |

## 5. 度量偏差（gpui 用 rem，1 单位 = 4px）

| 项 | 规格 | 实测 | 说明 |
| --- | --- | --- | --- |
| 标签栏高 | 36px | 36 ✅ | `editor_view.rs:2021` `h_9` |
| 状态栏高 | 24px | 24 ✅ | `status_bar.rs:213` |
| 标签高 | 28px | **36** ❌ | 组件写死，`editor/src/lib.rs:140-147` 已登记偏差 |
| 标签间距 | 4px | **16** ❌ | 组件写死（`tab_bar.rs:393-403`） |
| 编辑器行高 | 20px | **21** ❌ | `mono_font_size × 1.5` = 14×1.5（`input/editor.rs:14,143`） |
| 编辑器宿主 padding | 上 8 / 左 16 / 下 8 / 右 16 | **0** ❌ | `editor_view.rs:2463-2466` |
| 面包屑栏高 | 28px | 无实现 | — |

## 6. 分批计划（按 用户可见价值 ÷ 实现风险 排序）

- **E1 度量与真值（一行级样式 + 状态栏真值）**：宿主 padding 8/16/8/16；行高对齐 20px；状态栏补真值（只读文字、git 变更数、内存轮询、缩进读设置）——现在内存那项是**写死的假数据**。
- **E2 面包屑（只读路径分段版）**：不做目录下拉与 LSP 符号；`lithe.*` 键已存在。
- **E3 标签 pin**：`Buffer` 加 pinned + 排序前置 + 关闭按钮换字形（右键菜单缺项 5→4）。上游无 pin，需自绘。
- **E4 侧栏联动**：`reveal_item` 自动定位活动文件；拖文件到编辑区。
- **E5 分屏**：采用 `gpui_kit::base::resizable`（规格判定可一比一），先做左右/上下分屏 + 分隔条拖拽 + 双击重置，再做拖放五区与窗格全屏/锁定。
- **不做（需单独立项）**：24 种 buffer 渲染器（各是一种新子系统：图片/PDF/database/webViewer/pr/issue…）、标签拖拽重排（上游无 API，要照 dock 抄 500 行）、`role="tablist"`（上游无）。

## 7. 与菜单批次的交叉

- B3 的「拆分编辑器」「切换自动换行」「还原文件」「后退/前进」「上/下一个标签页」等占位项，E5/A 批次落地后可以逐条接线——**菜单里的占位项就是这份清单的入口**。
- 菜单 89 条的覆盖审计结论：**差集为空**（89 个 `menu.*` 键逐条命中，各自"已接线"或"已声明占位"），占位 51 条。但 `menu_bar.rs:1442-1443`、`:2670-2673`、`:3070`、`:1005-1006` 的**自陈数字是错的**（41+51=92≠89；本侧新增是 4 条不是 3 条；§2.10 被误引成 "❌57"，原文是 "~64"），需在下一批改对。
