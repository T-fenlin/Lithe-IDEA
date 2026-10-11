//! 应用图标常量表 —— 全部取自本仓库 `assets/icons/idea/**`，由 `tools/` 侧一次性核对生成，不在运行时回落到别的图标集。
//!
//! 命名口径：
//! * `FILE_*` —— 按文件类型取的图标（`fileTypes/` 与 `legacy/files/` 两处）；
//! * `EXPUI_*` —— 新版 UI 图标集（`expui/`），与同名的旧版图标是不同的图；
//! * 其余按文件名转 `SCREAMING_SNAKE`；撞名时补顶层目录前缀。
//!
//! `ALL` 供 `lithe-gpui-app` 的 `assets` 测试逐条取字节校验路径，新增常量请一并登记进去，否则路径写错不会有任何报错。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdeaIcon {
    pub light: &'static str,
    pub dark: &'static str,
    pub has_dark: bool,
}

impl IdeaIcon {
    pub const fn path(&self, dark: bool) -> &'static str {
        if dark { self.dark } else { self.light }
    }
}

pub const ALL: [IdeaIcon; 186] = [
    ANNOTATION_ICON,
    ARROW_CLOCKWISE_ICON,
    ARROW_COUNTER_CLOCKWISE_ICON,
    ARROW_DOWN_ICON,
    ARROW_LEFT_ICON,
    ARROW_RIGHT_ICON,
    ARROW_UP_ICON,
    ATTACH_TO_PROCESS_ICON,
    BELL_ICON,
    BOOK_OPEN_ICON,
    BUG_ICON,
    CARET_DOWN_ICON,
    CARET_LEFT_ICON,
    CARET_RIGHT_ICON,
    CHECK_CIRCLE_ICON,
    CHECK_ICON,
    CLASS_ICON,
    CLOCK_COUNTER_CLOCKWISE_ICON,
    COLUMNS_ICON,
    COMMIT_ICON,
    COPY_ICON,
    DATABASE_ICON,
    DB_DISABLED_BREAKPOINT_ICON,
    DB_MUTED_BREAKPOINT_ICON,
    DB_SET_BREAKPOINT_ICON,
    DB_VERIFIED_BREAKPOINT_ICON,
    DOTS_THREE_ICON,
    DOWNLOAD_ICON,
    ENUM_ICON,
    EVALUATE_EXPRESSION_ICON,
    EXCLUDE_ROOT_ICON,
    EXECUTE_ICON,
    EXPUI_COLLAPSE_ALL_ICON,
    EXPUI_COMMIT_ICON,
    EXPUI_COVERAGE_ICON,
    EXPUI_DEBUG_ICON,
    EXPUI_EDIT_ICON,
    EXPUI_EXPAND_ALL_ICON,
    EXPUI_NOTIFICATIONS_ICON,
    EXPUI_PROBLEMS_ICON,
    EXPUI_PROJECT_ICON,
    EXPUI_RUN_ICON,
    EXPUI_SETTINGS_ICON,
    EXPUI_TOOL_WINDOW_COMPONENTS_ICON,
    EXPUI_TOOL_WINDOW_DATA_VIEW_ICON,
    EYE_ICON,
    EYE_SLASH_ICON,
    FIELD_ICON,
    FILES_ICON,
    FILE_ARCHIVE_ICON,
    FILE_BINARY_DATA_ICON,
    FILE_CPP_ICON,
    FILE_CSHARP_ICON,
    FILE_CSS_ICON,
    FILE_CSV_ICON,
    FILE_C_ICON,
    FILE_DOCKER_ICON,
    FILE_EDITOR_CONFIG_ICON,
    FILE_FONT_ICON,
    FILE_GO_ICON,
    FILE_GRADLE_ICON,
    FILE_GRAPHQL_ICON,
    FILE_GROOVY_ICON,
    FILE_HTML_ICON,
    FILE_HTTP_ICON,
    FILE_H_ICON,
    FILE_ICON,
    FILE_IMAGE_ICON,
    FILE_JAVASCRIPT_ICON,
    FILE_JSON_ICON,
    FILE_JSP_ICON,
    FILE_KOTLIN_ICON,
    FILE_MARKDOWN_ICON,
    FILE_PATCH_ICON,
    FILE_PERL_ICON,
    FILE_PHP_ICON,
    FILE_PLUS_ICON,
    FILE_POWERSHELL_ICON,
    FILE_PROPERTIES_ICON,
    FILE_PYTHON_ICON,
    FILE_RST_ICON,
    FILE_RUBY_ICON,
    FILE_RUST_ICON,
    FILE_SCALA_ICON,
    FILE_SHELL_ICON,
    FILE_SQL_ICON,
    FILE_SWIFT_LANG_ICON,
    FILE_TERRAFORM_ICON,
    FILE_TOML_ICON,
    FILE_TYPESCRIPT_ICON,
    FILE_UNKNOWN_ICON,
    FILE_VUE_ICON,
    FILE_WSDL_ICON,
    FILE_XHTML_ICON,
    FILE_XML_ICON,
    FILE_XSD_ICON,
    FILE_YAML_ICON,
    FIND_TOOL_WINDOW_ICON,
    FLOPPY_DISK_ICON,
    FOLDER_ICON,
    FOLDER_PLUS_ICON,
    FRAME_ICON,
    FUNNEL_ICON,
    GEAR_ICON,
    GENERAL_GEAR_ICON,
    GIT_BRANCH_ICON,
    GIT_DIFF_ICON,
    GLOBE_HEMISPHERE_WEST_ICON,
    HASH_ICON,
    IMPLEMENTED_METHOD_ICON,
    IMPLEMENTING_METHOD_ICON,
    INFO_ICON,
    INTERFACE_ICON,
    JAVA_ICON,
    KEYBOARD_ICON,
    LIGHTBULB_ICON,
    LIGHTNING_ICON,
    LOCK_ICON,
    LOCK_OPEN_ICON,
    MAGNIFYING_GLASS_ICON,
    MAGNIFYING_GLASS_MINUS_ICON,
    MAGNIFYING_GLASS_PLUS_ICON,
    MAVEN_ICON,
    MAVEN_PROJECT_ICON,
    MINUS_ICON,
    MODULE_JAVA_ICON,
    MONITOR_ICON,
    MORE_ICON,
    MORE_VERTICAL_ICON,
    MUTE_BREAKPOINTS_ICON,
    OVERRIDEN_METHOD_ICON,
    OVERRIDING_METHOD_ICON,
    PACKAGE_ICON,
    PALETTE_ICON,
    PAUSE_ICON,
    PLAY_ICON,
    PLUGIN_ICON,
    PLUS_ICON,
    PROJECT_STRUCTURE_ICON,
    RECORD_ICON,
    REFRESH_ICON,
    RERUN_ICON,
    RESOURCES_ROOT_ICON,
    RESTART_DEBUG_ICON,
    RESUME_ICON,
    ROWS_ICON,
    RUN_ICON,
    RUN_RUN_ICON,
    RUN_TO_CURSOR_ICON,
    SCISSORS_ICON,
    SEARCH_ICON,
    SMART_STEP_INTO_ICON,
    SOURCE_ROOT_ICON,
    STEP_INTO_ICON,
    STEP_OUT_ICON,
    STEP_OVER_ICON,
    STOP_ICON,
    TERMINAL_ICON,
    TEST_FAILED_ICON,
    TEST_PASSED_ICON,
    TEST_RESOURCES_ROOT_ICON,
    TEST_ROOT_ICON,
    THREADS_ICON,
    THREAD_AT_BREAKPOINT_ICON,
    THREAD_CURRENT_ICON,
    THREAD_FROZEN_ICON,
    THREAD_RUNNING_ICON,
    THREAD_SUSPENDED_ICON,
    TOOL_WINDOW_AGENT_ICON,
    TOOL_WINDOW_COMMIT_ICON,
    TOOL_WINDOW_DEBUGGER_ICON,
    TOOL_WINDOW_FIND_ICON,
    TOOL_WINDOW_MAVEN_ICON,
    TOOL_WINDOW_PROBLEMS_ICON,
    TOOL_WINDOW_PROJECT_ICON,
    TOOL_WINDOW_RUN_ICON,
    TOOL_WINDOW_STRUCTURE_ICON,
    TOOL_WINDOW_VCS_ICON,
    TRASH_ICON,
    VARIABLE_ICON,
    VCS_ICON,
    VIEW_BREAKPOINTS_ICON,
    WARNING_CIRCLE_ICON,
    WATCH_ICON,
    XCIRCLE_ICON,
    X_ICON,
];

