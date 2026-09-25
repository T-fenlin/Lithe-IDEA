# Lithe Windows 前端「主菜单栏」规格逆向（→ Rust + gpui-kit 0.6.6）

> 本文由只读逆向调研生成。界面规格唯一来源为 Windows 前端 `windows/tauri/src/`（Tauri v2 + React + TypeScript + Tailwind + shadcn/Base UI）。
> 每条结论给 `相对路径:行号` 证据；无法确证处集中写在第 6 节，不做推测。
> 未修改 `windows/`、`macos/`、`rust/`、`shared/`、`extensions/`、`Plugins/`、`.agents/`，未改任何代码，未跑 cargo，未启动应用。

---

## 0. 结论摘要

| 问题 | 结论 | 证据 |
| --- | --- | --- |
| 真源文件 | **不在** `features/menu/`，也不在 `src-tauri`；是 `features/window/components/window-menu-bar.tsx`（579 行，单一文件定义全部菜单） | `windows/tauri/src/features/window/components/window-menu-bar.tsx:36,131-533` |
| 原生还是自绘 | **纯自绘**（React + `@base-ui/react` 的 `Menubar`）。Windows 侧**没有** Tauri 原生菜单：`src-tauri` 全部 `.rs` 里 `menu` 零命中 | `windows/tauri/src/ui/menubar.tsx:1,22-43`；`windows/tauri/src-tauri/src/*.rs`（`menu` grep 无命中） |
| 顶级菜单数 | **9 个**：文件 / 编辑 / 视图 / 转到 / 终端 / 运行 / 工具 / 窗口 / 帮助 | `window-menu-bar.tsx:133,200,277,365,411,427,440,462,504`；中文键 `locale.ts:7724-7732` |
| 菜单项总数 | **89 条**（88 条动作 + 1 条二级子菜单容器）+ **21 条分隔线** | 见 §2.11 |
| 「常驻」形态 | `compactMenuBar=false` → 菜单栏**画在标题栏那一行内部**（不是标题栏下面单独一条） | `title-bar.tsx:230-232`；`window-menu-bar.tsx:552` |
| 「点左上角图标下拉」形态 | `compactMenuBar=true`（**默认值**）→ 标题栏最左是一个 `ListIcon` 图标按钮，点开后菜单栏作为**浮动层出现在标题栏正下方**（`absolute top-full left-0 mt-1 w-max`） | `title-bar.tsx:207-229`；`window-menu-bar.tsx:539,549`；`settings/config/default-settings.ts:105` |
| 「原生菜单栏」设置 | **在 Windows 上永远不生效**（连设置项都不渲染）——`nativeMenuBar` 只在 `!IS_MAC && !IS_WINDOWS && !IS_LINUX` 时出现 | `appearance-settings.tsx:512-528`；`title-bar.tsx:79`；`utils/platform.ts:31-33` |
| gpui-kit 推荐做法 | **自绘 24px 胶囊容器 + 20px 顶级项 + 每项 `PopupMenu`**（与 `gpui/UI-MAP-WINDOWS.md:514` 同口径）；「常驻」与「图标下拉」共用同一份菜单数据。**不要**用 `NativeMenu` 做常驻栏 | §4.5 |
| 键盘可达性 | 自绘方案需自己接 `key_context` + `left/right/escape`（可照抄 `AppMenuBar` 的 91 行）；`AppMenuBar` 现成但改不动像素 | §4.6 |

### ⚠️ 关于「12 个顶级菜单」的截图

需求里给出的英文清单 `File Edit View Navigate Code Refactor Build Run Tools VCS Window Help`（Navigate / Code / Refactor / Build / VCS）是 **JetBrains IDEA 的菜单**，在 Lithe 的 Windows 真源里**没有任何对应物**（`locale.ts:7724-7732` 只有 9 条顶级键，`window-menu-bar.tsx` 只有 9 个顶级项）。
需求里给出的中文清单（文件 编辑 视图 转到 终端 运行 工具 窗口 帮助）与真源**逐条一致**。本文件一律以实现为准，见 §6 第 1 条。

---

## 1. 真源在哪 / 原生 vs 自绘 / 两种形态各走哪条路

### 1.1 文件与组件

| 角色 | 文件 | 证据 |
| --- | --- | --- |
| 菜单数据 + 渲染（唯一真源） | `windows/tauri/src/features/window/components/window-menu-bar.tsx` | `:36`（默认导出组件）、`:131-533`（`menus` useMemo）、`:535-576`（渲染） |
| 菜单原语 | `windows/tauri/src/ui/menubar.tsx` | `:1`（`import { Menu, Menubar as BaseMenubar } from "@base-ui/react"`）、`:243-256`（导出 12 个子组件） |
| 宿主（标题栏） | `windows/tauri/src/features/window/components/title-bar/title-bar.tsx` | `:42`（import）、`:205-233`（三分支选择形态） |
| 宿主（外壳） | `windows/tauri/src/features/layout/components/main-layout.tsx` | `:292`（`<TitleBarWithSettings …/>`，菜单栏因此位于标题栏内） |
| 事件桥（菜单 → 行为） | `windows/tauri/src/features/window/hooks/use-menu-events.ts` + `use-menu-events-wrapper.ts` | `use-menu-events.ts:29-71`（监听 38 个 `menu_*` 事件）、`use-menu-events-wrapper.ts:296-298`（`onExecuteCommand` → `keymapRegistry.executeCommand`） |
| 快捷键展示 | `windows/tauri/src/features/keymaps/components/keybinding.tsx` | `:13`（`keybindingToDisplayParts`）、`:22-34`（非 Mac 用 `+` 连接，和弦用 `then`） |

### 1.2 原生 or 自绘：**只有自绘，没有原生**

- `windows/tauri/src-tauri/src/` 下共 20 个 `.rs` 文件，全部 `menu` / `Menu` grep **零命中**；`main.rs` 也没有调用 Tauri 的 `MenuBuilder` / `set_menu`。（`src-tauri/src/*.rs` 列表：`main.rs`、`platform.rs`、`core.rs`、`terminal.rs`、`debug.rs`、`run.rs`、`maven.rs`、`watcher.rs`、`document.rs`、`file_events.rs`、`host.rs`、`logging.rs`、`lsp.rs`、`memory.rs`、`diagnostics.rs`、`secure_storage.rs`、`ai_commit.rs` 等）
- 全仓库 `MenuBuilder` / `tauri::menu` / `set_menu` / `NSMenu` 的命中全部落在 `macos/`（SwiftUI）与 `CodeEditorView.swift`，与 Windows 前端无关。
- 因此 `settings.nativeMenuBar` 这条设置为**死配置**：
  - 它只在 `!IS_MAC && !IS_WINDOWS && !IS_LINUX` 时渲染开关（`appearance-settings.tsx:512-528`），而 `IS_MAC/IS_WINDOWS/IS_LINUX` 是三分枚举（`utils/platform.ts:31-33`），**三个平台全被排除** → 开关永不显示；
  - 其消费者 `shouldUseNativeMenuBar = !isWindows && !isLinux && nativeMenuBar`（`title-bar.tsx:79`）在 Windows/Linux 上恒为 `false`；
  - `toggleNativeMenuBar()` 在 `isMac() || isLinux()` 时直接返回，而唯一能显示开关的平台又不满足 `!settings.nativeMenuBar`（`window-command-actions.ts:48-56`）；
  - Rust 侧也没有 `toggle_menu_bar` 命令（前端 `invoke("toggle_menu_bar")` 的 4 处调用，见 `use-menu-events-wrapper.ts:353`、`window-command-actions.ts:55`、`view-actions.tsx:203`、`appearance-settings.tsx:523`，都 `catch(console.error)`）。
  - 默认值 `nativeMenuBar: false`（`settings/config/default-settings.ts:104`）。
- 结论：**Windows 规格里不存在"原生菜单栏"这一支**，`nativeMenuBar` 只需在 gpui 侧理解为"不存在的分支"，不必实现。

### 1.3 两种可见形态各走哪条路

`TitleBar` 用**同一个** `menuItem` JSX 变量 + 同一个 `WindowMenuBar` 组件表达两种形态，差别只在 `compactMenuBar` 与 `compactFloating` 两个开关：

```text
title-bar.tsx:205-233
menuItem = (!isMacOS && !shouldUseNativeMenuBar)
  ? ( compactMenuBar
        ? <div className="relative">                      // ← 形态 A：图标下拉
            <Tooltip content={t("window.menu")} side="bottom">
              <Button onClick={handleCompactMenuToggle} … aria-expanded={…}><ListIcon /></Button>
            </Tooltip>
            {isCompactMenuVisible ? <WindowMenuBar … compactFloating onCompactClose={…} /> : null}
          </div>
        : <WindowMenuBar activeMenu={…} setActiveMenu={…} /> )   // ← 形态 B：常驻
  : null
```

