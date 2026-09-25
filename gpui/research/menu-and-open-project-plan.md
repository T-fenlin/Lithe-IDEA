# 菜单栏 / 标签页 / 打开项目 —— 可执行规格（2026-09-26，维护者已逐条拍板）

> 本文是 grilling 会话的产物。**每一条决策都来自维护者的明确回答**（Q 编号对应会话里的问题），
> 事实与行号来自三份侦察：`menu-tabs-and-open-file.md`、`menu-dropdown-dismissal.md`、
> `open-project-and-windows.md`。开工前如需变更，改这份文档，不要在代码里悄悄偏离。

## 0. 术语（这一轮踩过坑，必须分开用）

| 词 | 指什么 | 不要混用 |
| --- | --- | --- |
| **菜单标题胶囊** | 菜单栏上的 `文件 / 编辑 / 视图 / …` 按钮 | 不要叫"文件胶囊" |
| **编辑器标签页** | 编辑区顶部 `文件名 ×` 那些标签 | —— |
| **项目标签条** | 左侧项目树上方、多项目时出现的标签条 | —— |

## 1. 锁定的决策（维护者原话口径）

| # | 决策 |
| --- | --- |
| Q2 | 未实现的菜单项：**渲染 + 可点**，点了在**状态栏左侧**给"尚未接入：缺 X"（几秒自动消失） |
| Q3 | 未实现项**不绑快捷键**；只有真做出来的功能才绑**并显示**键位 |
| Q6 | 提示通道 = **状态栏左侧临时消息** ＋ 可 grep 诊断（通知区留到通知模型做出来之后） |
| Q7 | 下拉收起 **1–5 全要**：点空白 / `Esc` / 再点同标题 / 悬停切换 / 选中项后关闭 |
| Q8 | **三批都做**（B2/B3/B4/B5 全要），没有的先提示未实现 |
| Q9 | 打开项目/文件 **a/b/c/d 都做**：打开文件 · 打开文件夹（项目）· 最近项目 · 克隆仓库 |
| Q10 | "禁死项"单测**翻面**：从"不许有死项"改成"**每项要么已接线，要么在声明里给出缺什么**" |
| Q11 | 标签页四修：非活动标签悬停显示 `×` · 补 `stop_propagation` ＋ 订正错误注释 · File 菜单加齐关闭项 · 实测 24px vs 26px 裁剪 |
| Q12 | **「打开…」= 打开项目（选文件夹）**，与「打开文件夹」同义；"打开文件"另给独立入口 |
| Q13 | 只给真功能绑并显示键位，走 `.action(..)` 让上游解析（不手写键位文案） |
| Q14 | 声明形态 **1b**（理由挂在每个菜单项上，不填单测不过）＋ 文案粒度 **2d**（按能力种类，≈12 个键，每句说清缺什么） |
| Q15 | 五批顺序 B1→B5，**B1 先做**，B4 单独验收 |
| Q16 | **克隆仓库先占位**（不做最小版） |
| Q17 | 关闭路径统一 `close(reason)` ＋ `S1_MENU_CLOSE reason=dismiss\|toggle\|escape\|select\|switch`；订正错误文档、删不可达 `state=run` |
| Q18 | **「打开文件」进文件菜单、不绑键位**；`Ctrl+O` 保持「打开文件夹」（照真源） |
| Q19 | 换项目对话框**照真源三按钮**，"新窗口"点了给"尚未接入：缺窗口级句柄路由" |
| Q20 | 最近项目落**同目录另起一份 json**（`%APPDATA%\Lithe\recent-projects.json`），不污染 `Settings` 扁平模型 |
| Q21 | **项目标签条这轮不动**（换根时替换唯一一项）；多项目标签留到"拆单例"那批 |

## 2. 五批：内容 / 写域 / 验收线 / 不做项

### B1 —— 修"坏的"（先做）

