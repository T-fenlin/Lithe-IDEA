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
    zh: "调整编辑器与终端正文的字号。界面字号在外观页。",
    en: "Adjusts the text size of the editor and terminal. Interface text size lives in Appearance.",
    reason:
      "真源的「编辑器 → 字体大小」这一行没有描述（macos-settings-panels.tsx:283-292），而 gpui 侧的字号落在唯一的等宽字号 token 上（Theme::mono_font_size），编辑器与终端正文共用它；补一句说明作用范围，避免与外观页的「界面字体大小」（uiFontSize，rem 基准）混淆。",
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
    zh: "覆盖值保存在当前电脑的全局设置文件里；留空则使用自动检测到的值。真源的同一页按项目保存在 .lithe/run/local.json，gpui 侧还没有项目级存储，所以这一页的作用范围是全局，不是当前项目。",
    en: "Overrides live in this computer's global settings file; leave a field empty to use the detected value. The Windows page stores them per project in .lithe/run/local.json, and gpui has no project-level store yet, so this page is global rather than project-scoped.",
    reason:
      "「项目 · JDK 与 Maven」页的作用域说明。真源的 settings.project.scope 写的是「仅保存在当前电脑，作用于当前项目」（project-environment-settings.tsx:212 + services/project-environment.ts 写 .lithe/run/local.json），而 gpui 侧的设置 crate 只能落**全局**设置文件（它不依赖外壳、拿不到工作区根，也没有项目级存储）—— 照 task 的硬要求，文案必须如实区分，不能假装是项目级。",
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
    `# 另有 ${GPUI_ONLY_KEYS.length} 条 gpui 侧自有 key（settings.gpui.* / editor.gpui.* / appearance.gpui.* / maven.gpui.*）真源里没有，由脚本的 GPUI_ONLY_KEYS 提供（含理由）。`,
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
