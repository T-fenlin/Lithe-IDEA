# 编辑器基本功剩余视觉规范（B3 查找替换 / B1 文件增删改名 / 面包屑核对）

- 状态：设计稿（纯设计，**未改任何源码**）
- 作者：颜思齐（UI 设计师）
- 分支：`feat/gpui-editor-basics`
- 上游依据：`reports/editor-basics-remaining-design.md`（架构师闻本立）；其中 **§8 主理人裁决（2026-10-13）已定死，本文不翻结论，只做视觉细化**。
- 强制前置：`.agents/skills/develop-lithe/SKILL.md`（硬编码禁令、共享控件 owner、验证口径）与专家包 `lithe-ui-gallery` Skill。
- 视觉口径：**只用 `rust/lithe-ui/src/theme/` 的真实 token**。本文出现的每个数值都标了出处；标「★核对」的都在下面「读码依据」里给了文件:行号。

## 0. 读码依据（本文所有事实的来源）

| 结论 | 出处 |
| --- | --- |
| `FindWidget` / `FindOptions` / `find_all` 现状实现 | `rust/lithe-ui/src/editor/find.rs`（全文件，1–368） |
| 主题 token（尺寸/圆角/字号/图标/阴影） | `rust/lithe-ui/src/theme/tokens.rs`（`control_height`/`radius`/`text_size`/`icon_size`/`elevation`/`menu_width`/`label_width`） |
| 调色板字段与浅/深取值 | `rust/lithe-ui/src/theme/palette.rs`（`mixable!` 字段表 96–174；`Palette::light` 210–289；`Palette::dark` 291–369） |
| 组件库 i18n 缝与占位符方言 | `rust/lithe-ui/src/i18n/mod.rs`（`text()` 26；`fill()` 71；**`%{name}`**） |
| 组件库文案目录现状 | `rust/lithe-ui/locales/ui.yml`（62 行；根 = `palette`/`dialog`/`state`/`dock`/`keys`，**无 `editor.*` 根**；叶列 7 语言 `en-US/en-GB/de-DE/fr-FR/ja-JP/he-IL/zh-CN`） |
| `IconName`（306 条）真源 | `rust/lithe-ui/src/primitives/icon.rs` + `rust/lithe-ui/assets/icons/*.svg` |
| `IdeaIcon`（186 条）真源 | `rust/lithe-gpui/crates/shared/src/icons/idea.rs`（`ALL` 23–210，逐条路径 212–1330） |
| IDEA 图标实际磁盘清单 | `rust/lithe-gpui/assets/icons/idea/**`（`actions/ expui/ fileTypes/ general/ gutter/ legacy/ maven/ nodes/ terminal/ testState/ toolwindows/ vcs/`） |
| 文件类型图标映射 | `rust/lithe-gpui/assets/icons/idea/extension.json`（1–60+）；消费方 `crates/shared/src/icons/file_icon.rs` |
| explorer 行菜单 | `rust/lithe-ui/src/lists/project/menu.rs`（`ProjectAction` 8–15、`row_menu_entries` 18–36、`PROJECT_ACTIONS` 39–45、`ProjectLabels` 62–70） |
| explorer 行渲染与 token | `rust/lithe-ui/src/lists/project/row.rs`（111–256） |
| `ProjectTree` 挂 `ContextMenu`（manual） | `rust/lithe-ui/src/lists/project/mod.rs`（`row_menu` 538；`Render`/`render` 632–681，`ContextMenu::new(..).manual(..)` 674–675） |
| 宿主 explorer 菜单/端口 | `crates/explorer/src/explorer_view.rs`（`options_menu` 498–527、`row_labels` 648–657、`run_row_action` 373–413） |
| 菜单面板与行 token | `rust/lithe-ui/src/menus/draw.rs`（20–26 行色，33–37/63–67 行度量，176–184 面板）+ `forms/options.rs::surface` 195–206 |
| 菜单行能力（**无 danger 通道**） | `rust/lithe-ui/src/menus/model.rs`（`MenuItem` 21–33；API 35–116：`new/element/check/radio/submenu/icon/icon_element/keys/disabled/on_click`） |
| 按钮能力（有 Danger 档） | `rust/lithe-ui/src/buttons/button.rs`（`ButtonVariant` 73–86；`tone()` 96+；`square` 方形按钮 418–461） |
| `ToggleButton`/`ToggleItem` | `rust/lithe-ui/src/buttons/group.rs`（58–94、171–207） |
| 输入框 frame 与聚焦色 | `rust/lithe-ui/src/forms/input.rs`（101–148：`danger`/`focus`/`border_strong`、`sunken`/`surface`） |
| 行内编辑原语 | `rust/lithe-ui/src/forms/inline.rs`（`Editing` 19–88；`InlineEdit` 91–121） |
| 危险操作对话框（**已有 destructive**） | `rust/lithe-ui/src/overlays/dialogs.rs`（`ConfirmDialog` 116–196，`destructive()` 151 → `ButtonVariant::Danger`） |
| 对话框面板 token | `rust/lithe-ui/src/overlays/dialog.rs`（276–281：`dialog_width`/`Radius::Xl`/`overlay`/`border`/`Modal`；169–178 标题与正文） |
| 面包屑现状（产品侧） | `crates/editor/src/editor_view.rs::render_breadcrumbs`（2425–2472，**纯文本 `/` 分隔，无文件图标**） |
| 面包屑现状（库内） | `rust/lithe-ui/src/navigation/breadcrumb.rs`（`Crumb`/`Breadcrumb` 1–120） |
| 查找页画廊 | `rust/lithe-ui/examples/gallery/pages/editor/search.rs`（`find()` 29–140） |
| 画廊故事清单 | `rust/lithe-ui/examples/gallery/stories.json`（`Editor` 页 `"FindWidget / ReplaceWidget"`） |
| 宿主文案真源与生成器 | `crates/shared/locales/source/locale.ts`（en 与 `zh-CN` 两块）；`crates/shared/locales/lithe.en.yml`、`lithe.zh-CN.yml` **自动生成**；`lithe-gpui/tools/extract-locale.mjs`（`GPUI_ONLY_KEYS` 82+） |
| 宿主 i18n 缝与占位符方言 | `crates/shared/src/i18n.rs`（`tr` / `tr_args`；**`{name}`**；`interpolate` 用法示例）；守卫测试 `every_wired_key_resolves_in_both_locales` |

---

## 1. 结论速览（先看这三条）

1. **图标：B3 零新增，B1 零新增（硬依赖清单见 §8.1）**。查找条要的 8 个角色里，`find-next`/`find-previous`/`close` 两套真源都有对应物；`match-case`/`whole-word`/`regex` **只在库内 `IconName` 有**（`CaseSensitive`/`WholeWord`/`Regex`，均**已存在**），`IdeaIcon` 侧**缺这 3 个**；`replace`/`replace-all` 两套都**没有**，按 IDEA 口径也用**文字按钮**、不需要图标。因为查找条走的是库内 `IconName`，**`IdeaIcon` 的 3 个缺口不下传到实现**。explorer 菜单现状零图标，若决定加，`IconName` 侧 9 个角色**全部已存在**（§3.3）。
2. **宿主 i18n 零新增**：`reports/editor-basics-remaining-design.md` §4.3 说「删除确认正文需新增 `lithe.explorer.deleteConfirm`」——**实测该文案已存在**，且是按文件/文件夹分档的两条：`lithe.fileExplorer.deleteFileMessage` / `lithe.fileExplorer.deleteFolderMessage`（含 `{name}` 占位符，en/zh-CN 齐全）。见 §5.3。
3. **库内新增**：`editor.find.*` 15 条键（7 语言），落在 `rust/lithe-ui/locales/ui.yml`（§8 #3 方案 A）。另有 2 处现状缺陷：替换行左内边距差 4px（§2.4）、正则错误文本无截断（§2.5）。

---

## 2. B3 查找 / 替换条视觉规格

### 2.1 复用组件核对（**先确认存在再引用**）

| 用途 | 真实存在的组件 | 出处 | 备注 |
| --- | --- | --- | --- |
| 查找条外壳 | `FindWidget`（`#[derive(IntoElement)]`） | `editor/find.rs:60` | **已有**，产品侧零接线 |
| 查询/替换输入 | `forms::Input` + `forms::InputState` | `forms/input.rs:15/27`、`forms/text/mod.rs:50` | `Input::new(&Entity<InputState>).size(Size::Small)` |
| 输入框文案 | `InputState::placeholder(text)`、`Input::aria_label(label)` | `forms/text/mod.rs:139`、`forms/input.rs:68` | placeholder 缺省时 `aria_label` 兜底成无障碍名（`input.rs:73-78`） |
| 三个选项开关 | `buttons::ToggleButton` + `buttons::ToggleItem` | `buttons/group.rs:171/58` | `ToggleItem::new(value).icon(IconName).tooltip(text)` |
| 上一处/下一处/关闭 | `buttons::Button`（`ButtonVariant::Ghost`，纯图标 = 方形） | `buttons/button.rs:247/340/308`；方形尺寸 418–461 | 纯图标 + `Size::Small` → 24×24px（`size(control_height(Small))`） |
| 替换 / 全部替换 | `Button::new(..).label(..).variant(Ghost)` | 同上 | 文字按钮，`px(control_padding(Small))` = 8px 左右内边距 |
| 匹配高亮 | `CodeEditor::set_backgrounds(Vec<(Range, Hsla)>, cx)` | `editor/state.rs`（§2.3 已证 gallery 在用） | **§8 #2 裁决：v1 复用 `set_backgrounds`**，不加 `set_find_matches` |

**不存在、不要引用**：`IconButton`（库里没有这个名字，用 `Button` 纯图标形态）；`ReplaceWidget`（gallery 的 `stories.json` 把它写成独立组件，实际是 `FindWidget` + `.replace(&field)` 的展开态）。

**引用路径（已核 `mod.rs` 再导出）**：`lithe_ui::editor::{FindWidget, FindOptions, find_all}`（`editor/mod.rs:46`）；`lithe_ui::forms::{Input, InputState, InputEvent}`（`forms/mod.rs:79/119`）；`lithe_ui::buttons::{Button, ButtonVariant}`（`buttons/mod.rs:19`）、`{ToggleButton, ToggleItem}`（`buttons/mod.rs:23`）；`lithe_ui::menus::{Menu, MenuItem}`（`menus/mod.rs:15`）、`ContextMenu`（`menus/mod.rs:13`）；`lithe_ui::overlays::ConfirmDialog`（`overlays/mod.rs:12`）。

### 2.2 容器（`FindWidget` 外层）

| 属性 | 值 | token 出处 |
| --- | --- | --- |
| 宽度 | 440px = `label_width()`(160px) × 2.75 | `tokens.rs:384` `label_width` |
| 布局 | `flex_col`，行间距 4px | 库内间距档位 `gap_1()` |
| 内边距 | 4px 全向 | `p_1()` |
| 圆角 | 6px | `radius(Radius::Md)`（`tokens.rs:487`） |
| 描边 | 1px `colors.border` | `palette.rs:137` |
| 背景 | `colors.overlay` | `palette.rs:132` |
| 阴影 | `elevation(Elevation::Floating)` | `tokens.rs:498`（浅色 `1/2@4%`+`6/16@8%`；深色 `2/4@30%`+`8/24@45%`，色基 `colors.shadow`） |
| 基础字号 | 11px | `text_size(TextSize::Xs)`（`tokens.rs:472`） |

> **token 卫生（建议、非 v1 阻断）**：`label_width()` 的语义是「描述列表的标签列」，用它乘 2.75 当查找条宽度属**语义借用**。若要收干净，在 `theme/tokens.rs` 加 `pub fn find_bar(&self) -> Rems { px_to_rems(440.0) }` 并改一处调用点；按 §8「v1 最小改动」口径，**可以先不动**，但要在实现 PR 里写明这是借用。