| | 形态 A：左上角图标下拉（截图里"标题栏下面那条"） | 形态 B：常驻 |
| --- | --- | --- |
| 开关 | `settings.compactMenuBar === true`（**默认**） | `settings.compactMenuBar === false` |
| 证据 | `default-settings.ts:105`（`compactMenuBar: true`）；`title-bar.tsx:207-229` | `title-bar.tsx:230-232` |
| 触发器 | `Button variant=ghost size=icon-xs` + `<ListIcon />`，`aria-label/aria-expanded = t("window.menu")`（中文「菜单」，`locale.ts:7903`），提示在底部 | `title-bar.tsx:209-219` |
| 位置 | 浮动层：根 `div` 加 `compactFloating && "absolute top-full left-0 mt-1 w-max"`；菜单栏本体加 `h-auto w-max flex-nowrap rounded-2xl border border-border bg-background/95 px-1 py-1 shadow-(--shadow-popover) backdrop-blur-sm` | 内联：根 `div` 加 `absolute inset-0`；菜单栏本体加 `h-full rounded-none border-none bg-transparent px-2 py-0` |
| 证据 | `window-menu-bar.tsx:538-541,547-553` | 同左 |
| 视觉 | 浮动**胶囊**：9 个顶级项横排一行，圆角 `rounded-2xl`=**14.4**、内边距 4、上间距 `mt-1`=**4** | 与标题栏同一行；菜单栏本体高 `h-6`=**24**、`px-2`=8、无边框无底色 |
| 打开/关闭 | 点图标切换 `isCompactMenuVisible`；任何一项被点后 `closeMenu()` 一并收起（`onCompactClose`） | 顶级项悬停即开（`MenubarTrigger openOnHover`）；`activeMenu` 受控 |
| 证据 | `title-bar.tsx:165-173`（toggle/close）；`window-menu-bar.tsx:47-50,105-112`；`ui/menubar.tsx:84`（`openOnHover`） | 同左 |

> **重要（与需求措辞的差异）**：真源里**没有**"标题栏下方单独常驻一条"的形态。截图里"标题栏下面那条"就是形态 A 的浮动胶囊——它靠 `absolute top-full mt-1` 定位在标题栏正下方，宽度是 `w-max`（只包住 9 个顶级项），不是通栏。形态 B 的常驻菜单栏与标题栏**共享同一行 40px**（占其中左侧一段）。

### 1.4 一个菜单项被点后走哪条路（3 条，混合存在）

| 路径 | 触发方式 | 证据 | 说明 |
| --- | --- | --- | --- |
| ① Tauri 窗口事件 | `handleClickEmit(event, payload)` → `currentWindow.emitTo(label, event, payload)` → `use-menu-events` 监听 | `window-menu-bar.tsx:105-112`；`use-menu-events.ts:29-71` | 38 个 `menu_*` 事件（含 `menu_theme_change` 带 payload） |
| ② 命令注册表 | `handleCommand(id)` → `emit "menu_execute_command"` → `keymapRegistry.executeCommand(id)` | `window-menu-bar.tsx:119-124`；`use-menu-events.ts:59-61`；`use-menu-events-wrapper.ts:296-298` | payload 是命令 ID，如 `workbench.newTab`、`editor.duplicateLine`、`file.saveAll`（`window-menu-bar.tsx:135,157,244`；注册表定义见 `features/keymaps/commands/command-registry.ts:139,167,412`） |
| ③ 直接调前端/原生 API | `createAppWindow()`、`getCurrentWindow().minimize()/maximize()/setFullscreen()`、`exit(0)`、`invoke("reopen_current_webview_devtools")` | `window-menu-bar.tsx:126-129,114-117,464-501,195` | 窗口类与退出类不经注册表 |

统计（按 `onClick` 分类）：走 ② 的有 24 条、走 ① 的有 33 条、走 ③ 的有 8 条（含条件项）。gpui 侧不需要保留这个三分法，但要知道**同一份菜单里 3 条通路混用**，接口设计上建议统一收敛成"一个 Action 表"。

---

## 2. 逐项清单

### 2.0 读表约定

| 列 | 含义 |
| --- | --- |
| 中文文案 | 取自 `windows/tauri/src/i18n/locale.ts` 的 zh-CN 目录（`menu.*` 全部键在 `:7724-7822`），与 gpui 侧已生成的 `gpui/crates/shared/locales/lithe.zh-CN.yml:6588-6785` 逐字一致（前缀 `lithe.`） |
| i18n 键 | 写 `menu.xxx`；gpui 侧对应 `lithe.menu.xxx`（**已全部在产物 locale 里**，见 §4.7） |
| 快捷键 | 写源文件里的原始字符串（`mod` 在 Windows 上 = `Ctrl`，`utils/platform.ts:51-54`）。带空格的是**和弦**，界面上用 `then` 连接（`keybinding.tsx:24`） |
| 分隔线 | 写它出现在该项**之后**（源文件里 `<MenubarSeparator />` 的位置） |
| 子菜单 | 该顶级菜单内是否有二级子菜单 |
| 条件 | 禁用 / 显示条件 |
| gpui | 见 §2.11 的标记：`✅` 已有对应实现 / `🟡` 有底层能力但缺动作或快捷键 / `❌` 无能力 |

**没有任何菜单项按"当前 buffer / 是否有项目"禁用**——真源的禁用只有 §2.11 那 4 处能力开关。这是一个需要明确的设计事实（gpui 侧照做即可，不要自作主张加 `Save` 灰化）。

### 2.1 文件 File（`window-menu-bar.tsx:133-199`，19 项，3 分隔线）

| # | 中文文案 | i18n 键 | 快捷键 | 分隔线 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 新建标签页 | `menu.newTab` | `mod+n` | | ❌ | `:135-137` |
| 2 | 新建窗口 | `menu.newWindow` | `mod+shift+n` | | ❌ | `:138-140` |
| 3 | 新建文件 | `menu.newFile` | — | | ❌ | `:141-143` |
| 4 | 打开文件夹 | `menu.openFolder` | `mod+o` | | 🟡（`explorer` 有"打开文件夹"文案键 `lithe.titleProject.openFolder`，但没有打开文件夹的动作） | `:144-146` |
| 5 | 关闭文件夹 | `menu.closeFolder` | — | ✔ | ❌ | `:147-149` |
| 6 | 保存 | `menu.save` | `mod+s` | | ✅ `SaveBuffer`（`gpui/crates/editor/src/lib.rs:161-163`，绑在 `ctrl-s`；处理在 `workspace.rs:981-985`） | `:151-153` |
| 7 | 另存为... | `menu.saveAs` | `mod+shift+s` | | ❌ | `:154-156` |
| 8 | 全部保存 | `menu.saveAll` | `mod+alt+s` | | ❌ | `:157-159` |
| 9 | 还原文件 | `menu.revertFile` | — | | ❌ | `:160-162` |
| 10 | 显示本地历史 | `menu.showLocalHistory` | — | ✔ | ❌ | `:163-165` |
| 11 | 关闭标签页 | `menu.closeTab` | `mod+w` | | 🟡 编辑器页签有关闭按钮（`editor_view.rs`），无动作/快捷键 | `:167-169` |
| 12 | 关闭窗口 | `menu.closeWindow` | `mod+shift+w` | | ❌（gpui 有标题栏三键，但那走 `WindowControlArea`，不是 action） | `:170-175` |
| 13 | 关闭所有标签页 | `menu.closeAllTabs` | — | | ❌ | `:176-178` |
| 14 | 关闭其他标签页 | `menu.closeOtherTabs` | — | | ❌ | `:179-181` |
| 15 | 关闭已保存标签页 | `menu.closeSavedTabs` | — | | ❌ | `:182-184` |
| 16 | 关闭左侧标签页 | `menu.closeTabsToLeft` | — | | ❌ | `:185-187` |
| 17 | 关闭右侧标签页 | `menu.closeTabsToRight` | — | | ❌ | `:188-190` |
| 18 | 重新打开已关闭标签页 | `menu.reopenClosedTab` | `mod+shift+t` | | ❌ | `:191-193` |
| 19 | 退出 | `menu.quit` | `mod+q` | | ❌ | `:195-197`，直接 `exit(0)` |

### 2.2 编辑 Edit（`:200-276`，19 项，4 分隔线）

