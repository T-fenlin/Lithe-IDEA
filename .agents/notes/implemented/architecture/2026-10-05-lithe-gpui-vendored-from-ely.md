# Agent 笔记：lithe-gpui 改为移植 Ely 组件库

状态：已实现

## 先说结论

`lithe-gpui` 不再是从零手写的组件库，而是 **Ely GPUI Components 的衍生副本**
（vendored fork，即把别人的源码拷进本仓库、由我们长期维护的分支）。落地手法是
Cargo 的**包重命名**：让 Ely 源码里的 `use gpui::…` 解析到 Lithe 钉住的
`gpui-pre`，因此 847 个源文件的 import 一行没改。

开发者最需要记住三件事：改这个 crate 前先读同目录的 `UPSTREAM.md`；
**不要清掉源码里 30 处文档注释中的 "Ely"**（那是 MIT 要求的来源署名，不是待改的旧名）；
API 目前是 Ely 自己的词汇，**没有**对齐 gpui-kit。

## 问题

手写组件库的产出速度与目标（对齐 Ely 的 44 章规模）差太远：两个迭代只覆盖
13 章、约 36 个组件家族，其中大半还是"给 gpui-base 的 headless 行为套样式"。
而所需的东西——Ely 的视觉语言、44 章目录、千级变体——已经存在。问题变成：
能不能把已有的搬过来，而不是照着它重画一遍。

关键未知数是**改造深度**：Ely 绑定 zed 上游 gpui 的 git rev，与 Lithe 钉住的
`gpui-pre 0.3.7` 不是同一条版本线。如果 API 面差异很大，移植就等于重写。

## 决策

**移植 Ely 源码作为 `lithe-gpui` 的实现，替换原手写实现。**

事实支撑（决定这刀划不划得下去）：Ely 引用的 141 个 gpui 类型，**141 个在
gpui-pre 0.3.7 里都存在**；Ely 完全没用到 `.transform()` 这类已知有语义分歧的 API。
实测结果是 `cargo check --all-targets` 零错误零警告。

### 正确做法

- **靠包重命名消化版本差异**，不要改源码里的 import：

  ```toml
  gpui = { package = "gpui-pre", version = "=0.3.7", default-features = false, features = ["stacker"] }
  gpui_platform = { package = "gpui-pre-platform", version = "=0.3.7", features = ["font-kit"] }
  ```

  版本必须与其它 gpui 消费者（gpui-kit）一致，否则会是两个 crate 实例、类型不通用。
- **改标识符与用户可见文案，保留署名**：`ely_gpui_component` → `lithe_gpui`、
  `actions!(ely, …)` → `actions!(lithe, …)`、`"ely: …"` panic 文案 → `"lithe-gpui: …"`、
  元素 ID / 临时文件名里的 `ely*` → `lithe*`；文档注释里的 "Ely" 原样保留。
- **改动逐条记进 `UPSTREAM.md`**：来源提交、改了什么、同步上游的步骤。它是这个
  crate 的"我们与上游差在哪"的唯一真源。

### 不要这样做

- **不要引用 Ely 作为 git 依赖**：它 `publish = false`，且直接依赖 zed 仓库的 git rev，
  会把 zed 整棵依赖树拉进 workspace。必须是拷贝进仓库的副本。
- **不要为了"统一命名"清掉注释里的 Ely 字样**：抹掉来源是许可问题，不是整洁问题。
- **不要指望同名 API 行为一致**：名字对得上只保证能编译；行为差异要靠测试暴露，
  目前只修到能编译为止。

## 考虑过的备选方案

### 继续在 gpui-base 上手写

产出可控、测试是我们自己写的。但两轮只到 13 章，与 44 章的目标差距过大，
且每个组件都要重新实现一遍视觉语言。

### 只移植非演示章

少搬 258 个文件（Finance/Mail/Maps/Calendar 等）。但演示章与通用章共享
`theme`、`primitives`、`layout` 等底层，裁剪会在 `lib.rs` 与 `mod.rs` 上撕出大量
改动，且演示章本身也是回归样本。**决定全量搬入，不裁剪。**

### 把 API 全面对齐 gpui-kit（`Sizable`、路径注入图标、`new(id).label(…)`）

调用写起来统一。但 `pub fn new(` 出现在 **543 个文件**里，改定义还不够、每个内部
调用点都要跟着改；`IconName` 还和既有图标体系（expui）撞车。改动量/收益比最差，
**不做**。

## 后果

- 收益：一次性拿到 44 章、千级变体；布局、终端、图表、文档渲染、i18n 等
  此前没有的章节直接到位。
- 代价：**我们从此维护一个衍生分支**——上游更新要手动对照与合并；依赖面扩大
  （新增 ~17 个 crate，含 alacritty_terminal / pulldown-cmark / qrcode 等）；
  构建更重。
- 已知冲突（尚未解决，见下）：**字体与图标**。
- 需要重新评估的触发条件：上游与 `gpui-pre` 的 API 分歧扩大到需要频繁手改，
  或组件的可维护性证明不如自建。

## 验证

- `cargo check -p lithe-gpui --all-targets`：0 错误 0 警告（含画廊示例）。
- `cargo run -p lithe-gpui --example docs_hello`：窗口正常开启、无 panic。
  （Windows 上若 `stacker` 的构建脚本报 `cl.exe` exit 2 且无输出，把 `TEMP`/`TMP`
  指到仓库内目录再跑。）
- `Cargo.lock` 为纯增量，`gpui-pre`/`gpui-base` 版本未变动。
- 组件库当前**没有消费方**（宿主 crates 叫 `lithe-gpui-*`，是不同 crate），
  所以这次替换不动宿主。
- `.agents/skills/write-stable-tests/scripts/verify-test-stability.sh`

## 适用范围

- `rust/lithe-gpui/crates/lithe-gpui/**`
- 来源、改动清单与同步流程见同目录 `UPSTREAM.md` 与 `LICENSE-ELY`
- 被取代的"在 gpui-base 上手写"决策见
  `.agents/notes/archived/architecture/2026-10-05-lithe-gpui-component-library.md`

## 尚未解决

- **字体冲突**：Ely 以 **Inter** 作界面字体（`theme.font_family`），而
  `CONTEXT.md` 已定"界面字体用系统 UI 字体"。代码字体 JetBrains Mono 两边一致。
- **图标体系**：Ely 自带 307 个 Lucide SVG（ISC）与 `IconName` 枚举，与既有
  expui 图标方案（另一篇 Agent Note）并存，未合并。
- **API 对齐 gpui-kit**：未做，见上"备选方案三"。
- **语义漂移**：仅修到能编译；行为差异待测试暴露。
