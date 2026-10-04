# Agent 笔记：编辑器正文容器塌成 0 高度，打开文件后编辑区一片空

状态：已实现

## 先说结论

"打开文件后编辑区一片黑、没有内容"的根因**不是文件没读到**，而是**正文容器高度为 0**：
gpui 的 `h_flex()` 在交叉轴上**居中**子元素（不是拉伸），而编辑区正文那一层只给了
`flex_1()`（管横向宽度）和 `min_h_0()`，**没有确定高度**；里面 gpui-component 的
`Editor::new(..).h(relative(1.))`（父高的 100%）因此解析成 0 —— 高度为 0 的 Editor 什么都不画，
用户看到的就是标签栏下面一整块背景色。修法是给那一层补 `.h_full()`，并补一条"正文容器真的拿到了
高度"的回归测试。**这条规则要记住：在 `h_flex()` 里想让子元素撑满整行高度，必须显式写
`.h_full()`；只有 `v_flex()` 才会在交叉轴上拉伸。**

## 问题

用户在真机上反馈"打开文件，里面直接黑屏，没有内容"。实测复现（debug 构建 +
`--session-probe <file>` 指定要打开的文件，`PrintWindow` 抓窗口）：

- 标签栏里 `hello.md` 已经打开，状态栏显示 `1:1 UTF-8 2 个空格`，标准输出里也有
  `S1_EDITOR_LANG name=markdown file=hello.md` 与 `S1_EDITOR_CURSOR line=1 col=1`
  —— 说明**文件确实被读进 buffer 了**（`crates/editor/src/buffer.rs` 的 `read_body` 走
  `std::fs::read`，与 Core 无关）；
- 但编辑区那块矩形**是纯色**：对截图做逐像素统计，编辑区范围内只有 **1 种颜色**
  `(43, 43, 43)`，与背景不同的像素 **0 个**（连行号、当前行高亮都没有）——
  不是"字是黑的看不见"，而是**根本没画任何东西**；
- 在测试里量同一个容器的 bounds：`size = 626px × 0px`（y 还落在编辑区竖直中点，
  因为它是被 `items_center` 居中的），补上 `.h_full()` 后同一处变成 `626px × 444px`
  （480 高的测试窗口减 36 的标签栏）。

## 决策

### 一、编辑区正文容器必须有确定高度

`crates/editor/src/editor_view.rs` 的 `render_body`：正文那一层（`EDITOR_HOST_ELEMENT_ID`）
在 `flex_1()` / `min_h_0()` 之外补 `.h_full()`。旁边那条断点列本来就是 `.h_full()`，
补完之后两个子元素都撑满整行了。

### 正确做法

- 行容器用 `h_flex()`（交叉轴居中）时，想让子元素占满整行高度：显式 `.h_full()`；
- 需要"子元素自动拉伸"的语义时，用 `v_flex()`（gpui-base 的
  `h_flex_centers_and_v_flex_stretches_on_the_cross_axis` 单测钉着这条差异），
  或者显式改对齐（`.items_stretch()`）；
- 内层用百分比高度（`.h(relative(1.))` / `.h_full()`）时，先确认**直接父级**有确定高度。

### 不要这样做

- 不要因为"界面上这块是空的"就去怀疑文件读取或 Core：本例 buffer 是好的，问题纯在布局；
- 不要给 `Editor` 写死一个高度（如 `.h(px(320.))`）来绕过：那样窗口一变高，编辑区就不跟着变，
  真正的修法是让**容器**有高度；
- 不要在没给容器确定高度的情况下依赖 `.h_full()` 之外的百分比：链上任何一层不定高，
  百分比都会退化成 0（这正是本缺陷）。

### 二、给正文容器一个稳定 ElementId 作为测试接缝

`EDITOR_HOST_ELEMENT_ID`（`lithe-editor-host`，从 `lithe_gpui_editor` 导出）+ `.test_support()`：
`crates/workbench/tests/ui.rs` 的 `editor_body_host_fills_the_pane_below_the_tab_bar`
用它断言"打开一个真实文件后，正文容器拿到 ≥100px 的高度"。**没有这条断言，
这个缺陷可以一路活到用户面前**：既有的编辑器用例只断言状态访问器（tab_size、只读、
当前标签名），从来不量布局。

## 考虑过的备选方案

### 备选方案一：把正文行改成 `.items_stretch()`

`h_flex()` 的居中语义是上游既定契约（gpui-base 有专门单测），改对齐会同时影响这一行里
断点列的定位方式；只给需要高度的那个子元素补 `.h_full()` 是更小的改动，也和旁边断点列的
写法一致。

### 备选方案二：给 `Editor` 固定高度

文档示例里确实有 `Editor::new(&editor).h(px(320.))`。但那会让编辑区不随窗口/分栏变化，
而且这条宿主里已经有可拖拽的分栏（编辑区高度由用户拖），固定值等于把缺陷换个形式留着。

### 备选方案三：只做真机验证、不写测试

真机截图（`PrintWindow`）确实能证明修好了，但截图不进 CI、也不防回归；
上一轮"欢迎窗口没有窗口键"的缺陷就是这么漏过去的（单测只测数据层，界面壳没人量）。

## 后果

- 收益：打开文件后能看到正文、行号与当前行高亮；`Editor` 的滚动/可见行计算也才有意义
  （高度为 0 时可见行范围恒空）。
- 代价：无（多一个 `ElementId` 与一条测试）。
- 需要重新评估的触发条件：上游 gpui 若把 `h_flex()` 的交叉轴语义从"居中"改成"拉伸"，
  这一处 `.h_full()` 变成冗余（但不会错）；`EDITOR_HOST_ELEMENT_ID` 只用于测试接缝，
  不要拿它当业务身份。

## 验证

```bash
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-editor            # 72 条全绿
node .agents/skills/write-stable-tests/scripts/run-rust-tests-with-timing.mjs \
    --manifest rust/Cargo.toml --package lithe-gpui-workbench \
    --suite-timeout-ms 180000 --report .artifacts/test-stability/gpui-workbench.json   # 205 条全绿
cargo fmt --manifest-path rust/Cargo.toml --all -- --check                  # 退出码 0
node lithe-gpui/tools/check-ui-px.mjs                                       # 违规 0
./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1        # 通过
```

- 新增用例 `ui::editor_body_host_fills_the_pane_below_the_tab_bar`：**修前实测
  `626px × 0px`（红）→ 修后 `626px × 444px`（绿）**。
- 真机对照（debug 构建 + `--session-probe <proj>\hello.md`，脚本 `.artifacts/probe-printwindow.ps1`，
  用 `PrintWindow` 抓窗口，**锁屏状态也能抓到内容**）：
  - 修前：`.artifacts/editor-probe/with-file/printwindow.png` —— 标签栏有 `hello.md`，编辑区整块空白；
  - 修后：`.artifacts/editor-probe/with-file-fixed/printwindow.png` —— 行号 1–6、`# hello`
    当前行高亮、"正文第一行 / 第二行" 全部画出。
- 逐像素判据（`.artifacts/analyze-editor-pixels.py`）：修前编辑区只有 1 种颜色、与背景不同的像素 0 个。

## 适用范围

- `rust/lithe-gpui/crates/editor/src/editor_view.rs` —— `render_body` 的正文容器与 `EDITOR_HOST_ELEMENT_ID`。
- `rust/lithe-gpui/crates/editor/src/lib.rs` —— 该 id 的导出。
- `rust/lithe-gpui/crates/workbench/tests/ui.rs` —— 正文高度的回归判据。
