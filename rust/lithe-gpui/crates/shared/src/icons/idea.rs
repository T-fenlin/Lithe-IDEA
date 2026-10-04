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

pub const ALL: [IdeaIcon; 128] = [
    ANNOTATION_ICON,
    ARCHIVE_ICON,
    ARROW_CLOCKWISE_ICON,
    ARROW_COUNTER_CLOCKWISE_ICON,
    ARROW_DOWN_ICON,
    ARROW_LEFT_ICON,
    ARROW_RIGHT_ICON,
    ARROW_SQUARE_OUT_ICON,
    ARROW_UP_ICON,
    BELL_ICON,
    BOOK_OPEN_ICON,
    BOOKMARK_ICON,
    BUG_ICON,
    C_ICON,
    CARET_DOWN_ICON,
    CARET_LEFT_ICON,
    CARET_RIGHT_ICON,
    CARET_UP_ICON,
    CHAT_CIRCLE_TEXT_ICON,
    CHECK_ICON,
    CHECK_CIRCLE_ICON,
    CLASS_ICON,
    CLOCK_COUNTER_CLOCKWISE_ICON,
    COLUMNS_ICON,
    COPY_ICON,
    CPP_ICON,
    CSHARP_ICON,
    CSS_ICON,
    CSV_ICON,
    DATABASE_ICON,
    DOCKER_ICON,
    DOTS_THREE_ICON,
    DOWNLOAD_ICON,
    EDITOR_CONFIG_ICON,
    ENUM_ICON,
    EXCEPTION_ICON,
    EYE_ICON,
    EYE_SLASH_ICON,
    FILE_ICON,
    FILE_PLUS_ICON,
    FILES_ICON,
    FLOPPY_DISK_ICON,
    FOLDER_ICON,
    FOLDER_PLUS_ICON,
    FONT_ICON,
    FUNCTION_ICON,
    FUNNEL_ICON,
    GEAR_ICON,
    GIT_BRANCH_ICON,
    GIT_COMMIT_ICON,
    GIT_DIFF_ICON,
    GIT_MERGE_ICON,
    GITIGNORE_ICON,
    GLOBE_HEMISPHERE_WEST_ICON,
    GRADLE_ICON,
    GRAPHQL_ICON,
    GROOVY_ICON,
    H_ICON,
    HASH_ICON,
    HOUSE_ICON,
    HTML_ICON,
    HTTP_ICON,
    IMAGE_ICON,
    INFO_ICON,
    INTERFACE_ICON,
    JAVA_ICON,
    JAVA_SCRIPT_ICON,
    JSON_ICON,
    JSP_ICON,
    KEYBOARD_ICON,
    LIGHTBULB_ICON,
    LIGHTNING_ICON,
    LOCK_ICON,
    LOCK_OPEN_ICON,
    MAGNIFYING_GLASS_ICON,
    MAGNIFYING_GLASS_MINUS_ICON,
    MAGNIFYING_GLASS_PLUS_ICON,
    MARKDOWN_ICON,
    MAVEN_ICON,
    MINUS_ICON,
    MONITOR_ICON,
    PACKAGE_ICON,
    PALETTE_ICON,
    PATCH_ICON,
    PAUSE_ICON,
    PEN_ICON,
    PERL_ICON,
    PLAY_ICON,
    PLUS_ICON,
    POWERSHELL_ICON,
    PROPERTIES_ICON,
    PUSH_PIN_ICON,
    QUESTION_ICON,
    RECORD_ICON,
    REFRESH_ICON,
    RESOURCES_ROOT_ICON,
    ROWS_ICON,
    RST_ICON,
    SCISSORS_ICON,
    SHELL_ICON,
    SIDEBAR_SIMPLE_ICON,
    SIGN_OUT_ICON,
    SOURCE_ROOT_ICON,
    SQL_ICON,
    STOP_ICON,
    SWIFT_LANG_ICON,
    TABLE_ICON,
    TERRAFORM_ICON,
    TEST_RESOURCES_ROOT_ICON,
    TEST_ROOT_ICON,
    TOML_ICON,
    TRANSLATE_ICON,
    TRASH_ICON,
    TREE_STRUCTURE_ICON,
    TYPE_SCRIPT_ICON,
    UPLOAD_ICON,
    USER_ICON,
    USERS_THREE_ICON,
    VUE_ICON,
    WARNING_CIRCLE_ICON,
    WRENCH_ICON,
    WSDL_ICON,
    X_ICON,
    XCIRCLE_ICON,
    XHTML_ICON,
    XML_ICON,
    XSD_ICON,
    YAML_ICON,
];

