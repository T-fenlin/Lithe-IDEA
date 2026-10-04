//! 调试工具窗（U19）：**断点模型** + **调用栈 / 变量树 / 监视 / 求值 / 单步控制**五块 + **诚实状态**。
//!
//! ## 为什么这是一个"诚实优先"的界面
//!
//! Core 的 `debug.*` 18 条命令**全都在**（`rust/lithe-core/src/protocol/command.rs:115-149`），
//! `debug/engine.rs` 4046 行；但 Core 自己在 `debug/engine.rs:1` 就写明
//! "byte transport and process lifecycle remain platform owned"，而宿主侧那两件事
//! **一件都没做**（`docs/tasks/gpui-core-first.md:135` 的 **H10** 调试适配器 transport、
//! **H11** Java Debug Server 启动与端口协商）。没有 transport ⇒ `debug.receive`
//! （"从平台侧 transport 收到字节"的入口）永远收不到字节 ⇒ **没有活会话**。
//!
//! 所以本模块的界面口径是：**把"做不到"画成可核对的状态，而不是画成转圈或死按钮**。
//! 具体见下面三处设计。
//!
//! ## 一、会话与 transport 是**两个**事实（不是一件事）
//!
//! [`DebugTransportFacts`] 与 [`DebugSessionFacts`] 分开建模，[`start_blocker`] 是它们到
//! "能不能启动"的**纯函数**（三个分支：只有 transport 缺 / 只有会话缺 / 两个都缺）。
//! 这个函数的存在就是为了让"把 transport 缺失也算成可启动"这种退化**在单测里变红**。
//!
//! 当前运行期两个事实恒为 `Absent`（见 [`DebugPane::transport`] / [`DebugPane::session`] 的文档），
//! 因为宿主**没有任何东西能造出会话**：`debug.createSession` 需要一个 `adapterId`，
//! 而 adapterId 正是 H11（`vscode.java.startDebugSession` 拿到适配器/端口）才会给出的东西 ——
//! 编一个 `"java"` 会建出"适配器并不存在"的会话，那正是**假装**。所以本切片**不调**
//! `debug.createSession` / `debug.launch` / `debug.javaTestLaunch`（理由逐条在
//! [`DebugPane::start_session`] 上）。
//!
//! ## 二、真正接上的 Core 命令（与真的没接上的，各自说清楚）
//!
//! | 命令 | 本切片 | 依据 |
//! | --- | --- | --- |
//! | `debug.steppingFilters` | ✅ **真的调** | `engine.rs:163-172` 只做归一化，**不用 `with_session`** ⇒ 无会话也能调；驱动步进过滤器那一块 |
//! | `debug.relocateBreakpoints` | ✅ **真的调** | `breakpoint_relocation.rs:42-105` 是**纯函数**（源文本 + 一次 UTF-16 编辑 + 断点集 → 新断点集），也不碰会话；驱动"编辑导致行号位移" |
//! | `debug.setBreakpoints` | ⚠️ **只构造、不派发** | `engine.rs:197` 走 `with_session` ⇒ 无会话必然 `session_not_found`。请求构造是纯函数并单测，派发走 [`DebugDispatchGate`]，闸门当前恒关 |
//! | 其余 15 条 | ❌ 不调 | 都要会话（`debug.receive` 还要 transport）；详见模块文档末的"没做"清单 |
//!
//! ## 三、断点模型归**宿主**
//!
//! 这不是妥协而是正确的所有权：真机也是宿主先存（编辑器 gutter 要立刻画出标记，
//! 与"有没有会话"无关），再由会话把整份文件级断点集发给适配器 —— Core 的
//! `SetBreakpointsRequest` 就是"替换某个文件的**全部**断点"（`types.rs:190-198`）。
//! 所以 [`DebugBreakpointStore`] 按文件分组、按行号排序、可启用/禁用，**不依赖 transport**。
//!
//! ## 本切片**没做**的（逐条给出为什么 + 缺什么）
//!
//! 1. **真实 DAP transport（H10）**：socket / 子进程 / 适配器进程生命周期，属平台端口任务；
//!    缺的是"平台侧一串 `AsyncRead + AsyncWrite`"与它的进程生命周期，不在本任务范围。
//! 2. **Java Debug Server 启动与端口协商（H11）**：`vscode.java.startDebugSession` 不在
//!    Core 命令面里（`run/src/lib.rs:34` 把它列为下一步入口）；缺的是 JDT LS 侧的会话启动。
//! 3. **`debug.createSession` / `debug.launch` / `debug.javaTestLaunch` 的真实调用**：
//!    见上面"一"的解释（不调，因为 adapterId 是 H11 的事实）。
//! 4. **数据断点**（`dataBreakpointInfo` / `setDataBreakpoints`）、**`debug.setVariable`**、
//!    **`debug.runInTerminalResponse`**（集成终端）、**`debug.disconnect` / `destroySession`**：
//!    全部需要活会话，靠 1/2 解锁；本切片不画它们的按钮（不画 = 不做假入口）。
//! 5. **异常断点 / 函数断点**：请求构造**已做**（[`set_exception_breakpoints_payload`] /
//!    [`set_function_breakpoints_payload`]），但适配器给的 `exceptionBreakpointFilters`
//!    只在 `capabilities` 事件里到（`types.rs:527-537`）⇒ **没有 transport 就拿不到过滤器清单**，
//!    所以界面**不画**这两个列表（画出来必然是空的假清单）。
//! 6. **断点持久化**：真机把断点写进 IDE 配置；宿主没有这条存储通路，所以本切片的断点是
//!    **进程内**的（关掉窗就没了）。缺的是"断点存哪"这个产品决定与一个持久化 store。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Task,
    Window, div, rems,
};

use lithe_gpui_editor::GutterBreakpoint;
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::{core_json, tr, tr_args};
use serde_json::{Value, json};

/// 诊断前缀（`eprintln!`，stderr）。与 `S1_*` 一族同口径：stdout 重定向到文件是块缓冲的，
/// 进程还在跑时可能一行都没落盘，所以宿主诊断一律走 stderr。
pub const DEBUG_DIAGNOSTIC_TAG: &str = "S1_DEBUG";

/// `debug.steppingFilters`（`protocol/command.rs:121-122`）。
const STEPPING_FILTERS_COMMAND: &str = "debug.steppingFilters";
/// `debug.relocateBreakpoints`（`protocol/command.rs:123-124`）。
const RELOCATE_BREAKPOINTS_COMMAND: &str = "debug.relocateBreakpoints";
/// `debug.setBreakpoints`（`protocol/command.rs:125-126`）。
const SET_BREAKPOINTS_COMMAND: &str = "debug.setBreakpoints";

/// 步进过滤器默认取哪个适配器的（`DebugSteppingFilters::defaults_for_adapter`，
/// `types.rs:92-98`：只有 `"java"` 那份是带过滤的默认值，其余适配器是 `unfiltered`）。
///
/// ⚠️ 这是**本侧唯一需要 adapterId 的地方**，而它与 H11 无关：这只是一个"取哪一份默认值"的
/// 选择器，Core 不校验适配器是否存在，界面上也如实标注"取自 Java 适配器默认档"。
const STEPPING_FILTERS_ADAPTER: &str = "java";

// ---------------------------------------------------------------------------
// 断点模型（纯逻辑，宿主所有；不依赖 transport）
// ---------------------------------------------------------------------------

/// 一个宿主断点。字段与 Core 的 `SourceBreakpoint`（`types.rs:169-184`）逐条对应，
/// 因为它的唯一去向就是把整个文件的断点集写成 `debug.setBreakpoints` 的载荷。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugBreakpoint {
    /// 一基行号（Core 口径，`types.rs:171`）。
    pub line: i64,
    /// 可选一基列号；宿主 gutter 只按行，所以恒为 `None`（真机的列断点也只在 Java 上出现）。
    pub column: Option<i64>,
    pub enabled: bool,
    /// 适配器校验结果。没有会话 ⇒ 恒为 `false`（**不是**"未校验"的占位，就是"还没被校验过"）。
    pub verified: bool,
    /// 适配器给的校验消息（`DebugBreakpoint.message`）。
    pub message: Option<String>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

impl DebugBreakpoint {
    /// 行断点（宿主唯一会造出来的形状）。
    fn line(line: i64) -> Self {
        Self {
            line,
            column: None,
            enabled: true,
            verified: false,
            message: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }
    }

    /// 喂给 Core 的 JSON 形状（`SourceBreakpoint` 的字段名，`types.rs:169-184`）。
    fn to_payload(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("line".to_string(), json!(self.line));
        if let Some(column) = self.column {
            map.insert("column".to_string(), json!(column));
        }
        map.insert("enabled".to_string(), json!(self.enabled));
        insert_optional(&mut map, "condition", self.condition.as_deref());
        insert_optional(&mut map, "hitCondition", self.hit_condition.as_deref());
        insert_optional(&mut map, "logMessage", self.log_message.as_deref());
        Value::Object(map)
    }

    /// 从 Core 反回的 `SourceBreakpoint` 形状读回来（`debug.relocateBreakpoints` 的响应）。
    ///
    /// 缺字段一律按 Core 的 `#[serde(default)]` 口径回落（`types.rs:173-183`）：
    /// `enabled` 缺省是 **true**、其余可选字段缺省是 `None`。
    fn from_payload(value: &Value) -> Option<Self> {
        Some(Self {
            line: value.get("line")?.as_i64()?,
            column: value.get("column").and_then(Value::as_i64),
            enabled: value
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            verified: false,
            message: None,
            condition: string_field(value, "condition"),
            hit_condition: string_field(value, "hitCondition"),
            log_message: string_field(value, "logMessage"),
        })
    }
}

/// 宿主断点表：**按文件分组**，文件内按 (行号, 列号) 排序。
///
/// 键是**工作区相对路径**并用 `/` 分隔（`application-boundary.md` 的路径口径；
/// Core 的 `sourcePath` 收绝对路径或 URI，两边都行 —— 本侧统一用工作区相对路径，
/// 与 `search_index` 的 `index_path` 同口径）。
#[derive(Default, Clone, Debug)]
pub struct DebugBreakpointStore {
    by_file: BTreeMap<String, Vec<DebugBreakpoint>>,
    /// 全部断点静音（真源工具栏的 `muteBreakpoints`，`DebugToolbarPresentation.swift:30`）。
    muted: bool,
}

/// 一次断点变更的落点。调用方拿它决定"要不要同步给 Core / 要不要重画 gutter"。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugBreakpointChange {
    /// 没有任何变化（例如切换一个不存在的行）。
    Unchanged,
    /// 这个文件的断点集变了（值 = 文件键）。
    File(String),
}

impl DebugBreakpointStore {
    /// 该文件当前的断点（已排序）。没有就是空切片。
    pub fn file(&self, path: &str) -> &[DebugBreakpoint] {
        self.by_file.get(path).map(Vec::as_slice).unwrap_or(&[])
    }

    /// 全部文件键（`BTreeMap` ⇒ 有序，诊断与遍历都确定）。
    pub fn files(&self) -> impl Iterator<Item = &String> {
        self.by_file.keys()
    }

    /// 全部断点总数（工具栏的 `muteBreakpoints` 禁用判据用它，真源 `:427`）。
    pub fn total(&self) -> usize {
        self.by_file.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    /// 切换静音。没有断点时也允许翻（真源按钮靠 `total()` 禁用，所以这条只被 UI 闸门保护）。
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// 点击行号槽：有就删、没有就加（真源 `toggleBreakpoint`，`AppModel+Debugging.swift:308`）。
    pub fn toggle(&mut self, path: &str, line: i64) -> DebugBreakpointChange {
        if line < 1 {
            return DebugBreakpointChange::Unchanged;
        }
        let entry = self.by_file.entry(path.to_string()).or_default();
        match entry.iter().position(|breakpoint| breakpoint.line == line) {
            Some(index) => {
                entry.remove(index);
                if entry.is_empty() {
                    self.by_file.remove(path);
                }
            }
            None => {
                entry.push(DebugBreakpoint::line(line));
                entry.sort_by_key(|breakpoint| (breakpoint.line, breakpoint.column.unwrap_or(0)));
            }
        }
        DebugBreakpointChange::File(path.to_string())
    }

    /// 启用 / 禁用某一行（真源 gutter 右键的 `Disable Breakpoint` / `Enable Breakpoint`）。
    pub fn set_enabled(&mut self, path: &str, line: i64, enabled: bool) -> DebugBreakpointChange {
        let Some(entry) = self.by_file.get_mut(path) else {
            return DebugBreakpointChange::Unchanged;
        };
        let Some(breakpoint) = entry.iter_mut().find(|item| item.line == line) else {
            return DebugBreakpointChange::Unchanged;
        };
        if breakpoint.enabled == enabled {
            return DebugBreakpointChange::Unchanged;
        }
        breakpoint.enabled = enabled;
        DebugBreakpointChange::File(path.to_string())
    }

    /// 移除某一行（真源 gutter 右键的 `Remove Breakpoint`）。
    pub fn remove(&mut self, path: &str, line: i64) -> DebugBreakpointChange {
        let Some(entry) = self.by_file.get_mut(path) else {
            return DebugBreakpointChange::Unchanged;
        };
        let before = entry.len();
        entry.retain(|breakpoint| breakpoint.line != line);
        if entry.len() == before {
            return DebugBreakpointChange::Unchanged;
        }
        if entry.is_empty() {
            self.by_file.remove(path);
        }
        DebugBreakpointChange::File(path.to_string())
    }

    /// 整份替换这个文件的断点集 —— **`debug.relocateBreakpoints` 的采纳口**。
    ///
    /// 空集合 = 这个文件不再有断点（真源同样会把 `setBreakpoints` 发成空数组，
    /// 这是 DAP 的"清空该源"语义）。
    pub fn replace_file(
        &mut self,
        path: &str,
        breakpoints: Vec<DebugBreakpoint>,
    ) -> DebugBreakpointChange {
        let mut sorted = breakpoints;
        sorted.sort_by_key(|breakpoint| (breakpoint.line, breakpoint.column.unwrap_or(0)));
        sorted.dedup_by_key(|breakpoint| (breakpoint.line, breakpoint.column));
        if sorted.is_empty() {
            self.by_file.remove(path);
        } else {
            self.by_file.insert(path.to_string(), sorted);
        }
        DebugBreakpointChange::File(path.to_string())
    }

    /// 清空全部（真源「清空断点」`debugger.clearBreakpoints`）。
    pub fn clear(&mut self) -> DebugBreakpointChange {
        if self.by_file.is_empty() {
            return DebugBreakpointChange::Unchanged;
        }
        let paths: Vec<String> = self.by_file.keys().cloned().collect();
        self.by_file.clear();
        // 多文件时只报第一个是**不够**的（同步要给每个文件发一条清空），
        // 所以调用方拿 [`Self::files`] 自己遍历；这里返回 `Unchanged` 会掩盖"清了"这件事，
        // 于是约定：返回 `File("")` 的空键不存在 ⇒ 调用方一律走 [`Self::files`] 重放。
        // 为避免这个含糊，clear 的契约是"返回值只用来说明**是否发生了清空**"。
        let _ = paths;
        DebugBreakpointChange::File(String::new())
    }

    /// 按文件生成 `debug.setBreakpoints` 的载荷（**按文件替换语义**，`types.rs:190-198`）。
    ///
    /// 纯函数：没有会话、没有 transport 也照样能算出"该发什么"，所以它能被单测钉住。
    pub fn set_breakpoints_payload(&self, session_id: &str, path: &str) -> Value {
        json!({
            "sessionId": session_id,
            "sourcePath": path,
            "breakpoints": self
                .file(path)
                .iter()
                .map(DebugBreakpoint::to_payload)
                .collect::<Vec<_>>(),
        })
    }
}

// ---------------------------------------------------------------------------
// Core 请求构造（纯函数；全部可单测）
// ---------------------------------------------------------------------------

/// 一次编辑器编辑（UTF-16 偏移，`DebugSourceEdit` 的字段口径，`breakpoint_relocation.rs:11-21`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugSourceEdit {
    pub start_utf16_offset: usize,
    pub end_utf16_offset: usize,
    pub replacement: String,
}

