//! 「最近项目」的数据层：一份**独立 JSON 文件**的读 / 写 / 维护
//! （不碰 GPUI、不碰 UI、**不自己取时钟**）。
//!
//! ## 落盘位置：设置文件的同目录兄弟文件（决策 Q20）
//!
//! ```text
//! %APPDATA%\Lithe\settings.json          ← 扁平标量设置，不动
//! %APPDATA%\Lithe\recent-projects.json   ← 本模块（路径见 crate::paths::recent_projects_file_path）
//! ```
//!
//! 为什么不塞进 [`crate::schema::Settings`]：最近项目是**机器高频改写**的东西，写它会把
//! 用户手改过的设置文件也一起重写一遍（维护者原话：不污染 `Settings` 模型，
//! `gpui/research/menu-and-open-project-plan.md:35`）。设置结构现在允许数组与嵌套
//! （键表由 schema 派生，见 `crate::persistence` 的模块文档），所以这里的理由只剩
//! "高频改写的机器状态不该混进用户手改的文件"这一条。
//!
//! 为什么不落 `.lithe/`：gpui 侧**没有**项目级写入通路（对 `.lithe` 只有一处**读**：
//! `gpui/crates/java/src/jdtls.rs:402-407` 把 `.lithe/toolchains/jdtls` 当探测候选根），
//! 而且最近项目是**跨项目、跨会话**的历史，本来就不属于某一个工作区
//! （侦察 `gpui/research/open-project-and-windows.md:253-272`）。
//!
//! 环境变量 `LITHE_GPUI_SETTINGS_FILE`（[`crate::paths::SETTINGS_FILE_ENV`]）一旦把设置文件
//! 指到临时目录，最近项目也跟着落到**同一个临时目录**里 —— 机器验证因此不会污染真实用户目录。
//!
//! ## 字段与规则：逐条照抄真源
//!
//! 真源是 Windows 侧 `windows/tauri/src/features/file-system/stores/recent-folders.store.ts`
//! 与它的纯函数层 `utils/recent-folders.ts`、类型 `types/recent-folders.types.ts`：
//!
//! | 规则 | 本模块落点 | 真源 |
//! | --- | --- | --- |
//! | 上限 **12**（常量） | [`MAX_RECENT_PROJECTS`] | `utils/recent-folders.ts:3` |
//! | **`pinned` 不占额度**（先排好序，再 `pinned` 全留 + 未 pinned 截 12） | [`normalize`] | `utils/recent-folders.ts:36-42`（`slice` 在 `:39`，拼接在 `:41`） |
//! | 排序：**`pinned` 优先，再 `lastOpenedAt` 降序** | [`normalize`] | `utils/recent-folders.ts:26-34`（`:28-30` pinned 优先、`:32` 时间降序） |
//! | 按 **`path` 精确去重**（**不做**大小写归一） | [`RecentProjects::record_open`] / [`normalize`] | `utils/recent-folders.ts:64-89`（查找 `:69`，去重 `:87`） |
//! | 记录一次打开 = **已有项提到最前 + 更新时间 + 不产生重复项**（已有的 `name` / `pinned` 保留） | [`RecentProjects::record_open`] | 同上（`name: existing?.name ?? getFolderName(path)`、`pinned: existing?.pinned`） |
//! | 显示名 = 路径**末段** | [`folder_name`] | `utils/recent-folders.ts:5-8` |
//! | 字段名（驼峰） | [`RecentProject`] | `types/recent-folders.types.ts:1-13` |
//! | 落盘外层键 | [`RECENT_PROJECTS_KEY`] | `stores/recent-folders.store.ts:194`（`partialize: ({ recentFolders }) => …`） |
//! | 老数据补齐时间戳（解析不出就用当前时间） | [`normalize`] + [`RecentProjects::record_open`] | `stores/recent-folders.store.ts:200-221`（`migrate`） |
//!
//! **只取侦察 §2.4.2 给的"最小可照抄字段集"**（`path` / `name` / `lastOpenedAt` / `pinned` /
//! `missing` / `openInNewWindow`），三个真源字段**故意不带**，理由如下：
//!
//! - `lastOpened`（本地化时间字符串，`new Date(ts).toLocaleString()`）：那是**渲染结果**，
//!   真源自己也只是把它当展示用，排序真源是 `lastOpenedAt`。落一份中文/英文写死的时间串，
//!   切换显示语言后就与界面不一致了；文案一律 `tr`，格式化留给界面。
//! - `activeProjectTabId`：多项目标签条这轮不动（决策 Q21），没有数据来源也没有消费方。
//! - `customIcon` / `importSourceId` / `importSourceName`：自定义图标与「从其它 IDE 导入」
//!   都不在本批（导入需要真源 `importRecentFolders` 那套探测，本批不做）。
//!
//! ## 容错口径：照 `persistence.rs`
//!
//! | 情况 | 行为 |
//! | --- | --- |
//! | 文件不存在 | 空列表，**不创建文件**（与设置文件"改动才写"同一条口径） |
//! | 文件不是合法 JSON / 不是对象 | 空列表 + 一条诊断，**不 panic** |
//! | 外层键缺失 | 空列表（`#[serde(default)]` 的"缺键回落默认值"语义），无诊断 |
//! | 外层键类型坏了 | 空列表 + 一条诊断 |
//! | **某一条记录**坏了（缺 `path`、类型不符） | **只丢那一条** + 一条诊断，其余照用 |
//! | 某条记录缺 `name` | 用 [`folder_name`] 从 `path` 补（不是丢条目） |
//! | 重复 `path` / 超出上限（手改过的文件） | 收口到不变量 + 一条诊断（[`normalize`]） |
//! | 未知键 | 静默忽略 |
//!
//! 诊断词表（可 grep）：`no_recent_projects_path` / `read_failed` / `invalid_json` / `not_an_object` /
//! `bad_key key=recentProjects` / `bad_entry index=N error=…` / `normalized removed=N`。
//! 调用方用 [`LoadedRecentProjects::report`] 打印（`S1_SETTINGS_RECENT …`，与 `S1_SETTINGS` 一族同口径）。
//!
//! ## 「无参数启动」怎么挑根
//!
//! 双击 `Lithe.exe` 时资源管理器不会传任何参数（位置参数在原实现里是**必填**的），所以
//! 这里补一条纯选择逻辑 [`RecentProjects::select_launch_root`]：按列表顺序取**第一条
//! 仍然存在**的项目，并把扫描中判定失效的条目交回调用方去标 `missing`（与项目菜单的
//! 既有行为一致）。文件系统探测由调用方注入，本模块因此仍然不碰 GPUI、也不需要真的
//! 建 / 删目录就能单测。兜底（列表为空 / 全失效时开哪个根）**不在本模块**，
//! 它属于 App Shell 的启动策略，见 `gpui/crates/app/src/main.rs` 的 `resolve_launch_root`。
//!
//! ## 为什么数据层不取时钟
//!
//! 时间戳是 `record_open` 的**入参**（[`now_unix_ms`] 只是"要现取时用哪个"的唯一落点）。
//! 这样整个模块是纯数据 + 纯文件 IO：单测给任意时刻即可，不需要用真实时间同步状态
//! （`.agents/skills/write-stable-tests/SKILL.md` 的硬规则），`lastOpenedAt` 也永远可预期。

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::persistence;

