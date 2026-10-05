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

## 章序（grill 定稿，按宿主痛点排序）

1 Primitives → 2 Typography → 3 Layout → 4 Window & Shell → 5 Buttons & Actions →
6 Forms → 8 Menus → 9 Overlays → 13 Lists & Trees → 14 Tables → 19 Git（DiffView）→
7 Navigation → Dock（3/4 章内）→ 10 Feedback → 11 Loading & Motion → 12 Data Display →
17 Editor → 18 Terminal → 其余按章号推进。

## 状态总览

| 章 | 名称 | 状态 | 组件 | 变体 | 测试 |
| --- | --- | --- | --- | --- | --- |
| 1 | Primitives | ✅ 本 PR | 10 | 36 | 2 窗口 + 语义/色板纯测试 |
| 2 | Typography | 待做 | — | — | — |
| 3 | Layout（含 Dock 面板系统） | 待做 | — | — | — |
| 4 | Window & Shell | 待做 | — | — | — |
| 5 | Buttons & Actions | 待做 | — | — | — |
| 6 | Forms | 待做 | — | — | — |
| 7 | Navigation | 待做 | — | — | — |
| 8 | Menus | 待做 | — | — | — |
| 9 | Overlays（含 Tooltip，自第 1 章移入） | 待做 | — | — | — |
| 10 | Feedback | 待做 | — | — | — |
| 11 | Loading & Motion | 待做 | — | — | — |
| 12 | Data Display | 待做 | — | — | — |
| 13 | Lists & Trees | 待做 | — | — | — |
| 14 | Tables | 待做 | — | — | — |
| 15 | Charts | 待做 | — | — | — |
| 16 | Finance | 跳过（范围 B） | — | — | — |
| 17 | Editor | 待做 | — | — | — |
| 18 | Terminal | 待做 | — | — | — |
| 19 | Git | 待做（DiffView 先行） | — | — | — |
| 20 | Debug | 待做 | — | — | — |
| 21 | Documents | 待做 | — | — | — |
| 22–26 | Collaboration / AI Chat / Agent / Generative / Media | 跳过（范围 B） | — | — | — |
| 27 | Files | 待做 | — | — | — |
| 28–32 | Messaging / Mail / Calendar / Project / Canvas | 跳过（范围 B） | — | — | — |
| 33 | DB & Dev Tools | 待做 | — | — | — |
| 34 | Dashboard & Monitoring | 待做 | — | — | — |
| 35 | Settings | 待做 | — | — | — |
| 36–37 | Account / Onboarding & Help | 跳过（范围 B） | — | — | — |
| 38 | Interaction | 待做 | — | — | — |
| 39 | Theme | 待做（高对比度/自定义色板入口） | — | — | — |
| 40 | i18n & a11y | 待做 | — | — | — |
| 41 | Maps | 跳过（范围 B） | — | — | — |
| 42 | Misc | 待做 | — | — | — |
| 43 | Library Tooling | 待做 | — | — | — |
| 44 | Rendering | 待做 | — | — | — |

## 第 1 章 Primitives 对账

| 组件 | 变体 | gallery | 测试 |
| --- | --- | --- | --- |
| Icon | 6（3 尺寸 + 2 语义色 + 1 自定义 px） | icon-* | 存在性（tests/primitives.rs） |
| Separator | 2（水平/竖直） | separator-* | 存在性 |
| Avatar | 4（4 档尺寸） | avatar-* | 存在性 |
| Badge | 8（6 变体 + 2 尺寸） | badge-* | 存在性 |
| Tag | 2（带/不带关闭钮） | tag-* | 回调点击通路 ×2 |
| Label | 2（普通/必填） | label-* | 存在性 |
| Kbd | 2 | kbd-* | 存在性 |
| Spinner | 2（2 档尺寸） | spinner-* | 存在性 |
| Progress | 3（25%/60%/100%） | progress-* | 存在性 + clamp 纯测试 |
| Severity | 5（5 语义级别） | severity-* | 区分度纯测试（light/dark） |

合计：10 组件 / 36 变体；主题语义契约 3 项（默认值锚定 / 模式切换 / 高对比度）。

## 范围备注（第 1 章）

- Ely 第 1 章的 `backdrop/layer/measure/files/focus/pressable/ghost/image` 是其
  画廊内部件：行为已由 gpui-base（FocusHandle/StateStyle/Measure/img/GlobalState）
  覆盖或与 Lithe 无关，按范围 B 决策不移植。
- `Tooltip` 依赖 overlay 层（base `TooltipOverlay` + Root plugin 挂载），移至第 9 章。
- `Progress` 为薄实现（2px 轨道 + 填充）；base 的 headless `Progress` 在第 14 章
  表格化时再评估是否收敛过去。
- Spinner 的 reduced-motion 联动随第 11 章（Loading & Motion）统一补齐。
