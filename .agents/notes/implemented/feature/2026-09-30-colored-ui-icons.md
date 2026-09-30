# Agent 笔记：UI 图标走彩色渲染（`img()` 而不是单色 `svg()`）

状态：已实现

## 先说结论

IntelliJ `expui` 那套 UI 图标**本身是彩色的**，而界面里一直画成灰的 —— 因为 UI 图标走的是
`svg()` 管线（渲成 alpha 遮罩 + 用 `text_color` 上**单色**）。

现在改成：**expui 的 UI 图标走 `img()`**（与文件图标同一条管线，原色保留），
Lucide 那些单色字形继续走 `.icon(..)`（它们本来就没有颜色可保留）。

开发者要记住的三条：

1. **彩色图标不跟随 `text_color`**。悬停 / 选中的反馈靠**底色**（`ButtonCustomVariant::hover`、
   `Button::selected`），与 IDEA New UI 一致；不要再写"图标变色表示选中"的样式 —— 对 expui 不生效。
2. **要用哪个 helper**：`shared::icons::idea_icon_image(icon, cx, side)`（彩色）；
   只有确实想要单色时才用 `idea_icon_svg` / `idea_icon_svg_px`。
3. `idea_icon_image` **自己兜底**：资产取不到时回落到单色版本，所以调用点直接
   `.child(idea_icon_image(..))`，不必判空。

## 问题

维护者口径（2026-09-30）："界面图标怎么还是单色的？" 查证结果：

- `lithe-gpui/assets/ui-icons/idea/**` 的 **157 个 SVG，0 个**用 `currentColor`，
  色值全是硬编码的，而且每个图标都有 `light` / `dark` 两套
  （深色下另一组色值，由 `IdeaIcon::path(is_dark)` 选）。
- ⚠️ **但"硬编码"不等于"彩色"**（2026-09-30 复核，第一版笔记在这里说过头了）：
  expui 分两族 —— **内容类**（`general/warningDialog` 琥珀、`general/successDialog` 绿、
  `fileTypes/**` 文件类型）**真有颜色**；**镶边类**（`general/listFiles` / `general/settings` /
  `toolwindows/notifications` 等轨道与工具窗那一族）**是灰阶两档**：浅色 `#6C707E`、
  深色 `#CED0D6` —— IDEA 的镶边图标本来就是"安静的灰"，不是彩色。
- 而 `svg()` 这条管线把 SVG 当**形状遮罩**用：`SvgRenderer::render_alpha_mask` + 单一
  `style.text.color`（源码依据见 `shared/src/icons/file_icon.rs` 的 `theme_file_icon` 文档）。
  于是所有色值被抹掉，界面上就是"图标全是灰的"。
- **文件图标没有这个问题**：它们 2026-09-29 已经改走 `img()`（`FileIcon::render`），
  所以 `.java` 橙、`pom.xml` 蓝是彩色的。UI 图标那一批没跟着改。

## 决策

### 1. 新 helper `idea_icon_image`（`shared/src/icons/mod.rs`）

```rust
pub fn idea_icon_image(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> AnyElement
```

- 明暗选择沿用 `IdeaIcon::path(is_dark)`（与单色版本同一份判据）；
- 渲染**复用** `file_icon::theme_file_icon`（同一个 `img()` 实现、同一份"资源在不在"的检查），
  不另写一条 —— 两条彩色路径各写一份迟早漂移；
- 取不到资产时回落到 `idea_icon_svg_px`：**不画空白**，"找不到图"不该变成"少一块界面"。

### 2. 换掉的地方（本批）

| 位置 | 图标 | 尺寸 | 换管线后看到的差别 |
| --- | --- | --- | --- |
| 左活动栏轨道项（`workbench/src/activity_bar.rs`） | 项目 / 更改 / 搜索 / 诊断 / 设置 | `rems(1.)` = 16 | **只有诊断明显变彩**（警告三角 → 琥珀 `#FFAF0F` / `#F2C55C`）；其余几张是镶边灰（`#6C707E` / `#CED0D6`），只是从"主题前景色"换成 IDEA 自己那两档灰 |
| 右活动栏轨道项（同一文件） | 通知（铃铛） | `rems(1.)` = 16 | 铃铛资产是 `stroke="#6C707E"` / `#CED0D6` 的灰描边，**看着不变**（它本来就不是彩色图标） |
| 项目标签头的文件夹（`workbench/src/project_tabs.rs`） | `FOLDER_ICON` | `rems(14. / 16.)` | 镶边灰（同上） |
| 设置对话框头部（`settings/src/dialog.rs`） | `GEAR_ICON` | `rems(1.)` = 16 | 镶边灰（`#6C707E` / `#CED0D6`） |
| 状态栏（`workbench/src/status_bar.rs`） | 分支 / 锁 / 对勾 | `rems(14. / 16.)` | **本批没换**（原因见下）；换上去才是真正"变色"的一处 —— `successDialog` 是绿 `#55A76A` |