/// 未置顶的最近项目上限（置顶项**不占**这个额度）。
///
/// 真源 `windows/tauri/src/features/file-system/utils/recent-folders.ts:3`
/// （`export const MAX_RECENT_PROJECTS = 12`）。
pub const MAX_RECENT_PROJECTS: usize = 12;

/// 落盘 JSON 的外层键名。照真源的 `partialize: ({ recentFolders }) => ({ recentFolders })`
/// （`stores/recent-folders.store.ts:194`）：本侧的文件名已经说明了内容，键名仍用复数容器，
/// 将来要加同级元数据（版本号之类）不必改文件格式。
pub const RECENT_PROJECTS_KEY: &str = "recentProjects";

/// 本模块诊断行的前缀（可 grep；与 `S1_SETTINGS` / `S1_SETTINGS_PROJECT` 一族同口径）。
pub const RECENT_PROJECTS_DIAGNOSTIC_TAG: &str = "S1_SETTINGS_RECENT";

/// 一条「最近项目」记录。字段名与真源类型逐字一致（驼峰），见模块文档的字段取舍说明。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    /// 显示名。真源 `getFolderName(path)` = 路径末段；已有记录的 `name` 在重新打开时**不改**
    /// （真源 `existing?.name ?? getFolderName(folderPath)`），所以项目目录改名后列表里仍是旧名。
    ///
    /// `#[serde(default)]`：文件里缺这个键时用空串进来，由 [`normalize`] 从 `path` 补出末段
    /// —— 只因为少一个展示字段就丢掉一整条记录是不划算的（真源 `migrate` 也是尽量补齐而不是丢）。
    #[serde(default)]
    pub name: String,
    /// 项目根目录的**原生绝对路径**，也是去重键。真源按 `path` **精确**比较（不做大小写归一），
    /// 本模块照抄：Windows 上 `C:\Proj` 与 `c:\proj` 是**两条**记录（见模块测试）。
    pub path: String,
    /// 上次打开时刻（Unix 毫秒）。排序真源（真源 `lastOpenedAt?: number`）。
    ///
    /// 为什么用毫秒整数而不是真源的本地化字符串 `lastOpened`：见模块文档的字段取舍说明。
    #[serde(default)]
    pub last_opened_at: u64,
    /// 置顶。真源 `pinned?: boolean`（缺席 = `false`，真源到处用 `!!folder.pinned` 判真值）。
    #[serde(default)]
    pub pinned: bool,
    /// 目录已失效（真源 `missing?: boolean`）。由"点击最近项目"那条链路探测到目录不在时置位
    /// （真源 `openRecentFolder` 的 `updateRecentFolder(.., {missing: true})`，
    /// `stores/recent-folders.store.ts:106-120`）；本模块只提供 [`RecentProjects::set_missing`]。
    #[serde(default)]
    pub missing: bool,
    /// 上次是用「新窗口」打开的（真源 `openInNewWindow?: boolean`）。
    /// `None` = 从没记录过（真源里这个字段本身可选），序列化时省略这个键。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_in_new_window: Option<bool>,
}

/// 最近项目列表，**恒满足不变量**：按 `path` 去重 · `pinned` 优先 + `lastOpenedAt` 降序 ·
/// `pinned` 全留、未 pinned 最多 [`MAX_RECENT_PROJECTS`] 条。
///
/// 字段私有：顺序与上限是这份数据的不变量，只能通过下面这几个方法改动
/// （每个方法结束都会重新走一遍 [`normalize`]）；反序列化路径也走 [`normalize`]
/// （见 `deserialize_entries`），所以从**任何**入口拿到的实例都是规范化的。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProjects {
    /// 落盘键名 = [`RECENT_PROJECTS_KEY`]。
    ///
    /// `default`：文件里没有这个键 → 空列表（"缺键回落默认值"）。
    /// `deserialize_with`：**读进来就规范化**，包括 `serde_json::from_str::<RecentProjects>` 这条
    /// 不经过 [`load_from_str`] 的路径。
    #[serde(
        default,
        rename = "recentProjects",
        deserialize_with = "deserialize_entries"
    )]
    entries: Vec<RecentProject>,
}

/// 「无位置参数启动」时挑启动根的结果（纯数据）。
///
/// `Lithe` 双击启动（Explorer 传不进任何参数）时没有工作区根，只能从最近项目里挑一个。
/// 这个结构体把"挑了谁"和"扫描中发现谁已经失效"分开返回，于是调用方可以照项目菜单
/// 既有行为把失效条目标成 `missing`，而**不必**在选根逻辑里做任何文件写入。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchRootPick {
    /// 选中的启动根 = 列表里**第一条真的存在**的项目路径（列表顺序 = 界面顺序）。
    /// `None` = 列表为空，或者所有条目都已失效（调用方据此走兜底，**不得静默退出**）。
    pub root: Option<String>,
    /// 扫描中判定为**已失效**的条目路径（按列表顺序，含排在选中项之后的那些）。
    /// 调用方逐条 [`RecentProjects::set_missing`] 后落盘，与项目菜单点开失效条目同一行为。
    pub missing: Vec<String>,
}

