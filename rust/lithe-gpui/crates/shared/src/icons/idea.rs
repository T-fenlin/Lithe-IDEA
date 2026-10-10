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

pub const ALL: [IdeaIcon; 62] = [
    ARROW_CLOCKWISE_ICON,
    ARROW_COUNTER_CLOCKWISE_ICON,
    ARROW_DOWN_ICON,
    ARROW_LEFT_ICON,
    ARROW_RIGHT_ICON,
    ARROW_UP_ICON,
    BELL_ICON,
    BOOK_OPEN_ICON,
    BUG_ICON,
    CARET_DOWN_ICON,
    CARET_LEFT_ICON,
    CARET_RIGHT_ICON,
    CHECK_ICON,
    CHECK_CIRCLE_ICON,
    CLASS_ICON,
    CLOCK_COUNTER_CLOCKWISE_ICON,
    COLUMNS_ICON,
    COPY_ICON,
    DATABASE_ICON,
    DOTS_THREE_ICON,
    DOWNLOAD_ICON,
    EYE_ICON,
    EYE_SLASH_ICON,
    FILE_ICON,
    FILE_PLUS_ICON,
    FILES_ICON,
    FLOPPY_DISK_ICON,
    FOLDER_ICON,
    FOLDER_PLUS_ICON,
    FUNNEL_ICON,
    GEAR_ICON,
    GIT_BRANCH_ICON,
    GIT_DIFF_ICON,
    GLOBE_HEMISPHERE_WEST_ICON,
    HASH_ICON,
    INFO_ICON,
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
    MINUS_ICON,
    MONITOR_ICON,
    PALETTE_ICON,
    PLAY_ICON,
    PLUGIN_ICON,
    PLUS_ICON,
    REFRESH_ICON,
    ROWS_ICON,
    SCISSORS_ICON,
    STOP_ICON,
    TERMINAL_ICON,
    TRASH_ICON,
    VCS_ICON,
    WARNING_CIRCLE_ICON,
    X_ICON,
    XCIRCLE_ICON,
];

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
    dark: "icons/idea/debugger/debug.svg",
    has_dark: false,
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
    dark: "icons/idea/expui/general/chevronRight.svg",
    has_dark: false,
};

pub const CHECK_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/greenCheckmark.svg",
    dark: "icons/idea/legacy/ui/greenCheckmark_dark.svg",
    has_dark: true,
};

pub const CHECK_CIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/successDialog.svg",
    dark: "icons/idea/legacy/ui/successDialog_dark.svg",
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

pub const FILE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/fileTypes/text.svg",
    dark: "icons/idea/fileTypes/text.svg",
    has_dark: false,
};

pub const FILE_PLUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/addFile.svg",
    dark: "icons/idea/legacy/ui/addFile_dark.svg",
    has_dark: true,
};

pub const FILES_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/listFiles.svg",
    dark: "icons/idea/legacy/ui/listFiles_dark.svg",
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

pub const INFO_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/informationDialog.svg",
    dark: "icons/idea/legacy/ui/informationDialog_dark.svg",
    has_dark: true,
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
    dark: "icons/idea/expui/general/search.svg",
    has_dark: false,
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

pub const MINUS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/remove.svg",
    dark: "icons/idea/legacy/ui/remove_dark.svg",
    has_dark: true,
};

pub const MONITOR_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/desktop.svg",
    dark: "icons/idea/legacy/ui/desktop_dark.svg",
    has_dark: true,
};

pub const PALETTE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/palette.svg",
    dark: "icons/idea/legacy/ui/palette_dark.svg",
    has_dark: true,
};

pub const PLAY_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/testState/run.svg",
    dark: "icons/idea/testState/run.svg",
    has_dark: false,
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

pub const REFRESH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/actions/refresh.svg",
    dark: "icons/idea/actions/refresh.svg",
    has_dark: false,
};

pub const ROWS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/splitHorizontally.svg",
    dark: "icons/idea/legacy/ui/splitHorizontally_dark.svg",
    has_dark: true,
};

pub const SCISSORS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/cut.svg",
    dark: "icons/idea/legacy/ui/cut_dark.svg",
    has_dark: true,
};

pub const STOP_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/debugger/stop.svg",
    dark: "icons/idea/debugger/stop.svg",
    has_dark: false,
};

pub const TERMINAL_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/terminal/terminal@20x20.svg",
    dark: "icons/idea/terminal/terminal@20x20_dark.svg",
    has_dark: true,
};

pub const TRASH_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/delete.svg",
    dark: "icons/idea/legacy/ui/delete_dark.svg",
    has_dark: true,
};

pub const VCS_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/expui/toolwindows/vcs@20x20.svg",
    dark: "icons/idea/expui/toolwindows/vcs@20x20_dark.svg",
    has_dark: true,
};

pub const WARNING_CIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/warningDialog.svg",
    dark: "icons/idea/legacy/ui/warningDialog_dark.svg",
    has_dark: true,
};

pub const X_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/close.svg",
    dark: "icons/idea/legacy/ui/close_dark.svg",
    has_dark: true,
};

pub const XCIRCLE_ICON: IdeaIcon = IdeaIcon {
    light: "icons/idea/legacy/ui/errorDialog.svg",
    dark: "icons/idea/legacy/ui/errorDialog_dark.svg",
    has_dark: true,
};