**内容**
1. **菜单下拉两条根因一起修**（`menu_bar.rs`）：
   A. 订阅上游 `DismissEvent`（`popup_menu.rs:1475 → handle_dismiss:1084-1108 → dismiss:1055-1057 → emit:1407`）
      → 落 `close("dismiss")`；渲染链签名从 `&mut App` 改透传 `&mut Context<MenuBar>`
      （照 `project_menu.rs:720-736/946-956/780-792`）。
   B. 摆位改 `Positioner::side(trigger_bounds).placement(Bottom).align(Start).offset(4).margin(8)`
      ＋无内边距包装层 `on_prepaint` 量 bounds（照 `project_menu.rs:645-656, 971-989`）；
      紧凑菜单形态（`menu_bar.rs:913-925`）同病同修。
   C. 遵循 Q7 的 5 条关闭路径，统一走 `close(reason)`（Q17）。
2. **编辑器标签页四修**（`editor_view.rs`）：所有标签悬停显示 `×`（活动标签常显，`:2164-2169`）；
   补 `cx.stop_propagation()` 并订正 `:2216` 的错误注释；实测 `×`（24×24）在 26px 容器里的裁剪；
   File 菜单加齐关闭项（关闭标签页/其他/全部/已保存/左侧/右侧 ＋ 重新打开已关闭标签页，键位见 Q13）。
   `CommandId` 补 CloseTab 系（现在完全没有，`command_palette.rs:187-210`）。
3. **打开文件**（`Ctrl+O` 留给"打开文件夹"，见 Q18）：
   `cx.prompt_for_paths(PathPromptOptions{files:true,directories:false,multiple, prompt})`
   → oneshot → `EditorPane::open`（`editor_view.rs:648`，去重/建状态/诊断全在）。

**写域**：`workbench/src/{menu_bar.rs,workspace.rs,command_palette.rs}`、`editor/src/editor_view.rs`。
**验收线**
- 点空白 / `Esc` / 再点同标题都能收起；悬停另一标题会切换；选中项后收起；
  且 `S1_MENU_CLOSE reason=…` 能区分是哪条路径；**点菜单标题正中必须能命中**（用日志证明回调跑了）。
- 后台标签悬停出现 `×`，点击后标签消失且**不激活它**（`stop_propagation` 生效）；
  脏标签弹出已有确认框（`:1684-1767`）。
- 「打开文件…」选一个文件 → 新标签出现且**切到该文件**。
**不做**：`Ctrl+P` 快速打开（真源另一功能）；项目换根（B4）。

### B2 —— 补 21 条"能力已在、只缺菜单项"的真接线 ＋ 显示键位

**内容**：按 `gpui/research/windows/10-menu-bar.md:114-259` 的"菜单项 → 命令"清单，把**能力已经存在**的
21 条接上（每条都要有真反应，不许出现点了不动的项）；每条菜单项传 `.action(..)`
（`popup_menu.rs:1119-1144,1301` 会自己解析并显示键位）；Q13 列表里的功能同时绑键。
**写域**：`workbench/src/{menu_bar.rs,workspace.rs}`、`app/src/main.rs`（若需注册键位）。
**验收线**：菜单里出现的项**逐条点过**都有真反应；显示的键位与真按下去的效果一致（至少抽 8 条实测）。
**不做**：57 条无能力的项（进 B3）。

### B3 —— 57 条占位 ＋ 状态栏提示 ＋ 单测翻面

**内容**：`MenuEntry` 带 `missing: Option<&'static str>`（或等价的能力键）；按**能力种类**写文案
（≈12 个新键：调试/全局搜索/诊断/数据库/Web 检查器/GitHub/更新/克隆仓库/…），每句说清缺什么；
点击 → 状态栏左侧临时消息 ＋ `S1_MENU notWired id=… missing=…`；**不绑键位**。
单测从"禁死项"翻成"**未接线的项必须有声明**、已接线的项其 action 必须存在"。
**写域**：`workbench/src/{menu_bar.rs,status_bar.rs,workspace.rs}` ＋ i18n 四处。
**验收线**：插一条无声明的项 → 单测红；点任意占位项 → 状态栏出现那句、日志有 `S1_MENU notWired`。
**注**：`克隆仓库…` 进这张表，理由写 **"Core 的 `git.write` 已含 clone，缺的是 URL/凭据/进度 UI"**
（不是"没有能力"，见侦察 §7）。同理调试写"没有进程宿主"、全局搜索写"没有索引器"。

### B4 —— 打开文件夹（项目）＋ 最近项目（单独验收）

