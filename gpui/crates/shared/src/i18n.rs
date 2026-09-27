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
//! | `editor` 的标签右键菜单（阶段 11） | `lithe.files.{copyPath,copyRelativePath,reveal,openInTerminal}`、`lithe.tabs.{reload,close,closeOthers,closeToRight,closeAll}`、`lithe.menu.saveAll`（批量确认的"全部保存"）、`lithe.editor.gpui.unsavedChangesBatchBody` |
//! | `explorer`（空态 / 加载 / 树 a11y） | `lithe.workbench.project`、`lithe.fileExplorer.*`、`lithe.quickOpen.loadingFiles`、`lithe.search.clear`、`lithe.ui.retry`、`lithe.titleProject.openFolder`（⚠️ 2026-09-27 起「项目」树头与树内搜索行已按维护者要求删除，`lithe.fileExplorer.searchFiles` 仍用作搜索输入框的 placeholder，`lithe.search.clear` 等键随搜索入口一起**暂时没有界面落点**，待左活动栏「搜索」项接上） |
//! | `git`（标题栏 / 筛选 / 提交表 / 引用树 / Inspector / 控制台 / 占位） | `lithe.git.*`、`lithe.workbench.gitLog`、`lithe.footer.readOnly` |
//! | `terminal`（页签栏 / 状态行 / 空态 / 失败态 / 能力提示除外的全部） | `lithe.terminal.*`、`lithe.run.*`、`lithe.git.console.exit`、`lithe.git.console.scrollToEnd`、`lithe.commandPalette.placeholder` |
//! | `workbench`（活动栏 / 状态栏 / 项目标签条 / 右工具窗） | `lithe.workbench.*`、`lithe.footer.spaces`、`lithe.titleProject.closeProject`、`lithe.maven.title`、`lithe.maven.notDetected`、`lithe.notifications.empty`、`lithe.extensions.noneFound`、`lithe.commandPalette.close` |
//! | `workbench` 的命令面板（阶段 6 第二半） | `lithe.commandPalette.{title, placeholder, noCommands, categories.*}`、`lithe.commandPalette.actions.toggle-*`、`lithe.commandPalette.actions.color-theme.label`、`lithe.settings.appearance.theme`、`lithe.settings.appearance.showStatusBar`、`lithe.settings.appearance.showStatusBarDescription`、`lithe.settings.tabs.*`、`lithe.appearance.gpui.*`、`lithe.maven.gpui.*` |
//! | `settings`（设置对话框：分类名 / 行标签 / 描述 / 按钮 / 确认对话框） | `lithe.settings.*`（含 26 条 `lithe.settings.gpui.*` 自有文案 —— 全部 26 条自有键都已接线）、`lithe.ui.cancel` |
//! | `workbench` 的分支弹窗（阶段 11 第三件） | `lithe.git.{searchBranchesAria,repositories,branches,worktrees,selectorSections,searchBranches,branchCount,branchesCount,noMatchingBranches,noBranchesFound,current,refresh}` |
//! | `workbench` 的打开项目 / 最近项目（B4） | `lithe.menu.openFolder`、`lithe.projectOpen.{title,where,doNotAskAgain,cancel,newWindow,thisWindow}`（全部真源既有键）、`lithe.gpui.newWindowNotWired`（**唯一**一条 gpui 自有键，理由见 `extract-locale.mjs`） |
//! | `workbench` 的菜单占位项（B3） | **17 条** `lithe.gpui.menuMissing.*`（一种能力一句："尚未接入：缺 X"），由 `menu_bar.rs` 的 `MISSING_GROUPS` 使用、状态栏左侧显示（其中 `cloneUi` / `newProjectScaffolding` 两条用在项目下拉那两行上）；全部是 gpui 自有键（真源的菜单项恒可执行，catalog 里没有这类句子），理由见 `extract-locale.mjs` |
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
//! 4. **不是界面文案**：CLI 用法提示（`app/src/main.rs` 的「用法：Lithe [<workspace-root>]」）、
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
            // 通知中心的搜索 / 筛选 / 详情 / 清除。这批键 2026-09-27 之前就在
            // catalog 里但无人引用（右工具窗只有空态，见 right_tool_window.rs 的模块
            // 文档）；通知中心落地后全部接上。注意 `filterSuccess` **故意不用**：中心
            // 没有 success 档（`lithe-gpui-notify` 的 `severity.rs` 模块文档）。
            ("lithe.notifications.search", "搜索通知"),
            ("lithe.notifications.noMatch", "没有匹配的通知。"),
            ("lithe.notifications.filter", "筛选通知"),
            ("lithe.notifications.filterAll", "全部"),
            ("lithe.notifications.filterInfo", "信息"),
            ("lithe.notifications.filterWarnings", "警告"),
            ("lithe.notifications.filterErrors", "错误"),
            ("lithe.notifications.details", "通知详情"),
            ("lithe.notifications.delete", "删除"),
            ("lithe.notifications.clearAll", "全部清除"),
            // Core 稳定错误码 → 通知文案。11 个码扣掉 `cancelled`（用户自己取消的，
            // 不进中心），所以是 10 条。`{detail}` 填 `CoreError::message`。
            // 分档与文案都在 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里有理由。
            (
                "lithe.notifications.core.invalidRequest",
                "请求无效：{detail}",
            ),
            (
                "lithe.notifications.core.workspaceNotFound",
                "找不到工作区：{detail}",
            ),
            (
                "lithe.notifications.core.permissionDenied",
                "没有权限：{detail}",
            ),
            ("lithe.notifications.core.notSupported", "当前版本不支持这个操作：{detail}"),
            ("lithe.notifications.core.runtimeMissing", "缺少运行时：{detail}"),
            (
                "lithe.notifications.core.processStartFailed",
                "无法启动进程：{detail}",
            ),
            ("lithe.notifications.core.processFailed", "进程执行失败：{detail}"),
            ("lithe.notifications.core.parseFailed", "无法解析结果：{detail}"),
            ("lithe.notifications.core.timedOut", "操作超时：{detail}"),
            ("lithe.notifications.core.unknown", "操作失败：{detail}"),
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
            // 阶段 14（设置的「编辑器」页）：分组标题 + 两项标签 + 单位词。
            // 全部是**真源既有**键（`macos-settings-panels.tsx:275-328` 的渲染点；
            // `settings.mac.spaces` 在那个文件里被拼成 `` `${size} ${t("settings.mac.spaces")}` ``）。
            ("lithe.settings.tabs.editor", "编辑器"),
            ("lithe.settings.mac.display", "显示"),
            ("lithe.settings.mac.fontSize", "字体大小"),
            ("lithe.settings.mac.indentation", "缩进"),
            ("lithe.settings.mac.tabWidth", "制表符宽度"),
            ("lithe.settings.mac.spaces", "个空格"),
            // 阶段 14（设置的「终端」页）：真源 `macos-settings-panels.tsx:372-396` 的
            // 分组标题 / 行标签 / 描述，以及四个下拉选项名（`:388-391`）。
            ("lithe.settings.tabs.terminal", "终端"),
            ("lithe.settings.mac.shell", "Shell"),
            ("lithe.settings.mac.defaultShell", "默认 Shell"),
            (
                "lithe.settings.mac.defaultShellDescription",
                "用于新的终端会话。",
            ),
            ("lithe.settings.mac.systemDefault", "系统默认"),
            ("lithe.settings.mac.shellPowerShell", "PowerShell"),
            ("lithe.settings.mac.shellCommandPrompt", "命令提示符"),
            ("lithe.settings.mac.shellWsl", "WSL"),
            // 阶段 14（设置左栏保留的分类）：7 个空态分类的**分类名**都是真源既有键
            // （`settings.tabs.*` 与 `settings.project.title` / `settings.run.title`，
            // 分类表 `settings-dialog.tsx:35-48`）。
            ("lithe.settings.tabs.keyboard", "快捷键"),
            ("lithe.settings.tabs.lsp", "LSP"),
            ("lithe.settings.tabs.git", "Git"),
            ("lithe.settings.tabs.logs", "日志"),
            ("lithe.settings.tabs.updates", "更新"),
            ("lithe.settings.project.title", "项目 · JDK 与 Maven"),
            ("lithe.settings.run.title", "运行配置"),
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
            // 阶段 14：真源的「编辑器 → 字体大小」这一行没有描述，本侧补一句说明作用范围
            // （本侧有三个互不影响的字号键：界面 / 编辑器 / 终端）——理由写在 `GPUI_ONLY_KEYS` 里。
            (
                "lithe.settings.gpui.editorFontSizeDescription",
                "调整代码编辑器的字号。界面字号在外观页，终端字号在终端页。",
            ),
            // 字体三键（本批）：两个字族与终端字号都用「空值 = 不覆盖」的语义，
            // 而真源的对应设置永远有具体值，所以「不覆盖」这个选项名是 gpui 侧新增的。
            ("lithe.settings.gpui.valueNotOverridden", "默认（不覆盖）"),
            // 阶段 14：7 个"还没有子系统"的分类的**明确空态**文案（标题 + 每个分类一句前置条件）。
            // 真源里这些页都有内容，所以这一族键全是 gpui 侧新增；理由逐条写在
            // `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里。
            (
                "lithe.settings.gpui.pageNotAvailableTitle",
                "此分类尚未接入",
            ),
            (
                "lithe.settings.gpui.prerequisiteKeyboard",
                "前置条件：键位表与快捷键预设（现在只有固定绑定的少数快捷键）。",
            ),
            (
                "lithe.settings.gpui.prerequisiteGit",
                "前置条件：Git 身份（user.name / user.email）的读写接线；Core 的 git.repositorySetup 与 git.configureIdentity 已经就绪。",
            ),
            (
                "lithe.settings.gpui.prerequisiteLogs",
                "前置条件：日志系统（日志目录、诊断开关与诊断包导出）。",
            ),
            (
                "lithe.settings.gpui.prerequisiteUpdates",
                "前置条件：更新器（检查更新、更新通道与安装流程）。",
            ),
            // 「项目 · JDK 与 Maven」页（阶段 16）：作用域说明 + 「来源」那一栏里两种环境变量来源
            // + Maven 提示 + settings.xml / 本地仓库的三条说明。理由逐条写在
            // `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里。
            //
            // ⚠️ 这一族**不是**空态文案：这一页有真实页面（探测 + 生效值 + 覆盖值），
            // 所以 `prerequisiteProject` 已经被删掉（`Category::prerequisite_key` 现在返回 `None`）。
            (
                "lithe.settings.gpui.projectScopeGlobal",
                "没有打开项目：覆盖值保存在当前电脑的全局设置文件里，作为本机默认；留空则使用自动检测到的值。打开项目后，这一页改的是那个项目自己的 .lithe/ 本机层，它的优先级更高。",
            ),
            (
                "lithe.settings.gpui.projectScopeProject",
                "覆盖值保存在当前项目的 .lithe/ 本机层（run.local.json 与 maven.local.json，不进版本控制）；本机层留空的值回落到全局设置，两层都空就用自动检测。",
            ),
            (
                "lithe.settings.gpui.overrideFromProject",
                "覆盖值来自本项目",
            ),
            (
                "lithe.settings.gpui.overrideFromGlobal",
                "覆盖值来自全局设置",
            ),
            (
                "lithe.settings.gpui.sourceFromEnv",
                "来自 {name} 环境变量",
            ),
            (
                "lithe.settings.gpui.mavenExecutableHint",
                "可填 Maven 主目录或 mvn 启动器；留空则依次查找 MAVEN_HOME、M2_HOME 与 PATH 上的 Maven。",
            ),
            (
                "lithe.settings.gpui.mavenSettingsMissing",
                "未检测到 settings.xml（已查用户目录的 .m2/settings.xml 与 Maven 安装目录的 conf/settings.xml）",
            ),
            (
                "lithe.settings.gpui.mavenLocalRepositoryDefault",
                "Maven 默认位置（settings.xml 未指定 localRepository）",
            ),
            (
                "lithe.settings.gpui.mavenLocalRepositoryFromSettings",
                "来自用户 settings.xml 的 <localRepository>",
            ),
            (
                "lithe.settings.gpui.mavenLocalRepositoryUnknown",
                "未知（拿不到用户主目录，推不出 Maven 默认的本地仓库位置）",
            ),
            (
                "lithe.settings.gpui.projectNothingDetected",
                "既没有探测到 JDK，也没有探测到 Maven。下面每一行都写明了查过哪些位置、各自为什么不行。",
            ),
            // 「项目 · JDK 与 Maven」页的 ≥ 21 闸门标注（阶段性批次的 A1）：
            // 判据与数值的唯一真源是 `java/src/jdtls.rs` 的 `MINIMUM_JAVA_MAJOR` / `probe_java`，
            // 镜像落在 `settings/src/project.rs`（那里写清了对应哪一条）。{minimum} 由调用点填。
            (
                "lithe.settings.gpui.jdkBelowLanguageServiceMinimum",
                "低于语言服务最低要求（JDT LS 需要 JDK {minimum} 或更新版本），语言服务无法启动。",
            ),
            // Maven 两个覆盖值的「尚未生效」标注（同批次 A2）：gpui 侧不执行 mvn、
            // 也没有采购 `runConfig.createLaunchPlan`，所以这两个键今天存了没人读。
            (
                "lithe.settings.gpui.mavenOverrideNotEffective",
                "尚未生效：gpui 侧还没有执行 Maven 的通路（不执行 mvn，也没采购 Core 的启动计划），这个值现在不改变任何行为。",
            ),
            // 「LSP」页（阶段 18）：真源三键里唯一有真消费方的那一个的开关注释，
            // 加两句"为什么这一页只有一项"与"还没有可用运行时"的如实说明。
            (
                "lithe.settings.gpui.autoCompletionDescription",
                "输入时自动显示补全建议（Java 语言服务优先，语言服务不可用时用当前文件的标识符兜底）。关掉之后不再自动弹出补全菜单，兜底候选也一并停止。",
            ),
            (
                "lithe.settings.gpui.lspOnlyAutoCompletion",
                "本页只做「自动补全」：真源另两项（参数提示、语义高亮）在 gpui 侧没有可挂载的接口（上游没有签名帮助 provider，语义高亮也没有 Java 侧实现），JDTLS / JDK 运行时路径则归「项目 · JDK 与 Maven」页，所以这里不画控件。",
            ),
            (
                "lithe.settings.gpui.noLanguageServer",
                "还没有可用的语言服务运行时（本机没有探测到满足要求的 JDK；打开 Java 项目时也会再检测一次）。",
            ),
            // 「运行配置」页（阶段 17）：无项目 / 空态 / 失败 / 没有命令行 / 执行未接入 五句。
            //
            // ⚠️ 这一族**不是**空态文案：这一页有真实页面（Core 的 `runConfig.generate` 给出的
            // 启动目标 + 刷新），所以 `prerequisiteRun` 已经被删掉
            // （`Category::prerequisite_key` 现在对 `Run` 返回 `None`）。
            // 五条的理由逐条写在 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里。
            (
                "lithe.settings.gpui.runNoProject",
                "打开项目后才能识别可运行配置。",
            ),
            (
                "lithe.settings.gpui.runReadOnlyDescription",
                "本页列出 Core 从项目文件识别到的可运行目标：名称、类型、命令行、工作目录与生效来源。识别结果不写入任何文件；本页只读，没有编辑配置与启动的入口。",
            ),
            (
                "lithe.settings.gpui.runNothingDetected",
                "没有在项目文件里识别到可运行配置。Core 已按 Maven（Spring Boot / Quarkus / Micronaut）、Gradle、npm、Compose、Procfile、Python、Cargo、Go、Make 与 just 逐类探测过；Java 主类由语言服务提供，gpui 侧还没接那一路，所以这里只会出现 Maven 模块与脚本类目标。",
            ),
            (
                "lithe.settings.gpui.runLoadFailed",
                "识别运行配置失败：{reason}",
            ),
            (
                "lithe.settings.gpui.runNoCommand",
                "没有固定命令行（可执行文件由工具链提供，真正的命令行由启动计划组装）",
            ),
            (
                "lithe.settings.gpui.runNotWired",
                "点击启动尚未接入：gpui 侧还没有运行面板与进程宿主，「运行」工具窗与「运行」菜单目前都是占位。Core 的 runConfig.createLaunchPlan（平台无关的启动计划）已经能给出计划，但还没有消费方。",
            ),
            // 「项目 · JDK 与 Maven」页的路径选择按钮 + Maven 配置那两行的提示与生效说明
            // （5 条，都由 `GPUI_ONLY_KEYS` 提供，理由写在脚本里：真源那颗按钮是纯图标且
            // 没有这两行的说明）。
            ("lithe.settings.gpui.pickPath", "选择…"),
            (
                "lithe.settings.gpui.mavenSettingsHint",
                "留空则用检测到的那一份：用户目录的 .m2/settings.xml 优先，其次 Maven 安装目录的 conf/settings.xml。",
            ),
            (
                "lithe.settings.gpui.mavenSettingsEffective",
                "生效：下一次语言服务启动时作为 Maven 用户设置交给 Java 语言服务。",
            ),
            (
                "lithe.settings.gpui.mavenLocalRepositoryHint",
                "留空则用生效 settings.xml 里的 <localRepository>；没写就是 Maven 默认位置 ~/.m2/repository。",
            ),
            (
                "lithe.settings.gpui.overridePathMissing",
                "这个路径不存在；留空会回到自动检测。",
            ),
            // 阶段 9 的编辑器侧（3 条，同样由 `GPUI_ONLY_KEYS` 提供，理由写在脚本里）。
            ("lithe.editor.gpui.discardChanges", "放弃修改"),
            (
                "lithe.editor.gpui.unsavedChangesBody",
                "对“{name}”的修改尚未保存。",
            ),
            // 阶段 11（标签右键菜单）的批量关闭确认正文：正文如实说"有几个文件"，
            // 而不是只显示第一个脏文件名（真源的行为，读起来像 bug）。
            (
                "lithe.editor.gpui.unsavedChangesBatchBody",
                "有 {count} 个文件的修改尚未保存。",
            ),
            // 阶段 11（标签右键菜单）的 9 个菜单项 + 批量确认的"全部保存"按钮。
            // 10 条全是**真源既有**键（`windows/tauri/src/i18n/locale.ts` 的 `files.*` /
            // `tabs.*` / `menu.saveAll`，生成在 `locales/*.yml`）——本菜单**零新增真源键**，
            // 只新增上面那条 `editor.gpui.*`。
            ("lithe.files.copyPath", "复制路径"),
            ("lithe.files.copyRelativePath", "复制相对路径"),
            ("lithe.files.reveal", "在资源管理器中显示"),
            ("lithe.files.openInTerminal", "在终端中打开"),
            ("lithe.tabs.reload", "重新加载"),
            // 「关闭」在上面 `lithe.tabs.close` 已经接过（标签上的关闭按钮 tooltip），不重复。
            ("lithe.tabs.closeOthers", "关闭其他"),
            ("lithe.tabs.closeToRight", "关闭右侧"),
            ("lithe.tabs.closeAll", "全部关闭"),
            ("lithe.menu.saveAll", "全部保存"),
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
            // 右侧工具窗「Spring」视图（`workbench/src/right_tool_window.rs`）：
            // 标题与空态都是 gpui 侧新增（真源 Windows 没有独立的 Spring 面板，
            // 理由逐条写在 `extract-locale.mjs` 的 GPUI_ONLY_KEYS 里）。
            ("lithe.spring.title", "Spring"),
            ("lithe.spring.notDetected", "未检测到 Spring 组件与端点"),
            // 阶段 6 第二半（命令面板，`gpui/crates/workbench/src/command_palette.rs`）。
            // 前三条是**真源既有**的命令面板外壳文案：标题 / 占位 / 空态
            // （`command-palette.tsx:401,449,455`；`locale.ts:7904-7906`，en `:3585-3587`）。
            ("lithe.commandPalette.title", "命令面板"),
            ("lithe.commandPalette.placeholder", "输入命令..."),
            ("lithe.commandPalette.noCommands", "未找到命令"),
            // 动作的分类名（真源 `Action.category` 经
            // `commandPalette.categories.<Category>` 本地化，`utils/action-localization.ts:29-30`）。
            ("lithe.commandPalette.categories.Settings", "设置"),
            ("lithe.commandPalette.categories.View", "视图"),
            // 打开设置（`commandPalette.actions.open-settings.label` = 「首选项：打开设置」，
            // `settings-actions.tsx:155-157`；`locale.ts:8036` / en `:3719`）。
            ("lithe.commandPalette.actions.open-settings.label", "首选项：打开设置"),
            // 终端的显示 / 隐藏两条（`view-actions.tsx:125-145`；`locale.ts:8104-8105` / en `:3801-3802`）。
            ("lithe.commandPalette.actions.toggle-terminal.enableLabel", "视图：显示终端"),
            ("lithe.commandPalette.actions.toggle-terminal.disableLabel", "视图：隐藏终端"),
            // 配色主题：真源是命令面板的二级视图（`commandPalette.actions.color-theme.label`，
            // `settings-actions.tsx` 的 `pushPaletteView("color-theme")` 路径；`locale.ts:8041` / en `:3724`）。
            ("lithe.commandPalette.actions.color-theme.label", "首选项：颜色主题"),
            // 设置里的三个文案：主题分组名 / 状态栏开关的标签与描述
            // （`macos-settings-panels.tsx`；`locale.ts:6644-6645`，en `:2276-2277`）。
            ("lithe.settings.appearance.theme", "主题"),
            ("lithe.settings.appearance.showStatusBar", "显示状态栏"),
            (
                "lithe.settings.appearance.showStatusBarDescription",
                "在底部边缘显示应用控件和状态信息",
            ),
            // 下面 9 条真源里没有，由 `gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 提供
            // （每条都写了理由）：切换主题的两条 + Maven 工具窗的三条 + 状态栏的两条父项标签
            // （状态栏的**描述**复用真源 `settings.appearance.showStatusBarDescription`）。
            ("lithe.appearance.gpui.switchThemeLight", "首选项：切换到浅色主题"),
            ("lithe.appearance.gpui.switchThemeLightDescription", "使用 Lithe Light 配色"),
            ("lithe.appearance.gpui.switchThemeDark", "首选项：切换到深色主题"),
            ("lithe.appearance.gpui.switchThemeDarkDescription", "使用 Lithe Dark 配色"),
            ("lithe.maven.gpui.toggleToolWindowShow", "视图：显示 Maven"),
            ("lithe.maven.gpui.toggleToolWindowHide", "视图：隐藏 Maven"),
            (
                "lithe.maven.gpui.toggleToolWindowDescription",
                "开关右侧的 Maven 工具窗",
            ),
            ("lithe.appearance.gpui.toggleStatusBarShow", "视图：显示状态栏"),
            ("lithe.appearance.gpui.toggleStatusBarHide", "视图：隐藏状态栏"),
            // B4「新窗口」那一颗按钮的能力提示（状态栏左侧临时消息）。**唯一一条 B4 新增键**：
            // 真源的那颗按钮恒可执行（它真的开第二个窗口），所以 catalog 里没有对应的
            // 「尚未接入」句；理由写在 `tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS`。
            (
                "lithe.gpui.newWindowNotWired",
                "尚未接入：在「新窗口」中打开项目需要多窗口支持（缺窗口级句柄路由）。",
            ),
            // B3 的**能力分组提示**（状态栏左侧那 17 句话，`Missing::text_key`）。
            // 全部是 gpui 侧新增键（真源那 89 条菜单项恒可执行，catalog 里没有
            // "尚未接入"这类句子），理由逐条写在 `tools/extract-locale.mjs` 的
            // `GPUI_ONLY_KEYS`；`menu_bar.rs` 的 `MISSING_GROUPS` 是它们的唯一使用点。
            (
                "lithe.gpui.menuMissing.editorEntry",
                "尚未接入：缺编辑器侧的公开入口，本批未授权改动 editor crate。",
            ),
            (
                "lithe.gpui.menuMissing.editorFeatures",
                "尚未接入：编辑器还没有这些功能（行级编辑、转到行、缩略图 / 行号 / 空白字符开关、分屏）。",
            ),
            (
                "lithe.gpui.menuMissing.lspRequests",
                "尚未接入：Java 服务侧还没有这条 LSP 请求（现在只接了定义 / 补全 / 快速修复）。",
            ),
            (
                "lithe.gpui.menuMissing.indexer",
                "尚未接入：还没有索引器（跨文件搜索与快速打开都依赖它）。",
            ),
            (
                "lithe.gpui.menuMissing.debuggerHost",
                "尚未接入：没有进程宿主（调试适配器还没接）。",
            ),
            (
                "lithe.gpui.menuMissing.subsystem",
                "尚未接入：还没有对应子系统。",
            ),
            (
                "lithe.gpui.menuMissing.github",
                "尚未接入：还没有 GitHub 集成（认证 / 仓库 / 拉取请求三条链都不在）。",
            ),
            (
                "lithe.gpui.menuMissing.updater",
                "尚未接入：还没有更新器（检查更新 / 下载 / 安装都没接）。",
            ),
            (
                "lithe.gpui.menuMissing.cloneUi",
                "尚未接入：Core 的 git.write 已含 clone，缺的是 URL / 凭据 / 进度 UI。",
            ),
            (
                "lithe.gpui.menuMissing.newProjectScaffolding",
                "尚未接入：缺项目脚手架生成（真机是新建目录 + 起终端跑 npm create）。",
            ),
            (
                "lithe.gpui.menuMissing.fileLifecycle",
                "尚未接入：缺新建文件 / 空标签页所需的文件生命周期（无标题 buffer + 命名 + 落盘）。",
            ),
            (
                "lithe.gpui.menuMissing.projectLifecycle",
                "尚未接入：缺关闭项目的项目生命周期（销毁项目窗口 + 落盘项目列表）。",
            ),
            (
                "lithe.gpui.menuMissing.localHistory",
                "尚未接入：缺本地历史存储（没有内容仓库，也就没有可看的历史）。",
            ),
            (
                "lithe.gpui.menuMissing.windowRouting",
                "尚未接入：缺窗口级句柄路由（多窗口与窗口生命周期都还没做）。",
            ),
            (
                "lithe.gpui.menuMissing.helpAbout",
                "尚未接入：缺帮助 / 关于的落地页（文档站与外链入口都没接）。",
            ),
            (
                "lithe.gpui.menuMissing.shortcutsView",
                "尚未接入：缺快捷键一览界面（现在只有 keymap，没有能看的清单）。",
            ),
            (
                "lithe.gpui.menuMissing.terminalSplit",
                "尚未接入：终端没有拆分能力（一个终端面板只有一组页签，没有分栏）。",
            ),
            // 主菜单栏（`gpui/crates/workbench/src/menu_bar.rs`）。全部是**真源既有**键
            // （`windows/tauri/src/i18n/locale.ts` 的 `menu.*` 段，本侧前缀 `lithe.`）：
            // 9 个顶级菜单名 + 本轮 v1 真的画出来的项的文案。
            // ⚠️ 99 条 `lithe.menu.*` 早就生成在 `locales/` 里（`extract-locale.mjs` 从真源
            // 抽的），但**没有进过这张表** —— 没进表就不会被测到，键写错只会静默回显键名。
            // 这里只补 v1 真正用到的那些（没用到的先不进表，免得把"没接线的键"当成已接线）。
            ("lithe.menu.file", "文件"),
            ("lithe.menu.edit", "编辑"),
            ("lithe.menu.view", "视图"),
            ("lithe.menu.go", "转到"),
            ("lithe.menu.terminal", "终端"),
            ("lithe.menu.run", "运行"),
            ("lithe.menu.tools", "工具"),
            ("lithe.menu.window", "窗口"),
            ("lithe.menu.help", "帮助"),
            ("lithe.menu.save", "保存"),
            ("lithe.menu.commandPalette", "命令面板"),
            ("lithe.menu.toggleTerminal", "切换终端"),
            // ⚠️ 状态栏那条**复用真源设置项**的文案（真源视图菜单里没有状态栏开关，
            // 它是设置项 `settings.appearance.showStatusBar`，`locale.ts:6644`）——
            // 所以这里不再列一遍 `lithe.settings.appearance.showStatusBar`（上面已有）。
            ("lithe.menu.goToDefinition", "转到定义"),
            ("lithe.menu.theme", "主题"),
            ("lithe.menu.newTerminal", "新建终端"),
            ("lithe.menu.closeTerminal", "关闭终端"),
            ("lithe.menu.preferences", "首选项"),
            ("lithe.menu.minimize", "最小化"),
            ("lithe.menu.maximize", "最大化"),
            ("lithe.menu.toggleFullscreen", "切换全屏"),
            // B1 接进「文件」菜单的一批（`gpui/crates/workbench/src/menu_bar.rs`）。
            // **零新增键、零新增文案**：七条关闭系全是真源 `menu.*` 段既有键，
            // 「打开文件」复用真源 `outline.openFile`（真源 89 条菜单里没有这一项，
            // `menu.*` 段也就没有 `openFile`；复用与「显示状态栏」同一条口径）。
            // 所以 `tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 与两份 yml **都没有动**。
            ("lithe.menu.closeTab", "关闭标签页"),
            ("lithe.menu.closeOtherTabs", "关闭其他标签页"),
            ("lithe.menu.closeAllTabs", "关闭所有标签页"),
            ("lithe.menu.closeSavedTabs", "关闭已保存标签页"),
            ("lithe.menu.closeTabsToLeft", "关闭左侧标签页"),
            ("lithe.menu.closeTabsToRight", "关闭右侧标签页"),
            ("lithe.menu.reopenClosedTab", "重新打开已关闭标签页"),
            ("lithe.outline.openFile", "打开文件"),
            // 左上角图标按钮的 tooltip / 无障碍名（真源 `t("window.menu")` = 「菜单」，
            // `title-bar.tsx:209-219`；`locale.ts:7903`）。
            ("lithe.window.menu", "菜单"),
            // 标题栏项目下拉（`gpui/crates/workbench/src/project_menu.rs`）。7 条全是**真源既有**键
            // （`windows/tauri/src/i18n/locale.ts:6171-6191` 的 `titleProject.*` 段，
            // 生成在 `locales/*.yml:3538-3579`）；本轮**零新增键**。
            // 三条动作行 + 两个分组标题 + 空态 + 触发器的无障碍名（`项目：{project}`，带插值）。
            ("lithe.titleProject.newProject", "新建项目…"),
            ("lithe.titleProject.open", "打开…"),
            ("lithe.titleProject.cloneRepository", "克隆仓库…"),
            ("lithe.titleProject.openProjects", "打开的项目"),
            ("lithe.titleProject.recentProjects", "最近项目"),
            ("lithe.titleProject.noRecentProjects", "没有最近项目"),
            ("lithe.titleProject.trigger", "项目：{project}"),
            // B4「打开其他项目」：文件菜单的「打开文件夹」+ 换项目对话框（6 条 `projectOpen.*`）。
            // 7 条全是**真源既有**键（`lithe.menu.openFolder` 在 `locale.ts` 的菜单段；
            // `projectOpen.*` 在 `locale.ts` 的 `projectOpen` 段，生成在 `locales/*.yml:3526-3537`）
            // —— 本轮**零新增真源键**，`GPUI_ONLY_KEYS` 只多了一条「新窗口」能力提示（见下）。
            ("lithe.menu.openFolder", "打开文件夹"),
            ("lithe.projectOpen.title", "打开项目"),
            ("lithe.projectOpen.where", "你想在哪里打开项目“{project}”？"),
            ("lithe.projectOpen.doNotAskAgain", "不再询问"),
            ("lithe.projectOpen.cancel", "取消"),
            ("lithe.projectOpen.newWindow", "新窗口"),
            ("lithe.projectOpen.thisWindow", "此窗口"),
            // 标题栏分支项 + 分支弹窗（`gpui/crates/workbench/src/branch_panel.rs`）。
            // 9 条全是**真源既有**键（`windows/tauri/src/i18n/locale.ts:6836-6948` 的
            // `git.*` 段，生成在 `locales/*.yml`）——本轮**零新增键**，也没有动
            // `tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS`（Windows 缺的键一条都没用到）。
            // 触发器无障碍名（`git.searchBranchesAria` = 搜索分支，`locale.ts:6917`）。
            ("lithe.git.searchBranchesAria", "搜索分支"),
            // 三个页签（`locale.ts:6836-6838`）+ 页签容器无障碍名（`:6916`）。
            ("lithe.git.repositories", "仓库"),
            ("lithe.git.branches", "分支"),
            ("lithe.git.worktrees", "工作树"),
            ("lithe.git.selectorSections", "Git 选择分区"),
            // 搜索框 placeholder（`locale.ts:6907`）。
            ("lithe.git.searchBranches", "搜索分支..."),
            // 计数徽章：单复数两键**同值**（`locale.ts:6910,6911`），所以中文下看不出单复数，
            // 两个键都要接线（`1 个分支` 走单数键，与真源 `branchCount` / `branchesCount` 一致）。
            ("lithe.git.branchCount", "{count} 个分支"),
            ("lithe.git.branchesCount", "{count} 个分支"),
            // 空态两句（`locale.ts:6920,6934`）：有搜索词 / 没有搜索词。
            ("lithe.git.noMatchingBranches", "没有匹配的分支"),
            ("lithe.git.noBranchesFound", "未找到分支"),
            // 行内「当前」徽章（`locale.ts:6948`）与底栏「刷新」（`:6944`）。
            ("lithe.git.current", "当前"),
            ("lithe.git.refresh", "刷新"),
            // 左栏「源代码管理」（`gpui/crates/git/src/changes_view.rs`，阶段 11）。
            // 同样**零新增键**：视图用到的每一条真源里都已经有（`locale.ts` 的 `git.*` 段与
            // `git.setup.*` 段），所以 `tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 没有动。
            // 标题栏（`git-view.tsx:707,474,481`）。
            ("lithe.workbench.sourceControl", "源代码管理"),
            ("lithe.git.actions", "Git 操作"),
            ("lithe.git.refreshStatus", "刷新状态"),
            // 加载 / 失败两态（`git-view.tsx:663-685`）。
            ("lithe.git.loadingGitStatus", "正在加载 Git 状态"),
            ("lithe.git.statusLoadFailed", "无法加载 Git 数据，请重试刷新仓库。"),
            // 分类头（`git-status-panel.tsx:858-864`）与树容器 aria（`:1190`）。
            ("lithe.git.tracked", "已跟踪"),
            ("lithe.git.untracked", "未跟踪"),
            ("lithe.git.trackedFiles", "已跟踪文件"),
            ("lithe.git.untrackedFiles", "未跟踪文件"),
            // 工具行（`git-status-panel.tsx:1157-1180`）。
            ("lithe.git.stageAllChanges", "暂存所有更改"),
            ("lithe.git.unstageAllChanges", "取消暂存所有更改"),
            // 行内暂存按钮（`git-status-file-item.tsx:150`）。
            ("lithe.git.stageFileNamed", "暂存 {name}"),
            ("lithe.git.unstageFileNamed", "取消暂存 {name}"),
            // 空态（`git-status-panel.tsx:1220-1226`、`git-repository-empty-state.tsx:94-118`）。
            ("lithe.git.workingTreeClean", "工作区干净"),
            ("lithe.git.setup.notRepository", "此项目尚未初始化 Git 仓库"),
            (
                "lithe.git.setup.initializeHint",
                "初始化 Git 后即可跟踪更改并创建首次提交。",
            ),
            ("lithe.git.setup.initialize", "初始化 Git 仓库"),
            ("lithe.git.setup.noCommits", "当前分支尚无提交"),
            (
                "lithe.git.setup.firstCommit",
                "在“更改”面板暂存文件、填写提交消息，然后创建首次提交。",
            ),
            // 阶段 15（设置 →「Git」页的**提交身份**）：全部是**真源既有**键
            // （`windows/tauri/src/i18n/locale.ts` 的 `git.setup.*` 段，渲染点
            // `components/git-identity-settings.tsx`）。本页**零新增 locale 键** ——
            // 包括那个会写回 Git 配置的开关（`settings.git.confirmDiscard*` 也在真源里）。
            ("lithe.git.setup.identity", "提交身份"),
            (
                "lithe.git.setup.identityDescription",
                "Git 会将此姓名和邮箱记录在新提交中，不会修改已有提交。",
            ),
            ("lithe.git.setup.scope", "配置范围"),
            ("lithe.git.setup.local", "当前仓库"),
            ("lithe.git.setup.global", "全局 Git 配置"),
            (
                "lithe.git.setup.globalDescription",
                "全局身份会用于其他仓库，仓库自身的配置可以覆盖它。",
            ),
            (
                "lithe.git.setup.localDescription",
                "仓库身份会覆盖继承的全局值。清除覆盖后将使用继承的配置。",
            ),
            (
                "lithe.git.setup.initializeFirst",
                "请先在 Git 面板初始化项目，再保存当前仓库的身份配置。",
            ),
            ("lithe.git.setup.name", "提交者姓名"),
            ("lithe.git.setup.email", "提交者邮箱"),
            ("lithe.git.setup.save", "保存"),
            ("lithe.git.setup.clear", "清除覆盖"),
            ("lithe.git.setup.effective", "当前生效值"),
            ("lithe.git.setup.unconfigured", "尚未配置有效值"),
            ("lithe.git.setup.saved", "Git 配置已保存"),
            (
                "lithe.git.setup.separateSave",
                "每个字段单独保存。切换范围时会重新加载已保存的值。",
            ),
            ("lithe.git.setup.reload", "重新加载 Git 配置"),
            ("lithe.git.setup.loading", "正在检查 Git 仓库…"),
            ("lithe.git.setup.readFailed", "Git 无法读取此仓库"),
            // 「Git」页里唯一一个**真有消费方**的设置开关（阶段 15）：
            // 分组标题 + 行标签 + 行描述（`tabs/git-settings.tsx:93-106`）。
            ("lithe.settings.git.integration", "集成"),
            ("lithe.settings.git.confirmDiscard", "丢弃前确认"),
            (
                "lithe.settings.git.confirmDiscardDescription",
                "丢弃文件或仓库更改前显示确认提示",
            ),
            // 提交面板（`git-commit-panel.tsx:148-172,281-389`）。
            ("lithe.git.commitMessagePlaceholder", "提交说明..."),
            ("lithe.git.filesStaged", "已暂存 {count} 个文件"),
            ("lithe.git.noFilesStaged", "没有已暂存的文件"),
            ("lithe.git.selectFilesToCommit", "请选择要提交的文件。"),
            ("lithe.git.enterCommitMessage", "输入提交说明："),
            ("lithe.git.commit", "提交"),
            ("lithe.git.committing", "正在提交..."),
            ("lithe.git.changesCommitted", "更改已提交"),
            (
                "lithe.git.finishOperationBeforeCommit",
                "请先完成或中止当前 Git 操作，再提交文件。",
            ),
            // 操作横幅（`git-operation-banner.tsx:57-137`）。
            ("lithe.git.mergeInProgress", "合并进行中"),
            ("lithe.git.rebaseInProgress", "变基进行中"),
            ("lithe.git.cherryPickInProgress", "拣选进行中"),
            ("lithe.git.revertInProgress", "还原进行中"),
            ("lithe.git.operationStep", "第 {step} / {total} 步。"),
            (
                "lithe.git.resolveConflicts",
                "解决 {count} 个冲突文件，暂存后继续。",
            ),
            (
                "lithe.git.conflictsResolved",
                "所有冲突已解决。继续完成，或中止以撤销。",
            ),
            (
                "lithe.git.resolveConflictsFirst",
                "请先解决冲突：{paths}",
            ),
            ("lithe.git.continueMerge", "继续合并"),
            ("lithe.git.continueRebase", "继续变基"),
            ("lithe.git.continueCherryPick", "继续拣选"),
            ("lithe.git.continueRevert", "继续还原"),
            ("lithe.git.skipCommit", "跳过提交"),
            ("lithe.git.abort", "中止"),
            // 「更多」菜单的「丢弃全部更改」+ 确认框（`git-actions-menu.tsx:146-153,308`）。
            ("lithe.git.discardAllChanges", "丢弃全部更改"),
            (
                "lithe.git.discardChangesConfirm",
                "丢弃所有未暂存的更改吗？此操作无法撤销。",
            ),
            ("lithe.git.rollback", "回滚"),
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
    ///
    /// 阶段 14 追加的三条是「终端 → 默认 Shell」下拉里的选项名：`Shell`、`PowerShell`、`WSL`
    /// 在真源的两份 catalog 里本来就是同一个词（`settings.mac.shell` / `shellPowerShell` /
    /// `shellWsl`），没有可翻译的内容；`命令提示符`（`shellCommandPrompt`）有英文对照
    /// （`Command Prompt`），所以**不进**这张表。
    /// 同批追加的 `settings.tabs.lsp` / `settings.tabs.git` 同理：LSP 与 Git 是中英同形词。
    const SAME_IN_BOTH_LOCALES: &[&str] = &[
        "lithe.workbench.maven",
        "lithe.maven.title",
        "lithe.settings.mac.shell",
        "lithe.settings.mac.shellPowerShell",
        "lithe.settings.mac.shellWsl",
        "lithe.settings.tabs.lsp",
        "lithe.settings.tabs.git",
        // 产品名，中英同形（右侧工具窗「Spring」视图的标题）。
        "lithe.spring.title",
    ];
}
