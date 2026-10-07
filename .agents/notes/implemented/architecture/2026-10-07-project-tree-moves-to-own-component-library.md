# Agent 笔记：项目树改用组件库自己的 ProjectTree

状态：已实现

## 先说结论

左侧「项目」树的行渲染、展开折叠、多选、右键菜单、行预算与 git 装饰，从宿主
`crates/explorer` 搬进了组件库的 `lithe_gpui::lists::ProjectTree`；宿主只留 Core 读取
（`workspace.snapshot` / `git.status`）、端口（剪贴板、文件管理器）、面板的加载/空/失败态，
以及自己的文案与图标资源。

`crates/explorer` 因此多了一条对 `lithe-gpui` 的依赖，项目树控件本身**落在 px 守卫的
排除范围之外**（守卫按 crate 名排除 vendored 的组件库），这条代价写在「后果」里。

## 问题

1. **控件不是自己的。** 宿主用 gpui-kit 的 `Tree` + `TreeState`，而
   `docs/tasks/idea-project-tree.md` §6 早已把「控件从 gpui-kit 的 `Tree` 换成
   `lithe_gpui::lists::Tree`」登记为下一片；2026-10-07 维护者选定的是「整体组件化」：
   项目树的逻辑一并进组件库，宿主只喂数据与回调。
2. **宿主的六样东西在通用树里没有对等物。** gpui-kit 之上，宿主自己搭了：多选模型
   （`ProjectTreeSelection`，真源 `ProjectTreeSelection.swift` 逐条移植）、右键菜单
   （5 条，条目表是纯数据 + 穷尽 `match`）、活动文档行高亮、IDEA 文件类型图标、
   空目录占位行、行预算（`RENDER_LIMIT`）。组件库的通用 `lists::Tree` 一样都不提供，
   而且它的行把 caret、图标、行高、缩进、选中底色全写死在 `lists/tree/row.rs` 里，
   展开状态还藏在 `window.use_keyed_state` 里，宿主读不到。
3. **git 词汇有两套。** 宿主的 `GitTone` / `GitDecoration` 与库的 `GitStatus` 各一份，
   而 `docs/tasks/idea-project-tree.md` §4 要求「项目树与 `lists::FileTree` 的 git 效果
   必须一致，真相源放组件库」。

## 决策

**项目树成为组件库里的一个有状态组件 `lithe_gpui::lists::ProjectTree`，宿主只喂数据与回调。**

- **落点**：`crates/lithe-gpui/src/lists/project/`（`mod.rs` 组件、`node.rs` 节点与可见行、
  `select.rs` 选择模型、`menu.rs` 行菜单与文案、`row.rs` 行渲染、`tests.rs` 纯单测），
  入口在 `crates/lithe-gpui/src/lists/mod.rs` 导出。**不新建 crate**（理由与代价见「后果」）。
- **有状态 Entity**：展开集合、选择、活动文档行、行预算都在 entity 里（`layout::Dock` 的
  同一形状）。宿主拿 `Entity<ProjectTree>`，读 `selected_paths()` /
  `selected_in_display_order()` / `rows()` / `truncated()` / `status_counts()`，
  调 `set_root` / `set_status` / `reveal` / `follow_active_document` / `set_query` /
  `select_all_in_siblings`。
- **宿主留下**：`workspace.snapshot` 与 `git.status` 的调用与解析（porcelain → `GitStatus`
  属于解释 Core 的响应，留在宿主）、`FileClipboardPort` 与 reveal 端口、行菜单 5 条动作的
  实现（`run_row_action`）、文案（`ProjectLabels`，仍走宿主的 `tr` 目录）、图标
  （`icon_for` 回调按 `theme_icon_path` / `lucide_fallback` 给资源路径）。
- **行渲染与改造前逐条对齐**：行高 24px（`Size::Small`）、缩进 `8 + 14×depth`
  （真源 `ProjectSidebarView.swift:558`）、字号 14（`TextSize::Md`）、行尾 9px 粗体等宽
  git 字母、根行 semibold、圆角 `Radius::Sm`（4）。选中行与活动文档行都用
  `palette.active`，组件库主题桥接里把它从 `host.secondary` 改成 `host.list_active`
  —— 那正是改造前 `explorer_view` 用的底色。
- **⌘A / ⌘C 仍是宿主的 gpui action**（`SelectAllInTree` / `CopySelection`）。gpui 先派发
  keybinding，再走 `on_key_down`（`gpui-pre-0.3.7/src/window.rs:5943-5965`），所以宿主动作
  赢，组件库自带的那套「⌘A 全选可见行」不会截胡；⌘A 的语义仍是真源的「同级兄弟」
  （`selection.selectAll(in:)`）。