活动栏的 `ActivityIcon` 因此从 `button_icon(&self) -> Icon` 改成
`apply(&self, button: Button, cx) -> Button`：expui 走 `.child(idea_icon_image(..))`、
Lucide 走 `.icon(..)`（两条路各自用对的管线）。**几何与状态样式一行没动** ——
悬停与选中本来就是底色（`ButtonCustomVariant::hover` / `Button::selected`），
所以这次只换图标颜色，交互外观不变。

### 3. 状态栏**没换**（有意，附原因）

`workbench/src/status_bar.rs` 的 `entry_icon` 现在仍是 `svg()` + `text_color(muted_foreground)`。
那里的注释记着一次**真实失败**：早期用 `img()` 时状态栏那一格整块空白
（`statuszoom-before/after.png` 对比），当时归因于"`Img::request_layout` 只有拿到
`GlobalElementId` 才 `use_data`"。

那份归因后来被 `file_icon.rs` 的复核**推翻**了（那个 `global_id.is_some()` 只门控多帧动画续帧，
图像数据在 paint 阶段取，不需要全局 id），但**本批没有像素级验证手段**
（这台机器抓不到 gpui 的 GPU 帧，见 `.artifacts/probe/NOTES.md`），
所以在"能亲眼确认状态栏图标没变空白"之前不动它 —— 换它是一条 5 行的改动，
风险却是"状态栏图标整块消失"。状态栏那几个图标也因此仍是单色（含本应是绿色的
`successDialog` 对勾）。接手照 `S1_*` / 截图复核后可直接改成 `idea_icon_image`。

## 考虑过的备选方案

- **给每个状态各备一套彩色资产**（默认 / 悬停 / 选中）：能同时保住"彩色 + 图标随状态变色"，
  但 `assets/ui-icons/idea/**` 只有明暗两套，另外两套要么自己调色重导、要么改资产来源；
  而 IDEA 本身也不是这么做的（它的轨道图标不随悬停变色）—— 被否。
- **把彩色图标渲成图像后再 `text_color` 染色**：那又回到单色，等于没做。
- **自己预渲染 `RenderImage` 并缓存**（`cx.svg_renderer().render_single_frame(bytes, scale)`）：
  能同时解决"原色 + 任意 DPI 锐利"，但要自建缓存与失效逻辑；`file_icon.rs` 已把它登记为
  后续优化，本批不改（见下条代价）。
- **只改活动栏、别处不动**：会得到"轨道彩色、状态栏与标签头灰"的割裂界面 ——
  所以把项目标签与设置头部一起换了（它们没有历史失败记录），只有状态栏按上面那条留着。

## 后果

收益：

- expui UI 图标不再被主题前景色统一重画：**内容类**恢复真源颜色（活动栏的诊断警告三角
  从灰变琥珀；状态栏那个绿色对勾一旦也换过来同样会变绿）；**镶边类**换成 IDEA 自己的
  两档灰（`#6C707E` / `#CED0D6`），比"跟着前景色"更贴近真源。
- 彩色与单色两条路各有**一个**入口 helper，调用点选错的代价是"颜色不对"，不是"画不出来"
  （兜底在 helper 内部）。

代价与例外：

- **`img()` 固定按 `scale_factor = 1.0` 栅格化**（`gpui-pre/elements/img.rs` 里上游留了
  `TODO: Can we make SVGs always rescale?`），所以 125% / 150% DPI 下是把 16×16 位图放大，
  边缘比矢量遮罩**略软**。文件图标从 2026-09-29 起就是这个取舍（原色优先）。
- **彩色图标不再参与主题前景色**：深色主题下靠 `_dark` 变体，浅色下靠 `_light` 变体；
  如果将来加"高对比度"主题，需要额外准备资产（当前主题集没有这一档）。
- 状态栏仍是单色（原因见上）。

## 验证

- `cargo test -p lithe-gpui-shared --lib`（64）、`-p lithe-gpui-workbench --lib`（83）、
  `-p lithe-gpui-settings --lib`（169）、`cargo fmt --check`、
  `node rust/lithe-gpui/tools/check-ui-px.mjs`（仅剩 terminal 的 2 处既有违规）、
  `cargo build --bin Lithe`。
