# 编辑器基本功交付方案（B1 文件增删改名 / B2 文件生命周期 / B3 查找替换）

- 状态：**已实现并验证**（未 commit，分支 `feat/gpui-editor-basics`）
- 主理人：郝链成（Lithe 编辑器开发团）
- 参与：闻本立（架构）· 陆井然（交互/键位）· 颜思齐（视觉）· 娄码通（实现）
- 上游文档：
  - `reports/editor-basics-remaining-design.md`（架构 + 主理人裁决 12 项 + 后续单 §9 + 验收基线 §10）
  - `reports/editor-basics-interaction-spec.md`（交互/键位，74 条验收断言）
  - `reports/editor-basics-visual-spec.md`（视觉/token/图标/i18n/可访问性）

---

## 1. 一句话结论

**菜单栏里最贴近用户可感知的「编辑器基本功」三块已全部接通**：编辑器内查找/替换（Ctrl+F / Ctrl+R / F3 / Shift+F3）、文件生命周期五项（新建标签/新建文件/另存为/全部保存/还原文件）、explorer 文件增删改名（含行内重命名与删除确认）。菜单 `NotWired` 由 45 降到 **25**（余项全在本批范围外：LSP/Git/更新器/本地历史/分屏/帮助页）。全量 `cargo check -p lithe-gpui-app` **通过**。

---

## 2. 交付内容（按层）

### 2.1 组件库 `rust/lithe-ui`
- `src/editor/find.rs`：查找条文案 **i18n 化**（走 `i18n::text` + `ui.yml`，新增 `editor.find.*` 16 键 × 7 语言）；修 3 处既有缺陷——替换行 4px 缩进错位、正则错误文本撑破 80px 计数格（改短文案 + 原文进 tooltip）、删调试 `log::info!`；空查询不再误报 `No results`；`set_backgrounds` 加「已被查找条占用，第二用途出现时须收敛」Rustdoc。
- `locales/ui.yml`：新增 `editor.find.*`（`en-US` 值与旧硬编码逐字一致，**gallery 截图基线不变**）。
- `src/forms/inline.rs`：`Editing::open(state, value, select, check, …)` 支持**初始选区范围**（行内改名只选主名，`Foo.java → Bar.java`）与**校验回调**；`Editing::refusal()` 暴露失败原因；`Editing::begin` 原签名保留。
- `src/lists/project/{menu,mod,row}.rs`：`ProjectAction` 增 `Rename/Delete/NewFile/NewFolder`（+`icon()` 映射，两处菜单同源）；`ProjectLabels` 增 4 字段；`ProjectTree` **行内重命名**（复用 `Editing` 键控状态）+ `on_rename` / `on_rename_check` 通道；校验失败**保持编辑态 + 红框 + 同行 `colors.fg` 文字**（`uniform_list` 固定行高硬约束）；工作区根行 Rename/Delete 置灰。
- `src/editor/keys.rs`：**F1 修复**——`secondary-d` 原双绑 `SelectNext` 与 `DuplicateLine`（其中一颗是死键），`DuplicateLine` 移至 `secondary-shift-d`。

### 2.2 宿主 `rust/lithe-gpui/crates/editor`
- `EditorPane` 增 `find: Option<FindBar>`，作**正文容器内非模态绝对定位浮层**（非 `open_dialog`）；高亮 `set_backgrounds`（当前 `warning@0.45` / 其余 `@0.2`）、定位 `select`（自带滚动居中）、替换 `edit`（一批 = 一步撤销）；查询 120ms 防抖（带代次失效）；`FIND_BAR_KEY_CONTEXT`；`OpenFind/OpenFindReplace/FindNext/FindPrevious/CloseFind` action 声明于此并导出。
- `BufferKey { Disk(PathBuf), Untitled(u32) }`（`buffer.rs:180`）——**无路径未命名缓冲**的核心结构改动，`open/index_of/activate/close` 等改按 key 比较。
- 新增 `new_untitled` / `create_file`（强制 `create_new(true)`，绝不静默截断）/ `save_as` / `save_all`（全不脏=成功 no-op；脏未命名通知一次）/ `revert_active` / `remap_paths` / `forget_paths`；未命名缓冲 `Ctrl+S` 自动转另存为，且不进会话、不进 `closed`。
- `navigation.rs`：`JumpHistory::remap` / `remove_path`。

