#!/usr/bin/env node
/**
 * extract-locale.mjs —— 把 Windows 前端的界面文案真源转换成 GPUI Kit 的 rust-i18n 资源。
 *
 * 真源：
 *   windows/tauri/src/i18n/locale.ts   （平面 key 的 `catalogs` 对象，内含 "en-US" 与 "zh-CN"）
 *   windows/tauri/src/i18n/ai-commit.ts（locale.ts 通过 `...aiCommitEnglish` / `...aiCommitChinese` 展开）
 *
 * 产物：
 *   gpui/crates/shared/locales/lithe.en.yml
 *   gpui/crates/shared/locales/lithe.zh-CN.yml
 *
 * 用法：
 *   node gpui/tools/extract-locale.mjs          # 重新生成两个 YAML
 *   node gpui/tools/extract-locale.mjs --check  # 只校验产物是否与真源一致（不写文件）
 *
 * 设计取舍：
 *   1. 不做正则解析，而是把三个对象字面量整段抽出来求值（`new Function`）。
 *      这样字符串转义、单双引号、跨行 prettier 折行都由 JS 引擎处理，不会解析错。
 *      如果将来有人在 catalog 里写函数调用/模板拼接，求值结果会出现非字符串值，
 *      本脚本会跳过该 key、打印清单并以退出码 1 结束（宁可缺也不写错）。
 *   2. key 顺序保持真源顺序，便于逐行 review diff。
 *   3. 文件使用 UTF-8、LF、无 BOM。
 *   4. 真源里没有、但 GPUI 侧确实需要的文案放 `GPUI_ONLY_KEYS`（见下），
 *      仍然由本脚本写进产物，所以 `--check` 依然能守住"产物可重现"。
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..");
const LOCALE_TS = resolve(REPO_ROOT, "windows/tauri/src/i18n/locale.ts");
const AI_COMMIT_TS = resolve(REPO_ROOT, "windows/tauri/src/i18n/ai-commit.ts");
// 持有 `locales/` 的 crate 是 `crates/shared`（`rust_i18n::i18n!` 必须在那个 crate 的根，
// 见 `gpui/crates/shared/src/i18n.rs`）；早期版本的脚本写的是 `gpui/shell/locales`，
// 那个目录在 workspace 重组后已经不存在，会把产物写到错的地方。
const OUT_DIR = resolve(REPO_ROOT, "gpui/crates/shared/locales");

/** rust-i18n 的 locale 名（YAML 里的语言 key）→ 真源 catalog 的键。 */
const CATALOG_LOCALES = [
  { yamlLocale: "zh-CN", catalogKey: "zh-CN", file: "lithe.zh-CN.yml" },
  { yamlLocale: "en", catalogKey: "en-US", file: "lithe.en.yml" },
];

/** 应用自己的顶层 namespace。页面文案一律挂在这里，不要塞进 gpui_component。 */
const APP_NAMESPACE = "lithe";

/** 官方文档与 gpui-component 内置 locales/ui.yml 使用的 locale 文件版本。 */
const FILE_VERSION = 2;

/**
 * 覆盖 gpui-kit（gpui-component）组件自身文案的 key。
 * 只有「Lithe 文案与组件内置文案同义、且中英文都逐字一致」的条目才允许放进来；
 * 值仍然取自真源，避免手写翻译漂移。
 * 见 gpui/shell/locales/README.md「gpui_component 覆盖段」。
 */
const GPUI_COMPONENT_OVERRIDES = [
  { target: "Dialog.ok", source: "ui.ok" },
  { target: "Dialog.cancel", source: "ui.cancel" },
];

/**
 * 真源里**没有**、但 GPUI 侧确实需要的文案（`settings.gpui.*` / `editor.gpui.*` /
 * `appearance.gpui.*` / `maven.gpui.*`）。
 *
 * 允许出现的只有两类，别的一律加进真源：
 *   1. Windows 把文案**硬编码**在 TSX 里、没进 catalog（下拉选项名之类）；
 *   2. Windows 的文案在 GPUI 侧**不成立**（例如"语言会立即生效"，而 gpui 只能重启后生效），
 *      或有意的设计偏离（按 `gpui/docs/gpui-kit/0.6.6/zh-CN/docs/design-guides.md` 的
 *      「界面用词」改写确认对话框），或真源把同一件事做成了**另一条路径**（命令面板的
 *      二级视图 vs 本侧的一等动作）。
 *
 * `reason` 只写给读脚本的人，不进产物；`zh` / `en` 两侧都要写，`--check` 会守住它们
 * 与两个 YAML 一致。
 */
