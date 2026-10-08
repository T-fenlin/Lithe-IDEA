use std::collections::HashSet;

use gpui::{
    App, AssetSource, Global, Hsla, IntoElement, Radians, RenderOnce, SharedString,
    StyleRefinement, Transformation, Window, prelude::*, size, svg,
};

use crate::theme::{ActiveTheme, IconSize};

macro_rules! icons {
    ($($variant:ident => $file:literal,)*) => {
        /// Lucide icons bundled with Ely.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum IconName {
            $($variant,)*
        }

        impl IconName {
            pub const ALL: &[IconName] = &[$(IconName::$variant,)*];

            pub fn path(self) -> &'static str {
                match self {
                    $(IconName::$variant => concat!("icons/", $file, ".svg"),)*
                }
            }

            /// Lucide name, such as `chevron-down`.
            pub fn name(self) -> &'static str {
                match self {
                    $(IconName::$variant => $file,)*
                }
            }
        }
    };
}

icons! {
    AArrowDown => "a-arrow-down",
    AArrowUp => "a-arrow-up",
    Accessibility => "accessibility",
    Activity => "activity",
    AlarmClock => "alarm-clock",
    AlignCenter => "align-center",
    AlignCenterHorizontal => "align-center-horizontal",
    AlignCenterVertical => "align-center-vertical",
    AlignEndHorizontal => "align-end-horizontal",
    AlignEndVertical => "align-end-vertical",
    AlignHorizontalDistributeCenter => "align-horizontal-distribute-center",
    AlignJustify => "align-justify",
    AlignLeft => "align-left",
    AlignRight => "align-right",
    AlignStartHorizontal => "align-start-horizontal",
    AlignStartVertical => "align-start-vertical",
    AlignVerticalDistributeCenter => "align-vertical-distribute-center",
    AppWindow => "app-window",
    Archive => "archive",
    ArrowDownLeft => "arrow-down-left",
    ArrowDownUp => "arrow-down-up",
    ArrowDownToDot => "arrow-down-to-dot",
    ArrowDownToLine => "arrow-down-to-line",
    ArrowDownWideNarrow => "arrow-down-wide-narrow",
    ArrowDown => "arrow-down",
    ArrowLeft => "arrow-left",
    ArrowRight => "arrow-right",
    ArrowUpFromDot => "arrow-up-from-dot",
    ArrowUpNarrowWide => "arrow-up-narrow-wide",
    ArrowUpRight => "arrow-up-right",
    ArrowUp => "arrow-up",
    AtSign => "at-sign",
    AudioLines => "audio-lines",
    Ban => "ban",
    Battery => "battery",
    Bell => "bell",
    Bold => "bold",
    BookOpen => "book-open",
    Bookmark => "bookmark",
    Bot => "bot",
    Box => "box",
    Braces => "braces",
    Brush => "brush",
    Bug => "bug",
    CalendarDays => "calendar-days",
    CalendarRange => "calendar-range",
    Calendar => "calendar",
    Camera => "camera",
    Captions => "captions",
    CaptionsOff => "captions-off",
    CaseSensitive => "case-sensitive",
    ChartArea => "chart-area",
    ChartBar => "chart-bar",
    ChartCandlestick => "chart-candlestick",
    ChartColumn => "chart-column",
    ChartLine => "chart-line",
    ChartPie => "chart-pie",
    CheckCheck => "check-check",
    ChartGantt => "chart-gantt",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    ChevronsLeft => "chevrons-left",
    ChevronsLeftRight => "chevrons-left-right",
    ChevronsRight => "chevrons-right",
    ChevronsUpDown => "chevrons-up-down",
    CircleAlert => "circle-alert",
    CircleCheck => "circle-check",
    CircleDashed => "circle-dashed",
    CircleDot => "circle-dot",
    CircleDotDashed => "circle-dot-dashed",
    CircleHelp => "circle-help",
    CirclePause => "circle-pause",
    CircleStop => "circle-stop",
    CircleX => "circle-x",
    Circle => "circle",
    ClipboardCheck => "clipboard-check",
    Clipboard => "clipboard",
    Clock => "clock",
    CloudAlert => "cloud-alert",
    CloudCheck => "cloud-check",
    CloudOff => "cloud-off",
    Cloud => "cloud",
    CloudUpload => "cloud-upload",
    Code => "code",
    Columns2 => "columns-2",
    Columns3 => "columns-3",
    Command => "command",
    Compass => "compass",
    Contrast => "contrast",
    Copy => "copy",
    CornerDownRight => "corner-down-right",
    Cpu => "cpu",
    CreditCard => "credit-card",
    Crop => "crop",
    Crosshair => "crosshair",
    Database => "database",
    Delete => "delete",
    Dices => "dices",
    DollarSign => "dollar-sign",
    Dot => "dot",
    Download => "download",
    EllipsisVertical => "ellipsis-vertical",
    Ellipsis => "ellipsis",
    Eraser => "eraser",
    ExternalLink => "external-link",
    EyeOff => "eye-off",
    Eye => "eye",
    FileArchive => "file-archive",
    FileAudio => "file-audio",
    FileCode => "file-code",
    FileCog => "file-cog",
    FileImage => "file-image",
    FileJson => "file-json",
    FilePlus => "file-plus",
    FileSpreadsheet => "file-spreadsheet",
    FileTerminal => "file-terminal",
    FileText => "file-text",
    File => "file",
    FileType => "file-type",
    FileVideoCamera => "file-video-camera",
    Filter => "filter",
    Fingerprint => "fingerprint",
    Flag => "flag",
    FolderOpen => "folder-open",
    FolderPlus => "folder-plus",
    Folder => "folder",
    Forward => "forward",
    Frame => "frame",
    Frown => "frown",
    Fullscreen => "fullscreen",
    Gauge => "gauge",
    GitBranch => "git-branch",
    GitCommitHorizontal => "git-commit-horizontal",
    GitMerge => "git-merge",
    GitPullRequest => "git-pull-request",
    GitPullRequestClosed => "git-pull-request-closed",
    GitPullRequestDraft => "git-pull-request-draft",
    Globe => "globe",
    GripHorizontal => "grip-horizontal",
    GripVertical => "grip-vertical",
    Hand => "hand",
    HardDrive => "hard-drive",
    Hash => "hash",
    Heading => "heading",
    Heading1 => "heading-1",
    Heading2 => "heading-2",
    Heading3 => "heading-3",
    Headphones => "headphones",
    Heart => "heart",
    Hexagon => "hexagon",
    History => "history",
    Hourglass => "hourglass",
    House => "house",
    Image => "image",
    ImageOff => "image-off",
    Inbox => "inbox",
    Info => "info",
    Italic => "italic",
    Kanban => "kanban",
    Key => "key",
    Keyboard => "keyboard",
    Languages => "languages",
    Laugh => "laugh",
    Layers => "layers",
    LayoutDashboard => "layout-dashboard",
    LayoutGrid => "layout-grid",
    Library => "library",
    Lightbulb => "lightbulb",
    Link => "link",
    ListChecks => "list-checks",
    ListOrdered => "list-ordered",
    ListTodo => "list-todo",
    List => "list",
    LoaderCircle => "loader-circle",
    LockOpen => "lock-open",
    Lock => "lock",
    LogIn => "log-in",
    LogOut => "log-out",
    Mail => "mail",
    MapPin => "map-pin",
    Map => "map",
    Maximize2 => "maximize-2",
    Meh => "meh",
    MemoryStick => "memory-stick",
    Menu => "menu",
    Merge => "merge",
    MessageSquare => "message-square",
    MessageSquareDiff => "message-square-diff",
    MicOff => "mic-off",
    Mic => "mic",
    Minimize2 => "minimize-2",
    Minus => "minus",
    Monitor => "monitor",
    Moon => "moon",
    MousePointer2 => "mouse-pointer-2",
    Move => "move",
    MoveUpRight => "move-up-right",
    Music => "music",
    Network => "network",
    NotebookPen => "notebook-pen",
    OctagonAlert => "octagon-alert",
    Package => "package",
    Palette => "palette",
    PanelBottom => "panel-bottom",
    PanelLeft => "panel-left",
    PanelRight => "panel-right",
    Paperclip => "paperclip",
    Pause => "pause",
    PenLine => "pen-line",
    PenTool => "pen-tool",
    Pencil => "pencil",
    PhoneOff => "phone-off",
    Phone => "phone",
    PictureInPicture2 => "picture-in-picture-2",
    Pipette => "pipette",
    PinOff => "pin-off",
    Pin => "pin",
    Play => "play",
    Plus => "plus",
    Power => "power",
    Printer => "printer",
    Puzzle => "puzzle",
    Quote => "quote",
    Redo2 => "redo-2",
    RedoDot => "redo-dot",
    RefreshCw => "refresh-cw",
    Regex => "regex",
    Repeat => "repeat",
    Repeat1 => "repeat-1",
    Reply => "reply",
    ReplyAll => "reply-all",
    Rocket => "rocket",
    RotateCcw => "rotate-ccw",
    RotateCw => "rotate-cw",
    Rows2 => "rows-2",
    Save => "save",
    Scissors => "scissors",
    ScreenShare => "screen-share",
    ScreenShareOff => "screen-share-off",
    Search => "search",
    SearchX => "search-x",
    Send => "send",
    Server => "server",
    Settings => "settings",
    Shapes => "shapes",
    Share2 => "share-2",
    Shield => "shield",
    Shuffle => "shuffle",
    SkipBack => "skip-back",
    SkipForward => "skip-forward",
    Slash => "slash",
    SlidersHorizontal => "sliders-horizontal",
    Smartphone => "smartphone",
    Smile => "smile",
    SmilePlus => "smile-plus",
    Sparkles => "sparkles",
    Spline => "spline",
    SquareCheck => "square-check",
    Square => "square",
    SquareFunction => "square-function",
    SquarePen => "square-pen",
    SquareRoundCorner => "square-round-corner",
    Star => "star",
    StepForward => "step-forward",
    StickyNote => "sticky-note",
    Strikethrough => "strikethrough",
    Sun => "sun",
    Table => "table",
    Tag => "tag",
    Target => "target",
    Terminal => "terminal",
    ThumbsDown => "thumbs-down",
    ThumbsUp => "thumbs-up",
    Timer => "timer",
    Trash2 => "trash-2",
    TrendingDown => "trending-down",
    TrendingUp => "trending-up",
    TriangleAlert => "triangle-alert",
    Type => "type",
    Underline => "underline",
    Undo2 => "undo-2",
    Upload => "upload",
    User => "user",
    UserPlus => "user-plus",
    Users => "users",
    Variable => "variable",
    VideoOff => "video-off",
    Video => "video",
    Volume => "volume",
    Volume1 => "volume-1",
    Volume2 => "volume-2",
    VolumeX => "volume-x",
    Wallet => "wallet",
    WandSparkles => "wand-sparkles",
    WholeWord => "whole-word",
    WifiOff => "wifi-off",
    Wifi => "wifi",
    Workflow => "workflow",
    Wrench => "wrench",
    X => "x",
    Zap => "zap",
    ZoomIn => "zoom-in",
    ZoomOut => "zoom-out",
}