pub const ANNOTATION_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/annotation.svg",
    dark: "icons/idea/icons/files/annotation_dark.svg",
    has_dark: true,
};
pub const ARCHIVE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/archive.svg",
    dark: "icons/idea/icons/files/archive_dark.svg",
    has_dark: true,
};
pub const ARROW_CLOCKWISE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/redo.svg",
    dark: "icons/idea/icons/ui/redo_dark.svg",
    has_dark: true,
};
pub const ARROW_COUNTER_CLOCKWISE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/undo.svg",
    dark: "icons/idea/icons/ui/undo_dark.svg",
    has_dark: true,
};
pub const ARROW_DOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/down.svg",
    dark: "icons/idea/icons/ui/down_dark.svg",
    has_dark: true,
};
pub const ARROW_LEFT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/left.svg",
    dark: "icons/idea/icons/ui/left_dark.svg",
    has_dark: true,
};
pub const ARROW_RIGHT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/right.svg",
    dark: "icons/idea/icons/ui/right_dark.svg",
    has_dark: true,
};
pub const ARROW_SQUARE_OUT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/externalLink.svg",
    dark: "icons/idea/icons/ui/externalLink_dark.svg",
    has_dark: true,
};
pub const ARROW_UP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/up.svg",
    dark: "icons/idea/icons/ui/up_dark.svg",
    has_dark: true,
};
pub const BELL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/notifications.svg",
    dark: "icons/idea/icons/ui/notifications_dark.svg",
    has_dark: true,
};
pub const BOOK_OPEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/documentation.svg",
    dark: "icons/idea/icons/ui/documentation_dark.svg",
    has_dark: true,
};
pub const BOOKMARK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/bookmark.svg",
    dark: "icons/idea/icons/ui/bookmark_dark.svg",
    has_dark: true,
};
pub const BUG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/debug.svg",
    dark: "icons/idea/icons/ui/debug_dark.svg",
    has_dark: true,
};
pub const C_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/c.svg",
    dark: "icons/idea/icons/files/c_dark.svg",
    has_dark: true,
};
pub const CARET_DOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/chevronDown.svg",
    dark: "icons/idea/icons/ui/chevronDown_dark.svg",
    has_dark: true,
};
pub const CARET_LEFT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/chevronLeft.svg",
    dark: "icons/idea/icons/ui/chevronLeft_dark.svg",
    has_dark: true,
};
pub const CARET_RIGHT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/chevronRight.svg",
    dark: "icons/idea/icons/ui/chevronRight_dark.svg",
    has_dark: true,
};
pub const CARET_UP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/chevronUp.svg",
    dark: "icons/idea/icons/ui/chevronUp_dark.svg",
    has_dark: true,
};
pub const CHAT_CIRCLE_TEXT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/messages.svg",
    dark: "icons/idea/icons/ui/messages_dark.svg",
    has_dark: true,
};
pub const CHECK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/greenCheckmark.svg",
    dark: "icons/idea/icons/ui/greenCheckmark_dark.svg",
    has_dark: true,
};
pub const CHECK_CIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/successDialog.svg",
    dark: "icons/idea/icons/ui/successDialog_dark.svg",
    has_dark: true,
};
pub const CLASS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/class.svg",
    dark: "icons/idea/icons/files/class_dark.svg",
    has_dark: true,
};
pub const CLOCK_COUNTER_CLOCKWISE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/history.svg",
    dark: "icons/idea/icons/ui/history_dark.svg",
    has_dark: true,
};
pub const COLUMNS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/splitVertically.svg",
    dark: "icons/idea/icons/ui/splitVertically_dark.svg",
    has_dark: true,
};
pub const COPY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/copy.svg",
    dark: "icons/idea/icons/ui/copy_dark.svg",
    has_dark: true,
};
pub const CPP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/cpp.svg",
    dark: "icons/idea/icons/files/cpp_dark.svg",
    has_dark: true,
};
pub const CSHARP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/Csharp.svg",
    dark: "icons/idea/icons/files/Csharp_dark.svg",
    has_dark: true,
};
pub const CSS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/css.svg",
    dark: "icons/idea/icons/files/css_dark.svg",
    has_dark: true,
};
pub const CSV_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/csv.svg",
    dark: "icons/idea/icons/files/csv_dark.svg",
    has_dark: true,
};
pub const DATABASE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/toolWindowDataView.svg",
    dark: "icons/idea/icons/ui/toolWindowDataView_dark.svg",
    has_dark: true,
};
pub const DOCKER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/docker.svg",
    dark: "icons/idea/icons/files/docker_dark.svg",
    has_dark: true,
};
pub const DOTS_THREE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/moreHorizontal.svg",
    dark: "icons/idea/icons/ui/moreHorizontal_dark.svg",
    has_dark: true,
};
pub const DOWNLOAD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/download.svg",
    dark: "icons/idea/icons/ui/download_dark.svg",
    has_dark: true,
};
pub const EDITOR_CONFIG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/editorConfig.svg",
    dark: "icons/idea/icons/files/editorConfig_dark.svg",
    has_dark: true,
};
pub const ENUM_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/enum.svg",
    dark: "icons/idea/icons/files/enum_dark.svg",
    has_dark: true,
};
pub const EXCEPTION_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/exception.svg",
    dark: "icons/idea/icons/files/exception_dark.svg",
    has_dark: true,
};
pub const EYE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/show.svg",
    dark: "icons/idea/icons/ui/show_dark.svg",
    has_dark: true,
};
pub const EYE_SLASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/hide.svg",
    dark: "icons/idea/icons/ui/hide_dark.svg",
    has_dark: true,
};
pub const FILE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/text.svg",
    dark: "icons/idea/icons/files/text_dark.svg",
    has_dark: true,
};
pub const FILE_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/addFile.svg",
    dark: "icons/idea/icons/ui/addFile_dark.svg",
    has_dark: true,
};
pub const FILES_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/listFiles.svg",
    dark: "icons/idea/icons/ui/listFiles_dark.svg",
    has_dark: true,
};
pub const FLOPPY_DISK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/save.svg",
    dark: "icons/idea/icons/ui/save_dark.svg",
    has_dark: true,
};
pub const FOLDER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/folders/folder.svg",
    dark: "icons/idea/icons/folders/folder_dark.svg",
    has_dark: true,
};
pub const FOLDER_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/newFolder.svg",
    dark: "icons/idea/icons/ui/newFolder_dark.svg",
    has_dark: true,
};
pub const FONT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/font.svg",
    dark: "icons/idea/icons/files/font_dark.svg",
    has_dark: true,
};
pub const FUNCTION_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/function.svg",
    dark: "icons/idea/icons/files/function_dark.svg",
    has_dark: true,
};
pub const FUNNEL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/filter.svg",
    dark: "icons/idea/icons/ui/filter_dark.svg",
    has_dark: true,
};
pub const GEAR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/settings.svg",
    dark: "icons/idea/icons/ui/settings_dark.svg",
    has_dark: true,
};
pub const GIT_BRANCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/branch.svg",
    dark: "icons/idea/icons/ui/branch_dark.svg",
    has_dark: true,
};
pub const GIT_COMMIT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/commit.svg",
    dark: "icons/idea/icons/ui/commit_dark.svg",
    has_dark: true,
};
pub const GIT_DIFF_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/diff.svg",
    dark: "icons/idea/icons/ui/diff_dark.svg",
    has_dark: true,
};
pub const GIT_MERGE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/merge.svg",
    dark: "icons/idea/icons/ui/merge_dark.svg",
    has_dark: true,
};
pub const GITIGNORE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/gitignore.svg",
    dark: "icons/idea/icons/files/gitignore.svg",
    has_dark: false,
};
pub const GLOBE_HEMISPHERE_WEST_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/web.svg",
    dark: "icons/idea/icons/ui/web_dark.svg",
    has_dark: true,
};
pub const GRADLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/gradle.svg",
    dark: "icons/idea/icons/files/gradle_dark.svg",
    has_dark: true,
};
pub const GRAPHQL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/graphql.svg",
    dark: "icons/idea/icons/files/graphql_dark.svg",
    has_dark: true,
};
pub const GROOVY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/groovy.svg",
    dark: "icons/idea/icons/files/groovy_dark.svg",
    has_dark: true,
};
pub const H_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/h.svg",
    dark: "icons/idea/icons/files/h_dark.svg",
    has_dark: true,
};
pub const HASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/hashtag.svg",
    dark: "icons/idea/icons/ui/hashtag_dark.svg",
    has_dark: true,
};
pub const HOUSE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/home.svg",
    dark: "icons/idea/icons/ui/home_dark.svg",
    has_dark: true,
};
pub const HTML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/html.svg",
    dark: "icons/idea/icons/files/html_dark.svg",
    has_dark: true,
};
pub const HTTP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/http.svg",
    dark: "icons/idea/icons/files/http.svg",
    has_dark: false,
};
pub const IMAGE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/image.svg",
    dark: "icons/idea/icons/files/image_dark.svg",
    has_dark: true,
};
pub const INFO_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/informationDialog.svg",
    dark: "icons/idea/icons/ui/informationDialog_dark.svg",
    has_dark: true,
};
pub const INTERFACE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/interface.svg",
    dark: "icons/idea/icons/files/interface_dark.svg",
    has_dark: true,
};
pub const JAVA_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/java.svg",
    dark: "icons/idea/icons/files/java_dark.svg",
    has_dark: true,
};
pub const JAVA_SCRIPT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/javaScript.svg",
    dark: "icons/idea/icons/files/javaScript_dark.svg",
    has_dark: true,
};
pub const JSON_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/json.svg",
    dark: "icons/idea/icons/files/json_dark.svg",
    has_dark: true,
};
pub const JSP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/jsp.svg",
    dark: "icons/idea/icons/files/jsp_dark.svg",
    has_dark: true,
};
pub const KEYBOARD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/keyboard.svg",
    dark: "icons/idea/icons/ui/keyboard_dark.svg",
    has_dark: true,
};
pub const LIGHTBULB_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/realIntentionBulb.svg",
    dark: "icons/idea/icons/ui/realIntentionBulb_dark.svg",
    has_dark: true,
};
pub const LIGHTNING_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/lightning.svg",
    dark: "icons/idea/icons/ui/lightning_dark.svg",
    has_dark: true,
};
pub const LOCK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/locked.svg",
    dark: "icons/idea/icons/ui/locked_dark.svg",
    has_dark: true,
};
pub const LOCK_OPEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/unlocked.svg",
    dark: "icons/idea/icons/ui/unlocked_dark.svg",
    has_dark: true,
};
pub const MAGNIFYING_GLASS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/search.svg",
    dark: "icons/idea/icons/ui/search_dark.svg",
    has_dark: true,
};
pub const MAGNIFYING_GLASS_MINUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/zoomOut.svg",
    dark: "icons/idea/icons/ui/zoomOut_dark.svg",
    has_dark: true,
};
pub const MAGNIFYING_GLASS_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/zoomIn.svg",
    dark: "icons/idea/icons/ui/zoomIn_dark.svg",
    has_dark: true,
};
pub const MARKDOWN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/markdown.svg",
    dark: "icons/idea/icons/files/markdown_dark.svg",
    has_dark: true,
};
pub const MAVEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/maven.svg",
    dark: "icons/idea/icons/files/maven_dark.svg",
    has_dark: true,
};
pub const MINUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/remove.svg",
    dark: "icons/idea/icons/ui/remove_dark.svg",
    has_dark: true,
};
pub const MONITOR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/desktop.svg",
    dark: "icons/idea/icons/ui/desktop_dark.svg",
    has_dark: true,
};
pub const PACKAGE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/folders/package.svg",
    dark: "icons/idea/icons/folders/package_dark.svg",
    has_dark: true,
};
pub const PALETTE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/palette.svg",
    dark: "icons/idea/icons/ui/palette_dark.svg",
    has_dark: true,
};
pub const PATCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/patch.svg",
    dark: "icons/idea/icons/files/patch_dark.svg",
    has_dark: true,
};
pub const PAUSE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/pause.svg",
    dark: "icons/idea/icons/ui/pause_dark.svg",
    has_dark: true,
};
pub const PEN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/edit.svg",
    dark: "icons/idea/icons/ui/edit_dark.svg",
    has_dark: true,
};
pub const PERL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/perl.svg",
    dark: "icons/idea/icons/files/perl_dark.svg",
    has_dark: true,
};
pub const PLAY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/run.svg",
    dark: "icons/idea/icons/ui/run_dark.svg",
    has_dark: true,
};
pub const PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/add.svg",
    dark: "icons/idea/icons/ui/add_dark.svg",
    has_dark: true,
};
pub const POWERSHELL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/powershell.svg",
    dark: "icons/idea/icons/files/powershell_dark.svg",
    has_dark: true,
};
pub const PROPERTIES_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/properties.svg",
    dark: "icons/idea/icons/files/properties_dark.svg",
    has_dark: true,
};
pub const PUSH_PIN_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/pin.svg",
    dark: "icons/idea/icons/ui/pin_dark.svg",
    has_dark: true,
};
pub const QUESTION_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/questionDialog.svg",
    dark: "icons/idea/icons/ui/questionDialog_dark.svg",
    has_dark: true,
};
pub const RECORD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/record.svg",
    dark: "icons/idea/icons/files/record_dark.svg",
    has_dark: true,
};
pub const REFRESH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/refresh.svg",
    dark: "icons/idea/icons/ui/refresh_dark.svg",
    has_dark: true,
};
pub const RESOURCES_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/folders/resourcesRoot.svg",
    dark: "icons/idea/icons/folders/resourcesRoot_dark.svg",
    has_dark: true,
};
pub const ROWS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/splitHorizontally.svg",
    dark: "icons/idea/icons/ui/splitHorizontally_dark.svg",
    has_dark: true,
};
pub const RST_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/rst.svg",
    dark: "icons/idea/icons/files/rst_dark.svg",
    has_dark: true,
};
pub const SCISSORS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/cut.svg",
    dark: "icons/idea/icons/ui/cut_dark.svg",
    has_dark: true,
};
pub const SHELL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/shell.svg",
    dark: "icons/idea/icons/files/shell_dark.svg",
    has_dark: true,
};
pub const SIDEBAR_SIMPLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/layout.svg",
    dark: "icons/idea/icons/ui/layout_dark.svg",
    has_dark: true,
};
pub const SIGN_OUT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/exit.svg",
    dark: "icons/idea/icons/ui/exit_dark.svg",
    has_dark: true,
};
pub const SOURCE_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/folders/sourceRoot.svg",
    dark: "icons/idea/icons/folders/sourceRoot_dark.svg",
    has_dark: true,
};
pub const SQL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/sql.svg",
    dark: "icons/idea/icons/files/sql_dark.svg",
    has_dark: true,
};
pub const STOP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/stop.svg",
    dark: "icons/idea/icons/ui/stop_dark.svg",
    has_dark: true,
};
pub const SWIFT_LANG_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/swiftLang.svg",
    dark: "icons/idea/icons/files/swiftLang_dark.svg",
    has_dark: true,
};
pub const TABLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/dataTables.svg",
    dark: "icons/idea/icons/ui/dataTables_dark.svg",
    has_dark: true,
};
pub const TERRAFORM_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/terraform.svg",
    dark: "icons/idea/icons/files/terraform_dark.svg",
    has_dark: true,
};
pub const TEST_RESOURCES_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/folders/testResourcesRoot.svg",
    dark: "icons/idea/icons/folders/testResourcesRoot_dark.svg",
    has_dark: true,
};
pub const TEST_ROOT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/folders/testRoot.svg",
    dark: "icons/idea/icons/folders/testRoot_dark.svg",
    has_dark: true,
};
pub const TOML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/toml.svg",
    dark: "icons/idea/icons/files/toml_dark.svg",
    has_dark: true,
};
pub const TRANSLATE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/language.svg",
    dark: "icons/idea/icons/ui/language_dark.svg",
    has_dark: true,
};
pub const TRASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/delete.svg",
    dark: "icons/idea/icons/ui/delete_dark.svg",
    has_dark: true,
};
pub const TREE_STRUCTURE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/tree.svg",
    dark: "icons/idea/icons/ui/tree_dark.svg",
    has_dark: true,
};
pub const TYPE_SCRIPT_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/typeScript.svg",
    dark: "icons/idea/icons/files/typeScript_dark.svg",
    has_dark: true,
};
pub const UPLOAD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/upload.svg",
    dark: "icons/idea/icons/ui/upload_dark.svg",
    has_dark: true,
};
pub const USER_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/user.svg",
    dark: "icons/idea/icons/ui/user_dark.svg",
    has_dark: true,
};
pub const USERS_THREE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/groups.svg",
    dark: "icons/idea/icons/ui/groups_dark.svg",
    has_dark: true,
};
pub const VUE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/vue.svg",
    dark: "icons/idea/icons/files/vue.svg",
    has_dark: false,
};
pub const WARNING_CIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/warningDialog.svg",
    dark: "icons/idea/icons/ui/warningDialog_dark.svg",
    has_dark: true,
};
pub const WRENCH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/externalTools.svg",
    dark: "icons/idea/icons/ui/externalTools_dark.svg",
    has_dark: true,
};
pub const WSDL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/wsdl.svg",
    dark: "icons/idea/icons/files/wsdl_dark.svg",
    has_dark: true,
};
pub const X_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/close.svg",
    dark: "icons/idea/icons/ui/close_dark.svg",
    has_dark: true,
};
pub const XCIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/ui/errorDialog.svg",
    dark: "icons/idea/icons/ui/errorDialog_dark.svg",
    has_dark: true,
};
pub const XHTML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/xhtml.svg",
    dark: "icons/idea/icons/files/xhtml_dark.svg",
    has_dark: true,
};
pub const XML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/xml.svg",
    dark: "icons/idea/icons/files/xml_dark.svg",
    has_dark: true,
};
pub const XSD_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/xsd.svg",
    dark: "icons/idea/icons/files/xsd_dark.svg",
    has_dark: true,
};
pub const YAML_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/icons/files/yaml.svg",
    dark: "icons/idea/icons/files/yaml_dark.svg",
    has_dark: true,
};

pub const ALIASES: [(&str, IdeaIcon); 16] = [
    ("ChevronDownIcon", CARET_DOWN_ICON),
    ("ChevronLeftIcon", CARET_LEFT_ICON),
    ("ChevronRightIcon", CARET_RIGHT_ICON),
    ("ChevronUpIcon", CARET_UP_ICON),
    ("CopySimpleIcon", COPY_ICON),
    ("DownloadSimpleIcon", DOWNLOAD_ICON),
    ("FileTextIcon", FILE_ICON),
    ("GearSixIcon", GEAR_ICON),
    ("GlobeIcon", GLOBE_HEMISPHERE_WEST_ICON),
    ("OpenExternalIcon", ARROW_SQUARE_OUT_ICON),
    ("PencilIcon", PEN_ICON),
    ("PencilLineIcon", PEN_ICON),
    ("PencilSimpleIcon", PEN_ICON),
    ("PencilSimpleLineIcon", PEN_ICON),
    ("UploadSimpleIcon", UPLOAD_ICON),
    ("WarningIcon", WARNING_CIRCLE_ICON),
];