pub const ANNOTATION_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/annotation.svg",
    dark: "icons/idea/nodes/annotation.svg",
    has_dark: false,
};

pub const ARROW_CLOCKWISE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/redo.svg",
    dark: "icons/idea/legacy/ui/redo_dark.svg",
    has_dark: true,
};

pub const ARROW_COUNTER_CLOCKWISE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/undo.svg",
    dark: "icons/idea/legacy/ui/undo_dark.svg",
    has_dark: true,
};

pub const ARROW_DOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/down.svg",
    dark: "icons/idea/legacy/ui/down_dark.svg",
    has_dark: true,
};

pub const ARROW_LEFT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/left.svg",
    dark: "icons/idea/legacy/ui/left_dark.svg",
    has_dark: true,
};

pub const ARROW_RIGHT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/right.svg",
    dark: "icons/idea/legacy/ui/right_dark.svg",
    has_dark: true,
};

pub const ARROW_UP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/up.svg",
    dark: "icons/idea/legacy/ui/up_dark.svg",
    has_dark: true,
};

pub const ATTACH_TO_PROCESS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/attachToProcess.svg",
    dark: "icons/idea/debugger/attachToProcess_dark.svg",
    has_dark: true,
};

pub const BELL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/notifications.svg",
    dark: "icons/idea/legacy/ui/notifications_dark.svg",
    has_dark: true,
};

