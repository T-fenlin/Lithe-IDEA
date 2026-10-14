# Agent 笔记：gpui 菜单栏 chrome 与快捷键设置的架构决策

状态：已实现

## 先说结论

用户报的「Lithe 菜单是透明的」是两层不同的缺陷叠在一起，不是一件事：

1. 窗口框架色块（chrome 槽，指标题栏、状态栏这类属于窗口框架而不是页面内容的色块）的默认值和页面底色 `bg` 相同，画了等于没画；
2. 主题文件里那一栏 `overlay` 其实是遮罩 scrim（模态浮层压在最下面的那层半透明暗底，两组内置主题都给 `#00000033`），却被当成浮层自己的底色，于是菜单面板、下拉列表、气泡卡片全部半透明。

两层都已修掉。由此定下五条长期规则：chrome 槽必须与页面底色不同值；不新增 `menu_bar` 槽，菜单栏直接用 `title_bar`；快捷键改绑走 gpui 自带能力，不自建按键转发表；快捷键的用户覆盖在设置文件里存成数组而不是对象；子组件不握外壳实体句柄去回戳外壳，改由外壳观察子组件。

**决策定稿不等于代码写完。** 动这块之前先看下面的逐项落地状态：chrome 槽与浮层底色已进代码，菜单项动态置灰（决策十二）已进代码，其余大部分只在设计里。

菜单项动态置灰是 2026-10-14 补上的一块：没有打开任何文件时，菜单栏里依赖活动缓冲的行（Save、Find、行编辑、跳转等）自动变灰且点不动。做法是**宿主每次展开菜单时现算一个标量快照，逐行问「你启用吗」**，库侧一行没改（它本来就有 `MenuItem::disabled()`，只是此前没人传过 `true`）。三个容易做反的点写在决策十二里，其中「跳转历史自己会撒谎」一条已经真实发生过一次。

## 问题

gpui 前端这一侧有三件事挤在一起，且互相牵扯：

- 菜单栏在深色主题下看不出边界，与 IDEA 的观感不一致；
- 设置里的「键盘」分类（Keymap）是一个空壳，快捷键既不能看也不能改，而 macOS 那边早就 ship 了一套成熟的预设与自定义机制；
- 菜单栏的显示模式只有「收进标题栏」和「汉堡紧凑」两种，用户想要 IDEA 那种「抬出一行」和「默认隐藏、鼠标移上去浮出来」。

三者都碰到同一批承重结构：主题调色板的语义槽位、设置持久化文档的合并语义、以及 gpui 的实体借用规则。分开做会互相踩，所以合成一篇决策。

## 决策

### 一、chrome 槽与页面底色必须分开，且默认值取 IDEA 的 header 色

`title_bar` 与 `status_bar` 的默认值从「与 `bg` 同值」改成浅 `0xf2f2f2` / 深 `0x313335`。依据是 `rust/lithe-gpui/themes/jetbrains.json` 里两组内置主题的 `title_bar.background` 与 `status_bar.background` **字面同值**（浅色都是 `#f2f2f2`，深色都是 `#313335`），且都不等于 `background`（浅 `#ffffff` / 深 `#2b2b2b`）。

两个槽必须一起改。只改 `title_bar` 会出现「顶部分离、底部不分离」的自相矛盾：同一个窗口，上方的标题栏有边界、下方的状态栏没有。

`palette.rs` 里那句「`status_bar` 默认取页面底色」的文档注释已同步改写，否则注释会变成假的。

### 二、浮层底色不能用主题文件里的 `overlay` 栏

`palette_for()`（`rust/lithe-gpui/crates/settings/src/theme.rs`）原来把主题文件的 `overlay` 当成浮层底色。实测那栏是 scrim，于是经过 `forms::options::surface()` 影响到库里 19 处共用的浮层表面（菜单面板、下拉列表、气泡卡片、popover、tooltip、命令面板浮层等），另有 27 处直接读 `colors.overlay`。

修法：`overlay` 归 `popover.background`，scrim 归 `backdrop`。再加一条守卫——浅色里 `popover.background` 与 `list.hover.background` 常常同值（两组内置主题都给 `#f2f2f2`），那样菜单里正被指着的那一行会与面板底色融成一片、等于没有高亮；撞色时退到 `surface`，宁可浮层与页面同色靠描边和投影分辨层次，也不丢掉行高亮。

### 三、不新增 `menu_bar` 槽