> 容器与 `Input` 的底色关系：`Input` 的底色是 `colors.surface`、描边是 `colors.border_strong`（`forms/input.rs:135-145`）。浅色下 `surface` 与 `overlay` **同为 `#ffffff`**，所以输入框只靠 `border_strong` 描边与容器区分。这与 IDEA「白底灰边查找框」一致，也与 gallery 展示一致 → **保持**，不是缺陷。

### 2.3 第一行（查询 + 三个开关 + 计数 + 步进 + 关闭）

元素顺序与度量（`flex items_center gap_1()`，全部 `Size::Small`）：

| # | 元素 | 视觉 | 条件 |
| --- | --- | --- | --- |
| 1 | 展开/收起替换行 | `Button` 纯图标方形 24×24，`Ghost`；图标 `IconName::ChevronDown`（已展开）/`ChevronRight`（已收起）；tooltip `editor.find.toggleReplace` | 仅当 `on_toggle_replace` 有值 |
| 2 | 查询输入 | `Input`，`flex_1 min_w_0`，高 24px，`aria_label = editor.find.queryField` | 恒有 |
| 3 | 区分大小写 | `ToggleButton` 纯图标，`IconName::CaseSensitive`，tooltip `editor.find.matchCase`，选中态由 `FindOptions.case` 驱动 | 仅当 owner 接管选项 |
| 4 | 全字匹配 | `ToggleButton` + `IconName::WholeWord`，tooltip `editor.find.wholeWord` | 同上 |
| 5 | 正则 | `ToggleButton` + `IconName::Regex`，tooltip `editor.find.regularExpression` | 同上 |
| 6 | 匹配计数 | 宽 80px = `label_width() × 0.5`，`text_center()`，字号 11px；正常色 `colors.fg_muted`，无结果/出错时 `colors.danger` | 恒有 |
| 7 | 上一处 | `Button` 纯图标 `IconName::ArrowUp`，tooltip `editor.find.previousMatch`；`disabled(!found)` | 仅当 `on_step` 有值 |
| 8 | 下一处 | `Button` 纯图标 `IconName::ArrowDown`，tooltip `editor.find.nextMatch`；`disabled(!found)` | 同上 |
| 9 | 关闭 | `Button` 纯图标 `IconName::X`，tooltip `editor.find.close` | 仅当 `on_close` 有值 |

计数文案三态（走 i18n，见 §5.2）：
- 空结果 → `editor.find.noResults`，色 `colors.danger`
- 有当前项 → `editor.find.matchCount`，参数 `%{current}`（1 基）、`%{total}`，色 `colors.fg_muted`
- 正则编译失败 → `editor.find.invalidPattern`（短文案），色 `colors.danger`；**原始 `regex` 报错文本（`find.rs:46` 的 `error.to_string()`，英文、可能多行）改挂 tooltip**，不进计数格。见 §2.5。

> **建议（1 行改动，待对齐）**：库内顺序是「开关组 → 计数 → 步进」。IDEA 把「计数 + ⌃⌄ 步进」当成一组相邻元素（计数紧贴箭头）。建议把计数格移到「开关组之后、上一处之前」**保持不变即可**——现状已经是这个顺序，只是计数被三个开关隔在左边。若要严格贴 IDEA，把计数格从「开关之后」挪到「上一处之前」即可（本来就是相邻的，无需改动）。**结论：现状无需改。**

### 2.4 第二行（替换，展开态）

| # | 元素 | 视觉 |
| --- | --- | --- |
| 1 | 左侧对齐占位 | 现为 `pl(control_height(Size::Small))` = 24px |
| 2 | 替换输入 | `Input`，`flex_1 min_w_0`，高 24px，`aria_label = editor.find.replaceField` |
| 3 | 替换 | `Button` 文字按钮，label `editor.find.replace`，`Ghost`，`disabled(!found)` |
| 4 | 全部替换 | `Button` 文字按钮，label `editor.find.replaceAll`，`Ghost`，`disabled(!found)` |

**★缺陷 1：替换行比查询行少 4px 缩进。**
第一行查询框的左边缘相对 widget 内容盒 = `fold(24px) + gap_1(4px) = 28px`；第二行 `pl = 24px` → **差 4px**。逐行核对：`find.rs:294-296`（`pl(theme.control_height(Size::Small))`）对比 `find.rs:212-231`（`icon_button` 在 `gap_1()` 行内，方形按钮 = `control_height(Small)`）。
修法（token 化、不含魔法数）：把 `.pl(theme.control_height(Size::Small))` 换成 `.child(div().w(theme.control_height(Size::Small)))`，让行自身的 `gap_1()` 补齐那 4px。这样对齐由布局推导，不写死数字。
**影响面**：仅 `FindWidget` 第二行；`on_toggle_replace` 为 `None` 时第二行没有占位（与第一行没有折叠按钮）仍自洽。

### 2.5 匹配高亮与错误态

**高亮（口径 = gallery 已验证的那一套，产品照抄）**：

| 角色 | 颜色 | 出处 |
| --- | --- | --- |
| 当前匹配 | `colors.warning.opacity(0.45)` | gallery `editor/search.rs:53-57`；`warning` = 浅 `#a25f12` / 深 `#e8b45e`（`palette.rs:242/322`） |
| 其余匹配 | `colors.warning.opacity(0.2)` | 同上 |
| 清空 | `set_backgrounds(Vec::new(), cx)` | 查询为空 / 出错时 |
| 定位 | `editor.select([matches[current].clone()])`（自带 `scroll_to_item(Center)`） | §2.3 设计已定 |

可读性（按 sRGB 直接合成估算，`warning@0.45` 叠在编辑器底色上）：浅色主题正文 `#181613` 对叠底后约 `#d3b490` ≈ **9.2:1**；深色主题正文 `#f3f1f0` 对叠底后约 `#72653d` ≈ **5.7:1**。两者都 ≥ 4.5:1，正文不会因高亮变难读。

**★缺陷 2：正则错误的旧文案会撑破计数格。**
`find.rs:173-178` 把 `Err(message)` 直接当计数文案渲染，计数格固定 80px、`text_center()` 但**没有** `truncate()` / `overflow_hidden()` / `whitespace_nowrap()`。`regex::Error::to_string()` 是多行长文本（gallery 用 `message.lines().last()` 兜了一下，但一行的 tail 也可能很长），实测会在固定宽容器里换行、把整个 widget 撑高。
修法：计数格补 `.truncate() .whitespace_nowrap() .overflow_hidden()`；短文案走 `editor.find.invalidPattern`；原始报错文本改挂 `Tooltip`（库内 `primitives::Tooltip` 已有；tooltip 配色 `colors.tooltip_bg`/`colors.tooltip_fg`，`palette.rs:165-166`，对比度见 §6.1）。

### 2.6 展开/收起过渡

**规格：无过渡（瞬时出现/消失）。** 依据：
- IDEA 的替换行是即点即现，无位移/淡入；
- `develop-lithe` 对共享浮层有明确反例约束：共享下拉面板 `animationBehavior = .none`，不许弹跳/缩放动画；查找条的折叠行属同类「共享浮层内部的结构变化」。
- 实现侧约束：查找条是**绝对定位浮层**（§2.2 设计：`.absolute().top_2().right_2()`），任何高度过渡都会让浮层边缘与编辑器正文反复重绘，与高频 UI 性能边界（`.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`）相冲。

若主理人另要过渡：只用「高度/透明度」二选一，时长 ≤ 120ms，且**不得**动外层盒子（`absolute` 定位的 anchor），并且要按上条 Note 的复核流程申请。

### 2.7 产品版 vs gallery 版异同

| 维度 | gallery（`pages/editor/search.rs`） | 产品版（B3） |
| --- | --- | --- |
| 挂载 | `framed(840.0, 320.0).relative()` + `.absolute().top_1().right_4()`（138 行） | 正文容器 `div#lithe-editor-host` 内 `.absolute().top_2().right_2()`（§2.2 设计） |
| 状态主体 | `keep(...)` 的裸 `FindOptions`/`usize`/`bool` + `window.use_keyed_state` 的输入 | 宿主 `EditorPane.find: Option<FindBar>`（`Entity<InputState>` + `matches` + `current` + `error`） |
| 高亮 | `colors.warning.opacity(0.45/0.2)` | **逐字相同**（统一口径） |
| 选项开关 | 常显三开关（`on_options` 已给） | 常显三开关（同） |
| 替换行 | 由 `replacing` 布尔默认展开 | 由 `showing_replace` 控制，默认按 Ctrl+R 决定 |
| 文案 | **硬编码英文**（`find.rs` 内） | 走 `i18n::text` + `editor.find.*`（§5.2） |
| 归属编辑器 | 页面内独立 `CodeEditor` | `EditorPane` 的当前活动缓冲；切标签即收起（`find.editor` 不变量） |
| z 序 | 页面内唯一浮层 | 编辑器 canvas → `completion_popup` → **find 条** → `code_action_menu` |

画廊需要同步什么：**页面内容与截图都不必改**——`editor.find.*` 的 `en-US` 值按 §5.2 要求**与现有硬编码字串逐字相同**，所以 gallery 既有截图/capture 不会因 i18n 化而变。`stories.json` 的 `Editor` 页已有 `{"title": "FindWidget / ReplaceWidget", "slug": "findwidget-replacewidget", "components": ["FindWidget", "ReplaceWidget"]}`，**无需新增**；但 `ReplaceWidget` 这个「组件名」在源码里不存在（它是展开态），建议后续把它从 `components` 里删掉，只留 `FindWidget` —— 属画廊口径修正，本批可不做。

---

## 3. 图标规格

### 3.1 三套图标真源（先分清，别串）

| 真源 | 位置 | 数量 | 谁在用 | 通道 |
| --- | --- | --- | --- | --- |
| **库内 UI 图标 `IconName`** | `rust/lithe-ui/src/primitives/icon.rs` + `rust/lithe-ui/assets/icons/*.svg` | 306 | **所有 lib 组件**：`Button::icon`、`ToggleItem::icon`、`MenuItem::icon`、`FindWidget` | `IconName` |
| **IDEA UI 图标 `IdeaIcon`** | `crates/shared/src/icons/idea.rs` + `rust/lithe-gpui/assets/icons/idea/**` | 186 | 宿主工具栏/标签/产物树等：`crate::icons::idea::X_ICON` | `IconName` 之外，走 `AppIcon`/`ProjectAsset`/`MenuItem::icon_element` |
| **文件类型图标** | `assets/icons/idea/extension.json` + `crates/shared/src/icons/file_icon.rs` | 按扩展名/文件名/folder | explorer 行、编辑器标签（`buffer.icon`） | `FileIcons::lookup` → `ProjectIcon::Image` / `buffer.icon.render` |

### 3.2 查找类图标逐条核对（**硬依赖清单**）

查询口径：本文列的 8 个角色，逐个在**两套真源**里查。

| 角色 | `IconName`（库内，查找条实际使用） | `IdeaIcon`（宿主真源） | 判定 |
| --- | --- | --- | --- |
| 下一处 find-next | ✅ `IconName::ArrowDown`（`icon.rs:64`） | ✅ `ARROW_DOWN_ICON` = `legacy/ui/down.svg`（深浅齐） | 已有 |
| 上一处 find-previous | ✅ `IconName::ArrowUp`（`icon.rs:70`） | ✅ `ARROW_UP_ICON` = `legacy/ui/up.svg`（深浅齐） | 已有 |
| 关闭 close | ✅ `IconName::X`（`icon.rs:342`） | ✅ `X_ICON` = `legacy/ui/close.svg`（深浅齐） | 已有 |
| 大小写 match-case | ✅ `IconName::CaseSensitive`（`icon.rs:90`，`assets/icons/case-sensitive.svg`） | ❌ **缺失**（`assets/icons/idea/**` 无 `matchCase*`；`legacy/ui` 全表已核） | 库内已有；IdeaIcon 缺 |
| 整词 whole-word | ✅ `IconName::WholeWord`（`icon.rs:337`，`whole-word.svg`） | ❌ **缺失**（无 `wholeWord*`） | 库内已有；IdeaIcon 缺 |
| 正则 regex | ✅ `IconName::Regex`（`icon.rs:269`，`regex.svg`） | ❌ **缺失**（无 `regex*`） | 库内已有；IdeaIcon 缺 |
| 替换 replace | ❌ 无 `Replace` 图标 | ❌ **缺失** | **不需要**：IDEA 与库内都用**文字按钮** |
| 全部替换 replace-all | ❌ 无 `ReplaceAll` 图标 | ❌ **缺失** | **不需要**：同上，文字按钮 |
| （附加）搜索徽标 | ✅ `IconName::Search`/`SearchX` | ✅ `MAGNIFYING_GLASS_ICON`(expui/general/search, 深浅) / `SEARCH_ICON`(actions/search, 单图) / `FIND_TOOL_WINDOW_ICON`+`TOOL_WINDOW_FIND_ICON` | 已有 |

