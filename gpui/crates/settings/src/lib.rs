//! Lithe 的 GPUI 设置：数据模型、持久化、主题应用与设置对话框。
//!
//! ## 职责与依赖方向
//!
//! 本 crate **只向下依赖** `gpui-kit` + `lithe-gpui-shared`（要 `tr` / `tr_args`）+ `serde` /
//! `serde_json`（`gpui-kit` 不重导出后两者，见 `Cargo.toml` 的注释）。
//! **不依赖** `lithe-gpui-workbench` / `lithe-gpui-app`：设置是"被别处读"的东西，
//! 反过来依赖外壳会成环。`workbench` 读设置只有两条路：
//!
//! 1. [`store`] 拿到 [`SettingsStore`] 这个 `Entity`，`cx.observe(&store, ..)` 后重绘
//!    （状态栏的「显示状态栏」就是这么接的）；
//! 2. [`Settings::show_status_bar`] 之类的纯字段读取。
//!
//! ## 文件分工
//!
//! | 文件 | 职责 |
//! | --- | --- |
//! | [`schema`] | `Settings` 结构体、逐键默认值、不依赖运行时的规范化（**纯数据，无 GPUI**） |
//! | [`paths`] | 设置文件路径（唯一允许出现 `#[cfg(target_os)]` 的地方）+ 环境变量覆盖 |
//! | [`persistence`] | 读（逐键容错）、原子写、300ms 防抖状态机（**纯数据，无 GPUI**） |
//! | [`theme`] | 主题目录装载/监听、按名字应用主题、UI 字号 → rem 基准（真源是 `gpui/themes/`） |
//! | [`store`] | 状态所有者 `SettingsStore`：改设置 → 立即生效 → 防抖落盘（唯一需要 `App` 的"逻辑"） |
//! | [`row`] | 行/分组零件（`SettingsGroup` / `SettingsRow` / 宽度档） |
//! | [`identity`] | 「Git」页的数据边界：宿主钩子（`GitIdentityHost`）+ 身份值的纯判据 |
//! | [`dialog`] | 设置对话框（820×620 模态）：头部 / 分类栏 / 内容页 / 底部 + 确认子对话框 |
//! | [`restart`] | 切换语言后的"用相同参数重启自己"（语言无法运行中热切，见该模块文档） |
//!
//! ## 启动顺序（`main.rs` 里的调用次序是有要求的）
//!
//! ```text
//! let loaded = lithe_gpui_settings::load();          // 纯读文件，不需要 App
//! gpui_kit::component::set_locale(..);               // 必须在 gpui_kit::init 之前
//! gpui_kit::init(cx);
//! lithe_gpui_settings::init_store(cx, Init { .. });  // 登记 Global + 应用字号（主题留给下一步）
//! lithe_gpui_settings::watch_lithe_themes(cx);       // 装载 + 监听 gpui/themes/，回调里应用主题
//! lithe_gpui_settings::install_actions(cx);          // Ctrl+, → 打开设置
//! ```
//!
//! 主题为什么不在 `init_store` 里应用：`watch_dir` 是**异步**装载的
//! （`gpui-component-0.6.6/src/theme/registry.rs:104-115`），启动那一刻注册表里还没有
//! `Lithe Dark`；让装载回调统一应用，启动时 `S1_THEME applied=` 就只会出现一次。

pub mod dialog;
pub mod identity;
pub mod paths;
pub mod persistence;
pub mod project;
pub mod restart;
pub mod row;
pub mod schema;
pub mod store;
pub mod theme;

pub use dialog::{
    Category, OpenSettings, install_actions, open_settings_dialog, open_settings_dialog_at,
};
pub use identity::{
    GitIdentityHost, GitIdentityPage, IdentityField, IdentityScope, IdentitySetup,
    IDENTITY_MAX_BYTES, git_identity_page, identity_value_is_valid, set_git_identity_host,
};
pub use paths::{SETTINGS_FILE_ENV, settings_file_path};
pub use persistence::{DebounceState, Loaded, SAVE_DEBOUNCE_MS, load, load_from};
pub use project::{
    EffectiveToolchain, MavenConfiguration, MavenDiscovery, Overrides, ProjectEnvironment,
    PROJECT_DIAGNOSTIC_TAG, ToolMode, ToolSource, discover as discover_project_environment,
};
pub use restart::restart_application;
pub use schema::Settings;
pub use store::{AppearanceMode, Init, SettingsStore, init_store, store, try_store};
pub use theme::watch_lithe_themes;