/// 从一个"编辑前 / 编辑后"的文本对推出**一次** `DebugSourceEdit`（纯函数）。
///
/// 为什么这样推：编辑区只报"正文变了"（`editor_view.rs` 的 `on_input_change` 收到
/// `InputEvent::Change`），而上游没有把 `InputEdit` 交出来。取最长公共前缀 / 后缀就得到
/// 覆盖这次变化的**最小单区间**，正是 Core 的 `RelocateBreakpointsRequest` 要的形状
/// （`breakpoint_relocation.rs:23-32`：一次编辑 + 之前的完整源文本）。
///
/// ⚠️ **已知近似**：多光标 / 多区间同时在一次 Change 里落地时，这个单区间是它们的
/// **并集**，于是并集**内部**的断点会被 Core 折到区间起点（`relocate_anchor`，
/// `breakpoint_relocation.rs:156-169` 的 `edit_start + replacement_length` 分支）。
/// 区间外（绝大多数行）的断点位移仍然逐字正确。
///
/// 返回 `None` = 两段文本相同（没有 Edit 可发）。偏移按 **UTF-16** 计（Core 的口径）。
pub fn derive_source_edit(before: &str, after: &str) -> Option<DebugSourceEdit> {
    if before == after {
        return None;
    }
    let before_units: Vec<u16> = before.encode_utf16().collect();
    let after_units: Vec<u16> = after.encode_utf16().collect();

    // 公共前缀（按 UTF-16 code unit）。
    let mut prefix = 0usize;
    while prefix < before_units.len()
        && prefix < after_units.len()
        && before_units[prefix] == after_units[prefix]
    {
        prefix += 1;
    }
    // 公共后缀：不能越过前缀。
    let mut suffix = 0usize;
    while suffix < before_units.len() - prefix
        && suffix < after_units.len() - prefix
        && before_units[before_units.len() - 1 - suffix]
            == after_units[after_units.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let replacement_units = &after_units[prefix..after_units.len() - suffix];
    Some(DebugSourceEdit {
        start_utf16_offset: prefix,
        end_utf16_offset: before_units.len() - suffix,
        replacement: String::from_utf16_lossy(replacement_units),
    })
}

/// `debug.relocateBreakpoints` 的载荷（`RelocateBreakpointsRequest`，`breakpoint_relocation.rs:23-32`）。
pub fn relocate_breakpoints_payload(
    source_before: &str,
    edit: &DebugSourceEdit,
    breakpoints: &[DebugBreakpoint],
) -> Value {
    json!({
        "source": source_before,
        "edit": {
            "startUtf16Offset": edit.start_utf16_offset,
            "endUtf16Offset": edit.end_utf16_offset,
            "replacement": edit.replacement,
        },
        "breakpoints": breakpoints
            .iter()
            .map(DebugBreakpoint::to_payload)
            .collect::<Vec<_>>(),
    })
}

/// 读 `RelocateBreakpointsResult`（`breakpoint_relocation.rs:34-39`：`{ "breakpoints": [...] }`）。
///
/// 返回 `None` = 形状不认识（Core 契约变了）；调用方**保留旧断点**而不是清空 ——
/// 一次读不懂的响应不该让用户的断点消失。
pub fn parse_relocate_result(value: &Value) -> Option<Vec<DebugBreakpoint>> {
    let items = value.get("breakpoints")?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(DebugBreakpoint::from_payload)
            .collect(),
    )
}

/// `debug.setExceptionBreakpoints` 的载荷（`SetExceptionBreakpointsRequest`，`types.rs:211-218`）。
pub fn set_exception_breakpoints_payload(session_id: &str, filters: &[String]) -> Value {
    json!({
        "sessionId": session_id,
        "breakpoints": filters
            .iter()
            .map(|filter| json!({ "filter": filter, "enabled": true }))
            .collect::<Vec<_>>(),
    })
}

/// `debug.setFunctionBreakpoints` 的载荷（`SetFunctionBreakpointsRequest`，`types.rs:233-240`）。
pub fn set_function_breakpoints_payload(session_id: &str, names: &[String]) -> Value {
    json!({
        "sessionId": session_id,
        "breakpoints": names
            .iter()
            .map(|name| json!({ "name": name, "enabled": true }))
            .collect::<Vec<_>>(),
    })
}

/// `debug.steppingFilters` 的载荷。
///
/// `filters` **故意不传**（`types.rs:156-157`：`#[serde(default)] filters: Option<..>`，
/// 缺省即"要适配器默认档"）—— 传一份宿主自造的默认值就是把 Core 的默认档复制一份，
/// 那正是"两个真源"。
pub fn stepping_filters_payload(adapter_id: &str) -> Value {
    json!({ "adapterId": adapter_id })
}

// ---------------------------------------------------------------------------
// 步进过滤器（真源 `JavaSteppingFiltersView`，`GenericDebugView.swift:2109-2213`）
// ---------------------------------------------------------------------------

/// `DebugSteppingFilters`（`types.rs:70-89`）的界面投影。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SteppingFiltersView {
    pub class_name_filters: Vec<String>,
    pub skip_synthetics: bool,
    pub skip_static_initializers: bool,
    pub skip_constructors: bool,
    pub hide_filtered_stack_frames: bool,
}

impl SteppingFiltersView {
    /// 从 Core 的响应读回来（`types.rs:70-89` 的 camelCase 字段名）。
    pub fn from_payload(value: &Value) -> Option<Self> {
        Some(Self {
            class_name_filters: value
                .get("classNameFilters")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            skip_synthetics: value
                .get("skipSynthetics")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            skip_static_initializers: value
                .get("skipStaticInitializers")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            skip_constructors: value
                .get("skipConstructors")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            hide_filtered_stack_frames: value
                .get("hideFilteredStackFrames")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

/// 五条滤镜开关在界面上的**稳定顺序**（真源 `JavaSteppingFiltersView` 的五行，
/// `GenericDebugView.swift:2137-2213`）。`label_key` 走 catalog 既有 `debugger.*` 键。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SteppingFilterToggle {
    SkipSynthetics,
    SkipConstructors,
    SkipStaticInitializers,
    HideFilteredStackFrames,
}

impl SteppingFilterToggle {
    pub fn value(self, filters: &SteppingFiltersView) -> bool {
        match self {
            Self::SkipSynthetics => filters.skip_synthetics,
            Self::SkipConstructors => filters.skip_constructors,
            Self::SkipStaticInitializers => filters.skip_static_initializers,
            Self::HideFilteredStackFrames => filters.hide_filtered_stack_frames,
        }
    }

    fn set(self, filters: &mut SteppingFiltersView, value: bool) {
        match self {
            Self::SkipSynthetics => filters.skip_synthetics = value,
            Self::SkipConstructors => filters.skip_constructors = value,
            Self::SkipStaticInitializers => filters.skip_static_initializers = value,
            Self::HideFilteredStackFrames => filters.hide_filtered_stack_frames = value,
        }
    }
}

/// 步进过滤器那一块的四态（与仓库既有的 `idle` / `loading` / `ready` / `failed` 同口径）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SteppingFiltersState {
    Idle,
    Loading,
    Ready(SteppingFiltersView),
    Failed(String),
}

// ---------------------------------------------------------------------------
// 会话 / transport 两个事实 + "能不能启动"（纯函数，含反证）
// ---------------------------------------------------------------------------

/// 平台侧 transport 的事实。**H10 未做 ⇒ 只有 `Absent` 会在运行期出现。**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugTransportFacts {
    Absent,
    Present,
}

/// 调试会话的事实。与 transport **分开**：有会话不等于有 transport。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugSessionFacts {
    Absent,
    Present,
}

/// "现在能不能启动一次调试会话"被**哪一个事实**挡住。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugStartBlocker {
    /// 有会话但没有字节通道（要 transport）。
    NoTransport,
    /// 有通道但还没有会话（要 `debug.createSession` + 一个真实 adapterId）。
    NoSession,
    /// 两个都没有 —— 当前运行期的真实情形。
    NoTransportAndNoSession,
}

impl DebugStartBlocker {
    /// 界面用的 catalog / gpui 自有文案键。
    pub fn text_key(self) -> &'static str {
        match self {
            Self::NoTransport => "lithe.debugger.gpui.startBlockedNoTransport",
            Self::NoSession => "lithe.debugger.gpui.startBlockedNoSession",
            Self::NoTransportAndNoSession => "lithe.debugger.gpui.startBlockedBoth",
        }
    }

    /// 诊断行用的稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoTransport => "no-transport",
            Self::NoSession => "no-session",
            Self::NoTransportAndNoSession => "no-transport+no-session",
        }
    }
}

/// **判据本体**：两个事实 → 能不能启动。
///
/// `Some(blocker)` = **不能**启动，并给出被挡的原因；`None` = 两个事实都在，可以启动。
/// 纯函数，所以"把 transport 缺失也算成可启动"这种退化会在单测里变红。
pub fn start_blocker(
    transport: DebugTransportFacts,
    session: DebugSessionFacts,
) -> Option<DebugStartBlocker> {
    match (transport, session) {
        (DebugTransportFacts::Present, DebugSessionFacts::Present) => None,
        (DebugTransportFacts::Absent, DebugSessionFacts::Absent) => {
            Some(DebugStartBlocker::NoTransportAndNoSession)
        }
        (DebugTransportFacts::Absent, DebugSessionFacts::Present) => {
            Some(DebugStartBlocker::NoTransport)
        }
        (DebugTransportFacts::Present, DebugSessionFacts::Absent) => {
            Some(DebugStartBlocker::NoSession)
        }
    }
}

/// 把断点集发给某会话的派发闸门。
///
/// **为什么需要它而不是直接调**：Core 的 `set_breakpoints` 走 `with_session`
/// （`engine.rs:197`），无会话必然 `session_not_found`。所以调用点必须显式问一句
/// "现在有会话吗"；把这件事做成一个类型（而不是散在各处的 `if let Some(session)`），
/// 是为了让"当前恒不派发"这个事实在代码里只有一处、并且带诊断。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugDispatchGate {
    pub session_id: Option<String>,
    pub transport: DebugTransportFacts,
}

impl DebugDispatchGate {
    /// 当前运行期的真实状态：没有会话、没有 transport。
    pub fn disconnected() -> Self {
        Self {
            session_id: None,
            transport: DebugTransportFacts::Absent,
        }
    }

    /// 会话 id（要派发时才有）。
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// 为 false 时**任何** `debug.*` 的会话级命令都不该发出去，只留诊断。
    pub fn can_dispatch(&self) -> bool {
        self.session_id.is_some()
    }
}

// ---------------------------------------------------------------------------
// 执行状态机（idle / running / paused / terminated 四态 + 分代）
// ---------------------------------------------------------------------------

/// 真源 `DebugAdapterState`（`DebugToolbarPresentation.swift:42-58`）在宿主侧归一成的四态。
///
/// 真源还有 `launching` / `initializing` / `ready` / `failed`：前三个都需要"会话在启动中"
/// 这个只有 transport 才能产生的过程，`failed` 则靠 `error_message` 单独表达（见
/// [`DebugPane::error_message`]）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugExecutionState {
    Idle,
    Running,
    Paused,
    Terminated,
}

impl DebugExecutionState {
    /// 状态胶囊的文案键（catalog 既有 `debugger.status*`，真源 `statusText` 的对应档）。
    pub fn status_key(self) -> &'static str {
        match self {
            Self::Idle => "lithe.debugger.statusIdle",
            Self::Running => "lithe.debugger.statusRunning",
            Self::Paused => "lithe.debugger.statusPaused",
            Self::Terminated => "lithe.debugger.statusExited",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Terminated => "terminated",
        }
    }
}

/// 单步 / 执行控制的种类 = 真源工具栏里那些要发 `debug.execute` 的动作
/// （`DebugToolbarPresentation.swift:22-32` 去掉 `viewBreakpoints` / `muteBreakpoints`
/// 两个纯界面动作）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugControl {
    Continue,
    Pause,
    StepOver,
    StepIn,
    StepOut,
    Restart,
    Terminate,
}

impl DebugControl {
    /// Core 的 `DebugExecutionCommand` 取值（`types.rs:309-337` 的 serde camelCase 名）。
    pub fn command(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Pause => "pause",
            Self::StepOver => "next",
            Self::StepIn => "stepIn",
            Self::StepOut => "stepOut",
            Self::Restart => "restart",
            Self::Terminate => "terminate",
        }
    }

    /// `debug.execute` 的载荷（`ExecuteRequest`，`types.rs:340-353`）。
    ///
    /// `operationId` 由调用方给（宿主按代次生成），`singleThread` 恒 `false`
    /// （真源只有右键菜单里的"单线程续跑"会置 true，本切片不画那条）。
    pub fn payload(self, session_id: &str, operation_id: &str, thread_id: Option<i64>) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("sessionId".to_string(), json!(session_id));
        map.insert("operationId".to_string(), json!(operation_id));
        map.insert("command".to_string(), json!(self.command()));
        if let Some(thread_id) = thread_id {
            map.insert("threadId".to_string(), json!(thread_id));
        }
        map.insert("singleThread".to_string(), json!(false));
        Value::Object(map)
    }
}

/// 在飞的一次执行请求（有 `operationId` 与它属于哪一代）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingExecution {
    pub operation_id: String,
    pub generation: u64,
    pub control: DebugControl,
}

/// 执行状态机：四态 + **分代**。
///
/// 分代解决的是本项目一贯的 stale 问题：控制命令是异步的，回来时状态可能已经变了
/// （用户又点了别的、或已经 terminate）。旧代次的回包一律不采纳。
#[derive(Clone, Debug, Default)]
pub struct DebugExecution {
    state: DebugExecutionState,
    generation: u64,
    pending: Option<PendingExecution>,
    /// 最近一次 stopped 的 DAP `reason`（真源状态胶囊的 `Paused · Breakpoint`）。
    stopped_reason: Option<String>,
    /// 最近一次 stopped 的线程（DAP 的单步请求要它，真源 `:418-422` 同样要求）。
    stopped_thread: Option<i64>,
}

impl Default for DebugExecutionState {
    fn default() -> Self {
        Self::Idle
    }
}