**结论（B3）**：查找条 100% 走库内 `IconName`，**8 个角色全部已存在、零新增**。`IdeaIcon` 侧缺 `matchCase`/`wholeWord`/`regex` **不影响本批**——除非将来把查找条搬成宿主自绘（本批不搬）。

**若将来确实要在宿主侧补（仅登记，不在本批新增文件）**：
| 建议常量名 | 建议路径 | 参考来源 |
| --- | --- | --- |
| `MATCH_CASE_ICON` | `icons/idea/actions/matchCase.svg` + `matchCase_dark.svg` | IntelliJ `icons/actions/matchCase*.svg`（Apache-2.0，与本集同源） |
| `WHOLE_WORD_ICON` | `icons/idea/actions/wholeWordsOnly.svg` + `_dark.svg` | 同上 |
| `REGEX_ICON` | `icons/idea/actions/regex.svg` + `_dark.svg` | 同上 |
（新增时必须同步登记进 `idea.rs::ALL`，否则 `lithe-gpui-app` 的 assets 测试不会校验路径——`idea.rs:8` 有明文提醒。）

### 3.3 explorer 右键菜单图标映射建议

**先记事实**：产品侧 explorer 的右键菜单**当前没有任何图标**——`ProjectTree::row_menu`（`lists/project/mod.rs:555`）与宿主 `options_menu`（`explorer_view.rs:518`）都只调 `MenuItem::new(label).on_click(..)`；`MenuItem` 虽有 `.icon(IconName)`（`model.rs:84`）与 `.icon_element(..)`（`model.rs:96`）两条通道，但两处都没用。

**IntelliJ 口径**：Project 视图的右键菜单以**文字 + 快捷键**为主，标准动作**不带头部图标**（少数子菜单如 `New` 的条目才带文件类型图标）。所以：
- **推荐（贴 IDEA）**：v1 保持无图标，`MenuItem::keys("Shift+F6" / "Delete")` 之类补快捷键在右端，视觉上比加图标更"IDEA"。
- **备选（若主理人要求可扫读性）**：按下表给 `IconName`（**不是** `IdeaIcon`，避免跨图标体系混用；`MenuItem::icon` 只收 `IconName`）。

| 菜单动作 | 建议 `IconName` | 现状 | `IdeaIcon` 等价物（若要 icon_element） |
| --- | --- | --- | --- |
| Open | `IconName::ExternalLink` | 已有 | ❌ 无；`COPY_ICON` 语义不符 |
| Rename | `IconName::Pencil` | 已有 | ❌ 无（`EXPUI_EDIT_ICON` 是"编辑"，语义接近但不同图） |
| Delete | `IconName::Trash2` | 已有 | ✅ `TRASH_ICON` = `legacy/ui/delete.svg`（深浅齐） |
| Reveal in File Manager | `IconName::FolderOpen` | 已有 | ✅ `FOLDER_ICON`（`nodes/folder_light.svg`/`folder.svg`） |
| Copy Path | `IconName::Copy` | 已有 | ✅ `COPY_ICON` |
| Copy Relative Path | `IconName::Copy` | 已有（同图，靠文字区分） | ✅ `COPY_ICON` |
| Refresh | `IconName::RefreshCw` | 已有 | ✅ `REFRESH_ICON` = `actions/refresh.svg`（单图） |
| New File | `IconName::FilePlus` | 已有 | ✅ `FILE_PLUS_ICON` = `legacy/ui/addFile.svg`（深浅齐） |
| New Folder | `IconName::FolderPlus` | 已有 | ✅ `FOLDER_PLUS_ICON` = `legacy/ui/newFolder.svg`（深浅齐） |

**结论**：菜单图标**零缺口**，两套真源都有可用物；但点 `IconName` 时要先给 `ProjectAction` 补一个 `icon() -> IconName` 映射（或让 `ProjectLabels` 之外再带一层），否则宿主与库两处菜单会各写一遍，重复且易漂移。**建议**：把图标映射挂在 `ProjectAction` 上（`menu.rs`），两处菜单都读它。

### 3.4 面包屑口径核对

**产品现状（`editor_view.rs:2425-2472`）逐条**：
- 分隔符：**纯文本 `"/"`**，`div().text_color(colors.fg_muted).child("/")`（2456 行附近）→ **不用图标**。
- 层级文字：`colors.fg_muted`；末段（当前文件）`colors.fg` → 亮度高一档。
- 条带：高 24px（`h_6()`）、横向 8px 内边距（`px_2()`）、段间 4px（`gap_1()`）、`overflow_hidden()`、底色 `colors.bg`、下边框 1px `colors.border`。
- **无文件类型小图标**：连末段都没画 `buffer.icon`。（对比：标签栏用 `buffer.icon.render(rems(0.75), cx)`，即 `FileIcon`。）
- 路径源：优先相对 `workspace_root` 的路径，越界（虚拟源码/外部文件）时退全路径。

**库内现成的 `Breadcrumb`（`navigation/breadcrumb.rs`）是另一套视觉**：每段是一个 24px 高、`px_1p5()`、`radius(Radius::Md)`、`text_size(TextSize::Sm)` 的可点 chip，聚焦时描 `colors.focus`，可挂 `Crumb::icon(IconName)` 与「点开同级列表」。**产品现状没有用它**（产品只做展示，段不可点）。

**口径建议（供对齐，不在本批改视觉）**：
1. **分隔符**：保持文本 `/`（贴 IDEA 的 `src ▸ main ▸ App.java` 是 chevron，但产品已选 `/` 且有测试/截图基线；本批不动）。若将来要换图，用 `IconName::ChevronRight`（14px，`icon_size(IconSize::Sm)`），色 `colors.fg_subtle`。
2. **文件类型图标**：若要加，走**与标签栏同一口径**（`buffer.icon` → `extension.json` 的 `fileTypes/*.svg`），放在末段左侧、`icon_size(IconSize::Sm)`=14px，**不要**混用 `IconName`。当前**不加**（IDEA 的面包屑末段确实带文件图标，但加图标会改现有截图基线，需主理人点头）。
3. **可访问性**：`render_breadcrumbs`（`h_stack().id("editor-breadcrumbs")`）**没有 `Role` / `aria_label`**（对比 `explorer_view.rs:584-585` 的 `.role(Role::Tree).aria_label(tr(TREE_ARIA_LABEL_KEY))`）。建议补 `aria_label`（读作"路径"），并把分隔符 `/` 标记为装饰（否则读屏会念出斜杠）。**这是本批唯一建议动的面包屑项**，且是纯属性、零视觉变化。

---

## 4. B1 explorer 右键菜单视觉

### 4.1 菜单面板（与既有 `ContextMenu` 一致）

菜单两处都是库内同一套渲染：`ProjectTree::row_menu` 喂 `ContextMenu::new(id, menu).manual(Some((number, at)))`（`mod.rs:658-679`），面板由 `forms::options.rs::surface` 画（195-206），行由 `menus/draw.rs` 画。

| 属性 | 值 | token 出处 |
| --- | --- | --- |
| 面板圆角 | 8px | `radius(Radius::Lg)`（`surface` 200） |
| 面板底色 | `colors.overlay` | `palette.rs:132`（浅 `#ffffff` / 深 `#201f1d`） |
| 面板描边 | 1px `colors.border` | `palette.rs:137` |
| 面板阴影 | `elevation(Elevation::Floating)` | `tokens.rs:498` |
| 面板基础字号 | 12px | `text_size(TextSize::Sm)`（`surface` 205） |
| 面板最小宽 | 200px | `menu_width()`（`tokens.rs:394`，`draw.rs:184`） |
| 行高 | 28px | `control_height(Size::Medium)`（`draw.rs:63`） |
| 行圆角 | 6px | `radius(Radius::Md)`（`draw.rs:65`） |
| 行悬停底 | `colors.chrome_hover` | `draw.rs:67`（浅 `#e1dfdd` / 深 `#413f3d`） |
| 行文字 | `colors.fg`（禁用时 `colors.fg_disabled`） | `draw.rs:20-26` |
| 行图标 | `colors.fg_muted`（禁用时 `colors.fg_disabled`） | `draw.rs:44`，尺寸 `icon_size(IconSize::Sm)`=14px（`draw.rs:75`） |
| 分隔条 | 1px `colors.border`，上下 4px | `draw.rs:128` |
| 分组标题 | `colors.fg_subtle`，11px | `draw.rs:133-135` |

`ProjectAction` → 菜单结构的建议（§8 #11 已裁决「增」纳入本批）：
- File 行：`Open │ Rename, Delete │ Reveal, CopyPath, CopyRelativePath`
- Directory 行：`Rename, Delete │ Reveal, CopyPath, CopyRelativePath │ Refresh`
- `EmptyFolder` 占位行：空菜单（现状保持）
- 新增 `New File` / `New Folder`（两处菜单都要有；放在 `Rename` 之前的独立分组更贴 IDEA，但组序由架构师定，这里只提视觉上「与 Rename/Delete 同组还是分组」不影响外观）

### 4.2 行内重命名态

口径：`§8 #10` 已裁决**行内编辑（方案 A）**，复用 `forms::Editing` + 键控状态（`lists/tree/mod.rs` 的做法）。

| 属性 | 值 | 出处 / 理由 |
| --- | --- | --- |
| 载体 | `forms::Editing`（`forms/inline.rs:19-88`） | `Editing::begin(state, value, window, cx)` 会**全选内容并聚焦**（inline.rs:34-36）→ 无需另写选中逻辑 |
| 提交 | `InputEvent::Submit` → `Editing::finish(cancel=false)` 触发 `Editing::on_commit` | inline.rs:39-43、73-87 |
| 失败不提交 | `on_commit` 里校验失败就**不落盘、不改树**，保持编辑态；提示见下 | IDEA 口径 |
| 输入框外观 | `Input` 的默认 frame：底 `colors.surface`、描边 `colors.border_strong`，聚焦 `colors.focus`（`forms/input.rs:111-115`/`135-145`） | **与库内其它行内编辑一致**，不要自定义 |
| 行高 | 保持行高 `control_height(Size::Small)`=24px，输入框 `Size::Small` 高 24px → **不跳行** | `row.rs:128`；IDEA 重命名也不改行高 |
| 行的选中底 | 重命名中该行仍保持 `colors.active`（选中态），`row.rs:221` | 让「正在编辑哪一行」同时有底色与输入框两个信号 |
| 名称的扩展名 | `Editing::begin` 传**不含扩展名的主体**（IDEA 只选中主体），避免用户误删扩展名 | IDEA 口径；实现细节归宿主 |
| Esc | `Editing::finish(cancel=true)` 丢弃 | inline.rs:77-79 |
| 变更后的行图标/文字 | 重命名成功后 `name`/`icon`/`language` 跟随新名（`remap_paths` 内），行的文件类型图标由 `extension.json` 重新解析 | `file_icon.rs` |

