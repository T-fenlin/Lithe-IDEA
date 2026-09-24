//! 国际化包装：`rust_i18n::t!` + `{name}` 插值。
//!
//! ## 为什么需要这一层
//!
//! 项目的文案真源是 `windows/tauri/src/i18n/locale.ts`（经 `gpui/tools/extract-locale.mjs`
//! 生成 `crates/app/locales/lithe.{zh-CN,en}.yml`），它沿用前端语法：**`{count} 个空格`**
//! （`lithe.footer.spaces`）。
//!
//! 而 `rust-i18n` 4.2.2 的 `t!` 只把 **`%{...}`** 当占位符
//! （`rust-i18n-4.2.2/src/lib.rs:45-91` 的 `replace_with_args`），不认 `{...}`。
//! 直接用 `t!` + `args!` 会原样输出 `{count} 个空格`。
//!
//! 所以这一层做两件事：**先 `t!`，再按 `{name}` 替换**。locale 数据与前端保持逐字一致
//! （不为了迁就 `rust-i18n` 去改写生成出来的 yml —— 那份文件是自动生成的，改了会被覆盖）。
//!
//! ## locale loader 的位置约束（本仓库实测，决定了 `locales/` 放在本 crate）
//!
//! `rust_i18n::t!` 展开成 `crate::_rust_i18n_try_translate(..)`
//! （`rust-i18n-macro-4.2.2/src/tr.rs:438,454`），而 `crate::_rust_i18n_try_translate`
//! 由 `i18n!` 在**调用它的那个 crate** 里生成（`rust-i18n-4.2.2/src/lib.rs:402`）。
//! 所以 `t!` 只能在自己调过 `i18n!` 的 crate 里解析 —— 在别的 crate 里写
//! `rust_i18n::t!(..)` 会报 `E0433: cannot find _rust_i18n_t in crate`（本轮实测）。
//!
//! 本仓库要求 `tr` / `tr_args` 是 `shared` 对外的稳定包装，所以 `i18n!` 与 `locales/`
//! 一并落在 `shared`（`src/lib.rs`），`crates/app` 只组合窗口与 Feature。
//!
//! 附带结果：`rust-i18n` 的 `i18n!` 只会执行一次（`_RUST_I18N_BACKEND` 是 crate 内的
//! `LazyLock`），所以**不要**在 `app` 里再调一次 `i18n!` —— 那会生成第二份 backend，
//! 变成两个真相源。
//!
//! ## 返回值为什么是 `SharedString`
//!
//! 调用点绝大多数是 GPUI element 的 `.child(..)` / `.text(..)`，它们要 `IntoElement`，
//! 而 `String` 不是 `IntoElement`。这里直接给 `SharedString`，省掉每个调用点一次 `.into()`。
//!
//! ## 哪些文案走 i18n、哪些故意不走
//!
//! **走 i18n（`tr` / `tr_args`）**：一切用户看得见的界面文案。判据是"这句话会画在界面上、
//! 而且 locale 里有一条 zh-CN 值与它逐字相同的键"。zh-CN 下 `tr(key)` 返回的就是这句原文，
//! 所以接线不改变中文界面；en 下会变成英文，这是"真的接上了"的证据。当前已接线：
//!
//! | 位置 | 键 |
//! | --- | --- |
//! | `editor`（标签栏导航 / 关闭 / 空状态 / 打不开文件 / 保存与自动保存失败 / 关闭未保存确认 / 跳转失败） | `lithe.tabs.*`、`lithe.workbench.emptyEditor*`、`lithe.ui.noActionsHere`、`lithe.files.openFailed`、`lithe.editor.saveFailed`、`lithe.editor.autoSaveFailed`、`lithe.editor.gpui.*`、`lithe.unsavedChanges.title`、`lithe.ui.save`、`lithe.navigation.definition`、`lithe.navigation.noTargetFound` |
//! | `explorer`（头部 / 空态 / 加载 / 树 a11y） | `lithe.workbench.project`、`lithe.fileExplorer.*`、`lithe.quickOpen.loadingFiles`、`lithe.search.clear`、`lithe.ui.retry`、`lithe.titleProject.openFolder` |
//! | `git`（标题栏 / 筛选 / 提交表 / 引用树 / Inspector / 控制台 / 占位） | `lithe.git.*`、`lithe.workbench.gitLog`、`lithe.footer.readOnly` |
//! | `terminal`（页签栏 / 状态行 / 空态 / 失败态 / 能力提示除外的全部） | `lithe.terminal.*`、`lithe.run.*`、`lithe.git.console.exit`、`lithe.git.console.scrollToEnd`、`lithe.commandPalette.placeholder` |
//! | `workbench`（活动栏 / 状态栏 / 项目标签条 / 右工具窗） | `lithe.workbench.*`、`lithe.footer.spaces`、`lithe.titleProject.closeProject`、`lithe.maven.title`、`lithe.maven.notDetected`、`lithe.notifications.empty`、`lithe.extensions.noneFound`、`lithe.commandPalette.close` |
//! | `settings`（设置对话框：分类名 / 行标签 / 描述 / 按钮 / 确认对话框） | `lithe.settings.*`（含 6 条 `lithe.settings.gpui.*` 自有文案）、`lithe.ui.cancel` |
//!
//! 具体调用的键由本文件末尾的 `every_wired_key_resolves_in_both_locales` 测试守住：
//! **任何键拼错都会让测试失败**（缺键时 `rust-i18n` 原样回显键名，所以断言 `tr(key) != key`）。
//!
//! **故意不走 i18n**，理由分四类（都在调用点写了同样的说明）：
//!
//! 1. **内部诊断信息**：写进日志 / 错误字符串给开发者看的，不是给用户看的界面文案。
//!    例：`explorer` 的「工作区根不是合法 UTF-8 路径」「响应缺少 data.files」、`git` 的
//!    「响应缺少 files」、`core_client` 的「响应不是合法 JSON：{message}」「(无消息)」。
//!    给它们建翻译键等于把内部契约形状当成用户可见文案，会让 locale 变成第二份诊断表。
//! 2. **locale 里确实没有对应键**：逐字查过 `windows/tauri/src/i18n/locale.ts` 生成的两份
//!    yml，没有语义等价的键。例：`terminal` 的 `CAPABILITY_NOTICE`（"不能运行 vim/top 等
//!    全屏程序；Ctrl+C 不可用"，`terminal.*` 全量 `:7503-7558` 里没有描述这两条限制的键）、
//!    `editor` 的「文件超过 N MiB，暂不在编辑器中打开。」与「二进制文件不在编辑器中打开。」
//!    （`editor.largeFileServicesDisabled` 说的是"关了语言服务"，语义不同）、
//!    `core_client` 的「{command} 的响应缺少 data」、`workbench` 的状态栏内存项。
//!    这些要么需要前端补键（已登记），要么属于第 1 类。
//! 3. **占位文案（阶段 6 第二半未实现区域）**：`workbench` 的「{label} 工具窗（未实现）」
//!    （底部工具窗里仅剩的「运行 / 诊断」两项）。它是临时脚手架，落地的真实界面会用真键；
//!    现在接键只会把 `CONFIGURATION` 这类代码名翻成中文，反而更难认。⚠️ 例外：那句里的
//!    `{label}` 已经接了键 —— `BottomPaneKind::label()` 用
//!    `lithe.workbench.run` / `.diagnostics`（都在 WIRED 表里），
//!    所以剩下的中文字面量只是那句话的固定部分。
//!    （阶段 6 第一半之前这里还有一句「右侧工具窗（阶段 6）· 工作区：{}」，右工具窗做成真实
//!    区域后已删除，它的位置现在是 `lithe.maven.title` 等真键。）
//! 4. **不是界面文案**：CLI 用法提示（`app/src/main.rs` 的「用法：Lithe <workspace-root>」）、
//!    单元测试夹具（`terminal/src/ansi.rs` 的 `"终"` 是 ansi-cleaner 的字节夹具）。
//!
//! `tr_args` 的**占位符名以 locale 值为准**，不是以调用点原来的 `format!` 位置为准：
//! 例如 `lithe.git.log.logLabel` 在 locale 里是 `日志：{name}`（不是 `{reference_name}`），
//! `lithe.panes.filesCount` / `lithe.git.log.filesCount` 与
//! `lithe.git.log.filterPlaceholder` 分别是 `{count}` / `{field}`。
//! `interpolate` 对没配到的占位符**原样保留**（见下面的测试），所以名字写错会看见 `{name}`
//! 直接漏在界面上，而不是被静默清空。