- **未做像素级验证**：本机抓不到 gpui 的 GPU 帧（`PrintWindow` 纯黑、`CopyFromScreen` 只有底色，
  见 `.artifacts/probe/NOTES.md`）。所以"图标真的画出彩色"这一条要靠维护者看一眼：
  活动栏「项目 / 更改 / 搜索 / 诊断 / 设置」应恢复成 IDEA 的彩色图标
  （诊断那一个是彩色警告三角，通知是彩色铃铛）。
- 已知可回退点：`idea_icon_image` 内部就是 `theme_file_icon` + 单色兜底，
  要退回全单色只需把三处调用点换回 `idea_icon_svg*`。

## 适用范围

- `rust/lithe-gpui/crates/shared/src/icons/mod.rs`（`idea_icon_image`）
- `rust/lithe-gpui/crates/shared/src/icons/file_icon.rs`（`theme_file_icon` 的管线依据）
- `rust/lithe-gpui/crates/workbench/src/activity_bar.rs`、`project_tabs.rs`
- `rust/lithe-gpui/crates/settings/src/dialog.rs`
- 已在第二批照换：`rust/lithe-gpui/crates/workbench/src/status_bar.rs`（`entry_icon`，见追加二）

## 追加：两处"以为它有色、其实没有"的地方（2026-09-30 复核）

维护者问到 Maven 与铃铛，逐个查了来源，结论是**这两处在真源里也不是彩色图标**，
换管线换不出颜色：

- **铃铛**（`icons/idea/icons/toolwindows/notifications.svg`）：`stroke="#6C707E"`
  （深色版 `#CED0D6`）—— IDEA 的工具窗图标都是灰描边。本批已经把它换到 `img()` 了，
  所以"看着没变"是**正确结果**，不是没切换。
- **Maven**：真源 `features/maven/components/maven-icon.tsx` 那枚 16×16 字形是
  `fill="currentColor"`（**单色**，跟随文字色），所以 Windows 前端的轨道 Maven 也是单色的；
  IDEA 自己的 Maven **工具窗**图标（原 `legacy-ide-icons/maven/toolWindowMaven.svg`，`#CED0D6` 的灰；
  该归档目录已于同日从磁盘删除，文件在 git 历史里）是 `#CED0D6` 的灰。
  我们**有**的那枚蓝色 Maven 标志（`icons/idea/icons/fileTypes/maven.svg`，`#3574F0`）
  是**文件类型**图标，已经用在文件树 / 编辑器标签的 `pom.xml` 上（那处是彩色的）。
  右栏 Maven 现在用 Lucide `package` 字形，是"真源只有内联组件、没有 SVG 可搬"时的既有取舍。
- 想让右栏 Maven 显示**彩色蓝色 Maven 标志**（有意偏离真源与 IDEA 的灰轨道图标）：
  一行就能换 —— 把 `icons/idea/icons/fileTypes/maven.svg` 包成
  `ActivityItem::idea`（该资产**已在内嵌资源里**）。
  ⚠️ 前提是接受"轨道图标不再与 IDEA 一致"。

## 追加二：界面图标全面 expui 化 + 资产归一（2026-09-30 第二批，维护者决策）

### 先说结论

维护者拍板："所有图标都走 `img()`"。据此做了两件事：

1. **资产归一**：`lithe-gpui/assets/ui-icons/idea/**`（157 个 expui 界面 SVG）**并入**
   `assets/icons/idea/icons/**` 图标包，`ui-icons/` 目录删除。从此界面图标与文件类型图标
   **同一个包、同一棵 expui 树**；`icons/idea` 现在还是 4 套里的 2 套（idea 内置 + pierre 可卸载）
   之外未接线包（lithe/symbols/material）的邻居，但只有 idea+pierre 进二进制。
2. **全量清扫**：凡 expui 有等价字形的 Lucide 用点（约 70 种字形：箭头/雪佛龙/关闭/加号/
   撤销重做/运行停止/警告错对/文件夹/git 族…）全部换成 `idea_icon_image(&idea::CONST, cx, side)`；
   expui **没有**等价物的（终端、窗口控制、Package、Puzzle、Panel*、GitGraph、各 save 变体等）
   保留 Lucide 单色 `.icon(..)`。**两条管线的分工从此是"资产有没有"而不是"想不想"**：
   有 expui 字形就必须走彩色管线，没有的才轮到 Lucide。

开发者要记住的：