**校验失败提示（复用宿主已有文案，零新增）**：
| 失败 | 文案键 | en | zh-CN |
| --- | --- | --- | --- |
| 名含分隔符/路径穿越 | `lithe.files.invalidFileNameCharacters` ✅已存在 | `Invalid file name: path traversal and special characters are not allowed` | `文件名无效：不允许路径穿越和特殊字符` |
| 其它非法名 | `lithe.files.invalidFileName` ✅已存在 | `Invalid file name` | `文件名无效` |
| 同目录重名（文件） | `lithe.fileExplorer.fileAlreadyExistsMessage` ✅已存在 | `A file with this name already exists.` | （zh 见 locale.ts 对应条目） |
| 同目录重名（目录） | `lithe.fileExplorer.folderAlreadyExistsMessage` ✅已存在 | `A folder with this name already exists.` | 同上 |
| 重命名的无障碍名 | `lithe.files.renameItem` ✅已存在 | `Rename {name}` | `重命名 {name}` |
| 新建文件/目录的输入提示 | `lithe.files.enterNewFileName` / `enterNewFolderName` / `fileNamePlaceholder` / `folderNamePlaceholder` / `nameNewFile` / `nameNewFolder` ✅全已存在 | — | — |

> **Windows 保留名 / 尾随点空格**：宿主文案里**没有**对应条目（设计 §4.2 会要求单测覆盖）。建议实现时用软失败：沿用 `lithe.files.invalidFileName`（"文件名无效"）作为通用兜底，**不要**为它单独造 key（保留名对绝大多数用户是噪声文案）。

### 4.3 Delete 项的危险色

**★组件缺口：`MenuItem` 没有 danger 通道。** `menus/model.rs:21-33` 的字段里没有颜色/语气，API（35-116）也没有。当前 `draw.rs` 只会取 `colors.fg` / `colors.fg_muted` / `colors.fg_disabled`。

两条路：
- **(a) 零库改动（自绘整行）**：`MenuItem::element(|cx| ...)`（`model.rs:62`）整行自画，文字与图标取 `colors.danger`。代价：行的 hover 底仍由库给（`chrome_hover`），但**禁用态染色要自己判断**，且富行不参与 `SearchableMenu` 的字母过滤（`model.rs:56-58` 有明文）。**不推荐**给一个普通动作项用整行自绘。
- **(b) 建议的库内小增（推荐）**：给 `MenuItem` 加 `pub fn danger(mut self) -> Self` 或 `pub fn tone(mut self, tone: MenuTone) -> Self`，让 `draw.rs` 在 `danger` 时把**文字与图标**染成 `colors.danger`，hover 底**不变**（仍 `colors.chrome_hover`），禁用时仍走 `colors.fg_disabled`（危险 + 禁用 = 不可点的灰，不是红灰）。

**危险色取值（用哪个 token）**：`colors.danger` —— 浅色 `#ba3e38`、深色 `#e5756e`（`palette.rs:243/323`）。

**但先看 IDEA 口径再定**：IntelliJ 的 Project 视图右键菜单里 `Delete…` 用的是**普通前景色**（IDEA 不用红色菜单项；红色破坏性语义是 macOS / VS Code 习惯）。本产品是 IntelliJ 风格，所以：
- **推荐（贴 IDEA）**：菜单 `Delete` 项**不用**危险色，走 `colors.fg`，把「危险」交给**确认对话框**（§4.4 的 `destructive()` 按钮）。
- **备选（若要更醒目）**：按 (b) 给 `Delete` 项上 `colors.danger`，且**只染文字与图标**，不染 hover 底、不染分隔条。两主题对比度都达标（浅 5.48:1 / 深 5.57:1，见 §6.1）。

**这一条需要主理人/界面设计师拍一下**：菜单项是否着色，属产品口径而非纯视觉，我不单方面改结论。

### 4.4 删除确认对话框

**已有组件直接用，不要新画**：`lithe_ui::overlays::ConfirmDialog`（`overlays/dialogs.rs:116-196`）。

| 属性 | 值 | 出处 |
| --- | --- | --- |
| 面板宽 | 440px | `dialog_width()`（`tokens.rs:292`） |
| 面板圆角 | 12px | `radius(Radius::Xl)`（`dialog.rs:277`） |
| 面板底/边/影 | `colors.overlay` / `colors.border` / `elevation(Elevation::Modal)` | `dialog.rs:278-281` |
| 标题 | `text_size(TextSize::Lg)`=16px，`colors.fg` | `dialog.rs:169-171` |
| 正文 | `text_size(TextSize::Sm)`=12px，`colors.fg_muted` | `dialog.rs:177-178` |
| 取消按钮 | `ButtonVariant::Ghost`，文案取库内 `dialog.cancel`（`ui.yml` 已有） | `dialogs.rs:172/178-181` |
| **确认按钮** | **`.destructive()` → `ButtonVariant::Danger`** | `dialogs.rs:151-154`、`165-169`；`tone()` 的 `Danger` 档取 `colors.danger` 系 |

用法（三个文案全部已存在，见 §5.3）：
```
ConfirmDialog::new(id, tr(标题键), tr_args(正文键, &[("name", name)]), on_close)
    .confirm(tr("lithe.ui.delete"))   // "Delete" / "删除"
    .destructive()
    .on_confirm(...)
```
- 文件：标题 `lithe.fileExplorer.deleteFile`，正文 `lithe.fileExplorer.deleteFileMessage`
- 目录：标题 `lithe.fileExplorer.deleteFolder`，正文 `lithe.fileExplorer.deleteFolderMessage`（正文已含"及其所有内容"）

**禁用/危险项处理汇总**：
| 情形 | 处理 |
| --- | --- |
| 菜单 `Delete` 项（若走推荐口径） | 常规 `colors.fg`；**不要禁用**——它是可执行动作（禁用会让"能删但入口灰"的语义矛盾） |
| 危险 + 不可执行（如只读 fs） | `colors.fg_disabled`（`draw.rs:20-26`）；危险色不叠加 |
| 确认对话框的破坏性按钮 | `ButtonVariant::Danger`（唯一用红的地方） |
| 菜单无选中行（`options_menu` 的 `None` 分支，`explorer_view.rs:503-508`） | 5 项全 `MenuItem::disabled(true)`，色 `fg_disabled`（现状保持） |

---

## 5. i18n 文案键与字串

### 5.1 两套目录、两种占位符方言（**先说清，否则实现必错**）

| | 组件库 | 宿主（Lithe 产品） |
| --- | --- | --- |
| 文件 | `rust/lithe-ui/locales/ui.yml`（**手写**） | `crates/shared/locales/lithe.{en,zh-CN}.yml`（**自动生成，勿手改**） |
| 真源 | 就是 `ui.yml` 本身 | `crates/shared/locales/source/locale.ts` + `lithe-gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS` |
| 语言 | **7 个叶列**：`en-US, en-GB, de-DE, fr-FR, ja-JP, he-IL, zh-CN` | **2 个**：`en`、`zh-CN` |
| 读法 | `lithe_ui::i18n::text(cx, key, &[])`（`i18n/mod.rs:26`） | `crates/shared/src/i18n.rs` 的 `tr(key)` / `tr_args(key, &[("name", v)])` |
| **占位符** | **`%{name}`**（`i18n/mod.rs:71-88` 的 `fill`） | **`{name}`**（`i18n.rs::interpolate` 用 `replace("{name}", ..)`） |
| 结构 | `根: 子: 叶: { locale: "文本", ... }`（`_version: 2`） | `根: 子.叶:\n  en: "文本"`（生成物，`_version: 2`） |
| 生成/校验 | `cargo test -p lithe-ui`（库内自测） | `node lithe-gpui/tools/extract-locale.mjs` 重生；守卫测试 `crates/shared/src/i18n.rs::every_wired_key_resolves_in_both_locales` 要求新键进 `WIRED` 清单 |

> 结论：**别把 `%{x}` 写进宿主 yml，也别把 `{x}` 写进 ui.yml。**

### 5.2 `rust/lithe-ui/locales/ui.yml` 需新增的键（`editor.find.*` 全套，15 条）

按 §8 #3（方案 A）。**新增一个 `editor:` 根 + `find:` 子节**（`ui.yml` 现有根只有 `palette`/`dialog`/`state`/`dock`/`keys`，无冲突）。**`en-US` 必须与现有硬编码字串逐字相同**（保证 gallery 截图基线不变）。

```yaml
editor:
  find:
    queryField:         {en-US: "Find", en-GB: "Find", de-DE: "Suchen", fr-FR: "Rechercher", ja-JP: "検索", he-IL: "חיפוש", zh-CN: "查找"}
    replaceField:       {en-US: "Replace with", en-GB: "Replace with", de-DE: "Ersetzen durch", fr-FR: "Remplacer par", ja-JP: "置換後の文字列", he-IL: "החלף ב־", zh-CN: "替换为"}
    toggleReplace:      {en-US: "Toggle replace", en-GB: "Toggle replace", de-DE: "Ersetzen ein-/ausblenden", fr-FR: "Afficher/masquer le remplacement", ja-JP: "置換の表示切り替え", he-IL: "הצג/הסתר החלפה", zh-CN: "切换替换"}
    matchCase:          {en-US: "Match case", en-GB: "Match case", de-DE: "Groß-/Kleinschreibung beachten", fr-FR: "Respecter la casse", ja-JP: "大文字と小文字を区別", he-IL: "התאם רישיות", zh-CN: "区分大小写"}
    wholeWord:          {en-US: "Whole word", en-GB: "Whole word", de-DE: "Nur ganze Wörter", fr-FR: "Mot entier", ja-JP: "単語単位", he-IL: "מילה שלמה", zh-CN: "全字匹配"}
    regularExpression:  {en-US: "Regular expression", en-GB: "Regular expression", de-DE: "Regulärer Ausdruck", fr-FR: "Expression régulière", ja-JP: "正規表現", he-IL: "ביטוי רגולרי", zh-CN: "正则表达式"}
    noResults:          {en-US: "No results", en-GB: "No results", de-DE: "Keine Treffer", fr-FR: "Aucun résultat", ja-JP: "一致なし", he-IL: "אין תוצאות", zh-CN: "无结果"}
    matchCount:         {en-US: "%{current} of %{total}", en-GB: "%{current} of %{total}", de-DE: "%{current} von %{total}", fr-FR: "%{current} sur %{total}", ja-JP: "%{total} 件中 %{current} 件目", he-IL: "%{current} מתוך %{total}", zh-CN: "第 %{current} / 共 %{total}"}
    invalidPattern:     {en-US: "Invalid regex", en-GB: "Invalid regex", de-DE: "Regex falsch", fr-FR: "Regex invalide", ja-JP: "正規表現エラー", he-IL: "ביטוי שגוי", zh-CN: "正则表达式无效"}
    previousMatch:      {en-US: "Previous match", en-GB: "Previous match", de-DE: "Vorheriger Treffer", fr-FR: "Correspondance précédente", ja-JP: "前の一致", he-IL: "התאמה קודמת", zh-CN: "上一个匹配"}
    nextMatch:          {en-US: "Next match", en-GB: "Next match", de-DE: "Nächster Treffer", fr-FR: "Correspondance suivante", ja-JP: "次の一致", he-IL: "התאמה הבאה", zh-CN: "下一个匹配"}
    close:              {en-US: "Close", en-GB: "Close", de-DE: "Schließen", fr-FR: "Fermer", ja-JP: "閉じる", he-IL: "סגור", zh-CN: "关闭"}
    replace:            {en-US: "Replace", en-GB: "Replace", de-DE: "Ersetzen", fr-FR: "Remplacer", ja-JP: "置換", he-IL: "החלף", zh-CN: "替换"}
    replaceAll:         {en-US: "Replace all", en-GB: "Replace all", de-DE: "Alle ersetzen", fr-FR: "Tout remplacer", ja-JP: "すべて置換", he-IL: "החלף הכל", zh-CN: "全部替换"}
```

