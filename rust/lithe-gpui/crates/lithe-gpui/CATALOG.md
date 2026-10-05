# lithe-gpui 组件目录（对账表）

真源分工：本表管"做了什么、还差什么"；gallery 示例管"长什么样"
（`cargo run -p lithe-gpui --example gallery`）；架构决策见 Agent Note
`.agents/notes/implemented/architecture/2026-10-05-lithe-gpui-component-library.md`。

## 口径

- 对齐 Ely GPUI Components 的 44 章框架；"组件数"口径 = 组件 × 变体的展示数
  （Ely 官网：1277 components / 623 stories / 44 chapters），不是独立 API 数。
- 范围决策（grill 2026-10-05）：全做 1–15、35、38–40、43、44；IDE 章 17–21、27、33–34；
  跳过纯演示章 16、22–26、28–32、36–37、41（其中验证过通用的零件随需收编）。
- 章号沿用 Ely 编号，跳过的章留洞，将来想补随时能对齐。
- 视觉值来源：色板逐项移植 Ely（MIT OR Apache-2.0，见 `src/theme/palette.rs` 头注）；
  圆角刻度 Sm 4 / Md 6 / Lg 8 / Xl 12；字体不移植 Ely（保持系统 UI 字体）。
- 可观测性约定：所有带 `ElementId` 的组件根都跟 `.test_support()`（gpui-base 的
  元素观测只登记显式包裹的元素）。

## 章序（grill 定稿，按宿主痛点排序）

1 Primitives → 2 Typography → 3 Layout → 4 Window & Shell → 5 Buttons & Actions →
6 Forms → 8 Menus → 9 Overlays → 13 Lists & Trees → 14 Tables → 19 Git（DiffView）→
7 Navigation → Dock（3/4 章内）→ 10 Feedback → 11 Loading & Motion → 12 Data Display →
17 Editor → 18 Terminal → 其余按章号推进。

## 状态总览

| 章 | 名称 | 状态 | 组件 | 备注 |
| --- | --- | --- | --- | --- |
| 1 | Primitives | ✅ | Icon, Separator, Avatar, Badge, Tag, Label, Kbd, Spinner, Progress, Severity | 36 变体见 gallery |
| 2 | Typography | ✅ | Heading(5 级), Text(4 变体), Code | |
| 3 | Layout | ✅（部分） | Card, CardTitle, SectionHeader, Spacer | Resizable 样式化、Grid 助手、**可滚动容器/Scrollbar 样式化**（base `Scrollbar`+`ScrollbarStyles`，gallery 暂用原生 `overflow_y_scroll`）待做；Dock 见章 3/4 备注 |
| 4 | Window & Shell | ✅（部分） | TitleBar, StatusBar, Sidebar | Dock/面板系统（base dock 全套）按计划单独成批，见下"Dock 备忘" |
| 5 | Buttons & Actions | ✅ | Button(5 变体×4 尺寸), IconButton, ButtonVariant, Link | Toggle/ToggleGroup 包装待做 |
| 6 | Forms | ✅（部分） | Checkbox, Switch, Radio, Input(+InputState/InputEvent 再导出) | Textarea/NumberInput/Slider 包装待做 |
| 7 | Navigation | ✅（部分） | Tabs, Tab, Breadcrumb, Pagination | Accordion/Collapsible 包装待做 |
| 8 | Menus | ✅（v1） | Menu, MenuItem, MenuEntry（open_menu/close_menu） | 键盘导航（Esc/方向键）与子菜单待做 |
| 9 | Overlays | ✅（v1） | OverlayState 插件（通知卡片 + 菜单承载层） | Dialog/Sheet/Tooltip/Popover 挂载待做（base 已有 headless 全套） |
| 10 | Feedback | ✅ | Alert, Empty | |
| 11 | Loading & Motion | 待做 | — | Spinner reduced-motion 联动归此章 |
| 12 | Data Display | 待做 | — | |
| 13 | Lists & Trees | 待做 | — | |
| 14 | Tables | 待做 | — | |
| 15 | Charts | 待做 | — | 色板 chart/ansi/syntax 已就位 |
| 16 | Finance | 跳过（范围 B） | — | |
| 17 | Editor | ✅（v1） | CodeBlock（只读） | 语法高亮待 base tree-sitter feature；可编辑器随 input 编辑器模式 |
| 18 | Terminal | 待做 | — | |
| 19 | Git | ✅（v1） | DiffView(unified + stats), DiffLine/DiffLineKind, CommitList, CommitSummary | split 视图随表格章；CommitGraph 待做 |
| 20 | Debug | ✅（v1） | FrameList/StackFrame, VariableList/Variable | 断点/监视列表待做 |
| 21 | Documents | 待做 | — | |
| 22–26 | Collaboration / AI / Media | 跳过（范围 B） | — | |
| 27 | Files | 待做 | — | |
| 28–32 | Messaging / Mail / Calendar / Project / Canvas | 跳过（范围 B） | — | |
| 33 | DB & Dev Tools | 待做 | — | |
| 34 | Dashboard & Monitoring | 待做 | — | |
| 35 | Settings | 待做 | — | |
| 36–37 | Account / Onboarding & Help | 跳过（范围 B） | — | |
| 38 | Interaction | 待做 | — | |
| 39 | Theme | 部分 | 高对比度/light/dark 已入 Theme | 自定义色板入口待做 |
| 40 | i18n & a11y | 待做 | — | |
| 41 | Maps | 跳过（范围 B） | — | |
| 42 | Misc | 待做 | — | |
| 43 | Library Tooling | 待做 | — | |
| 44 | Rendering | 待做 | — | |