pub const BOOK_OPEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/documentation.svg",
    dark: "icons/idea/legacy/ui/documentation_dark.svg",
    has_dark: true,
};

pub const BUG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/debug.svg",
    dark: "icons/idea/debugger/debug_dark.svg",
    has_dark: true,
};

pub const CARET_DOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/chevronDown.svg",
    dark: "icons/idea/legacy/ui/chevronDown_dark.svg",
    has_dark: true,
};

pub const CARET_LEFT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/chevronLeft.svg",
    dark: "icons/idea/legacy/ui/chevronLeft_dark.svg",
    has_dark: true,
};

pub const CARET_RIGHT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/general/chevronRight.svg",
    dark: "icons/idea/expui/general/chevronRight_dark.svg",
    has_dark: true,
};

pub const CHECK_CIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/successDialog.svg",
    dark: "icons/idea/legacy/ui/successDialog_dark.svg",
    has_dark: true,
};

pub const CHECK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/greenCheckmark.svg",
    dark: "icons/idea/legacy/ui/greenCheckmark_dark.svg",
    has_dark: true,
};

pub const CLASS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/class.svg",
    dark: "icons/idea/nodes/class.svg",
    has_dark: false,
};

pub const CLOCK_COUNTER_CLOCKWISE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/history.svg",
    dark: "icons/idea/legacy/ui/history_dark.svg",
    has_dark: true,
};

pub const COLUMNS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/splitVertically.svg",
    dark: "icons/idea/legacy/ui/splitVertically_dark.svg",
    has_dark: true,
};

pub const COMMIT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/vcs/commit.svg",
    dark: "icons/idea/vcs/commit.svg",
    has_dark: false,
};

pub const COPY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/copy.svg",
    dark: "icons/idea/legacy/ui/copy_dark.svg",
    has_dark: true,
};

pub const DATABASE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/toolWindowDataView.svg",
    dark: "icons/idea/legacy/ui/toolWindowDataView_dark.svg",
    has_dark: true,
};

pub const DB_DISABLED_BREAKPOINT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/db_disabled_breakpoint.svg",
    dark: "icons/idea/debugger/db_disabled_breakpoint_dark.svg",
    has_dark: true,
};

pub const DB_MUTED_BREAKPOINT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/db_muted_breakpoint.svg",
    dark: "icons/idea/debugger/db_muted_breakpoint_dark.svg",
    has_dark: true,
};

pub const DB_SET_BREAKPOINT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/db_set_breakpoint.svg",
    dark: "icons/idea/debugger/db_set_breakpoint_dark.svg",
    has_dark: true,
};

pub const DB_VERIFIED_BREAKPOINT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/db_verified_breakpoint.svg",
    dark: "icons/idea/debugger/db_verified_breakpoint_dark.svg",
    has_dark: true,
};

pub const DOTS_THREE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/moreHorizontal.svg",
    dark: "icons/idea/legacy/ui/moreHorizontal_dark.svg",
    has_dark: true,
};

pub const DOWNLOAD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/download.svg",
    dark: "icons/idea/legacy/ui/download_dark.svg",
    has_dark: true,
};

pub const ENUM_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/enum.svg",
    dark: "icons/idea/nodes/enum.svg",
    has_dark: false,
};

pub const EVALUATE_EXPRESSION_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/evaluateExpression.svg",
    dark: "icons/idea/debugger/evaluateExpression_dark.svg",
    has_dark: true,
};

pub const EXCLUDE_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/excludeRoot_light.svg",
    dark: "icons/idea/nodes/excludeRoot.svg",
    has_dark: true,
};

pub const EXECUTE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/actions/execute.svg",
    dark: "icons/idea/actions/execute.svg",
    has_dark: false,
};