### 2.3 宿主 `crates/explorer` + `crates/workbench` + locale
- explorer：行菜单 Rename/Delete/NewFile/NewFolder + `RenameHost/DeleteHost/NewHost` 端口 + `F2` 行内改名快路径；`refresh_and_reveal`（快照落地后再 reveal，解决异步时序）；改名/删除后按交互稿 §4.5 处理选中与刷新。
- workbench：
  - `MenuAction` 增 5 个文件生命周期变体并五映射齐全；File 菜单 5 条 `NotWired → Action`（项序不动）。
  - 键位 `secondary-f` / `secondary-r` / `f3` / `shift-f3` / `escape(EditorFindBar)`；`Find`/`FindAndReplace` 从 `dispatch_editor_action` 拆出直调 `open_find`。
  - **F2 修复**：宿主 `ctrl-*` 字面量系统性改为 `secondary-*`（修 macOS「设置页显示 ⌘S、实际吃 Ctrl+S」的错位）；例外仅 `ctrl-tab/ctrl-shift-tab`（macOS ⌘Tab 被系统抢占）。
  - 守卫测试同步（`b2_actions…`、`NON_COMMAND`、`keymap_engine::menu_bar_non_command_contains`、`menu_key_actions…`）；顺带收口三处既有守卫漂移（Tools/Help 键盘快捷键入口、`MISSING_GROUPS` 13→12 退役空组）。
  - **修一处真 bug**：名称校验先 `trim()` 导致「尾随点/空格必须拒」成死代码，已按规则表修正。
- 宿主 locale：**仅新增 2 键**（`files.deleteFailed`、`editor.saveAllUntitled`）；删除确认/重名提示全部复用现成键（`fileExplorer.deleteFile(Message)` / `deleteFolder(Message)` / `fileAlreadyExists(Message)`）。

---

## 3. 关键决策（主理人裁决，详见设计稿 §8）

| # | 项 | 裁决 |
|---|---|---|
| 1 | 查找键位 | Ctrl+F / Ctrl+R（IDEA 口径）+ F3 / Shift+F3，三端统一；**字面量写 `secondary-`**（经交互设计师三条硬理由纠正架构师原 `ctrl-` 建议） |
| 2 | 高亮通道 | v1 复用 `set_backgrounds`，`set_find_matches` 记后续 |
| 3 | FindWidget 文案 | 走组件库 `i18n::text` + `locations/ui.yml` |
| 5 | newFile/newTab | newTab=未命名 scratch；newFile=项目内真实文件 |
| 6 | 另存为 | 保留原文件 |
| 7 | 无活动缓冲 | no-op + 诊断日志（不加通知键） |
| 8 | 写盘 | `std::fs`，Core 契约不动 |
| 9 | 删除 | 强确认 + 直接删，端口留 trash 可替换点 |
| 10 | explorer 重命名 | 行内编辑 |
| 11 | 「增」 | 纳入本批（New File/Folder） |
| 12 | LSP 通知 | 列后续（属 C 批） |

补充裁决：Q1 行内校验内联（照 `PromptDialog::check` 形态）；Q2 `Editing` 加初始选区；Q3 树内 New 用 `PromptDialog`；F1 本批修；F2 本批修；F4 no-op。

---

## 4. 验证结果（主理人独立复核）

| 项 | 结果 |
|---|---|
| `cargo check -p lithe-gpui-app`（整树） | **EXIT 0**（仅 1 处既有 unused-import warning） |
| `cargo test -p lithe-ui --lib --features test-support` | **1636 passed / 0 failed** |
| `cargo test -p lithe-gpui-editor` | **92 passed / 0 failed** |
| `cargo test -p lithe-gpui-explorer` | **16 passed / 0 failed** |
| `cargo test -p lithe-gpui-workbench --lib` | 250 passed / **3 failed**（`debug_transport::*`，`S1_DEBUG_TRANSPORT spawn_failed os error 231`） |
| `node tools/extract-locale.mjs --check` | 产物与真源一致 |
| 菜单 `NotWired` | 45 → **25**（余项均在本批范围外） |
| `git diff --stat` | 48 files, +4162 / −453（未 commit） |

**关于 3 条失败**：`debug_transport.rs` 经 `git status` 复核**本批 0 处改动**；失败为沙箱无法 spawn `cmd` 子进程（`ERROR_PIPE_BUSY`），与既有记忆中的沙盒限制一致，非本批回归。

---

## 5. 后续单（不占本批）

见设计稿 §9（S1–S10）与团队任务 **#6**：菜单图标补齐（23 项清单 + 2 项白留的空 + `menu_bar.rs:493` 注释失真）、`Button::tone()` Danger 前景耦合、`revertFile` 语义借用 `REFRESH_ICON`、`set_find_matches` 收敛点、菜单动态 `disabled` 通道、回收站 trash、rename/delete 的 LSP 通知、`Theme::find_bar()`、`stories.json` 假组件、`find_all` debounce。

---

## 6. 未验证 / 风险

- **未在真机跑界面**：本批只做到「编译 + 单测/集成测试绿」，**未启动应用做交互验收**（74 条断言中的 V 类需实机）。建议下一步启动 `Lithe.exe` 手工验收：`Ctrl+F/Ctrl+R/F3`、未命名缓冲保存转另存为、explorer 行内改名与删除。
- 3 条 `debug_transport` 测试因沙箱无法验证。
- 改动**尚未 commit**，如需提交建议按 `lithe-ui` / `crates/editor` / `explorer+workbench+locale` 分三个提交。