- **图标按宿主给的资源路径画**：`ProjectIcon::Glyph` 画 tinted `svg`，`ProjectIcon::Image`
  画 `img`，caret 先看资源在不在（`cx.asset_source().load`）再决定 `img` 还是 tinted `svg`
  —— 与改造前 `shared::icons` 的通路一致。
- **不做的**（宿主没有写命令面，登记在 C34）：拖拽移动、F2 重命名、空格勾选。

### 正确做法

- 需要「一个宿主控件有状态地活着」时，照 `layout::Dock` / `lists::ProjectTree` 写：
  普通 struct + `impl Render` + `impl Focusable`，宿主持 `Entity`，状态用
  `&mut Context<Self>` 的 setter，能改才 `cx.notify()`。
- 宿主回调**不要在借用 entity 的更新里调用**：`click_key` 只返回待执行的动作，
  由行的手柄在 `update` 返回之后再调 `on_action`；键盘路径用 `window.defer`。
- 行要画宿主的图标时，宿主 callback 返回**资源路径**（`ProjectIcon`），库不猜文件类型；
  缺失资源应当画空白而不是 panic（测试环境没有 `AssetSource`）。

### 不要这样做

- 不要为了让项目树复用通用 `lists::Tree` 而给它加一堆只有项目树消费的开关
  （自定义 caret、第二套高亮、行高/缩进、右键菜单、禁用行）——通用树的行是把上游实现
  改成人造的可配置点，收益为零、回归面是整个组件库。
- 不要在组件里调 `Icon::from_path` 画宿主资源：它会为缺失路径 panic，而
  `gpui::TestAppContext` 的 asset source 固定是 `()`，组件自身也就无法在测试里渲染。
- 不要把 git 字母另按 porcelain 的 worktree 字符算：字母与色调必须来自同一个
  `GitStatus`，否则字母与颜色会互相矛盾。

## 考虑过的备选方案

1. **只换控件，行内容仍归宿主**（把通用 `Tree` 加 6 个开关）：不选。行渲染写死在
   `lists/tree/row.rs`，展开状态在 window keyed state 里，宿主读不到就做不了
   「收起时清理选择」和「⌘C 按显示顺序取载荷」；加出来的开关没有第二个消费方。
2. **新建第一方组件 crate 装 `ProjectTree`**：不选。它能满足 px 守卫注释里那句
   「要新写的第一方组件请放独立 crate」，但维护者 2026-10-07 定的是放进组件库
   （`lists::ProjectTree`）；代价作为已知偏差登记在「后果」。
3. **保留两套 git 词汇，只在渲染处对齐颜色**：不选。doc §4 明确要求单一真源，
   而且宿主侧那套的目录汇总逻辑本来就该只有一份。
4. **只搬 git 词汇，树控件继续用 gpui-kit**：不选，与「控件换自己组件」的目标相反。

## 后果

- **收益**：`crates/explorer` 从 982 + 1322 + 325 行降到约 700 行；项目树的节点模型、
  选择语义（含真源的同级 ⌘A、Shift 区间、右键保留选择）、行菜单、行预算、行渲染进了
  组件库，能被别的宿主直接复用；git 字母与颜色和 `lists::FileTree` 同一词汇。
- **代价（架构）**：新增依赖边 `lithe-gpui-explorer → lithe-gpui`（以及既有的
  `workbench → lithe-gpui`）。描述依赖方向的三份文档
  （`rust/lithe-gpui/README.md`、`.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md`、
  `.github/lithe-review/review-prompt.md`）都没有这条边，本片已把三处一并补上
  （组件库不依赖任何宿主 crate，宿主内部方向不变）。
- ⚠️ **px 守卫不再覆盖项目树控件**：`rust/lithe-gpui/tools/check-ui-px.mjs:58-60` 按 crate 名
  排除 vendored 组件库，并明说「要新写的第一方组件请放独立 crate，不要挂进这个被排除的
  目录」。本片按维护者定的落点执行，库内尽量用主题档位表达度量
  （`Size::Small` / `TextSize::Md` / `theme.radius(Radius::Sm)`），只有真源给的
  `8 + 14×depth` 与 9px 字母是常量，并在使用处注明出处。