曾提议给调色板加一个 `menu_bar` 专用槽，已否决，理由见「考虑过的备选方案」。菜单栏画 `title_bar`。

### 四、快捷键走 gpui 原生改绑，不自建转发表

gpui-pre 0.3.8 原生支持运行时改绑：`App::bind_keys` / `App::clear_key_bindings` / `App::key_bindings()` 都是公开 API，而且窗口的分发树与 App 共享同一个 `Rc<RefCell<Keymap>>`（`window.rs` 构造两棵分发树时传的就是 `cx.keymap.clone()`）。也就是说改一次，按键分发和「这一动作当前显示什么快捷键」的查询看到的是同一份数据。

改绑策略是**「快照 → 清 → 回填基线 → 追加用户覆盖」，不是直接清**：`Keymap` 没有删除单条绑定的公开 API，而 `bindings_for_input` 每次击键都全表扫描，只追加会让表无上界地膨胀。

**但 `clear()` 会连库一起清掉**，这一点是本决策最容易被做错的地方：`App::clear_key_bindings()` 清空的是整个 `Keymap`，包括 `lithe-ui` 编辑器那批约 30 条（带 context `ElyEditor`）和其他 crate 登记的那 23 条。而 lithe-ui 的 `bind_keys` 是 `pub(crate)`、只在 `lithe_ui::init` 里调用一次，**宿主编不回去**。所以直接「清 + 重建」会把编辑器的光标移动、删除、补全键位全部抹掉。

正确顺序：`lithe_ui::init(cx)` 与所有 `install_actions` 跑完之后，用 `cx.key_bindings().borrow().bindings().cloned().collect::<Vec<_>>()` 取一次基线快照（`KeyBinding` 是 `Clone`，`Keymap::bindings()` 是公开迭代器）；之后每次重建都 `clear()` → `add_bindings(快照)` → `add_bindings(用户覆盖)`。用户覆盖最后加，`bindings_for_action` 的文档写明「后加者优先」，正好压住基线。

### 五、用户覆盖必须存成数组，不能存成对象

`rust/lithe-gpui/crates/shared/src/document.rs` 的 `preserve_unknown()` 对嵌套对象做深合并、对数组整体替换。存成对象的话，用户删掉一条覆盖永远不生效——旧值会被合并回来。数组则整体替换，删除才真的生效。

### 六、预设三档、用户覆盖按预设分桶

预设取 `lithe` / `idea-classic` / `eclipse` 三档，不做 macOS 档（⌘ 在 Windows 上没有意义）。`KeySource` 从 `{Default, User}` 扩到三档 `{Default, Preset, User}`，**不加 `Extension`**：Windows 前端的类型定义里有 `"user" | "extension" | "default" | "preset"` 四档，但 gpui 这一侧没有扩展键位机制，加进去只会有一档永远没人填。

用户覆盖按预设分桶：`KeyOverride` 带 `preset` 字段，按 `(preset, command)` 排序去重。切换预设保留各桶的覆盖。这与 macOS 已 ship 的行为一致，也是平台能力矩阵里那条验收（分别改两套预设的同一命令、切换与重启后互不覆盖）唯一能过的形态。

有效值合成顺序：① 当前预设下的用户覆盖 → ② 预设表（含显式的空绑定）→ ③ 注册表默认值 → ④ 未绑定。显式空绑定必须能表达「这条命令在此预设下没有键位」，不能被下一层回填（macOS 那条笔记里 Eclipse 的 `Find` 为空、不能回退成 `⌘F` 就是这个规则）。

**已知缺口，登记在此不掩盖**：三端命令 id 命名空间互不相同。gpui 侧 23 个 id（`open-settings`、`save-buffer`、`navigate-to-definition`……）里只有 `toggle-terminal` 和 `toggle-maven` 两个与 macOS 的 39 个 id 同名，Windows 又是第三套（`file.save`、`workbench.commandPalette`、`editor.goToLine`）。所以预设表**无法从任何一端机械移植**，只能人工建显式映射。而且 macOS 那份 `ideaClassicBindings` 是 Ctrl/Option 方言、Eclipse 两份表是 ⌘ 方言，Windows 上都不对。本次只映射两端同名同键的条目，Eclipse 档的真源缺口保留为未解决项。

### 七、写 `cmd-` 没有归一化，归一化要用 `secondary-`