impl RecentProjects {
    /// 空列表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从最近项目里挑「双击启动」要打开的根（**纯选择逻辑**：不碰文件系统、不取时钟、
    /// 不做落盘）。
    ///
    /// 「存在」由调用方通过 `is_dir` 注入（生产代码传 `Path::is_dir`，单测传闭包），
    /// 所以这条选择逻辑可以在不构造 GPUI、不真的建 / 删目录的情况下单测
    /// —— 与模块文档那条"数据层不取时钟"同一口径。
    ///
    /// 顺序口径：**按 [`Self::entries`] 的顺序取第一条存在的**，也就是用户在项目下拉里
    /// 看到的顺序（`pinned` 优先，再 `lastOpenedAt` 降序）。因此
    /// - 没有置顶项时，选中的就是"最近且仍然存在的那个项目"；
    /// - 有置顶项且它还在时，优先打开置顶的那个（用户的置顶意图比"最近"更强，
    ///   否则每次双击都会无视他明确钉住的项目）。
    ///
    /// 扫描**不提前退出**：即使已经选中了某一条，剩下的条目仍然会被探测一遍，这样
    /// "列表里所有失效条目都标出来"这件事与"选中哪一条"无关（列表最多十几条，代价可忽略）。
    pub fn select_launch_root(&self, mut is_dir: impl FnMut(&str) -> bool) -> LaunchRootPick {
        let mut pick = LaunchRootPick::default();
        for entry in &self.entries {
            if is_dir(&entry.path) {
                if pick.root.is_none() {
                    pick.root = Some(entry.path.clone());
                }
            } else {
                pick.missing.push(entry.path.clone());
            }
        }
        pick
    }

    /// 列出最近项目，顺序 = 界面上该显示的顺序（`pinned` 优先 + `lastOpenedAt` 降序）。
    pub fn entries(&self) -> &[RecentProject] {
        &self.entries
    }

    /// 条数（含 `pinned` 项）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否一条都没有（界面据此画真源那个空态 `noRecentProjects`）。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按 `path` 精确查一条。
    pub fn find(&self, path: &str) -> Option<&RecentProject> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// **记录一次「打开项目」**（真源 `addToRecents` / `upsertRecentFolder`，
    /// `utils/recent-folders.ts:64-89`）。
    ///
    /// - 已有同 `path` 的项：**不新增**，更新 `lastOpenedAt`、`missing` 复位为 `false`、
    ///   按 `open_in_new_window` 更新打开方式，并保留原来的 `name` / `pinned`；
    ///   新项插到队首，随后由 [`normalize`] 按"pinned 优先 + 时间降序"落位
    ///   （时间戳相同则稳定排序把刚打开的这条留在最前）。
    /// - `opened_at_ms` 由调用方给（现取用 [`now_unix_ms`]）；`open_in_new_window = None`
    ///   表示"这次不更新打开方式"（真源 `metadata.openInNewWindow ?? existing?.openInNewWindow`）。
    /// - 返回 `false` = **没有记录**（`path` 是空的或只有空白）；空路径不写进列表，
    ///   否则会产生一条去重键为空、永远点不开的记录。
    pub fn record_open(
        &mut self,
        path: &str,
        opened_at_ms: u64,
        open_in_new_window: Option<bool>,
    ) -> bool {
        if path.trim().is_empty() {
            return false;
        }

        let existing = self.entries.iter().find(|entry| entry.path == path);
        let next = RecentProject {
            name: existing
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| folder_name(path)),
            path: path.to_string(),
            last_opened_at: opened_at_ms,
            pinned: existing.map(|entry| entry.pinned).unwrap_or(false),
            // 真源 `missing: metadata.missing ?? false`：重新记录一次打开就是"它又能用了"。
            missing: false,
            open_in_new_window: open_in_new_window
                .or_else(|| existing.and_then(|entry| entry.open_in_new_window)),
        };

        // 先按 path 去掉旧项，再插到队首 —— 这就是真源 `[nextFolder, ...folders.filter(..)]`
        // 那一步；不这么做就会留下重复项（真源的 `filter` 在 `utils/recent-folders.ts:87`）。
        self.entries.retain(|entry| entry.path != path);
        self.entries.insert(0, next);
        self.entries = normalize(std::mem::take(&mut self.entries));
        true
    }

    /// 设置置顶（`pinned` 不占额度，置顶后一定留在列表里）。
    ///
    /// 返回 `false` = 列表里没有这条 `path`（调用方据此打诊断，而不是静默无操作）。
    pub fn set_pinned(&mut self, path: &str, pinned: bool) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) else {
            return false;
        };
        entry.pinned = pinned;
        self.entries = normalize(std::mem::take(&mut self.entries));
        true
    }

    /// 切换置顶（真源 `togglePinned`，`utils/recent-folders.ts:108-119`）。
    pub fn toggle_pinned(&mut self, path: &str) -> bool {
        let Some(entry) = self.entries.iter().find(|entry| entry.path == path) else {
            return false;
        };
        let pinned = !entry.pinned;
        self.set_pinned(path, pinned)
    }

    /// 标记目录是否失效（真源 `updateRecentFolder(path, {missing})`，
    /// `stores/recent-folders.store.ts:179-187`）。返回 `false` = 没有这条 `path`。
    pub fn set_missing(&mut self, path: &str, missing: bool) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) else {
            return false;
        };
        entry.missing = missing;
        true
    }

    /// 移除一项（真源 `removeFromRecents`）。返回 `false` = 本来就没有这条 `path`。
    pub fn remove(&mut self, path: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.path != path);
        self.entries.len() != before
    }

    /// 清空（真源 `clearRecents`）。
    ///
    /// 清空后列表里一条都没有；调用方照常 [`save`]，落盘的是 `{"recentProjects": []}` ——
    /// **不是**删文件（删文件与"从没记录过"就区分不开了；下一次读取同样得到空列表）。
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// 从 `path` 取显示名（真源 `getFolderName`，`utils/recent-folders.ts:5-8`）。
///
/// 逐条照抄真源的取分隔符规则：**含 `\` 就按 `\` 切，否则按 `/` 切**，丢掉空段后取最后一段；
/// 一段都不剩（空串、`/`、`///`）时回落到整条路径（真源 `|| folderPath`）。
pub fn folder_name(path: &str) -> String {
    let separator = if path.contains('\\') { '\\' } else { '/' };
    path.split(separator)
        .filter(|segment| !segment.is_empty())
        .next_back()
        .unwrap_or(path)
        .to_string()
}