const GPUI_ONLY_KEYS = [
  {
    key: "settings.gpui.languageEnglish",
    zh: "英语",
    en: "English",
    reason:
      "Windows 把语言下拉的选项名硬编码在 macos-settings-panels.tsx:187-188（English / 简体中文），不在 catalog；中文取死代码页签 tabs/general-settings.tsx 的「英语」。",
  },
  {
    key: "settings.gpui.languageChinese",
    zh: "简体中文",
    en: "Simplified Chinese",
    reason: "同上；Windows 的英文侧没有对应条目。",
  },
  {
    key: "settings.gpui.languageRestartDescription",
    zh: "切换语言会立即重启 Lithe。",
    en: "Switching the language restarts Lithe immediately.",
    reason:
      "Windows 的 settings.mac.languageDescription 写的是「界面语言会立即生效」，而 gpui 侧 set_locale 只在启动早期调用一次（crates/app/src/main.rs），运行中换语言会一半新一半旧。本侧的做法是改完语言立刻用相同参数重启自己（settings/src/restart.rs），所以描述如实写「会重启」。",
  },
  {
    key: "settings.gpui.restoreDefaultsOpen",
    zh: "恢复默认设置…",
    en: "Restore default settings…",
    reason:
      "Windows 的 settings.mac.restoreDefaults 是「恢复默认设置」且不带省略号；本命令会先打开确认对话框，按 design-guides.md:444 补单个省略号。",
  },
  {
    key: "settings.gpui.restoreDefaultsTitle",
    zh: "恢复默认设置？",
    en: "Restore default settings?",
    reason:
      "Windows 的 settings.mac.restoreDefaultsConfirm（「⚠️确认恢复所有配置吗？」）是 design-guides.md:434 点名的反例；按指南改成「标题写决策 + 正文写后果 + 按钮用结果词」。",
  },
  {
    key: "settings.gpui.restoreDefaultsBody",
    zh: "所有设置都会回到默认值。",
    en: "Every setting returns to its default value.",
    reason: "同上：正文只补充作用范围与后果，不重复标题的提问。",
  },
  {
    key: "settings.gpui.editorFontSizeDescription",
    zh: "调整代码编辑器的字号。界面字号在外观页，终端字号在终端页。",
    en: "Adjusts the code editor text size. Interface text size lives in Appearance; terminal text size lives in Terminal.",
    reason:
      "真源的「编辑器 → 字体大小」这一行没有描述（macos-settings-panels.tsx:283-292）。本侧补一句说明作用范围，避免与另外两个字号键混淆：外观页的「界面字体大小」（uiFontSize，rem 基准）与终端页的「字体大小」（terminalFontSize，终端视图自己的覆盖值）。这三者互不影响。",
  },
  {
    key: "settings.gpui.valueNotOverridden",
    zh: "默认（不覆盖）",
    en: "Default (no override)",
    reason:
      "字体族与终端字号两个键用「空值 = 不覆盖」的语义（两个字族是空串、终端字号是 0），这样主题文件里指定的字体族不会被用户的空设置抹掉。真源的字体族设置永远有一个具体值、终端字号也永远有具体值，没有对应的「不覆盖」选项，所以要自己的键。",
  },
  {
    key: "settings.gpui.pageNotAvailableTitle",
    zh: "此分类尚未接入",
    en: "This category is not available yet",
    reason:
      "设置左栏保留 Windows 的 10 个分类（去掉 AI 两个），其中 7 个在 gpui 侧还没有子系统。HANDOFF §4 的口径是「没有数据源的做成明确空态并写清前置条件，不塞假控件」，所以这些页只有标题 + 前置条件一句，真源里没有这种空态文案，需要自己的键。",
  },
  {
    key: "settings.gpui.projectScopeGlobal",
    zh: "没有打开项目：覆盖值保存在当前电脑的全局设置文件里，作为本机默认；留空则使用自动检测到的值。打开项目后，这一页改的是那个项目自己的 .lithe/ 本机层，它的优先级更高。",
    en: "No project is open: overrides are stored in this computer's global settings file as machine-wide defaults, and an empty field falls back to the detected value. With a project open, this page edits that project's own .lithe/ local layer instead, which takes precedence.",
    reason:
      "「项目 · JDK 与 Maven」页的作用域说明（**没有工作区**的那一支）。真源的 settings.project.scope 写的是「仅保存在当前电脑，作用于当前项目」，本侧拆成两支：有工作区时写项目本机层（见下一条），没有工作区时写全局设置文件充当本机默认。这一句必须如实说明「现在写的是哪一层」，否则用户会以为改的是项目、其实改的是全局。",
  },
  {
    key: "settings.gpui.projectScopeProject",
    zh: "覆盖值保存在当前项目的 .lithe/ 本机层（run.local.json 与 maven.local.json，不进版本控制）；本机层留空的值回落到全局设置，两层都空就用自动检测。",
    en: "Overrides are stored in this project's .lithe/ local layer (run.local.json and maven.local.json, never committed); an empty field falls back to the global settings, and if both are empty the detected value is used.",
    reason:
      "同一句说明的**有工作区**那一支。设计真源把优先级定为「项目本机 > 全局默认 > 自动发现」（.agents/notes/proposed/architecture/2026-09-26-workspace-configuration-layers.md 第四节），这里把它讲给用户听：写哪里、留空会怎样、两层都空会怎样。",
  },
  {
    key: "settings.gpui.overrideFromProject",
    zh: "覆盖值来自本项目",
    en: "override from this project",
    reason:
      "五个覆盖值现在有两层来源，而输入框只显示生效值。不标来源的话，「我明明在全局设了 JDK、为什么这个项目用的是另一个」无从判断。这一条标「来自项目本机层」。",
  },
  {
    key: "settings.gpui.overrideFromGlobal",
    zh: "覆盖值来自全局设置",
    en: "override from global settings",
    reason:
      "同上的另一半：这一格的值来自全局设置文件（本机默认）。来源为「自动发现」时不画任何标注，所以不需要第三条键。",
  },
  {
    key: "settings.gpui.sourceFromEnv",
    zh: "来自 {name} 环境变量",
    en: "from the {name} environment variable",
    reason:
      "工具链「来源」那一栏里两种**环境变量**来源的措辞（JDK 的 LITHE_JDTLS_JAVA、Maven 的 MAVEN_HOME）。真源只有 toolchain.source.javaHome / .path / .project / .detected / .mavenWrapper 五条（locale.ts），没有\"某个环境变量\"这一档，JDK 的显式覆写入口（java/src/jdtls.rs:40 的 LITHE_JDTLS_JAVA）与 MAVEN_HOME 都需要它。{name} 由调用点用 tr_args 填变量名。",
  },
  {
    key: "settings.gpui.mavenExecutableHint",
    zh: "可填 Maven 主目录或 mvn 启动器；留空则依次查找 MAVEN_HOME、M2_HOME 与 PATH 上的 Maven。",
    en: "A Maven home or mvn launcher; leave empty to search MAVEN_HOME, M2_HOME, then PATH.",
    reason:
      "「Maven 主目录 / 可执行文件」这一行的提示。真源的 run.mavenExecutableHint 是「可选择 Maven 主目录；留空则使用项目 Wrapper 或系统 Maven。」—— gpui 侧**没有**项目 Wrapper 那一级（拿不到工作区根，见 project.rs 的模块文档），照抄会承诺一个不存在的能力；照 discovery 的三级顺序如实改写。",
  },
  {
    key: "settings.gpui.mavenSettingsMissing",
    zh: "未检测到 settings.xml（已查用户目录的 .m2/settings.xml 与 Maven 安装目录的 conf/settings.xml）",
    en: "No settings.xml detected (checked .m2/settings.xml in the user directory and conf/settings.xml in the Maven installation)",
    reason:
      "「settings.xml」这一行拿不到值时的说明（真源那一行永远有值，因为值来自 Maven 工具窗的项目本地配置）。两个被查过的位置写进文案里，用户才能自己排查，而不是看到一个空的\"未知\"。",
  },
  {
    key: "settings.gpui.mavenLocalRepositoryDefault",
    zh: "Maven 默认位置（settings.xml 未指定 localRepository）",
    en: "Maven's default location (settings.xml declares no localRepository)",
    reason:
      "「本地仓库」的来源说明之一。真源那一行的值来自 Maven 工具窗的本地配置（maven.store.ts 的 localRepositoryPath），gpui 侧只能报事实：用户级 settings.xml 没写 <localRepository> 时 Maven 用的就是它自己的默认位置 ~/.m2/repository —— 把\"这是默认值而不是从配置里读来的\"写清楚，避免看起来像探测结果。",
  },
  {
    key: "settings.gpui.mavenLocalRepositoryFromSettings",
    zh: "来自用户 settings.xml 的 <localRepository>",
    en: "from <localRepository> in the user settings.xml",
    reason:
      "「本地仓库」的来源说明之二：值是从用户级 settings.xml 的 <localRepository> 元素里读出来的（Maven 文档规定该元素只在用户级 settings 生效）。真源没有这条措辞，因为它的值来自应用自己保存的配置。",
  },
  {
    key: "settings.gpui.mavenLocalRepositoryUnknown",
    zh: "未知（拿不到用户主目录，推不出 Maven 默认的本地仓库位置）",
    en: "Unknown (the user home directory is unavailable, so Maven's default local repository cannot be derived)",
    reason:
      "「本地仓库」既没有 settings.xml 的声明、又连用户主目录（USERPROFILE / HOME）都拿不到时的显示值。任务要求拿不到的字段显示\"未知\"并写清原因，而不是猜一个 ~/.m2/repository。",
  },
  {
    key: "settings.gpui.projectNothingDetected",
    zh: "既没有探测到 JDK，也没有探测到 Maven。下面每一行都写明了查过哪些位置、各自为什么不行。",
    en: "Neither a JDK nor Maven was detected. Each row below states which locations were checked and why none qualified.",
    reason:
      "这一页唯一一句整页级提示（仅在 JDK 与 Maven 都没有可用生效值时出现）。空态口径要求：不许再用「此分类尚未接入」那句（页面已经接入了），也不能静默 —— 所以补一句如实说明，并指向每一行各自的原因。",
  },
  {
    key: "settings.gpui.jdkBelowLanguageServiceMinimum",
    zh: "低于语言服务最低要求（JDT LS 需要 JDK {minimum} 或更新版本），语言服务无法启动。",
    en: "Below the language service minimum (JDT LS requires JDK {minimum} or newer); the language service cannot start.",
    reason:
      "「项目 · JDK 与 Maven」页 JDK 那一行在**版本低于 JDT LS 门槛**时的补充说明。闸门数值的唯一真源是 gpui/crates/java/src/jdtls.rs:80 的 MINIMUM_JAVA_MAJOR（= third_party/jdtls/manifest.json 的 minimumJavaVersion = 21），判据落点是 jdtls.rs:820-837 的 probe_java（低于闸门即拒，整条链路失败时 jdtls.rs:713-715 说的是「未找到 Java 21 或更新版本的 JDK，JDT LS 无法启动」）。真源 Windows 那页只显示「自动 → JDK 1.8.0_221」而**不提**这一层，用户因此无法预判语言服务能不能起来 —— 本批（A1）要修的正是这条「显示 ≠ 实际」。{minimum} 由调用点用 tr_args 填 21。",
  },
  {
    key: "settings.gpui.mavenOverrideNotEffective",
    zh: "尚未生效：gpui 侧还没有执行 Maven 的通路（不执行 mvn，也没采购 Core 的启动计划），这个值现在不改变任何行为。",
    en: "Not effective yet: gpui has no path that runs Maven (it never executes mvn and does not consume Core's launch plan), so this value changes nothing today.",
    reason:
      "「Maven 主目录 / 可执行文件」与「Maven JDK 主目录」两行的如实标注（A2）。已核实：gpui 全仓没有任何地方执行 mvn（maven.scan 是 Core 进程内的项目描述符解析，右侧 Maven 工具窗只呈现结论），runConfig.createLaunchPlan 在 gpui 侧没有消费方；语言服务那一侧也**故意不登记**这两个键（java/src/toolchain.rs 模块文档：登记了没人读只是把「存了不生效」搬到另一层）。真源 Windows 那页有 Maven 工具窗与运行配置编辑器消费这两个值，所以没有这类措辞。口径是「不藏起来、但必须标出来」——用户需要看到自己填的值，也需要知道它现在不起作用。",
  },
  {
    key: "settings.gpui.autoCompletionDescription",
    zh: "输入时自动显示补全建议（Java 语言服务优先，语言服务不可用时用当前文件的标识符兜底）。关掉之后不再自动弹出补全菜单，兜底候选也一并停止。",
    en: "Shows completion suggestions automatically while typing (the Java language service first, falling back to identifiers in the current file when it is unavailable). Turning this off stops the menu from popping up automatically, including the fallback suggestions.",
    reason:
      "「LSP」页「自动补全」那一行的描述。真源的 settings.mac.autoCompletionDescription 是「显示活动语言服务器提供的补全建议。」（macos-settings-panels.tsx:406-415）—— 它只提「活动语言服务器」，而 gpui 侧关掉这个开关时**连 Core 的轻量兜底（lsp.builtinCompletions）也一起不再自动弹出**（兜底是同一个 provider 的第二个数据源，见 editor/src/completion.rs 的模块文档）。照抄会让用户以为「关掉之后兜底还在」，所以如实写明两件事：优先/兜底这条降级关系，以及关掉之后两者一起停。",
  },
  {
    key: "settings.gpui.lspOnlyAutoCompletion",
    zh: "本页只做「自动补全」：真源另两项（参数提示、语义高亮）在 gpui 侧没有可挂载的接口（上游没有签名帮助 provider，语义高亮也没有 Java 侧实现），JDTLS / JDK 运行时路径则归「项目 · JDK 与 Maven」页，所以这里不画控件。",
    en: "This page only wires Auto Completion: the other two settings in the Windows page (parameter hints and semantic highlighting) have no attachable interface in gpui (the upstream editor has no signature-help provider, and semantic highlighting has no Java implementation), while the JDTLS / JDK runtime path belongs to the Project · JDK and Maven page, so no control is drawn here.",
    reason:
      "「LSP」页里那句「为什么只有一项」的说明。真源那页有三个开关，本侧只有 autoCompletion 有消费方（参数提示：上游 gpui-base-0.6.6 的 Lsp 结构体没有签名帮助接口，全 crate SignatureHelp 零命中；语义高亮：上游 trait 在但 Java 侧零实现、零装载点）；JDTLS 运行时路径输入框真源里没有，且 Windows 已主动退役同类键（settings-normalization.ts:550 + 守卫测试:26-33），JDK 路径已有「项目 · JDK 与 Maven」页这个真生效的入口。按「不画假控件」的口径，缺的那几项要有一句如实说明，否则用户会以为设置丢了。",
  },
  {
    key: "settings.gpui.noLanguageServer",
    zh: "还没有可用的语言服务运行时（本机没有探测到满足要求的 JDK；打开 Java 项目时也会再检测一次）。",
    en: "No usable language service runtime yet (no JDK meeting the requirement was detected on this machine; detection runs again when a Java project is opened).",
    reason:
      "「LSP」页「已检测语言服务器」那一组的空态。真源那里是一句静态文本，没有空态；本侧把它做成真事实（值来自「项目」页同一次真实探测），所以也需要一句「现在为什么是空的」。数值门槛由 java/src/jdtls.rs:80 的 MINIMUM_JAVA_MAJOR 决定，缺失时那一行会改画 jdkBelowLanguageServiceMinimum。",
  },
  {
    key: "settings.gpui.runNoProject",
    zh: "打开项目后才能识别可运行配置。",
    en: "Open a project to identify runnable configurations.",
    reason:
      "「运行配置」页拿不到工作区根时的整页提示（宿主没登记钩子）。真源同位的 settings.project.openProject 说的是「打开项目后，可在这里配置项目 JDK 和 Maven。」，与本页无关，照抄会指错地方，所以新增一条。",
  },
  {
    key: "settings.gpui.runReadOnlyDescription",
    zh: "本页列出 Core 从项目文件识别到的可运行目标：名称、类型、命令行、工作目录与生效来源。识别结果不写入任何文件；本页只读，没有编辑配置与启动的入口。",
    en: "This page lists the runnable targets Core identified from the project files: name, type, command line, working directory, and effective source. The result is written nowhere; this page is read-only and has no configuration editor or launch entry point.",
    reason:
      "页面第一句。真源的 settings.run.description 是「选择服务或任务，配置启动参数、环境变量和项目环境覆盖项；点击保存后生效。」（run-configuration-settings.tsx:81）—— 本页**只读**（没有配置编辑器、没有项目级存储、没有运行子系统），照抄会承诺三个不存在的控件，正是 HANDOFF §4 与 07-settings-ui.md §7.3-D 禁止的「说了做不到」。所以按本页真实能力改写，并在句子里点明「识别结果不写入任何文件」（runConfig.generate 的契约行为）。",
  },
  {
    key: "settings.gpui.runNothingDetected",
    zh: "没有在项目文件里识别到可运行配置。Core 已按 Maven（Spring Boot / Quarkus / Micronaut）、Gradle、npm、Compose、Procfile、Python、Cargo、Go、Make 与 just 逐类探测过；Java 主类由语言服务提供，gpui 侧还没接那一路，所以这里只会出现 Maven 模块与脚本类目标。",
    en: "No runnable configuration was found in the project files. Core checked Maven (Spring Boot / Quarkus / Micronaut), Gradle, npm, Compose, Procfile, Python, Cargo, Go, Make, and just; Java main classes come from the language service, which gpui does not wire yet, so only Maven modules and script-style targets can appear here.",
    reason:
      "空态（runConfig.generate 返回 0 条）。真源的 run.noRunnableActionsFound（「未找到可运行操作」）属于「扫描项目操作」那一族（同族还有 run.scanningProjectActions / run.filterActions / run.tryAnotherActionSearch），落到本页会被读成「筛选结果为空」；而本页要说的是「Core 走完了哪些探测器」与「哪一路还没接」，真源里没有这句话，所以新增，并按探测器全集（rust/lithe-core/src/execution/detectors/mod.rs:227-237）如实写。",
  },
  {
    key: "settings.gpui.runLoadFailed",
    zh: "识别运行配置失败：{reason}",
    en: "Failed to identify run configurations: {reason}",
    reason:
      "失败态。真源的 settings.project.reloadFailed（「无法重新加载项目环境。」）说的是项目环境，而且不带原因；任务要求失败时显示可排查的原因，所以新增一条带 {reason} 的键 —— {reason} 由调用点填 Core 的错误码原文（例如 workspace_not_found: …）。",
  },
  {
    key: "settings.gpui.runNoCommand",
    zh: "没有固定命令行（可执行文件由工具链提供，真正的命令行由启动计划组装）",
    en: "No fixed command line (the toolchain provides the executable; the launch plan assembles the real command)",
    reason:
      "工具链接管的条目 Core 会把 command 置空（rust/lithe-core/src/execution/detectors/mod.rs:156-171 的 with_toolchains），这是语义而不是缺字段 —— Maven 的 spring-boot:run 由 maven.launchPlan 组装。真源在编辑器里把这类条目的 run.command 留空即可（用户知道自己在配什么），本页是只读列表，必须说清「为什么这里没有命令」，否则看起来像数据缺失。",
  },
  {
    key: "settings.gpui.runNotWired",
    zh: "点击启动尚未接入：gpui 侧还没有运行面板与进程宿主，「运行」工具窗与「运行」菜单目前都是占位。Core 的 runConfig.createLaunchPlan（平台无关的启动计划）已经能给出计划，但还没有消费方。",
    en: "Starting a target is not wired yet: gpui has no run panel and no process host, and the Run tool window and Run menu are placeholders. Core's runConfig.createLaunchPlan already produces a platform-neutral plan, but nothing consumes it yet.",
    reason:
      "「运行配置」页的收尾说明（任何状态下都画），只讲「执行缺什么」。真源那一页的行是**可点按钮**（点进去是 RunConfigurationEditor，run-configuration-settings.tsx:100-111），本侧没有配置编辑器也没有运行子系统，所以必须如实写出「列表是真的、执行没接」，否则用户会以为行能点、或以为绿三角在别处。这是 HANDOFF §4 与 07-settings-ui.md §7.3-D「不画假控件」的口径；「本页只读」由 settings.gpui.runReadOnlyDescription 说，两处不重复。",
  },
  {
    key: "settings.gpui.prerequisiteKeyboard",
    zh: "前置条件：键位表与快捷键预设（现在只有固定绑定的少数快捷键）。",
    en: "Requires a keybinding table and presets; today only a few shortcuts are bound with fixed keys.",
    reason:
      "「快捷键」页的前置条件（同 pageNotAvailableTitle）。真源那一页的预设下拉要整套 keymap 基础设施（features/keymaps/**），gpui 侧只有少量 KeyBinding::new 的固定绑定。",
  },
  {
    key: "settings.gpui.prerequisiteGit",
    zh: "前置条件：Git 身份（user.name / user.email）的读写接线；Core 的 git.repositorySetup 与 git.configureIdentity 已经就绪。",
    en: "Requires wiring Git identity (user.name / user.email); Core already provides git.repositorySetup and git.configureIdentity.",
    reason:
      "「Git」页的前置条件（同 pageNotAvailableTitle）。真源那页 11 项里大部分作用于 gpui 侧还没有的 Git 视图开关，另有「提交身份」子面板要读写 git config —— 后者缺的只是接线，所以文案里点明 Core 已经就绪。",
  },
  {
    key: "settings.gpui.prerequisiteLogs",
    zh: "前置条件：日志系统（日志目录、诊断开关与诊断包导出）。",
    en: "Requires the logging subsystem (log directory, diagnostic toggle, and diagnostic bundle export).",
    reason: "「日志」页的前置条件（同 pageNotAvailableTitle）；gpui 侧没有日志系统与 log-api 等价物。",
  },
  {
    key: "settings.gpui.prerequisiteUpdates",
    zh: "前置条件：更新器（检查更新、更新通道与安装流程）。",
    en: "Requires an updater (update check, channels, and installation flow).",
    reason: "「更新」页的前置条件（同 pageNotAvailableTitle）；gpui 侧没有更新器。",
  },
  {
    key: "settings.gpui.pickPath",
    zh: "选择…",
    en: "Choose…",
    reason:
      "「项目 · JDK 与 Maven」页每一行路径右侧的按钮（打开系统目录/文件对话框）。真源那颗按钮是**纯图标**（project-environment-settings.tsx:243-260 的 Button size=\"icon-sm\" + aria-label={t(\"ui.browse\")}），而 design-guides.md 的无障碍清单要求「图标按钮必须有可访问名」—— gpui 的 Button 没有可访问名接口，所以只能画带字的按钮；又按 design-guides.md 的省略号规则（打开对话框的命令补单个省略号）写成「选择…」。真源既有的 lithe.ui.browse（「浏览」）不带省略号，且那份文案由本脚本逐字同步、不能改。对象名由同一行的字段标签给出，按钮不重复它。",
  },
  {
    key: "settings.gpui.mavenSettingsHint",
    zh: "留空则用检测到的那一份：用户目录的 .m2/settings.xml 优先，其次 Maven 安装目录的 conf/settings.xml。",
    en: "Leave empty to use the detected file: .m2/settings.xml in your home directory first, then conf/settings.xml in the Maven installation.",
    reason:
      "「项目 · JDK 与 Maven」页 Maven 配置那一行（settings.xml）的提示。真源那一行只有 placeholder t(\"maven.automatic\")（「自动检测」），没有说明它到底会挑哪一份；gpui 侧的探测真的按这两级找（project.rs 的 maven_configuration），所以如实写出来。",
  },
  {
    key: "settings.gpui.mavenSettingsEffective",
    zh: "生效：下一次语言服务启动时作为 Maven 用户设置交给 Java 语言服务。",
    en: "Applies: takes effect on the next language-service start, as the Maven user settings for the Java language service.",
    reason:
      "同上那一行的生效说明。事实依据：这个值经 JavaToolchainOverride → mavenContext.settingsPath → Core 发布成 java.configuration.maven.userSettings（contracts/rust-core-api.md:1170-1178），时机与 javaHomePath 相同（下一次语言服务启动，今天是重启应用）。真源没有这条 —— 它写进项目级文档、由运行侧的 Maven 工具窗消费。",
  },
  {
    key: "settings.gpui.mavenLocalRepositoryHint",
    zh: "留空则用生效 settings.xml 里的 <localRepository>；没写就是 Maven 默认位置 ~/.m2/repository。",
    en: "Leave empty to use <localRepository> from the effective settings.xml; when absent, Maven's default ~/.m2/repository.",
    reason:
      "「项目 · JDK 与 Maven」页本地仓库那一行的提示（判据同 mavenSettingsHint）。真源同一行只有 placeholder t(\"maven.automatic\")。",
  },
  {
    key: "settings.gpui.overridePathMissing",
    zh: "这个路径不存在；留空会回到自动检测。",
    en: "This path does not exist; clearing the field returns to automatic detection.",
    reason:
      "同上两行的失败态：用户选的 settings.xml / 本地仓库不是一个存在的文件时标出来（判据是 project.rs 的 override_settings_missing）。真源没有这条 —— 它的输入框不做存在性检查；本侧这两行会被交给语言服务，指错了必须当场说清，不能看起来像已生效。",
  },
  {
    key: "spring.title",
    zh: "Spring",
    en: "Spring",
    reason:
      "右侧工具窗「Spring」视图的标题。真源 Windows 前端没有独立的 Spring 面板（Spring 能力留在语言服务与运行配置里），catalog 里因此没有这个键；命名与 lithe.maven.title / lithe.workbench.maven 同一口径（产品名，中英同形）。",
  },
  {
    key: "spring.notDetected",
    zh: "未检测到 Spring 组件与端点",
    en: "No Spring components or endpoints detected",
    reason:
      "同一个视图的空态。判据是 Core 的 spring.index 没给出任何事实（响应为 null 或集合全空），所以文案如实说「组件与端点」而不是「项目」——免得把「打开了一个没有 Spring 的 Java 项目」说成「没检测到项目」。句式沿用 maven.notDetected（「未检测到 X」），保持同一面板族的读感。",
  },
  {
    key: "editor.gpui.discardChanges",
    zh: "放弃修改",
    en: "Discard changes",
    reason:
      "关闭未保存 buffer 的确认对话框（阶段 9 的 E）。真源的字是「不保存」（unsavedChanges.doNotSave，windows/tauri/src/i18n/locale.ts:7873），但 gpui/docs/gpui-kit/0.6.6/zh-CN/docs/design-guides.md:421 对「未保存修改」这一档明确推荐「放弃修改」（『不保存』看不出放弃了什么），按指南改写；这与 settings.gpui.restoreDefaults* 同一类有意偏离。",
  },
  {
    key: "editor.gpui.unsavedChangesBody",
    zh: "对“{name}”的修改尚未保存。",
    en: 'Your changes to "{name}" are not saved yet.',
    reason:
      "同一个对话框的正文。真源只有拆成三段拼接的 unsavedChanges.messagePrefix/messageSuffix（「是否要保存对」+ 文件名 +「 所做的更改？」，locale.ts:7874-7875），是 design-guides.md:427-434 点名的『您确定要……吗』式提问；按指南改成『正文只补充作用范围与后果』，文件名仍是唯一新增信息。",
  },
  {
    key: "editor.gpui.unsavedChangesBatchBody",
    zh: "有 {count} 个文件的修改尚未保存。",
    en: "{count} files have unsaved changes.",
    reason:
      "标签右键菜单的三个批量关闭（关闭其他 / 关闭右侧 / 全部关闭）的确认正文（阶段 11）。真源是「逐个发现脏标签、只弹一次确认」，但弹窗正文只取 pendingClose.bufferId 那**一个**文件名（pending-buffer-close-dialog.tsx:9-12 + buffer.store.ts:1746 的 find），用户看到「是否要保存对 A 所做的更改？」却会连带关掉 B、C —— 读起来像 bug。维护者口径：正文改为如实显示**有几个**文件未保存，所以本侧需要一条自己的键（真源没有「N 个文件」这种句式）。",
  },
  {
    key: "appearance.gpui.switchThemeLight",
    zh: "首选项：切换到浅色主题",
    en: "Preferences: Switch to Light Theme",
    reason:
      "阶段 6 第二半（命令面板）。真源把「切换配色主题」做成 command-palette 的二级视图 `color-theme`（components/theme-selector.tsx，入口 `commandPalette.actions.color-theme.label` = 「首选项：颜色主题」，locale.ts:8041 / en :3724），本侧命令面板还没有二级视图，先做成一等的切换动作，所以要有自己的标签。前缀沿用真源同一族（`Preferences: Open … Settings`，settings-actions.tsx:129）；动作名取真机主题名既有译名「浅色 / 深色」（locale.ts:6250-6251）。",
  },
  {
    key: "appearance.gpui.switchThemeLightDescription",
    zh: "使用 Lithe Light 配色",
    en: "Use the Lithe Light color theme",
    reason:
      "同上：命令面板每行可以带一句描述（`ui/command.tsx:528-537`）。真源只有二级视图，没有这一句，本侧按主题名如实写。",
  },
  {
    key: "appearance.gpui.switchThemeDark",
    zh: "首选项：切换到深色主题",
    en: "Preferences: Switch to Dark Theme",
    reason: "同上；两条互为反向。",
  },
  {
    key: "appearance.gpui.switchThemeDarkDescription",
    zh: "使用 Lithe Dark 配色",
    en: "Use the Lithe Dark color theme",
    reason: "同上。",
  },
  {
    key: "maven.gpui.toggleToolWindowShow",
    zh: "视图：显示 Maven",
    en: "View: Show Maven",
    reason:
      "阶段 6 第二半（命令面板）。真源左活动栏底部组的 maven 项点下去走 `toggleMavenPane` → `applyRightToolWindowIntent`（features/maven/actions/maven-tool-window-actions.ts:58），是真机「开/关 Maven 工具窗」的入口；但真源没有一个进 catalog 的命令行标签（左活动栏项只有 aria-label `run.title - maven.title`，sidebar-pane-selector.tsx:243,247）。本侧按真源 View 组既有的措辞仿写（`commandPalette.actions.toggle-terminal.enableLabel` = 「视图：显示终端」，locale.ts:8104 / en :3801），所以是 gpui 侧新增键。",
  },
  {
    key: "maven.gpui.toggleToolWindowHide",
    zh: "视图：隐藏 Maven",
    en: "View: Hide Maven",
    reason: "同上；两条互为反向。",
  },
  {
    key: "maven.gpui.toggleToolWindowDescription",
    zh: "开关右侧的 Maven 工具窗",
    en: "Toggle the Maven tool window on the right",
    reason:
      "同上。真机 Maven 工具窗在右侧栏（features/layout/components/main-layout.tsx:336-340），所以描述里点明「右侧」。",
  },
  {
    key: "appearance.gpui.toggleStatusBarShow",
    zh: "视图：显示状态栏",
    en: "View: Show Status Bar",
    reason:
      "阶段 6 第二半（命令面板）。真源只有设置项 `settings.appearance.showStatusBar` = 「显示状态栏」（locale.ts:6644 / en :2276），命令面板在这一项上没有动作条目；本侧按 View 组既有措辞补一条切换动作，所以是 gpui 侧新增键。",
  },
  {
    key: "appearance.gpui.toggleStatusBarHide",
    zh: "视图：隐藏状态栏",
    en: "View: Hide Status Bar",
    reason: "同上；两条互为反向。",
  },
  {
    key: "gpui.newWindowNotWired",
    zh: "尚未接入：在「新窗口」中打开项目需要多窗口支持（缺窗口级句柄路由）。",
    en: "Not wired yet: opening a project in a new window needs multi-window support (window-level handle routing is missing).",
    reason:
      "B4（打开其他文件夹）的换项目对话框里「新窗口」那一颗按钮的提示。真源的 projectOpen.newWindow 是**真的会开第二个窗口**的按钮（windows/tauri 的 createAppWindow），而 gpui 侧维护者已拍板「多窗口暂不做」（gpui/research/menu-and-open-project-plan.md §5），菜单栏 / 项目下拉 / 命令面板 / Git 身份宿主这几个 thread_local 单例仍是「一个进程一份」—— 点了它只能给一句「尚未接入：缺 X」。真源没有「能力尚未接入」这类文案（它的按钮恒可执行），而按 Q2 / Q14 的口径这句话必须说清缺什么，所以是 gpui 侧新增键。同一批其余的能力提示词归 B3，这里只补这一个。",
  },
  {
    key: "gpui.menuMissing.editorEntry",
    zh: "尚未接入：缺编辑器侧的公开入口，本批未授权改动 editor crate。",
    en: "Not wired yet: the editor crate exposes no public entry point for this, and changing that crate is out of scope for this batch.",
    reason:
      "B3 的占位项文案（规格 §B3 / Q14 粒度 2d：一种能力一句话）。这一句给「另存为 / 全部保存 / 还原文件 / 后退 / 前进 / 下一个·上一个标签页 / 切换自动换行」这 8 条：能力在 editor crate 里是**私有**的（EditorPane::request_reload / go_back / go_forward / activate、EditorState::set_soft_wrap 都够不到），而规格 §5 第 5 条明确本批不授权改那个 crate。真源的这 8 条恒可执行（catalog 里没有对应提示句），所以是 gpui 侧新增键。",
  },
  {
    key: "gpui.menuMissing.editorFeatures",
    zh: "尚未接入：编辑器还没有这些功能（行级编辑、转到行、缩略图 / 行号 / 空白字符开关、分屏）。",
    en: "Not wired yet: the editor has none of these features (line editing, go to line, minimap / line numbers / whitespace toggles, split view).",
    reason:
      "B3（同上）。给「切换注释 / 复制行 / 删除行 / 上移行 / 下移行 / 转到行 / 拆分编辑器 / 切换缩略图 / 切换行号 / 切换空白字符显示」。与 editorEntry 的差别是**能力本身不存在**（整个仓库里 toggle_comment / duplicate_line / move_line / go_to_line / minimap / 分屏 零命中），不是「私有方法挡住了」。真源对应 10 条菜单项，catalog 里没有任何描述这些实现缺失的键。",
  },
  {
    key: "gpui.menuMissing.lspRequests",
    zh: "尚未接入：Java 服务侧还没有这条 LSP 请求（现在只接了定义 / 补全 / 快速修复）。",
    en: "Not wired yet: the Java service does not expose this LSP request (only definition, completion and code actions are wired).",
    reason:
      "B3（同上）。给「转到实现 / 转到类型定义 / 转到引用 / 重命名符号 / 显示悬停信息 / 触发参数提示 / 格式化文档 / 格式化所选内容」。事实依据：crates/java/src/service.rs 只暴露 definition / completion / code_actions / diagnostics / sync_document 五条请求，textDocument/implementation|typeDefinition|references|rename|hover|signatureHelp|formatting 一条都没有 —— 缺的是上一条链的请求，不是编辑器入口。真源这 8 条恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.indexer",
    zh: "尚未接入：还没有索引器（跨文件搜索与快速打开都依赖它）。",
    en: "Not wired yet: there is no indexer (cross-file search and quick open both need it).",
    reason:
      "B3（同上）。给「视图 → 全局搜索」与「转到 → 快速打开」。维护者口径（规格 §B3 的注）点名这两条写「没有索引器」：真源这两项都走跨文件索引，gpui 侧还没有索引层。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.debuggerHost",
    zh: "尚未接入：没有进程宿主（调试适配器还没接）。",
    en: "Not wired yet: there is no process host (no debug adapter is wired).",
    reason:
      "B3（同上）。给「运行 → 开始调试 / 停止调试 / 切换断点」三条。维护者口径点名调试类写「没有进程宿主」：真源走 DAP，本侧连能挂调试器的进程宿主都没有（BottomPaneKind::Run 只是占位文案）。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.subsystem",
    zh: "尚未接入：还没有对应子系统。",
    en: "Not wired yet: the corresponding subsystem does not exist.",
    reason:
      "B3（同上）。给「工具 → 数据库 / Web 检查器」两条。维护者口径点名这两条写「还没有对应子系统」：真源「数据库」默认就是灰的（backend-capabilities.ts:5 的 database:false），「Web 检查器」走 invoke('reopen_current_webview_devtools')，两者在 gpui 侧都没有对应物。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.github",
    zh: "尚未接入：还没有 GitHub 集成（认证 / 仓库 / 拉取请求三条链都不在）。",
    en: "Not wired yet: there is no GitHub integration (auth, repositories and pull requests are all missing).",
    reason:
      "B3（同上）。给「视图 → GitHub」。维护者口径点名写「还没有 GitHub 集成」。真源那一项走 GitHub 认证与仓库/PR 面板，gpui 侧零实现（crates 里没有任何 GitHub 客户端）。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.updater",
    zh: "尚未接入：还没有更新器（检查更新 / 下载 / 安装都没接）。",
    en: "Not wired yet: there is no updater (check, download and install are all missing).",
    reason:
      "B3（同上）。给「帮助 → 检查更新」。维护者口径点名写「还没有更新器」：真源走 Tauri updater（checkForUpdates + toast，use-menu-events-wrapper.ts:337-344），gpui 侧没有更新通道与签名校验。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.cloneUi",
    zh: "尚未接入：Core 的 git.write 已含 clone，缺的是 URL / 凭据 / 进度 UI。",
    en: "Not wired yet: Core's git.write already covers clone; what is missing is the URL, credential and progress UI.",
    reason:
      "B3（同上）。给**标题栏项目下拉**的「克隆仓库…」（那一行在 project_menu.rs，规格 §B3 的注明确把它收进这张占位表）。⚠️ 这一句**必须**这么写：能力已经在 Core 里（rust/lithe-core 的 git.write 命令含 clone，语义见 gpui/research/menu-open-prereqs.md），缺的是它外面那层界面 —— 写成「没有能力」是错的。真源那条恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.newProjectScaffolding",
    zh: "尚未接入：缺项目脚手架生成（真机是新建目录 + 起终端跑 npm create）。",
    en: "Not wired yet: project scaffolding is missing (Windows creates the directory and runs npm create in a terminal).",
    reason:
      "B3 那一批（同上；本轮补的第 17 组能力）。给**标题栏项目下拉**的「新建项目…」（那一行在 project_menu.rs，与 gpui.menuMissing.cloneUi 同一张面板）。事实依据：真机的新建项目是一条完整模态链路 —— createNewDirectory(destinationPath) → handleOpenFolderByPath → nextjs/vite 源再开一个终端跑脚手架命令（windows/tauri/src/features/project-picker/new-project-content.tsx:239-277），gpui 侧没有这条链路，所以那句「缺什么」说的是**能力本身不在**。⚠️ 与 gpui.menuMissing.cloneUi **不是**同一件事（那条说的是「能力已在 Core、只缺界面」），两句不许互相借用，所以它是独立一组、也是**第 17 组**（menu_bar.rs 的组数上界断言同步改到 17）。真源那条恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.fileLifecycle",
    zh: "尚未接入：缺新建文件 / 空标签页所需的文件生命周期（无标题 buffer + 命名 + 落盘）。",
    en: "Not wired yet: the untitled-buffer lifecycle (create, name, save to disk) does not exist.",
    reason:
      "B3（同上）。给「文件 → 新建标签页 / 新建文件」。事实依据：EditorPane 的 buffers 全是「盘上文件」，没有无标题 buffer，也没有新建文件对话框（editor_view.rs:1844 自认没有 + 按钮）。真源这两条恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.projectLifecycle",
    zh: "尚未接入：缺关闭项目的项目生命周期（销毁项目窗口 + 落盘项目列表）。",
    en: "Not wired yet: closing a project needs the project lifecycle (dispose the project window and persist the project list).",
    reason:
      "B3（同上）。给「文件 → 关闭文件夹」。事实依据：workspace.rs 的项目标签条 on_close 现在只把 active_project 取消选中，注释自认「关闭项目要销毁项目窗口 / 落盘项目列表，属项目生命周期，本轮不做」。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.localHistory",
    zh: "尚未接入：缺本地历史存储（没有内容仓库，也就没有可看的历史）。",
    en: "Not wired yet: there is no local history store (no content repository, so nothing to show).",
    reason:
      "B3（同上）。给「文件 → 显示本地历史」。事实依据：gpui 侧没有任何本地历史/快照存储（crates 里零命中）。真源那一项走 IDEA 式 Local History 面板，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.windowRouting",
    zh: "尚未接入：缺窗口级句柄路由（多窗口与窗口生命周期都还没做）。",
    en: "Not wired yet: window-level handle routing is missing (multi-window and the window lifecycle are not implemented).",
    reason:
      "B3（同上）。给「文件 → 新建窗口 / 关闭窗口 / 退出」。与 B4 的 gpui.newWindowNotWired 同一个根因（规格 §5 第 1 条：多窗口暂缓，菜单栏 / 项目下拉 / 命令面板 / Git 身份宿主那几个 thread_local 单例仍是「一个进程一份」）；这里换成菜单项那一侧的说法，也覆盖「退出 / 关闭窗口」这条窗口生命周期。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.helpAbout",
    zh: "尚未接入：缺帮助 / 关于的落地页（文档站与外链入口都没接）。",
    en: "Not wired yet: there is no help/about surface (the documentation site and external links are not wired).",
    reason:
      "B3（同上）。给「帮助 → 文档 / 新增功能 / 更新日志 / 报告 Bug / 请求新功能」。事实依据：这五条真源都走外部链接或环境信息拼装（use-menu-events-wrapper.ts:299-336），gpui 侧没有外链打开器与 issue 模板入口。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.shortcutsView",
    zh: "尚未接入：缺快捷键一览界面（现在只有 keymap，没有能看的清单）。",
    en: "Not wired yet: there is no keyboard-shortcuts view (the keymap exists but nothing displays it).",
    reason:
      "B3（同上）。给「工具 → 键盘快捷键」与「帮助 → 键盘快捷键」（真源两处各一条、共用同一个文案键，见 gpui/research/windows/10-menu-bar.md §2.7 的注）。事实依据：gpui 侧有 7 条 KeyBinding，但没有任何「列出全部快捷键」的界面。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "gpui.menuMissing.terminalSplit",
    zh: "尚未接入：终端没有拆分能力（一个终端面板只有一组页签，没有分栏）。",
    en: "Not wired yet: the terminal cannot be split (a terminal panel has one tab strip and no panes).",
    reason:
      "B3（同上）。给「终端 → 向右拆分终端 / 向下拆分终端」。事实依据：terminal crate 只有标签页（TerminalPane::new_tab / close_tab），没有分栏树（terminal_view.rs:695 也记着 terminal.close 之前同样未绑）。真源恒可执行，catalog 里没有这句。",
  },
  {
    key: "settings.gpui.appearanceSourceGlobal",
    zh: "当前外观跟随全局设置（这个键在工作区两层里都没有覆盖）",
    en: "This appearance follows the global settings (neither workspace layer overrides this key)",
    reason:
      "外观键的来源有四档，这是最后一档（跟随全局）。默认形态下界面**不画**任何标注（设计 Note 第七节：默认静默，不把「还没共享」当异常），这条键只在诊断与将来可能的显式说明里用到。",
  },
  {
    key: "settings.gpui.appearanceSourceTeam",
    zh: "当前外观来自团队设置（.lithe/settings.json，已被 Git 跟踪）",
    en: "This appearance comes from the team settings (.lithe/settings.json, tracked by Git)",
    reason:
      "外观被工作区覆盖时的来源三态之一。判据是「.lithe/settings.json 设了这个键，且该文件已被 Git 跟踪」——跟踪与否由 git ls-files 判定（见 settings/src/workspace.rs 的 AppearanceSource）。真源 Windows 侧没有项目级外观覆盖，也就没有这三条来源文案。",
  },
  {
    key: "settings.gpui.appearanceSourceProject",
    zh: "当前外观来自本项目的设置（.lithe/settings.json，尚未提交）",
    en: "This appearance comes from this project's settings (.lithe/settings.json, not committed yet)",
    reason:
      "同上的第二态：共享层文件设了这个键、但该文件还没被 Git 跟踪。它与「团队设置」是同一个文件的两个状态（设计 Note 第七节的表），界面文案必须是两句 —— 把未提交的文件说成团队设置会指向一个不存在的团队约定。默认不共享，所以这是常见形态。",
  },
  {
    key: "settings.gpui.appearanceSourceLocal",
    zh: "当前外观来自你的个人覆盖（.lithe/settings.local.json）",
    en: "This appearance comes from your personal override (.lithe/settings.local.json)",
    reason:
      "同上的第三态：本机层（.lithe/settings.local.json，不进版本控制）设了这个键，它的优先级最高。有工作区时用户在设置页改外观就是写这一层（见 store.rs 的 commit_appearance），所以用户改完立刻会看到这一句 —— 这正是「外观被工作区覆盖」这条决策需要的可见性。",
  },
  {
    key: "settings.gpui.appearanceRevertToGlobal",
    zh: "改回我的全局外观",
    en: "Reset to my global appearance",
    reason:
      "来源标注右侧的按钮（设计 Note 第四节与验收标准要求的「一键改回」）。它把这个键从工作区两层文件里**删掉**，于是生效值真的回落到全局值 —— 不是留一个空值。真源没有这条：Windows 侧外观只有全局一层，不存在「被项目覆盖」这个状态。",
  },
  {
    key: "gpui.workspaceConfigFailed",
    zh: "建立工作区配置失败：{reason}",
    en: "Failed to set up the workspace configuration: {reason}",
    reason:
      "打开项目时建立 `.lithe/project.json` 失败（写不进、或写入被静默丢弃）。真源没有对应文案：Windows 侧打开项目不写 `.lithe`，macOS 侧只在显式「识别并生成」时写。这条是 gpui 侧「打开即建」这条产品决策带来的失败面 —— 它必须**可见**，因为用户双击启动时看不到 stderr，只会发现 `.lithe` 没出现（父代理实测踩到过：环境静默丢弃工作区之外的写入，日志里只有 stderr）。仓库口径见 gpui/crates/git/src/changes_view.rs 的 render_write_error：失败一定可见、常驻红条、不自动消失。{reason} 由调用点填 WorkspaceConfigError 的 Display 原文。",
  },
  // ── 通知中心：Core 稳定错误码 ──────────────────────────────────────────
  // 这 10 条把 `rust/lithe-core/src/protocol/error.rs:11-34` 的 11 个 snake_case 错误码
  // （扣掉 `cancelled` —— 用户自己取消的，不进中心）映射成通知文案。
  //
  // 为什么真源里没有：Windows 侧通知中心的内容全部是产品层文案，从不显示 Core 错误码
  // （它的 389 个 `toast.*` 调用点传的都是 `t("git.historyMutationFailed")` 这类句子）。
  // gpui 是第一个把 Core 的**稳定码**放进通知中心的实现，所以这批键是 gpui 侧自有。
  //
  // 键名 camelCase（不是码本身的 snake_case）：`tr` 的键要人可读，且 `lithe.notifications.*`
  // 命名空间下与旧 Windows 的 `notifications.*` 键不冲突。码本身作为 `coreCode` 参数存进
  // 通知条目，于是既可搜（`Entry::matches_query` 对参数值原样匹配）也可显示在详情里。
  {
    key: "notifications.core.invalidRequest",
    zh: "请求无效：{detail}",
    en: "Invalid request: {detail}",
    reason:
      "Core 稳定错误码 `invalid_request` 的通知文案。真源没有：Windows 侧通知中心从不显示 Core 错误码。{detail} 填 `CoreError::message`（`error.rs:41` 写明它是 user-facing summary that is safe to display）。",
  },
  {
    key: "notifications.core.workspaceNotFound",
    zh: "找不到工作区：{detail}",
    en: "Workspace not found: {detail}",
    reason: "Core 稳定错误码 `workspace_not_found` 的通知文案。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.permissionDenied",
    zh: "没有权限：{detail}",
    en: "Permission denied: {detail}",
    reason: "Core 稳定错误码 `permission_denied` 的通知文案。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.notSupported",
    zh: "当前版本不支持这个操作：{detail}",
    en: "Not supported: {detail}",
    reason:
      "Core 稳定错误码 `not_supported` 的通知文案。分到 warning 档（不是 error）：`error.rs:18-19` 说它是「行为合法但本版 Core 没有」，也就是降级而非失败。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.runtimeMissing",
    zh: "缺少运行时：{detail}",
    en: "Required runtime is missing: {detail}",
    reason:
      "Core 稳定错误码 `runtime_missing` 的通知文案。分到 warning 档：`error.rs:20-21` 说它是「宿主发现的可执行文件或运行时不可用」，IDE 本身还能用。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.processStartFailed",
    zh: "无法启动进程：{detail}",
    en: "Could not start the process: {detail}",
    reason: "Core 稳定错误码 `process_start_failed` 的通知文案。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.processFailed",
    zh: "进程执行失败：{detail}",
    en: "The process failed: {detail}",
    reason: "Core 稳定错误码 `process_failed` 的通知文案。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.parseFailed",
    zh: "无法解析结果：{detail}",
    en: "Could not parse the result: {detail}",
    reason: "Core 稳定错误码 `parse_failed` 的通知文案。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.timedOut",
    zh: "操作超时：{detail}",
    en: "The operation timed out: {detail}",
    reason:
      "Core 稳定错误码 `timed_out` 的通知文案。分到 warning 档：超时通常是可重试的，不是一次不可逆的失败。真源没有，理由同 invalidRequest。",
  },
  {
    key: "notifications.core.unknown",
    zh: "操作失败：{detail}",
    en: "The operation failed: {detail}",
    reason:
      "Core 稳定错误码 `unknown` 的通知文案，也是**未识别码的兜底键**（`Severity::notification_code_for_core_code` 的 `_` 分支）。分到 warning 档而不是 error 是刻意的：Core 以后新增码时我们不知道它要不要用户处理，先按降级处理，不谎报严重性。真源没有，理由同 invalidRequest。",
  },
  // ── 通知中心：相对时间四档 ───────────────────────────────────────────
  // 旧 Windows 有 `formatNotificationAge`（`notification-formatters.ts`）但它没有自己的
  // 文案键 —— 相对时间在真源里是拼出来的英文/中文片段，不在 catalog。这 4 条是 gpui 侧
  // 通知中心要的**可翻译**相对时间：面板每行都要显示「多久以前」，而 gpui 的规矩是界面
  // 文案一律走 `tr`（`shared/contracts/application-boundary.md:228` 文案归展示层）。
  //
  // 只到分钟、只四档：秒级相对时间每帧都在变，会让整列表每帧重排
  // （`gpui/crates/workbench/src/notifications.rs` 的 `format_age` 注释）。
  {
    key: "notifications.justNow",
    zh: "刚刚",
    en: "just now",
    reason:
      "通知中心相对时间的第一档（不足 1 分钟）。真源没有：Windows 侧 `formatNotificationAge` 直接拼字符串、不进 catalog，而 gpui 侧界面文案一律走 `tr`。",
  },
  {
    key: "notifications.minutesAgo",
    zh: "{count} 分钟前",
    en: "{count} min ago",
    reason:
      "通知中心相对时间的第二档（1–59 分钟）。中文无复数变化，一个键够用；英文用 `min ago` 这种缩写形态，1 与 N 共用一个键，避免为一处时间戳引入复数规则（`extract-locale.mjs` 对真源里那些 `[count,plural]` 的不一致已经在提示了）。",
  },
  {
    key: "notifications.hoursAgo",
    zh: "{count} 小时前",
    en: "{count} h ago",
    reason: "通知中心相对时间的第三档（1–23 小时）。同上，无复数变化。",
  },
  {
    key: "notifications.daysAgo",
    zh: "{count} 天前",
    en: "{count} d ago",
    reason:
      "通知中心相对时间的第四档（≥1 天）。同 macOS 中心用绝对时刻（`WorkbenchView.swift:1900` 的 `date: .omitted, time: .shortened`）是同一层信息，跨天再折成天数比显示时刻更好比较。",
  },
];

