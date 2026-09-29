# Agent 笔记：字体键（界面字体 / 代码字体 / 终端字号）与"字族必须已安装"

状态：已实现

## 先说结论

外观设置里落地了三个字体键：`fontFamily`（界面字体）、`monoFontFamily`（代码字体：编辑器与终端正文）
写主题的字族 token；`terminalFontSize`（终端字号）落在**终端视图自己的覆盖值**上。空串与 `0` 都表示
**不覆盖**，沿用主题或默认档。

两条开发者必须知道的规则：

1. **用户填的字族必须先确认系统已安装，认不出就不写入。** GPUI 在字族找不到时会在**首次布局那一行
   panic**，而字族是用户在设置文件里手写的字符串——一次手写错就能让应用再也起不来。设置页的下拉因此
   **只列已安装的字族**加一个「默认（不覆盖）」项，用户选不出会崩的值。
2. **`terminalFontSize` 不能写主题的 `mono_font_size`。** 那个 token 是**编辑器正文**在用的，终端正文
   用的是 typography 的 `sm` token（14px）；写它会连带把编辑器字号改掉，而且"终端字号"这个 token
   本来就不存在。

## 问题

- 外观设置里有界面字号（`uiFontSize`）与编辑器字号（`fontSize`），但**没有字体族**，也没有终端字号；
  而真源的设置页有这三个控件。
- 侦察时发现两处**事实错误**：文档与既有注释都写着"终端正文用 `mono_font_size`（编辑器与终端共用）"。
  读代码后确认终端正文走的是 typography 的 `sm` token，两者并不共用。
- **崩溃风险**：`gpui-component` 的 `theme/mono_font.rs` 模块文档明说 GPUI 在字族缺失时首次布局
  panic，连 `Font::fallbacks` 都救不了（回退链只在字族本身加载成功后才被查）。把这样一个值是用户
  输入直接写进主题，等于给用户一个"改错就起不来"的开关。

## 决策

### 一、三个键的落点与"不覆盖"语义

| 键 | 默认 | 落点 | 生效 |
| --- | --- | --- | --- |
| `fontFamily` | 空串 = 不覆盖 | 主题 token `Theme::font_family`（界面正文） | 立即 |
| `monoFontFamily` | 空串 = 不覆盖 | 主题 token `Theme::mono_font_family`（编辑器与终端正文） | 立即 |
| `terminalFontSize` | `0` = 不覆盖 | `TerminalPane::set_font_size`（终端视图自己的覆盖值） | 立即，作用于所有已开页签 |

`terminalFontSize` 的钳制范围**沿用编辑器字号那一档（10–22）**，不发明第三套范围；`0`、负数与非有限值
一律归一到 `0`（**不能被夹到下界**——那会把"不覆盖"变成"最小字号"）。

### 二、字族写入前必须校验已安装

- 写入前对照 `cx.text_system().all_font_names()`；认不出的**不写主题**并留诊断
  `S1_THEME font_family_rejected key=… family=… reason=not_installed`；
- `.SystemUIFont` 是 GPUI 的虚拟家族，**永远放行**；
- 已安装列表**按进程缓存一次**（枚举字体在 macOS 上要上百毫秒，与上游 `mono_font.rs` 同一种做法）；
- 判据抽成纯函数 `usable_family(key, value, installed)`，因此四种分支能脱离文本系统直测。

### 三、设置页只列已安装字族

两个字族控件用既有的下拉 helper，候选来自已安装列表 + 一个「默认（不覆盖）」项。这样"会崩的值"
在交互层就不可达，安全约束不依赖用户读文档。

## 考虑过的备选方案

### 加 `ligatures`（连字）键
设计草案里有。不采用的原因：上游 `FontFeatures` 结构存在，但编辑器那条样式链**没有任何地方读它**，
加键就是"存了没用"——本仓库明确不要没有消费方的键。`lineHeight` 与 `iconTheme` 同理不做。

### 把 `terminalFontSize` 写成主题的 `mono_font_size`
改动最小（复用现成的字号通道）。不采用的原因：会连带改掉**编辑器**字号，而用户要的是终端单独一档；
且终端正文本来就不读那个 token。

### 不校验字族，直接写主题
实现最简单。不采用的原因：见"问题"一节的崩溃风险——一个手写错的值会让应用再也起不来，而用户没有
任何线索指向"字体设置"。

### 校验失败就回落到一个安全字族（例如 `.SystemUIFont`）
行为上"总能起来"。不采用的原因：那是**静默改变用户的选择**；保留当前值 + 一条诊断更诚实，用户改回
正确值即可恢复。

## 后果

**收益**：外观设置与真源对齐；终端字号不再被迫与编辑器共用一档；用户手写字族写错只会被忽略并留诊断，
**不会**让应用起不来；设置页在交互层就避免了会崩的值。

**代价**：

- 已安装字族列表按进程缓存一次，所以**运行中新装字体不会出现在下拉里**，要重启；
- 字族下拉会列出机器上所有已装字族（可能几百条），每次打开下拉时构建菜单项，条数极多时的构建成本
  没有实测；
- `terminalFontSize` 的覆盖值只作用在终端视图的输出行上（终端是按行渲染的 `MessageScroller`，
  不是字符网格），所以它不是"终端字体渲染参数"的完整实现。

## 验证

- `cargo test --workspace`（gpui）：**393 通过 / 0 失败**；settings 144 条含三个键的归一化、
  序列化键名与 `usable_family` 的四分支；terminal 8 条。
- **端到端**（工作区之外的仓库；全局设置里同时放一个可用与一个不可用的字族）：

  ```text
  S1_THEME font_family key=fontFamily value=.SystemUIFont                     ← 虚拟家族放行
  S1_THEME font_family_rejected key=monoFontFamily family=NoSuchFontXYZ reason=not_installed   ← 不存在的字族被拒
  S1_SETTINGS wiring=workbench … terminal_font_size=18 …                      ← 终端字号进入转发链
  ```

  第二行在 **stderr**（诊断走 stderr），只在 stdout 里找会漏看。
- `cargo check --workspace --all-targets`：exit=0；静态 gate 与 Note 校验通过。

## 适用范围

- `rust/lithe-gpui/crates/settings/src/schema.rs`
- `rust/lithe-gpui/crates/settings/src/store.rs`
- `rust/lithe-gpui/crates/settings/src/theme.rs`
- `rust/lithe-gpui/crates/settings/src/dialog.rs`
- `rust/lithe-gpui/crates/terminal/src/terminal_view.rs`
- `rust/lithe-gpui/crates/terminal/src/session.rs`
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`