| # | 中文文案 | i18n 键 | 快捷键 | 分隔线 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 撤销 | `menu.undo` | `mod+z` | | 🟡 `gpui-base` 输入上下文已绑 `ctrl-z`→`Undo`（`gpui-base-0.6.6/src/input/base/state.rs:288`），但只在输入框获得焦点时 | `:202-204` |
| 2 | 重做 | `menu.redo` | `mod+shift+z` | ✔ | 🟡 同上（`state.rs` 有 `Redo` 动作） | `:205-207` |
| 3 | 剪切 | `menu.cut` | `mod+x` | | 🟡 `ctrl-x`→`Cut`（`state.rs:254`） | `:209-211` |
| 4 | 复制 | `menu.copy` | `mod+c` | | 🟡 `ctrl-c`→`Copy`（`state.rs:250`） | `:212-214` |
| 5 | 粘贴 | `menu.paste` | `mod+v` | | 🟡 `ctrl-v`→`Paste`（`state.rs:258`） | `:215-217` |
| 6 | 全选 | `menu.selectAll` | `mod+a` | ✔ | 🟡 `ctrl-a`→`SelectAll`（`state.rs:246`） | `:218-220` |
| 7 | 查找 | `menu.find` | `mod+f` | | 🟡 `ctrl-f`→`Search`（`state.rs:298`），`editor/lib.rs:152` 已注明归 `Input` 上下文 | `:222-224` |
| 8 | 查找并替换 | `menu.findAndReplace` | `mod+alt+f` | | 🟡 `ctrl-h`→`Replace`（`state.rs:302`）——**能力在，快捷键与真源不同**（真源 `mod+alt+f`） | `:225-227` |
| 9 | 切换注释 | `menu.toggleComment` | `mod+/` | | ❌ | `:228-230` |
| 10 | 快速修复... | `menu.quickFix` | `mod+.` | | ❌ | `:231-233` |
| 11 | 触发参数提示 | `menu.triggerParameterHints` | `mod+shift+space` | | ❌ | `:234-239` |
| 12 | 显示悬停信息 | `menu.showHover` | `mod+k mod+i`（和弦） | ✔ | ❌ | `:240-242` |
| 13 | 复制行 | `menu.duplicateLine` | `mod+d` | | ❌ | `:244-246` |
| 14 | 删除行 | `menu.deleteLine` | `mod+shift+k` | | ❌ | `:247-249` |
| 15 | 上移行 | `menu.moveLineUp` | `alt+up` | | ❌ | `:250-252` |
| 16 | 下移行 | `menu.moveLineDown` | `alt+down` | | ❌ | `:253-255` |
| 17 | 格式化文档 | `menu.formatDocument` | `mod+alt+l` | | ❌ | `:256-261` |
| 18 | 格式化所选内容 | `menu.formatSelection` | `mod+k mod+f`（和弦） | ✔ | ❌ | `:262-267` |
| 19 | 命令面板 | `menu.commandPalette` | `mod+shift+p` | | ✅ `OpenCommandPalette`（`gpui/crates/workbench/src/command_palette.rs:262,271`） | `:269-274` |

### 2.3 视图 View（`:277-364`，18 项，5 分隔线，1 个二级子菜单）

| # | 中文文案 | i18n 键 | 快捷键 | 分隔线 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 切换活动侧栏 | `menu.toggleActivitySidebar` | `mod+b` | | 🟡 左活动栏常驻、没有收起态（`workspace.rs:230-232` 明确记录 `toggle-sidebar` 被有意排除） | `:279-284` |
| 2 | 切换辅助侧栏 | `menu.toggleSecondarySidebar` | `mod+e` | | 🟡 同上 | `:285-287` |
| 3 | 切换终端 | `menu.toggleTerminal` | `mod+j` | ✔ | ✅ 能力已有：`CommandId::ToggleTerminal`（`workspace.rs:617-636`，含懒建会话），但**没有 `ctrl-j` 绑定** | `:288-290` |
| 4 | 全局搜索 | `menu.globalSearch` | `mod+shift+f` | | ❌ 全局搜索界面未实现 | `:292-297` |
| 5 | 诊断 | `menu.diagnostics` | `mod+shift+j` | ✔ | 🟡 底部工具窗有 `BottomPaneKind::Diagnostics` 但内容是占位（`workspace.rs:958-962`） | `:298-303` |
| 6 | 文件资源管理器 | `menu.fileExplorer` | `mod+shift+e` | | 🟡 左栏项目树已实现（`Explorer`），但没有"打开该视图"的动作 | `:305-310` |
| 7 | 源代码管理 | `menu.sourceControl` | `mod+shift+g` | | 🟡 左栏「更改」项即此视图，无动作 | `:311-316` |
| 8 | GitHub | `menu.github` | — | | ❌ | `:317-319` |
| 9 | 运行和调试 | `menu.runAndDebug` | — | ✔ | 🟡 底部工具窗 `BottomPaneKind::Run` 为占位 | `:320-322` |
| 10 | 拆分编辑器 | `menu.splitEditor` | — | | ❌ | `:324-326` |
| 11 | 切换缩略图 | `menu.toggleMinimap` | — | | ❌ | `:327-329` |
| 12 | 切换自动换行 | `menu.toggleWordWrap` | `alt+z` | | ❌ | `:330-332` |
| 13 | 切换行号 | `menu.toggleLineNumbers` | — | | ❌ | `:333-335` |
| 14 | 切换空白字符显示 | `menu.toggleRenderWhitespace` | — | | ❌ | `:336-338` |
| 15 | 放大 | `menu.zoomIn` | `mod+=` | ✔ | ❌ | `:340-342` |
| 16 | 缩小 | `menu.zoomOut` | `mod+-` | | ❌ | `:343-345` |
| 17 | 重置缩放 | `menu.resetZoom` | `mod+0` | ✔ | ❌ | `:346-348` |
| 18 | **主题**（子菜单） | `menu.theme` | — | | ✅ 能力已有：`SettingsStore::set_theme_explicit`（`gpui/crates/settings/src/store.rs:269`）+ 两条命令（`workspace.rs:614-616,670-694`）；但 gpui 侧只有"切浅色/切深色"两条，真源是**列出全部主题** | `:350-362` |

**子菜单内容（动态，见 §3.1）**：`themes.map(theme => <MenubarItem onClick={() => handleClickEmit("menu_theme_change", theme.id)}>{theme.name}</MenubarItem>)`，`window-menu-bar.tsx:353-360`。
**无勾选态**：当前主题不打勾（`MenubarItem` 没有 checked/indicator API，`ui/menubar.tsx:142-160`）。

### 2.4 转到 Go（`:365-410`，11 项，3 分隔线）

| # | 中文文案 | i18n 键 | 快捷键 | 分隔线 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 快速打开 | `menu.quickOpen` | `mod+p` | | ❌（`quickOpen` 相关键存在，界面未实现） | `:367-369` |
| 2 | 转到行 | `menu.goToLine` | `mod+g` | ✔ | ❌（`gpui/crates/workbench/src/status_bar.rs:67` 明确记"未实现"） | `:370-372` |
| 3 | 后退 | `menu.goBack` | `ctrl+alt+left` | | 🟡 编辑器页签有后退按钮（`lithe.tabs.goBack`），无动作/快捷键 | `:374-376` |
| 4 | 前进 | `menu.goForward` | `ctrl+alt+right` | ✔ | 🟡 同上 | `:377-382` |
| 5 | 转到定义 | `menu.goToDefinition` | `f12` | | ✅ `NavigateToDefinition`（`gpui/crates/editor/src/lib.rs:130,163`） | `:384-386` |
| 6 | 转到实现 | `menu.goToImplementation` | `mod+f12` | | ❌ | `:387-392` |
| 7 | 转到类型定义 | `menu.goToTypeDefinition` | — | | ❌ | `:393-395` |
| 8 | 转到引用 | `menu.goToReferences` | `mod+b` | | ❌ | `:396-398` |
| 9 | 重命名符号 | `menu.renameSymbol` | `f2` | ✔ | ❌ | `:399-401` |
| 10 | 下一个标签页 | `menu.nextTab` | `mod+alt+right` | | 🟡 编辑器页签栏已实现，无动作/快捷键 | `:403-405` |
| 11 | 上一个标签页 | `menu.previousTab` | `mod+alt+left` | | 🟡 同上 | `:406-408` |

### 2.5 终端 Terminal（`:411-426`，4 项，无分隔线；**顶级项受能力开关禁用**）

| # | 中文文案 | i18n 键 | 快捷键 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- |
| 1 | 新建终端 | `menu.newTerminal` | — | ✅ `TerminalPane::new_tab`（`gpui/crates/terminal/src/session.rs:345`）；页签栏有 `+` 按钮；**无 action/快捷键** | `:413-415` |
| 2 | 向右拆分终端 | `menu.splitTerminalRight` | `mod+d` | ❌ 无拆分能力（`terminal_view.rs:695` 明确记 `terminal.close` 未绑） | `:416-418` |
| 3 | 向下拆分终端 | `menu.splitTerminalDown` | `mod+shift+d` | ❌ | `:419-421` |
| 4 | 关闭终端 | `menu.closeTerminal` | — | ✅ `TerminalPane::close_tab`（`session.rs:451`）；无 action | `:422-424` |

**顶级项禁用条件**：`!isBackendCapabilityAvailable("terminal")` → `disabled` + `title=BACKEND_UNAVAILABLE_TOOLTIP`（"待开发"）。当前值为 `terminal: true`，所以默认**可用**。证据：`window-menu-bar.tsx:558-567`；`config/backend-capabilities.ts:1,14`。

### 2.6 运行 Run（`:427-439`，3 项，无分隔线；**顶级项受能力开关禁用**）

