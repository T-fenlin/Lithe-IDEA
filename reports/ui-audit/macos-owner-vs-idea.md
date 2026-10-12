# 三方取值对照：macOS 共享 owner × IDEA 真源 × lithe-ui token

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。判定目标：macOS 已落地的共享控件约定
与 IDEA Community 视觉真源之间**是否存在结构性冲突**，GPUI 侧能否按 IDEA 真源实现
而不推翻 macOS 已有共享 owner。

## 0. 三个取值来源

| 来源 | 位置 | 性质 |
|---|---|---|
| **macOS owner** | `macos/Sources/Lithe/Theme/LitheTheme.swift`（1263 行）、`LitheIcons.swift`（833 行） | 已落地的共享控件约定，reference product |
| **IDEA 真源** | 仓库内无 IDEA 源码检出；IDEA 侧取值经 macOS 的 note 与矩阵 verification 字段转述 | 规格权威，但**当前只能经二手转述读** |
| **lithe-ui token** | `rust/lithe-ui/src/theme/tokens.rs`（545 行）、`palette.rs`（411 行） | 目标前端的设计系统 |

**重要限制**：仓库内 `Get-ChildItem -Recurse -Filter 'jetbrains.json'` 只找到
**1 个**文件——`rust/lithe-gpui/themes/jetbrains.json`（259 行）。这是**主题配色**文件，
不是 IDEA Community 的控件几何源。IDEA Community 源码**未在仓库内检出**
（检索 `idea/community`、`idea-community` 只命中 note 里的文字引用）。
所以本文件的「IDEA 真源」一列，**只能引用仓库 note 与矩阵 JSON 里已转述的取值**，
不是直接读 IDEA 源码所得。note 里出现的 IDEA 版本号有三个：
`fb72b4df43ab`（`.agents/notes/implemented/feature/2026-09-24-windows-git-log-multi-repository-groups.md:140`）、
`c7f91397daa3a961b4e78bc634fe467a0a7d9ade`（`2026-10-02-macos-shared-scrollbar.md:19`）、
`fb72b4df43aba102479eb0502d20b03586b9c5b8`（`2026-10-04-windows-commit-file-diff-preview.md:206`）。
**这三个不是同一个 revision**，本身就是一处需要先统一的真源口径。

## 1. 圆角（Radius）：lithe-ui 把 macOS 的「按角色分级」压平成一档

macOS owner 是**按控件角色分档**的（`LitheTheme.swift:718-740`，注释明说
「统一的尺寸与间距刻度，避免各视图各写一套魔法数字」）：

| 角色 | macOS 取值 | 行号 |
|---|---|---|
| 项目树选中弧 | **4** | `:729` `projectTreeSelectionCornerRadius` |
| 悬浮提示 | **4** | `:518` `cornerRadius`（HoverTooltip） |
| Commit 控件 | **4** | `:751` `Commit.controlCornerRadius` |
| 通用 `Metrics.cornerRadius` | **5** | `:736` |
| 控件 | **6** | `:739` `controlCornerRadius` |
| 工具栏 | **6** | `:462` `toolbarRadius` |
| 右键菜单 | **8** | `:738` `contextMenuCornerRadius` |
| 弹窗 | **10** | `:737` `popupCornerRadius` |

lithe-ui 只有 **4 档**（`tokens.rs:484-492`）：`Sm`=4、`Md`=6、`Lg`=8、`Xl`=12，
且**控件几乎一律取 `Md`（6）**：

| 控件 | lithe-ui | 位置 |
|---|---|---|
| 悬浮提示 | **6**（`Md`） | `primitives/tooltip.rs:27` |
| 菜单面板 | **6**（`Md`） | `menus/draw.rs:35`、`:65` |
| 按钮 | **6**（`Md`） | `buttons/button.rs:424` |
| 输入框 | **6**（`Md`） | `forms/input.rs:133` |
| 项目树行 | **4**（`Sm`） | `lists/project/row.rs:216` |

**三方对照结论**：

| 角色 | macOS owner | lithe-ui | 判定 |
|---|---|---|---|
| 项目树选中 | 4 | 4（`Sm`） | **一致** |
| 控件 / 按钮 / 输入框 | 6 | 6（`Md`） | **一致** |
| 工具栏 | 6 | 6（`Md`，复用） | **一致** |
| 悬浮提示 | **4** | **6** | **不一致** |
| 右键菜单 | **8** | **6** | **不一致** |
| 弹窗 / 浮层 | **10** | **6** | **不一致** |
| 通用 `Metrics.cornerRadius` | **5** | 无对应档 | **缺一档** |