gpui 的 `Keystroke::parse` 把 `cmd` 解析成 `modifiers.platform`，在 Windows 上就是 Win 键——抄 macOS 那套 ⌘ 写法一定会错。

但 gpui **有**自己的平台归一化开关，是 `secondary-`：`keystroke.rs` 里 macOS 展开成 `modifiers.platform`、其他平台展开成 `modifiers.control`。所以预设表和默认键位一律写 `secondary-`，不要写 `cmd-`。现有 `explorer` 那组 `cmd-a` / `ctrl-a` 双写是历史形态，新代码不要照抄。

改绑时还有一条：**context 谓词必须原样带上**。组件库编辑器的键位（`lithe-ui/src/editor/keys.rs`，context `ElyEditor`）如果改成无 context 的全窗绑定，`secondary-z` 会在编辑区之外也触发。目录每行要留 `context: Option<&'static str>`。

### 八、重入纪律：子组件不回戳外壳，改由外壳观察子组件

`MenuBar::notify_shell()` 原来直接 `shell.update(cx, |_, cx| cx.notify())`。问题在于它的调用链是「`ShellWorkspace` 正在被 update → 菜单回调 → `MenuBar::close` → `notify_shell`」，此刻 `ShellWorkspace` 还在借用里，第二次可变借用会触发 `entity_map.rs` 的 panic（`cannot update ShellWorkspace while it is already being updated`）。

中途试过 `cx.defer`，**实测治不了这个 panic** —— `defer` 只是把回调挪到本轮 effect 之后，并不改变「子组件手里握着一个外壳实体句柄」这件事本身。所以定案不是换个时机去戳，而是**把这条边去掉**：

- `MenuBar` 不再持有 `ShellWorkspace` 的实体句柄，需要重画时对**自己** `cx.notify()`；
- 外壳侧 `cx.observe(&menu_bar, |_, _, cx| cx.notify())`，时机由 gpui 的 observe 派发保证，不再依赖我们去猜借用边界。
- 这与 `ShellWorkspace` 里 `project_menu`、`branch_panel` 两个子组件的既有做法一致，不是新发明的模式。

新增代码必须遵守三条：

- 子组件不要自持外壳实体句柄；要重画就 notify 自己，让外壳来 observe；
- 高频事件（hover、鼠标移动）不得触发整个外壳重画，见 [高频可调整 UI 的性能边界](../architecture/2026-09-13-resizable-ui-performance-boundaries.md)；
- 子组件需要外壳**执行动作**（不只是重画）时，走既有的「待处理队列 + 在外壳自己的 `render` 里排空」那条路（`MenuBar::pending` + `take_pending_runs`），而不是手里握着句柄去 `update`。

### 九、AutoHide 的浮出态放进独立实体

菜单栏四态为 `Merged`（收在标题栏行内）、`Above`（抬出独立一行）、`AutoHide`（默认隐藏、hover 浮出）、`Compact`（汉堡）。其中 `AutoHide` 的 `revealed`（当前是否浮出）**刻意不放在 `MenuBar` 本体、也不进外壳**，而是单独一个实体由 `MenuBar` 持有。

原因是 hover 是高频事件：如果它每次都去 notify 外壳，就直接踩上第八条和性能边界那条笔记。放进独立实体后，hover 只更新那个实体，不经过外壳，结构上就不可能触发重入。

热区取**整条顶边**，但用**坐标判断**（读 `on_mouse_move` 的 y 坐标）而不是放一个真实元素，因为真实元素会占掉标题栏行里 `drag_region()` 的横向空间、并且命中高度被元素高度锁死。横向要排除窗口控制区（宽度取 `title_bar.rs` 里的 `WINDOW_CONTROL_WIDTH_SPEC` 常量或 `window_controls` 元素的 bounds，**不要写死像素**），否则鼠标停在关闭按钮上也会浮出。

`on_mouse_move` 可以用，前提是死区过滤：每次回调只做一次比较算出布尔值，只在布尔翻转时才 `cx.notify()`，收起走 150–300ms 代际防抖（照 `persistence.rs` 里 `DebounceState` 的形态）。浮出用 `.absolute()` 覆盖，不推挤布局。另外必须保留键盘唤出作为可达性兜底，触发机制是「key-up 且期间没有其他键按下」，具体键位由交互设计定。

### 十、命令目录与预设表下沉 `lithe-gpui-shared`