| # | 中文文案 | i18n 键 | 快捷键 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- |
| 1 | 开始调试 | `menu.startDebugging` | `f5` | ❌（`BottomPaneKind::Run` 为占位） | `:429-431` |
| 2 | 停止调试 | `menu.stopDebugging` | `shift+f5` | ❌ | `:432-434` |
| 3 | 切换断点 | `menu.toggleBreakpoint` | `f9` | ❌ | `:435-437` |

**禁用条件**：`!isBackendCapabilityAvailable("debugger")`；当前 `debugger: true` → 可用。证据：`window-menu-bar.tsx:558-567`；`backend-capabilities.ts:6`。

### 2.7 工具 Tools（`:440-461`，4 项，2 分隔线）

| # | 中文文案 | i18n 键 | 快捷键 | 分隔线 | 条件 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 数据库 | `menu.databases` | — | ✔ | **禁用**：`!isBackendCapabilityAvailable("database")`，且 `database: false` → **默认灰化**，`title="待开发"` | ❌ | `:442-448`；`backend-capabilities.ts:5` |
| 2 | Web 检查器 | `menu.webInspector` | `mod+alt+i` | ✔ | | ❌（`invoke("reopen_current_webview_devtools")`，`:114-117`） | `:450-452` |
| 3 | 首选项 | `menu.preferences` | — | | | ✅ `OpenSettings`（`gpui/crates/settings/src/dialog.rs:80,89`，绑 `ctrl-,`） | `:454-456` |
| 4 | 键盘快捷键 | `menu.keyboardShortcuts` | — | | | ❌ | `:457-459` |

> 注：`menu.preferences` 的中文是「首选项」**不是**「设置…」，且源里没有 `Ctrl+,` 标签（`shortcut` 未传）。命令面板里对应的是「首选项：打开设置」（`lithe.commandPalette.actions.open-settings.label`，`gpui/crates/shared/src/i18n.rs:349`）。
> 「键盘快捷键」在**工具**与**帮助**两个菜单里各出现一次（`:457` 与 `:509`），走同一个命令 `workbench.openKeyboardShortcuts`。

### 2.8 窗口 Window（`:462-503`，Windows 上 4 项，2 分隔线）

| # | 中文文案 | i18n 键 | 快捷键 | 分隔线 | 条件 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 最小化 | `menu.minimize` | `alt+f9` | | | 🟡 标题栏三键已实现（走 `WindowControlArea`，`gpui/crates/workbench/src/title_bar.rs:140-192`），无 action | `:464-472` |
| 2 | 最大化 | `menu.maximize` | `alt+f10` | ✔（`!IS_LINUX` 时才画） | | 🟡 同上 | `:473-481` |
| 3 | 切换菜单栏 | `menu.toggleMenuBar` | `alt+m` | ✔（`!IS_LINUX`） | **仅 `!IS_LINUX`** | ❌（且 Rust 侧无 `toggle_menu_bar` 命令） | `:482-490` |
| 4 | 切换全屏 | `menu.toggleFullscreen` | `f11` | | | 🟡 gpui 侧无 action（平台能力在） | `:491-501` |

Linux 上第 3 项与两条分隔线整体不渲染（`{!IS_LINUX && (<>…</>)}`，`:482-490`）。

### 2.9 帮助 Help（`:504-530`，7 项，2 分隔线）

| # | 中文文案 | i18n 键 | 分隔线 | gpui | 证据 |
| --- | --- | --- | --- | --- | --- |
| 1 | 文档 | `menu.documentation` | | ❌（打开 `getServiceUrls().docsUrl`，`use-menu-events-wrapper.ts:299-302`） | `:506-508` |
| 2 | 键盘快捷键 | `menu.keyboardShortcuts` | | ❌ | `:509-511` |
| 3 | 新增功能 | `menu.whatsNew` | | ❌（`onWhatsNew`，`use-menu-events-wrapper.ts:307-309`） | `:512-514` |
| 4 | 更新日志 | `menu.changelog` | ✔ | ❌（打开 GitHub releases，`use-menu-events-wrapper.ts:303-306`） | `:515-517` |
| 5 | 报告 Bug | `menu.reportBug` | | ❌（拼环境信息进剪贴板 + 开 issue 模板，`use-menu-events-wrapper.ts:310-332`） | `:519-521` |
| 6 | 请求新功能 | `menu.requestFeature` | ✔ | ❌（开 feature 模板，`use-menu-events-wrapper.ts:333-336`） | `:522-524` |
| 7 | 检查更新 | `menu.checkForUpdates` | | ❌（`checkForUpdates` + toast，`use-menu-events-wrapper.ts:337-344`） | `:526-528` |

### 2.10 顶级容器与项度量（gpui 复刻用）

| 元素 | Tailwind | 值 | 证据 |
| --- | --- | --- | --- |
| 菜单栏根（外层 div） | `z-100000 flex flex-col` | z-index **100000** | `window-menu-bar.tsx:537-538` |
| 菜单栏本体 | `flex h-6 items-center gap-0.5 rounded-full border border-border/70 bg-background/65 px-0.5 py-0.5` | 高 **24**、gap **2**、全圆角、1px 边框、px/py **2** | `ui/menubar.tsx:35-38` |
| 顶级项 | `ui-text-sm flex h-5 shrink-0 select-none items-center whitespace-nowrap rounded-md px-1.5 text-subtle-foreground … hover:bg-accent/50 hover:text-foreground … data-popup-open:bg-accent/80` | 高 **20**、px **6**、圆角 **6.4**、字号 **13** | `ui/menubar.tsx:85-88` |
| 顶级项 icon 尺寸 | Button 基类 `[&_svg:not([class*='size-'])]:size-3.5`（`ListIcon` 走这条） | **14×14** | `ui/button.tsx:9`（见 `01-shell.md` §2.3 同款引用） |
| 下拉面板 | `z-10031 w-max min-w-60 max-w-[min(480px,calc(100vw-16px))] rounded-xl border border-border bg-surface/95 p-1 shadow-(--shadow-popover) backdrop-blur-sm` | 最小宽 **240**、p **4**、圆角 **11.2** | `ui/menubar.tsx:125-127` |
| 菜单项 | `min-h-7 … gap-6 rounded-lg px-2.5 py-1.5` | 最小高 **28**、gap **24**、px **10**、py **6**、圆角 **8** | `ui/menubar.tsx:146-147` |
| 菜单项快捷键文本 | `font-mono ml-auto shrink-0 text-subtle-foreground/75 ui-text-sm` | 字号 **13**，自动右对齐 | `ui/menubar.tsx:175-176` |
| 分隔线 | `-mx-1 my-1 h-px bg-border` | 高 **1**、上下外边距 **4**、左右外扩 **4** | `ui/menubar.tsx:165-166` |
| 子菜单内容 | 同下拉面板，`side="right"` `sideOffset=4`，`z-10050` | — | `ui/menubar.tsx:211-240` |
| 浮动（compact）胶囊 | `h-auto w-max flex-nowrap rounded-2xl border border-border bg-background/95 px-1 py-1 shadow-(--shadow-popover) backdrop-blur-sm` | 圆角 **14.4**、p **4** | `window-menu-bar.tsx:548-549` |
| 浮动定位 | `absolute top-full left-0 mt-1 w-max` | 上间距 **4** | `window-menu-bar.tsx:539` |
| 窗口置顶（菜单打开时） | Windows/Linux：打开菜单时 `setAlwaysOnTop(true)`，关闭后还原 | — | `window-menu-bar.tsx:46,52-103` |

顶级项文案取 `t(\`menu.${menuName.toLowerCase()}\`)`（`window-menu-bar.tsx:569`）——**键名由英文键名小写推导**，即 JSX 里的 `File`/`Go` 等键名与 i18n 键是耦合的。

### 2.11 计数与「已有实现」统计

| 顶级菜单 | 项数 | 分隔线 | 子菜单 | 禁用/条件 |
| --- | --- | --- | --- | --- |
| 文件 File | 19 | 3 | — | — |
| 编辑 Edit | 19 | 4 | — | — |
| 视图 View | 18 | 5 | 「主题」**是** | 主题子菜单动态生成 |
| 转到 Go | 11 | 3 | — | — |
| 终端 Terminal | 4 | 0 | — | 顶级项按 `terminal` 能力禁用 |
| 运行 Run | 3 | 0 | — | 顶级项按 `debugger` 能力禁用 |
| 工具 Tools | 4 | 2 | — | 「数据库」按 `database` 能力禁用 |
| 窗口 Window | 4 | 2 | — | 「切换菜单栏」仅 `!IS_LINUX` |
| 帮助 Help | 7 | 2 | — | — |
| **合计** | **89**（88 动作 + 1 子菜单容器） | **21** | **1** | 4 处能力/平台条件 |

按 gpui 侧能力分档（依据 §2 各表最右列）：

