# Agent 笔记：外观可被工作区覆盖（九键白名单、三态来源可见、一键改回）

状态：已实现

## 先说结论

外观键现在可以在**工作区层**被覆盖，生效顺序是
**内置默认 < 全局 `settings.json` < 工作区 `.lithe/settings.json`（共享）< 工作区 `.lithe/settings.local.json`（本机）**。
可覆盖的只有**九个外观键**（主题四键 + 三个字号 + 两个字族），语言、终端 shell、缩进、工具链五值、
Git 键**仍然只有全局层**——写进工作区文件里也会被当未知键逐字保留，但**不生效**。

开发者需要记住三条：**合并顺序只有一份实现**（`workspace::resolve_effective`，不要再写第二遍"项目优先"）；
**界面必须能说出"这个值来自哪一层"**（团队设置 / 本项目的设置 / 你的个人覆盖，靠一条 Git 索引判据区分）；
**改一个外观值永远只写本机层，而落盘永远只写全局层那个字段**（否则改一次字号会把工作区覆盖写进全局文件）。

## 问题

1. **外观键只能全局**：`Settings` 里十九个键从前全是一个全局文件，所以"团队统一主题与字号"这件事
   在产品里**没有落点**——同事拉到一个仓库，看到的还是自己的主题。
2. **但"外观只跟人走"的直觉也是真的风险**：允许工作区覆盖外观之后，打开同事的仓库会看到
   "我的字号莫名变了"。这个困惑必须被对冲掉，否则宁可不做这个能力。
3. **覆盖值一旦越界就能把界面弄坏**：`uiFontSize` / `fontSize` / `terminalFontSize` 本来有钳制范围
   （10–24 / 10–22 / 0 或 10–22），如果工作区覆盖走另一条路，就会出现"界面显示一个值、实际生效另一个"。
4. **写入方向很容易做错**：设置实体同时持有"生效值"与"全局层"。只留生效值的话，用户在有工作区时
   改一次字号，就会把工作区覆盖的数值写进**全局**文件——这属于静默的数据污染，跨项目都会跟着变。

## 决策

### 一、可覆盖范围是九键白名单，其余键只有全局层

`AppearanceKey::ALL` 就是白名单，共九键：

| 组 | 键 |
| --- | --- |
| 主题（四键） | `theme` / `syncSystemTheme` / `autoThemeLight` / `autoThemeDark` |
| 字号（三键） | `uiFontSize` / `fontSize` / `terminalFontSize` |
| 字族（两键） | `fontFamily` / `monoFontFamily` |

**主题为什么是四个键而不是一个**：跟随系统时生效主题由 `autoThemeLight` / `autoThemeDark` 决定，
`theme` 那一格根本不被读。只让 `theme` 可覆盖，会出现"工作区设了主题，但用户开着跟随系统，
于是**静默无效**"——用户看不到任何反应，也查不出原因。所以设置页按
`workspace::effective_theme_key`（"这一刻真正决定生效主题的那个键"）判来源，
一键改回则一次清四个键。

**其余键（语言、终端 shell、缩进、工具链五值、Git 键）不在白名单里**，工作区文件里写了它们
不会改变生效值。守卫测试 `non_appearance_keys_are_not_overridable` 逐键断言这一点。
给开发者的话：**想加一个可被工作区覆盖的键，先改 `AppearanceKey` 与
`AppearanceOverlay`，再加一条"非外观键不生效"的对应断言**；不要绕过白名单直接合并。

### 二、合并顺序只有一份实现

```text
内置默认 < 全局 < .lithe/settings.json < .lithe/settings.local.json
```

唯一的落点是 `workspace::resolve_effective(global, shared, local)`
（`rust/gpui/crates/settings/src/workspace.rs`）：设置页显示的生效值、主题应用、字号转发、字族校验
走的都是它算出来的那一份 `Settings`。**不要在任何别处再写一遍"项目优先"**——两份实现漂移的表现是
"界面显示 A、实际生效 B"，这是最难查的一类错（与工具链五值的 `ToolchainPaths::resolve` 同一条口径）。

"未设"与"设成默认值"是两件事：`AppearanceOverlay` 的字段是 `Option<..>`，
`None` = 这一层没管这个键（交给下一层），`Some(14.0)` = 这一层显式定了 14。