依赖方向决定了落点：`shared`（→ `lithe-ui`、`lithe-core`）在栈底，`settings`（→ `lithe-ui`、`shared`、`run`）与 `workbench`（→ `settings`、`shared` 等）都够得到它，而 `settings` 够不到 `workbench`。所以：

- `MenuBarMode` 落在 `settings::schema`（它既要持久化、又要在设置页里选）；
- 命令目录与键位预设表落在 `lithe-gpui-shared`（预设表要对着命令 id 做**编译期校验**：目录在 shared，预设表就能写带类型的键，命令被删或 id 拼错时编译不过，而不是等运行时才发现）。

**目录必须与真实键位对齐。** 实测下来今天对不上，这是下沉之前必须先解决的：

- `MenuAction`（`menu_bar.rs`）51 个，`CommandId`（`command_palette.rs`）23 个，`MenuAction::command_id()` 只把其中 **20 个**映射到 `CommandId`；反过来 `CommandId` 里有 3 个（`OpenSettings`、`SwitchToLightTheme`、`SwitchToDarkTheme`）没有任何 `MenuAction` 指向它们。
真正的约束不在 id 数量，而在**有没有可派发的 action**。`MenuAction`（`menu_bar.rs`）51 个里，`keybind_action()` 只给出 **22 个**真正的 `gpui::Action`；其余 29 个返回 `None`，而派发入口 `dispatch_editor_action`（`workspace.rs`）第一句就是 `let Some(bound) = action.keybind_action() else { return }` —— 也就是说这 29 个今天**派发不出去**。`CommandId` 那 23 个里有 10 个（`OpenFile`、`CloseOtherTabs`、`CloseAllTabs`、`CloseSavedTabs`、`CloseTabsToLeft`、`CloseTabsToRight`、`ToggleMaven`、`ToggleStatusBar`、`OpenAppearanceSettings`、`NewWindow`）正是这种「有 id、有目录行、点了没反应」的死行。

所以下沉时 `CommandId` 扩到覆盖可绑定行，但**每个进目录的 id 必须有真实 action 兜底，没有的宁可不进目录**——否则用户改完键位不生效，页面上有这行、点下去也不报错，比缺行更难发现。差出来的那些是「需要新声明 gpui action」的工作量，不是「少做几个 id」的缩水。

（数字更正：早先记过「gpui 只 `bind_keys` 过 9 条」，是漏 grep 了其余 crate。实际全仓有 6 处共 23 条，另有组件库编辑器一批约 30 条带 context `ElyEditor`。）

id 字符串要一次定死。现在改最便宜：这套 id 还没有任何东西在持久化它。

### 十一、四态本轮全做，且 `MenuBarMode` 下沉 `settings::schema`

四档 `Merged` / `Above` / `AutoHide` / `Compact` **本轮都要做到能选、能存、能渲染、重启后保持**，不留空壳——用户已明确要「抬出一行」和「隐藏 + 悬停唤出」两个都有。

`MenuBarMode` 从 `workbench::menu_bar` 迁到 `settings::schema`，`Pinned` 一并改名 `Merged`（持久化里今天没有这个字段，改名无迁移成本）；`workbench` 侧改成 `use lithe_gpui_settings::schema::MenuBarMode`，**不在 workbench 留第二份**。

本轮不做：面板动画曲线、按窗口宽度自动降级之外的规则、键盘唤出的具体键位（但 `AutoHide` 的唤出入口在架构上不能写死成只有鼠标悬停）。宽度降级就一条：窗口宽度不足时 `Above` 退化成 `Compact`，写死，**且在渲染时算、不进持久化**。

### 正确做法

- 需要一条「不属于页面内容」的框架底色时，用 `title_bar` / `status_bar` / `tab_bar` 这三个 chrome 槽，不要退回 `bg`。
- 新增可持久化的用户数据时，先问它能不能被删除：能删的必须存成数组。
- 快捷键相关的新增行为，先查 gpui 原生 API 能不能做，能就不自建机制。
- 两端都要用到的枚举（模式、命令 id）放 `lithe-gpui-shared` 或 `settings::schema`，按依赖图从栈底往上放，不要放 `workbench`。

### 不要这样做