const TS_QUOTES = new Set(['"', "'", "`"]);

/** 从源码里定位 `anchor` 之后的第一个 `{`，返回与之匹配的对象字面量文本（含花括号）。 */
function extractObjectLiteral(source, anchor) {
  const anchorAt = source.indexOf(anchor);
  if (anchorAt === -1) {
    throw new Error(`在源码中找不到锚点：${anchor}`);
  }

  const start = source.indexOf("{", anchorAt + anchor.length);
  if (start === -1) {
    throw new Error(`锚点 ${anchor} 之后找不到对象字面量起始的 “{”`);
  }

  let depth = 0;
  for (let i = start; i < source.length; i += 1) {
    const char = source[i];

    if (TS_QUOTES.has(char)) {
      i = skipStringLiteral(source, i);
      continue;
    }
    if (char === "/" && source[i + 1] === "/") {
      const newline = source.indexOf("\n", i + 2);
      if (newline === -1) break;
      i = newline;
      continue;
    }
    if (char === "/" && source[i + 1] === "*") {
      const end = source.indexOf("*/", i + 2);
      if (end === -1) throw new Error("对象字面量里有未闭合的块注释");
      i = end + 1;
      continue;
    }
    if (char === "{") {
      depth += 1;
      continue;
    }
    if (char === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, i + 1);
    }
  }

  throw new Error(`锚点 ${anchor} 的对象字面量没有闭合`);
}