键的用途对照：
| 键 | 谁用 | 拆掉哪段硬编码 |
| --- | --- | --- |
| `queryField` | `Input::aria_label`（无障碍名） | 新增（现状无） |
| `replaceField` | `Input::aria_label`（无障碍名） | 新增（现状无） |
| `toggleReplace` | 折叠按钮 tooltip | `find.rs:220` `"Toggle replace"` |
| `matchCase` / `wholeWord` / `regularExpression` | 三个开关 tooltip | `find.rs:236/243/250` |
| `noResults` | 计数格（空结果） | `find.rs:174` `"No results"` |
| `matchCount` | 计数格（有当前项） | `find.rs:175` `format!("{} of {total}", current + 1)` |
| `invalidPattern` | 计数格（正则错）+ 原始报错挂 tooltip | `find.rs:177` 直接吐 `regex` 英文报错 |
| `previousMatch` / `nextMatch` | 步进 tooltip | `find.rs:269/277` |
| `close` | 关闭 tooltip | `find.rs:287` |
| `replace` / `replaceAll` | 替换按钮 label | `find.rs:305/315` |

✅ **`invalidPattern` 已定稿（team-lead 拍定，按 §10.8 提案收口）**：计数格固定 80px、字 11px，原长句在 **6 / 7 个语言放不下**（只有 zh-CN 完整），故全部压进预算（en-US/en-GB `Invalid regex`、de-DE `Regex falsch`、fr-FR `Regex invalide`、ja-JP `正規表現エラー`、he-IL `ביטוי שגוי`，zh-CN 保持）。理由与测算见 §10.8；**这是全表唯一不适用「en-US 与旧硬编码串逐字节一致」规则的一条**（错误态旧渲染是动态 `regex` 报错串，本就会变）。

**⚠ 落地时必须同时改两处，少一处测试就红（已核，2026 复核）：**
1. `rust/lithe-ui/locales/ui.yml:74` —— 替换 7 语言取值（复核时**仍是旧长句**）。
2. `rust/lithe-ui/src/editor/find.rs:452-455` —— 测试断言 `assert_eq!(word(cx, "en-US", "editor.find.invalidPattern"), "Invalid regular expression")` **写的是旧值**，必须同步改成 `"Invalid regex"`。这是全库唯一引用该字面量的测试（全仓 grep 仅命中 `ui.yml:74`、`find.rs:453` 两处代码位置）。
3. **`find.rs:96` 的文档注释在修复后失真**：现写「The query failed to compile; **its message shows in place of a count**.」—— 修复后**计数格显示的是短文案，原文只进 tooltip**，注释须一并更正（与 `menu_bar.rs:493-494` 那条失真注释同类）。

**画廊侧影响（我的职责，已核）**：
- **无已提交的页面截图基线** —— `examples/gallery/assets/` 下的 PNG 全是装饰素材（stickers / captcha / lift / slide），不是页面快照；页面证据由 `capture.rs` 按需产出。**故缩值不破坏任何基线图。**
- `examples/gallery/pages/editor/search.rs:67-70` 是 FindWidget 的错误态示范，它用 `.error(message.lines().last().unwrap_or("bad pattern").to_string())` —— 复用了 `find.rs:97-100` 的 `.error()`（该 builder 就是 `self.matches = Err(..)`，**只有一个通道**）。修复后该分支的画廊显示会从「原始 regex 报错行」变为「`Invalid regex`」（**这是本次修复的目的，且与产品一致**）。
- 但**画廊那句 `message.lines().last()` 自己做的截断现在多余了**：tooltip 已经能承载多行原文，画廊这样反而比产品少显示。建议顺手改成交完整原文（一行，可选）。
- 该错误分支**没有 script 驱动**（`pages/editor/search.rs` 无 `script`），只能交互触发 ⇒ 错误态不在任何自动证据里。

**注意（已完成）**：`find.rs:167` 那条调试输出 `log::info!("find: {next:?}")`，实现方已顺手删掉（复核 `find.rs` 已无此行）——它把 `FindOptions` 打到日志里，且不是诊断口径 `S1_*`。

### 5.3 宿主 `lithe.*` 键：**B1 零新增**（含一处对设计稿的更正）

`reports/editor-basics-remaining-design.md` §4.3 写「确认正文需新增 `lithe.explorer.deleteConfirm`」。**实测无需要新增**——宿主目录里已有一整套：

| 用途 | 键 | 状态 | en | zh-CN |
| --- | --- | --- | --- | --- |
| 菜单 Rename | `lithe.files.rename` | ✅已存在 | `Rename` | `重命名` |
| 菜单 Delete | `lithe.files.delete` | ✅已存在 | `Delete` | `删除` |
| 菜单 New File | `lithe.files.newFile` | ✅已存在 | `New File` | `新建文件` |
| 菜单 New Folder | `lithe.files.newFolder` | ✅已存在 | `New Folder` | `新建文件夹` |
| 确认框标题（文件） | `lithe.fileExplorer.deleteFile` | ✅已存在 | `Delete File` | `删除文件` |
| **确认框正文（文件）** | `lithe.fileExplorer.deleteFileMessage` | ✅**已存在** | `Are you sure you want to delete the file "{name}"? This action cannot be undone.` | `确定要删除文件“{name}”吗？此操作无法撤销。` |
| 确认框标题（目录） | `lithe.fileExplorer.deleteFolder` | ✅已存在 | `Delete Folder` | `删除文件夹` |
| **确认框正文（目录）** | `lithe.fileExplorer.deleteFolderMessage` | ✅**已存在** | `Are you sure you want to delete the folder "{name}" and all its contents? This action cannot be undone.` | `确定要删除文件夹“{name}”及其所有内容吗？此操作无法撤销。` |
| 确认框动作 | `lithe.ui.delete` / `lithe.ui.cancel` | ✅已存在 | `Delete` / `Cancel` | `删除` / `取消` |
| 行内重命名无障碍名 | `lithe.files.renameItem` | ✅已存在 | `Rename {name}` | `重命名 {name}` |
| 校验失败 | `lithe.files.invalidFileName` / `invalidFileNameCharacters` / `fileExplorer.fileAlreadyExistsMessage` / `folderAlreadyExistsMessage` | ✅已存在 | 见 §4.2 | 见 §4.2 |
| 未命名缓冲命名 | `lithe.files.untitled` / `lithe.files.untitledNumber` | ✅已存在（**B2 应复用**） | `Untitled` / `Untitled-{number}` | `未命名` / `未命名-{number}` |

**一处建议更正**：设计稿 §4.1 说 `ProjectLabels` 的 `rename`/`delete` 由 `tr("lithe.ui.rename")` / `tr("lithe.ui.delete")` 填。这两个键**确实存在**，但 `row_labels()`（`explorer_view.rs:648-657`）现有六项全部取自 **`lithe.files.*` 组**（`lithe.files.open` / `reveal` / `copyPath` / `copyRelativePath` / `refresh`）。建议改用 **`lithe.files.rename` / `lithe.files.delete`**：同组同源、随文件相关文案一起演进，且 `lithe.ui.*` 是泛用组（`ui.edit`/`ui.remove`/`ui.clear`…），混进来会破坏这一处的一致性。

> 若主理人坚持 `lithe.ui.rename`/`lithe.ui.delete`，视觉上无任何差别（两组 en/zh 文案不同：`files.reveal` 是 `Reveal in File Explorer`、`files.copyPath` 是 `Copy Path`，与 `ui.*` 组没有重叠风险）——**只是分类口径问题，不影响观感**。

### 5.4 多语言叶列如何组织

- **库内（`ui.yml`）**：7 个叶列**必须全给**（缺任一语言，`rust_i18n` 会回显键名，界面直接出 `editor.find.noResults`）。`editor.find` 这 15 条已按 §5.2 给全。
- **宿主（`lithe.*`）**：**只有 en / zh-CN 两个 locale**，新键必须**两边都加**，否则 `every_wired_key_resolves_in_both_locales`（`crates/shared/src/i18n.rs`）会失败（它断言 `tr(key) != key` 且 en≠zh）。本批 B1 **零新增**，所以该守卫不受影响；但若实现期真的加了键（例如保留名提示），要同步进该测试的 `WIRED` 清单。
- **宿主文案改动流程**（别手改生成物）：改 `crates/shared/locales/source/locale.ts`（或解释理由后加进 `tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS`）→ `node lithe-gpui/tools/extract-locale.mjs` → 得到 `lithe.en.yml` / `lithe.zh-CN.yml`。

---

## 6. 可访问性

### 6.1 对比度实测（按 WCAG 2.x 相对亮度公式；色值取自 `palette.rs`）

**查找条（面板底 = `colors.overlay`：浅 `#ffffff` / 深 `#201f1d`）**

| 前景 | 用途 | 浅色 | 深色 | 判定 |
| --- | --- | --- | --- | --- |
| `colors.fg` | 输入文本 | 约 **18.1:1** | 约 **14.6:1** | ✅ AAA |
| `colors.fg_muted` | 匹配计数（正常） | 约 **6.5:1** | 约 **7.4:1** | ✅ AA（≥4.5） |
| `colors.danger` | 计数（无结果/正则错） | **5.48:1** | **5.57:1** | ✅ AA |
| `colors.focus` | 输入框聚焦描边（`input.rs:113`） | 约 **4.9:1** | 约 **6.2:1** | ✅ ≥3:1（1.4.11 非文本） |
| `colors.border_strong` | 输入框**未聚焦**描边（`input.rs:115`） | 约 **1.6:1** | 约 **1.6:1** | ⚠️ <3:1（见下） |
| `colors.border` | 面板描边 | 约 **1.3:1** | 约 **1.5:1** | ⚠️ 装饰（另有 Floating 阴影兜底） |
| `colors.tooltip_fg` on `colors.tooltip_bg` | 三个开关与步进按钮的 tooltip | 约 **17.3:1** | 约 **10.8:1** | ✅ AAA |

**右键菜单（面板底 = `colors.overlay`）**

| 前景 | 用途 | 浅色 | 深色 | 判定 |
| --- | --- | --- | --- | --- |
| `colors.fg` | 菜单项文字 | 约 **18.1:1** | 约 **14.6:1** | ✅ AAA |
| `colors.fg_muted` | 菜单项图标 | 约 **6.5:1** | 约 **7.4:1** | ✅ AA |
| `colors.danger`（若启用危险色） | Delete 项文字/图标 | **5.48:1** | **5.57:1** | ✅ AA |
| `colors.fg_disabled` | 禁用项 | 约 **2.3:1** | 约 **2.4:1** | ⚠️ 低，但 WCAG 1.4.3 明确豁免「非活动组件」——**属既定设计，不算违规** |
| `colors.chrome_hover` vs 面板底 | 行悬停反馈 | 约 **1.33:1** | 约 **1.57:1** | ⚠️ 低。这是全库统一的 hover 洗色档（`draw.rs:67`），IDEA 自己的 `#f2f2f2` hover 在白底上也只有 1.13:1 → **口径一致，维持** |

**匹配高亮叠加**（`warning` @ 0.45 叠在编辑器底色上，sRGB 直接合成估算）
| 主题 | 正文对叠底后 | 判定 |
| --- | --- | --- |
| 浅 | 约 **9.2:1** | ✅ AAA |
| 深 | 约 **5.7:1** | ✅ AA |

### 6.2 结论与依据

1. **文字类全部达标**：查找条与菜单的正文/图标/计数/危险色/tooltip，两主题都 ≥ 4.5:1（多数 ≥ 6.5:1）。
2. **两处刻意低于 3:1，且都不违规**：
   - `fg_disabled`（约 2.3:1）：WCAG 1.4.3 豁免非活动组件；库内禁用态全库一致。
   - `border_strong` 的**未聚焦**输入框描边（约 1.6:1）：1.4.11 要求"识别控件所必需的视觉边界"达 3:1。当前查找框位于一个**有阴影 + 有描边 + 位于编辑器之上**的独立面板内，控件边界不单靠这条描边识别；**聚焦态已切到 `colors.focus`（4.9/6.2:1）**。建议把「聚焦态必须走 `focus` 色」写进 B3 验收项（现状 `Input` 已满足，无需改动）。