#[derive(IntoElement)]
pub struct Icon {
    /// 调用方用 gpui 的样式助手给的细化（`.size_10()` / `.absolute()` …）；见本文件的 `Styled` 实现。
    style: StyleRefinement,
    path: SharedString,
    bundled: bool,
    size: IconSize,
    color: Option<Hsla>,
    hover: Option<(SharedString, Hsla)>,
    rotation: Option<Radians>,
    scale: f32,
}

/// 上游 `gpui-component` 的 `Icon` 是 `Styled`（宿主拿它当元素用，`.size_10()` / `.absolute()`
/// 这类 gpui 助手直接落在图上），我们这份丢了这一条。补回来：**档位（[`IconSize`]）是库内的
/// 默认值，调用方用 gpui 助手细化过就以后者为准**——否则宿主那句 `icon.size_10()` 会编译不过，
/// 或者要为了 40px 这种档位外的尺寸在调用点套一层 div 而改变几何。
impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }

    /// 颜色收进图标自己的颜色槽，而不是样式细化：`render` 只用一处决定画什么颜色。
    fn text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            bundled: true,
            ..Self::from_path(name.path())
        }
    }

    /// Draws an SVG supplied by the application's asset source; a missing one fails at render.
    pub fn from_path(path: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            path: path.into(),
            bundled: false,
            size: IconSize::Md,
            color: None,
            hover: None,
            rotation: None,
            scale: 1.0,
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Defaults to `colors.fg`.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Color while the pointer is over `group`.
    pub fn group_hover_color(
        mut self,
        group: impl Into<SharedString>,
        color: impl Into<Hsla>,
    ) -> Self {
        self.hover = Some((group.into(), color.into()));
        self
    }

    pub fn rotate(mut self, angle: impl Into<Radians>) -> Self {
        self.rotation = Some(angle.into());
        self
    }

    /// Draws the glyph larger or smaller inside the same box.
    pub fn scale(mut self, factor: f32) -> Self {
        self.scale = factor;
        self
    }
}

