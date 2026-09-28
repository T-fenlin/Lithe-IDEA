# GPUI Kit 界面文案资源（由 Windows 前端提取）

本目录是 `gpui/shell`（GPUI Kit 桌面外壳）的 rust-i18n 资源目录。文件内容**全部由脚本从
Windows 前端的文案真源提取生成**，不手工维护；改文案请改真源后重新生成，或直接改 GPUI 侧的新真源并把
`windows/` 前端整体删除（见文末「提取完成后是否需要保留真源」）。

## 1. 文件与真源

| 产物 | 内容 |
| --- | --- |
| `lithe.zh-CN.yml` | 4321 条 `lithe.*` 应用文案（真源 catalog）+ 13 条 gpui 侧自有 key + 2 条 `gpui_component.*` 覆盖（简体中文） |
| `lithe.en.yml` | 同上（英文） |
| 生成脚本 | `gpui/tools/extract-locale.mjs` |
| 真源 | `windows/tauri/src/i18n/locale.ts`（`catalogs` 对象，内含 `"en-US"` 与 `"zh-CN"` 两套） |
| 真源（展开） | `windows/tauri/src/i18n/ai-commit.ts`（`...aiCommitEnglish` / `...aiCommitChinese`，**不属于 locale.ts 但属于同一 catalog**） |

真源结构：`catalogs` 是**平面 key**（`"git.console.matches"` 这种点分字符串）的两语言对象，
值全部是字符串字面量，无复数/上下文变体、无函数、无动态拼接。两套语言的 key 集合一致
（`windows/tauri/src/i18n/locale.test.ts:99-104` 有断言），所以两个 YAML 的 key 集合也逐条相同。

`windows/tauri/src/i18n/` 下没有按命名空间拆分的其它 locale 文件：`locale-provider.tsx` 只是 React context，
`locale.test.ts` 只做断言。

## 2. 重新生成

```bash
node gpui/tools/extract-locale.mjs          # 覆盖写入两个 YAML
node gpui/tools/extract-locale.mjs --check  # 只校验产物是否与真源一致（不一致则退出码 1）
```

脚本的行为约定：

- 用**整段求值**（`new Function`）而不是正则解析对象字面量，prettier 折行、单双引号、`\uXXXX`/`\n` 转义都由 JS 引擎处理；
- key 顺序保持真源顺序，便于逐行 review diff；
- 输出 UTF-8、LF、**无 BOM**；
- 每跑一次都会校验两个 locale 的 key 集合一致，并提示占位符差异；
- 若真源出现非字符串值（函数调用、模板拼接等），该 key 会被**跳过**并打印清单，脚本以退出码 1 结束——宁可缺也不写错。

## 3. rust-i18n 4.2 的 YAML 约定（以本地源码为准）

本地 crate 源码：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`。

- **`_version` 决定文件形态**：`_version: 1` 是「一个 locale 一个文件、key 直接映射到译文」；
  `_version: 2` 是「key 在前、locale 在后」（`rust-i18n-4.2.2/README.md:96-101`、`:173-185`）。
  `get_version()` 缺省返回 1（`rust-i18n-support-4.2.2/src/lib.rs:239-245`），`_version: 2` 走 `parse_file_v2`
  （`.../src/lib.rs:178-190`），它把 namespace（可多层嵌套）拼成点分平面 key 后按 locale 建表
  （`.../src/lib.rs:198-236`）。
- **locale 键名**=`zh-CN` / `en`（写作 `yamlLocale`），与 `gpui_kit::component::set_locale("zh-CN")` 取值一致。
- **文件名**：locale 由 `file_stem().split('.').last()` 推导（`.../src/lib.rs:129-133`）。本项目选用
  `lithe.<locale>.yml` 这种「namespace.locale.yml」写法，与 rust-i18n 自带示例
  `rust-i18n-4.2.2/examples/app/locales/view.en.yml` 一致：在 `_version: 2` 下文件名不参与解析，
  但若将来退回 `_version: 1`，文件名仍然是正确的 locale 来源。
- **`gpui_component` namespace**：`gpui-component-0.6.6/locales/ui.yml:1-3` 就是 `_version: 2` + 顶层组件 namespace；
  应用通过 `rust_i18n::extend!(gpui_component)` 把本目录同名 namespace 挂到组件后端上，应用值优先
  （`rust-i18n-4.2.2/src/lib.rs:199-219`、`rust-i18n-macro-4.2.2/src/lib.rs:358-378`）。
- 应用侧接线（由主代理完成，不在本次提取范围）：crate root `rust_i18n::i18n!("locales", fallback = "en")`，
  在 `gpui_kit::init(cx)` **之前**调用 `rust_i18n::extend!(gpui_component)`。

## 4. key 布局与 namespace

```yaml
_version: 2
lithe:                       # ← 应用自己的顶层 namespace
  git.console.copyCommand:   # ← 原 key，前面统一加 lithe. 前缀
    zh-CN: "复制完整命令"