- 不要给 chrome 槽赋 `bg` 的值。`palette.rs` 的文档注释明确说了 chrome 槽既不能合并成一个、也不能映射回 `surface`/`bg`，并有测试守着。
- 不要让子组件持有外壳实体句柄去 `shell.update`（见决策八：`cx.defer` 也救不了，必须改成 notify 自己 + 外壳 observe）。
- 不要往 `KeySource` 里加 gpui 侧不存在的来源档位。
- 不要把用户可见文案写死成英文。库里 `KeybindingsEditor` 渲染来源徽标处（`keybindings.rs` 的 `match binding.source` 那一支）现在是 `Badge::new("Default")` / `Badge::new("User")` 硬编码，扩档时必须同一次改动搬进 `rust/lithe-ui/locales/ui.yml` 改读 `t!()`——默认显示语言是 zh-CN，不修中文用户会看到英文徽标。

### 十二、菜单项的启用状态由宿主现算，库只负责画

菜单里有些行依赖「当前有没有打开的文件」：没有活动缓冲时，Save、Revert、Find、行编辑、跳转这些行点了也不会有效果。**置灰就是为了让「点了没反应」这件事在按下去之前就可见**，而不是让它在运行时静默失败。

选择让宿主算、库不动，是因为库侧其实早就备齐了：`MenuItem::disabled(bool)`（`menus/model.rs`）存在，绘制层会自动把文字与图标换成 `fg_disabled`、不给 hover 底、不设手型光标，键盘导航（`Menu::step`）会自动跳过禁用行，点击分发也会直接返回。缺的一直只是「谁来说这一行该不该灰」。

形态是 `EditorContext`（`workbench/src/menu_bar.rs`，八个 `bool`/`usize`/`Option<usize>` 的 `Copy` 标量）配 `MenuAction::is_enabled(ctx) -> bool`。外壳在 `ShellWorkspace::render` 里用 `self.editor.read(cx)` 取一次快照，然后沿 `render_bar → menu_bar → menu_row → menu_item → build_menu` **显式传参**下去。刻意不做缓存：菜单面板本来就每次展开现建，而外壳早就 `cx.observe` 了编辑窗格，编辑窗格一 notify 外壳就重画，菜单开着时状态变了会自动跟上。

三个已经踩过、且**不看代码一定会再踩**的点：

1. **跳转历史会撒谎。** `go_back` / `go_forward` 的第一句都是 `current_entry(cx)`，而它第一行是 `self.active?`；`close()` 却不清跳转历史。于是「有跳转历史、一个标签页都没开」是可达状态，此时 `can_go_back()` 返回 `true`，而点了毫无反应。所以 `GoBack` / `GoForward` 必须写成 `has_active_buffer && can_go_back`，只写后者等于亲手造一条死行。
2. **「还原」不看可写性。** `RevertFile` 是从磁盘**读回**，执行侧 `revert_active` 只判 `active` 与 `is_disk()`。有损 UTF-8 转码的缓冲 `writable == false`，可它的正文恰恰是文件内容的近似转写——用户改坏了最需要的就是这一行，带上 `active_writable` 会把唯一的出口堵死。只有 `Save` 才需要 `has_active_buffer && active_writable`。
3. **自画图标不继承禁用色。** 菜单图标走 `icon_element()` 交给宿主自己画，库明确不会替它上色（`model.rs` 里 `icon_element` 的文档注释写了这件事）。宿主必须在建行时按状态自己选 `fg_disabled` / `fg`。

配套两条规矩：

- **不要给 `MenuBar` 挂 `cx.observe(&editor, ...)`。** 需要的是外壳重画，不是 `MenuBar` 自己被 notify——`render_bar` 跑在外壳的 `render` 里，编辑窗格的 notify 早已被外壳那条 `observe` 转发。挂上去会让子组件攥住编辑窗格句柄，正违反决策八。
- **禁用行照常显示快捷键。** 「这个动作绑了什么键」是静态事实，抽掉它等于让用户失去学键位的机会。降档靠库：键帽文字与「+」会跟到 `fg_disabled`。这里有个坑——键帽的颜色是 `typography/inline.rs` 里 `cap()` 自己从主题取的，父级 `text_color` 传不进键帽内部，所以在 `menus/draw.rs` 的容器上改颜色是死样式，只能从 `KbdCombo` 这一层传进去。

本批明确不做，避免范围扩散：快捷键触发路径不跟着禁用（那条路不经过 `build_menu`，要拦得让 `MenuBar` 拿到上下文，会牵动 `keymap_engine`）；命令面板不跟着禁用（它的语义是裁剪行，不是置灰）；dirty、选区、剪贴板、撤销栈都不判（代价高或语义不对）。所以会出现「菜单里灰着、但按 ⌘S 仍然静默 no-op」的行——两者结果一致，用户撞不见矛盾，但实现注释里要点明，否则下一个人会当成 bug。