impl DebugExecution {
    pub fn state(&self) -> DebugExecutionState {
        self.state
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn pending(&self) -> Option<&PendingExecution> {
        self.pending.as_ref()
    }

    /// 有在飞的控制请求 ⇒ 工具栏那一档要禁用（真源 `isExecutionRequestPending`，`:415-422`）。
    pub fn is_request_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn stopped_reason(&self) -> Option<&str> {
        self.stopped_reason.as_deref()
    }

    pub fn stopped_thread(&self) -> Option<i64> {
        self.stopped_thread
    }

    /// 这一档控制**现在**能不能点（真源 `isDebugToolbarActionDisabled`，`:408-429`）。
    ///
    /// ⚠️ 与真源的一处有意差异：真源把 `resume`（继续）的条件写成 `state != .paused`，
    /// 把 `pause` 写成 `state != .running`；本侧保持**逐条一致**，只把
    /// "没有一个具体的停止线程就禁用单步"这条（`:418-422` 的注释）也照抄。
    pub fn is_enabled(&self, control: DebugControl) -> bool {
        if self.is_request_pending() {
            return false;
        }
        match control {
            // 真源 `restartOrStart`：有会话但不可重启时禁用。本侧把"重启"与"停止"并到
            // `Terminate` 之外的语义上：只有真的处在 running/paused 才允许。
            DebugControl::Restart | DebugControl::Terminate => {
                matches!(
                    self.state,
                    DebugExecutionState::Running | DebugExecutionState::Paused
                )
            }
            DebugControl::Continue => self.state == DebugExecutionState::Paused,
            DebugControl::Pause => self.state == DebugExecutionState::Running,
            DebugControl::StepOver | DebugControl::StepIn | DebugControl::StepOut => {
                self.state == DebugExecutionState::Paused && self.stopped_thread.is_some()
            }
        }
    }

    /// 登记一次在飞请求：**+1 代**，返回这一代。
    ///
    /// 只有 [`Self::is_enabled`] 为真时才该被调用（调用点就是工具栏的禁用闸门）。
    pub fn begin(&mut self, operation_id: &str, control: DebugControl) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(PendingExecution {
            operation_id: operation_id.to_string(),
            generation: self.generation,
            control,
        });
        // 真源在发出请求后就用 `isExecutionRequestPending` 把按钮禁掉，但状态**不立刻**变
        // （状态由适配器的事件推断）—— 所以这里也不动 `state`。
        self.generation
    }

    /// 这一代还在飞吗（旧代次的响应一律丢）。
    pub fn accepts(&self, generation: u64) -> bool {
        self.generation == generation
    }

    /// 适配器说"停住了"。
    pub fn on_stopped(&mut self, reason: &str, thread_id: Option<i64>) {
        self.state = DebugExecutionState::Paused;
        self.stopped_reason = (!reason.is_empty()).then(|| reason.to_string());
        self.stopped_thread = thread_id;
        self.pending = None;
    }

    /// 适配器说"继续跑了"。
    pub fn on_continued(&mut self) {
        self.state = DebugExecutionState::Running;
        self.stopped_reason = None;
        self.stopped_thread = None;
        self.pending = None;
    }

    /// 会话终了。
    pub fn on_terminated(&mut self) {
        self.state = DebugExecutionState::Terminated;
        self.stopped_reason = None;
        self.stopped_thread = None;
        self.pending = None;
    }

    /// 某条在飞请求失败（`OperationFailed`）。
    pub fn on_operation_failed(&mut self, operation_id: &str) -> bool {
        match self.pending.as_ref() {
            Some(pending) if pending.operation_id == operation_id => {
                self.pending = None;
                true
            }
            _ => false,
        }
    }

    /// 状态胶囊的文本：`已暂停 · Breakpoint`（真源 `statusText` + `stopReasonLabel`，`:42-74`）。
    ///
    /// `reason` 的翻译表照抄真源 `stopReasonLabel` 的**键**（`breakpoint` / `step` / …）——
    /// 但 catalog 里没有 `debugger.stopReason.*` 一族，所以这里只把已知取值折到
    /// 状态本身，不硬编中英文（未识别的取值直接不显示，宁可少显示也不编造）。
    pub fn status_text(&self) -> SharedString {
        match (self.state, self.stopped_reason.as_deref()) {
            (DebugExecutionState::Paused, Some(reason)) => {
                if let Some(label) = stop_reason_label(reason) {
                    let label = tr(label);
                    tr_args(
                        "lithe.debugger.gpui.pausedWithReason",
                        &[("reason", label.as_ref())],
                    )
                } else {
                    tr(self.state.status_key())
                }
            }
            _ => tr(self.state.status_key()),
        }
    }
}

/// 真源 `stopReasonLabel`（`DebugToolbarPresentation.swift:61-74`）里**catalog 有对应键**的取值。
///
/// 只有 `breakpoint` / `exception` / `step` / `pause` 四条有现成词（`debugger.breakpoints` /
/// 没有 exception 键 / `debugger.stepOver` 太具体 / `debugger.paused`）；本着"不许自造中英文"
/// 的口径，只承认有键的两条，其余返回 `None`（界面就只显示"已暂停"）。
fn stop_reason_label(reason: &str) -> Option<&'static str> {
    match reason.to_ascii_lowercase().as_str() {
        "breakpoint" => Some("lithe.debugger.breakpoints"),
        "pause" => Some("lithe.debugger.paused"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// DAP 字节归约的输入形状（`debug.receive` / `debug.inspect` / `debug.execute`）
// ---------------------------------------------------------------------------

/// `DebugSessionUpdate`（`types.rs:489-498`）的宿主投影。
///
/// `outbound_frames` 是**完整帧的 base64**（`types.rs:495-496`），本侧**故意只留条数**：
/// 没有 transport，这些字节没有任何去处，把它们搬到前台只是让"看起来在发"更像真的。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugUpdate {
    pub state: Option<DebugExecutionState>,
    pub events: Vec<DebugEventView>,
    pub outbound_frame_count: usize,
}

/// `DebugEventBody`（`types.rs:545-592`）的宿主投影。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugEventView {
    StateChanged {
        state: DebugExecutionState,
    },
    Initialized,
    Capabilities,
    Output {
        category: Option<String>,
        output: String,
    },
    Stopped {
        reason: String,
        thread_id: Option<i64>,
    },
    Continued {
        thread_id: Option<i64>,
    },
    Terminated {
        exit_code: Option<i64>,
    },
    Breakpoint {
        verified: bool,
        message: Option<String>,
        line: Option<i64>,
    },
    OperationCompleted {
        operation_id: String,
        result: DebugOperationResultView,
    },
    OperationFailed {
        operation_id: String,
        message: String,
    },
    RunInTerminalRequested {
        request_id: String,
    },
    /// 认不出的 `type`（Core 契约变了）—— **不静默丢**，界面上留痕。
    Unknown {
        kind: String,
    },
}

/// `DebugOperationResult`（`types.rs:603-651`）的宿主投影。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugOperationResultView {
    Threads(Vec<DebugThreadView>),
    StackTrace(Vec<DebugStackFrameView>),
    Scopes(Vec<DebugScopeView>),
    Variables(Vec<DebugVariableView>),
    Evaluate(Box<DebugVariableView>),
    ExceptionInfo {
        description: Option<String>,
        break_mode: String,
    },
    Acknowledged {
        command: String,
    },
    /// 只留条数：本切片不画热替换 / 智能步入 / 运行到光标的界面。
    Other {
        kind: String,
        count: usize,
    },
}

/// `DebugThread`（`types.rs:694-700`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugThreadView {
    pub id: i64,
    pub name: String,
}

/// `DebugStackFrame`（`types.rs:702-713`）。`line` / `column` 是**一基**。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugStackFrameView {
    pub id: i64,
    pub name: String,
    pub source_path: Option<String>,
    pub line: i64,
    pub column: i64,
    /// 真源用 `isFiltered` 把框架降透明度（`GenericDebugView.swift:657-680`）。
    pub is_filtered: bool,
}

/// `DebugScope`（`types.rs:715-726`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugScopeView {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
    pub named_variables: i64,
    pub indexed_variables: i64,
}

/// `DebugVariable`（`types.rs:728-741`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugVariableView {
    pub name: String,
    pub value: String,
    pub type_name: Option<String>,
    pub evaluate_name: Option<String>,
    pub variables_reference: i64,
    pub named_variables: i64,
    pub indexed_variables: i64,
}

impl DebugVariableView {
    /// 还有子节点吗（真源 `isExpandable`，`GenericDebugView.swift:827`）。
    pub fn is_expandable(&self) -> bool {
        self.variables_reference != 0
    }

    fn from_payload(value: &Value) -> Option<Self> {
        Some(Self {
            name: value.get("name")?.as_str()?.to_string(),
            value: value
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            type_name: string_field(value, "type"),
            evaluate_name: string_field(value, "evaluateName"),
            variables_reference: value
                .get("variablesReference")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            named_variables: value
                .get("namedVariables")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            indexed_variables: value
                .get("indexedVariables")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        })
    }
}

/// 变量树里的一行（**深度** + 变量）。
///
/// 变量树的投影口径照真源 `visibleVariableRows`（`GenericDebugView.swift:819-891`：
/// 按深度缩进 `10 + depth * 14`）。是否展开由宿主自己记（真源的展开状态也在 feature model 里）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugVariableRow {
    pub depth: usize,
    pub variable: DebugVariableView,
}

/// 哪些变量是展开的（键 = 父变量的 `variablesReference`，值 = 已加载的子节点）。
#[derive(Default, Clone, Debug)]
pub struct DebugVariableTree {
    expanded: BTreeSet<i64>,
}

impl DebugVariableTree {
    pub fn is_expanded(&self, variables_reference: i64) -> bool {
        self.expanded.contains(&variables_reference)
    }