3. **不靠颜色单独传达（1.4.1）**：
   - 匹配计数：空结果/出错时不仅变红，**文案本身**从「计数」变成「无结果 / 正则无效」→ 有文字信号。
   - 当前匹配：除了 `warning@0.45` 高亮，**同时被 `select` 选中**（另有 `selection` 底 + 光标）→ 有形状/位置信号。
   - 选项开关：`ToggleButton` 的选中态是**底色 + `on` 语义**（`group.rs:171-207`），不只靠色相。
   - 危险操作：确认对话框用 `destructive()`，按钮文案是「Delete/删除」而取消按钮是「Cancel/取消」→ 不靠红色区分。
4. **键盘与焦点**：§2.5 设计已定 Esc / F3 / Shift+F3 的 key context；`ToggleButton`/`Button`/`Input` 都有 `tab_stop`/`track_focus`（`group.rs`、`button.rs:290`、`input.rs:101`）。**建议补一条验收**：查找条打开后焦点落在查询框，Tab 顺序 = 折叠 → 查询 → 大小写 → 整词 → 正则 → 上一处 → 下一处 → 关闭 →（展开时）替换框 → 替换 → 全部替换。
5. **面包屑**：现状无 `Role`/`aria_label`（对比 explorer 的 `.role(Role::Tree).aria_label(..)`），且段间 `/` 会被读屏念出。建议补 `aria_label`（如 `lithe.files.title` 那种已有键或新增一条）并把分隔符标为装饰。**纯属性改动，零视觉变化。**

---

## 7. 需要 interface-designer（陆井然）对齐的点

| # | 事项 | 我的口径 | 需要你确认 |
| --- | --- | --- | --- |
| A1 | Ctrl+R 与既有键位是否冲突 | §8 已定 `Ctrl+F`/`Ctrl+R`/`F3`/`Shift+F3`；本文只关心"查找条在哪、怎么收起" | 请复核 `Ctrl+R` 未被占；查找条**失焦是否自动收起**（IDEA：不自动收）。我按"不自动收"出规格 |
| A2 | Esc 的归属 | 查找条容器挂 `key_context("EditorFindBar")`，Esc 只关查找条不关标签/面板 | 请确认与菜单栏全局 Esc 拦截器（`menu_bar.rs:1168`）的优先级裁决 |
| A3 | 切标签 / 关标签 / 换缓冲时查找条行为 | 设计：`close_find`（收起，不保留查询） | 请确认是否要"按缓冲各自记住查询"（IDEA 会记住，§8 #4 已定 v1 不持久化 → 我按不记住出规格） |
| A4 | 有选区时 Ctrl+F 预填 | §2.5 设计已定：用选区文本预填 | 请确认"选区过长/含换行"时是否仍预填（我建议：单行选区才预填，多行不预填——需你拍） |
| A5 | 替换行默认展开 | 由 `open_find(with_replace)` 决定（Ctrl+F 收起 / Ctrl+R 展开） | 与你的键位表一致？ |
| A6 | 查找条的定位 | `.top_2().right_2()`（相对正文容器右上角，非指针锚点） | IDEA 是右上角贴边；请确认是否要留出 minimap/滚动条空间 |
| A7 | 面包屑是否加文件类型图标 | 建议**不加**（会改现有截图基线） | 若你要加，我按标签栏同口径（`buffer.icon`，14px）出细化 |
| A8 | 菜单 Delete 是否上危险色 | 推荐不加（IDEA 口径），危险色留给确认框 | 若你要加，我按 §4.3(b) 出库内小改（`MenuItem::danger`） |
| A9 | 删除是否走系统回收站 | §8 #9 已定 v1 直删 + 强确认 | 强确认的文案我给的是"无法撤销"（沿用已有 en/zh 键）；若要加"移到回收站"复选，需新键，请先说 |

---

## 8. 缺口清单（实现期硬依赖）

### 8.1 图标缺口
| 缺口 | 影响 | 是否阻断本批 | 处置 |
| --- | --- | --- | --- |
| `IdeaIcon` 无 `matchCase` | 无 | **不阻断** | 查找条走 `IconName::CaseSensitive`（已存在） |
| `IdeaIcon` 无 `wholeWord` | 无 | **不阻断** | 走 `IconName::WholeWord`（已存在） |
| `IdeaIcon` 无 `regex` | 无 | **不阻断** | 走 `IconName::Regex`（已存在） |
| `IconName` / `IdeaIcon` 均无 `replace` / `replaceAll` 图标 | 无 | **不阻断** | 用文字按钮（IDEA 同口径） |
| explorer 菜单零图标（两处菜单均未用 `.icon()`） | 观感差异 | **不阻断** | 若加图标，`IconName` 侧 9 个角色**全部已存在**（§3.3 表） |

**结论：B3/B1 的实现不需要新增任何 `.svg`。**

### 8.2 组件 / token 缺口
| 缺口 | 影响面 | 处置 |
| --- | --- | --- |
| **`MenuItem` 无 danger 通道** | 菜单危险项无法上色 | 需主理人先定口径（§4.3）：不加色则**无需改动**；要加色则给 `MenuItem` 加 `danger()`/`tone()`（约 10 行 + `draw.rs` 1 处） |
| `FindWidget` 宽用 `label_width()*2.75`（语义借用） | 1 处 | 可选：加 `Theme::find_bar()`；非阻断 |
| `FindWidget` 计数格无截断 | 正则错误态撑破布局 | **建议本批修**（3 行，见 §2.5） |
| 替换行缩进差 4px | 对齐 | **建议本批修**（1 行，见 §2.4） |
| `find.rs:167` 遗留 `log::info!("find: {next:?}")` | 日志噪声 | 建议 i18n 化时顺手删 |
| 面包屑无 `Role`/`aria_label` | 无障碍 | 建议本批补纯属性 |

### 8.3 文案缺口
| 缺口 | 处置 |
| --- | --- |
| 库内 `editor.find.*` 15 条 | **必须新增**（§5.2 已给全 7 语言；`en-US` 逐字沿用现状） |
| 宿主删除确认正文 | **无需新增**（`fileExplorer.deleteFileMessage`/`deleteFolderMessage` 已存在）——更正设计稿 §4.3 |
| 宿主未命名缓冲名 | **无需新增**（`files.untitled` / `files.untitledNumber` 已存在）——B2 应复用 |
| Windows 保留名提示 | **建议不新增**，用 `files.invalidFileName` 兜底 |

---

## 9. 未执行 / 待确认

- **未跑任何构建或截图**（本文是纯设计交付，按要求不改源码；`cargo check` / gallery capture 属 Phase3 实现期的验证项）。
- 对比度数值是按 `Palette::light/dark` 的 **sRGB 十六进制值 + WCAG 相对亮度公式**手算，高亮叠加部分按 **sRGB 直接 alpha 合成**估算。`gpui` 的实际合成空间若为线性空间，数值会有偏移，但各结论的余量（≥4.5:1 的最小余量为 5.5:1）足以覆盖。
- `workspace_root` 与 minimap/滚动条的占位关系（A6）、Esc 优先级（A2）、选区预填的换行边界（A4）需 interface-designer 回复。
- 菜单危险色与面包屑文件图标两项（A7/A8）需主理人拍板；不拍则按本文推荐值（不加）实现。
- 我未核对 IDEA 上游 `ProjectViewPopupMenu` / `findbar` 的原始 SVG 与几何（本仓 `assets/icons/idea/**` 是精选子集，186 条；上游全量不在本仓）。§3.3 的「IDEA 菜单以文字为主」是基于 IDEA 实际形态的产品判断，若主理人要求逐条对齐上游，需要另行取源并登记。

---

## 10. 对交互稿（`reports/editor-basics-interaction-spec.md`）5 问的视觉回复

> 本节是对 interface-designer 提问的答复，**全部结论都重新读了源码/算了对比度**，其中 2 条修正了提问的前提。

### 10.1 未聚焦选区配色（提问 1）——**够用，但单看很弱；不要改库**

**先给源码事实**（这是回答的根据）：

| 事实 | 出处 |
| --- | --- |
| 未聚焦选区**减半透明度** | `editor/line.rs:191-195`：`.bg(if focused { colors.selection } else { colors.selection.opacity(0.5) })`（`focused` 取自 `line.rs:162`） |
| `colors.selection` 本身带 alpha | 浅 `rgba(0x3772bb33)` → **0.20**；深 `rgba(0x69a1e84d)` → **0.302**（`palette.rs:240/320`） |
| ⇒ 有效 alpha | 浅：聚焦 0.20 / **未聚焦 0.10**；深：聚焦 0.302 / **未聚焦 0.151** |
| 编辑器底色 = `colors.surface` | `editor/view.rs:517` → 浅 `#ffffff` / 深 `#1b1917` |
| **未聚焦时根本不画光标** | `line.rs:256`：`.filter(|_| focused && self.caret_on)` |
| 当前行高亮也门控 | `line.rs:324`：`.when(current && focused, ..)` |
| 「其它出现处」高亮也门控 | `editor/view.rs:413`：`Some(needle) if self.focus.is_focused(window) =>` |

**对比度实测（相对编辑器底色 `colors.surface`）**

| 角色 | 浅色 | 深色 | 3:1 门槛 |
| --- | --- | --- | --- |
| 聚焦选区 | 1.30:1 | 1.72:1 | 未达 |
| **未聚焦选区** | **1.14:1** | **1.27:1** | **未达** |
| 当前匹配 `warning@0.45` | 1.90:1 | 2.90:1 | 深基本达 |
| **未聚焦选区 + 当前匹配（两者叠加）** | **2.09:1** | **3.37:1** | 深达 / 浅差 0.91 |
| 聚焦态光标 `colors.focus` 竖线 | 4.90:1 | 6.17:1 | 达 |

叠加说明：选区是行内**绝对定位的底层 div**（`line.rs:185-196`），查找高亮是**文字 run 的背景**（`line.rs:58-71` 的 `marks.backgrounds`），后者画在前者之上，所以「当前匹配」实际是 warning 洗色叠在选区底上，取两者合成值。

**结论与建议**

1. **F3 / Shift+F3 不会出现"以为没跳过去"**，因为一次步进有 **4 个独立信号**：
   - 当前匹配的 `warning@0.45` 洗色（深 2.90:1）；
   - 选区与它**叠加**（深 3.37:1、浅 2.09:1）；
   - 编辑器把匹配**滚到视口中心**（`select` → `scroll_to_item(Center)`，§2.3）；
   - 查找条计数文案从 `"3 of 12"` 变成 `"4 of 12"`，且它就在用户视线停留的查找框旁边。
   另外未聚焦时不画光标、不画当前行底、不画「其它出现处」，是组件**统一**的"编辑器已失活"约定（三处都读同一个 `focused`），不是查找条独有的缺失。
2. **不要为它改库**：半透明减半影响所有非聚焦场景（点侧栏、切终端…），那里半透明正是对的语义。改成不透明会让"编辑器失活"失去视觉区分。
3. **若主理人坚持要更稳**，按代价从低到高：
   - (a) 什么都不做（推荐）：靠上面 4 个信号。
   - (b) 把**当前匹配**的 alpha 从 `0.45` 提到 `0.55`：❌ 我不建议——会与 gallery 已展示的口径（`pages/editor/search.rs:53-57`）不一致，且这点增益不抵一致性损失。
   - (c) 走 §8 #2 的收敛点 `CodeEditor::set_find_matches`，在里面按 `focused` 给"当前匹配"更强的一档。**这是唯一干净的做法**，但属后续单（§8 #2 已定"第二用途出现时"才加）。