| 分档 | 条数 | 明细 |
| --- | --- | --- |
| ✅ **已有对应实现**（动作 + 快捷键都在） | **5** | 保存 `mod+s`；命令面板 `mod+shift+p`；首选项 `Ctrl+,`；转到定义 `f12`；主题切换（能力在，形态不同，真源是子菜单列全部主题、gpui 只有浅/深两命令） |
| 🟡 **有底层能力，缺 Action / 快捷键** | **~20** | 切换终端、撤销/重做/剪切/复制/粘贴/全选/查找/替换（8 条，均在 `Input` 上下文绑好，只在输入框聚焦时生效）、新建/关闭终端、关闭标签页、后退/前进、上/下一个标签页、文件资源管理器、源代码管理、诊断、运行和调试、最小化/最大化/切换全屏 |
| ❌ **无能力** | **~64** | 文件菜单除保存外的 18 条、编辑菜单的 10 条语言服务/行操作、视图的全局搜索/GitHub/拆分编辑器/缩略图/换行/行号/空白字符/缩放 3 条、转到 6 条、终端拆分 2 条、运行全部 3 条、工具的数据库与 Web 检查器与键盘快捷键、窗口的切换菜单栏、帮助全部 7 条 |

精确到 89 条的 `✅/🟡/❌` 逐项标记见 §2.1–§2.9 各表。

---

## 3. 最近使用 / 动态项

### 3.1 菜单栏内部：只有 **1 处**动态生成 —— 视图 → 主题子菜单

- 规则：把**主题注册表当前全部主题**逐条列成一个二级子菜单，`key={theme.id}`，点击 `emit("menu_theme_change", theme.id)`。
- 数据来源：`useRegisteredThemes()`（`window-menu-bar.tsx:44`）→ `useMemo(() => themeRegistry.getAllThemes(), [registryVersion])`，用 `useSyncExternalStore` 订阅注册表版本号（`extensions/themes/use-registered-themes.ts:5-17`）。
- 落地行为：`onThemeChange` 时若 `syncSystemTheme` 为真，**先关掉跟随系统再写 theme**（`use-menu-events-wrapper.ts:287-295`）——这与 gpui 侧 `set_theme_explicit` 的语义一致（`gpui/crates/settings/src/store.rs:269`、`workspace.rs:663-694`）。
- 勾选态：**没有**（当前主题不打勾）。

### 3.2 菜单栏里**没有**最近文件 / 打开的编辑器列表

- 文件菜单**没有**「打开最近使用」项（`window-menu-bar.tsx:133-199` 逐项核对）。
- 「最近打开的项目」不在菜单栏，而在**标题栏最左的项目下拉**（`TitleProjectMenu`），它是**另一个组件、另一套数据**：

| 动态分组 | 规则 | 数据来源 | 证据 |
| --- | --- | --- | --- |
| 顶部 3 条固定动作 | 新建项目… / 打开… / 克隆仓库… | — | `title-project-menu.tsx:172-192` |
| **打开的项目** | 当前打开的**全部**项目标签（不过滤、不截断），当前项显示勾选（`CheckIcon`）且点击无效（`if (project.isActive) return`），切换中全部 `disabled` | `useWorkspaceTabsStore.projectTabs` | `title-project-menu.tsx:111-112,199-213`；`utils/title-project-menu-model.ts:42-43` |
| **最近项目** | 先**剔除已打开的项目**（按路径比较 `areProjectTabPathsEqual`），再取前 `MAX_RECENT_PROJECTS`；空时显示「没有最近项目」 | `useRecentFoldersStore.recentFolders` | `title-project-menu-model.ts:44-49`；`title-project-menu.tsx:113,221-234` |
| 上限 | `MAX_RECENT_PROJECTS = 12`（**未固定项**上限；固定项不计入、排在前面） | — | `features/file-system/utils/recent-folders.ts:3,36-42` |
| 排序 | 固定项优先，其余按 `lastOpenedAt` 降序 | — | `recent-folders.ts:26-34` |
| 持久化 | zustand `persist`，storage key `lithe-code-recent-folders`，`partialize` 只存 `recentFolders` | — | `features/file-system/stores/recent-folders.store.ts:191-195` |
| 项目徽标 | 有自定义图标用图标（28×28）；否则取名字首字母 + 按名字哈希选 5 色之一 | — | `title-project-menu-model.ts:6-31`；`title-project-menu.tsx:47-69` |

- 项目下拉的形态与项目行度量见 `gpui/research/windows/01-shell.md` §2.3（另一份文档已覆盖），本文不重复。

### 3.3 对 gpui 的启示

- **菜单栏本身可以是静态表**（唯一例外是主题列表）。这意味着第一版不需要"动态菜单重算"机制就能覆盖 88/89 条，主题子菜单用 `SettingsStore` 的主题列表生成即可。
- 「最近项目」如果要做，归**标题栏项目下拉**（阶段 1 的 `ProjectTab` 目前只有一个项目，`workspace.rs:503-506`），不要塞进菜单栏。
- gpui 侧还有一条**OS 级**的"最近使用"通路可用（真源未用）：`App::add_recent_document`（`gpui-pre-0.3.6/src/app.rs:2550`）与 `App::update_jump_list`（`:2556`，Windows 跳转列表）。第一版不建议接。

---

## 4. gpui-kit 0.6.6 侧怎么做

> 本节全部结论来自本地解压源码与本地文档，未凭印象推测。crate 路径记为 `<REG>` = `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837`（`gpui-kit 0.6.6` 依赖 `gpui-component 0.6.6`、`gpui-base 0.6.6`、`gpui-pre 0.3.6`；见 `gpui/Cargo.lock:2485-2488`）。

### 4.1 文档与组件盘点

| 组件 | 本地文档 | crate 源码 | 是否适合本需求 |
| --- | --- | --- | --- |
| `menu::PopupMenu` / `PopupMenuItem` | `gpui/docs/gpui-kit/0.6.6/zh-CN/component/menu.md`（全文） | `<REG>/gpui-component-0.6.6/src/menu/popup_menu.rs`（1533 行） | ✅ **主力**：下拉面板、子菜单、分隔线、勾选、禁用、图标、快捷键提示、键盘导航全都有 |
| `menu::AppMenuBar` | **文档里没有独立页**，只在 `component/title-bar.md:47-66` 的示例里出现 | `<REG>/.../src/menu/app_menu_bar.rs`（365 行） | ⚠️ **唯一现成的"常驻菜单栏"**，但度量写死、无图标（见 §4.2） |
| `menu::ContextMenu` / `ContextMenuExt` | `menu.md:22-38` | `<REG>/.../src/menu/context_menu.rs` | 不适用（右键菜单） |
| `menu::DropdownMenu`（ext trait） | `menu.md:40-64,74-87` | `<REG>/.../src/menu/dropdown_menu.rs`（319 行） | ✅ 单按钮下拉；`dropdown_menu_with_anchor` 用于"点图标弹一层" |
| `native_menu::NativeMenu` | 无 zh-CN 文档页 | `<REG>/.../src/native_menu/{mod,windows,macos,fallback}.rs` | ❌ 不适合常驻栏；适合"点图标弹 OS 菜单"（见 §4.3） |
| `component::TitleBar` | `component/title-bar.md`（全文） | `<REG>/.../src/title_bar.rs`（434 行） | ❌ 本项目已明确不用（见 §4.4） |
| `PopupMenu` 的 Action 机制 | `menu.md:66-72,201-219` | `popup_menu.rs:1119-1144,1301` | ✅ 用 Action 定义菜单项 → 快捷键提示**自动**从 keymap 解析并显示在同一帧 |

`component.md` 的组件总目录里**没有** "menubar" 条目；`zh-CN/component/` 下**没有** `menubar.md` / `dropdown-menu.md` / `native-menu.md`（只有 `menu.md`、`dropdown_button.md`、`title-bar.md`）。也就是说 **gpui-kit 0.6.6 没有独立的"菜单栏"文档**，`AppMenuBar` 是"有实现、文档只在 title-bar 示例里带一句"的状态。

### 4.2 `AppMenuBar`：唯一现成的常驻菜单栏，但改不动像素