**内容**
1. `打开…` / `打开文件夹` → `prompt_for_paths(directories:true)` → 换项目对话框（**幽灵键已在**
   `lithe.zh-CN.yml:3526-3537` 的 `projectOpen.*`，零新增文案）：`取消` / `新窗口`（Q19：给"尚未接入"）/ `此窗口`
   ＋ `不再询问`；照真源先打开成功再写偏好（`project-open-destination.ts:112-131`），
   新键 `askWhereToOpenProjects` / `openFoldersInNewWindow`（默认 true）。
2. **「此窗口」= 重建整个 `ShellWorkspace`**：`window.replace_root(cx, |window,cx| { … Root::new(ws,…) })`
   （窗口根必须是 `Root`，`main.rs:709`）；`compact_menu` 要先从 `main.rs` 局部变量提成 Global。
   ⚠️ 不能逐个 reset：`prepare_java` 幂等早退（`editor_view.rs:436-438`）+ 6 个实体/JDTLS 会话/`GIT_IDENTITY_HOST`
   都在构造期绑根（详见侦察 §3 的 12 处清单）。
3. **最近项目**：`%APPDATA%\Lithe\recent-projects.json`，复用 `persistence.rs` 的原子写；
   字段照抄真源（上限 12、pinned 不占额度、pinned 优先 + `lastOpenedAt` 降序、按 `path` 精确去重）；
   `project_menu.rs:665` 的 `RECENT_PROJECTS=0` 改成真数据源，空态沿用 `noRecentProjects`。
**写域**：`workbench/src/{project_menu.rs,workspace.rs}`、`app/src/main.rs`、`settings/src/{schema.rs,persistence.rs,store.rs}`、i18n 四处。
**验收线**：换根后项目树 / JDTLS 会话 / Maven·Spring 面板 / Git 变更视图**都指向新根**
（用 `S1_*` 日志证明，不只看界面）；最近项目重启后仍在、去重与上限生效；旧实例的 JDTLS/终端**不留残留进程**。
**不做**：多项目标签条（Q21）；多窗口（B5）。

### B5 —— 拆单例 → 真多窗口（最后，独立一批）

**内容**：把 5 个 `thread_local` 单例改成窗口级句柄路由
（`command_palette.rs:115,141`、`menu_bar.rs:766,774`、`project_menu.rs:813`、`settings/src/identity.rs:311-312`），
然后才能把对话框的"新窗口"做成真的（`App::open_window`，`app.rs:1347-1380`）。
**验收线**：两个窗口各自独立（命令面板/菜单/项目下拉/换项目**不串台**），关掉一个不影响另一个。
**不做**：多项目标签条仍归它自己的批。

## 3. 全局硬规则（沿用本仓库既有纪律）

1. **一个 crate 一个写者**；GUI 同一时刻只允许一个代理启动 Lithe；cargo 共享 `gpui/target`、撞锁就等。
2. **占位项不许假装能用**：必须能回答"缺什么"，且由单测强制声明（Q10）。
3. 文案一律 `lithe_gpui_shared::tr`；颜色 `cx.theme()`；布局 rem 档位；诊断 `S1_*` 可 grep。
4. 每批：`cargo build --bin Lithe`（uplift 被运行中 GUI 占用时如实标注）+ 本 crate 单测全绿 + **实机证据**
   （日志原文 + 截图/像素判据），子代理一律不 commit。
5. 收尾：`Get-Process Lithe` 清零；JDTLS 的 `java.exe` **按命令行**杀（含
   `-eclipse.application=org.eclipse.jdt.ls.core.id1`），**绝不**无差别杀 `java`（会误杀 `mvn`）。

## 4. 未确认项（会改变实现方案，动手前需先解决）

1. gpui-kit 0.6.6 有没有"带 checkbox 的二选一对话框"（Q19 那个对话框的形态）——需读 `dialog` 源码。
2. `git.write` 的 `clone` 在"目标目录还不存在"时 `root` 传什么（B3 只写文案，不阻塞；真做时要查）。
3. 旧 `ShellWorkspace` 被 `replace_root` 丢弃时，JDTLS 与终端会话是否自己 `shutdown`
   （只确认方法存在：`java/service.rs:623`、`terminal/session.rs:172`）——B4 换根前必须实测无残留。