## Dock 备忘（章 3/4 的遗留大件）

base 的 `dock` 模块（Panel trait、DockArea、TabGroup、拖拽落点、panel registry）是
"面板可复用"目标的承载层。放在 8/9 章 overlay 稳定之后单独成批实现：它依赖
Menu/Tooltip/DnD 与拖拽性能边界（见
`.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`）。

## 第 1 章对账（本批之前已交付）

10 组件 / 36 变体：Icon(6)、Separator(2)、Avatar(4)、Badge(8)、Tag(2)、Label(2)、
Kbd(2)、Spinner(2)、Progress(3)、Severity(5)。

## 本批（2–10、17、19、20）对账

| 组件 | 变体/要点 |
| --- | --- |
| Heading | Display/H1/H2/H3/H4 |
| Text | default/muted/subtle/small |
| Code | 行内代码 chip |
| Card / CardTitle / SectionHeader / Spacer | 容器类，Styled 细化透传 |
| TitleBar / StatusBar / Sidebar | Ely 外壳刻度（38px 标题栏、sunken 状态栏、240/56px 侧栏） |
| Button | Primary/Secondary/Outline/Ghost/Danger × XSmall/Small/Medium/Large + selected/disabled/自定义 px |
| IconButton | 同变体，方形 |
| Link | 外链专用（产品约定：站内命令走 Button） |
| Checkbox / Switch / Radio | 受控 on_change；交互根复用 base Button（焦点/键盘/禁用） |
| Input | 受控 InputState；base 别名经公开再导出（InputBaseState<InputMode>） |
| Tabs / Tab | 受控 selected_index + on_select(ix) |
| Breadcrumb / Pagination | 可点条目回调；分页 on_change(page) |
| Menu / MenuItem / MenuEntry | open_menu/close_menu；danger/disabled/分隔线；点击外部关闭 |
| Notification | push_notification；severity 语义色；手动关闭 |
| Alert / Empty | severity 语义色 + subtle 底 |
| CodeBlock | 行号栏 + 等宽只读 |
| DiffView / DiffLine / DiffStats | unified 视图、行号双列、+/− 语义色 |
| CommitList / CommitSummary | 选中回调 |
| FrameList / StackFrame、VariableList / Variable | Debug 数据列表，changed 用 warning 语义 |

## 范围备注

- Ely 第 1 章的 `backdrop/layer/measure/files/focus/pressable/ghost/image` 是其
  画廊内部件：行为已由 gpui-base 覆盖或与 Lithe 无关，按范围 B 决策不移植。
- `Tooltip` 依赖 overlay 层挂载，随第 9 章第二期；`Dialog/Sheet/Popover` 同批。
- `Progress` 为薄实现；base 的 headless `Progress` 在第 14 章表格化时再评估收敛。
- Spinner 的 reduced-motion 联动随第 11 章（Loading & Motion）统一补齐。
- 本 gpui fork 的三个实现约束（已内化）：`Pixels` 字段私有（取值走 `f32::from`）、
  `Pixels/Pixels` 除法返回 `f32`、`Transformation`/`.transform()` 仅 SVG 可用
  （Div 旋转在动画闭包内逐帧重排 children）。