| 项目 | 事实 | 证据 |
| --- | --- | --- |
| 定位 | 源码注释原文：`/// The application menu bar, for Windows and Linux.` | `app_menu_bar.rs:25` |
| 数据来源 | `GlobalState::global(cx).app_menus()` —— **不是** `App::set_menus()` | `app_menu_bar.rs:49-53` |
| **⚠️ 关键坑** | `cx.set_menus(..)`（GPUI 官方 API）只把菜单交给**平台层**（`platform.set_menus`），**不会**写进 `GlobalState::app_menus`。要喂 `AppMenuBar` 必须显式 `GlobalState::global_mut(cx).set_app_menus(vec![…])` | `gpui-pre-0.3.6/src/app.rs:2526-2529`；`gpui-base-0.6.6/src/global_state.rs:108-115` |
| 菜单项类型 | `gpui::OwnedMenu`，由 `Menu::new(name).items([MenuItem::…]).owned()` 构造 | `gpui-base-0.6.6/src/global_state.rs:3,16,113`；`gpui-pre-0.3.6/src/platform/app_menu.rs:15-45` |
| 项的能力 | `MenuItem::action(name, action)` / `.checked(bool)` / `.disabled(bool)` / `MenuItem::separator()` / `MenuItem::submenu(menu)` | `app_menu.rs:108,113,126,176,198` |
| 刷新 | `AppMenuBar::reload(cx)` 重新读列表 | `app_menu_bar.rs:47-62` |
| 顶级项度量 | 写死 `Button::new("menu").small().py_0p5().compact().ghost().label(name)` → `Size::Small` = **`h_6()`（24px）**、`px_1p5()`（6px） | `app_menu_bar.rs:265-271`；`button/button.rs:629-632` |
| 栏容器度量 | `h_flex().size_full().gap_x_1()` → 栏内间距 **4** | `app_menu_bar.rs:121-132` |
| **与真源的差** | 真源：容器 24px / 顶级项 **20px** / gap **2**；`AppMenuBar`：顶级项 **24px** / gap **4** → **项高 +4px、间距 +2px，且无任何 hook 可覆盖**（`AppMenu` 的 `render` 里度量是硬编码的） | 真源 `ui/menubar.tsx:36,86` vs `app_menu_bar.rs:121-132,265-271` |
| 下拉项 | 由 `PopupMenu::with_menu_items` 生成 → 走 `menu_with_check_and_disabled(name, checked, action, disabled)`，**没有 icon 参数**；`OwnedMenuItem::Submenu` 支持二级 | `popup_menu.rs:784-826`（尤其 `:802-807`） |
| 下拉项图标 | ❌ 做不了（`OwnedMenuItem` 无 icon 字段；`From<gpui::Menu>` 映射时 icon 一律 `None`，`native_menu/mod.rs:338-344`） | 同上 |
| 自动滚动 | 面板项数 > 20 时自动 `scrollable = true`（真源 File 菜单 19 项、Edit 19 项，都不到阈值） | `popup_menu.rs:821-823` |
| 键盘（栏级） | `init` 绑 `escape`→`Cancel`、`left`→`SelectLeft`、`right`→`SelectRight`，上下文 `"AppMenuBar"` | `app_menu_bar.rs:16-23,124` |
| ⚠️ 栏级方向键的**局限** | `on_move_left/right` 在 `selected_index` 为 `None` 时**直接 return** → **无法用方向键从零打开菜单**；必须先点开（或 Tab+Enter）某一路菜单 | `app_menu_bar.rs:64-75,77-88` |
| 键盘（打开某一项） | 触发器是 `Button`（`tab_stop` 默认 `true`、`.track_focus(..)`），`handle_trigger_click` 只在**非鼠标** ClickEvent 上 toggle → **Tab 聚焦 + Enter/Space 可打开** | `app_menu_bar.rs:220-231,272-281`；`button/button.rs:273,466,773-775`；`ClickEvent::Keyboard` 定义 `gpui-pre-0.3.6/src/interactive.rs:290-297` |
| 拖拽保护 | 触发器 `on_mouse_down` 里 `window.prevent_default(); cx.stop_propagation();`，注释原文 `// Stop propagation to avoid dragging the window.` → 专为**放进标题栏拖拽区**设计 | `app_menu_bar.rs:272-280` |
| 下拉项键盘 | `enter`/`escape`/`up`/`down`/`left`/`right`，上下文 `"PopupMenu"` | `popup_menu.rs:21-30` |
| `action_context` | 打开菜单时记住当前焦点，菜单项快捷键提示与 `Ctrl+` 类语义按它解析；关闭时把焦点还回去（有单测覆盖） | `app_menu_bar.rs:94-111,174-201,322-364` |

### 4.3 `NativeMenu`：**只适合"点图标弹一层"，不能做常驻栏**

- 定义原文：`NativeMenu` 由**操作系统**渲染，用于**超出窗口边界**的弹出；用法是 `NativeMenu::new().menu(..).show(position, window, cx)` —— **以某个坐标弹出一次**，没有"常驻"概念。证据：`native_menu/mod.rs:1-23,196-224`。
- Windows 实现是 `CreatePopupMenu()` + **`TrackPopupMenuEx(menu, flags, x, y, hwnd, …)`**（`TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_NONOTIFY`）。
  - **全文件没有 `SetMenu`** → 不可能成为窗口的常驻菜单栏（Win32 的常驻菜单栏必须 `SetMenu(hwnd, hmenu)`）。
  - 证据：`native_menu/windows.rs:25-27,114-120,204`；`SetMenu` 在该文件 grep 无命中。
- 因此：`NativeMenu` 适合用来做**"点左上角图标 → 在图标下方弹出一层 OS 菜单"**（若确实想要 OS 观感），**不适合**"常驻在界面上"的需求；而且它一次只弹一层，要做真源那种"9 个顶级项横排 + 各自二级"要自己拼 9 次 `show`，交互上并不等价。
- 另注：`NativeMenu` **不支持超长/滚动**，且图标走 `AssetSource`（`native_menu/mod.rs:107-128,226-257`）——与 `gpui/research/app-icon-and-assets.md:233-234` 记录的 `AssetSource` 消费者一致。
- 结论：**本需求不用 `NativeMenu`**。

### 4.4 `TitleBar` 是否预留了菜单栏槽位？—— 预留了，但本项目已决定不用它

- **预留了**：`TitleBar` 是 `ParentElement`，`child(..)` 追加的元素被放进 `h_flex().id("bar").flex_1().window_control_area(WindowControlArea::Drag)`。官方文档示例正是把 `AppMenuBar` 放进第一个 child：
  - `component/title-bar.md:8`（原文"支持插入菜单栏"）、`:47-66`（`带菜单栏` 示例：`.child(div().flex().items_center().child(AppMenuBar::new(window, cx)))`）
  - 源码：`<REG>/.../src/title_bar.rs:302-306`（`extend`）、`:371-378`（子元素槽 = `h_flex` + `flex_1` + `Drag` 命中区）
  - ⚠️ 注意 `title-bar.md:55` 写的是 `AppMenuBar::new(window, cx)`，而**实际签名是 `AppMenuBar::new(cx)`**（`app_menu_bar.rs:34`）——文档示例有一处笔误。
- **但本项目不能用它**（已有决策，不是新结论）：`gpui/crates/workbench/src/title_bar.rs:17-29` 明确记录了三条理由——组件内部**无条件**再画一组自带窗口三键（会和自绘的 56×40 三键叠成两组）、子元素槽是 `h_flex().flex_1()`、外层自带 `pl(12)` 与 1px 下边框，都与 Windows 标题栏度量不符。
- 因此本需求要落在**已有的自绘标题栏**上：`gpui/crates/workbench/src/title_bar.rs:76-97` 的 `title_bar(project_name, window, cx)`，目前结构是 `h_flex().justify_between().child(drag_region(project_name)).child(window_controls(...))`。

### 4.5 推荐做法与理由

**推荐：自绘菜单栏容器 + 每个顶级项一个 `PopupMenu`（与 `gpui/UI-MAP-WINDOWS.md:514` 同口径）。**

```text
title_bar() 左组（现状只有 drag_region）
├─ menu_bar()                          ← 新增：h_flex 24px 胶囊 + 9 个自绘顶级项
│    ├─ trigger "文件" ─┐
│    ├─ trigger "编辑"  ├─ 每项：自绘 div(20px 高, px 6, 圆角 6.4)
│    ├─ …              │   点击/悬停 → anchored() 内显示对应的 Entity<PopupMenu>
│    └─ trigger "帮助" ─┘
│    自绘 h_flex 承担键盘：key_context("lithe-menu-bar") + left/right/escape
└─ drag_region(project_name)          ← 保持不变（WindowControlArea::Drag）
```

理由（每条都有证据）：

1. **只有自绘才能逐值对齐真源**。真源容器的度量组合（高 24 的胶囊 + 高 20 的顶级项 + gap 2 + `rounded-md` 6.4 + `bg-background/65` + `border-border/70`）在 `AppMenuBar` 里**没有对应的覆盖点**（`app_menu_bar.rs:265-271` 硬编码 `Button::small()`=24px、`:129` `gap_x_1()`=4px）。`UI-MAP-WINDOWS.md:514` 也已经这么判断：*"Lithe 是自绘 24px 胶囊 + 20px 项 → 用 `menu::popup_menu::PopupMenu` 自搭"*。
2. **`PopupMenu` 已经把所有"难的部分"做完了**：二级子菜单（`PopupMenu::submenu`，`popup_menu.rs:694`）、分隔线（`:680` 一族）、勾选（`menu_with_check`）、禁用（`menu_with_disabled`）、图标（`menu_with_icon`）、链接项、自定义元素、滚动、以及**完整的键盘导航**（`popup_menu.rs:21-30`：`enter/escape/up/down/left/right`）+ **快捷键提示自动从 keymap 解析**（`:1119-1144,1301`）。自绘方案实际只需写"容器 + 9 个触发器 + 打开/关闭/左右切换"。
3. **快捷键提示是 Action 驱动的免费产物**：`menu.md:66-72,201-219` 明确"推荐优先用 `Action` 定义菜单行为"，`popup_menu.rs:1143-1144` 用 `Kbd::binding_for_action_in` / `global_binding_for_action` 反查键位。若菜单项走 Action，则 gpui 侧只要把 88 条命令登记成 Action + keymap，菜单上的"Ctrl+S"等提示**不需要手写字符串**（真源是 `shortcut="mod+s"` 手写字符串，`:151`——这一点 gpui 侧反而更优）。
4. **拖拽保护可以少写**：如果菜单栏放在 `drag_region` **之外**（作为它的兄弟），就不依赖 `prevent_default`/`stop_propagation` 这套；如果按官方示例放进拖拽区内，则必须照 `app_menu_bar.rs:272-280` 那样消费按下事件（`gpui/crates/workbench/src/title_bar.rs:26-29` 记录了"一旦有监听器消费掉按下事件，系统就收不到该次点击"的机制）。**推荐放成兄弟节点**，风险最小。