    /// 点一行：翻转展开态，返回"翻转后是不是展开的"。
    pub fn toggle(&mut self, variables_reference: i64) -> bool {
        if self.expanded.contains(&variables_reference) {
            self.expanded.remove(&variables_reference);
            false
        } else {
            self.expanded.insert(variables_reference);
            true
        }
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// 把一个 scope 的变量摊成树行。当前只摊**一层**（子节点要再发
    /// `debug.inspect{kind:variables}` 去取，而那需要活会话）—— 所以嵌套展开在界面上
    /// 会显示"该层子节点尚未加载"，而不是画一个空壳。
    pub fn rows(&self, variables: &[DebugVariableView]) -> Vec<DebugVariableRow> {
        variables
            .iter()
            .map(|variable| DebugVariableRow {
                depth: 0,
                variable: variable.clone(),
            })
            .collect()
    }
}

/// `debug.receive` / `debug.inspect` / `debug.execute` 响应的归约（**纯函数**）。
///
/// 单测用**合成响应**打这一层：真机形状取自 `DebugSessionUpdate` 与 `DebugEventBody`
/// （`types.rs:489-592`），不需要真 DAP 进程、不 sleep、不联网。
pub fn parse_session_update(value: &Value) -> DebugUpdate {
    let mut update = DebugUpdate {
        state: value
            .get("state")
            .and_then(Value::as_str)
            .and_then(parse_execution_state),
        outbound_frame_count: value
            .get("outboundFrames")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        events: Vec::new(),
    };
    let Some(events) = value.get("events").and_then(Value::as_array) else {
        return update;
    };
    update.events = events.iter().filter_map(parse_event).collect();
    update
}

/// Core 的 `DebugSessionState` 序列化名（`types.rs:6-19`）→ 宿主四态。
pub fn parse_execution_state(state: &str) -> Option<DebugExecutionState> {
    match state {
        "idle" => Some(DebugExecutionState::Idle),
        "running" | "launching" | "initializing" | "ready" => Some(DebugExecutionState::Running),
        "paused" => Some(DebugExecutionState::Paused),
        "terminated" | "terminating" | "failed" => Some(DebugExecutionState::Terminated),
        _ => None,
    }
}

fn parse_event(value: &Value) -> Option<DebugEventView> {
    let kind = value.get("type")?.as_str()?;
    Some(match kind {
        "stateChanged" => DebugEventView::StateChanged {
            state: value
                .get("state")
                .and_then(Value::as_str)
                .and_then(parse_execution_state)
                .unwrap_or(DebugExecutionState::Idle),
        },
        "initialized" => DebugEventView::Initialized,
        "capabilities" => DebugEventView::Capabilities,
        "output" => DebugEventView::Output {
            category: string_field(value, "category"),
            output: value
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "stopped" => DebugEventView::Stopped {
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            thread_id: value.get("threadId").and_then(Value::as_i64),
        },
        "continued" => DebugEventView::Continued {
            thread_id: value.get("threadId").and_then(Value::as_i64),
        },
        "terminated" => DebugEventView::Terminated {
            exit_code: value.get("exitCode").and_then(Value::as_i64),
        },
        "breakpoint" => {
            let breakpoint = value.get("breakpoint")?;
            DebugEventView::Breakpoint {
                verified: breakpoint
                    .get("verified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                message: string_field(breakpoint, "message"),
                line: breakpoint.get("line").and_then(Value::as_i64),
            }
        }
        "operationCompleted" => DebugEventView::OperationCompleted {
            operation_id: value
                .get("operationId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            result: value
                .get("result")
                .and_then(parse_operation_result)
                .unwrap_or(DebugOperationResultView::Other {
                    kind: "unknown".to_string(),
                    count: 0,
                }),
        },
        "operationFailed" => DebugEventView::OperationFailed {
            operation_id: value
                .get("operationId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            message: value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "runInTerminalRequested" => DebugEventView::RunInTerminalRequested {
            request_id: value
                .get("requestId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        other => DebugEventView::Unknown {
            kind: other.to_string(),
        },
    })
}

fn parse_operation_result(value: &Value) -> Option<DebugOperationResultView> {
    let kind = value.get("kind")?.as_str()?;
    Some(match kind {
        "threads" => DebugOperationResultView::Threads(
            value
                .get("threads")?
                .as_array()?
                .iter()
                .filter_map(|thread| {
                    Some(DebugThreadView {
                        id: thread.get("id")?.as_i64()?,
                        name: thread
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    })
                })
                .collect(),
        ),
        "stackTrace" => DebugOperationResultView::StackTrace(
            value
                .get("stackFrames")?
                .as_array()?
                .iter()
                .filter_map(|frame| {
                    Some(DebugStackFrameView {
                        id: frame.get("id")?.as_i64()?,
                        name: frame
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        source_path: string_field(frame, "sourcePath"),
                        line: frame.get("line").and_then(Value::as_i64).unwrap_or(0),
                        column: frame.get("column").and_then(Value::as_i64).unwrap_or(0),
                        is_filtered: frame
                            .get("isFiltered")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect(),
        ),
        "scopes" => DebugOperationResultView::Scopes(
            value
                .get("scopes")?
                .as_array()?
                .iter()
                .filter_map(|scope| {
                    Some(DebugScopeView {
                        name: scope
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        variables_reference: scope
                            .get("variablesReference")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        expensive: scope
                            .get("expensive")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        named_variables: scope
                            .get("namedVariables")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        indexed_variables: scope
                            .get("indexedVariables")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                    })
                })
                .collect(),
        ),
        "variables" => DebugOperationResultView::Variables(
            value
                .get("variables")?
                .as_array()?
                .iter()
                .filter_map(DebugVariableView::from_payload)
                .collect(),
        ),
        "evaluate" => DebugOperationResultView::Evaluate(Box::new(
            DebugVariableView::from_payload(value.get("variable")?)?,
        )),
        "exceptionInfo" => {
            let info = value.get("exceptionInfo")?;
            DebugOperationResultView::ExceptionInfo {
                description: string_field(info, "description"),
                break_mode: info
                    .get("breakMode")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }
        }
        "acknowledged" => DebugOperationResultView::Acknowledged {
            command: value
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        other => DebugOperationResultView::Other {
            kind: other.to_string(),
            count: value
                .get(other)
                .and_then(Value::as_array)
                .map(Vec::len)
                .or_else(|| value.get("targets").and_then(Value::as_array).map(Vec::len))
                .unwrap_or(0),
        },
    })
}

// ---------------------------------------------------------------------------
// 调用栈 / 变量 / 异常的宿主投影（`debug.inspect` 的结果 → 五块里那几块）
// ---------------------------------------------------------------------------

/// 检查结果：线程 / 调用栈 / 变量（真源 `inspector` 的左、右两列的**全部**数据）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugInspection {
    pub threads: Vec<DebugThreadView>,
    pub stack_frames: Vec<DebugStackFrameView>,
    pub scopes: Vec<DebugScopeView>,
    pub variables: Vec<DebugVariableView>,
    pub exception: Option<DebugExceptionView>,
}

/// `DebugExceptionInfo`（`types.rs:743-751`）的宿主投影（真源 `exceptionInspector`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugExceptionView {
    pub description: Option<String>,
    pub break_mode: String,
}

impl DebugInspection {
    /// 把一条 `OperationCompleted` 的结果并进来。
    ///
    /// 这是"五块的数据从哪来"的**唯一**入口：`debug.inspect` 的每一种 `kind` 只落一格
    /// （`DebugOperationResult` 的变体与 `DebugInspectKind` 一一对应，`types.rs:355-385`）。
    pub fn adopt(&mut self, result: &DebugOperationResultView) {
        match result {
            DebugOperationResultView::Threads(threads) => self.threads = threads.clone(),
            DebugOperationResultView::StackTrace(frames) => self.stack_frames = frames.clone(),
            DebugOperationResultView::Scopes(scopes) => {
                self.scopes = scopes.clone();
                // 换了栈帧 ⇒ 变量表作废（它属于上一个 frame 的 scope）。
                self.variables.clear();
            }
            DebugOperationResultView::Variables(variables) => self.variables = variables.clone(),
            DebugOperationResultView::ExceptionInfo {
                description,
                break_mode,
            } => {
                self.exception = Some(DebugExceptionView {
                    description: description.clone(),
                    break_mode: break_mode.clone(),
                });
            }
            _ => {}
        }
    }

    /// 清空所有 inspect 结果（状态离开 paused 时调用）。
    pub fn clear(&mut self) {
        self.threads.clear();
        self.stack_frames.clear();
        self.scopes.clear();
        self.variables.clear();
        self.exception = None;
    }
}

// ---------------------------------------------------------------------------
// 监视（真源 `watchSectionHeader` + `Watches` 列表，`GenericDebugView.swift:1126-1155`）
// ---------------------------------------------------------------------------

/// 一条监视表达式。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugWatch {
    pub id: u64,
    pub expression: String,
    /// 求值状态。真源有「Evaluating…」/「Not available」/值/错误四档
    /// （`GenericDebugView.swift:910-929`）。
    pub state: WatchState,
    pub value: Option<String>,
    pub type_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchState {
    NotEvaluated,
    Evaluating,
    Ready,
    Failed,
}

/// 监视表。`id` 是自增的**业务身份**（不随排序变化），也用作 `ElementId` 的一部分。
#[derive(Default, Clone, Debug)]
pub struct DebugWatches {
    items: Vec<DebugWatch>,
    next_id: u64,
    /// 哪几条监视的求值请求正在飞（`operationId` 是 Core 的相关键，`types.rs:407-409`）。
    ///
    /// **一个列表而不是一个槽**：多条监视可以同时求值（刷新一次就是把每条都发一遍），
    /// 单槽会让先发的那条永远停在 `Evaluating`（它的响应回来时槽已经被后发的占了）。
    inflight: Vec<(String, u64)>,
}

impl DebugWatches {
    pub fn items(&self) -> &[DebugWatch] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 加一条；空白表达式与重复表达式都不加（真源 `addWatch` 也做去空）。
    pub fn add(&mut self, expression: &str) -> Option<u64> {
        let expression = expression.trim();
        if expression.is_empty() {
            return None;
        }
        if self
            .items
            .iter()
            .any(|watch| watch.expression == expression)
        {
            return None;
        }
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        self.items.push(DebugWatch {
            id,
            expression: expression.to_string(),
            state: WatchState::NotEvaluated,
            value: None,
            type_name: None,
            error: None,
        });
        Some(id)
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|watch| watch.id != id);
        self.items.len() != before
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 把所有监视标成"要重新求值"（真源 `refreshWatches`）。
    pub fn mark_all_stale(&mut self) {
        for watch in &mut self.items {
            watch.state = WatchState::NotEvaluated;
            watch.value = None;
            watch.error = None;
        }
    }

    /// 登记一条在飞求值：返回 `operationId`（调用方用它发 `debug.inspect{kind:evaluate}`）。
    pub fn begin_evaluation(&mut self, id: u64, operation_id: &str) -> bool {
        let Some(watch) = self.items.iter_mut().find(|watch| watch.id == id) else {
            return false;
        };
        watch.state = WatchState::Evaluating;
        watch.error = None;
        self.inflight.push((operation_id.to_string(), id));
        true
    }

    /// `OperationCompleted{Evaluate}` 回到前台：按 `operationId` 认领。
    ///
    /// 认不出 `operationId` ⇒ 返回 `false`（**不是**"随便找一条填上" —— 那会把 A 的值写到 B 上）。
    pub fn adopt_evaluation(&mut self, operation_id: &str, variable: &DebugVariableView) -> bool {
        let Some(index) = self
            .inflight
            .iter()
            .position(|(pending, _)| pending == operation_id)
        else {
            return false;
        };
        let (_, id) = self.inflight.remove(index);
        if let Some(watch) = self.items.iter_mut().find(|watch| watch.id == id) {
            watch.state = WatchState::Ready;
            watch.value = Some(variable.value.clone());
            watch.type_name = variable.type_name.clone();
            return true;
        }
        false
    }

    /// `OperationFailed` 回到前台：按 `operationId` 认领，把错误记在那一条上。
    pub fn adopt_failure(&mut self, operation_id: &str, message: &str) -> bool {
        let Some(index) = self
            .inflight
            .iter()
            .position(|(pending, _)| pending == operation_id)
        else {
            return false;
        };
        let (_, id) = self.inflight.remove(index);
        if let Some(watch) = self.items.iter_mut().find(|watch| watch.id == id) {
            watch.state = WatchState::Failed;
            watch.error = Some(message.to_string());
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------------------
// 视图
// ---------------------------------------------------------------------------

/// 面板向宿主播报的事（宿主拿它做**与面板内容无关的布局反应**）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugPaneEvent {
    /// 用户要把这个工具窗收起来（真源工具栏右端的「隐藏」，`GenericDebugView.swift:151-155`）。
    Hide,
    /// 断点集变了（加 / 删 / 改启用 / 清空 / 静音 / `relocate` 采纳）。
    ///
    /// 宿主据此把快照重推给编辑区的行号槽 —— 断点表的 owner 是本模块，编辑区只画，
    /// 所以"谁改了"必须由这一个事件说出来（不然 relocate 改了断点、gutter 不跟着动）。
    BreakpointsChanged,
}

/// 断点表的键：工作区根下的文件取**工作区相对路径**（`/` 分隔），根外的取绝对路径。
///
/// 为什么这样切：Core 的 `sourcePath` 两种都收（`debug/engine.rs:178` 的 `validate_path`
/// 只要求非空且长得像路径），而工作区相对路径与仓库其它地方（`searchIndex.update` 的
/// `paths`）同口径 —— 换根之后断点表不会被旧根的绝对路径污染。
///
/// 纯函数，所以有单测钉住"根内相对 / 根外绝对 / 根本身"三种情形。
pub fn debug_key(root: &std::path::Path, path: &std::path::Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => {
            let text = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            if text.is_empty() {
                // `path == root`：不是文件，退回绝对路径（不要产出空键 —— 空键会把
                // 所有"根本身"的调用混成一条）。
                path.to_string_lossy().replace('\\', "/")
            } else {
                text
            }
        }
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

/// 真源工具栏九档的**稳定顺序**（`DebugToolbarPresentation.swift:22-32`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugToolbarAction {
    RestartOrStart,
    Stop,
    Resume,
    Pause,
    StepOver,
    StepInto,
    StepOut,
    ViewBreakpoints,
    MuteBreakpoints,
}

/// 工具栏顺序（真源 `primaryActions` 是**有序数组**，`DebugToolbarPresentation.swift:22-32`）。
pub const DEBUG_TOOLBAR_ACTIONS: [DebugToolbarAction; 9] = [
    DebugToolbarAction::RestartOrStart,
    DebugToolbarAction::Stop,
    DebugToolbarAction::Resume,
    DebugToolbarAction::Pause,
    DebugToolbarAction::StepOver,
    DebugToolbarAction::StepInto,
    DebugToolbarAction::StepOut,
    DebugToolbarAction::ViewBreakpoints,
    DebugToolbarAction::MuteBreakpoints,
];

/// 哪些档之后画一条分隔线（真源 `separatorsAfter: [.stop, .stepOut]`，
/// `DebugToolbarPresentation.swift:34`）。
pub fn debug_toolbar_has_separator_after(action: DebugToolbarAction) -> bool {
    matches!(
        action,
        DebugToolbarAction::Stop | DebugToolbarAction::StepOut
    )
}

/// 工具栏高度（真源 `toolbarHeight = 36`，`DebugToolbarPresentation.swift:39`）。
pub const DEBUG_TOOLBAR_HEIGHT_SPEC: f32 = 36.;
/// 会话头高度（真源 `sessionHeaderHeight = 34`，`DebugToolbarPresentation.swift:40`）。
pub const DEBUG_SESSION_HEADER_HEIGHT_SPEC: f32 = 34.;
/// 工具栏图标尺寸（真源 `iconSize = 18`，`DebugToolbarPresentation.swift:38`）。
pub const DEBUG_TOOLBAR_ICON_SPEC: f32 = 18.;
/// 内容页签行高（真源 `contentTabs` 的 `.frame(height: 25)` + `.padding(.horizontal, 8)`
/// + 外层 `.frame(height: 32)`，`GenericDebugView.swift:96-133`）。
pub const DEBUG_CONTENT_TABS_HEIGHT_SPEC: f32 = 32.;
/// 段标题行高（真源 `sectionHeader` 的 `.frame(height: 27)`，`GenericDebugView.swift:1379`；
/// `variablesHeader` 同值 `:994`）。
pub const DEBUG_SECTION_HEADER_HEIGHT_SPEC: f32 = 27.;
/// 求值行高（真源 `evaluateRow` 的 `.frame(height: 32)`，`GenericDebugView.swift:1123`）。
pub const DEBUG_EVALUATE_ROW_HEIGHT_SPEC: f32 = 32.;
/// 变量树每一层的缩进（真源 `10 + depth * 14`，`GenericDebugView.swift:853`）。
pub const DEBUG_VARIABLE_INDENT_SPEC: f32 = 14.;
/// 左侧「调用栈」列的最小宽（真源 `minWidth: 240`，`GenericDebugView.swift:614`）。
pub const DEBUG_STACK_COLUMN_MIN_WIDTH_SPEC: f32 = 240.;
/// 右侧「数据」列的最小宽（真源 `minWidth: 300`，`GenericDebugView.swift:616`）。
pub const DEBUG_DATA_COLUMN_MIN_WIDTH_SPEC: f32 = 300.;

/// 面板内容页（真源 `DebugContent`：`debugger` / `console`，`GenericDebugView.swift:87-94`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugContent {
    Debugger,
    Console,
}

impl DebugContent {
    /// 页签名键（catalog 既有 `debugger.threadsAndVariables` / `debugger.console`，
    /// 真源 `contentTabs` 用的是 `DebugContent.title`）。
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Debugger => "lithe.debugger.threadsAndVariables",
            Self::Console => "lithe.debugger.console",
        }
    }
}

/// 调试工具窗的实体。
pub struct DebugPane {
    breakpoints: DebugBreakpointStore,
    execution: DebugExecution,
    inspection: DebugInspection,
    tree: DebugVariableTree,
    watches: DebugWatches,
    filters: SteppingFiltersState,
    /// 求值输入框的当前文本（真源 `@State evaluateExpression`，`GenericDebugView.swift:9`）。
    evaluate_input: String,
    content: DebugContent,
    /// 面板最近一次的失败消息（真源 `feature.errorMessage`，`GenericDebugView.swift:27`）。
    error_message: Option<String>,
    /// 适配器输出（真源 `feature.outputPresentation`）。本切片**没有 transport ⇒ 恒空**。
    output: Vec<String>,
    /// 在飞的核心调用（`debug.steppingFilters` / `debug.relocateBreakpoints`）。
    ///
    /// **必须被持有**：gpui 的 `Task` 一 drop 就取消。
    filters_task: Option<Task<()>>,
    /// 最近一次编辑时缓存的前一份正文（`debug.relocateBreakpoints` 要"编辑前的完整源文本"）。
    last_source: Option<(String, String)>,
    /// 派发闸门（见 [`DebugDispatchGate`]）。当前恒为 [`DebugDispatchGate::disconnected`]。
    gate: DebugDispatchGate,
}

impl EventEmitter<DebugPaneEvent> for DebugPane {}

impl Default for DebugPane {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugPane {
    pub fn new() -> Self {
        Self {
            breakpoints: DebugBreakpointStore::default(),
            execution: DebugExecution::default(),
            inspection: DebugInspection::default(),
            tree: DebugVariableTree::default(),
            watches: DebugWatches::default(),
            filters: SteppingFiltersState::Idle,
            evaluate_input: String::new(),
            content: DebugContent::Debugger,
            error_message: None,
            output: Vec::new(),
            filters_task: None,
            last_source: None,
            gate: DebugDispatchGate::disconnected(),
        }
    }

    /// 平台侧 transport 的事实。**运行期恒 `Absent`**（H10 未做，见模块文档）。
    pub fn transport(&self) -> DebugTransportFacts {
        self.gate.transport
    }

    /// 会话的事实。**运行期恒 `Absent`**（本切片不调 `debug.createSession`，
    /// 理由见 [`Self::start_session`]）。
    pub fn session(&self) -> DebugSessionFacts {
        if self.gate.can_dispatch() {
            DebugSessionFacts::Present
        } else {
            DebugSessionFacts::Absent
        }
    }

    /// **能不能启动**：直接就是 [`start_blocker`] 的调用点（界面的禁用与文案都由它来）。
    pub fn start_blocker(&self) -> Option<DebugStartBlocker> {
        start_blocker(self.transport(), self.session())
    }

    pub fn breakpoints(&self) -> &DebugBreakpointStore {
        &self.breakpoints
    }

    /// 行号槽要的快照：键还原成**绝对路径**（编辑器按 `Buffer::path` 取），
    /// 值是该文件的标记。店里的键可能是工作区相对路径（见 [`debug_key`]），所以这里
    /// 用 `root` 拼回去；键本身是绝对路径（根外文件）时原样用。
    pub fn gutter_snapshot(&self, root: &Path) -> BTreeMap<PathBuf, Vec<GutterBreakpoint>> {
        self.breakpoints
            .files()
            .map(|key| {
                let path = if Path::new(key.as_str()).is_absolute() {
                    PathBuf::from(key)
                } else {
                    root.join(key)
                };
                let marks = self
                    .breakpoints
                    .file(key)
                    .iter()
                    .map(|breakpoint| GutterBreakpoint {
                        // 一基行号；店里的 `line` 已经被 `toggle` 挡过 `< 1`，这里再兜一次底，
                        // 免得一个坏值把 `usize` 转换变成负数回绕。
                        line: usize::try_from(breakpoint.line.max(1)).unwrap_or(1),
                        enabled: breakpoint.enabled,
                        verified: breakpoint.verified,
                    })
                    .collect();
                (path, marks)
            })
            .collect()
    }

    pub fn execution(&self) -> &DebugExecution {
        &self.execution
    }

    pub fn inspection(&self) -> &DebugInspection {
        &self.inspection
    }

    pub fn watches(&self) -> &DebugWatches {
        &self.watches
    }

    pub fn filters(&self) -> &SteppingFiltersState {
        &self.filters
    }

    pub fn content(&self) -> DebugContent {
        self.content
    }

    /// 有内容可显示吗（真源 `feature.isSessionActive || feature.hasOutput || errorMessage != nil`，
    /// `GenericDebugView.swift:27`）。
    pub fn has_content(&self) -> bool {
        self.session() == DebugSessionFacts::Present
            || !self.output.is_empty()
            || self.error_message.is_some()
    }

    /// 读一次 Java 适配器的**默认**步进过滤器（真源进入调试面板时读一次
    /// `javaSteppingFilters`）。
    ///
    /// 这是本切片**真的调到 Core** 的两条命令之一：`debug.steppingFilters`
    /// （`protocol/command.rs:121-122`）只做归一化、不用 `with_session`
    /// （`engine.rs:163-172`）⇒ 无会话也能调。Core 调用进 `background_spawn`
    /// （`shared/src/core_client.rs:27-28` 的口径），**不在 UI 线程上等**。
    pub fn load_stepping_filters(&mut self, cx: &mut Context<Self>) {
        if matches!(self.filters, SteppingFiltersState::Loading) {
            return;
        }
        let payload = stepping_filters_payload(STEPPING_FILTERS_ADAPTER);
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} steppingFilters adapter={STEPPING_FILTERS_ADAPTER}");
        self.filters = SteppingFiltersState::Loading;
        self.filters_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { core_json(STEPPING_FILTERS_COMMAND, payload) })
                .await;
            let _ = this.update(cx, |pane, cx| {
                pane.filters = match result {
                    Ok(Some(data)) => match SteppingFiltersView::from_payload(&data) {
                        Some(filters) => {
                            eprintln!(
                                "{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=ok classes={} synthetics={} constructors={} staticInit={} hide={}",
                                filters.class_name_filters.len(),
                                filters.skip_synthetics,
                                filters.skip_constructors,
                                filters.skip_static_initializers,
                                filters.hide_filtered_stack_frames,
                            );
                            SteppingFiltersState::Ready(filters)
                        }
                        None => {
                            eprintln!(
                                "{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=shape_unknown"
                            );
                            SteppingFiltersState::Failed(tr("lithe.debugger.gpui.filtersShapeUnknown").to_string())
                        }
                    },
                    Ok(None) => {
                        eprintln!("{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=missing_data");
                        SteppingFiltersState::Failed(
                            tr("lithe.debugger.gpui.filtersMissingData").to_string(),
                        )
                    }
                    Err(error) => {
                        eprintln!(
                            "{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=failed error={error}"
                        );
                        SteppingFiltersState::Failed(error.to_string())
                    }
                };
                cx.notify();
            });
        }));
    }

    /// 一次编辑：调 `debug.relocateBreakpoints` 把断点跟着正文走。
    ///
    /// 这是本切片**真的调到 Core** 的第二条命令：`breakpoint_relocation.rs:42-105` 是纯函数
    /// （源文本 + 一次 UTF-16 编辑 + 断点集 → 新断点集），不碰会话 ⇒ 无 transport 也能调，
    /// 而且**必须**调（宿主自己算位移就是第二份真源）。
    ///
    /// `path` 是工作区相对路径（与 [`DebugBreakpointStore`] 的键同口径）。
    pub fn apply_source_edit(
        &mut self,
        path: &str,
        source_before: &str,
        source_after: &str,
        cx: &mut Context<Self>,
    ) {
        self.last_source = Some((path.to_string(), source_after.to_string()));
        let breakpoints = self.breakpoints.file(path);
        if breakpoints.is_empty() {
            // 这个文件没有断点 ⇒ 没有可移的锚点，不必发 Core（也避免每次敲键都跑一趟）。
            return;
        }
        let Some(edit) = derive_source_edit(source_before, source_after) else {
            return;
        };
        let payload = relocate_breakpoints_payload(source_before, &edit, breakpoints);
        let path = path.to_string();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} start={} end={} breakpoints={}",
            edit.start_utf16_offset,
            edit.end_utf16_offset,
            breakpoints.len()
        );
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    core_json(RELOCATE_BREAKPOINTS_COMMAND, payload)
                })
                .await;
            let _ = this.update(cx, |pane, cx| {
                match result {
                    Ok(Some(data)) => match parse_relocate_result(&data) {
                        Some(relocated) => {
                            eprintln!(
                                "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=ok breakpoints={}",
                                relocated.len()
                            );
                            pane.breakpoints.replace_file(&path, relocated);
                            // 断点位置真的变了 ⇒ 外壳要把新快照推给行号槽。
                            cx.emit(DebugPaneEvent::BreakpointsChanged);
                            cx.notify();
                        }
                        // 读不懂的响应**保留旧断点**：一次契约漂移不该让用户的断点消失。
                        None => eprintln!(
                            "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=shape_unknown"
                        ),
                    },
                    Ok(None) => {
                        eprintln!("{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=missing_data")
                    }
                    Err(error) => {
                        eprintln!(
                            "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=failed error={error}"
                        );
                    }
                }
            });
        })
        .detach();
    }

    /// 点击行号槽 / 「切换当前行断点」（真源 `toggleCurrentLineBreakpoint`，
    /// `debugger.toggleCurrentLineBreakpoint`）。
    pub fn toggle_breakpoint_at(&mut self, path: &str, line: i64, cx: &mut Context<Self>) {
        if line < 1 {
            return;
        }
        let change = self.breakpoints.toggle(path, line);
        if change == DebugBreakpointChange::Unchanged {
            return;
        }
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} breakpoint toggle path={path} line={line} total={}",
            self.breakpoints.total()
        );
        self.sync_breakpoints(path);
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    /// 「启用 / 禁用断点」（真源 gutter 右键）。
    pub fn set_breakpoint_enabled(
        &mut self,
        path: &str,
        line: i64,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self.breakpoints.set_enabled(path, line, enabled) == DebugBreakpointChange::Unchanged {
            return;
        }
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} breakpoint enabled={enabled} path={path} line={line}");
        self.sync_breakpoints(path);
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    /// 「移除断点」（真源 gutter 右键的 `Remove Breakpoint`）。
    pub fn remove_breakpoint(&mut self, path: &str, line: i64, cx: &mut Context<Self>) {
        if self.breakpoints.remove(path, line) == DebugBreakpointChange::Unchanged {
            return;
        }
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} breakpoint remove path={path} line={line}");
        self.sync_breakpoints(path);
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    /// 「清空断点」（真源 `debugger.clearBreakpoints`）。
    pub fn clear_breakpoints(&mut self, cx: &mut Context<Self>) {
        if self.breakpoints.is_empty() {
            return;
        }
        let paths: Vec<String> = self.breakpoints.files().cloned().collect();
        self.breakpoints.clear();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} breakpoint clear files={}",
            paths.len()
        );
        // 每个文件都要发一条"空集"才算真的清干净（DAP 是**按文件替换**语义）。
        for path in paths {
            self.sync_breakpoints(&path);
        }
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    /// 「静音断点」（真源工具栏 `muteBreakpoints`）。
    pub fn toggle_mute_breakpoints(&mut self, cx: &mut Context<Self>) {
        self.breakpoints.toggle_mute();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} breakpoint muted={}",
            self.breakpoints.muted()
        );
        for path in self.breakpoints.files().cloned().collect::<Vec<_>>() {
            self.sync_breakpoints(&path);
        }
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    /// 把某个文件的断点集发给 Core —— **走派发闸门**。
    ///
    /// 闸门关着（当前运行期的常态）时**一个字节都不发**，只留一行诊断说明原因。
    /// 这是"不假装"的关键：`debug.setBreakpoints` 需要会话（`engine.rs:197` 的
    /// `with_session`），无会话必然 `session_not_found` —— 与其发出去再报错，
    /// 不如把"为什么没发"写在诊断里。
    fn sync_breakpoints(&mut self, path: &str) {
        match self.gate.session_id() {
            Some(session_id) => {
                let payload = self.breakpoints.set_breakpoints_payload(session_id, path);
                eprintln!(
                    "{DEBUG_DIAGNOSTIC_TAG} setBreakpoints session={session_id} path={path} status=dispatch"
                );
                // 真实派发要等会话存在（H10/H11）。这里**故意不 spawn 一个发不出去的调用**：
                // 闸门开着就说明会话在，届时换成 `cx.background_spawn(core_json(...))`。
                let _ = payload;
            }
            None => eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} setBreakpoints path={path} status=skipped reason=no-session (H10/H11)"
            ),
        }
    }

    /// 「启动调试」（真源空态的 `Start Debugging`，`GenericDebugView.swift:1331`）。
    ///
    /// **本切片不调 `debug.createSession` / `debug.launch` / `debug.javaTestLaunch`**，
    /// 三条理由：
    ///
    /// 1. `debug.createSession` 要一个 `adapter_id`（`types.rs:24-31`），而 adapterId 正是
    ///    H11（`vscode.java.startDebugSession`）才会给出的事实；编一个 `"java"` 会建出
    ///    "适配器并不存在"的会话 —— 那是**假装有会话**。
    /// 2. 建出来的会话**什么都做不了**：`launch` / `inspect` / `execute` 的 `outbound_frames`
    ///    没有出口（没有 transport），`did_receive_initialized` 永远为假 ⇒
    ///    `set_breakpoints` 连帧都不会排（`engine.rs:201-203`）。它在 Core 里只能停在 `Idle`。
    /// 3. 真源的「启动调试」会立刻把状态推到 `launching`；本侧推不动，于是"启动"这个动作
    ///    在界面上只能表现为**不变**，那是最坏的一种假装。
    ///
    /// 所以入口是**禁用 + 写明原因**（[`start_blocker`]），而不是画一个点了没反应的按钮。
    pub fn start_session(&mut self, cx: &mut Context<Self>) {
        match self.start_blocker() {
            Some(blocker) => {
                eprintln!(
                    "{DEBUG_DIAGNOSTIC_TAG} start status=blocked reason={}",
                    blocker.as_str()
                );
                cx.notify();
            }
            None => {
                // 到这一步说明 transport 与会话都在 —— 当前运行期**不可能**走到
                // （两个事实恒为 `Absent`）。留一条诊断而不是 `unreachable!()`：
                // 面板比平台层活得长是正常情形，不该 panic。
                eprintln!("{DEBUG_DIAGNOSTIC_TAG} start status=ready_to_dispatch");
            }
        }
    }

    /// 「添加监视」（真源 `debugger.addWatch`）。
    pub fn add_watch(&mut self, expression: &str, cx: &mut Context<Self>) {
        if self.watches.add(expression).is_some() {
            eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} watch add total={}",
                self.watches.len()
            );
            cx.notify();
        }
    }

    pub fn remove_watch(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.watches.remove(id) {
            eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} watch remove total={}",
                self.watches.len()
            );
            cx.notify();
        }
    }

    /// 「刷新监视」（真源 `debugger.refreshWatches`）：全部标脏，下一次 paused 时重取。
    pub fn refresh_watches(&mut self, cx: &mut Context<Self>) {
        self.watches.mark_all_stale();
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} watch refresh");
        cx.notify();
    }

    /// 求值（真源 `evaluateRow` 的 ▶，`.disabled(feature.state != .paused || expr.isEmpty)`，
    /// `GenericDebugView.swift:1119`）。
    ///
    /// 本切片**不发** `debug.inspect{kind:evaluate}`（要会话）；把"点了会发生什么"如实说清：
    /// 追加到监视列表并留一行诊断 —— 这样用户在能调试之前就能把表达式准备好。
    pub fn evaluate(&mut self, cx: &mut Context<Self>) {
        let expression = self.evaluate_input.trim().to_string();
        if expression.is_empty() {
            return;
        }
        if self.execution.state() != DebugExecutionState::Paused {
            eprintln!("{DEBUG_DIAGNOSTIC_TAG} evaluate status=skipped reason=not-paused");
            return;
        }
        let blocker = self.start_blocker();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} evaluate status=skipped reason={}",
            blocker
                .map(DebugStartBlocker::as_str)
                .unwrap_or("no-session")
        );
        self.add_watch(&expression, cx);
        self.evaluate_input.clear();
        cx.notify();
    }

    pub fn set_evaluate_input(&mut self, value: String, cx: &mut Context<Self>) {
        if self.evaluate_input != value {
            self.evaluate_input = value;
            cx.notify();
        }
    }

    pub fn select_content(&mut self, content: DebugContent, cx: &mut Context<Self>) {
        if self.content != content {
            self.content = content;
            eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} content={}",
                match content {
                    DebugContent::Debugger => "debugger",
                    DebugContent::Console => "console",
                }
            );
            cx.notify();
        }
    }

    /// 采纳一份 `DebugSessionUpdate`（`debug.receive` / `debug.execute` / `debug.inspect` 的响应）。
    ///
    /// **当前没有任何调用方**（要 transport 才有响应），但归约逻辑是本任务要求的纯逻辑，
    /// 单测用合成响应直接打它。
    pub fn adopt_update(&mut self, update: &DebugUpdate) {
        if let Some(state) = update.state {
            match state {
                DebugExecutionState::Paused => {}
                DebugExecutionState::Terminated => {
                    self.execution.on_terminated();
                    self.inspection.clear();
                }
                _ => {}
            }
        }
        for event in &update.events {
            match event {
                DebugEventView::StateChanged { state } => {
                    if *state == DebugExecutionState::Terminated {
                        self.execution.on_terminated();
                        self.inspection.clear();
                    }
                }
                DebugEventView::Stopped { reason, thread_id } => {
                    self.execution.on_stopped(reason, *thread_id)
                }
                DebugEventView::Continued { .. } => self.execution.on_continued(),
                DebugEventView::Terminated { .. } => {
                    self.execution.on_terminated();
                    self.inspection.clear();
                }
                DebugEventView::OperationCompleted {
                    operation_id,
                    result,
                } => {
                    self.inspection.adopt(result);
                    if let DebugOperationResultView::Evaluate(variable) = result {
                        self.watches.adopt_evaluation(operation_id, variable);
                    }
                }
                DebugEventView::OperationFailed {
                    operation_id,
                    message,
                } => {
                    self.execution.on_operation_failed(operation_id);
                    self.watches.adopt_failure(operation_id, message);
                    self.error_message = Some(message.clone());
                }
                DebugEventView::Output { output, .. } => {
                    if !output.is_empty() {
                        self.output.push(output.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // -- 渲染 ---------------------------------------------------------------

    fn header(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .pl_3()
            .pr_2()
            .h(rems(DEBUG_SESSION_HEADER_HEIGHT_SPEC / 16.))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.debugger.gpui.toolWindowTitle")),
            )
            // 会话 / transport 两件事**分开显示**（本任务的重点之一）。
            .child(self.fact_chip(
                tr("lithe.debugger.gpui.transportLabel"),
                self.transport(),
                cx,
            ))
            .child(self.fact_chip(tr("lithe.debugger.gpui.sessionLabel"), self.session(), cx))
            .child(div().flex_1())
            .child(
                Button::new("debug-pane-hide")
                    .small()
                    .ghost()
                    .icon(idea_icon_mono(&idea::STOP_ICON, cx.theme().is_dark()))
                    .tooltip(tr("lithe.debugger.hideRunAndDebug"))
                    // 真源的「隐藏」（`GenericDebugView.swift:151-155`）：面板自己不知道底部工具窗
                    // 是否可见（`RunPane` / `TerminalPane` 同一口径），所以只报一个事件、
                    // 由外壳改 `bottom_visible`。**不画死按钮**。
                    .on_click(cx.listener(|_pane, _, _, cx| {
                        cx.emit(DebugPaneEvent::Hide);
                    })),
            )
            .into_any_element()
    }

    /// 一个"事实"小胶囊：`标签 · 缺失/在`。
    fn fact_chip(
        &self,
        label: SharedString,
        present: impl FactPresent,
        cx: &Context<Self>,
    ) -> AnyElement {
        let missing = present.missing_key();
        h_flex()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .rounded_md()
            .bg(cx.theme().muted)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(label)
            .child(div().text_color(cx.theme().danger).child(tr(missing)))
            .into_any_element()
    }

    fn toolbar(&self, cx: &Context<Self>) -> AnyElement {
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(rems(DEBUG_TOOLBAR_HEIGHT_SPEC / 16.))
            .px_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border);
        for action in DEBUG_TOOLBAR_ACTIONS {
            if let Some(control) = toolbar_control(action) {
                let enabled = self.execution.is_enabled(control);
                row = row.child(
                    Button::new(SharedString::from(format!(
                        "debug-toolbar-{}",
                        toolbar_action_id(action)
                    )))
                    .small()
                    .ghost()
                    .disabled(!enabled)
                    .tooltip(toolbar_tooltip(action))
                    .on_click(cx.listener(move |pane, _, _, cx| {
                        pane.press_toolbar_action(action, cx);
                    })),
                );
            } else {
                // 两个纯界面动作（查看断点 / 静音断点）没有 DAP 控制命令。
                let disabled = match action {
                    DebugToolbarAction::ViewBreakpoints => false,
                    DebugToolbarAction::MuteBreakpoints => pane_is_empty(&self.breakpoints),
                    _ => false,
                };
                row = row.child(
                    Button::new(SharedString::from(format!(
                        "debug-toolbar-{}",
                        toolbar_action_id(action)
                    )))
                    .small()
                    .ghost()
                    .disabled(disabled)
                    .tooltip(toolbar_tooltip(action))
                    .on_click(cx.listener(move |pane, _, _, cx| {
                        pane.press_toolbar_action(action, cx);
                    })),
                );
            }
            if debug_toolbar_has_separator_after(action) {
                row = row.child(
                    div()
                        .w(rems(1. / 16.))
                        .h(rems(DEBUG_TOOLBAR_ICON_SPEC / 16.))
                        .mx_1()
                        .bg(cx.theme().border),
                );
            }
        }
        // 状态胶囊（真源 `debugExecutionStatus`，`GenericDebugView.swift:483-520`）。
        row.child(div().flex_1())
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .px_2()
                    .rounded_full()
                    .bg(cx.theme().muted)
                    .text_xs()
                    .text_color(match self.execution.state() {
                        DebugExecutionState::Paused => cx.theme().warning,
                        DebugExecutionState::Terminated => cx.theme().muted_foreground,
                        _ => cx.theme().success,
                    })
                    .child(self.execution.status_text()),
            )
            .into_any_element()
    }

    fn press_toolbar_action(&mut self, action: DebugToolbarAction, cx: &mut Context<Self>) {
        match action {
            DebugToolbarAction::RestartOrStart => self.start_session(cx),
            DebugToolbarAction::ViewBreakpoints => {
                self.select_content(DebugContent::Debugger, cx);
            }
            DebugToolbarAction::MuteBreakpoints => self.toggle_mute_breakpoints(cx),
            _ => {
                let Some(control) = toolbar_control(action) else {
                    return;
                };
                let blocker = self.start_blocker();
                eprintln!(
                    "{DEBUG_DIAGNOSTIC_TAG} execute control={} status=skipped reason={}",
                    control.command(),
                    blocker
                        .map(DebugStartBlocker::as_str)
                        .unwrap_or("no-session")
                );
            }
        }
    }

    fn content_tabs(&self, cx: &Context<Self>) -> AnyElement {
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(rems(DEBUG_CONTENT_TABS_HEIGHT_SPEC / 16.))
            .px_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border);
        for content in [DebugContent::Debugger, DebugContent::Console] {
            let selected = self.content == content;
            row = row.child(
                div()
                    .id(SharedString::from(format!(
                        "debug-content-{}",
                        match content {
                            DebugContent::Debugger => "debugger",
                            DebugContent::Console => "console",
                        }
                    )))
                    .px_2()
                    .h(rems(25. / 16.))
                    .flex()
                    .items_center()
                    .rounded_md()
                    .cursor_pointer()
                    .when(selected, |this| {
                        this.bg(cx.theme().secondary)
                            .text_color(cx.theme().foreground)
                    })
                    .when(!selected, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .text_xs()
                    .child(tr(content.title_key()))
                    .on_click(cx.listener(move |pane, _, _, cx| {
                        pane.select_content(content, cx);
                    })),
            );
        }
        row.into_any_element()
    }

    /// 段标题（真源 `sectionHeader`，`GenericDebugView.swift:1368-1381`）：左标题 + 右计数。
    fn section_header(&self, title: SharedString, count: usize, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h(rems(DEBUG_SECTION_HEADER_HEIGHT_SPEC / 16.))
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(title)
            .child(div().flex_1())
            .child(SharedString::from(count.to_string()))
            .into_any_element()
    }

    /// 左列：线程 + **调用栈** + 断点。
    fn stack_column(&self, cx: &Context<Self>) -> AnyElement {
        let mut column = v_flex()
            .flex_1()
            .min_w(rems(DEBUG_STACK_COLUMN_MIN_WIDTH_SPEC / 16.))
            .min_h_0()
            .child(self.section_header(
                tr("lithe.debugger.stack"),
                self.inspection.stack_frames.len(),
                cx,
            ));
        if self.inspection.stack_frames.is_empty() {
            column = column.child(self.placeholder(tr("lithe.debugger.stackEmpty"), cx));
        } else {
            for frame in &self.inspection.stack_frames {
                let location = frame
                    .source_path
                    .as_ref()
                    .map(|path| {
                        let name = path.rsplit('/').next().unwrap_or(path);
                        format!("{name}:{}", frame.line)
                    })
                    .unwrap_or_default();
                column = column.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .when(frame.is_filtered, |this| this.opacity(0.58))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(SharedString::from(frame.name.clone())),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(SharedString::from(location)),
                        ),
                );
            }
        }
        // 断点列表（真源 `DebugBreakpointManagerView` 的源断点一段，`:1727-1807`）。
        let mut breakpoint_section = v_flex().w_full().child(self.section_header(
            tr("lithe.debugger.breakpoints"),
            self.breakpoints.total(),
            cx,
        ));
        if self.breakpoints.is_empty() {
            breakpoint_section = breakpoint_section
                .child(self.placeholder(tr("lithe.debugger.breakpointsEmpty"), cx));
        } else {
            let rows: Vec<(String, i64, bool)> =
                self.breakpoints
                    .files()
                    .flat_map(|path| {
                        self.breakpoints.file(path).iter().map(move |breakpoint| {
                            (path.clone(), breakpoint.line, breakpoint.enabled)
                        })
                    })
                    .collect();
            for (path, line, enabled) in rows {
                let line_text = line.to_string();
                breakpoint_section = breakpoint_section.child(
                    h_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .gap_2()
                        .text_xs()
                        .child(div().text_color(cx.theme().muted_foreground).child(tr_args(
                            "lithe.debugger.lineNumber",
                            &[("line", line_text.as_str())],
                        )))
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .text_color(if enabled {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .child(SharedString::from(path)),
                        ),
                );
            }
        }
        column.child(breakpoint_section).into_any_element()
    }

    /// 右列：**求值** + **变量树** + **监视**。
    fn data_column(&self, cx: &Context<Self>) -> AnyElement {
        let mut column = v_flex()
            .flex_1()
            .min_w(rems(DEBUG_DATA_COLUMN_MIN_WIDTH_SPEC / 16.))
            .min_h_0()
            // 求值行（真源 `evaluateRow`，`GenericDebugView.swift:1088-1124`）。
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .h(rems(DEBUG_EVALUATE_ROW_HEIGHT_SPEC / 16.))
                    .px_2()
                    .gap_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.variables"))
                    .child(div().flex_1())
                    // ⚠️ 这里**故意不画输入框**：真源的求值行是 TextField +
                    // 加监视 + 求值三个控件，其中"求值"必须 `state == .paused`
                    // （`:1119`）；没有 transport ⇒ 永远不可能 paused ⇒ 画一个永远敲不动、
                    // 点了没反应的输入框就是假装。空态里已把原因写清。
                    .child(tr("lithe.debugger.evaluating")),
            )
            .child(self.section_header(
                tr("lithe.debugger.variables"),
                self.inspection.variables.len(),
                cx,
            ));
        if self.inspection.variables.is_empty() {
            column = column.child(self.placeholder(tr("lithe.debugger.variablesEmpty"), cx));
        } else {
            for row in self.tree.rows(&self.inspection.variables) {
                column = column.child(self.variable_row(&row, cx));
            }
        }
        column =
            column.child(self.section_header(tr("lithe.debugger.watch"), self.watches.len(), cx));
        if self.watches.is_empty() {
            column = column.child(self.placeholder(tr("lithe.debugger.watchEmpty"), cx));
        } else {
            for watch in self.watches.items() {
                let detail = match watch.state {
                    WatchState::Ready => watch.value.clone().unwrap_or_default(),
                    WatchState::Failed => watch.error.clone().unwrap_or_default(),
                    WatchState::Evaluating => tr("lithe.debugger.evaluating").to_string(),
                    WatchState::NotEvaluated => tr("lithe.debugger.notEvaluated").to_string(),
                };
                column = column.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(SharedString::from(watch.expression.clone())),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(match watch.state {
                                    WatchState::Failed => cx.theme().danger,
                                    WatchState::Ready => cx.theme().success,
                                    _ => cx.theme().muted_foreground,
                                })
                                .child(SharedString::from(detail)),
                        ),
                );
            }
        }
        // 步进过滤器状态（真源 `javaSteppingFilters` 那一块；这里是**真的**从 Core 读来的）。
        column.child(self.filters_block(cx)).into_any_element()
    }

    fn variable_row(&self, row: &DebugVariableRow, cx: &Context<Self>) -> AnyElement {
        let indent = 10. + row.depth as f32 * DEBUG_VARIABLE_INDENT_SPEC;
        let name = format!(
            "{} = {}",
            row.variable.name,
            if row.variable.value.is_empty() {
                tr("lithe.debugger.empty").to_string()
            } else {
                row.variable.value.clone()
            }
        );
        h_flex()
            .w_full()
            .pr_2()
            .py_1()
            // 真源 `padding(.leading, 10 + depth * 14)`（`GenericDebugView.swift:853`）。
            .pl(rems(indent / 16.))
            .gap_2()
            .text_xs()
            .child(
                div()
                    .text_color(cx.theme().foreground)
                    .child(SharedString::from(name)),
            )
            .when_some(row.variable.type_name.clone(), |this, type_name| {
                this.child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(type_name)),
                )
            })
            .into_any_element()
    }

    /// 步进过滤器那一块：是**真的**从 Core 的 `debug.steppingFilters` 读来的。
    fn filters_block(&self, cx: &Context<Self>) -> AnyElement {
        let mut block = v_flex().w_full().child(self.section_header(
            tr("lithe.debugger.gpui.steppingFilters"),
            match &self.filters {
                SteppingFiltersState::Ready(filters) => filters.class_name_filters.len(),
                _ => 0,
            },
            cx,
        ));
        match &self.filters {
            SteppingFiltersState::Idle => {
                block = block.child(self.placeholder(tr("lithe.debugger.gpui.filtersIdle"), cx))
            }
            SteppingFiltersState::Loading => {
                block = block.child(self.placeholder(tr("lithe.debugger.gpui.filtersLoading"), cx))
            }
            SteppingFiltersState::Failed(error) => {
                block = block.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(tr("lithe.debugger.gpui.filtersFailed"))
                        .child(SharedString::from(error.clone())),
                )
            }
            SteppingFiltersState::Ready(filters) => {
                block = block.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(format!(
                            "{}: {}",
                            tr("lithe.debugger.gpui.filterClassPatterns"),
                            filters
                                .class_name_filters
                                .iter()
                                .take(4)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))),
                );
                for toggle in [
                    SteppingFilterToggle::SkipSynthetics,
                    SteppingFilterToggle::SkipConstructors,
                    SteppingFilterToggle::SkipStaticInitializers,
                    SteppingFilterToggle::HideFilteredStackFrames,
                ] {
                    block = block.child(
                        h_flex()
                            .w_full()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(tr(toggle.label_key()))
                            .child(div().flex_1())
                            .child(SharedString::from(
                                if toggle.value(filters) { "✓" } else { "—" }.to_string(),
                            )),
                    );
                }
            }
        }
        block.into_any_element()
    }

    /// 空态（真源 `emptyState`，`GenericDebugView.swift:1317-1344`）：
    /// 图标 + 标题 + 说明 + 三个按钮。**两个真的按不动** —— 原因写在正文里。
    fn empty_state(&self, cx: &Context<Self>) -> AnyElement {
        let blocker = self.start_blocker();
        let mut column = v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .px_4()
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.debugger.gpui.emptyTitle")),
            )
            .child(
                div()
                    .max_w(rems(520. / 16.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(match blocker {
                        Some(blocker) => tr(blocker.text_key()),
                        // 两个事实都在时**当前运行期不可达**（见模块文档）；真到那一步就退回
                        // 状态词而不是编一句新的：宁可少说，也不假装知道会话处于什么阶段。
                        None => tr("lithe.debugger.statusIdle"),
                    }),
            );
        // 两个"真的做不到"的原因分开写：H10 与 H11 是两件事，不合并成一句含糊的话。
        column = column
            .child(
                div()
                    .max_w(rems(520. / 16.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.gpui.h10Note")),
            )
            .child(
                div()
                    .max_w(rems(520. / 16.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.gpui.h11Note")),
            );
        let can_start = blocker.is_none();
        column = column.child(
            Button::new("debug-empty-start")
                .small()
                // 真源是 `.borderedProminent`（`:1332`）；本侧**禁用**它并写明原因，
                // 而不是让它看起来能点（见 `start_session` 的三条理由）。
                .disabled(!can_start)
                .tooltip(match blocker {
                    Some(blocker) => tr(blocker.text_key()),
                    None => tr("lithe.debugger.start"),
                })
                .label(tr("lithe.debugger.start"))
                .on_click(cx.listener(|pane, _, _, cx| pane.start_session(cx))),
        );
        column = column.child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr_args(
                    "lithe.debugger.gpui.breakpointSummary",
                    &[("count", self.breakpoints.total().to_string().as_str())],
                )),
        );
        column.into_any_element()
    }

    fn console(&self, cx: &Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.adapterOutputAppearsHere")),
            )
            .children(
                self.output
                    .iter()
                    .map(|line| {
                        div()
                            .px_2()
                            .text_xs()
                            .child(SharedString::from(line.clone()))
                    })
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }

    fn placeholder(&self, text: SharedString, cx: &Context<Self>) -> AnyElement {
        div()
            .w_full()
            .px_2()
            .py_2()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(text)
            .into_any_element()
    }

    fn debugger_content(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .size_full()
            .min_h_0()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(self.stack_column(cx)),
            )
            .child(div().w(rems(1. / 16.)).h_full().bg(cx.theme().border))
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(self.data_column(cx)),
            )
            .into_any_element()
    }
}