/** 返回字符串字面量结束引号的下标（跳过转义）。 */
function skipStringLiteral(source, start) {
  const quote = source[start];
  for (let i = start + 1; i < source.length; i += 1) {
    const char = source[i];
    if (char === "\\") {
      i += 1;
      continue;
    }
    if (char === quote) return i;
  }
  throw new Error(`从下标 ${start} 开始的字符串字面量没有闭合`);
}

function loadCatalogs() {
  const aiCommitSource = readFileSync(AI_COMMIT_TS, "utf8");
  const localeSource = readFileSync(LOCALE_TS, "utf8");

  const englishBlock = extractObjectLiteral(aiCommitSource, "export const aiCommitEnglish");
  const chineseBlock = extractObjectLiteral(aiCommitSource, "export const aiCommitChinese");
  const catalogsBlock = extractObjectLiteral(localeSource, "const catalogs =");

  // 只保留对象字面量本体：TS 的类型标注（`Record<...>` / `as const`）自然被排除。
  const evaluate = new Function(
    [
      `const aiCommitEnglish = ${englishBlock};`,
      `const aiCommitChinese = ${chineseBlock};`,
      `const catalogs = ${catalogsBlock};`,
      "return catalogs;",
    ].join("\n"),
  );

  return evaluate();
}