**次选：直接用 `AppMenuBar`**（如果接受 +4px 项高 / +2px 间距的视觉偏差）。它能省掉约 300 行（栏容器 + 键盘 + 焦点归还 + 悬停切换的完整实现见 `app_menu_bar.rs:119-298`），并自带 `action_context` 的单测（`:322-364`）。代价：① 顶级项比真源高 4px、间距宽 2px；② 下拉项**不能带图标**（`popup_menu.rs:802-807`）；③ 想动态换菜单（如主题列表）要重新 `set_app_menus` + `reload`。

**不推荐：`NativeMenu`**（理由见 §4.3：无 `SetMenu`，只弹一次）与 **`component::TitleBar`**（理由见 §4.4）。

### 4.6 键盘可达性：当前方案能做到什么

| 能力 | 自绘方案 | `AppMenuBar` 现成 | 证据 |
| --- | --- | --- | --- |
| 用键盘打开菜单（Alt/F10 之类助记键） | ❌ 做不到——gpui 的 keymap 是 **action + key_context** 模型，**没有 Alt 助记键 / 下划线助记符**的概念（`popup_menu.rs:21-30` 只有普通键位绑定；`gpui-pre` 的 `Menu`/`MenuItem` 也没有 mnemonic 字段，`app_menu.rs:76-104`） | ❌ 同样做不到（`init` 只绑 escape/left/right，`app_menu_bar.rs:16-23`） | — |
| **Tab 聚焦顶级项 → Enter/Space 打开** | ✅ 需自己保证触发器是 tab stop（用 `Button` 即默认 `tab_stop=true`，`button/button.rs:273,466`） | ✅ 现成（`handle_trigger_click` 只在非鼠标点击时 toggle，`app_menu_bar.rs:220-231`） | `ClickEvent::Keyboard`：`gpui-pre-0.3.6/src/interactive.rs:290-297` |
| 打开后 **← / →** 在顶级菜单间移动 | ✅ 需自己接（照 `app_menu_bar.rs:64-88`） | ✅ 现成，但**必须先有菜单被打开**（`selected_index.is_none()` 时直接 return，`app_menu_bar.rs:65-67,78-80`） | 同左 |
| **↑ / ↓** 在菜单项间移动 | ✅ 现成（`PopupMenu` 自带） | ✅ 现成 | `popup_menu.rs:25-26` |
| **← / →** 进/出子菜单 | ✅ 现成（`PopupMenu` 自带） | ✅ 现成 | `popup_menu.rs:27-28` |
| **Enter/Space** 激活当前项、**Escape** 关闭、**Tab** 关闭并移焦 | ✅ 现成（`menu.md:414-422` 的键盘表逐条对应 `popup_menu.rs:23-29`） | ✅ 现成 | `menu.md:416-422` |
| 关闭后焦点归还 | ✅ 需自己写 | ✅ 现成（记 `action_context` 并在关闭时 `focus` 回去，`app_menu_bar.rs:100-107`，有单测 `:322-364`） | 同左 |
| 菜单项右侧显示快捷键 | ✅ `PopupMenu` 自动从 keymap 反查（前提是该项用 `Action` 且键已绑） | ✅ 同 | `popup_menu.rs:1119-1144,1301`；`menu.md:217-219` |

**一言以蔽之**：**用键盘打开菜单**在当前 gpui/gpui-kit 方案下**做不到**（没有 Alt 助记键机制）；但"Tab 聚焦 + Enter 打开 → 方向键导航 → Enter 激活 → Escape 关闭 → 焦点归还"这条链路**两条路都能做到**，自绘方案需要自己写约 40–60 行。

### 4.7 文案（i18n）现状：**已经准备好了**

- 真源 `menu.*` 的 99 条键（9 条顶级 + 90 条项）**已经**在 gpui 产物 locale 里，前缀 `lithe.`：
  - `gpui/crates/shared/locales/lithe.zh-CN.yml:6588-6785`（`menu.file` … `menu.checkForUpdates`，逐条 `^  menu\.` 命中 99 条）
  - 生成链：`gpui/tools/extract-locale.mjs:34`（读 `windows/tauri/src/i18n/locale.ts`）→ `:39`（写 `gpui/crates/shared/locales/`）；调用入口 `lithe_gpui_shared::tr("lithe.menu.…")`（`gpui/crates/shared/src/i18n.rs:89-91`）
- ⚠️ `lithe.menu.*` **尚未进** `i18n.rs` 的 `WIRED` 测试表（`:157-379` 逐条列了已接线的键，没有 `menu.*`）。按该文件 `:151-153` 的规则，新增接线时**必须把键补进 `WIRED`**，否则拼错不会被测出来。
- 另外 `menu.toggleAiChat` / `menu.toggleVim` / `menu.toggleMenuBar` 三处的实际使用情况：前两个 `menu_*` 事件在 `use-menu-events.ts:49,51` 有监听，但**前端没有任何地方 emit**（grep `menu_toggle_ai_chat` / `menu_toggle_vim` 只命中监听处）——它们是为 macOS SwiftUI 菜单（`macos/Sources/Lithe/LitheApp.swift:541,637` 的 `CommandMenu("Navigate"/"History")`）准备的死监听。`menu.toggleAiChat` / `menu.toggleVim` 在 locale 的 `menu.*` 段里**也不存在**。→ gpui 侧不必实现这两条。

---

## 5. 落地评估

### 5.1 逐条分档

**A. 现在就能做（有真源文案 + gpui 侧动作/键位已存在，只差菜单项接线）—— 5 条**

| 菜单项 | 复用 |
| --- | --- |
| 文件 → 保存（`mod+s`） | `SaveBuffer`（`editor/lib.rs:162`）+ `workspace.rs:981-985` 的处理器 |
| 编辑 → 命令面板（`mod+shift+p`） | `OpenCommandPalette`（`command_palette.rs:262,271`） |
| 工具 → 首选项 | `OpenSettings`（`settings/dialog.rs:80,89`）；⚠️ 真源未标 `Ctrl+,`，若要标需确认 |
| 转到 → 转到定义（`f12`） | `NavigateToDefinition`（`editor/lib.rs:163`） |
| 视图 → 主题（子菜单） | `SettingsStore::set_theme_explicit`（`settings/store.rs:269`）；⚠️ gpui 侧只有浅/深两条命令（`workspace.rs:309-317`），要做成"列全部主题"需扩表 |

**B. 现在能做（动作现成、只缺菜单项与键位登记）—— 12 条**

| 菜单项 | 复用 |
| --- | --- |
| 视图 → 切换终端（`mod+j`） | `CommandId::ToggleTerminal`（`workspace.rs:617-636`） |
| 视图 → 显示状态栏 | `CommandId::ToggleStatusBar`（`workspace.rs:647-658`）——真源里**没有**这一项，属 gpui 侧已有命令 |
| 视图 → Maven | `CommandId::ToggleMaven`（`workspace.rs:637-646`）——同上，真源里没有 |
| 视图 → 外观页设置 | `CommandId::OpenAppearanceSettings`（`workspace.rs:610-613`） |
| 终端 → 新建终端 / 关闭终端 | `TerminalPane::new_tab` / `close_tab`（`session.rs:345,451`），需补 Action |
| 编辑 → 剪切/复制/粘贴/全选/撤销/重做 | `gpui-base` 输入上下文（`state.rs:246,250,254,258,288`），但**只在输入框聚焦时有效**；做成菜单项需决定是否转发到 `Input` |
| 编辑 → 查找/替换 | `ctrl-f`→`Search` / `ctrl-h`→`Replace`（`state.rs:298,302`）；⚠️ 真源是 `mod+f` / `mod+alt+f` |

**C. 建议本轮不做（无能力，且工作量不小）—— 其余 ~72 条**