### 三、三态来源：`git ls-files --error-unmatch` 的退出码区分"团队"与"本项目"

同一份 `.lithe/settings.json` 有**两个**状态，"文件在不在"区分不了它们，判据只能是 Git 的索引
（`rust/gpui/crates/workbench/src/workspace.rs` 的 `workspace_settings_tracked`）：

| 覆盖来自哪里 | 界面怎么说 | 判据 |
| --- | --- | --- |
| `.lithe/settings.local.json` 有这个键 | 当前外观来自**你的个人覆盖** | 本机层有这个键 |
| `.lithe/settings.json` 有这个键，**退出码 0** | **团队设置** | 文件已在 Git 索引里 |
| 同上，退出码非 0 / 不是仓库 / Git 不在 / 问不到 | **本项目的设置** | 更弱也更安全的那个说法 |
| 两层都没有这个键 | **不画任何标注**（跟随全局是默认形态） | — |

三态各有一个 locale 键（`settings.gpui.appearanceSource{Team,Project,Local}`，外加一个不画的 `Global`），
守卫测试 `the_three_sources_have_distinct_locale_keys` 断言它们互不相同。

**问不到 Git 时按"尚未提交"显示**（`source_for` 的 `shared_tracked: None` 分支）：
把未跟踪的文件说成"团队设置"会让用户以为同事也看得到它，那是**更强**的承诺，不能默认给。
只问一次、走 `background_spawn`（Core 要起 Git 子进程），答复回来走
`SettingsStore::set_workspace_settings_tracked` 回填。

### 四、写入分流：改外观写本机层；**落盘恒写全局层字段**

| 情况 | 写哪里 | 为什么 |
| --- | --- | --- |
| 有工作区 | `.lithe/settings.local.json` | 用户自己的改动**不该悄悄进团队文件**；共享层要由"共享此项目的配置"那个显式动作写（那一批还没落地） |
| 没有工作区 | 全局 `settings.json` | 与改动之前的行为完全一致 |

⚠️ **最容易做错的一处**：`SettingsStore` 同时持有生效值（`settings`）与全局层（`global`），
而全局文件落盘（`store.rs` 的 `write()`）**只序列化 `self.global`**。写成 `self.settings` 的后果是
"改一个字幕大小"把工作区覆盖的值写进全局文件——用户跨项目的默认外观被静默改掉，而且原因看不出来。
所有 `set_*` 入口因此分成两条：`commit_global`（非外观键）与 `commit_appearance`（外观键）。

工作区那一层**不参与 300ms 防抖**，改动当帧同步落盘：防抖器管的是全局设置文件那一份真源，
而工作区覆盖只是两份很小的 JSON，并且用户可能马上切项目——外壳重建后旧实体就没了，
防抖窗口里的改动会跟着丢。

### 五、一键改回清**两层**，且把键删掉而不是留 `null`

`revert_appearance_group_to_global`（单键入口 `revert_appearance_to_global`）把这个键从
`.lithe/settings.json` **与** `.lithe/settings.local.json` 两处都删掉。

- **为什么两层一起**："工作区那一层"由共享层与本机层共同构成（本机层盖在共享层上）。
  只清本机层的话，一个同时被两层覆盖的键会回落到**共享层的值**，而按钮的名字
  （「改回我的全局外观」）承诺的是**全局值**。
- **为什么删键而不是留 `null`**：`null` 不是这份文档的形状（用户手改时读不出"这个键被清掉了"与
  "这个键从来没写过"的差别），所以 `workspace::merged_document` 在 `preserve_unknown` 之上
  补一步"把未设的键从对象里删掉"。清完只剩 `{"version": 1}`。
- **`restore_defaults` 有意不清工作区覆盖**：一个叫"恢复默认设置"的按钮去改团队/项目文件是错的。
  被覆盖的键照旧显示工作区值，那一行的来源标注就是解释；日志留
  `S1_SETTINGS reset_to_defaults workspace_overridden=…` 让"按了没反应"可排查。

### 六、钳制只有一份实现，文档语义不重写第二版