pub const EXPUI_COLLAPSE_ALL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/general/collapseAll.svg",
    dark: "icons/idea/expui/general/collapseAll_dark.svg",
    has_dark: true,
};

pub const EXPUI_COMMIT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/commit@20x20.svg",
    dark: "icons/idea/expui/toolwindows/commit@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_COVERAGE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/coverage@20x20.svg",
    dark: "icons/idea/expui/toolwindows/coverage@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_DEBUG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/debug@20x20.svg",
    dark: "icons/idea/expui/toolwindows/debug@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_EDIT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/general/edit.svg",
    dark: "icons/idea/expui/general/edit_dark.svg",
    has_dark: true,
};

pub const EXPUI_EXPAND_ALL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/general/expandAll.svg",
    dark: "icons/idea/expui/general/expandAll_dark.svg",
    has_dark: true,
};

pub const EXPUI_NOTIFICATIONS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/notifications@20x20.svg",
    dark: "icons/idea/expui/toolwindows/notifications@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_PROBLEMS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/problems@20x20.svg",
    dark: "icons/idea/expui/toolwindows/problems@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_PROJECT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/project@20x20.svg",
    dark: "icons/idea/expui/toolwindows/project@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_RUN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/run@20x20.svg",
    dark: "icons/idea/expui/toolwindows/run@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_SETTINGS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/general/settings@20x20.svg",
    dark: "icons/idea/expui/general/settings@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_TOOL_WINDOW_COMPONENTS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/toolWindowComponents@20x20.svg",
    dark: "icons/idea/expui/toolwindows/toolWindowComponents@20x20_dark.svg",
    has_dark: true,
};

pub const EXPUI_TOOL_WINDOW_DATA_VIEW_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/toolWindowDataView@20x20.svg",
    dark: "icons/idea/expui/toolwindows/toolWindowDataView@20x20_dark.svg",
    has_dark: true,
};

pub const EYE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/show.svg",
    dark: "icons/idea/legacy/ui/show_dark.svg",
    has_dark: true,
};

pub const EYE_SLASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/hide.svg",
    dark: "icons/idea/legacy/ui/hide_dark.svg",
    has_dark: true,
};

pub const FIELD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/field.svg",
    dark: "icons/idea/nodes/field.svg",
    has_dark: false,
};

pub const FILES_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/listFiles.svg",
    dark: "icons/idea/legacy/ui/listFiles_dark.svg",
    has_dark: true,
};

pub const FILE_ARCHIVE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/archive.svg",
    dark: "icons/idea/fileTypes/archive.svg",
    has_dark: false,
};

pub const FILE_BINARY_DATA_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/binaryData.svg",
    dark: "icons/idea/fileTypes/binaryData.svg",
    has_dark: false,
};

pub const FILE_CPP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/cpp.svg",
    dark: "icons/idea/fileTypes/cpp.svg",
    has_dark: false,
};

pub const FILE_CSHARP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/csharp.svg",
    dark: "icons/idea/fileTypes/csharp.svg",
    has_dark: false,
};

pub const FILE_CSS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/css.svg",
    dark: "icons/idea/fileTypes/css.svg",
    has_dark: false,
};

pub const FILE_CSV_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/csv.svg",
    dark: "icons/idea/fileTypes/csv.svg",
    has_dark: false,
};

pub const FILE_C_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/c.svg",
    dark: "icons/idea/fileTypes/c.svg",
    has_dark: false,
};

pub const FILE_DOCKER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/docker.svg",
    dark: "icons/idea/fileTypes/docker.svg",
    has_dark: false,
};

pub const FILE_EDITOR_CONFIG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/editorConfig.svg",
    dark: "icons/idea/fileTypes/editorConfig.svg",
    has_dark: false,
};

pub const FILE_FONT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/font.svg",
    dark: "icons/idea/legacy/files/font_dark.svg",
    has_dark: true,
};

pub const FILE_GO_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/go.svg",
    dark: "icons/idea/fileTypes/go.svg",
    has_dark: false,
};

pub const FILE_GRADLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/gradle.svg",
    dark: "icons/idea/fileTypes/gradle.svg",
    has_dark: false,
};

pub const FILE_GRAPHQL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/graphql.svg",
    dark: "icons/idea/legacy/files/graphql_dark.svg",
    has_dark: true,
};

pub const FILE_GROOVY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/groovy.svg",
    dark: "icons/idea/fileTypes/groovy.svg",
    has_dark: false,
};

pub const FILE_HTML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/html.svg",
    dark: "icons/idea/fileTypes/html.svg",
    has_dark: false,
};