/// 一个"事实"（transport / session）在界面上的"缺失"文案键。
trait FactPresent {
    fn missing_key(&self) -> &'static str;
}

impl FactPresent for DebugTransportFacts {
    fn missing_key(&self) -> &'static str {
        match self {
            Self::Absent => "lithe.debugger.gpui.absent",
            Self::Present => "lithe.debugger.gpui.present",
        }
    }
}

impl FactPresent for DebugSessionFacts {
    fn missing_key(&self) -> &'static str {
        match self {
            Self::Absent => "lithe.debugger.gpui.absent",
            Self::Present => "lithe.debugger.gpui.present",
        }
    }
}

impl SteppingFilterToggle {
    /// 五条开关的文案键（catalog 既有 `debugger.*` 键里最贴的那几条）。
    fn label_key(self) -> &'static str {
        match self {
            Self::SkipSynthetics => "lithe.debugger.gpui.skipSynthetics",
            Self::SkipConstructors => "lithe.debugger.gpui.skipConstructors",
            Self::SkipStaticInitializers => "lithe.debugger.gpui.skipStaticInitializers",
            Self::HideFilteredStackFrames => "lithe.debugger.gpui.hideFilteredStackFrames",
        }
    }
}

fn pane_is_empty(store: &DebugBreakpointStore) -> bool {
    store.is_empty()
}