- 合并之后调一次 `Settings::normalize()`，再用 `AppearanceOverlay::clamp_with(&effective)`
  把**归一之后的值**写回覆盖层。文件里写 `99` 的话，界面与文件里都会是 `22`，
  不会出现"界面上是 24、文件里是 99"。测试 `clamping_is_reused_from_the_settings_normalizer`
  对着 `schema` 的常量（`UI_FONT_SIZE_MAX` / `EDITOR_FONT_SIZE_MAX` / `TERMINAL_FONT_SIZE_UNSET`）断言。
- 读走 `shared::document::parse`，写走 `preserve_unknown` + `save_json`（临时文件 + rename），
  文档 `version = 1`；版本过新的文件**只读**（内存里照常生效，但拒绝覆盖）。
- 键名在文档里出现**第二处**（`AppearanceKey::json_key()`，因为"从原始对象里删一个键"只能按字符串做），
  守卫是 `json_keys_come_from_the_overlay_schema`：逐个键断言它属于
  `known_keys::<AppearanceOverlay>()`，且能反查回同一个键。

## 考虑过的备选方案

### 不做工作区覆盖层（外观只跟人走）
零风险、零交互成本，也永远不会出现"打开同事的仓库字号变了"。不采用的原因：团队统一主题与字号是
维护者确认过的真实需求，IDEA 也有 Project 级配色方案。风险改用"独立文件 + 来源可见 + 一键改回 +
沿用钳制范围"四条对冲，它们都是本批的硬要求而不是可选项。

### 让所有键都能被工作区覆盖（不做白名单）
实现更省事——`resolve_effective` 就不需要`AppearanceKey` 这一层了。不采用的原因：语言、终端 shell、
缩进、工具链五值、Git 键的"项目层"各有自己的契约与落点（工具链还要拆"存名字 / 存路径"两层），
混进外观这条通道会让"工作区能不能改这个键"变成一句没有答案的话。白名单把答案钉在类型里。

### 把工作区覆盖写进全局文件
少一层文件、少一套读写。不采用的原因：那正是**静默数据污染**——用户在一个项目里改字号，
会改掉所有项目的默认外观，而界面上看不出发生了什么。

### 只给"来源可见"，不给"一键改回"
少一个动作、少一块界面。不采用的原因：用户看到"我的字号被团队设置改了"却只能去手改 JSON，
这才是最容易让人放弃这个能力的地方；一键改回同时是"来源三态"这条验收标准的落点。

### 一键改回只清本机层
更符合"改回我自己的"的字面直觉。不采用的原因见第四节：被两层都覆盖的键只会回落到**共享层的值**，
按钮变成"按了没用"，而且用户无法从界面解释为什么。

### 清掉覆盖时留一个空值（写 `null`）
实现最简单（typed 一方胜出即可，不用额外删键）。不采用的原因：`null` 不是这份文档的形状，
用户手改时读不出"清掉了"与"没写过"的差别；`merged_document` 因此多一步删除。
这条差别也写在 `workspace.rs` 的模块文档里，作为"为什么不直接用 `merge_document`"的答案。

### 钳制自己抄一份范围（或者不钳制）
不采用的原因：抄一份必然漂移（`normalize` 那边一改就两份），不钳制则工作区可以写
`"fontSize": 999` 把界面弄坏。复用 `normalize` + `clamp_with` 的代价只是"合并之后要再写回一次"。

### 问不到 Git 时按"团队设置"显示
看起来更"正常"（那是共享层文件的默认期待）。不采用的原因：那是更强的承诺；
判据拿不到时应该退到更弱、更安全的那一侧，否则用户会以为同事也看得到这份覆盖。

## 后果

**收益**：

- "团队统一主题与字号"第一次有了落点，且**外观之外的键没有被顺手打开**（白名单是类型级约束）；
- 生效值来自哪一层在界面上**可区分**（三种来源各有文案），"为什么我的字号变了"有答案；
- 一键改回是**单步**的，并且真的回落到用户自己的全局值；
- 钳制、文档语义（version / 未知键 / 原子写）全部复用既有实现，没有第二份范围表、第二版 version、
  第二套原子写。

**代价与未做到的部分**（如实登记，不要读成"全部验证通过"）：

