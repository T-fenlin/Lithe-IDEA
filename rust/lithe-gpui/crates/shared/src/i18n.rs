use gpui::SharedString;

pub fn tr(key: &str) -> SharedString {
    SharedString::from(rust_i18n::t!(key).to_string())
}

pub fn tr_args(key: &str, args: &[(&str, &str)]) -> SharedString {
    let template = rust_i18n::t!(key).to_string();
    SharedString::from(interpolate(&template, args))
}

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

    #[test]
    fn replaces_brace_placeholders() {
        assert_eq!(interpolate("{count} 个空格", &[("count", "4")]), "4 个空格");
    }

    #[test]
    fn replaces_every_occurrence() {
        assert_eq!(
            interpolate("{a}-{b}-{a}", &[("a", "x"), ("b", "y")]),
            "x-y-x"
        );
    }

    #[test]
    fn keeps_unknown_placeholders() {
        assert_eq!(interpolate("{missing}", &[("count", "4")]), "{missing}");
    }

    #[test]
    fn every_wired_key_resolves_in_both_locales() {
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
            ("lithe.quickOpen.searchFiles", "输入以搜索文件..."),
            ("lithe.quickOpen.noMatch", "未找到匹配的文件"),
            ("lithe.menu.quickOpen", "快速打开"),
            ("lithe.searchEverywhere.scope.all", "全部"),
            ("lithe.searchEverywhere.scope.classes", "类"),
            ("lithe.searchEverywhere.scope.files", "文件"),
            ("lithe.searchEverywhere.scope.symbols", "符号"),
            ("lithe.searchEverywhere.scope.actions", "操作"),
            (
                "lithe.searchEverywhere.noMatches.actions",
                "操作中没有匹配项",
            ),
            (
                "lithe.searchEverywhere.noMatchingCommands",
                "没有匹配的命令",
            ),
            ("lithe.searchEverywhere.typeSlash", "输入 / 查看命令"),
            ("lithe.searchEverywhere.scope.text", "文本"),
            (
                "lithe.searchEverywhere.noMatches.all",
                "全部范围内没有匹配项",
            ),
            ("lithe.searchEverywhere.noMatches.classes", "类中没有匹配项"),
            ("lithe.searchEverywhere.noMatches.files", "文件中没有匹配项"),
            (
                "lithe.searchEverywhere.noMatches.symbols",
                "符号中没有匹配项",
            ),
            ("lithe.searchEverywhere.noMatches.text", "文本中没有匹配项"),
            ("lithe.menu.globalSearch", "全局搜索"),
            ("lithe.search.gpui.replaceInFiles", "在项目中替换"),
            (
                "lithe.search.gpui.replaceEmptyDescription",
                "输入要查找的文本与替换文本，然后选择「预览」。",
            ),
            (
                "lithe.search.gpui.replacePreviewCounts",
                "{files} 个文件，{matches} 处匹配",
            ),
            ("lithe.search.gpui.before", "替换前"),
            ("lithe.search.gpui.after", "替换后"),
            ("lithe.search.gpui.preserveCase", "保留大小写"),
            ("lithe.search.gpui.preview", "预览"),
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
            ("lithe.diagnostics.problemCount", "{count} 个问题"),
            (
                "lithe.diagnostics.severityCount",
                "{label} ({visible}/{total})",
            ),
            ("lithe.diagnostics.errors", "错误"),
            ("lithe.diagnostics.warnings", "警告"),
            ("lithe.diagnostics.info", "信息"),
            ("lithe.diagnostics.empty", "未检测到问题"),
            ("lithe.diagnostics.noMatch", "没有符合当前筛选条件的问题"),
            ("lithe.diagnostics.resetFilters", "重置筛选"),
            ("lithe.workbench.changes", "更改"),
            ("lithe.workbench.search", "搜索"),
            ("lithe.workbench.terminal", "终端"),
            ("lithe.workbench.settings", "设置"),
            ("lithe.workbench.maven", "Maven"),
            ("lithe.extensions.title", "扩展"),
            ("lithe.extensions.searchPlaceholder", "搜索扩展"),
            ("lithe.extensions.availableCount", "{count} 个可用"),
            ("lithe.extensions.all", "全部"),
            ("lithe.extensions.themes", "主题"),
            ("lithe.extensions.icons", "图标"),
            ("lithe.extensions.contributionLine", "{label}：{items}"),
            (
                "lithe.extensions.languageUninstallHint",
                "卸载后这些文件打开时是纯文本。",
            ),
            ("lithe.extensions.category.fonts", "字体"),
            ("lithe.extensions.category.debug", "调试"),
            ("lithe.extensions.category.lsp", "LSP"),
            ("lithe.extensions.languages", "语言"),
            ("lithe.extensions.builtIn", "内置"),
            ("lithe.extensions.uninstalled", "已卸载"),
            ("lithe.extensions.current", "当前"),
            ("lithe.extensions.use", "使用"),
            ("lithe.extensions.uninstall", "卸载"),
            ("lithe.extensions.reinstall", "重新安装"),
            (
                "lithe.extensions.fontRestartHint",
                "字体在下次启动应用后生效。",
            ),
            ("lithe.extensions.byPublisher", "作者：{publisher}"),
            ("lithe.extensions.contributions", "贡献项"),
            ("lithe.extensions.loadingExtension", "正在加载扩展"),
            ("lithe.extensions.loadFailed", "加载扩展列表失败。"),
            ("lithe.notifications.title", "通知"),
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
            (
                "lithe.notifications.core.notSupported",
                "当前版本不支持这个操作：{detail}",
            ),
            (
                "lithe.notifications.core.runtimeMissing",
                "缺少运行时：{detail}",
            ),
            (
                "lithe.notifications.core.processStartFailed",
                "无法启动进程：{detail}",
            ),
            (
                "lithe.notifications.core.processFailed",
                "进程执行失败：{detail}",
            ),
            (
                "lithe.notifications.core.parseFailed",
                "无法解析结果：{detail}",
            ),
            ("lithe.notifications.core.timedOut", "操作超时：{detail}"),
            ("lithe.notifications.core.unknown", "操作失败：{detail}"),
            ("lithe.git.log.filesCount", "{count} 个文件"),
            ("lithe.git.log.filterPlaceholder", "{field} 筛选"),
            ("lithe.git.log.logLabel", "日志：{name}"),
            ("lithe.titleProject.closeProject", "关闭项目 {name}"),
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
            ("lithe.settings.tabs.editor", "编辑器"),
            ("lithe.settings.mac.display", "显示"),
            ("lithe.settings.mac.fontSize", "字体大小"),
            ("lithe.settings.mac.indentation", "缩进"),
            ("lithe.settings.mac.tabWidth", "制表符宽度"),
            ("lithe.settings.mac.spaces", "个空格"),
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
            ("lithe.settings.tabs.keyboard", "快捷键"),
            ("lithe.settings.tabs.lsp", "LSP"),
            ("lithe.settings.tabs.git", "Git"),
            ("lithe.settings.tabs.logs", "日志"),
            ("lithe.settings.tabs.updates", "更新"),
            ("lithe.settings.project.title", "项目 · JDK 与 Maven"),
            ("lithe.settings.run.title", "运行配置"),
            ("lithe.settings.gpui.languageEnglish", "英语"),
            ("lithe.settings.gpui.languageChinese", "简体中文"),
            (
                "lithe.settings.gpui.languageRestartDescription",
                "切换语言会立即重启 Lithe。",
            ),
            ("lithe.settings.gpui.restoreDefaultsOpen", "恢复默认设置…"),
            ("lithe.settings.gpui.restoreDefaultsTitle", "恢复默认设置？"),
            (
                "lithe.settings.gpui.restoreDefaultsBody",
                "所有设置都会回到默认值。",
            ),
            (
                "lithe.settings.gpui.editorFontSizeDescription",
                "调整代码编辑器的字号。界面字号在外观页，终端字号在终端页。",
            ),
            ("lithe.settings.gpui.valueNotOverridden", "默认（不覆盖）"),
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
            ("lithe.settings.gpui.sourceFromEnv", "来自 {name} 环境变量"),
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
            (
                "lithe.settings.gpui.jdkBelowLanguageServiceMinimum",
                "低于语言服务最低要求（JDT LS 需要 JDK {minimum} 或更新版本），语言服务无法启动。",
            ),
            (
                "lithe.settings.gpui.mavenOverrideNotEffective",
                "尚未生效：gpui 侧还没有执行 Maven 的通路（不执行 mvn，也没采购 Core 的启动计划），这个值现在不改变任何行为。",
            ),
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
                "启动入口在「运行」工具窗（左侧活动栏的 ▶）与标题栏的 ▶ / 🐞：那里能选配置、起进程、看输出、停止运行。本页仍然只读——配置编辑器（改参数、环境变量、JVM 参数）还没有落点。",
            ),
            (
                "lithe.run.gpui.debugUnsupported",
                "这条配置的启动计划里没有调试参数：只有 JVM 配置（java / spring-boot.maven）会以 JDWP 方式启动并等待调试器。",
            ),
            (
                "lithe.run.gpui.noWorkspace",
                "还没有打开项目：运行配置从工作区里识别。",
            ),
            (
                "lithe.run.gpui.noConfiguration",
                "没有可运行的配置：先在列表里选一条（列表为空说明项目里没识别到可运行目标）。",
            ),
            ("lithe.run.gpui.planFailed", "启动失败：{reason}"),
            (
                "lithe.run.gpui.debugWaiting",
                "等待调试器接入（127.0.0.1:{port}）",
            ),
            ("lithe.run.gpui.exitCode", "已退出（退出码 {code}）"),
            (
                "lithe.run.gpui.outputTruncated",
                "输出超过 5000 行，只保留最后 5000 行。",
            ),
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
            ("lithe.editor.gpui.discardChanges", "放弃修改"),
            (
                "lithe.editor.gpui.unsavedChangesBody",
                "对“{name}”的修改尚未保存。",
            ),
            (
                "lithe.editor.gpui.unsavedChangesBatchBody",
                "有 {count} 个文件的修改尚未保存。",
            ),
            ("lithe.files.copyPath", "复制路径"),
            ("lithe.files.copyRelativePath", "复制相对路径"),
            ("lithe.editor.gpui.revealInProjectTree", "在项目树中显示"),
            ("lithe.files.reveal", "在资源管理器中显示"),
            ("lithe.files.openInTerminal", "在终端中打开"),
            ("lithe.tabs.reload", "重新加载"),
            ("lithe.tabs.closeOthers", "关闭其他"),
            ("lithe.tabs.closeToRight", "关闭右侧"),
            ("lithe.tabs.closeAll", "全部关闭"),
            ("lithe.menu.saveAll", "全部保存"),
            (
                "lithe.editor.saveFailed",
                "无法保存“{name}”。请检查文件是否可写，然后重试。",
            ),
            (
                "lithe.editor.autoSaveFailed",
                "无法自动保存“{name}”。更改仍保留在编辑器中。",
            ),
            ("lithe.unsavedChanges.title", "未保存的更改"),
            ("lithe.ui.save", "保存"),
            ("lithe.navigation.definition", "定义"),
            ("lithe.navigation.noTargetFound", "未找到{target}。"),
            ("lithe.editor.goToLineColumn", "转到行和列"),
            ("lithe.navigation.goToLinePrompt", "转到行"),
            ("lithe.ui.ok", "确定"),
            ("lithe.settings.editor.wordWrap", "自动换行"),
            (
                "lithe.settings.editor.wordWrapDescription",
                "换行显示超出视口宽度的行",
            ),
            ("lithe.settings.mac.files", "文件"),
            ("lithe.settings.mac.autoSave", "自动保存更改的文件"),
            (
                "lithe.settings.editor.autoSaveDescription",
                "编辑时自动保存文件",
            ),
            ("lithe.menu.goToLine", "转到行"),
            ("lithe.menu.toggleWordWrap", "切换自动换行"),
            ("lithe.menu.showHover", "显示悬停信息"),
            ("lithe.maven.title", "Maven"),
            ("lithe.maven.notDetected", "未检测到 Maven 项目"),
            ("lithe.notifications.empty", "暂无通知。"),
            ("lithe.extensions.noneFound", "未找到扩展。"),
            ("lithe.commandPalette.close", "关闭命令面板"),
            ("lithe.spring.title", "Spring"),
            ("lithe.spring.notDetected", "未检测到 Spring 组件与端点"),
            ("lithe.commandPalette.title", "命令面板"),
            ("lithe.commandPalette.placeholder", "输入命令..."),
            ("lithe.commandPalette.noCommands", "未找到命令"),
            ("lithe.commandPalette.categories.Settings", "设置"),
            ("lithe.commandPalette.categories.View", "视图"),
            (
                "lithe.commandPalette.actions.open-settings.label",
                "首选项：打开设置",
            ),
            (
                "lithe.commandPalette.actions.toggle-terminal.enableLabel",
                "视图：显示终端",
            ),
            (
                "lithe.commandPalette.actions.toggle-terminal.disableLabel",
                "视图：隐藏终端",
            ),
            (
                "lithe.commandPalette.actions.color-theme.label",
                "首选项：颜色主题",
            ),
            ("lithe.settings.appearance.theme", "主题"),
            ("lithe.settings.appearance.showStatusBar", "显示状态栏"),
            (
                "lithe.settings.appearance.showStatusBarDescription",
                "在底部边缘显示应用控件和状态信息",
            ),
            (
                "lithe.appearance.gpui.switchThemeLight",
                "首选项：切换到浅色主题",
            ),
            (
                "lithe.appearance.gpui.switchThemeLightDescription",
                "使用 Lithe Light 配色",
            ),
            (
                "lithe.appearance.gpui.switchThemeDark",
                "首选项：切换到深色主题",
            ),
            (
                "lithe.appearance.gpui.switchThemeDarkDescription",
                "使用 Lithe Dark 配色",
            ),
            ("lithe.maven.gpui.toggleToolWindowShow", "视图：显示 Maven"),
            ("lithe.maven.gpui.toggleToolWindowHide", "视图：隐藏 Maven"),
            (
                "lithe.maven.gpui.toggleToolWindowDescription",
                "开关右侧的 Maven 工具窗",
            ),
            (
                "lithe.appearance.gpui.toggleStatusBarShow",
                "视图：显示状态栏",
            ),
            (
                "lithe.appearance.gpui.toggleStatusBarHide",
                "视图：隐藏状态栏",
            ),
            (
                "lithe.gpui.newWindowNotWired",
                "尚未接入：在「新窗口」中打开项目需要多窗口支持（缺窗口级句柄路由）。",
            ),
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
            ("lithe.menu.goToDefinition", "转到定义"),
            ("lithe.menu.theme", "主题"),
            ("lithe.menu.newTerminal", "新建终端"),
            ("lithe.menu.closeTerminal", "关闭终端"),
            ("lithe.menu.preferences", "首选项"),
            ("lithe.menu.minimize", "最小化"),
            ("lithe.menu.maximize", "最大化"),
            ("lithe.menu.toggleFullscreen", "切换全屏"),
            ("lithe.menu.closeTab", "关闭标签页"),
            ("lithe.menu.closeOtherTabs", "关闭其他标签页"),
            ("lithe.menu.closeAllTabs", "关闭所有标签页"),
            ("lithe.menu.closeSavedTabs", "关闭已保存标签页"),
            ("lithe.menu.closeTabsToLeft", "关闭左侧标签页"),
            ("lithe.menu.closeTabsToRight", "关闭右侧标签页"),
            ("lithe.menu.reopenClosedTab", "重新打开已关闭标签页"),
            ("lithe.outline.openFile", "打开文件"),
            ("lithe.window.menu", "菜单"),
            ("lithe.titleProject.newProject", "新建项目…"),
            ("lithe.titleProject.open", "打开…"),
            ("lithe.titleProject.cloneRepository", "克隆仓库…"),
            ("lithe.titleProject.openProjects", "打开的项目"),
            ("lithe.titleProject.recentProjects", "最近项目"),
            ("lithe.titleProject.noRecentProjects", "没有最近项目"),
            ("lithe.titleProject.trigger", "项目：{project}"),
            ("lithe.menu.openFolder", "打开文件夹"),
            ("lithe.projectOpen.title", "打开项目"),
            ("lithe.projectOpen.where", "你想在哪里打开项目“{project}”？"),
            ("lithe.projectOpen.doNotAskAgain", "不再询问"),
            ("lithe.projectOpen.cancel", "取消"),
            ("lithe.projectOpen.newWindow", "新窗口"),
            ("lithe.projectOpen.thisWindow", "此窗口"),
            ("lithe.git.searchBranchesAria", "搜索分支"),
            ("lithe.git.repositories", "仓库"),
            ("lithe.git.branches", "分支"),
            ("lithe.git.worktrees", "工作树"),
            ("lithe.git.selectorSections", "Git 选择分区"),
            ("lithe.git.searchBranches", "搜索分支..."),
            ("lithe.git.branchCount", "{count} 个分支"),
            ("lithe.git.branchesCount", "{count} 个分支"),
            ("lithe.git.noMatchingBranches", "没有匹配的分支"),
            ("lithe.git.noBranchesFound", "未找到分支"),
            ("lithe.git.current", "当前"),
            ("lithe.git.refresh", "刷新"),
            ("lithe.git.fetch", "获取"),
            ("lithe.git.log.updateBranch", "更新"),
            ("lithe.git.push", "推送"),
            ("lithe.git.fetchingChanges", "正在获取更改..."),
            ("lithe.git.pulling", "正在拉取"),
            ("lithe.git.pushingChanges", "正在推送更改..."),
            ("lithe.git.changesFetched", "已成功获取。"),
            ("lithe.git.pulledChanges", "更改已成功拉取。"),
            ("lithe.git.changesPushed", "更改已成功推送。"),
            ("lithe.git.fetchFailed", "获取更改失败。"),
            ("lithe.git.pushFailed", "推送更改失败。"),
            ("lithe.git.operationFailed", "Git 操作失败"),
            (
                "lithe.git.pullResult.noUpstream",
                "当前分支没有上游分支，请先设置上游分支。",
            ),
            ("lithe.git.pullResult.upToDate", "当前分支已是最新状态。"),
            (
                "lithe.git.pullResult.gitRejectedPull",
                "Git 拒绝了此次拉取。",
            ),
            ("lithe.git.pullResult.fetchFailed", "获取失败：{error}"),
            (
                "lithe.git.pullResult.preflightFailed",
                "拉取安全检查失败：{error}",
            ),
            (
                "lithe.git.pullResult.strategyRequired",
                "分支已经分叉，请打开源代码管理并选择合并或变基。",
            ),
            ("lithe.git.choosePullStrategy", "选择拉取策略"),
            (
                "lithe.git.pullDivergedMessage",
                "当前分支与 {upstream} 已经分叉。请选择如何协调它们。",
            ),
            (
                "lithe.git.pullStrategyHint",
                "合并会创建一个合并提交；变基会在上游分支上重放本地提交。",
            ),
            ("lithe.git.merge", "合并"),
            ("lithe.git.rebase", "变基"),
            (
                "lithe.git.uncommittedWouldOverwrite",
                "未提交的更改会被覆盖：{listed}{more}。请先贮藏或提交。",
            ),
            (
                "lithe.git.pullResult.dirty",
                "拉取前请先提交或贮藏本地更改。",
            ),
            ("lithe.git.checkout", "检出"),
            ("lithe.git.stashChanges", "贮藏更改"),
            ("lithe.git.switchingTo", "正在切换到 {branch}"),
            ("lithe.git.actionFailed", "{action} 失败"),
            ("lithe.git.conflictMarkersRemain", "仍存在冲突标记：{paths}"),
            (
                "lithe.git.verifyConflictMarkersFailed",
                "无法验证冲突标记。请重试后再提交。",
            ),
            ("lithe.git.openFile", "打开文件"),
            ("lithe.workbench.sourceControl", "源代码管理"),
            ("lithe.git.actions", "Git 操作"),
            ("lithe.git.refreshStatus", "刷新状态"),
            ("lithe.git.loadingGitStatus", "正在加载 Git 状态"),
            (
                "lithe.git.statusLoadFailed",
                "无法加载 Git 数据，请重试刷新仓库。",
            ),
            ("lithe.git.tracked", "已跟踪"),
            ("lithe.git.untracked", "未跟踪"),
            ("lithe.git.trackedFiles", "已跟踪文件"),
            ("lithe.git.untrackedFiles", "未跟踪文件"),
            ("lithe.git.stageAllChanges", "暂存所有更改"),
            ("lithe.git.unstageAllChanges", "取消暂存所有更改"),
            ("lithe.git.stageFileNamed", "暂存 {name}"),
            ("lithe.git.unstageFileNamed", "取消暂存 {name}"),
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
            ("lithe.settings.git.integration", "集成"),
            ("lithe.settings.git.confirmDiscard", "丢弃前确认"),
            (
                "lithe.settings.git.confirmDiscardDescription",
                "丢弃文件或仓库更改前显示确认提示",
            ),
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
            ("lithe.git.resolveConflictsFirst", "请先解决冲突：{paths}"),
            ("lithe.git.continueMerge", "继续合并"),
            ("lithe.git.continueRebase", "继续变基"),
            ("lithe.git.continueCherryPick", "继续拣选"),
            ("lithe.git.continueRevert", "继续还原"),
            ("lithe.git.skipCommit", "跳过提交"),
            ("lithe.git.abort", "中止"),
            ("lithe.git.discardAllChanges", "丢弃全部更改"),
            (
                "lithe.git.discardChangesConfirm",
                "丢弃所有未暂存的更改吗？此操作无法撤销。",
            ),
            ("lithe.git.rollback", "回滚"),
            ("lithe.search.gpui.queryPlaceholder", "搜索文件和内容"),
            (
                "lithe.search.gpui.indexWarming",
                "正在建立搜索索引，首次搜索可能较慢。",
            ),
            (
                "lithe.search.gpui.indexWarmFailed",
                "搜索索引未提前建立完成，首次搜索可能较慢（搜索本身仍然可用）。",
            ),
            ("lithe.welcome.projects", "项目"),
            ("lithe.welcome.searchProjects", "搜索项目"),
            ("lithe.welcome.open", "打开"),
            ("lithe.welcome.noRecentProjects", "暂无最近项目"),
            ("lithe.welcome.openFolderHint", "打开文件夹以开始使用。"),
            ("lithe.welcome.removeRecent", "从最近项目中移除 {name}"),
            ("lithe.welcome.gpui.noMatchingProjects", "没有匹配的项目"),
            ("lithe.debugger.gpui.toolWindowTitle", "调试"),
            ("lithe.debugger.gpui.transportLabel", "适配器通道"),
            ("lithe.debugger.gpui.sessionLabel", "调试会话"),
            ("lithe.debugger.gpui.absent", "未实现"),
            ("lithe.debugger.gpui.present", "已在"),
            (
                "lithe.debugger.gpui.startBlockedNoTransport",
                "已有会话，但平台侧的调试适配器通道尚未实现（H10）：收不到任何 DAP 事件，所以启动没有意义。",
            ),
            (
                "lithe.debugger.gpui.startBlockedNoSession",
                "已有适配器通道，但还没有调试会话（H11）：需要 `vscode.java.startDebugSession` 给出的适配器与端口。",
            ),
            (
                "lithe.debugger.gpui.startBlockedBoth",
                "调试会话与适配器通道都还没有：平台侧的 transport（H10）与 JDT 调试会话启动（H11）都未接。",
            ),
            (
                "lithe.debugger.gpui.h10Note",
                "缺 H10：平台侧调试适配器 transport（socket / 子进程 / 适配器进程生命周期）。Core 明确它归平台所有（`debug/engine.rs:1`），而 gpui 侧一个实现都没有。",
            ),
            (
                "lithe.debugger.gpui.h11Note",
                "缺 H11：Java Debug Server 的启动与端口协商（`vscode.java.startDebugSession`）。它不在 Core 的命令面里（`run/src/lib.rs:34` 把它列为下一步入口）。",
            ),
            ("lithe.debugger.gpui.emptyTitle", "调试"),
            (
                "lithe.debugger.gpui.breakpointSummary",
                "已设 {count} 个断点",
            ),
            ("lithe.debugger.gpui.steppingFilters", "步进过滤器"),
            ("lithe.debugger.gpui.filtersIdle", "尚未读取步进过滤器。"),
            ("lithe.debugger.gpui.filtersLoading", "正在读取步进过滤器…"),
            ("lithe.debugger.gpui.filtersFailed", "读取步进过滤器失败："),
            (
                "lithe.debugger.gpui.filtersShapeUnknown",
                "Core 返回的步进过滤器形状不认识（契约可能变了）。",
            ),
            (
                "lithe.debugger.gpui.filtersMissingData",
                "Core 说成功但没有返回数据。",
            ),
            ("lithe.debugger.gpui.filterClassPatterns", "类名模式"),
            ("lithe.debugger.gpui.skipSynthetics", "跳过合成方法"),
            ("lithe.debugger.gpui.skipConstructors", "跳过构造方法"),
            (
                "lithe.debugger.gpui.skipStaticInitializers",
                "跳过静态初始化块",
            ),
            (
                "lithe.debugger.gpui.hideFilteredStackFrames",
                "折叠匹配的栈帧",
            ),
            ("lithe.debugger.gpui.pausedWithReason", "已暂停 · {reason}"),
        ];

        static LOCALE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCALE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous = rust_i18n::locale().to_string();
        struct RestoreLocale(String);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                rust_i18n::set_locale(&self.0);
            }
        }
        let _restore = RestoreLocale(previous);

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

    const SAME_IN_BOTH_LOCALES: &[&str] = &[
        "lithe.workbench.maven",
        "lithe.maven.title",
        "lithe.settings.mac.shell",
        "lithe.settings.mac.shellPowerShell",
        "lithe.settings.mac.shellWsl",
        "lithe.settings.tabs.lsp",
        "lithe.settings.tabs.git",
        "lithe.spring.title",
        "lithe.extensions.category.lsp",
        "lithe.diagnostics.severityCount",
    ];
}