- **可见变更（有意，逐条）**：
  1. 重命名 / 移动的行：`magenta_light` → `Info`（doc §4 已接受，库的 `Tone` 没有品红档）。
  2. 混合状态（如 `AM`）的字母改用统一词汇：由 worktree 的 `M` 改为 `A`，颜色不变
     （字母与色调现在同源，不再各算一套）。
  3. 展开/折叠箭头改走组件库的行渲染（同一批 expui caret 资源，尺寸 12px）。
  4. 空目录那一行、行 hover 与占位行的文字色改读组件库主题（桥接后与宿主同值）。
- **行预算语义变化**：从「物化多少行（含折叠目录的子项）」变成「**可见**行」——折叠目录
  不再吃预算，深目录再也顶不掉旁边的同级目录；`RENDER_LIMIT`（5000）从宿主常量变成
  `ProjectTree::row_limit`（默认 `lithe_gpui::lists::ROW_LIMIT`）。诊断锚点
  `S1_EXPLORER ... rows= limit= truncated=` 全部保留，只是 `rows` 的口径变成可见行。
- **`lists::GitStatus` 变化**：补 `Copied`，`word()` / `letter()` / `tone()` 转 `pub`，
  新增 `dominant()`；`tone()` 的配色按 doc §4（`Modified → Accent`、`Added | Copied |
  Untracked → Success`、`Deleted → Danger`、`Renamed → Info`、`Conflicted → Warning`），
  组件库自己的 `git::badges` 与画廊 `FileTree` 演示随之一致变化。
- **测试口径**：宿主原来的 44 条 explorer 单测里，与树模型/选择/菜单/预算有关的条目
  随代码搬去了组件库（`lists::project::tests`，21 条纯单测），宿主留 13 条（Core 读取、
  porcelain 分类、路径归一、文案键）；行为级证据仍是 workbench 的
  `tests/explorer_selection.rs` 两条（⌘A 同级选择、⌘C 显示顺序载荷）。
- **需要重新评估的情况**：宿主若改用组件库的通用 `lists::Tree`（而不是项目树组件）；
  或要恢复拖拽/F2/空格勾选（需要 Core 的写命令面，C34）；或 px 守卫改为覆盖
  `crates/lithe-gpui`（那时这批度量常量要按守卫口径改写）。