4. **一句给测试/验收的话**：浅色主题「未聚焦 + 当前匹配」只有 2.09:1，是全链路最低的一格。若将来有"查找定位可发现性"的验收项，请以「深 3.37:1 / 浅 2.09:1 + 滚动居中」作为基线，而不是要求单靠选区洗色本身达 3:1。

### 10.2 `FindAndReplace` 图标位（提问 2）——**保持空槽是对的，但你的理由要换**

**修正前提**：紧邻的 `Find` **不是空的**——它**有**图标。

| 菜单项 | 图标 | 出处 |
| --- | --- | --- |
| `Find`（Ctrl+F） | `MenuIcon::Idea(&idea::MAGNIFYING_GLASS_ICON)`（expui/general/search，深浅齐） | `menu_bar.rs:473` |
| `FindAndReplace` | `MenuIcon::Empty` | `menu_bar.rs:507`（`Empty` 臂） |

且两者在 Edit 菜单里**直接相邻**：`menu_bar.rs:779-780`（`Find` 然后 `FindAndReplace`，同组无分隔）。所以真实画像是「**一对兄弟动作，一个有放大镜、一个空槽**」，比"一个图标混在一堆空槽里"更显眼；本批 `FindAndReplace` 从 NotWired 变成带 ⌘R 胶囊的真菜单项后会更突出。

**结论：本批保持 `MenuIcon::Empty`，不要给 `FindAndReplace` 加图标。** 依据：
1. 库内规则（`menu_bar.rs:371-375`）：`MenuIcon::Empty` 的文档注释就是「**我们没画过这张图。**」——`IdeaIcon` 里**确实没有** find/replace 类图标（§3.2 已逐条核过 `assets/icons/idea/**`）。
2. `develop-lithe` 的硬规则：未指派图标的动作**保留空槽**（"An unassigned action keeps an empty icon slot"）。
3. **不要**拿 `MAGNIFYING_GLASS_ICON` 顶替：Replace ≠ Search，属伪造图标；`MAGNIFYING_GLASS_PLUS/MINUS` 已被 `ZoomIn`/`ZoomOut` 占用（`menu_bar.rs:483-485`），也不能复用。

**要登记（不是本批做）**：
- 补图：`FIND_AND_REPLACE_ICON` → `icons/idea/actions/findAndReplace.svg` + `findAndReplace_dark.svg`（IntelliJ 有这张图，与本集同源 Apache-2.0），登记进 `idea.rs::ALL`。「Find / FindAndReplace」成对才真正对齐。
- 更该做的是**一条独立的"菜单图标补齐"单**：Edit 菜单里同样的不对称不止这一对——`Cut`/`Copy` 有图（`menu_bar.rs:471-472`）而 `Paste`/`SelectAll` 空（`505-506`）；File 菜单里 `Save` 有软盘（`456`）、`RevertFile` 有 refresh（`738`）而 `SaveAs`/`SaveAll` 空（`721/728`）。这是 `IdeaIcon` 精选子集（186 条）的**系统性**缺口，不是 `FindAndReplace` 一项的毛病。

### 10.3 `NewTab` / `NewFile` 同图（提问 3）——**修正：两者并不相邻，且"移出 New Window"会让问题变严重**

**修正前提**：File 菜单实际顺序是（`menu_bar.rs:706-716`）

```
newTab    → idea::FILE_PLUS_ICON
NewWindow → MenuIcon::Empty        ← 夹在中间
newFile   → idea::FILE_PLUS_ICON
```

两者**中间隔着 `New Window`**，不相邻。所以"紧挨着分不清"比你以为的轻；而**把 `New Window` 移出 New 组会让两者真的相邻、把问题加重**。

**结论**：
1. **不要移动 `New Window`**，保持现状 + 登记。你担心的"动 `MENUS` 项序与顺序敏感守卫断言"，在"不该动"这个结论下就自动规避了。
2. `idea::FILE_PLUS_ICON` = `legacy/ui/addFile.svg`（深浅齐），对 `NewTab`（未命名 scratch）语义上是"新文档"，**可接受**，不需要为它找图。
3. `NewTab`/`NewFile` 都改 `Empty` 属**倒退**：New 组三项目前是 `2 有图 + 1 空`，清空后会与 File 菜单里 `Save`/`RevertFile` 的有图口径更不一致。
4. IDEA 的真答案是 **`New` 子菜单**（Project / File / Directory / Scratch File 各自带图），那是结构性改动（新增 `MenuItem::submenu` 用法 + 项序大改 + 守卫断言）→ **独立单**，本批不做。

### 10.4 `SaveAs` / `SaveAll` 图标位（提问 4）——**同意另开单 + 一条补充约束**

同意不占本批。补充两条写进那一单的验收：
1. **不要**把 `idea::FLOPPY_DISK_ICON` 复用给 `SaveAs`/`SaveAll`。三个不同动作共用一个软盘图标，比空槽**更糟**（用户分不清「保存 / 另存为 / 全部保存」）。要补就补同族带修饰的变体（IDEA 的 `saveAs`/`saveAll` 带角标）。
2. 顺手登记一处**语义借用**：`lithe.menu.revertFile` 用了 `idea::REFRESH_ICON`（`menu_bar.rs:738`），但「刷新」≠「还原文件」。这与 §8.2 的「`find_bar` 宽度借用 `label_width`」同类，属该单的收项。

### 10.5 删除确认按钮的 danger 对比度（提问 5）——**满足可访问性，可以用；但有一处潜在耦合要登记**

`ButtonVariant::Danger` 的取值（`buttons/button.rs:143-149`）：
```rust
ButtonVariant::Danger => Tone {
    bg: colors.danger,
    hover: colors.danger.mix(&colors.fg, 0.14),
    pressed: colors.danger.mix(&colors.fg, 0.24),
    fg: colors.on_accent,   // ← 注意：不是 danger 专用前景
    border: clear,
}
```

**实测（两套主题、四个状态全算）**

| 项 | 浅色 | 深色 | 判定 |
| --- | --- | --- | --- |
| 标签 `on_accent` on `danger` | `#fcfaf7` / `#ba3e38` = **5.26:1** | `#131110` / `#e5756e` = **6.37:1** | ✅ AA |
| hover | 向 `fg` 混合后**变暗** → 更高 | 向 `fg` 混合后**变亮** → **7.29:1** | ✅ |
| pressed | 更高 | 更高 | ✅ |
| 按钮填充 vs 对话框面板 `overlay` | `#ba3e38` / `#ffffff` = **5.48:1** | `#e5756e` / `#201f1d` = **5.57:1** | ✅ ≥3:1（1.4.11） |
| 取消按钮（`Ghost`，`fg` on 面板） | **18.1:1** | **14.6:1** | ✅ AAA |

**结论：`ConfirmDialog::destructive()` 的 danger 前景/背景在深浅两套主题、hover/pressed 全状态都达标，可以直接用。** 你选的落点（复用库内组件、无输入框 ⇒ Enter 不会误触删除）我完全支持。

**要登记的一处潜在耦合**：Danger 的 `fg` 是 `colors.on_accent`，**不是** danger 专用前景。当前两套主题刚好成立（`on_accent` 与 `danger` 的明度差足够），但只要哪个宿主**单独改** `accent`/`on_accent`（例如蓝色 accent + 白色 `on_accent`），Danger 按钮的标签就会掉对比度——那时 `#fcfaf7` on 一个偏亮的 accent 系 `danger`… 具体取决于取值。属**潜在耦合，不是当前 bug**。建议登记为 `tone()` 的待收项：要么加 `on_danger` token，要么让 Danger 的 `fg` 从 `danger` 反向推导。

### 10.6 行内重命名校验失败提示（你新增的交互约束）——视觉口径

你的硬约束（**同一行、不能在下面加第二行**）我接受并确认为视觉规格：`ProjectTree` 用 `uniform_list` 固定行高（`lists/project/mod.rs:651-657`），行高 = `control_height(Size::Small)` = **24px**（`row.rs:128`），多一行就破坏行高一致性。

**布局（全部 token，无魔法数）**

```
row:  flex items_center  h(control_height(Size::Small))          // 24px，恒定
├─ caret slot / icon           （照旧，不受影响）
├─ Input::new(&field).size(Size::Small).invalid(true)            // 24px 高；.invalid(true) → 描边 colors.danger（forms/input.rs:111-112）
└─ 提示: div().flex_1().min_w_0()
         .truncate().whitespace_nowrap().overflow_hidden()        // 绝不换行 ⇒ 不会撑高
         .pl_1p5()                                                // 6px，与输入框留白（breadcrumb.rs 用了同族 px_1p5）
         .text_size(theme.text_size(TextSize::Xs))                // 11px（行内主文字是 14px，row.rs:217）
         .child(tr(键))
```
- 输入框与提示都用 `flex_1 min_w_0` → **对半分配**原来名称标签的那段空间；空间不够时**提示先截断**，输入框不会被挤没。要把完整文案挂 `Tooltip`（库内 `primitives::Tooltip`），截断才不丢信息。

**配色——这里我实测出一个坑，请你二选一**：正在重命名的行是选中/活动行，底色是 `colors.active`（`row.rs:221`）。

| 提示文字色 | 浅（底 `#e9e7e5`） | 深（底 `#2c2b29`） |
| --- | --- | --- |
| `colors.danger` | **4.44:1** ⚠️ 差 0.06 不到 AA 4.5 | 4.78:1 ✅ |
| `colors.fg` | **14.6:1** ✅ | **12.6:1** ✅ |

- **我推荐**：**红框 + `colors.fg` 文字**。即 `Input::invalid(true)` 让描边变 `colors.danger`（`forms/input.rs:111-112`；输入框自身填充仍是 `colors.surface`，`input.rs:137-140`），提示文字用 `colors.fg` at 11px。

  **红框要按两个相邻面分别核，两个都过（非文本门槛是 3:1，不是 4.5:1）**：
  | 相邻面 | 浅色 | 深色 | 门槛 |
  | --- | --- | --- | --- |
  | 框 vs 输入框自身填充 `colors.surface` | `#ba3e38`/`#ffffff` = **5.48:1** | `#e5756e`/`#1b1917` = **5.93:1** | 装饰/内部，高 |
  | **框 vs 所在行底 `colors.active`**（识别控件真正要看的那一面） | `#ba3e38`/`#e9e7e5` = **4.44:1** | `#e5756e`/`#2c2b29` = **4.78:1** | ✅ ≥3:1（1.4.11） |

  **这一格正是本方案成立的原因**：`colors.danger` 对行底是 **4.44:1** —— 同一个数值，压在**文字**通道上差 0.06 不到 AA 的 4.5:1，挂在**非文字**通道上却以 4.44 ≥ 3 稳稳过 1.4.11。把红色从文字搬到描边，等于换了一条门槛更低的通道，而不是「近似达标」。**验收时不要把 4.44:1 判成红框不合格**——它是对文字门槛的读数，不是对非文本门槛的读数。
  这样「哪里错了」由**红框**（非文字信号，4.44:1 ≥ 3:1）+ **文字内容本身**（"文件名无效：…"）双通道传达，满足 1.4.1「不靠颜色单独传达」；同时文字对比度拿满（14.6:1 / 12.6:1），11px 的小字也看得清。
- **备选**：坚持用红字。深色主题没问题（4.78:1），浅色主题 4.44:1 差 0.06——实际观感影响极小，但请如实记一笔（与 §6.1 的 `border_strong` 注记同类：属**已测量的偏差**，不是没测）。
- **不要**为了迁就走第三条路（重命名时把行的 `active` 底换成 `hover`）：那会削弱「我正在编辑哪一行」的信号，得不偿失。

**与 `PromptDialog` 的差异确认**：你说得对，这是唯一必须不同的地方。`PromptDialog::check` 的 `Result<(), SharedString>` 错误画在字段**下方**（`overlays/dialogs.rs:246/251`，对话框里垂直空间充足）；行内重命名**必须横向**。两处不要互相照抄。