- 替换尺寸口径：Icon 默认 → `rems(1.)`；`.small()` → `rems(14. / 16.)`；`.xsmall()` → `rems(0.75)`；
  `.large()` → `rems(1.5)`。按钮槽位放彩色图标用 `.child(idea_icon_image(..))`
  （gpui-kit 官方 image.md 画廊示例与 `ActivityIcon::apply` 同款），不再用 `.icon(IconName::..)`。
- `ui-icons/` 已不存在，新代码引用界面图标一律 `idea::常量`（`shared/src/icons/idea.rs`，
  由 `tools/generate-idea-icons.mjs` 从 `assets/icons/idea/icons/` 生成，128 个图标），
  不要再手写字符串路径。
- 上面"追加"一节提到的 `legacy-ide-icons/maven/toolWindowMaven.svg` 已随归档目录从磁盘删除
  （`legacy-ide-icons/`、`gutter-icons/`、`database-icons/`、`feature-icons/` 四个未接线归档目录
  于同日删除，git 历史可溯；`third_party/dbx/manifest.json` 的 retainedArtifacts 已清空）。

### 与"状态栏没换"一节的关系

第一批有意跳过状态栏（怕 `img()` 画空白）。第二批按维护者指令照换，风险已不存在：
`idea_icon_image` 现在自带"资产取不到 → 回落单色 svg"的兜底，最坏结果是"灰"而不是"空白"。

### 验证

`cargo test -p lithe-gpui-app`（内嵌口径 411 个文件 / 3 319 379 字节，
**这个 pin 的值已在追加三里更正为 3 318 863**，由
`embedded_range_stays_within_the_documented_envelope` 精确钉住）、
`node rust/lithe-gpui/tools/generate-idea-icons.mjs --check`、
`cargo fmt --check` 与受影响 crate 的 `cargo test`。像素级复核仍受本机限制（抓不到 GPU 帧），
彩色是否生效请维护者目验。

## 追加三：`expui/` 一层拉平（2026-09-30 第三批，维护者决策）

### 先说结论

本地资产树不再镜像上游的目录名：`icons/idea/icons/expui/<分类>/` 的 253 个 SVG 整体上移一层，
变成 `icons/idea/icons/<分类>/`（git 全部识别为重命名）。包根下直接就是分类目录，与
`icons/{pierre,lithe,symbols}` 的"包根 = `extension.json` + 一个 `icons/` 子目录"这条约定一致。

开发者要记住的：

- **清单**：`icons/idea/extension.json` 的 106 条路径（82 `fileTypes` + 24 `nodes`）由
  `./icons/expui/X` 改为 `./icons/X`；`tools/generate-idea-icons.mjs` 的扫描根
  （`assets/icons/idea/icons/`）与输出前缀（`icons/idea/icons`）**没动**，产物路径自动跟随 ——
  384 行 diff 全部只差 `expui/` 一段，128 个常量名与 16 个别名零变化（旧前端那 144 个显示名
  = 128 个规范常量 + 16 个 `ALIASES` 别名，一个都没丢）。
- **溯源与布局分家**：`tools/idea-icon-mappings.json` **一个字节都没改**，因为它的 `source`
  记的是上游路径。生成器新增 `UPSTREAM_PREFIX = "expui"`，查显示名时把这一层拼回去。
  要拿本地路径回上游核对，对应关系是 `<分类>/x.svg` ⇄ `platform/icons/src/expui/<分类>/x.svg`。
- **文件数不变、字节数变了**：只改路径、不减文件，仍是 **411 个文件**；但清单是**文本**，106 条
  路径去掉 `expui/` 后短了 **636 字节**，内嵌字节总数从 **3 319 499** 降到 **3 318 863**。这次实测
  同时暴露了旧登记值 `3 319 379` 的偏差：它比合并提交当时的实测值少 **120**（`d76bd6ee` 改过清单
  却没重测那个 pin），`embedded_range_stays_within_the_documented_envelope` 在本次改动**之前**
  就是红的 —— 本次一并改成实测值。**开发者注意**：改 `assets/` 下任何文件（含清单这种纯文本）
  都要重跑该测试，字节口径钉的是内嵌字节。包清单 `version` 0.2.0 → 0.3.0（延续第二批
  `0.1.0 → 0.2.0` 的做法）。搬运记录见 `rust/lithe-gpui/assets/README.md` 第 9.5 节。

### 验证

`node rust/lithe-gpui/tools/generate-idea-icons.mjs --check`（产物与文件系统一致）、受影响的
`cargo test -p lithe-gpui-shared -p lithe-gpui-app`（清单路径与内嵌口径）、以及全仓库残留
`expui/` 路径引用的检索。