- ⚠️ **2026-10-07 修订：补回面板标题行（维护者定的「IDEA 外壳版」）**。左栏原来没有标题行
  （Dock 面板是 `DockPanel::plain`，2026-09-27 删过），刷新只能从目录右键菜单进。现在
  `Explorer::render` 在树之上画一行高 39（真源 `ProjectSidebarView.swift:289`）的标题行：
  左边 `Project ⌄` 视图下拉（库里目前只有 Project 一个视图，因此菜单里只有这一项 —— 不画
  假选项，`Packages` / `Project Files` 等整形片落地后再进），右边四个把手：刷新（加载中是
  转圈）、全部折叠、`⋮` 选项菜单、隐藏。
  - 新增库 API：`ProjectTree::collapse_all`（只留根行展开）与 `ProjectTree::focused_row`
    （`⋮` 按焦点行给菜单；没有焦点行时五条动作**置灰**，不隐藏也不假装可用）。
  - 隐藏走新端口 `Explorer::set_hide_host`，外壳把它接成 `left_pane_visible = false`
    （活动栏留着）；刷新与全部折叠各有一条公开方法（`Explorer::refresh` / `collapse_all`），
    标题行的按钮调的就是它们。
  - 文案两条进 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS`（`fileExplorer.collapseAll` /
    `fileExplorer.hidePanel`，真源没有这两条动作），yml 由脚本重生成。
  - 未知项：标题行三个图标按钮的**指针点击**没有自动化测试 —— gpui-kit 的 UI 测试层
    `window.find/click` 在这个控件上拿不到观察路径（`Registered paths: none`），所以行为由
    `tests/explorer_header.rs` 走公开方法验证，按钮→方法的接线只由编译与真机冒烟覆盖。
- **未做**：真机视觉核对（`./rust/target/debug/Lithe.exe <项目根>`）与逐像素对照。

## 验证

本机（Windows）实测，命令 `--manifest-path rust/Cargo.toml`：

- `cargo test -p lithe-gpui --lib`：**677 passed / 4 failed**，失败全在 `forms::avatar`、
  `forms::files`、`settings::importer`，原因是写系统临时目录被拒（`PermissionDenied`），
  与被改的 `lists::*` 无关；`lists::project` 的 21 条全绿。
- `cargo test -p lithe-gpui --features test-support --lib`：**1514 passed / 39 failed**，
  39 条全部是 `media::*` / `files::tests::find` / `forms::*` / `settings::importer`
  的文件读写（`the picture writes: PermissionDenied`），环境所致。
- `cargo test -p lithe-gpui-explorer`：**13 passed**。
- `cargo test -p lithe-gpui-workbench --test explorer_header`：**1 passed**
  （标题行的两条动作：定位展开后 4 行 → 全部折叠 3 行 → 刷新后 4 行）；
  `--test explorer_selection` **2 passed**。
- `node rust/lithe-gpui/tools/extract-locale.mjs --check`：产物与真源一致
  （标题行两条新 key 已进 `GPUI_ONLY_KEYS`）。
- `cargo test -p lithe-gpui-workbench --lib`：**228 passed / 10 failed**（`session` /
  `search_index` / `spring*` 的临时目录写入被拒）。
- `cargo test -p lithe-gpui-workbench --test explorer_selection`：**2 passed**
  （⌘A 同级选择、⌘C 显示顺序载荷）；`--test activity_bar_geometry` 1 passed；
  `--test ui` 4 passed / 2 failed（`TempFile::new` 写系统临时目录被拒）。
- `cargo check -p lithe-gpui-app` 通过；`cargo fmt -- --check` 通过；
  `node rust/lithe-gpui/tools/check-ui-px.mjs` 违规 0；
  `node scripts/verify-agent-notes.mjs` 37 篇通过。
- ⚠️ 未做真机视觉核对（`./rust/target/debug/Lithe.exe <项目根>`），逐像素对照未核；
  本次也没有跑 `cargo test -p lithe-gpui`（不带 `--lib`）：本机上已有一个先于我这次会话
  启动的 `gallery.exe`（PID 4308）与 `Lithe.exe`（PID 25732）占着这两个二进制，示例链接会
  以 `LNK1104` 失败，所以两个库套件都用了 `--lib`。
- 测试稳定性（`write-stable-tests`）：静态门禁
  `.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1` 通过；
  按时长跑了两条受影响的面板——
  `node .agents/skills/write-stable-tests/scripts/run-rust-tests-with-timing.mjs
  --manifest rust/Cargo.toml --package lithe-gpui-explorer --report .artifacts/test-stability/gpui-explorer.json`
  **13 passed**（最慢 60ms），
  `… --package lithe-gpui-workbench --report .artifacts/test-stability/gpui-workbench.json`
  记录到两条**改动过的** UI 测试：`explorer_selection::command_a_without_focus_selects_the_top_level_items`
  **533ms**、`command_c_writes_the_selected_items_to_the_clipboard_port` **659ms**（都通过）；
  该报告整体 exit 1 只因 `ui::editor_body_host_fills_the_pane_below_the_tab_bar` 的环境失败
  （见上）。报告索引：`.artifacts/test-stability/index.html`。

## 适用范围

- `rust/lithe-gpui/crates/lithe-gpui/src/lists/project/**`：`ProjectTree`、`ProjectNode`、
  `ProjectRowKind`、`ProjectAsset`、`ProjectIcon`、`ProjectIconRequest`、`ProjectLabels`、
  `ProjectAction`、`row_menu_entries`、`ROW_LIMIT`
- `rust/lithe-gpui/crates/lithe-gpui/src/lists/files.rs`：`GitStatus` 的 `Copied` /
  `letter()` / `tone()` / `word()` / `dominant()`
- `rust/lithe-gpui/crates/explorer/**`：`Explorer`（含标题行 `render_header`、`HidePanelHost`、
  `refresh` / `collapse_all` / `tree_rows`）、`model.rs` 的 Core 读取与 porcelain 分类、
  `Cargo.toml` 的 `lithe-gpui` 依赖
- `rust/lithe-gpui/crates/explorer/src/explorer_view.rs` 标题行的两条文案 key 由
  `rust/lithe-gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 提供
- `rust/lithe-gpui/crates/workbench/tests/explorer_header.rs`：标题行动作的行为级验证
- `rust/lithe-gpui/crates/workbench/src/component_theme.rs`：`palette.active = host.list_active`
- `rust/lithe-gpui/crates/workbench/tests/explorer_selection.rs`：行为级两条
- 文档：`docs/tasks/idea-project-tree.md` §4/§6、`rust/lithe-gpui/UI-MAP.md` ③、
  `rust/lithe-gpui/crates/lithe-gpui/UPSTREAM.md`