const PLAIN_KEY = /^[A-Za-z_][A-Za-z0-9_.-]*$/;
const YAML_RESERVED = /^(?:true|false|null|yes|no|on|off|~)$/i;

function yamlKey(key) {
  return PLAIN_KEY.test(key) && !YAML_RESERVED.test(key) ? key : JSON.stringify(key);
}

/** JSON 字符串转义是 YAML 双引号标量的子集，直接复用。 */
function yamlValue(value) {
  return JSON.stringify(value);
}

function renderFile({ yamlLocale, entries, overrides, sourceLabel }) {
  const lines = [
    "# 本文件由 gpui/tools/extract-locale.mjs 自动生成，请勿手工编辑。",
    `# 真源：${sourceLabel}`,
    `# 另有 ${GPUI_ONLY_KEYS.length} 条 gpui 侧自有 key（settings.gpui.* / editor.gpui.* / appearance.gpui.* / maven.gpui.* / gpui.menuMissing.*）真源里没有，由脚本的 GPUI_ONLY_KEYS 提供（含理由）。`,
    "# 重新生成：node gpui/tools/extract-locale.mjs",
    "#",
    `# rust-i18n 4.2 约定：_version: ${FILE_VERSION}（key 在前、locale 在后）。`,
    `# ${APP_NAMESPACE} namespace = Lithe 页面文案；gpui_component namespace = 覆盖 gpui-kit 组件内置文案。`,
    "# 占位符沿用 Windows 前端的 {name} 语法，见本目录 README.md。",
    `_version: ${FILE_VERSION}`,
    "",
    `${APP_NAMESPACE}:`,
  ];

  for (const entry of entries) {
    lines.push(`  ${yamlKey(entry.key)}:`);
    lines.push(`    ${yamlLocale}: ${yamlValue(entry.value)}`);
  }

  if (overrides.length > 0) {
    lines.push("");
    lines.push("gpui_component:");
    for (const override of overrides) {
      lines.push(`  ${yamlKey(override.target)}:`);
      lines.push(`    ${yamlLocale}: ${yamlValue(override.value)}`);
    }
  }

  return `${lines.join("\n")}\n`;
}