pub const FILE_HTTP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/http.svg",
    dark: "icons/idea/legacy/files/http.svg",
    has_dark: false,
};

pub const FILE_H_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/h.svg",
    dark: "icons/idea/fileTypes/h.svg",
    has_dark: false,
};

pub const FILE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/text.svg",
    dark: "icons/idea/fileTypes/text.svg",
    has_dark: false,
};

pub const FILE_IMAGE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/image.svg",
    dark: "icons/idea/fileTypes/image.svg",
    has_dark: false,
};

pub const FILE_JAVASCRIPT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/javaScript.svg",
    dark: "icons/idea/fileTypes/javaScript.svg",
    has_dark: false,
};

pub const FILE_JSON_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/json.svg",
    dark: "icons/idea/fileTypes/json.svg",
    has_dark: false,
};

pub const FILE_JSP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/jsp.svg",
    dark: "icons/idea/legacy/files/jsp_dark.svg",
    has_dark: true,
};

pub const FILE_KOTLIN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/kotlin.svg",
    dark: "icons/idea/fileTypes/kotlin.svg",
    has_dark: false,
};

pub const FILE_MARKDOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/markdown.svg",
    dark: "icons/idea/fileTypes/markdown.svg",
    has_dark: false,
};

pub const FILE_PATCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/patch.svg",
    dark: "icons/idea/legacy/files/patch_dark.svg",
    has_dark: true,
};

pub const FILE_PERL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/perl.svg",
    dark: "icons/idea/legacy/files/perl_dark.svg",
    has_dark: true,
};

pub const FILE_PHP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/php.svg",
    dark: "icons/idea/fileTypes/php.svg",
    has_dark: false,
};

pub const FILE_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/addFile.svg",
    dark: "icons/idea/legacy/ui/addFile_dark.svg",
    has_dark: true,
};

// PowerShell 没有独立的深色美术：这里原先登记的 `fileTypes/powershell_dark.svg` 与浅色版
// **逐字节相同**（连注释都一样），是误加后缀的冗余副本，已删除。所以按单文件图标登记，
// 不要照 `BUG_ICON` 那样填 `has_dark: true` —— 那等于宣称有一条并不存在的深色变体，
// 后面的人会照这个名字去接另一张图（历史上已经误导过一次判断）。
pub const FILE_POWERSHELL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/powershell.svg",
    dark: "icons/idea/fileTypes/powershell.svg",
    has_dark: false,
};

pub const FILE_PROPERTIES_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/properties.svg",
    dark: "icons/idea/fileTypes/properties.svg",
    has_dark: false,
};

pub const FILE_PYTHON_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/python.svg",
    dark: "icons/idea/fileTypes/python.svg",
    has_dark: false,
};

pub const FILE_RST_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/rst.svg",
    dark: "icons/idea/legacy/files/rst_dark.svg",
    has_dark: true,
};

pub const FILE_RUBY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/ruby.svg",
    dark: "icons/idea/fileTypes/ruby.svg",
    has_dark: false,
};

pub const FILE_RUST_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/rust.svg",
    dark: "icons/idea/fileTypes/rust.svg",
    has_dark: false,
};

pub const FILE_SCALA_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/scala.svg",
    dark: "icons/idea/fileTypes/scala.svg",
    has_dark: false,
};

pub const FILE_SHELL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/shell.svg",
    dark: "icons/idea/fileTypes/shell.svg",
    has_dark: false,
};

pub const FILE_SQL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/sql.svg",
    dark: "icons/idea/fileTypes/sql.svg",
    has_dark: false,
};

pub const FILE_SWIFT_LANG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/swiftLang.svg",
    dark: "icons/idea/fileTypes/swiftLang.svg",
    has_dark: false,
};

pub const FILE_TERRAFORM_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/terraform.svg",
    dark: "icons/idea/legacy/files/terraform_dark.svg",
    has_dark: true,
};

pub const FILE_TOML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/toml.svg",
    dark: "icons/idea/fileTypes/toml.svg",
    has_dark: false,
};

pub const FILE_TYPESCRIPT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/typeScript.svg",
    dark: "icons/idea/legacy/files/typeScript_dark.svg",
    has_dark: true,
};

pub const FILE_UNKNOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/unknown.svg",
    dark: "icons/idea/fileTypes/unknown.svg",
    has_dark: false,
};

pub const FILE_VUE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/vue.svg",
    dark: "icons/idea/legacy/files/vue.svg",
    has_dark: false,
};

pub const FILE_WSDL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/wsdl.svg",
    dark: "icons/idea/legacy/files/wsdl_dark.svg",
    has_dark: true,
};