impl From<IconName> for Icon {
    fn from(name: IconName) -> Self {
        Icon::new(name)
    }
}

/// App paths already found in the asset source.
#[derive(Default)]
struct FoundIcons(HashSet<SharedString>);

impl Global for FoundIcons {}

/// 调用方细化过尺寸（gpui 助手，如 `.size_10()`）就不用 [`IconSize`] 档位；宽度或高度给了一个就算。
fn uses_style_size(style: &StyleRefinement) -> bool {
    style.size.width.is_some() || style.size.height.is_some()
}

fn ensure_found(source: &dyn AssetSource, path: &str) {
    match source.load(path) {
        Ok(Some(_)) => {}
        Ok(None) => panic!("lithe-ui: icon {path} is not in the app's asset source"),
        Err(error) => panic!("lithe-ui: icon {path} failed to load: {error:#}"),
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.bundled && !cx.default_global::<FoundIcons>().0.contains(&self.path) {
            ensure_found(cx.asset_source().as_ref(), &self.path);
            cx.default_global::<FoundIcons>()
                .0
                .insert(self.path.clone());
        }
        let theme = cx.theme();
        let color = self.color.unwrap_or(theme.colors.fg);
        // 调用方细化过尺寸就听它的；没细化才用档位（见上面 `Styled` 的说明）。
        let sized = uses_style_size(&self.style);
        let side = theme.icon_size(self.size);
        svg()
            .map(|mut svg| {
                *svg.style() = self.style;
                svg
            })
            .path(self.path)
            .flex_none()
            .text_color(color)
            .when(!sized, |svg| svg.size(side))
            .when_some(self.hover, |svg, (group, hover)| {
                svg.group_hover(group, |style| style.text_color(hover))
            })
            .when(self.rotation.is_some() || self.scale != 1.0, |svg| {
                svg.with_transformation(
                    Transformation::rotate(self.rotation.unwrap_or(Radians(0.0)))
                        .with_scaling(size(self.scale, self.scale)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Hsla, Styled as _};

    use super::{Icon, IconName, ensure_found, uses_style_size};
    use crate::theme::IconSize;

    #[test]
    fn every_icon_ships_in_the_bundle() {
        for icon in IconName::ALL {
            assert!(
                crate::Assets::get(icon.path()).is_some(),
                "{icon:?} missing at {}",
                icon.path()
            );
        }
    }

    #[test]
    fn an_app_path_draws_as_given_and_a_name_as_its_bundled_file() {
        let custom = Icon::from_path("app-icons/mark.svg");
        assert_eq!(custom.path.as_ref(), "app-icons/mark.svg");
        assert!(!custom.bundled);
        let named = Icon::new(IconName::Check);
        assert_eq!(named.path.as_ref(), IconName::Check.path());
        assert!(named.bundled);
    }

    #[test]
    fn a_path_the_source_holds_is_found() {
        ensure_found(&crate::Assets, IconName::Check.path());
    }

    #[test]
    fn a_size_step_and_a_gpui_style_helper_do_not_fight() {
        let plain = Icon::new(IconName::Check);
        assert_eq!(plain.size, IconSize::Md, "档位是库内的默认值");
        assert!(plain.color.is_none(), "没给颜色就用主题的前景");
        assert!(!uses_style_size(&plain.style), "没细化过尺寸，就听档位");

        let styled = Icon::new(IconName::Check)
            .size_10()
            .text_color(Hsla::default());
        assert!(uses_style_size(&styled.style), "细化过尺寸，就听细化的");
        assert!(
            styled.color.is_some(),
            "颜色收进图标自己的槽，不进样式细化：`render` 只从一处取颜色"
        );
    }

    #[test]
    #[should_panic(
        expected = "lithe-ui: icon app-icons/missing.svg is not in the app's asset source"
    )]
    fn a_path_the_source_lacks_fails() {
        ensure_found(&crate::Assets, "app-icons/missing.svg");
    }
}