**结构性判定：不冲突，但 lithe-ui 缺档**。macOS 的 4/5/6/8/10 五个值不是互相矛盾的
两套体系，而是**同一体系按角色分级**；lithe-ui 的 `Radius` 只有 4 档且把
菜单/弹窗/提示全压到 `Md`=6，是**分辨率不足**，不是方向冲突。
`Radius::Xl`=12 在 macOS owner 里**没有对应角色**（最大是弹窗 10）。
因此**可以用平台档位调和**：把浮层类角色补成独立档（菜单 8、弹窗 10、提示 4），
不推翻任何 macOS owner。按登记时的推翻条件（「发现同一控件同一属性上直接矛盾，
且无法用平台档位调和」），**「视觉真源双轨可对齐」在这一项上未被推翻**。

## 2. 树行与缩进：三个值并存，且 lithe-ui 自己就有两套

| 量 | macOS owner | lithe-ui 库自身 | lithe-ui 项目树专用 |
|---|---|---|---|
| 行高 | `rowHeight` **24**（`:720`）／项目树 `treeRowHeight` **27**（`:723`） | `control_height(Size::Small)` = **24**（`tokens.rs:98`） | 同左（`lists/project/row.rs:128`） |
| 缩进 | `indent` **19**（`:403`，Tree 枚举） | `tree_indent()` = **16**（`tokens.rs:341`） | `INDENT_BASE=8` + `INDENT_STEP=14`（`row.rs:29-30`） |
| 图标 | `treeIconSize` **16**（`:730`） | `icon_size(IconSize::Md)` = **16**（`tokens.rs:141`） | 同左（`row.rs:129`） |
| 字号 | `treeFontSize` **13**（`:731`） | `text_size(TextSize::Base)` = **13**（`tokens.rs:474`） | — |
| 行内边距 | 4 纵 / 12 横（`:727-728`） | — | — |

**判定**：
- 行高 24 与图标 16 与字号 13 **三方一致**（lithe-ui `Size::Small`=24 是 base，
  `Density::Standard` delta=0，`tokens.rs:102-106`；`Compact` 会变 20、`Comfortable` 变 28）。
- **缩进是三套不同值**：macOS Tree 19、lithe-ui 库自身 16、lithe-ui 项目树 8+14。
  注意 `lists/project/row.rs:27-30` 的注释**自己承认了这一点**：
  > 「The row indentation the IDE's project tree draws with: 8 plus 14 a level
  > (`ProjectSidebarView.swift:558`), which the library's own tree indent does not share.」

  即 lithe-ui **有意**为项目树绕开库自身的 `tree_indent()`。这不是冲突，
  但意味着**「库的树缩进」与「IDE 项目树缩进」是两件事**，规格文档必须分开写。
- **缺 `projectTreeContentVerticalInset`(4) / `HorizontalInset`(12) / `RowSpacing`(0)**：
  lithe-ui 无对应 token（检索 `projectTreeContent*` 命中 0）。
  capability 要求的「24pt **连续**行距」里的「连续」（spacing=0）在 lithe-ui 侧
  无显式 token，靠行高本身承担。

## 3. 滚动条：四处取值不一致

| 量 | macOS owner（经 note 转述 IDEA `c7f91397`） | lithe-ui | 判定 |
|---|---|---|---|
| rail 宽 | **14pt** + 2pt gap（note `:28`） | `thickness * 2.0` = **12px**（`scroll.rs:210/216`） | 不一致 |
| thumb 厚 | **7→10pt**（浅）/ **9→12pt**（深）（note `:32`） | `thickness` = **6px**（`tokens.rs:167`；`scroll.rs:172-173`） | 不一致 |
| 内边框 | **1pt**（note `:22`） | **无**（`scroll.rs:157-162` 只有 `bg` + `hover`） | 不一致 |
| 取色 | `#FFFFFF26` / `#FFFFFF4D`（深，note `:33`，`IslandSchemeDark.xml` 覆盖） | `fg.opacity(0.22)` / `0.42`（`scroll.rs:156`） | 不一致 |
| 最小 thumb | — | `scrollbar_min_thumb()` = **24px**（`tokens.rs:170`） | 新增，macOS 侧无对应 |
| Diff 标记 | **14pt** 宽，右 x=4 / 左 x=0，最少 2pt（note `:29-31`） | **未找到实现入口**（`crates/git` 下无自绘滚动条） | 缺 |

**判定**：**取值不一致，但方向不冲突**——两边都是「细胶囊 + 透明轨道」的同一路数，
只是数值与取色来源不同。note `:33` 的警告（「不能套用普通滚动区的灰色」）是关键：
lithe-ui 用 `fg` 的透明度派生，这在**深色 Island 主题下会得到与 IDEA 不同的观感**。
可调和：给滚动条补专用 token（thumb 色、rail 宽、边框）而非从 `fg` 派生。

## 4. 悬浮提示：四项全不一致