pub const FILE_XHTML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/xhtml.svg",
    dark: "icons/idea/legacy/files/xhtml_dark.svg",
    has_dark: true,
};

pub const FILE_XML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/xml.svg",
    dark: "icons/idea/fileTypes/xml.svg",
    has_dark: false,
};

pub const FILE_XSD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/xsd.svg",
    dark: "icons/idea/legacy/files/xsd_dark.svg",
    has_dark: true,
};

pub const FILE_YAML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/yaml.svg",
    dark: "icons/idea/fileTypes/yaml.svg",
    has_dark: false,
};

pub const FIND_TOOL_WINDOW_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/find@20x20.svg",
    dark: "icons/idea/expui/toolwindows/find@20x20_dark.svg",
    has_dark: true,
};

pub const FLOPPY_DISK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/save.svg",
    dark: "icons/idea/legacy/ui/save_dark.svg",
    has_dark: true,
};

pub const FOLDER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/folder_light.svg",
    dark: "icons/idea/nodes/folder.svg",
    has_dark: true,
};

pub const FOLDER_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/newFolder.svg",
    dark: "icons/idea/legacy/ui/newFolder_dark.svg",
    has_dark: true,
};

pub const FRAME_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/frame.svg",
    dark: "icons/idea/debugger/frame_dark.svg",
    has_dark: true,
};

pub const FUNNEL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/filter.svg",
    dark: "icons/idea/legacy/ui/filter_dark.svg",
    has_dark: true,
};

pub const GEAR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/settings.svg",
    dark: "icons/idea/legacy/ui/settings_dark.svg",
    has_dark: true,
};

pub const GENERAL_GEAR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/general/gear.svg",
    dark: "icons/idea/general/gear.svg",
    has_dark: false,
};

pub const GIT_BRANCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/vcs/branch.svg",
    dark: "icons/idea/vcs/branch.svg",
    has_dark: false,
};

pub const GIT_DIFF_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/vcs/diff.svg",
    dark: "icons/idea/vcs/diff.svg",
    has_dark: false,
};

pub const GLOBE_HEMISPHERE_WEST_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/web.svg",
    dark: "icons/idea/legacy/ui/web_dark.svg",
    has_dark: true,
};

pub const HASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/hashtag.svg",
    dark: "icons/idea/legacy/ui/hashtag_dark.svg",
    has_dark: true,
};

pub const IMPLEMENTED_METHOD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/gutter/implementedMethod.svg",
    dark: "icons/idea/gutter/implementedMethod.svg",
    has_dark: false,
};

pub const IMPLEMENTING_METHOD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/gutter/implementingMethod.svg",
    dark: "icons/idea/gutter/implementingMethod.svg",
    has_dark: false,
};

pub const INFO_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/informationDialog.svg",
    dark: "icons/idea/legacy/ui/informationDialog_dark.svg",
    has_dark: true,
};

pub const INTERFACE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/interface.svg",
    dark: "icons/idea/nodes/interface.svg",
    has_dark: false,
};

pub const JAVA_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/java.svg",
    dark: "icons/idea/fileTypes/java.svg",
    has_dark: false,
};

pub const KEYBOARD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/keyboard.svg",
    dark: "icons/idea/legacy/ui/keyboard_dark.svg",
    has_dark: true,
};

pub const LIGHTBULB_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/realIntentionBulb.svg",
    dark: "icons/idea/legacy/ui/realIntentionBulb_dark.svg",
    has_dark: true,
};

pub const LIGHTNING_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/lightning.svg",
    dark: "icons/idea/legacy/ui/lightning_dark.svg",
    has_dark: true,
};

pub const LOCK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/locked.svg",
    dark: "icons/idea/legacy/ui/locked_dark.svg",
    has_dark: true,
};

pub const LOCK_OPEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/unlocked.svg",
    dark: "icons/idea/legacy/ui/unlocked_dark.svg",
    has_dark: true,
};

pub const MAGNIFYING_GLASS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/general/search.svg",
    dark: "icons/idea/expui/general/search_dark.svg",
    has_dark: true,
};

pub const MAGNIFYING_GLASS_MINUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/zoomOut.svg",
    dark: "icons/idea/legacy/ui/zoomOut_dark.svg",
    has_dark: true,
};

pub const MAGNIFYING_GLASS_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/zoomIn.svg",
    dark: "icons/idea/legacy/ui/zoomIn_dark.svg",
    has_dark: true,
};

pub const MAVEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/files/maven.svg",
    dark: "icons/idea/legacy/files/maven_dark.svg",
    has_dark: true,
};