## 落地状态（逐项）

| 决策 | 落地情况 |
| --- | --- |
| 一、chrome 槽默认值分离 | 已落地（`palette.rs` 浅深两组默认值、文档注释、`theme/tests.rs` 两条断言） |
| 二、浮层底色不再取 scrim | 已落地（`settings/src/theme.rs` 的 `palette_for()`，含撞色退避与回归测试） |
| 三、不新增 `menu_bar` 槽 | 已落地（即「不加」，无需代码） |
| 八之重入修复 | **推倒重做**：`cx.defer` 版曾落地（`workbench/src/menu_bar.rs`），实测仍 panic，已废；改走「子组件 notify 自己 + 外壳 observe」 |
| 库自带 `TitleBar` 统一画 `title_bar` | 已落地（`shell/chrome.rs` 已换 `colors.title_bar`） |
| 四、五、六、七（快捷键页与预设） | 未开工（决策十已定目录与预设表的落点） |
| 九 / 十一（四态菜单栏与 AutoHide） | 未开工，且**本轮四档全做**（见决策十一）；`menus::MenuBar::standalone()` 已就位但无调用点，是为 `Above` 预留 |
| 十二、菜单项动态置灰 | 已落地（`EditorContext` 八字段 + `MenuAction::is_enabled` + `build_menu` 传参 + 穷举守卫测试；库侧仅 `KbdCombo` 加了一个默认关闭的 `disabled` 开关） |

## 考虑过的备选方案

### 新增 `Palette::menu_bar` 专用槽

最吸引人的地方是语义直白：菜单栏有自己的槽，谁改都不会误伤标题栏。**没有采用**，两条理由：`themes/jetbrains.json` 里没有 `menu_bar.background` 这个键，新槽只能落到我们自己编的默认值上，等于凭空造一个 IDEA 没有的概念；而且菜单栏当前在 40px 的标题栏行内，给它一个独立颜色会在一行里切出一条异色带，观感更差。将来若 `Above` 模式把菜单栏抬成真正的独立一行，可以重新评估。

### 自建「命令 → 按键」表 + 窗口按键处理转发

好处是不依赖 gpui 内部 API，看起来稳。**没有采用**：gpui-pre 0.3.8 的 `bind_keys` / `clear_key_bindings` / `key_bindings()` 都是公开的，且分发树与 App 共享同一份 `Keymap`，自建表反而要额外维护一份可能与真实分发不一致的副本。

### 追加式改绑（只 bind 新绑定，靠 `Unbind` 压制旧的）

实现上比「清 + 干净重建」省事。**没有采用**：`Keymap` 没有删除单条的公开 API，而 `bindings_for_input` 每次击键全表扫描，追加会让表无上界膨胀。保留为 A 方案失败时的退路。

### 四态只铺枚举与持久化、交互下轮再补

最吸引人的地方是战线短：先把数据面做对，交互面后面补。**没有采用**：只铺枚举不接线的话，用户打开设置页会看到四个选项、点下去三个没反应，比不做还糟。用户已经明确要「抬出一行」和「隐藏 + 悬停唤出」两个功能，那就得做到选完真的生效。

### 命令目录留在 `workbench`、settings 侧只认字符串 id

好处是改动小，不用动 `command_palette.rs`。**没有采用**：预设表只能拿字符串硬比对，预设里写错一个 id 要等运行时才发现；目录放 shared 之后预设表可以写带类型的键，编译期就能拦住。

### AutoHide 热区只覆盖左侧菜单栏占位区

好处是语义精确、不会与窗口控制区打架。**没有采用**：在标题栏行里放真实元素会占掉 `drag_region()` 的一部分横向空间（标题栏行靠 `flex_1()` 分配拖拽区），而且命中高度被元素高度锁死，做不到「命中面大于视觉面」（性能边界那条笔记的先例是 10pt 命中面）。改为整条顶边 + 坐标判断 + 排除窗口控制区。

### 用户覆盖不分桶（对齐 Windows 前端）

Windows 前端的 `getEffectiveKeybindings` 是「用户绑定优先，其余取基线里没被用户改过的」，不分桶。实现更简单。**没有采用**：macOS 侧已 ship 分桶行为，且平台能力矩阵那条验收要求两套预设的修改互不覆盖，只有分桶能过。Windows 的不分桶是它的历史形态，不跟随。