| 量 | macOS owner（矩阵 JSON `verification`） | lithe-ui | 判定 |
|---|---|---|---|
| 圆角 | **4pt** | `Radius::Md` = **6**（`tooltip.rs:27`） | 不一致 |
| 字号 | **13pt** | `TextSize::Sm` = **12**（`tooltip.rs:30`） | 不一致 |
| 内边距 | 左右 **12** / 上 **8** / 下 **9** | `px_2()`=8 / `py_1()`=4（`tooltip.rs:25-26`） | 不一致 |
| 内边框 | **1pt** | **无**（`surface()` 只有 `bg`+`shadow`） | 不一致 |
| 背景（深/浅） | `#33353B` / `#FFFFFF` | `tooltip_bg`：深 `0x373533` / 浅 `0x181613`（`palette.rs:334/254`） | 不一致 |
| 文字（深/浅） | `#D1D3D9` / `#000000` | `tooltip_fg`：深 `0xf3f1f0` / 浅 `0xfcfaf7`（`palette.rs:335/255`） | 不一致 |
| 最大宽 | — | `tooltip_max_width()` = **256px**（`tokens.rs:467`） | 新增 |
| 延迟 | 保留延时语义 | `DELAY` = **400ms**（`tooltip.rs:18`） | — |

**最刺眼的一条**：lithe-ui 的**浅色** tooltip 背景是 `0x181613`（近黑），
文字 `0xfcfaf7`（近白）——**是深底浅字的反色气泡**；而 IDEA/macOS 浅色是
**白底黑字**（`#FFFFFF` / `#000000`）。这不是数值微差，是**视觉方向不同**。

**判定**：**这一项存在方向性差异**。是否构成「结构性冲突」取决于口径：
- 若判据是「lithe-ui 默认值必须等于 IDEA」→ 冲突成立。
- 若判据是「能否用平台档位调和」→ **不冲突**：tooltip 的 `bg`/`fg` 是两个独立槽，
  宿主注入 IDEA 主题时若给了对应键就会覆盖；`palette_for` 目前**没有映射 tooltip 键**
  （`theme.rs:361-493` 的映射清单里无 `tooltip.*`），所以**默认值残留**。
  补一条映射即可调和，不需推翻 macOS owner。

按登记时的推翻条件（「同一控件同一属性上直接矛盾……且无法用平台档位调和」），
**「视觉真源双轨可对齐」未被推翻**——因为四个不一致项**都能靠补档/补映射调和**。

## 5. 菜单打开动画：这一项是方向相反

| | macOS owner | lithe-ui |
|---|---|---|
| 要求 | 「直接打开且无系统弹跳动画」（矩阵 JSON `capability`）；note 明写 `animationBehavior = .none` | `menus/menu.rs:361` `Animation::new(motion::duration(motion::FAST, cx))`；`:451` `root.opacity(t)`；`:467-470` 子菜单叠 `ml(motion::NUDGE * (t - 1.0))` |
| 时长 | — | `FAST` = **120ms**（`motion/curve.rs:7`） |
| 位移 | 无 | `NUDGE` = **4px**（`curve.rs:13`） |
| 降级 | — | 仅 `theme.reduced_motion` 时降为 **1ms**（`curve.rs:19-23`） |

**判定**：这是唯一一处**方向相反**的项——macOS 明令「无动画」，lithe-ui 默认**有**动画。
但它**仍可调和**：`motion::duration` 已经读 `reduced_motion` 开关，
只需把菜单入场时长接到一个「IDE 档」上（或让宿主把该档设为 1ms）。
不需要推翻 macOS owner，也不需要删掉动画能力。

## 6. 图标族：原判断错误，已更正（两边同属 expui）

原稿据目录名 `legacy/` 推断「GPUI 用 IDEA 经典 UI、macOS 用 expui，是两代图标语言，
属真冲突」。**该判断被实测推翻，此处更正。**

| | GPUI `assets/icons/idea` | macOS `Resources/IDEAIcons` |
|---|---|---|
| `legacy/`（目录名） | **122 个文件** | **目录不存在** |
| `expui/` | **36 个文件** | **146 个文件** |
| 总计 | 297 svg | 304 svg |

**更正依据**：`rust/lithe-gpui/assets/icons/idea/SOURCES.md:11-15` 明写：

> 「并集补充：macOS 文件夹没有、但界面仍在用的旧图标……放在 `legacy/` 下……
> 它们来自 Windows 前端 `windows/tauri/src/ui/icons/**`……
> **与 macOS 那份同属 intellij-community expui**，只是这一版更全
> （macOS 侧缺 `ui/` 那批通用字形与部分文件类型）。」

实测佐证：GPUI `legacy/ui/` 与 macOS `expui/general/` **同名 36 个文件全部逐字节相同**；
`legacy/ui/chevronDown.svg` ≡ macOS `expui/general/chevronDown.svg`（SHA-256 一致）。