gpui_component:        # ← 只用于覆盖 gpui-kit 组件内置文案
  Dialog.ok:
    zh-CN: "确定"
```

- Lithe 页面文案统一放在 **`lithe:`** namespace 下，不混入 `gpui_component:`；YAML 里的 key 是 `lithe.<原 key>`。
- 应用代码查找时必须带前缀：`t!("lithe.git.console.copyCommand")`（`extend!` 只改变组件查找**自身** key 的方式，
  应用调用不会因此读到组件内置文案）。
- 之所以再加一层 `lithe:`（而不是把原 key 直接放顶层）：给应用文案一个稳定的归属，和 `gpui_component:`
  覆盖段在同一文件里不混淆，也方便以后按产品/模块再挂新 namespace。

## 5. 占位符

- **本项目的占位符语法是 `{name}`**（例：`{count}`、`{path}`、`{added}/{updated}/{deleted}`），
  由 Windows 侧 `createTranslator` 的 `translatedText.split("{name}").join(value)` 实现
  （`windows/tauri/src/i18n/locale.ts:8765-8777`）。提取时**逐字保留**，未做任何改写。
- ⚠️ 接线注意：`rust_i18n::t!` 自己的插值只识别 **`%{name}`**（`rust-i18n-4.2.2/src/lib.rs:45-91` 的
  `replace_patterns` 要求 `%` 前缀；宏在 `rust-i18n-macro-4.2.2/src/tr.rs:455` 调用它）。
  gpui-kit 组件内置文案用的也是 `%{}`（`gpui-component-0.6.6/locales/ui.yml:349`）。
  因此外壳需要一个薄封装（例如 `t(key, args)`：先 `rust_i18n::t!(...)` 再按 `{name}` 替换），
  否则 `{count}` 会原样显示。
- 英文有 7 条 key 额外带 `{plural}` 占位符，值由调用点按单复数传入（如
  `plural: count === 1 ? "" : "s"`，见 `windows/tauri/src/features/git/components/git-tag-manager.tsx:274`），
  中文不用该占位符：`git.resolveConflicts`、`git.tagCount`、`git.remoteCount`、
  `git.localCommitsNotPushed`、`git.remoteCommitsNotPulled`、`git.diffFileCount`、`git.changedFilesCount`。
  这 7 条在 `en` 文件里保留 `{plural}`，接线时英文需要继续传该参数（脚本每次运行会列出这处差异）。

## 6. `gpui_component:` 覆盖段

本目录只放了 2 条，都是「Lithe 文案与 gpui-component 内置文案同义、且中英文逐字一致」的对话框按钮：

| key | 取值来源 | 内置值（gpui-component 0.6.6） |
| --- | --- | --- |
| `gpui_component.Dialog.ok` | `ui.ok`（OK / 确定） | `ui.yml:225-230` = OK / 确定 |
| `gpui_component.Dialog.cancel` | `ui.cancel`（Cancel / 取消） | `ui.yml:231-236` = Cancel / 取消 |

当前取值与上游一致，等于显式声明「Lithe 对话框按钮用这两个词」并挡住上游改词。引用不存在的真源 key 时脚本会直接报错。

**看过但故意没有放进覆盖段的候选**（写在这里避免以后重复判断）：

- `ui.selectPlaceholder`（选择…）、`ui.searchPlaceholder`（搜索…）：语义上对应 `Select.placeholder` /
  `ComboBox.placeholder` / `List.search_placeholder` / `Settings.search_placeholder`，但英文文案不同
  （内置是 `Please select`），覆盖会**改变**组件现有效果，属于产品决策而不是提取，故不自动映射。
- `ui.previous` / `ui.next`（上一个 / 下一个）：英文与 `Pagination.previous` / `Pagination.next` 一致，
  但中文内置是「上一页 / 下一页」，覆盖会把分页文案改成「上一个 / 下一个」。
- `ui.previousSlide` / `ui.nextSlide` / `ui.carousel` / `ui.close`：同理（中文或大小写与
  `Carousel.*`、`Dock.Close` 内置值不完全一致），且 Lithe 的这几条是自家无障碍标签，不是组件文案。

Windows 目录里**没有**月份名/星期名/日期选择器文案（已全文检索 `month`/`week`/`calendar`/`datePicker`），
所以覆盖段里没有日历条目。

## 7. 未能机械提取的清单

**locale.ts 的 catalog：0 条未能提取。** 4321 条 key 全部是字符串字面量，已 100% 转换
（`extract-locale.mjs` 运行后无「跳过」输出；非字符串值会被跳过并报错，本次没有发生）。
以下是**不在 catalog 里、因此没有进入 YAML** 的界面文案/本地化逻辑，接线时需要单独处理：

1. `DISPLAY_LANGUAGES = ["en-US", "zh-CN"]`、`DisplayLanguage`、`getLocaleCatalog`
   （`windows/tauri/src/i18n/locale.ts:2-4`、`:8761-8763`）：语言清单与 catalog 访问器，是配置而非文案。
2. `createTranslator`（`:8765-8777`）：`catalog[key] ?? key` 的**缺 key 回退**，以及 `{name}` 插值循环。
   两者都是运行时逻辑，YAML 无法表达，必须在 GPUI 外壳里重新实现（回退行为见第 5 节）。
3. `windows/tauri/src/config/backend-capabilities.ts:1`：`BACKEND_UNAVAILABLE_TOOLTIP = "待开发"`，
   硬编码中文、无英文对照、不在 catalog 内。
4. `windows/tauri/src/features/settings/lib/settings-search.ts:19-20` 与
   `windows/tauri/src/features/settings/config/search-index.ts:11,19`：设置搜索用的中文关键词串
   （不是展示文案，但属于本地化内容）。
5. `windows/tauri/src/features/settings/components/ai-commit-settings-panel.tsx:462-464`：
   示例提交信息的硬编码中英文；同文件 `:431` 与
   `windows/tauri/src/features/settings/components/macos-settings-panels.tsx:188`：硬编码的「简体中文」选项名。
6. 其余含中文的 `.ts/.tsx` 命中均在 `*.test.ts(x)` 或注释里（例如
   `features/run/services/java-run-markers.ts:13` 只是指向 Agent Note 的注释），不是界面文案。

## 8. 提取完成后是否需要保留真源

本目录已经是 Windows 前端的**全量**文案快照（含 `ai-commit.ts` 展开），删掉 `windows/` 前端不会丢文案。
删除前建议做最后两件事：跑一次 `node gpui/tools/extract-locale.mjs --check` 确认产物与真源一致，
并处理上面第 7 节里 3~5 项未纳入 catalog 的零散文案。真源被删除后，本目录即成为新的文案真源，
届时可以直接手工维护 YAML（记得同步修改表头注释与本文档）。