#### 10.6.1 落地复核（已核 `lists/project/row.rs` + `forms/inline.rs`）

**✅ 按规格落地**
- 同一行、单行绘制：`row.rs` 的 `body` 在重命名时换成 `div().flex_1().min_w_0().flex().items_center().gap_1()`，**没有第二行** → 满足 `uniform_list` 固定行高约束。
- 红框：`Input::new(&field).size(Size::Small).invalid(refusal.is_some())` ✅（`invalid` 走 `forms/input.rs:111-112`）。
- 提示不换行不撑高：`.truncate().whitespace_nowrap().overflow_hidden()` ✅。
- `Editing::refusal()` 由**行**读出来画（库侧不画提示）✅ 与「库原语保持无表现」一致。
- **加分项（我原稿没写）**：`Editing` 在 `InputEvent::Changed` 时 `refusal.take()` —— 用户一改就清红，不用等下一次提交。对的，采纳。
- `Editing::open(.., select: Option<Range<usize>>, ..)` 只选中主干名（`stem_len`）而非扩展名 ✅ 与「`Foo.java` → 键入 `Bar` 得 `Bar.java`」一致。

**✨ 优于我原稿的一处（以实现为准，我 §10.6 的「对半分配」作废）**
我原写「输入框与提示都用 `flex_1 min_w_0` ⇒ 对半分配」。实现是提示 `flex_none().max_w(relative(0.5))` + 输入框 `flex_1 min_w_0` —— 提示**按内容宽度、上限 50%**，文案短时输入框能多拿宽度。**实现更好，按实现口径。**

**○ 不算偏差的一处**：输入框与提示的分隔，实现用行自身 `gap_1()`(4px)，我原写 `pl_1p5()`(6px)。4px 与行内其它间距同源，**按实现口径**。

**★ 唯一偏差（1 个词的修复）：提示文字用了 `colors.danger`，而我与你已定的是 `colors.fg`。**
实测 `colors.danger` 在三种可能行底上：

| 行底 | 浅色 | 深色 |
| --- | --- | --- |
| `colors.active`（选中/活动行，`#e9e7e5` / `#2c2b29`） | **4.44:1 ✗ 差 0.06 不到 AA 1.4.3 的 4.5:1** | 4.78:1 ✅ |
| `colors.hover`（未选中悬停，`#f2f0ed`） | 4.81:1 ✅ | — |
| 面板底 `colors.surface`（白） | 5.48:1 ✅ | — |

重命名的那一行**通常就是选中行**（先点选中、再重命名是主路径；`begin_rename` 由右键菜单触发、本身不改选区，但用户多半已选中）⇒ **主路径落在 `colors.active` 的 4.44:1，浅色主题不达标**。11px 小字，16px 以下无「大字」豁免（`TextSize::Xs`=11px）。

**修法**：`colors.danger` → `colors.fg`（浅 **14.6:1** / 深 **12.6:1**），**红框保留**作错误通道（框 vs 行底 4.44:1 ≥ 非文本门槛 3:1，过 1.4.11）。这正是我们商定的「把红从文字搬到描边 = 换一条门槛更低的通道」；1.4.1「不靠颜色单独传达」由**红框 + 文案**双通道满足。

---

### 10.7 菜单图标缺口的**精确清单**（复核 `menu_bar.rs` 落地现状）

§10.3 / §10.4 之前只谈了 `FindAndReplace` 与 `SaveAs`/`SaveAll` 三点，**把规模说小了**。复核落地代码后补齐如下（登记项 #6 的票应按本节写）。

**机制**：`MenuIcon` 只有两档——`Idea(&'static IdeaIcon)` 与 `Empty`（`menu_bar.rs:371-375`，`Empty` 的文档注释就是「我们没画过这张图。」）。图标**不在 `MenuItem` 上，而在 `MenuAction` → `MenuIcon` 的映射函数里**。

**现状（已核）**：
- `Self::Find => MenuIcon::Idea(&idea::MAGNIFYING_GLASS_ICON)`（`:473`）——`Find` **有**图标；
- `Self::FindAndReplace` 落在 `Empty` 那条**多行 arm**（`:493-517`）里，与另外 22 个动作共享同一档 —— 即 `FindAndReplace` **不是唯一缺图标的项，它是 23 项之一**（这也正是 §10.3 建议「开系统性票而不是只补 FindAndReplace」的实测依据）。
- 该 arm 前有一行**代码内注释**（`:493-494`）写明原因：「上游那批借来的图标（`CloseAllTabs` 的方框叉、`Paste` 的剪贴板、窗口那三个、断点、自动换行…）我们自己的图标集里没有，**留空**。」

**23 项 `Empty` 清单（按 arm 顺序）**：`CloseAllTabs`、`CloseSavedTabs`、`CloseTabsToLeft`、`CloseTabsToRight`、`ToggleStatusBar`、`ReplaceInFiles`、`NewWindow`、`Minimize`、`Maximize`、`ToggleFullscreen`、`Paste`、`SelectAll`、`FindAndReplace`、`ToggleActivitySidebar`、`ToggleSecondarySidebar`、`ShowDiagnostics`、`ToggleBreakpoint`、`ToggleWordWrap`、`ToggleComment`、`DuplicateLine`、`ToggleMinimap`、`ToggleLineNumbers`、`ToggleRenderWhitespace`。

**另有 `MenuItem::NotWired`（功能未接、不可点）的 `Empty`**：`closeFolder`（`:723`）、`saveAs`（`:726`）、`saveAll`（`:731`）；同组的 `revertFile`（`:736`）反而有 `REFRESH_ICON`、`newFile`（`:714`）有 `FILE_PLUS_ICON` → **未接项之间的图标取舍也不一致**。

**分三类处置**：

**(a) 可立刻补齐——语义精确、图标已存在且含深色变体（2 项）**

| 动作 | 用哪个常量 | 出处 / 资源 |
| --- | --- | --- |
| `Self::ToggleBreakpoint` | `DB_SET_BREAKPOINT_ICON` | `idea.rs:356`；`icons/idea/debugger/db_set_breakpoint.svg` + `_dark` ✅ |
| `Self::ShowDiagnostics` | `TOOL_WINDOW_PROBLEMS_ICON` | `idea.rs:1254`；`icons/idea/toolwindows/toolWindowProblems.svg`；**注意 `has_dark: false`**，深浅同图 |

备选：`VIEW_BREAKPOINTS_ICON`（`idea.rs:1302`，含深色）可替 `DB_SET_BREAKPOINT_ICON`。这两项是**语义精确**、不是近似替换，与我 §10.3 的「不许拿近似图标凑」原则不冲突。

**(b) 确无对应图标、须新画（其余 21 项）**
已核实 `idea.rs` 里**不存在**：剪贴板（`Paste`）、自动换行、注释、缩略图、状态栏、窗口三键（最小化/最大化/全屏）、全选、复制行、行号、空白符渲染、`方框叉` 系列（`CloseAllTabs` 等四个）、诊断/替换于文件（后两者见 (a) 已单独列出）。**不要**用现有图标近似顶替（尤其：不得拿 `MAGNIFYING_GLASS_ICON` 冒充 Replace/`ReplaceInFiles`，不得把 `FLOPPY_DISK_ICON` 复用给 `saveAs`/`saveAll`）。新 SVG 必须**同时**给 `_dark` 变体并登记进 `idea.rs::ALL`（该数组 `:23-210`）。

**(c) 待主理人裁决（语义近似，默认不做）**
`ToggleActivitySidebar` / `ToggleSecondarySidebar` → `COLUMNS_ICON`（`idea.rs:320`，已在用）。属「看着像但语义未必对」，按 §10.3 口径默认留空，除非主理人点头。

**★复核发现（建议写进票里）：`menu_bar.rs:493-494` 的注释与事实不符。** 注释把「断点」归入「我们自己的图标集里没有，留空」，但 `DB_SET_BREAKPOINT_ICON`（`:356`）与 `VIEW_BREAKPOINTS_ICON`（`:1302`）都在 `idea.rs::ALL` 里、都带深色变体。**这 23 项里至少有 2 项是白留的空**，写票时请把注释与清单一起更正，否则后来人会照抄这句错注释继续留空。

**流程建议**：在途的 B4/B5/B6 已经新增了 `DeleteLine`/`MoveLineUp`/`MoveLineDown`/`GoBack`/`GoForward`/`ShowKeymapSettings`/`NextTab`/`PreviousTab` 等动作（都已配到真实图标），但 `ToggleRenderWhitespace`/`ToggleComment`/`DuplicateLine` 又落进了 `Empty` —— **图标债随每个新动作增长**。建议把「新增 `MenuAction` 时要么给图标、要么在 `Empty` arm 里登记并写清原因」做成 checklist 项（实现者其实已经这么做了，只是没收口成规则）。

---

### 10.8 复核发现②：`editor.find.invalidPattern` 的**短文案**在 6 / 7 个语言里放不进计数格 → **已定稿缩值**

**已落地部分（缺陷 2 已闭环）**：计数格 `.truncate().whitespace_nowrap().overflow_hidden()` 已到位（`find.rs:290-296`，并用 `.id((id,"count"))` 让 tooltip 可挂），原始 `regex` 报错改挂 `Tooltip`（`.when_some(refusal, ...)`）。**布局撑高的问题解决**。

**残余**：我 §5 写的「短文案」本身放不进这一格。实测口径：计数格是固定 **80px**（`.w(theme.label_width() * 0.5)`，`label_width()` = 160px，`tokens.rs:384`；`find.rs:291`），格内字 **11px**（`TextSize::Xs`，`tokens.rs:472`；容器 `find.rs:241` 设定）。⇒ 预算约 **13–14 个拉丁字符 / 7 个 CJK 字**。

| 语言 | `invalidPattern` 值 | 估宽 @11px | 放得下？ |
| --- | --- | --- | --- |
| en-US / en-GB | `Invalid regular expression`（26 字符） | ≈143px | ✗ 截断成 `Invalid regul…` |
| de-DE | `Ungültiger regulärer Ausdruck`（28） | ≈154px | ✗ |
| fr-FR | `Expression régulière non valide`（32） | ≈176px | ✗ |
| ja-JP | `正規表現が不正です`（9 CJK） | ≈99px | ✗ |
| he-IL | `ביטוי רגולרי שגוי` | ≈17 字符 | ✗ |
| zh-CN | `正则表达式无效`（7 CJK） | ≈77px | ✅（余量仅 3px） |

即：**只有 zh-CN 能完整显示**。而 tooltip 里挂的是**原始 `regex::Error`**（恒为英文、可能多行），不是本地化短文案 —— 所以 en-US 下用户看到的是「截断的英文短文案 + 英文原始报错」。

**这不违反我在 §5 定的「en-US 与旧硬编码串逐字节一致」规则**：错误态旧渲染用的是**动态** `message.to_string()`（`find.rs:173-178`），从来不是固定字面量 ⇒ `invalidPattern` 从一开始就是新串，错误态截图无论如何都会变，改短它**不额外付基线代价**。

**建议（二选一，非阻塞）**：
**已定稿（team-lead 拍「按上面 (a) 提案定稿」）⇒ §5.2 的表已按此替换、待确认标记已移除。** 下面两案留档，说明当初的取舍：
- **(a) 采纳：把值压进 80px。** 替换 §5 表中 `invalidPattern` 行：en-US/en-GB `Invalid regex`(13)、de-DE `Regex falsch`(12)、fr-FR `Regex invalide`(14)、ja-JP `正規表現エラー`(7)、he-IL `ביטוי שגוי`、zh-CN 保持 `正则表达式无效`。理由：这一格与 `No results` / `5 found` / `2 of 5` 同宽同族，本来就是**状态词**的预算；长句是我 §5 成稿时没核宽度。
- **(b) 未采纳：保持长文案、接受截断**，靠 tooltip 兜底（IDEA 自己在这一格也不显示长句）。

**本条已闭环。**