/// 现在（Unix 毫秒）。要求现取时刻时**只用这一个落点**（系统时钟早于 1970 时回落到 `0`，
/// 排序照样稳定，不会 panic）。
pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// 一次读取的结果（形状照 [`crate::persistence::Loaded`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedRecentProjects {
    /// 规范化之后的列表。**任何失败路径都返回可用列表**（最差是空列表）。
    pub projects: RecentProjects,
    /// 文件路径；`None` = 推导不出路径（只影响能否落盘）。
    pub path: Option<PathBuf>,
    /// 文件是否真的存在且被读到了（不存在 = 首次启动）。
    pub file_existed: bool,
    /// 诊断（坏 JSON、坏条目、路径推导失败等）。调用方负责打印。
    pub diagnostics: Vec<String>,
}

impl LoadedRecentProjects {
    /// 一行可 grep 的汇总（形状照 `store.rs` 的 `S1_SETTINGS loaded …`）。
    ///
    /// 形如：
    /// ```text
    /// S1_SETTINGS_RECENT loaded path=C:\...\recent-projects.json count=13 pinned=1 fileExisted=true diagnostics=0
    /// ```
    /// 界面上「最近项目」有几条、其中几条置顶，都能在这一行里核对。
    pub fn diagnostic_line(&self) -> String {
        let path_text = self
            .path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(none)".to_string());
        let pinned = self
            .projects
            .entries()
            .iter()
            .filter(|entry| entry.pinned)
            .count();
        format!(
            "{RECENT_PROJECTS_DIAGNOSTIC_TAG} loaded path={path_text} count={} pinned={pinned} fileExisted={} diagnostics={}",
            self.projects.len(),
            self.file_existed,
            self.diagnostics.len()
        )
    }

    /// 打印本次读取：汇总走 stdout、逐条诊断走 stderr（与 `store.rs` 的 `report_load` 同口径，
    /// 可 `grep S1_SETTINGS_RECENT` 核对）。
    pub fn report(&self) {
        println!("{}", self.diagnostic_line());
        for diagnostic in &self.diagnostics {
            eprintln!("{RECENT_PROJECTS_DIAGNOSTIC_TAG} {diagnostic}");
        }
    }
}

/// 读最近项目文件（带路径推导，见 [`crate::paths::recent_projects_file_path`]），
/// **不创建文件、不 panic**。
pub fn load() -> LoadedRecentProjects {
    load_from(crate::paths::recent_projects_file_path())
}

/// 从指定路径读。`path = None` 表示"推导不出路径"：空列表 + 一条诊断。
pub fn load_from(path: Option<PathBuf>) -> LoadedRecentProjects {
    let Some(path) = path else {
        return LoadedRecentProjects {
            projects: RecentProjects::new(),
            path: None,
            file_existed: false,
            diagnostics: vec![
                "no_recent_projects_path (推导不出最近项目文件路径，本次会话不落盘)".to_string(),
            ],
        };
    };

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let (projects, diagnostics) = load_from_str(&text);
            LoadedRecentProjects {
                projects,
                path: Some(path),
                file_existed: true,
                diagnostics,
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // 首次启动：空列表，且**不写文件**（与设置文件"改动才写"同一条口径）。
            LoadedRecentProjects {
                projects: RecentProjects::new(),
                path: Some(path),
                file_existed: false,
                diagnostics: Vec::new(),
            }
        }
        Err(error) => LoadedRecentProjects {
            projects: RecentProjects::new(),
            path: Some(path),
            file_existed: false,
            diagnostics: vec![format!("read_failed error={error}")],
        },
    }
}

/// 解析最近项目文件文本：**逐条容错 + 规范化**。
///
/// 返回 `(列表, 诊断)`。**永不失败**：最差返回空列表 + 一条诊断。
pub fn load_from_str(text: &str) -> (RecentProjects, Vec<String>) {
    let mut diagnostics = Vec::new();

    let parsed: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("invalid_json error={error}"));
            return (RecentProjects::new(), diagnostics);
        }
    };

    let Value::Object(object) = parsed else {
        diagnostics.push("not_an_object".to_string());
        return (RecentProjects::new(), diagnostics);
    };

    // 缺键 = 空列表（无诊断，与 `Settings` 的"缺键回落默认值"一致）；
    // 键在但类型坏了才是"坏键"。
    let Some(value) = object.get(RECENT_PROJECTS_KEY) else {
        return (RecentProjects::new(), diagnostics);
    };
    let Value::Array(items) = value else {
        diagnostics.push(format!("bad_key key={RECENT_PROJECTS_KEY}"));
        return (RecentProjects::new(), diagnostics);
    };

    // 逐条解析：坏的那条丢掉 + 诊断，其余照用（照 `persistence.rs` 的逐键容错口径）。
    let mut entries = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        match serde_json::from_value::<RecentProject>(item.clone()) {
            // `path` 是去重键，缺了这条记录就没法用（点不开、也去不了重）。
            Ok(entry) if entry.path.trim().is_empty() => {
                diagnostics.push(format!("bad_entry index={index} error=missing_path"));
            }
            Ok(entry) => entries.push(entry),
            Err(error) => diagnostics.push(format!("bad_entry index={index} error={error}")),
        }
    }

    let parsed_count = entries.len();
    let entries = normalize(entries);
    if entries.len() < parsed_count {
        // 手改过的文件里可能有重复 path 或超过上限的条数；收口是真的丢数据，所以留诊断。
        diagnostics.push(format!("normalized removed={}", parsed_count - entries.len()));
    }

    (RecentProjects { entries }, diagnostics)
}

/// 原子写最近项目文件（复用 [`crate::persistence::save_json`]：临时文件 + rename，
/// 目录不存在时先建）。
///
/// 直接写 `projects.entries()`，顺序就是内存里的顺序（已经规范化）。
pub fn save(path: &Path, projects: &RecentProjects) -> io::Result<usize> {
    persistence::save_json(path, projects)
}

/// 反序列化就把不变量收口：`serde_json::from_str::<RecentProjects>` 这条路径不经过
/// [`load_from_str`]，但读出来的顺序与上限必须和文件路径一致。
fn deserialize_entries<'de, D>(deserializer: D) -> Result<Vec<RecentProject>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries = Vec::<RecentProject>::deserialize(deserializer)?;
    Ok(normalize(entries))
}