**所以 `legacy/` 是「来源为 Windows 派生副本、目录名叫 legacy」的 expui 美术，
两边的图形语言是同一套。** 目录名误导了原判断。

**判定**：**这一项不构成结构性冲突**。GPUI 与 macOS 用同一套 IDEA expui 图标语言。

**真正的问题在别处（本次发现的两条）**：

1. **`SOURCES.md:3` 声明的生成器 `--check` 失败 129 条**（exit 1），
   工具算出的「保留 63 / 删除 123」与现状（186 组常量、122 个 legacy 文件）相差极远。
   **声明的「生成 + 校验」链路对当前树不成立。**
2. **13 条同相对路径文件内容不同，其中 7 条是极性错误**（浅色档 == macOS 深色档）：
   `debugger/debug.svg`、`debugger/stop.svg`、`expui/general/chevronRight.svg`、
   `expui/general/search.svg`、`fileTypes/powershell.svg`、`fileTypes/text.svg`、
   `fileTypes/toml.svg`。这 7 条深浅两档画同一张深色美术，**浅色主题下偏暗**。
   可见性取决于渲染路径（走 `idea_icon_image` 的 71 处可见，
   走 `idea_icon_mono`/`idea_icon_svg_px` 的 52 处不可见）。

**对 §7 的影响**：原本被我列为「唯一够格称结构性冲突」的项，**不成立**。
故 §7 的结论**更强**了——没有发现任何结构性冲突。

## 7. 对判断「视觉真源双轨可对齐」的读数

登记时的推翻条件是：**「发现 macOS 共享 owner 的取值与 IDEA Community 真源在同一控件
同一属性上直接矛盾（例如圆角、行高、图标尺寸），且无法用平台档位调和。」**

逐项读数：

| 项 | macOS owner vs IDEA 真源 | 是否直接矛盾 | 能否档位调和 |
|---|---|---|---|
| 圆角 | 4/5/6/8/10 分角色 | 否（同一体系） | 能 |
| 树行高/图标/字号 | 24 / 16 / 13 | 否 | — |
| 树缩进 | 19 | 否（lithe-ui 侧另有 16 与 8+14） | 能 |
| 滚动条 | 14 / 7~12 / 1pt 边框 | 否 | 能 |
| 悬浮提示 | 4 / 13 / 12-8-9 / 1pt / 白底黑字 | 否 | 能（补 tooltip 键映射） |
| 菜单动画 | 无 | 否 | 能（接 IDE 档） |
| 图标族 | expui | **否**（GPUI 的 legacy/ 也是 expui，见 §6 更正） | — |

**结论**：**未发现 macOS owner 与 IDEA 真源在同一属性上的数值矛盾**。
所有不一致都是 **lithe-ui 侧**相对两者偏低、或残留了另一套默认值。
原稿认为「图标族是结构性冲突」，经更正后**该冲突不存在**。

因此按字面推翻条件，**「视觉真源双轨可对齐」被支持**（未被推翻）：
macOS 已落地的共享控件约定与 IDEA 真源之间**没有**结构性冲突，
GPUI 侧可按 IDEA 真源实现而不推翻 macOS 已有共享 owner。

**但必须同时记两条限制**：

1. **IDEA 真源只能经二手转述读**。仓库内无 IDEA 源码检出
   （`Get-ChildItem -Recurse -Filter 'jetbrains.json'` 只找到
   `rust/lithe-gpui/themes/jetbrains.json`，那是主题配色不是控件几何）。
   若日后检出 IDEA 源码并逐项核对，发现 macOS owner 某取值与 IDEA 不符，
   则「macOS owner 忠于 IDEA」这一前提被推翻——**本结论随之作废**。这是最脆的假设。
2. **IDEA 版本号有三个**，note 里并存：`fb72b4df43ab`
   （`2026-09-24-windows-git-log-multi-repository-groups.md:140`）、
   `c7f91397daa3a961b4e78bc634fe467a0a7d9ade`（`2026-10-02-macos-shared-scrollbar.md:19`）、
   `fb72b4df43aba102479eb0502d20b03586b9c5b8`（`2026-10-04-windows-commit-file-diff-preview.md:206`）。
   **不是同一个 revision**，这本身就是需要先统一的口径。

## 8. 未执行的检索与测试

- **未检出 IDEA Community 源码**，未做任何 IDEA 侧直接核对；IDEA 一列全部是
  note 与矩阵 JSON 的转述值。
- **未运行任何构建、测试、渲染或截图**。全部判定基于读源码常量。
- 未在 macOS 实机运行，未验证 macOS 共享控件在真实窗口中的观感。
- 未验证 `palette_for` 在运行时实际覆盖了哪些槽（需跑起来看 `S1_THEME` 打点）。