function findPlaceholders(text) {
  const names = new Set();
  for (const match of text.matchAll(/\{([A-Za-z0-9_]+)\}/g)) {
    names.add(match[1]);
  }
  return [...names].sort();
}

function main() {
  const checkOnly = process.argv.includes("--check");
  const catalogs = loadCatalogs();

  const problems = [];
  const notices = [];
  const skipped = [];
  const outputs = [];

  for (const locale of CATALOG_LOCALES) {
    const catalog = catalogs[locale.catalogKey];
    if (catalog === undefined || typeof catalog !== "object" || catalog === null) {
      throw new Error(`真源里找不到 locale：${locale.catalogKey}`);
    }

    const entries = [];
    for (const [key, value] of Object.entries(catalog)) {
      if (typeof value !== "string") {
        skipped.push(`${locale.catalogKey} :: ${key} (${typeof value})`);
        continue;
      }
      entries.push({ key, value });
    }

    // GPUI 侧自有文案追加在真源 key 之后，顺序稳定（数组顺序）。
    for (const extra of GPUI_ONLY_KEYS) {
      entries.push({
        key: extra.key,
        value: locale.yamlLocale === "en" ? extra.en : extra.zh,
      });
    }

    const overrides = GPUI_COMPONENT_OVERRIDES.map(({ target, source }) => {
      const value = catalog[source];
      if (typeof value !== "string") {
        throw new Error(`gpui_component 覆盖 ${target} 引用的真源 key 不存在：${source}`);
      }
      return { target, value };
    });

    outputs.push({ locale, entries, overrides });
  }

  const [chinese, english] = outputs;
  const chineseKeys = new Set(chinese.entries.map((entry) => entry.key));
  const englishKeys = new Set(english.entries.map((entry) => entry.key));

  for (const key of chineseKeys) {
    if (!englishKeys.has(key)) problems.push(`zh-CN 有而 en 缺失：${key}`);
  }
  for (const key of englishKeys) {
    if (!chineseKeys.has(key)) problems.push(`en 有而 zh-CN 缺失：${key}`);
  }

  // 占位符差异只提示、不判失败：真源里英文有 {plural} 这类由调用点提供的复数占位符，
  // 中文用不着，属于预期内差异。见 README「占位符」一节。
  const englishByKey = new Map(english.entries.map((entry) => [entry.key, entry.value]));
  for (const entry of chinese.entries) {
    const englishValue = englishByKey.get(entry.key);
    if (englishValue === undefined) continue;
    const chinesePlaceholders = findPlaceholders(entry.value).join(",");
    const englishPlaceholders = findPlaceholders(englishValue).join(",");
    if (chinesePlaceholders !== englishPlaceholders) {
      notices.push(
        `占位符不一致：${entry.key} (zh-CN: [${chinesePlaceholders}] / en: [${englishPlaceholders}])`,
      );
    }
  }

  mkdirSync(OUT_DIR, { recursive: true });

  let changed = false;
  for (const { locale, entries, overrides } of outputs) {
    const content = renderFile({
      yamlLocale: locale.yamlLocale,
      entries,
      overrides,
      sourceLabel: "windows/tauri/src/i18n/locale.ts（含 ai-commit.ts 展开）",
    });
    const target = resolve(OUT_DIR, locale.file);
    const previous = tryRead(target);

    if (previous === content) continue;
    changed = true;
    if (checkOnly) {
      problems.push(`产物已过期：${relative(REPO_ROOT, target)}`);
      continue;
    }
    writeFileSync(target, content, "utf8");
    console.log(`${previous === null ? "已写入" : "已更新"} ${relative(REPO_ROOT, target)}`);
  }

  for (const { locale, entries, overrides } of outputs) {
    console.log(
      `${locale.file}: ${entries.length} 条 ${APP_NAMESPACE} key + ${overrides.length} 条 gpui_component 覆盖`,
    );
  }

  if (skipped.length > 0) {
    console.warn(`跳过 ${skipped.length} 条无法机械提取的 key：`);
    for (const item of skipped) console.warn(`  - ${item}`);
  }

  for (const notice of notices) console.log(`提示：${notice}`);
  for (const problem of problems) console.warn(`警告：${problem}`);

  if (checkOnly && !changed) {
    console.log("--check：产物与真源一致。");
  }

  if (problems.length > 0 || skipped.length > 0) {
    process.exitCode = 1;
  }
}

function tryRead(path) {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

main();