/// 规范化：补 `name` → 丢掉没有 `path` 的项 → 按 `path` 去重 → `pinned` 优先 + `lastOpenedAt`
/// 降序 → `pinned` 全留、未 pinned 截到 [`MAX_RECENT_PROJECTS`]。
///
/// 顺序逐条照真源 `limitRecentFolders`（`utils/recent-folders.ts:36-42`）；去重是照
/// `upsertRecentFolder` 的语义补的（真源只在写入时去重，本侧对**手改过的文件**也收口一次）：
/// 同 `path` 保留 `lastOpenedAt` **最大**的那条，时间并列时保留文件里靠前的那条，
/// 保留项的**相对顺序不变**。
///
/// 排序用 `sort_by`（**稳定**排序，等价于真源 `[...folders].sort` 在 ES2019+ 的稳定性）：
/// 时间并列时保持原有先后，于是"刚打开的那条已经在队首"这件事不会被排序打乱。
fn normalize(entries: Vec<RecentProject>) -> Vec<RecentProject> {
    // 去重键是 `path` 的**精确**值（不 trim、不大小写归一）。
    let mut index_of: HashMap<String, usize> = HashMap::with_capacity(entries.len());
    let mut kept: Vec<RecentProject> = Vec::with_capacity(entries.len());
    for mut entry in entries {
        if entry.path.trim().is_empty() {
            continue;
        }
        if entry.name.is_empty() {
            entry.name = folder_name(&entry.path);
        }
        match index_of.get(&entry.path).copied() {
            Some(index) => {
                if entry.last_opened_at > kept[index].last_opened_at {
                    kept[index] = entry;
                }
            }
            None => {
                index_of.insert(entry.path.clone(), kept.len());
                kept.push(entry);
            }
        }
    }

    // `pinned` 优先（`true` 在前），再按 `lastOpenedAt` 降序。
    kept.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| right.last_opened_at.cmp(&left.last_opened_at))
    });

    // 上限只截未置顶的：`pinned` 全留（真源 `unpinned.slice(0, MAX_RECENT_PROJECTS)`）。
    let mut unpinned = 0usize;
    kept.retain(|entry| {
        if entry.pinned {
            return true;
        }
        unpinned += 1;
        unpinned <= MAX_RECENT_PROJECTS
    });

    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试一个**自己的**临时目录，并在 `Drop` 里清理 —— 断言失败（panic）时也照样清
    /// （照 `.agents/skills/write-stable-tests/SKILL.md` 的"资源清理路径必须在断言失败后也跑到"）。
    ///
    /// 目录名带测试名，避免同一个测试二进制里并行跑的用例互相踩。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "lithe-recent-projects-test-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("recent-projects.json")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 文件级往返：**写进去能读回来**（本 crate 的历史坑：写得出、读不回）。
    ///
    /// 走的是真实的"原子写 + 读文件"，覆盖"键名拼错 / 序列化被 skip / 落盘那一段丢字段"
    /// 这一类只在文件路径上出现的失效；顺带覆盖"目录不存在时先建目录"（首次启动）。
    #[test]
    fn recent_projects_survive_a_file_round_trip() {
        let dir = TempDir::new("round-trip");
        let path = dir.file(); // 目录还不存在：写的时候要自己建出来

        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\alpha", 1_700_000_000_000, Some(false)));
        assert!(projects.record_open(r"D:\proj\beta", 1_700_000_001_000, Some(true)));
        assert!(projects.set_pinned(r"D:\proj\alpha", true));
        assert!(projects.set_missing(r"D:\proj\beta", true));

        let bytes = save(&path, &projects).expect("写入失败");
        assert!(bytes > 0);
        assert!(path.exists());
        assert!(
            !crate::persistence::tmp_path(&path).exists(),
            "临时文件必须被 rename 掉"
        );

        // 落盘文本里字段名必须是真源那套驼峰（写错就静默读不回来）。
        let text = std::fs::read_to_string(&path).expect("读文件失败");
        for key in [
            "\"recentProjects\"",
            "\"path\"",
            "\"name\"",
            "\"lastOpenedAt\"",
            "\"pinned\"",
            "\"missing\"",
            "\"openInNewWindow\"",
        ] {
            assert!(text.contains(key), "落盘文本缺键 {key}：{text}");
        }

        let loaded = load_from(Some(path.clone()));
        assert!(loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.projects, projects, "整个列表必须逐字段相等");
        assert_eq!(loaded.projects.entries()[0].path, r"D:\proj\alpha");
        assert!(loaded.projects.entries()[0].pinned);
        assert!(loaded.projects.entries()[1].missing);
        assert_eq!(loaded.projects.entries()[1].open_in_new_window, Some(true));

        // 覆盖写也必须成功（rename 覆盖已存在文件）。
        let mut second = projects.clone();
        second.clear();
        save(&path, &second).expect("覆盖写失败");
        let reloaded = load_from(Some(path));
        assert!(reloaded.diagnostics.is_empty(), "{:?}", reloaded.diagnostics);
        assert!(reloaded.projects.is_empty(), "清空必须落盘成空数组");
    }

    /// 上限：未置顶最多 12 条，且留下的是**最近的**那 12 条。
    #[test]
    fn cap_keeps_only_the_twelve_most_recent_unpinned_projects() {
        let mut projects = RecentProjects::new();
        for index in 0..20u64 {
            assert!(projects.record_open(&format!(r"D:\proj\p{index}"), 1_000 + index, None));
        }

        assert_eq!(projects.len(), MAX_RECENT_PROJECTS);
        assert_eq!(MAX_RECENT_PROJECTS, 12);
        // 降序：最新在前。
        assert_eq!(projects.entries()[0].path, r"D:\proj\p19");
        assert_eq!(projects.entries()[11].path, r"D:\proj\p8");
        // 最早的 8 条被截掉。
        assert!(projects.find(r"D:\proj\p7").is_none());
        assert!(projects.find(r"D:\proj\p0").is_none());
    }

    /// `pinned` 不占额度：3 条置顶 + 15 条未置顶 → 3 + 12 条，置顶一条都不丢。
    #[test]
    fn pinned_projects_do_not_consume_the_cap() {
        let mut projects = RecentProjects::new();
        for index in 0..3u64 {
            let path = format!(r"D:\proj\pinned{index}");
            assert!(projects.record_open(&path, 1_000 + index, None));
            assert!(projects.set_pinned(&path, true));
        }
        for index in 0..15u64 {
            assert!(projects.record_open(&format!(r"D:\proj\loose{index}"), 2_000 + index, None));
        }

        assert_eq!(projects.len(), 3 + MAX_RECENT_PROJECTS);
        for index in 0..3u64 {
            assert!(
                projects.find(&format!(r"D:\proj\pinned{index}")).is_some(),
                "置顶项不得被上限截掉"
            );
        }
        // 未置顶里最新的 12 条在：`loose14..loose3`（`loose2` 及更早被截）。
        assert!(projects.find(r"D:\proj\loose14").is_some());
        assert!(projects.find(r"D:\proj\loose3").is_some());
        assert!(projects.find(r"D:\proj\loose2").is_none());
    }

    /// 排序：`pinned` 优先，再 `lastOpenedAt` 降序（两块内部都按时间降序）。
    #[test]
    fn order_is_pinned_first_then_most_recent() {
        let mut projects = RecentProjects::new();
        // 故意乱序记录，并给置顶项**更旧**的时间戳 —— 置顶仍然必须排在前面。
        assert!(projects.record_open(r"D:\proj\old", 100, None));
        assert!(projects.record_open(r"D:\proj\new", 900, None));
        assert!(projects.record_open(r"D:\proj\mid", 500, None));
        assert!(projects.set_pinned(r"D:\proj\old", true));
        assert!(projects.set_pinned(r"D:\proj\mid", true));

        let paths: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![r"D:\proj\mid", r"D:\proj\old", r"D:\proj\new"],
            "置顶块在前（时间降序），未置顶块在后（时间降序）"
        );
    }

    /// 去重按 `path` **精确**比较：同一个字符串只留一条，大小写/分隔符不同的算两条。
    #[test]
    fn paths_are_deduplicated_exactly() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\Proj", 100, None));
        assert!(projects.record_open(r"d:\proj", 200, None));
        assert!(projects.record_open(r"D:/Proj", 300, None));
        assert_eq!(projects.len(), 3, "精确去重 ⇒ 大小写/分隔符不同是三条");

        assert!(projects.record_open(r"D:\Proj", 400, None));
        assert_eq!(projects.len(), 3, "同一条 path 不得产生第二项");
        let found: Vec<u64> = projects
            .entries()
            .iter()
            .filter(|entry| entry.path == r"D:\Proj")
            .map(|entry| entry.last_opened_at)
            .collect();
        assert_eq!(found, vec![400]);
    }

    /// 重复记录同一路径：只保留一条、时间更新、位置提到最前，且 `name` / `pinned` 保留、
    /// `missing` 复位（真源 `upsertRecentFolder` 的逐字段语义）。
    #[test]
    fn recording_the_same_path_updates_in_place() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\alpha", 100, Some(true)));
        assert!(projects.set_pinned(r"D:\proj\alpha", true));
        assert!(projects.set_missing(r"D:\proj\alpha", true));
        assert!(projects.record_open(r"D:\proj\beta", 200, None));

        assert!(projects.record_open(r"D:\proj\alpha", 300, Some(false)));
        assert_eq!(projects.len(), 2, "不得产生重复项");
        assert_eq!(projects.entries()[0].path, r"D:\proj\alpha", "刚打开的提到最前");
        let alpha = projects.find(r"D:\proj\alpha").expect("α 必须在列表里");
        assert_eq!(alpha.last_opened_at, 300, "时间必须更新");
        assert!(alpha.pinned, "重开不改变置顶");
        assert!(!alpha.missing, "重开一次就是又能用了");
        assert_eq!(alpha.open_in_new_window, Some(false), "打开方式按本次更新");
        assert_eq!(alpha.name, folder_name(r"D:\proj\alpha"), "名字仍是路径末段");

        // `open_in_new_window = None` = 不改上次的打开方式。
        assert!(projects.record_open(r"D:\proj\alpha", 400, None));
        assert_eq!(
            projects.find(r"D:\proj\alpha").unwrap().open_in_new_window,
            Some(false)
        );
    }

    /// 已有项的 `name` 在重开时**不被**覆盖（真源 `existing?.name ?? getFolderName(..)`）。
    #[test]
    fn recording_keeps_the_existing_display_name() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\alpha", 100, None));
        projects.entries[0].name = "自定义别名".to_string();
        assert!(projects.record_open(r"D:\proj\alpha", 200, None));
        assert_eq!(projects.find(r"D:\proj\alpha").unwrap().name, "自定义别名");
    }

    /// 空路径 / 纯空白路径不记录（否则会产生一条永远点不开、去重键为空的记录）。
    #[test]
    fn empty_paths_are_not_recorded() {
        let mut projects = RecentProjects::new();
        assert!(!projects.record_open("", 100, None));
        assert!(!projects.record_open("   ", 100, None));
        assert!(projects.is_empty());
    }

    /// 坏 JSON / 不是对象 / 外层键类型坏了：空列表 + 诊断，**绝不 panic**。
    #[test]
    fn broken_json_yields_an_empty_list_with_a_diagnostic() {
        let (projects, diagnostics) = load_from_str("{ not json ");
        assert!(projects.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].starts_with("invalid_json"), "{diagnostics:?}");

        let (projects, diagnostics) = load_from_str("[1, 2, 3]");
        assert!(projects.is_empty());
        assert_eq!(diagnostics, vec!["not_an_object".to_string()]);

        let (projects, diagnostics) = load_from_str(r#"{"recentProjects": "nope"}"#);
        assert!(projects.is_empty());
        assert_eq!(
            diagnostics,
            vec!["bad_key key=recentProjects".to_string()]
        );

        // 缺外层键 = 空列表，且**不**打诊断（"缺键回落默认值"）。
        let (projects, diagnostics) = load_from_str(r#"{"other": 1}"#);
        assert!(projects.is_empty());
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    /// 坏条目只丢自己：其余条目照用；缺 `name` 的条目用路径末段补齐而不是丢掉。
    #[test]
    fn a_broken_entry_falls_back_alone() {
        let (projects, diagnostics) = load_from_str(
            r#"{
                "recentProjects": [
                    {"path": "D:\\proj\\good", "name": "good", "lastOpenedAt": 300},
                    {"name": "no-path", "lastOpenedAt": 200},
                    {"path": 7, "name": "wrong-type"},
                    {"path": "D:\\proj\\noname", "lastOpenedAt": 100}
                ]
            }"#,
        );

        let paths: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, vec![r"D:\proj\good", r"D:\proj\noname"]);
        assert_eq!(
            projects.find(r"D:\proj\noname").unwrap().name,
            "noname",
            "缺 name 用路径末段补"
        );
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert!(diagnostics[0].contains("bad_entry index=1"), "{diagnostics:?}");
        assert!(diagnostics[1].contains("bad_entry index=2"), "{diagnostics:?}");
    }

    /// 手改过的文件里的重复 `path`：读进来就收口，保留 `lastOpenedAt` 最大的那条，并留诊断。
    #[test]
    fn duplicates_in_a_hand_edited_file_collapse_to_the_newest() {
        let (projects, diagnostics) = load_from_str(
            r#"{
                "recentProjects": [
                    {"path": "D:\\proj\\a", "name": "a", "lastOpenedAt": 100},
                    {"path": "D:\\proj\\b", "name": "b", "lastOpenedAt": 200},
                    {"path": "D:\\proj\\a", "name": "a2", "lastOpenedAt": 300}
                ]
            }"#,
        );

        assert_eq!(projects.len(), 2);
        let a = projects.find(r"D:\proj\a").unwrap();
        assert_eq!(a.last_opened_at, 300, "保留的是最新的那条");
        assert_eq!(a.name, "a2");
        assert_eq!(diagnostics, vec!["normalized removed=1".to_string()]);
    }

    /// 手改过的文件超过上限：读进来也截到 12（未置顶）+ 全部置顶。
    #[test]
    fn an_over_limit_file_is_trimmed_on_load() {
        let mut items = Vec::new();
        for index in 0..15u64 {
            items.push(format!(
                r#"{{"path": "D:\\proj\\p{index}", "name": "p{index}", "lastOpenedAt": {}}}"#,
                1_000 + index
            ));
        }
        items.push(
            r#"{"path": "D:\\proj\\pinned", "name": "pinned", "lastOpenedAt": 1, "pinned": true}"#
                .to_string(),
        );
        let text = format!(r#"{{"recentProjects": [{}]}}"#, items.join(","));

        let (projects, diagnostics) = load_from_str(&text);
        assert_eq!(projects.len(), MAX_RECENT_PROJECTS + 1);
        assert!(projects.find(r"D:\proj\pinned").unwrap().pinned);
        assert!(projects.find(r"D:\proj\p14").is_some());
        assert!(projects.find(r"D:\proj\p2").is_none());
        assert_eq!(diagnostics, vec!["normalized removed=3".to_string()]);
    }

    /// 空文件与不存在的文件：都是"可用但为空"，**不 panic、不创建文件**。
    #[test]
    fn empty_and_missing_files_behave_like_a_first_launch() {
        // 空文件（0 字节）不是合法 JSON：空列表 + 一条诊断。
        let (projects, diagnostics) = load_from_str("");
        assert!(projects.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].starts_with("invalid_json"), "{diagnostics:?}");

        // 文件不存在：空列表、无诊断、**不创建文件**。
        let dir = TempDir::new("missing-file");
        let path = dir.file();
        assert!(!dir.0.exists());
        let loaded = load_from(Some(path.clone()));
        assert!(loaded.projects.is_empty());
        assert!(!loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert!(!path.exists(), "读取不得创建文件");

        // 读到了但内容为空（0 字节）的文件：`file_existed = true` + 诊断，不 panic。
        std::fs::create_dir_all(&dir.0).expect("建目录失败");
        std::fs::write(&path, b"").expect("写空文件失败");
        let loaded = load_from(Some(path.clone()));
        assert!(loaded.projects.is_empty());
        assert!(loaded.file_existed);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].starts_with("invalid_json"), "{:?}", loaded.diagnostics);

        // 推导不出路径：空列表 + 一条诊断（不 panic）。
        let loaded = load_from(None);
        assert!(loaded.projects.is_empty());
        assert!(loaded.path.is_none());
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].starts_with("no_recent_projects_path"));
    }

    /// 维护动作的返回值和边界：切换置顶 / 标记失效 / 移除 / 清空，以及"没有这条 path"。
    #[test]
    fn maintenance_actions_report_whether_they_matched() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\a", 100, None));
        assert!(projects.record_open(r"D:\proj\b", 200, None));

        assert!(projects.toggle_pinned(r"D:\proj\a"));
        assert!(projects.find(r"D:\proj\a").unwrap().pinned);
        assert!(projects.entries()[0].path.ends_with('a'), "置顶后回到列表最前");
        assert!(projects.toggle_pinned(r"D:\proj\a"));
        assert!(!projects.find(r"D:\proj\a").unwrap().pinned);

        assert!(projects.set_missing(r"D:\proj\b", true));
        assert!(projects.find(r"D:\proj\b").unwrap().missing);
        assert!(projects.set_missing(r"D:\proj\b", false));
        assert!(!projects.find(r"D:\proj\b").unwrap().missing);

        // 没有这条 path：返回 false（调用方据此打诊断，而不是静默无操作）。
        assert!(!projects.toggle_pinned(r"D:\proj\ghost"));
        assert!(!projects.set_pinned(r"D:\proj\ghost", true));
        assert!(!projects.set_missing(r"D:\proj\ghost", true));

        assert!(projects.remove(r"D:\proj\b"));
        assert!(!projects.remove(r"D:\proj\b"), "第二次移除没有命中");
        assert_eq!(projects.len(), 1);

        projects.clear();
        assert!(projects.is_empty());
        assert!(projects.find(r"D:\proj\a").is_none());
    }

    /// 直接走 `serde_json::from_str::<RecentProjects>` 这条路径也要满足不变量
    /// （顺序 + 上限 + 去重），别只在 `load_from_str` 里收口。
    #[test]
    fn direct_deserialization_is_normalized_too() {
        let text = r#"{"recentProjects": [
            {"path": "D:\\proj\\old", "name": "old", "lastOpenedAt": 1},
            {"path": "D:\\proj\\new", "name": "new", "lastOpenedAt": 9},
            {"path": "D:\\proj\\new", "name": "new2", "lastOpenedAt": 5}
        ]}"#;
        let projects: RecentProjects = serde_json::from_str(text).expect("解析失败");
        let paths: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, vec![r"D:\proj\new", r"D:\proj\old"]);
        assert_eq!(projects.find(r"D:\proj\new").unwrap().last_opened_at, 9);
    }

    /// 显示名 = 路径末段（真源的取分隔符规则：含 `\` 按 `\` 切，否则按 `/` 切）。
    #[test]
    fn folder_name_is_the_last_segment() {
        assert_eq!(folder_name(r"D:\developmentProjects\Lithe-IDEA"), "Lithe-IDEA");
        assert_eq!(folder_name("/home/dev/lithe/"), "lithe");
        assert_eq!(folder_name("lithe"), "lithe");
        // `C:\` 的有效段只有 `C:`（真源 `filter(Boolean)` 之后取末段）。
        assert_eq!(folder_name(r"C:\"), "C:");
        // 一段都不剩 ⇒ 回落到整条路径（真源 `|| folderPath`）。
        assert_eq!(folder_name(""), "");
        assert_eq!(folder_name("///"), "///");
    }

    /// 诊断汇总行：界面上有几条、几条置顶，都能在这一行里核对。
    #[test]
    fn diagnostic_line_reports_count_and_pinned() {
        let dir = TempDir::new("diagnostic-line");
        let path = dir.file();
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\a", 100, None));
        assert!(projects.record_open(r"D:\proj\b", 200, None));
        assert!(projects.set_pinned(r"D:\proj\a", true));
        save(&path, &projects).expect("写入失败");

        let loaded = load_from(Some(path.clone()));
        let line = loaded.diagnostic_line();
        assert!(line.starts_with(RECENT_PROJECTS_DIAGNOSTIC_TAG), "{line}");
        assert!(line.contains("count=2"), "{line}");
        assert!(line.contains("pinned=1"), "{line}");
        assert!(line.contains("fileExisted=true"), "{line}");
        assert!(line.contains("diagnostics=0"), "{line}");
        assert!(
            line.contains(&path.display().to_string()),
            "诊断行必须带路径：{line}"
        );
    }

    /// 现取时刻的落点只有一个：它必须是**合理的 Unix 毫秒**（不比对真实时间，只看量级）。
    #[test]
    fn now_unix_ms_is_milliseconds_since_epoch() {
        let now = now_unix_ms();
        // 2020-01-01 之后的毫秒数；上界给 2100 年，避免把"秒"误当"毫秒"。
        assert!(
            (1_577_836_800_000..4_102_444_800_000).contains(&now),
            "now_unix_ms 必须是毫秒：{now}"
        );
    }

    /// 「无参数启动」选根：**空列表** → 没有根可选，也没有失效条目。
    ///
    /// 守的是"空列表不许 panic、也不许挑出一个不存在的路径"；调用方据此走兜底
    /// （见 `app/src/main.rs` 的 `resolve_launch_root`），所以这里必须如实返回 `None`。
    #[test]
    fn select_launch_root_on_an_empty_list_picks_nothing() {
        let projects = RecentProjects::new();
        let pick = projects.select_launch_root(|_| panic!("空列表不得探测任何路径"));

        assert_eq!(pick.root, None);
        assert!(pick.missing.is_empty(), "{pick:?}");
    }

    /// 「无参数启动」选根：**所有条目都失效** → 没有根，且每一条都被标成待标记的 missing。
    #[test]
    fn select_launch_root_reports_every_missing_entry() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\gone-newer", 300, None));
        assert!(projects.record_open(r"D:\proj\gone-older", 100, None));

        let pick = projects.select_launch_root(|_| false);

        assert_eq!(pick.root, None, "一条都不存在 ⇒ 没有根可选（调用方走兜底）");
        assert_eq!(
            pick.missing,
            vec![
                r"D:\proj\gone-newer".to_string(),
                r"D:\proj\gone-older".to_string()
            ],
            "失效条目按列表顺序（时间降序）交回，调用方逐条 set_missing + 落盘"
        );
    }

    /// 「无参数启动」选根：**最近且存在**的那条被选中；比它更新的失效条目被跳过并记进
    /// `missing`，比它更旧的条目**仍然会被探测**（失效的要标记，存在的只是不选）。
    #[test]
    fn select_launch_root_picks_the_most_recent_existing_project() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\gone", 300, None));
        assert!(projects.record_open(r"D:\proj\here", 200, None));
        assert!(projects.record_open(r"D:\proj\older-gone", 100, None));

        let pick = projects.select_launch_root(|path| path == r"D:\proj\here");

        assert_eq!(pick.root.as_deref(), Some(r"D:\proj\here"));
        assert_eq!(
            pick.missing,
            vec![r"D:\proj\gone".to_string(), r"D:\proj\older-gone".to_string()],
            "选中之后排在后面的条目照样被探测（要标记的失效条目与选中项无关）"
        );
    }

    /// 「无参数启动」选根：置顶项**优先**（列表顺序就是界面顺序），即使它比别的条目旧。
    #[test]
    fn select_launch_root_respects_pinned_order() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\newer", 900, None));
        assert!(projects.record_open(r"D:\proj\pinned-old", 100, None));
        assert!(projects.set_pinned(r"D:\proj\pinned-old", true));

        let pick = projects.select_launch_root(|_| true);

        assert_eq!(
            pick.root.as_deref(),
            Some(r"D:\proj\pinned-old"),
            "置顶项排在界面前面 ⇒ 双击打开的也是它（用户的置顶意图优先于最近）"
        );
        assert!(pick.missing.is_empty(), "{pick:?}");
    }

    /// 「无参数启动」选根：**只要有一条存在就不会没有根** —— 这条守着"绝不静默什么都不发生"
    /// 里能被单测覆盖的那一半（剩下那一半是调用方的兜底，见 `app/src/main.rs`）。
    #[test]
    fn select_launch_root_finds_the_only_surviving_project_at_the_end() {
        let mut projects = RecentProjects::new();
        for index in 0..3u64 {
            assert!(projects.record_open(&format!(r"D:\proj\gone{index}"), 100 + index, None));
        }
        assert!(projects.record_open(r"D:\proj\survivor", 1, None));

        let pick = projects.select_launch_root(|path| path == r"D:\proj\survivor");

        assert_eq!(pick.root.as_deref(), Some(r"D:\proj\survivor"));
        assert_eq!(pick.missing.len(), 3, "{pick:?}");
    }
}