use gpui_kit::SharedString;

/// 取一条文案（`key` 形如 `lithe.footer.spaces`），不带插值。
pub fn tr(key: &str) -> SharedString {
    SharedString::from(rust_i18n::t!(key).to_string())
}

/// 取一条文案并替换 `{name}` 占位符。
///
/// ```ignore
/// // lithe.spaces.count = "{count} 个空格"
/// let label = tr_args("lithe.footer.spaces", &[("count", "4")]);
/// ```
///
/// 参数是 `&[(&str, &str)]`（名字 + 已格式化好的值）：locale 数据里的占位符沿用前端语法，
/// 而"数字怎么格式化"是调用方的事，本层不做复数 / 数字格式的推断。
pub fn tr_args(key: &str, args: &[(&str, &str)]) -> SharedString {
    let template = rust_i18n::t!(key).to_string();
    SharedString::from(interpolate(&template, args))
}

/// 把模板里的 `{name}` 逐个替换成对应值；没有匹配的占位符原样保留。
///
/// 纯函数（不碰全局 locale 表），所以可以直接单测 —— 见文件末尾的测试。
/// 顺序替换而不是一次扫描：参数个数是个位数，这样读起来最直白。
fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut text = template.to_string();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::interpolate;

    /// `{name}` 必须被替换：这是本层存在的唯一理由
    /// （`rust-i18n` 只认 `%{name}`，直接 `t!` 会漏出花括号原文）。
    #[test]
    fn replaces_brace_placeholders() {
        assert_eq!(interpolate("{count} 个空格", &[("count", "4")]), "4 个空格");
    }

    /// 多个占位符、以及同一个占位符出现两次都要替换干净。
    #[test]
    fn replaces_every_occurrence() {
        assert_eq!(
            interpolate("{a}-{b}-{a}", &[("a", "x"), ("b", "y")]),
            "x-y-x"
        );
    }

    /// 模板里没有的占位符原样保留：漏配参数时应当看得见，而不是被静默清空。
    #[test]
    fn keeps_unknown_placeholders() {
        assert_eq!(interpolate("{missing}", &[("count", "4")]), "{missing}");
    }

    /// **所有已接线的文案键 × 两种 locale** 都必须命中（本层最有力的回归测试）。
    ///
    /// `rust-i18n` 在**缺键时原样回显键名**，所以 `tr(key) != key` 就是"这一条真的存在"的
    /// 判据。测试同时钉住 zh-CN 的**原文**：接线的前提是 `locale[key].zh-CN` 与被替换掉的
    /// 中文字面量逐字相同（否则中文界面会悄悄变样），这一列就是那句话的机器可校验版本。
    ///
    /// 表里的键 = 当前所有 `tr(..)` / `tr_args(..)` 调用点用到的键。
    /// **新增接线时把键补进这张表**：漏补不会失败（测试只覆盖表内的键），但把键写错、
    /// 或 locale 生成脚本把某条键删掉，这里会立刻红。
    #[test]
    fn every_wired_key_resolves_in_both_locales() {
        /// 已接线的键与它在 zh-CN 下的原文（原文 = 接线前写死在界面上的那一句）。
        const WIRED: &[(&str, &str)] = &[
            ("lithe.tabs.goBackShort", "后退"),
            ("lithe.tabs.goBack", "后退到上一个位置"),
            ("lithe.tabs.goForwardShort", "前进"),
            ("lithe.tabs.goForward", "前进到下一个位置"),
            ("lithe.tabs.close", "关闭"),
            ("lithe.ui.noActionsHere", "此处无任何内容"),
            ("lithe.workbench.emptyEditorTitle", "选择文件以查看"),
            (
                "lithe.workbench.emptyEditorDescription",
                "外部工具产生的更改会自动显示。",
            ),
            ("lithe.files.openFailed", "无法打开 {name}"),
            ("lithe.workbench.project", "项目"),
            ("lithe.fileExplorer.searchFiles", "搜索文件"),
            ("lithe.search.clear", "清除搜索"),
            ("lithe.fileExplorer.preferences", "文件资源管理器偏好设置"),
            ("lithe.fileExplorer.ariaLabel", "文件资源管理器"),
            ("lithe.quickOpen.loadingFiles", "正在加载文件"),
            ("lithe.ui.retry", "重试"),
            ("lithe.fileExplorer.noFolderOpen", "未打开文件夹"),
            ("lithe.titleProject.openFolder", "打开文件夹"),
            ("lithe.fileExplorer.noMatchingFiles", "没有匹配的文件"),
            ("lithe.fileExplorer.folderIsEmpty", "文件夹为空"),
            ("lithe.git.log.all", "全部"),
            ("lithe.workbench.gitLog", "提交记录"),
            ("lithe.git.log.showAll", "显示全部引用"),
            ("lithe.git.log.refresh", "刷新 Git 日志"),
            ("lithe.git.log.settings", "打开 Git 日志设置"),
            ("lithe.footer.readOnly", "只读"),
            ("lithe.git.log.hide", "隐藏提交记录"),
            ("lithe.git.console.log", "日志"),
            ("lithe.git.console.title", "控制台"),
            ("lithe.git.log.filterField", "Git 日志筛选字段"),
            ("lithe.git.log.hideDecorations", "隐藏分支和标签"),
            ("lithe.git.log.showDecorations", "显示分支和标签"),
            ("lithe.git.log.commit", "提交"),
            ("lithe.git.log.author", "作者"),
            ("lithe.git.log.date", "日期"),
            ("lithe.git.log.noCommits", "此视图中没有提交"),
            ("lithe.git.log.noMatch", "没有符合筛选条件的提交"),
            ("lithe.git.log.loadingCommits", "正在加载提交…"),
            ("lithe.git.log.loadMore", "加载更多提交"),
            ("lithe.git.expandAll", "全部展开"),
            ("lithe.git.collapseAll", "全部折叠"),
            ("lithe.git.log.toolbar.showAllBranches", "显示全部分支"),
            ("lithe.git.log.toolbar.showMyBranches", "显示我的分支"),
            ("lithe.git.log.toolbar.newBranch", "新建分支"),
            ("lithe.git.current", "当前"),
            ("lithe.git.log.headCurrentBranch", "HEAD（当前分支）"),
            ("lithe.git.log.none", "无"),
            ("lithe.git.log.references", "引用"),
            ("lithe.git.log.commitFiles", "提交文件"),
            ("lithe.git.log.loadingShort", "加载中…"),
            ("lithe.git.log.openCommitDiff", "打开提交差异"),
            ("lithe.git.log.selectCommit", "选择一个提交"),
            ("lithe.git.log.loadingChangedFiles", "正在加载更改的文件…"),
            ("lithe.git.log.unableToLoadFiles", "无法加载更改的文件"),
            ("lithe.git.log.noChangedFiles", "没有更改的文件"),
            ("lithe.git.log.commitDetails", "提交详情"),
            ("lithe.git.console.find", "在 Git 控制台中查找"),
            ("lithe.git.console.wrap", "自动换行"),
            ("lithe.git.console.scrollToEnd", "滚动到底部"),
            ("lithe.git.console.cancel", "取消正在运行的操作"),
            ("lithe.git.console.clear", "清空"),
            ("lithe.git.console.copy", "复制输出"),
            ("lithe.git.console.empty", "Git 命令及其输出将显示在这里。"),
            ("lithe.git.log.retry", "重试"),
            ("lithe.git.log.unableToRefresh", "无法刷新 Git 日志。"),
            ("lithe.git.noRepositoryOpen", "未打开仓库"),
            (
                "lithe.git.log.openWorkspace",
                "打开一个 Git 工作区以查看提交记录。",
            ),
            ("lithe.git.log.loading", "正在加载 Git 日志…"),
            ("lithe.git.log.filterText", "文本"),
            ("lithe.git.log.filterAuthor", "作者"),
            ("lithe.git.log.filterBranch", "分支"),
            ("lithe.git.log.local", "本地"),
            ("lithe.git.log.remote", "远程"),
            ("lithe.git.log.tags", "标签"),
            ("lithe.footer.spaces", "{count} 个空格"),
            ("lithe.terminal.noTerminals", "没有终端"),
            ("lithe.terminal.chooseTerminalProfile", "选择终端配置文件"),
            ("lithe.terminal.contextClear", "清除终端"),
            ("lithe.terminal.errorTitle", "终端错误"),
            ("lithe.terminal.errorFallback", "无法初始化终端"),
            ("lithe.terminal.retry", "重试"),
            ("lithe.run.running", "运行中"),
            ("lithe.run.succeeded", "成功"),
            ("lithe.run.failed", "失败"),
            ("lithe.git.console.exit", "退出码"),
            ("lithe.commandPalette.placeholder", "输入命令..."),
            ("lithe.terminal.terminals", "终端"),
            ("lithe.terminal.tabClose", "关闭 {name}"),
            ("lithe.terminal.newTerminal", "新建终端"),
            ("lithe.workbench.run", "运行"),
            ("lithe.workbench.diagnostics", "诊断"),
            ("lithe.workbench.changes", "更改"),
            ("lithe.workbench.search", "搜索"),
            ("lithe.workbench.terminal", "终端"),
            ("lithe.workbench.settings", "设置"),
            ("lithe.workbench.maven", "Maven"),
            ("lithe.extensions.title", "扩展"),
            ("lithe.notifications.title", "通知"),
            ("lithe.git.log.filesCount", "{count} 个文件"),
            ("lithe.git.log.filterPlaceholder", "{field} 筛选"),
            ("lithe.git.log.logLabel", "日志：{name}"),
            ("lithe.titleProject.closeProject", "关闭项目 {name}"),
            // settings（`gpui/crates/settings`，阶段 8）。
            ("lithe.settings.tabs.general", "常规"),
            ("lithe.settings.tabs.appearance", "外观"),
            ("lithe.settings.mac.language", "语言"),
            ("lithe.settings.mac.categories", "设置分类"),
            ("lithe.settings.mac.appearanceMode", "外观模式"),
            (
                "lithe.settings.mac.appearanceDescription",
                "选择配色主题，并设置是否跟随系统外观。",
            ),
            ("lithe.settings.mac.followSystem", "跟随系统"),
            ("lithe.settings.mac.light", "浅色"),
            ("lithe.settings.mac.dark", "深色"),
            ("lithe.settings.mac.done", "完成"),
            ("lithe.settings.mac.restoreDefaults", "恢复默认设置"),
            ("lithe.settings.appearance.theme", "主题"),
            ("lithe.settings.appearance.colorTheme", "颜色主题"),
            ("lithe.settings.appearance.typography", "字体排印"),
            ("lithe.settings.appearance.interface", "界面"),
            ("lithe.settings.appearance.uiFontSize", "界面字体大小"),
            (
                "lithe.settings.appearance.uiFontSizeDescription",
                "以 0.5 像素为单位调整界面文本和图标缩放",
            ),
            ("lithe.settings.appearance.showStatusBar", "显示状态栏"),
            ("lithe.ui.cancel", "取消"),
            // 下面 6 条真源（`windows/tauri/src/i18n/locale.ts`）里没有，由
            // `gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 提供（每条都写了理由）。
            ("lithe.settings.gpui.languageEnglish", "英语"),
            ("lithe.settings.gpui.languageChinese", "简体中文"),
            (
                "lithe.settings.gpui.languageRestartDescription",
                "切换语言会立即重启 Lithe。",
            ),
            ("lithe.settings.gpui.restoreDefaultsOpen", "恢复默认设置…"),
            ("lithe.settings.gpui.restoreDefaultsTitle", "恢复默认设置？"),
            ("lithe.settings.gpui.restoreDefaultsBody", "所有设置都会回到默认值。"),
            // 阶段 9 的编辑器侧（2 条，同样由 `GPUI_ONLY_KEYS` 提供，理由写在脚本里）。
            ("lithe.editor.gpui.discardChanges", "放弃修改"),
            (
                "lithe.editor.gpui.unsavedChangesBody",
                "对“{name}”的修改尚未保存。",
            ),
            // 阶段 9 的编辑器侧复用的**真源既有**键：保存失败的两句反馈，以及
            // 关闭未保存文件时确认对话框的标题与两个按钮
            // （`windows/tauri/src/i18n/locale.ts:4637-4638`、`:7872`、`:4617`、`:436`）。
            ("lithe.editor.saveFailed", "无法保存“{name}”。请检查文件是否可写，然后重试。"),
            (
                "lithe.editor.autoSaveFailed",
                "无法自动保存“{name}”。更改仍保留在编辑器中。",
            ),
            ("lithe.unsavedChanges.title", "未保存的更改"),
            ("lithe.ui.save", "保存"),
            // 阶段 10 第一批（Java 代码跳转）复用的**真源既有**键：跳转失败时的提示
            // 「未找到{target}。」+ `{target}` 的取值「定义」
            // （`windows/tauri/src/i18n/locale.ts:8435,8439`；英文侧同键 `:4137,4141`）。
            ("lithe.navigation.definition", "定义"),
            ("lithe.navigation.noTargetFound", "未找到{target}。"),
            // 阶段 6 第一半（右侧工具窗）：三个视图的头部标题 + 各自空态，以及面板关闭按钮的
            // 无障碍名 / tooltip。全部是**真源既有**键：
            // `maven.title`（`maven-pane.tsx:602`；`locale.ts:6043` / en `:1623`）、
            // `maven.notDetected`（`maven-pane.tsx:806`；`locale.ts:6102` / en `:1686`）、
            // `notifications.empty`（`notifications-tool-window.tsx:399`；`locale.ts:7324` / en `:2998`）、
            // `extensions.noneFound`（`locale.ts:7620` / en `:3300`）、
            // `commandPalette.close`（两个工具窗的关闭按钮都用它，`maven-pane.tsx:606`、
            // `notifications-tool-window.tsx:355`；`locale.ts:7938` / en `:3619`）。
            ("lithe.maven.title", "Maven"),
            ("lithe.maven.notDetected", "未检测到 Maven 项目"),
            ("lithe.notifications.empty", "暂无通知。"),
            ("lithe.extensions.noneFound", "未找到扩展。"),
            ("lithe.commandPalette.close", "关闭命令面板"),
        ];

        // locale 是进程级全局状态，而 `cargo test` 默认并行跑同一个二进制里的测试。
        // 本模块目前只有纯函数测试，但为将来加测试的人留一道锁，避免出"偶发失败"。
        static LOCALE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCALE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous = rust_i18n::locale().to_string();
        // 无论断言是否 panic，都要把 locale 还原：否则会影响同一二进制里其它测试
        // 或调用方的默认语言。`Drop` 在 unwind 时照样执行。
        struct RestoreLocale(String);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                rust_i18n::set_locale(&self.0);
            }
        }
        let _restore = RestoreLocale(previous);

        // 1. zh-CN：键必须命中，且原文与接线前写死在界面上的那一句逐字相同。
        rust_i18n::set_locale("zh-CN");
        for (key, expected) in WIRED {
            let actual = super::tr(key);
            assert_ne!(
                actual.as_ref(),
                *key,
                "zh-CN 缺键（rust-i18n 回显了键名）：{key}"
            );
            assert_eq!(actual.as_ref(), *expected, "zh-CN 原文与预期不一致：{key}");
        }

        // 2. en：键必须命中，且**不能**与 zh-CN 相同 —— 那是"真的接上了"的证据
        //    （若 en 的键缺失，`fallback = "en"` 之后仍会回显键名，第 1 条断言就已经守住；
        //    这里再要求中英不同，防止将来有人把 en 误填成中文）。
        rust_i18n::set_locale("en");
        for (key, zh) in WIRED {
            let actual = super::tr(key);
            assert_ne!(
                actual.as_ref(),
                *key,
                "en 缺键（rust-i18n 回显了键名）：{key}"
            );
            if SAME_IN_BOTH_LOCALES.contains(key) {
                continue;
            }
            assert_ne!(
                actual.as_ref(),
                *zh,
                "en 与 zh-CN 文案相同，等于没接上：{key}"
            );
        }
    }

    /// en 与 zh-CN **本来就该逐字相同**的键：专有名词 / 产品名，没有可翻译的内容。
    ///
    /// `rust-i18n` 缺键时会回显键名，所以"键存在"由 `tr(key) != key` 守住；而"en 不能等于中文"
    /// 这条对 `Maven` 这种名字不成立。这是一张**例外清单**，不是白名单：往里加键等于声明
    /// "这条英文与中文本来一样"，必须在 review 里给出理由。
    ///
    /// ⚠️ 现状：locale 里共有 68 条中英逐字相同的键（`settings.mac.shell`、`git.url`、
    /// `commandPalette.categories.AI` 这类），**只有进了 WIRED 才会被断言到**；目前 WIRED 里的
    /// 例外是 `lithe.workbench.maven`（**右**活动栏的 Maven 项，阶段 6 第一半后
    /// Maven 只归右栏）与 `lithe.maven.title`（右工具窗 Maven 视图的标题）—— 都是产品名，
    /// 没有可翻译的内容。
    const SAME_IN_BOTH_LOCALES: &[&str] = &["lithe.workbench.maven", "lithe.maven.title"];
}