- **共享层今天没有写入方**：用户改外观只写本机层；`.lithe/settings.json` 只能手写或等
  "共享此项目的配置"那一批。所以"同事的主题覆盖我"这条链路**只走通读的一半**。
- **工作区设置文件没有 watcher**：外部手改 `.lithe/settings.json` / `settings.local.json`
  要**重新打开项目**才生效（打开 / 切换项目时重读是本批的下限）。将来要加必须监听**父目录**——
  这份文件也是"临时文件 + rename"写的，文件级监听会被 rename 换掉。
- **对话框里"来源标注 + 一键改回按钮"没有像素级取证**：取证机器当时锁屏、无法注入鼠标，
  所以"按钮点下去"那一段是用 `--appearance-revert` 探针走**同一个 store 调用**验的，
  界面代码本身只有"编译通过 + 复用既有两个外观页同一套行零件"作为证据。
- **`restore_defaults` 不清工作区覆盖**（有意）：用户在"恢复默认设置"之后可能仍看到被覆盖的值，
  此时唯一的解释就是那一行的来源标注，界面上没有额外提示。
- **键名在代码里有两处**（`Settings` 的 serde rename 与 `AppearanceKey::json_key()`）：
  这是"删键只能按字符串做"的代价，靠 `json_keys_come_from_the_overlay_schema` 兜住。
- **工作区层不防抖**：数字输入框连续改值会产生几次小文件写（实测可接受，输入 `19` 只在值合法时写一次）。

## 验证

- `cargo test --workspace`（gpui）：**407 通过 / 0 失败**（基线 393；新增 14 条全部在 `workspace` 模块，
  settings crate 144 → 158）。父代理独立复跑得到同一数字。
- `cargo check --workspace --all-targets`：**exit=0**（只剩两条既有 `dead_code` 警告）。
- `./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1`：通过。
- `node scripts/verify-agent-notes.mjs`：通过。
- `node rust/gpui/tools/extract-locale.mjs --check`：报"产物与真源一致"。
- **GUI 端到端**（工作区之外的 exe 副本 + 工作区之外的真 Git 仓库；全局 `fontSize=14`）：
  1. 共享层设 18 → `appearance_source key=fontSize source=project value=18` + `mono_font_size=18px`；
  2. 本机层设 20 → `source=local value=20`；把共享文件 `git add -f` 后重启 →
     `settings_tracked tracked=true` + `source=team value=18`（三态在真机上各自取到）；
  3. `--appearance-revert fontSize` → `appearance_reverted key=fontSize layers=shared+local
     value=14 global=14` + `mono_font_size=14px`（**回落到全局 14**，不是空值、无崩溃）；
  4. 未知键逐字保留（`thirdParty{keepMe,alsoKeep}` 与 `futureFlag` 原样还在），本机层清键后
     只剩 `{"version": 1}`，无 `.tmp` 残留；
  5. `git status --porcelain` **为空**；移除本机排除行后 `git status -uall` 只出现三个可共享文件，
     `git check-ignore -v .lithe/settings.local.json` 命中 `.lithe/.gitignore:1:*.local.json`。
- **真实 UI 写入路径**（`--menu-probe view lithe.menu.zoomIn`，即「视图 → 放大」）：
  工作区文件出现 `"fontSize": 19.0` 而**全局文件仍然是 14**——外观改动没有污染全局层。
- 清理：临时仓库 / 全局设置 / 日志已删、无残留 `Lithe` 进程、`alt-tmp` 已删。

## 适用范围

- `rust/gpui/crates/settings/src/workspace.rs`
- `rust/gpui/crates/settings/src/store.rs`
- `rust/gpui/crates/settings/src/schema.rs`
- `rust/gpui/crates/settings/src/dialog.rs`
- `rust/gpui/crates/settings/src/row.rs`
- `rust/gpui/crates/settings/src/persistence.rs`
- `rust/gpui/crates/shared/src/workspace_config/`
- `rust/gpui/crates/workbench/src/workspace.rs`
- `rust/gpui/crates/shared/locales/lithe.zh-CN.yml`
- `rust/gpui/tools/extract-locale.mjs`
- `rust/gpui/HANDOFF.md`