- 文件菜单 18 条（新建/另存为/全部保存/还原/本地历史/关闭 7 种标签页/重开/退出/新建窗口）
- 编辑菜单的语言服务与行操作 10 条（切换注释、快速修复、参数提示、悬停、复制/删除/上下移行、格式化×2）
- 视图菜单的结构类 12 条（全局搜索、诊断内容、文件资源管理器、源代码管理、GitHub、运行和调试、拆分编辑器、缩略图、换行、行号、空白字符、缩放×3）
- 转到菜单 6 条（快速打开、转到行、转到实现/类型定义/引用、重命名符号；后退/前进/上下标签页可归入 B 但依赖编辑器导航动作）
- 终端菜单的拆分 2 条
- 运行菜单 3 条（调试器）
- 工具菜单的数据库（默认灰化）、Web 检查器、键盘快捷键
- 窗口菜单的切换菜单栏（真源自身在 Windows 上就是坏的）
- 帮助菜单 7 条（文档/新增功能/更新日志/Bug/新功能/检查更新）——全部依赖外部 URL 与更新服务

理由统一：这些动作在 `gpui/crates/` 里没有对应实现（§2 各表标记为 ❌），而且 gpui 侧的"能力表"目前只有 7 条命令（`COMMAND_ORDER`，`workspace.rs:309-317`）。**先有动作，后有菜单**——反着做会得到一堆"点了没反应"的项。

### 5.2 第一版范围建议

**做**：

1. **菜单栏外壳与两种形态**
   - 形态 B（常驻）：在 `gpui/crates/workbench/src/title_bar.rs` 的左组加菜单栏；按 §2.10 的度量复刻（容器 24px 胶囊 / 顶级项 20px / gap 2 / 圆角 6.4 / 字号 14——⚠️ 字号 13 不在 gpui 档位上，沿用本项目已确认的 `text_sm()`=14px，理由见 `workspace.rs:117-118`）。
   - 形态 A（图标下拉）：按真源 `compactMenuBar` 默认值 = `true`——**默认也应该给图标形态**；点 `ListIcon` 后在标题栏下方弹**同一条菜单栏**（浮动胶囊，圆角 14.4、`top-full mt-1`）。两形态共用一份菜单数据，只换容器样式与定位。
   - 数据源：静态表（9 个顶级 × 88 条动作），主题子菜单动态。
2. **9 个顶级菜单全部画出来**，项按 §2 逐条对齐（中文文案直接复用 `lithe.menu.*`——键已在 locale 里，只差接 `tr()` 与补 `WIRED`）。
3. **能做的项接真行为**：§5.1 A + B 共约 17 条；其余项**按真源原样显示但禁用**（`disabled(true)`），而不是隐藏——这样菜单结构与真源可比对，也不会出现"点了没反应"。⚠️ 这是**有意偏离真源**（真源只有 §2.11 那 4 处禁用），需在实现注释里写明理由。
4. **键盘**：Tab 聚焦 + Enter 打开 + ←/→ 切换顶级 + ↑/↓ 选项 + Enter 激活 + Esc 关闭 + 焦点归还（照 `app_menu_bar.rs:64-111` 的结构自写；`PopupMenu` 自带项级导航）。
5. **进度可见性**：沿用本项目已有的 `S1_*` 诊断口径（如 `S1_MENU_OPEN id=… state=…`），让"常驻/浮动两形态"和"哪些项可用"能被机器验证（参考 `workspace.rs:320-322` 的 `diagnose_run`）。

**不做**：§5.1 C 的全部；`NativeMenu`；`component::TitleBar`；Alt 助记键（机制不存在）；原生菜单栏（`nativeMenuBar` 在真源里是死配置）。

**明确记录为已知偏差**（若第一版用 `AppMenuBar` 而非自绘）：顶级项 24px（真源 20）、栏内 gap 4（真源 2）、下拉项无图标。

---

## 6. 未确认的点

1. **「12 个顶级菜单」的来源不明**。需求里给的英文清单（`Navigate` / `Code` / `Refactor` / `Build` / `VCS`）在 Lithe 的 Windows 真源里完全没有对应物：`locale.ts:7724-7732` 只有 9 条顶级键，`window-menu-bar.tsx` 只有 9 个顶级项。这批英文菜单**逐条对得上 JetBrains IDEA**（Navigate=转到、Code=代码、Refactor=重构、Build=构建、VCS=版本控制）。**未确认**：那两张截图里到底是 Lithe 还是 IDEA；如果维护者确实要 IDEA 的 12 项菜单，那就不是"逆向 Windows 前端"而是"新增 3 个顶级菜单"，需要单独确认（本文件按 Lithe 真源的 9 项写）。
2. **「常驻」的确切含义未确认**。真源只有两种形态，且二者都不产生"标题栏下方单独一条常驻栏"（§1.3）。截图里"标题栏下面那条"最可能是 `compactMenuBar=true` 的**浮动胶囊**（默认值）。**未确认**：维护者要的"常驻"是 (a) 真源的形态 B（与标题栏同一行）还是 (b) 一种真源里不存在的新形态（标题栏下方独立通栏）。这决定要不要偏离真源加一条新布局。
3. **左上角"那个图标"指哪一个**。标题栏最左有两组东西：① `compactMenuBar=true` 时的 `ListIcon` 按钮（`title-bar.tsx:209-219`）；② `TitleProjectMenu` 的 `logo.png` + 项目名触发器（`title-project-menu.tsx:151-164`，Logo 20×20 + 项目名 + 折叠箭头），点它开的是**项目列表**而不是 9 个主菜单。**未确认**：需求说"点左上角图标下拉出这些选项"指的是 ①（对，能下拉出 9 个主菜单）还是 ②（不对，那是项目列表）；若指 ②，则是新需求（要把项目菜单与主菜单合并成一个入口）。
4. **`Ctrl+,` 是否要显示在「首选项」上**。真源 `menu.preferences` 没传 `shortcut`（`window-menu-bar.tsx:454-456`），但 gpui 侧 `OpenSettings` 确实绑了 `ctrl-,`（`settings/dialog.rs:80`）。需求里把"设置…对应 `Ctrl+,`"当成已知项，与真源显示不一致。**未确认**：以真源（不显示）还是以 gpui 现状（显示）为准。
5. **菜单项的「禁用」策略**。真源只按后端能力禁用 4 处，**不按编辑状态禁用**（§2.0）。第一版若把 ~72 条无能力项统一置灰，是与真源的有意偏离。**未确认**：维护者接受"灰一片"还是希望"本轮先只画能做的项"（后者视觉更干净但结构对不上截图）。
6. **视图 → 主题子菜单是否要勾选当前主题**。真源无勾选（§2.3），gpui 侧 `menu_with_check` 支持。**未确认**：是否借机补上（属改进而非复刻）。
7. **字号 13 → 14 的影响**未在本文件重新核算。`--ui-text-chrome` 真值是 13px，gpui 档位只有 12/14，本项目已按 `text_sm()`=14 处理（`workspace.rs:117-118`、`title_bar.rs:55-57`）。菜单栏沿用同一口径，但**未确认**维护者是否希望菜单栏单独破例（例如用 `text_xs()`=12 反而更接近 13）。
8. **`AppMenuBar` 的 `action_context` 与 `PopupMenu` 快捷键提示的实测行为未验证**。文档 `menu.md:217-219` 与源码 `popup_menu.rs:1119-1144` 说"快捷键按 action 实际派发位置解析"，但**我没有运行**（约束禁止跑 cargo / 启动应用），所以"菜单上能自动显示 Ctrl+S"这条是**基于源码的推断**，未实测。
9. **`title-bar.md:55` 的 `AppMenuBar::new(window, cx)` 与源码 `new(cx)` 不一致**（文档笔误，已确认）；但由此**未确认**是否存在另一个头文件/版本差异化的构造函数（我只读了 0.6.6 一个版本，`gpui-component-0.6.1` 未比对）。
10. **`toggle_menu_bar` 的 Rust 实现是否存在**：`01-shell.md:460` 记"`windows/tauri/src-tauri/` 内未找到同名 `#[tauri::command]`"。我复核了 `src-tauri` 全部 `.rs` 的 `menu` grep（零命中），与之一致；但**未确认**它是否通过 `platform_invoke` 转发到 `rust/` 的 Core 命令（本轮未读 `rust/` 的命令表）。这只影响"窗口→切换菜单栏"这一条，不影响本需求主线。
11. **`PopupMenu` 在 `anchored()` 里由自绘触发器驱动的可行性未实测**。`AppMenuBar` 用的是 `deferred(anchored().anchor(TopLeft).snap_to_window_with_margin(px(8.)).child(div().size_full().occlude().top_1().child(popup_menu)))`（`app_menu_bar.rs:284-297`）；自绘方案若照抄这段，`occlude()` / `top_1()` 的行为需要实机验证（未运行）。
12. **浮动形态（形态 A）在 gpui 里怎么定位到"标题栏正下方"未确认**。真源靠 CSS `absolute top-full left-0 mt-1`（`window-menu-bar.tsx:539`）；gpui 的 `anchored()` 需要 `Anchor::BottomLeft` 之类的锚点与 `snap_to_window_with_margin`，**具体组合未验证**（`menu.md:74-87` 只演示了 `Anchor::TopRight` 这种四个角的枚举，未说明相对任意触发器的完整语义）。