/// 工具栏某一档对应的 DAP 控制命令（`None` = 纯界面动作）。
fn toolbar_control(action: DebugToolbarAction) -> Option<DebugControl> {
    match action {
        DebugToolbarAction::Stop => Some(DebugControl::Terminate),
        DebugToolbarAction::Resume => Some(DebugControl::Continue),
        DebugToolbarAction::Pause => Some(DebugControl::Pause),
        DebugToolbarAction::StepOver => Some(DebugControl::StepOver),
        DebugToolbarAction::StepInto => Some(DebugControl::StepIn),
        DebugToolbarAction::StepOut => Some(DebugControl::StepOut),
        DebugToolbarAction::RestartOrStart => Some(DebugControl::Restart),
        DebugToolbarAction::ViewBreakpoints | DebugToolbarAction::MuteBreakpoints => None,
    }
}

/// 工具栏档位的稳定 id（`ElementId` 用**业务身份**，不用序号）。
fn toolbar_action_id(action: DebugToolbarAction) -> &'static str {
    match action {
        DebugToolbarAction::RestartOrStart => "restart-or-start",
        DebugToolbarAction::Stop => "stop",
        DebugToolbarAction::Resume => "resume",
        DebugToolbarAction::Pause => "pause",
        DebugToolbarAction::StepOver => "step-over",
        DebugToolbarAction::StepInto => "step-into",
        DebugToolbarAction::StepOut => "step-out",
        DebugToolbarAction::ViewBreakpoints => "view-breakpoints",
        DebugToolbarAction::MuteBreakpoints => "mute-breakpoints",
    }
}