pub const MAVEN_PROJECT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/maven/mavenProject.svg",
    dark: "icons/idea/maven/mavenProject.svg",
    has_dark: false,
};

pub const MINUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/remove.svg",
    dark: "icons/idea/legacy/ui/remove_dark.svg",
    has_dark: true,
};

pub const MODULE_JAVA_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/moduleJava_light.svg",
    dark: "icons/idea/nodes/moduleJava.svg",
    has_dark: true,
};

pub const MONITOR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/desktop.svg",
    dark: "icons/idea/legacy/ui/desktop_dark.svg",
    has_dark: true,
};

pub const MORE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/actions/more.svg",
    dark: "icons/idea/actions/more.svg",
    has_dark: false,
};

pub const MORE_VERTICAL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/actions/moreVertical.svg",
    dark: "icons/idea/actions/moreVertical.svg",
    has_dark: false,
};

pub const MUTE_BREAKPOINTS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/muteBreakpoints.svg",
    dark: "icons/idea/debugger/muteBreakpoints_dark.svg",
    has_dark: true,
};

pub const OVERRIDEN_METHOD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/gutter/overridenMethod.svg",
    dark: "icons/idea/gutter/overridenMethod.svg",
    has_dark: false,
};

pub const OVERRIDING_METHOD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/gutter/overridingMethod.svg",
    dark: "icons/idea/gutter/overridingMethod.svg",
    has_dark: false,
};

pub const PACKAGE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/package_light.svg",
    dark: "icons/idea/nodes/package.svg",
    has_dark: true,
};

pub const PALETTE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/palette.svg",
    dark: "icons/idea/legacy/ui/palette_dark.svg",
    has_dark: true,
};

pub const PAUSE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/pause.svg",
    dark: "icons/idea/debugger/pause_dark.svg",
    has_dark: true,
};

pub const PLAY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/testState/run.svg",
    dark: "icons/idea/testState/run_dark.svg",
    has_dark: true,
};

pub const PLUGIN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/nodes/plugin.svg",
    dark: "icons/idea/expui/nodes/plugin_dark.svg",
    has_dark: true,
};

pub const PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/add.svg",
    dark: "icons/idea/legacy/ui/add_dark.svg",
    has_dark: true,
};

pub const PROJECT_STRUCTURE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/general/projectStructure.svg",
    dark: "icons/idea/general/projectStructure.svg",
    has_dark: false,
};

pub const RECORD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/record.svg",
    dark: "icons/idea/nodes/record.svg",
    has_dark: false,
};

pub const REFRESH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/actions/refresh.svg",
    dark: "icons/idea/actions/refresh.svg",
    has_dark: false,
};

pub const RERUN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/rerun.svg",
    dark: "icons/idea/debugger/rerun_dark.svg",
    has_dark: true,
};

pub const RESOURCES_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/resourcesRoot_light.svg",
    dark: "icons/idea/nodes/resourcesRoot.svg",
    has_dark: true,
};

pub const RESTART_DEBUG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/restartDebug.svg",
    dark: "icons/idea/debugger/restartDebug_dark.svg",
    has_dark: true,
};

pub const RESUME_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/resume.svg",
    dark: "icons/idea/debugger/resume_dark.svg",
    has_dark: true,
};

pub const ROWS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/splitHorizontally.svg",
    dark: "icons/idea/legacy/ui/splitHorizontally_dark.svg",
    has_dark: true,
};

pub const RUN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/run.svg",
    dark: "icons/idea/debugger/run_dark.svg",
    has_dark: true,
};

pub const RUN_RUN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/testState/run_run.svg",
    dark: "icons/idea/testState/run_run_dark.svg",
    has_dark: true,
};

pub const RUN_TO_CURSOR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/runToCursor.svg",
    dark: "icons/idea/debugger/runToCursor_dark.svg",
    has_dark: true,
};

pub const SCISSORS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/cut.svg",
    dark: "icons/idea/legacy/ui/cut_dark.svg",
    has_dark: true,
};

pub const SEARCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/actions/search.svg",
    dark: "icons/idea/actions/search.svg",
    has_dark: false,
};

pub const SMART_STEP_INTO_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/smartStepInto.svg",
    dark: "icons/idea/debugger/smartStepInto_dark.svg",
    has_dark: true,
};

pub const SOURCE_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/sourceRoot_light.svg",
    dark: "icons/idea/nodes/sourceRoot.svg",
    has_dark: true,
};

pub const STEP_INTO_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/stepInto.svg",
    dark: "icons/idea/debugger/stepInto_dark.svg",
    has_dark: true,
};