## 后果

收益：菜单栏和所有浮层终于有可见边界；快捷键有了一条从设置页到实际分发的单一路径；外壳重入崩溃有了结构性的规避方式（状态放对地方，而不是靠每次都记得写 `defer`）。

代价：

- 命令 id 命名空间在三端互不相同，预设表只能人工建显式映射，且 Eclipse 档有真源缺口（macOS 那份是 ⌘ 方言，Windows 上不对）。这个缺口没有在本轮解决，接手的人不要以为预设表可以从别端抄。
- `KeySource` 扩档与 `Palette` 默认值变化都是**破坏性兼容面**：前者是库的公开枚举，后者会改变所有未显式设 chrome 色的宿主的外观。
- 四态菜单栏把「显示模式」从 `bool` 变成枚举，且模式要持久化，会牵动设置 schema 与 CLI 开关的优先级（`compact_menu` 目前是 CLI 层的全局量，不在持久化设置里，所以没有 bool→enum 的迁移，要设计的是 CLI 覆盖设置的优先级）。

需要重新评估的触发条件：菜单栏真的抬成独立一行（`Above`）后，`menu_bar` 槽的否决理由之一即消失；gpui 若引入扩展键位机制，`KeySource` 的 `Extension` 档就有了真实来源。

## 验证

- `./scripts/verify-agent-notes.sh` —— **该脚本 2026-10-14 核实时在仓库里并不存在**，是 `.agents/skills/agent-notes/SKILL.md` 引用但缺失的命令，本条未能执行
- `(cd rust/lithe-ui && cargo test)` —— 覆盖 `theme/tests.rs` 里 chrome 槽与页面底色的分离断言；`shell/tests.rs` 里画了 `KbdCombo`，是键位胶囊默认色的哨兵
- `(cd rust/lithe-gpui && cargo test -p lithe-gpui-workbench --lib menu_bar)` —— 覆盖 `is_enabled` 的判定表与那张穷举守卫（每个 `MenuAction` 变体都必须有明确归属，漏一个就编译不过）
- `(cd rust/lithe-gpui && cargo test -p lithe-gpui-settings)` —— 覆盖 `palette_for()` 的浮层底色与 scrim 回归测试
- `(cd rust/lithe-gpui && cargo test -p lithe-gpui-shared)` —— 覆盖 `document.rs` 的数组/对象合并语义
- `./scripts/verify-platform-feature-matrix.sh` —— 快捷键页真正实现并验证完后，必须给 `keymap-customization.json` 补 gpui 档；现在补会写出假的验证状态

## 适用范围

- `rust/lithe-ui/src/theme/palette.rs`
- `rust/lithe-ui/src/theme/tests.rs`
- `rust/lithe-ui/src/shell/chrome.rs`
- `rust/lithe-ui/src/menus/bar.rs`
- `rust/lithe-ui/src/forms/options.rs`
- `rust/lithe-ui/src/editor/keybindings.rs`
- `rust/lithe-ui/src/menus/draw.rs`
- `rust/lithe-ui/src/typography/inline.rs`
- `rust/lithe-ui/locales/ui.yml`
- `rust/lithe-gpui/crates/settings/src/theme.rs`
- `rust/lithe-gpui/crates/settings/src/persistence.rs`
- `rust/lithe-gpui/crates/settings/src/schema.rs`
- `rust/lithe-gpui/crates/settings/src/dialog.rs`
- `rust/lithe-gpui/crates/workbench/src/menu_bar.rs`
- `rust/lithe-gpui/crates/workbench/src/title_bar.rs`
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-gpui/crates/workbench/src/command_palette.rs`
- `rust/lithe-gpui/crates/shared/src/document.rs`
- `rust/lithe-gpui/crates/shared/src/lib.rs`
- `rust/lithe-gpui/crates/editor/src/editor_view.rs`
- `rust/lithe-gpui/crates/editor/src/navigation.rs`
- `rust/lithe-gpui/themes/jetbrains.json`
- `shared/platform-feature-matrix/features/keymap-customization.json`

口径来源：[macOS 自定义快捷键集中目录与运行时覆盖](2026-08-15-macos-keymap-customization.md)；交互性能约束来源：[高频可调整 UI 的性能边界](../architecture/2026-09-13-resizable-ui-performance-boundaries.md)。