/// 工具栏档位的悬浮说明。
///
/// 真源是 `debugToolbarActionHelp`（`GenericDebugView.swift:431-452`）+ 快捷键后缀；
/// catalog 里对得上的键逐条复用，对不上的（Smart Step Into / Reveal…）不硬编。
fn toolbar_tooltip(action: DebugToolbarAction) -> SharedString {
    tr(match action {
        DebugToolbarAction::RestartOrStart => "lithe.debugger.start",
        DebugToolbarAction::Stop => "lithe.debugger.stop",
        DebugToolbarAction::Resume => "lithe.debugger.continueDebugging",
        DebugToolbarAction::Pause => "lithe.debugger.pauseDebugging",
        DebugToolbarAction::StepOver => "lithe.debugger.stepOver",
        DebugToolbarAction::StepInto => "lithe.debugger.stepInto",
        DebugToolbarAction::StepOut => "lithe.debugger.stepOut",
        DebugToolbarAction::ViewBreakpoints => "lithe.debugger.breakpoints",
        DebugToolbarAction::MuteBreakpoints => "lithe.debugger.breakpoints",
    })
}

impl Render for DebugPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .bg(cx.theme().background)
            .child(self.header(cx))
            .child(self.toolbar(cx))
            .child(if self.has_content() {
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.content_tabs(cx))
                    .child(div().flex_1().min_h_0().child(match self.content {
                        DebugContent::Debugger => self.debugger_content(cx),
                        DebugContent::Console => self.console(cx),
                    }))
            } else {
                v_flex().flex_1().min_h_0().child(self.empty_state(cx))
            })
            .into_any_element()
    }
}