pub const STEP_OUT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/stepOut.svg",
    dark: "icons/idea/debugger/stepOut_dark.svg",
    has_dark: true,
};

pub const STEP_OVER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/stepOver.svg",
    dark: "icons/idea/debugger/stepOver_dark.svg",
    has_dark: true,
};

pub const STOP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/stop.svg",
    dark: "icons/idea/debugger/stop_dark.svg",
    has_dark: true,
};

pub const TERMINAL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/terminal/terminal@20x20.svg",
    dark: "icons/idea/terminal/terminal@20x20_dark.svg",
    has_dark: true,
};

pub const TEST_FAILED_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/testState/red2.svg",
    dark: "icons/idea/testState/red2_dark.svg",
    has_dark: true,
};

pub const TEST_PASSED_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/testState/green2.svg",
    dark: "icons/idea/testState/green2_dark.svg",
    has_dark: true,
};

pub const TEST_RESOURCES_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/folders/testResourcesRoot.svg",
    dark: "icons/idea/legacy/folders/testResourcesRoot_dark.svg",
    has_dark: true,
};

pub const TEST_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/folders/testRoot.svg",
    dark: "icons/idea/legacy/folders/testRoot_dark.svg",
    has_dark: true,
};

pub const THREADS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/threads.svg",
    dark: "icons/idea/debugger/threads_dark.svg",
    has_dark: true,
};

pub const THREAD_AT_BREAKPOINT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/threadAtBreakpoint.svg",
    dark: "icons/idea/debugger/threadAtBreakpoint.svg",
    has_dark: false,
};

pub const THREAD_CURRENT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/threadCurrent.svg",
    dark: "icons/idea/debugger/threadCurrent.svg",
    has_dark: false,
};

pub const THREAD_FROZEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/threadFrozen.svg",
    dark: "icons/idea/debugger/threadFrozen.svg",
    has_dark: false,
};

pub const THREAD_RUNNING_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/threadRunning.svg",
    dark: "icons/idea/debugger/threadRunning.svg",
    has_dark: false,
};

pub const THREAD_SUSPENDED_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/threadSuspended.svg",
    dark: "icons/idea/debugger/threadSuspended.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_AGENT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowAgent.svg",
    dark: "icons/idea/toolwindows/toolWindowAgent.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_COMMIT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowCommit.svg",
    dark: "icons/idea/toolwindows/toolWindowCommit.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_DEBUGGER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowDebugger.svg",
    dark: "icons/idea/toolwindows/toolWindowDebugger.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_FIND_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowFind.svg",
    dark: "icons/idea/toolwindows/toolWindowFind.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_MAVEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/maven/toolWindowMaven.svg",
    dark: "icons/idea/maven/toolWindowMaven.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_PROBLEMS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowProblems.svg",
    dark: "icons/idea/toolwindows/toolWindowProblems.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_PROJECT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowProject.svg",
    dark: "icons/idea/toolwindows/toolWindowProject.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_RUN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowRun.svg",
    dark: "icons/idea/toolwindows/toolWindowRun.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_STRUCTURE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowStructure.svg",
    dark: "icons/idea/toolwindows/toolWindowStructure.svg",
    has_dark: false,
};

pub const TOOL_WINDOW_VCS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/toolwindows/toolWindowVcs.svg",
    dark: "icons/idea/toolwindows/toolWindowVcs.svg",
    has_dark: false,
};

pub const TRASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/delete.svg",
    dark: "icons/idea/legacy/ui/delete_dark.svg",
    has_dark: true,
};

pub const VARIABLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/nodes/variable.svg",
    dark: "icons/idea/nodes/variable.svg",
    has_dark: false,
};

pub const VCS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/vcs@20x20.svg",
    dark: "icons/idea/expui/toolwindows/vcs@20x20_dark.svg",
    has_dark: true,
};

pub const VIEW_BREAKPOINTS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/viewBreakpoints.svg",
    dark: "icons/idea/debugger/viewBreakpoints_dark.svg",
    has_dark: true,
};

pub const WARNING_CIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/warningDialog.svg",
    dark: "icons/idea/legacy/ui/warningDialog_dark.svg",
    has_dark: true,
};

pub const WATCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/watch.svg",
    dark: "icons/idea/debugger/watch.svg",
    has_dark: false,
};

pub const XCIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/errorDialog.svg",
    dark: "icons/idea/legacy/ui/errorDialog_dark.svg",
    has_dark: true,
};

pub const X_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/close.svg",
    dark: "icons/idea/legacy/ui/close_dark.svg",
    has_dark: true,
};