/// 把 `Option<String>` 按 Core 的"缺省即不写"口径塞进 map（`types.rs:173-183`）。
fn insert_optional(map: &mut serde_json::Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // -----------------------------------------------------------------------
    // 断点模型 + `debug.setBreakpoints` 的请求构造
    // -----------------------------------------------------------------------

    /// 切一次加一次、切两次删掉；文件键与行号都保持有序（Core 侧还会再排一次，
    /// 但宿主自己的顺序必须确定 —— 界面与诊断都读它）。
    #[test]
    fn toggle_adds_then_removes_a_line_and_keeps_lines_sorted() {
        let mut store = DebugBreakpointStore::default();
        assert_eq!(
            store.toggle("src/Main.java", 12),
            DebugBreakpointChange::File("src/Main.java".to_string())
        );
        store.toggle("src/Main.java", 4);
        store.toggle("src/Main.java", 9);

        let lines: Vec<i64> = store
            .file("src/Main.java")
            .iter()
            .map(|breakpoint| breakpoint.line)
            .collect();
        assert_eq!(lines, vec![4, 9, 12], "行号必须有序");
        assert_eq!(store.total(), 3);

        // 再切一次 9 ⇒ 删掉，且不留下空文件键。
        store.toggle("src/Main.java", 9);
        let lines: Vec<i64> = store
            .file("src/Main.java")
            .iter()
            .map(|breakpoint| breakpoint.line)
            .collect();
        assert_eq!(lines, vec![4, 12]);

        // 非法行号是**不变**，不是 panic、也不是悄悄加一条行号 0。
        assert_eq!(
            store.toggle("src/Main.java", 0),
            DebugBreakpointChange::Unchanged
        );
        assert_eq!(store.total(), 2);
    }

    /// `debug.setBreakpoints` 是"**替换**某个文件的全部断点"语义
    /// （`rust/lithe-core/src/debug/types.rs:190-198` 的 `SetBreakpointsRequest`），
    /// 所以载荷必须带**整份**集合、字段名逐条对上 Core 的 camelCase。
    #[test]
    fn set_breakpoints_payload_is_a_whole_file_replacement() {
        let mut store = DebugBreakpointStore::default();
        store.toggle("src/Main.java", 7);
        store.toggle("src/Other.java", 3);
        store.set_enabled("src/Other.java", 3, false);

        let payload = store.set_breakpoints_payload("s-1", "src/Other.java");
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["sourcePath"], json!("src/Other.java"));
        let items = payload["breakpoints"].as_array().expect("必须是数组");
        assert_eq!(items.len(), 1, "只发这一个文件的断点：{payload}");
        assert_eq!(items[0]["line"], json!(3));
        assert_eq!(items[0]["enabled"], json!(false));
        // `column` 缺省不写（`SourceBreakpoint.column` 是 `Option`，`types.rs:174-175`）。
        assert!(items[0].get("column").is_none());
        // 空的 `condition` / `hitCondition` / `logMessage` 也不写（Core 三者的缺省都是 `None`）。
        assert!(items[0].get("condition").is_none());

        // 整份替换的另一半：这个文件没有断点时载荷是**空数组**（DAP 的"清空该源"语义），
        // 而不是"整条不发"。
        let empty = store.set_breakpoints_payload("s-1", "src/Gone.java");
        assert_eq!(empty["breakpoints"], json!([]));
    }

    /// `replace_file` 是 `relocate` 的采纳口：排序 + 去重 + 空集合删键。
    #[test]
    fn replace_file_sorts_dedups_and_drops_empty_files() {
        let mut store = DebugBreakpointStore::default();
        let marks = vec![
            DebugBreakpoint::line(9),
            DebugBreakpoint::line(2),
            DebugBreakpoint::line(9),
        ];
        store.replace_file("a.java", marks);
        let lines: Vec<i64> = store.file("a.java").iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![2, 9], "排序 + 按 (行, 列) 去重");

        store.replace_file("a.java", Vec::new());
        assert!(store.file("a.java").is_empty());
        assert_eq!(store.total(), 0, "空集合要真的把文件键去掉");
    }

    // -----------------------------------------------------------------------
    // "能不能启动"：纯函数 + 反证
    // -----------------------------------------------------------------------

    /// 四格真值表。**关键是第三条**：有会话、没有 transport 时**仍然不能启动** ——
    /// 把它写成"可启动"就是本任务最想防住的那种假装（界面上会变成一个点了没反应的按钮）。
    #[test]
    fn a_missing_transport_is_never_startable_even_with_a_session() {
        assert_eq!(
            start_blocker(DebugTransportFacts::Present, DebugSessionFacts::Present),
            None,
            "两个事实都在才是可启动"
        );
        assert_eq!(
            start_blocker(DebugTransportFacts::Absent, DebugSessionFacts::Present),
            Some(DebugStartBlocker::NoTransport),
            "反证：没有 transport 就不可启动，哪怕会话存在"
        );
        assert_eq!(
            start_blocker(DebugTransportFacts::Present, DebugSessionFacts::Absent),
            Some(DebugStartBlocker::NoSession)
        );
        assert_eq!(
            start_blocker(DebugTransportFacts::Absent, DebugSessionFacts::Absent),
            Some(DebugStartBlocker::NoTransportAndNoSession),
            "当前运行期的真实状态"
        );
    }

    /// 三个分支的文案键互不相同，且都在 WIRED 表里（`shared/src/i18n.rs` 那条测试会兜住
    /// "键存在且中英不同"，这里只钉住"没有两条用同一句话"）。
    #[test]
    fn every_start_blocker_has_its_own_message_key() {
        let keys = [
            DebugStartBlocker::NoTransport.text_key(),
            DebugStartBlocker::NoSession.text_key(),
            DebugStartBlocker::NoTransportAndNoSession.text_key(),
        ];
        let unique: BTreeSet<&str> = keys.iter().copied().collect();
        assert_eq!(unique.len(), 3, "三个分支必须各有一句话：{keys:?}");
    }

    /// 面板的默认状态就是"两个事实都没有"，而它的 `start_blocker()` 必须与纯函数一致。
    #[test]
    fn a_fresh_pane_reports_both_facts_missing() {
        let pane = DebugPane::new();
        assert_eq!(pane.transport(), DebugTransportFacts::Absent);
        assert_eq!(pane.session(), DebugSessionFacts::Absent);
        assert_eq!(
            pane.start_blocker(),
            Some(DebugStartBlocker::NoTransportAndNoSession)
        );
        assert!(!pane.has_content(), "没有会话 / 输出 / 错误 ⇒ 画空态");
    }

    /// 派发闸门：没有会话就**一个字节都不发**（`debug.setBreakpoints` 走 `with_session`，
    /// `rust/lithe-core/src/debug/engine.rs:197`）。
    #[test]
    fn the_dispatch_gate_stays_closed_without_a_session() {
        let gate = DebugDispatchGate::disconnected();
        assert!(gate.session_id().is_none());
        assert!(!gate.can_dispatch());
    }

    // -----------------------------------------------------------------------
    // `debug.relocateBreakpoints`：编辑 → 请求 → 结果
    // -----------------------------------------------------------------------

    /// 在断点所在行**之前**插入一行 ⇒ 请求里的编辑区间必须落在插入点、
    /// 而 Core 回来的行号要 +1。
    ///
    /// 反证两件事：(a) 若 `derive_source_edit` 把区间算成整篇（前缀/后缀不取），
    /// 这个断言会红；(b) 若采纳结果时不做行号复位（例如忽略 Core 的返回值），
    /// 第二段的断言会红。
    #[test]
    fn relocating_a_breakpoint_across_an_inserted_line_moves_it_down() {
        let before = "class Main {\n    void run() {}\n}\n";
        let after = "class Main {\n    int x = 1;\n    void run() {}\n}\n";
        let edit = derive_source_edit(before, after).expect("两段文本不同 ⇒ 必须有一个 edit");
        // **不变量**：把 edit 应用回 before 必须逐字得到 after —— 这才是这个函数的契约，
        // 而"具体切在哪一个字符"是实现细节（最长公共前后缀在缩进上会有多个等价切点）。
        assert_eq!(apply_source_edit_for_test(before, &edit), after);
        // 反证"整篇替换"那种退化：前后缀真的被保留下来了。
        assert!(edit.start_utf16_offset > 0, "前半段必须活下来：{edit:?}");
        assert!(
            edit.end_utf16_offset < before.encode_utf16().count(),
            "后半段必须活下来：{edit:?}"
        );

        let breakpoints = vec![DebugBreakpoint::line(2)];
        let payload = relocate_breakpoints_payload(before, &edit, &breakpoints);
        assert_eq!(payload["source"], json!(before), "必须给**编辑前**的全文");
        assert_eq!(
            payload["edit"]["startUtf16Offset"],
            json!(edit.start_utf16_offset)
        );
        assert_eq!(
            payload["edit"]["endUtf16Offset"],
            json!(edit.end_utf16_offset)
        );
        assert_eq!(payload["edit"]["replacement"], json!(edit.replacement));
        assert_eq!(payload["breakpoints"][0]["line"], json!(2));

        // Core 的响应形状：`RelocateBreakpointsResult { breakpoints: [SourceBreakpoint] }`
        // （`rust/lithe-core/src/debug/breakpoint_relocation.rs:34-39`）。
        let response = json!({
            "breakpoints": [{ "line": 3, "enabled": true }]
        });
        let relocated = parse_relocate_result(&response).expect("形状要认得");
        assert_eq!(relocated.len(), 1);
        assert_eq!(relocated[0].line, 3, "断点跟着代码走");

        let mut store = DebugBreakpointStore::default();
        store.toggle("src/Main.java", 2);
        store.replace_file("src/Main.java", relocated);
        assert_eq!(store.file("src/Main.java")[0].line, 3);
    }

    /// 删除光标前的整段文字 ⇒ 断点行号上移；UTF-16 边界（表情）不能被切碎。
    #[test]
    fn derive_source_edit_handles_non_bmp_text_and_deletions() {
        // 前缀 / 后缀都取到最长 ⇒ 区间只覆盖真正变的那一段。
        let before = "a🚀b\nc\n";
        let after = "a🚀b\n\nc\n";
        let edit = derive_source_edit(before, after).expect("插入了一个换行");
        assert_eq!(edit.replacement, "\n");
        // 🚀 占两个 UTF-16 code unit，所以插入点在 1 + 2 + 1 + 1 = 5。
        assert_eq!(edit.start_utf16_offset, 5);
        assert_eq!(edit.end_utf16_offset, 5);
        // 替换文本能还原成合法字符串（说明没切在代理对中间）。
        assert_eq!(edit.replacement, "\n");

        // 两段相同 ⇒ 没有 edit 可发（调用方据此跳过 Core 调用）。
        assert!(derive_source_edit("same", "same").is_none());

        // 整段删除：区间覆盖全部旧文本。
        let edit = derive_source_edit("abc", "").expect("整段删掉");
        assert_eq!(edit.start_utf16_offset, 0);
        assert_eq!(edit.end_utf16_offset, 3);
        assert_eq!(edit.replacement, "");
    }

    /// 读不懂的 `relocate` 响应**不能**被当成"这个文件没有断点了" ——
    /// 那会让用户的断点因为一次契约漂移而消失。
    #[test]
    fn an_unrecognized_relocate_response_is_rejected_instead_of_clearing() {
        assert!(parse_relocate_result(&json!({ "nope": [] })).is_none());
        assert!(parse_relocate_result(&json!({ "breakpoints": "x" })).is_none());
        // 合法但为空 = 真的没有断点（与上一条要分得开）。
        assert_eq!(
            parse_relocate_result(&json!({ "breakpoints": [] })),
            Some(Vec::new())
        );
    }

    /// `debug_key`：根内相对（`/` 分隔）、根外绝对、以及"路径就是根本身"不能产出空键。
    #[test]
    fn debug_key_relativizes_inside_the_root_and_keeps_outsiders_absolute() {
        let root = Path::new("/work/project");
        assert_eq!(
            debug_key(root, Path::new("/work/project/src/Main.java")),
            "src/Main.java"
        );
        assert_eq!(
            debug_key(root, Path::new("/other/Thing.java")),
            "/other/Thing.java"
        );
        // 根本身：绝不产出空键（空键会把所有"根本身"的调用混成一条）。
        assert_eq!(debug_key(root, root), "/work/project");
    }

    // -----------------------------------------------------------------------
    // DAP 字节归约（`debug.receive` / `debug.inspect` / `debug.execute` 的合成响应）
    // -----------------------------------------------------------------------

    /// 一份合成的 `DebugSessionUpdate`（`rust/lithe-core/src/debug/types.rs:489-498`）：
    /// 归约成宿主投影之后，调用栈 / 变量 / 监视三块各自拿得到东西。
    #[test]
    fn a_synthetic_dap_update_projects_into_the_inspector_blocks() {
        let update = parse_session_update(&json!({
            "sessionId": "s-1",
            "state": "paused",
            "outboundFrames": ["ZnJhbWU=", "bW9yZQ=="],
            "events": [
                { "type": "stopped", "reason": "breakpoint", "threadId": 3 },
                {
                    "type": "operationCompleted",
                    "operationId": "op-1",
                    "result": {
                        "kind": "stackTrace",
                        "stackFrames": [
                            { "id": 11, "name": "Main.run", "sourcePath": "src/Main.java",
                              "line": 7, "column": 5, "isFiltered": false },
                            { "id": 12, "name": "java.lang.Thread.run", "sourcePath": null,
                              "line": 0, "column": 0, "isFiltered": true }
                        ]
                    }
                },
                {
                    "type": "operationCompleted",
                    "operationId": "op-2",
                    "result": {
                        "kind": "variables",
                        "variables": [
                            { "name": "this", "value": "Main@1", "type": "Main",
                              "variablesReference": 42, "namedVariables": 3 },
                            { "name": "count", "value": "7", "type": "int",
                              "variablesReference": 0 }
                        ]
                    }
                }
            ]
        }));

        assert_eq!(update.state, Some(DebugExecutionState::Paused));
        assert_eq!(update.outbound_frame_count, 2, "只留条数，不搬字节");
        assert_eq!(update.events.len(), 3);

        let mut inspection = DebugInspection::default();
        for event in &update.events {
            if let DebugEventView::OperationCompleted { result, .. } = event {
                inspection.adopt(result);
            }
        }
        assert_eq!(inspection.stack_frames.len(), 2);
        assert_eq!(inspection.stack_frames[0].line, 7);
        assert!(inspection.stack_frames[1].is_filtered, "被过滤的帧要标出来");
        assert_eq!(inspection.variables.len(), 2);
        assert_eq!(inspection.variables[0].named_variables, 3);
        assert!(inspection.variables[0].is_expandable());
        assert!(!inspection.variables[1].is_expandable());

        // 变量树投影：当前只摊一层（更深的一层要再发 `debug.inspect{kind:variables}`）。
        let tree = DebugVariableTree::default();
        let rows = tree.rows(&inspection.variables);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.depth == 0));
    }

    /// 认不出的事件类型**不静默丢**（Core 契约变了要在界面上留痕）。
    #[test]
    fn an_unknown_dap_event_keeps_its_kind() {
        let update = parse_session_update(&json!({
            "state": "running",
            "events": [{ "type": "somethingNew" }]
        }));
        assert_eq!(
            update.events,
            vec![DebugEventView::Unknown {
                kind: "somethingNew".to_string()
            }]
        );
        // Core 的状态名 → 宿主四态（`types.rs:6-19` 有九档，宿主归四档）。
        assert_eq!(update.state, Some(DebugExecutionState::Running));
        assert_eq!(
            parse_execution_state("initializing"),
            Some(DebugExecutionState::Running)
        );
        assert_eq!(
            parse_execution_state("terminated"),
            Some(DebugExecutionState::Terminated)
        );
        assert_eq!(parse_execution_state("no-such-state"), None);
    }

    // -----------------------------------------------------------------------
    // 监视：按 `operationId` 认领（不能张冠李戴）
    // -----------------------------------------------------------------------

    /// 两条监视同时求值，第二条的响应先回来 ⇒ 值必须落在**它自己**那条上；
    /// 一个对不上的 `operationId` 一律不采纳。
    #[test]
    fn watch_evaluation_is_claimed_by_operation_id_not_by_order() {
        let mut watches = DebugWatches::default();
        let first = watches.add("a + b").expect("非空表达式要能加");
        let second = watches.add(" c ".trim()).expect("去空白后非空");

        assert!(watches.begin_evaluation(first, "op-1"));
        assert!(watches.begin_evaluation(second, "op-2"));

        // 反证：op-9 不属于任何在飞请求 ⇒ 不采纳。
        assert!(!watches.adopt_evaluation(
            "op-9",
            &DebugVariableView {
                name: "x".to_string(),
                value: "1".to_string(),
                type_name: None,
                evaluate_name: None,
                variables_reference: 0,
                named_variables: 0,
                indexed_variables: 0,
            }
        ));

        // op-2 回来了：值落在第二条上，第一条**仍然在飞**（不能被别人的回包清掉）。
        assert!(watches.adopt_evaluation(
            "op-2",
            &DebugVariableView {
                name: "c".to_string(),
                value: "42".to_string(),
                type_name: Some("int".to_string()),
                evaluate_name: None,
                variables_reference: 0,
                named_variables: 0,
                indexed_variables: 0,
            }
        ));
        assert_eq!(watches.items()[0].state, WatchState::Evaluating);
        assert_eq!(watches.items()[1].state, WatchState::Ready);
        assert_eq!(watches.items()[1].value.as_deref(), Some("42"));

        // 同一表达式加两次只留一条（真源 `addWatch` 的语义）。
        assert_eq!(watches.add("a + b"), None);
        assert_eq!(watches.len(), 2);

        // 失败也按同一个键认领；同一条键**只能被认领一次**（第二次必须返回 false，
        // 否则一次失败会反复改写同一条监视）。
        assert!(watches.begin_evaluation(first, "op-3"));
        assert!(watches.adopt_failure("op-3", "cannot evaluate"));
        assert_eq!(watches.items()[0].state, WatchState::Failed);
        assert_eq!(watches.items()[0].error.as_deref(), Some("cannot evaluate"));
        assert!(!watches.adopt_failure("op-3", "again"));
    }

    /// 把一次 [`DebugSourceEdit`] 应用回旧文本（测试专用），用来钉住
    /// [`derive_source_edit`] 的契约：应用之后必须逐字等于新文本。
    fn apply_source_edit_for_test(before: &str, edit: &DebugSourceEdit) -> String {
        let units: Vec<u16> = before.encode_utf16().collect();
        let mut out: Vec<u16> = Vec::new();
        out.extend_from_slice(&units[..edit.start_utf16_offset]);
        out.extend(edit.replacement.encode_utf16());
        out.extend_from_slice(&units[edit.end_utf16_offset..]);
        String::from_utf16_lossy(&out)
    }

    // -----------------------------------------------------------------------
    // 执行状态机（四态 + 分代）
    // -----------------------------------------------------------------------

    /// 工具栏的禁用判据照真源 `isDebugToolbarActionDisabled`
    /// （`macos/Sources/Lithe/Views/Debug/GenericDebugView.swift:408-429`）：
    /// 继续只在 paused、暂停只在 running、单步要 paused **且有一个具体的停止线程**。
    #[test]
    fn the_execution_machine_gates_every_control_like_the_macos_source() {
        let mut execution = DebugExecution::default();
        assert_eq!(execution.state(), DebugExecutionState::Idle);
        assert!(!execution.is_enabled(DebugControl::Continue));
        assert!(!execution.is_enabled(DebugControl::Pause));
        assert!(!execution.is_enabled(DebugControl::StepOver));

        // 跑起来：只有"暂停"能点。
        execution.on_continued();
        assert!(execution.is_enabled(DebugControl::Pause));
        assert!(!execution.is_enabled(DebugControl::Continue));

        // 停在断点上、且有线程 3：继续与单步都能点。
        execution.on_stopped("breakpoint", Some(3));
        assert!(execution.is_enabled(DebugControl::Continue));
        assert!(execution.is_enabled(DebugControl::StepIn));
        assert!(!execution.is_enabled(DebugControl::Pause));

        // **反证**：暂停了但没有具体线程 ⇒ 单步必须禁用（真源 `:418-422` 的注释就是这条：
        // DAP 的 step 请求要一个具体线程，没有线程的单步是一个 no-op 请求）。
        let mut no_thread = DebugExecution::default();
        no_thread.on_stopped("pause", None);
        assert!(!no_thread.is_enabled(DebugControl::StepOver));
        assert!(no_thread.is_enabled(DebugControl::Continue));

        // 在飞请求期间所有控制都禁用（真源 `isExecutionRequestPending`）。
        let generation = execution.begin("op-1", DebugControl::Continue);
        assert!(execution.is_request_pending());
        assert!(!execution.is_enabled(DebugControl::StepOver));
        assert!(execution.accepts(generation));

        // 分代：旧代次一律不采纳。
        execution.on_continued();
        let next = execution.begin("op-2", DebugControl::Pause);
        assert!(next > generation);
        assert!(!execution.accepts(generation), "旧代次的结果必须被丢掉");
        assert!(execution.accepts(next));

        // 失败回包只认领自己那一条。
        assert!(!execution.on_operation_failed("op-1"));
        assert!(execution.on_operation_failed("op-2"));
        assert!(!execution.is_request_pending());

        // 终态。
        execution.on_terminated();
        assert_eq!(execution.state(), DebugExecutionState::Terminated);
        assert!(!execution.is_enabled(DebugControl::Restart));
        assert_eq!(execution.stopped_reason(), None);
    }

    /// 状态胶囊的文案：暂停且有**有词的原因**时是 `已暂停 · <词>`，其余就是状态词本身。
    #[test]
    fn the_status_capsule_only_names_stop_reasons_the_catalog_has_words_for() {
        let mut execution = DebugExecution::default();
        execution.on_stopped("breakpoint", Some(1));
        assert_eq!(
            stop_reason_label("breakpoint"),
            Some("lithe.debugger.breakpoints")
        );
        assert_eq!(stop_reason_label("PAUSE"), Some("lithe.debugger.paused"));
        // 没有现成词的取值不编造（真源有 9 档 stopReason，catalog 里没有 `debugger.stopReason.*`）。
        assert_eq!(stop_reason_label("data breakpoint"), None);
        assert_eq!(stop_reason_label("entry"), None);
    }

    /// 工具栏九档的顺序与两条分隔线的位置**逐条**照 `DebugToolbarPresentation.swift:22-34`。
    #[test]
    fn the_toolbar_order_and_separators_match_the_macos_source() {
        assert_eq!(DEBUG_TOOLBAR_ACTIONS.len(), 9);
        let ids: Vec<&str> = DEBUG_TOOLBAR_ACTIONS
            .iter()
            .copied()
            .map(toolbar_action_id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "restart-or-start",
                "stop",
                "resume",
                "pause",
                "step-over",
                "step-into",
                "step-out",
                "view-breakpoints",
                "mute-breakpoints",
            ]
        );
        // `separatorsAfter: [.stop, .stepOut]`（同文件 `:34`）。
        let separated: Vec<&str> = DEBUG_TOOLBAR_ACTIONS
            .iter()
            .copied()
            .filter(|action| debug_toolbar_has_separator_after(*action))
            .map(toolbar_action_id)
            .collect();
        assert_eq!(separated, vec!["stop", "step-out"]);

        // 两个纯界面动作**没有** DAP 控制命令（它们不该发 `debug.execute`）。
        assert!(toolbar_control(DebugToolbarAction::ViewBreakpoints).is_none());
        assert!(toolbar_control(DebugToolbarAction::MuteBreakpoints).is_none());
        // 单步三档映射到 DAP 的 `next` / `stepIn` / `stepOut`（`types.rs:324-337`）。
        assert_eq!(
            toolbar_control(DebugToolbarAction::StepOver).map(DebugControl::command),
            Some("next")
        );
        assert_eq!(
            toolbar_control(DebugToolbarAction::StepInto).map(DebugControl::command),
            Some("stepIn")
        );
        // 工具栏度量逐个照真源（`DebugToolbarPresentation.swift:38-40`）。
        assert_eq!(DEBUG_TOOLBAR_HEIGHT_SPEC, 36.);
        assert_eq!(DEBUG_SESSION_HEADER_HEIGHT_SPEC, 34.);
        assert_eq!(DEBUG_TOOLBAR_ICON_SPEC, 18.);
    }

    /// `debug.execute` 的载荷字段名逐条对上 Core 的 `ExecuteRequest`
    /// （`rust/lithe-core/src/debug/types.rs:340-353`）。
    #[test]
    fn execute_payload_matches_the_core_execute_request_shape() {
        let payload = DebugControl::StepOver.payload("s-1", "op-7", Some(3));
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["operationId"], json!("op-7"));
        assert_eq!(payload["command"], json!("next"));
        assert_eq!(payload["threadId"], json!(3));
        assert_eq!(payload["singleThread"], json!(false));

        // 没有线程时不写 `threadId`（`Option` 的缺省就是 `None`）。
        let payload = DebugControl::Continue.payload("s-1", "op-8", None);
        assert!(payload.get("threadId").is_none());
        assert_eq!(payload["command"], json!("continue"));
    }

    // -----------------------------------------------------------------------
    // 步进过滤器（本切片真的调到的 Core 命令之一）
    // -----------------------------------------------------------------------

    /// 请求**故意不传** `filters`（`types.rs:156-157` 的 `#[serde(default)] Option<..>`：
    /// 缺省 = 要适配器默认档）；响应按 camelCase 读回来。
    #[test]
    fn stepping_filters_payload_asks_for_the_adapter_defaults() {
        let payload = stepping_filters_payload("java");
        assert_eq!(payload["adapterId"], json!("java"));
        assert!(
            payload.get("filters").is_none(),
            "传一份宿主自造的默认值就是把 Core 的默认档复制一份：{payload}"
        );

        let filters = SteppingFiltersView::from_payload(&json!({
            "classNameFilters": ["$JDK", "$Libraries", "org.mockito.*"],
            "skipSynthetics": true,
            "skipStaticInitializers": true,
            "skipConstructors": false,
            "hideFilteredStackFrames": true
        }))
        .expect("Core 的默认档形状要认得");
        assert_eq!(filters.class_name_filters.len(), 3);
        assert!(filters.skip_synthetics);
        assert!(!filters.skip_constructors);
        assert!(SteppingFilterToggle::SkipConstructors.value(&filters) == false);
        assert!(SteppingFilterToggle::HideFilteredStackFrames.value(&filters));

        // 缺 `classNameFilters` ⇒ 形状不认识（返回 `None`，界面据此走 `Failed` 分支，
        // 而不是显示一个"没有过滤器"的假默认档）。
        assert!(SteppingFiltersView::from_payload(&json!({ "skipSynthetics": true })).is_none());
    }

    /// 异常断点 / 函数断点的请求构造（本切片**只构造、不派发** ——
    /// 它们的派发要活会话，`rust/lithe-core/src/debug/engine.rs:235` / `:276` 都走 `with_session`）。
    #[test]
    fn exception_and_function_breakpoint_payloads_match_the_core_requests() {
        let payload = set_exception_breakpoints_payload("s-1", &["uncaught".to_string()]);
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["breakpoints"][0]["filter"], json!("uncaught"));
        assert_eq!(payload["breakpoints"][0]["enabled"], json!(true));

        let payload = set_function_breakpoints_payload("s-1", &["Main.run".to_string()]);
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["breakpoints"][0]["name"], json!("Main.run"));
    }
}
