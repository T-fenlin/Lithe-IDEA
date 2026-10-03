//! 查找面板的**模型层**：请求 payload、四态状态机、响应解析、结果分组、查询代次。
//!
//! ## 为什么单独一层（U17 第一件）
//!
//! `crate::search_panel` 那一层要拿 `Context` / `Window` / `Entity<InputState>`，单测里
//! 既起不了窗口也不该起；而查找面板真正容易出错的地方全在**纯逻辑**里：字段名拼错一个字
//! Core 就用 `invalid_request` 拒掉整条命令、结果按 `kind` 分组的顺序会决定用户看到什么、
//! 旧查询的结果回来时算不算数。这些都可以用普通 `#[test]` 钉住，所以它们在本模块，
//! 视图层只做"把状态画出来"。
//!
//! ## 归属：确定性逻辑归 Core，本层只做"信封翻译"
//!
//! **排在哪个文件、按什么排序、怎么匹配（大小写 / 全词 / 正则）全部是 Core 的事**
//! （`rust/lithe-core/src/project/files.rs:190-268` 的 `search_with_index`）。本模块
//! 只做四件宿主该做的事：
//!
//! 1. 把面板的选项翻成 `workspace.search` / `workspace.replacePreview` 的 payload
//!    （字段名是**兼容面**）；
//! 2. 把响应翻成界面要画的行（**保持 Core 给出的顺序**，不重新排序、不重新匹配）；
//! 3. 把 Core 的顺序折成"文件头 + 命中行"（纯展示分组，见 [`group`]）；
//! 4. 用代次把过期结果挡掉（[`QueryTimeline`]），并把要取消的 `operationId` 交出来。
//!
//! ⚠️ **本模块不许出现"第二套匹配/排序"**：想按相关度排序、想自己判 regex、想过滤
//! `node_modules` —— 那都该改 Core，不该在这里加一层。宿主今天连可见性数组都不传
//! （与 `crate::search_index` 同一个理由：少一个会漂移的字段）。
//!
//! ## 唯一的例外：`Actions` / `/` 命令模式的候选**不在 Core**
//!
//! [`action_matches`] 是宿主侧的匹配规则，它与上面那条"匹配属于 Core"**不矛盾**：
//! 这一档的候选目录是**宿主自己的命令表**（`crate::command_palette::CommandId` 的 24 个变体，
//! 投影见 [`action_rows`]），Core 不可能知道它 —— Core 只认识工作区文件与索引里的符号。
//! 规则本身也不是新发明的，它逐条照真源 `LitheAction.matches`
//! （`macos/Sources/Lithe/Models/LitheAction.swift:37-51`）：**去空白 + 转小写后的子序列匹配**，
//! 候选串 = `title + subtitle + group`（同文件 `:35` 的 `searchText`），空 query 恒真。
//!
//! 上游那两套匹配器**都不可复用**，这也是"只能自己写一个"的证据：
//!
//! 1. Core 的 `search_with_index`（`rust/lithe-core/src/project/files.rs:190-268`）的输入是
//!    工作区内容，压根没有"命令目录"这一项；
//! 2. 命令面板的谓词**私有**：`gpui-component-0.7.0/src/command/state.rs:332` 的
//!    `fn item_matches(&self, …)` 不是 `pub fn`，宿主拿不到；而且它匹配的是 `CommandItem`
//!    的 label / keywords，与真源的"title + subtitle + group"不是同一条规则。
//!
//! 所以本仓有三处匹配，**判据来源不同、取值域互不重叠**：Core（文件 / 内容 / 类型 / 符号）、
//! 上游命令面板（label / keywords，私有）、以及这里的宿主命令目录（[`action_matches`]）。
//! 前两处本模块一点都不碰。
//!
//! ## 契约出处（写成字段名与行号，便于对着 Core 复核）
//!
//! - 命令名 `workspace.search`：`rust/lithe-core/src/protocol/command.rs:360`；
//! - 请求 `SearchRequest`：`rust/lithe-core/src/project/files.rs:47-74`
//!   （`root` / `query` / `caseSensitive` / `wholeWords` / `regularExpression` /
//!   `maxResults`（默认 200，`files.rs:155-157`）/ `maxFileResults` / `maxContentResults` /
//!   `maxSymbolResults` / `hiddenDirectoryNames` / `hiddenFilePatterns` / `fileMask`，camelCase）；
//! - 响应 `SearchResponse { matches }`：`rust/lithe-core/src/protocol/contracts.rs:107-112`；
//! - 命中 `SearchMatch { kind, path, line, preview, symbolName? }`：同文件 `:94-105`，
//!   `kind` 取 `"file"` / `"content"` / `"symbol"`（`files.rs:222,253`），
//!   `line` 是 **1 基**行号（`Option`，路径命中为 `null`）；
//! - `operationId` 是**信封级**字段（`rust/lithe-core/src/protocol/command.rs:14-15`），
//!   不在 payload 里：`rust/lithe-gpui/crates/shared/src/core_client.rs:169-183` 把它同时
//!   当 `id` 与 `operationId`；取消面是 `lithe_core::cancel_operation`
//!   （`rust/lithe-core/src/lib.rs:34` → `protocol/cancellation.rs:83`）。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

// `CommandAction` 是**数据**（`id` / `category` / `label` / `description` / `icon`），由
// `crate::workspace::command_action` 这个无副作用的纯函数产出；它身上唯一与界面有关的只有
// `icon` 这个字形的句柄。Actions 档的行型**直接复用它**，不为这一档再定义一份字段重复的
// 行型（那正是"不要第二份目录"的另一面）—— 同一个动作在命令面板与 Actions 档里是同一份数据。
use crate::command_palette::CommandAction;

/// `workspace.search`（`protocol/command.rs:360`）。
pub(crate) const SEARCH_COMMAND: &str = "workspace.search";

/// `workspace.searchEverywhere`（`protocol/command.rs:361`）。
///
/// **收的是同一个 `SearchRequest`**（`project/files.rs:272` 的
/// `search_everywhere(request: SearchRequest)`），所以 Quick Open 直接复用本模块的
/// [`payload`] / [`parse_matches`] / [`QueryTimeline`]，不另写第二份构造与解析。
/// 它与 `workspace.search` 的差别只有一件事：响应里**多**了 `kind: "symbol"` / `"type"`
/// 的命中（`files.rs:296-333` 的符号扫描），而 Quick Open 是 **Files scope**，只取
/// `kind: "file"`（见 [`quick_open_rows`]）。
pub(crate) const EVERYWHERE_COMMAND: &str = "workspace.searchEverywhere";

/// 一次查询的结果上限。
///
/// **显式传**而不是靠 Core 的 `#[serde(default)]`：面板要能说出"这一次最多取多少条"，
/// 而"上限是多少"不该藏在 Core 的默认值里。值与 Core 的默认一致
/// （`files.rs:155-157` 的 `default_max_results`），所以显式传不改变行为。
/// ⚠️ Core 里 `maxResults` 是**总上限**：路径命中与内容命中共享它
/// （`files.rs:203-212`），本层不拆成 `maxContentResults` —— 那是 Core 的语义，别在这里改。
pub(crate) const MAX_RESULTS: usize = 200;

/// Search Everywhere 一次查询取多少条（**比 [`MAX_RESULTS`] 大一个量级**）。
///
/// ## 为什么要单独一个上限（B10 的绕行）
///
/// Core 的 `search_everywhere` 把三段结果**拼在一个数组里**再统一截断
/// （`rust/lithe-core/src/project/files.rs:326-332`）：
///
/// ```text
/// matches = file + types + symbols + content   ← files.rs:322-330（注意 content 在最后）
/// matches.truncate(maxResults)                 ← files.rs:331-332
/// ```
///
/// 而前面那一段 `search_with_index` 自己也会在 `matches.len() >= maxResults` 时停
/// （`files.rs:203,211,236`）—— 也就是说 **file + content 的上界就是 `maxResults`**，
/// 它们俩任意一个宽查询就能把整个预算吃光；`types` / `symbols` 各最多 50 条
/// （`files.rs:295` 的 `symbol_limit = max_symbol_results.unwrap_or(50).min(50)`），
/// 排在中段却被末尾那次 `truncate` 整段砍掉 —— 这就是登记在 `BLOCKERS.md` B10 的
/// **符号饥饿**。Core 不返回分段，宿主无法事后补救。
///
/// 宿主能做且**只做**这一件事：把 `maxResults` 抬到 file/content 顶不满的高度，
/// 让那 100 条符号有位置可站。取值的推导：
///
/// - 符号最多占 `2 × 50 = 100` 格（types + symbols，两个 `symbol_limit`）；
/// - 于是"符号被砍"的门槛是 `file + content > maxResults − 100`；
/// - 取 `200`（= [`MAX_RESULTS`]，也是真机 `matchLimit`）时门槛只有 100 条 file/content；
/// - 取 `1000` 时门槛是 900 条 file/content —— 一个宽到在 24px 行高的列表里已经不可用的查询；
/// - 再往上没有收益：Core 自己把 `maxResults` 夹在 10_000（`files.rs:203`），而每多一条
///   都要在每次按键后跨进程序列化 + 解析，宿主一份都画不出来。
///
/// ⚠️ **这是缩小窗口而不是关掉窗口**：真正的修法在 Core（分段返回，或让
/// `truncate` 先保符号），宿主不许自己截断去"修"它（那会变成第二套上限）。
pub(crate) const EVERYWHERE_MAX_RESULTS: usize = 1000;

/// 一次查询的选项（与 `SearchRequest` 的四个用户可选项一一对应）。
///
/// `preserveCase` **不在这里**：它只服务替换（`ReplacementPreviewRequest`，
/// `files.rs:110-136`），所以它挂在 [`ReplaceOptions`] 上 —— 两个命令的选项面
/// 从类型上就分开，`workspace.search` 的 payload 不会多出一个 Core 不认的字段。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchOptions {
    /// `caseSensitive`。
    pub(crate) case_sensitive: bool,
    /// `wholeWords`。
    pub(crate) whole_words: bool,
    /// `regularExpression`。
    pub(crate) regular_expression: bool,
    /// `fileMask`：逗号分隔的 glob（`*.java, *.kt`）；空串 = 不过滤（`files.rs:71-73`）。
    pub(crate) file_mask: String,
}

/// `workspace.search` 的 payload（上限 = [`MAX_RESULTS`]）。
///
/// **不传** `hiddenDirectoryNames` / `hiddenFilePatterns`：Core 会把它们并进内置的
/// 隐藏规则（`files.rs:530-539`），而宿主侧今天没有任何一条 Core 调用传过它们
/// （`explorer` 的 `workspace.snapshot` 也只传 `root`）。少传 = 与 `searchIndex.warm`
/// **同一个索引缓存键**（`project/search_index.rs:72-78` 的键含可见性规则），
/// 不会让同一次搜索再建第二份索引 —— 与 `crate::search_index` 的口径一致。
///
/// **不传**三个 `max*Results`：留空时 Core 让它们各自回落到 `maxResults`
/// （`files.rs:204-205`），那正是我们要的"总上限"语义。
pub(crate) fn payload(root: &str, query: &str, options: &SearchOptions) -> Value {
    payload_with_limit(root, query, options, MAX_RESULTS)
}

/// 与 [`payload`] **同一个构造器**，只把 `maxResults` 换成调用方给的值。
///
/// 存在两个名字而不是两份 `json!`：字段名 / 故意不传哪些字段这些**兼容面**只有一处，
/// 改一处两处一起变（Search Everywhere 要的上限比查找面板大，见
/// [`EVERYWHERE_MAX_RESULTS`]，但请求形状必须逐字相同 —— 两条命令在 Core 里收的是
/// **同一个** `SearchRequest`，`files.rs:272`）。
pub(crate) fn payload_with_limit(
    root: &str,
    query: &str,
    options: &SearchOptions,
    max_results: usize,
) -> Value {
    json!({
        "root": root,
        "query": query,
        "caseSensitive": options.case_sensitive,
        "wholeWords": options.whole_words,
        "regularExpression": options.regular_expression,
        "maxResults": max_results,
        "fileMask": options.file_mask,
    })
}

/// 命中类别（Core 的 `kind` 字段）。
///
/// ## 取值域（四个，出处逐条）
///
/// 1. `"file"` / `"content"`：`workspace.search` 的 `search_with_index`
///    （`rust/lithe-core/src/project/files.rs:223,254`）；
/// 2. `"type"` / `"symbol"`：`workspace.searchEverywhere` 的符号扫描
///    （`files.rs:976-981,993-998` 造出这两种 `JavaSymbol`，`files.rs:296-320` 按 `kind`
///    分成 `types` / `symbols` 两段，经 `project/search_index.rs:456-464` 的
///    `make_symbol_match` 原样带出 `kind` 与 `symbolName`）。
///
/// `Other` 兜住契约以后新增的类别：**解析层不因为不认识就丢掉一整条结果**
/// （丢的是"这条怎么归类"，不是"这条存在"）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MatchKind {
    /// `"file"`：**文件路径**本身命中查询。
    File,
    /// `"content"`：文件内容某一行命中查询。
    Content,
    /// `"type"`：类 / 接口 / 枚举 / 注解这类**类型声明**的名字命中查询。
    Type,
    /// `"symbol"`：方法 / 字段等非类型符号的名字命中查询。
    Symbol,
    /// 其它类别（原样保留）。
    Other(String),
}

impl MatchKind {
    /// 从 `kind` 字段解析。
    fn parse(value: &str) -> Self {
        match value {
            "file" => Self::File,
            "content" => Self::Content,
            "type" => Self::Type,
            "symbol" => Self::Symbol,
            other => Self::Other(other.to_string()),
        }
    }

    /// 这一条是"路径命中"吗（分组时的判据）。
    pub(crate) fn is_file(&self) -> bool {
        matches!(self, Self::File)
    }

    /// 这一条是**正文行**命中吗（Text scope 的取法）。
    pub(crate) fn is_content(&self) -> bool {
        matches!(self, Self::Content)
    }

    /// 这一条是**类型名**命中吗（Classes 那一档的取法；All 也要它）。
    pub(crate) fn is_type(&self) -> bool {
        matches!(self, Self::Type)
    }

    /// 这一条是**符号名**命中吗（Symbols scope 的取法）。
    pub(crate) fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol)
    }
}

/// 一条命中（`contracts.rs:94-105`）。
///
/// `symbol_name` **必须留着**（它曾经被本层显式丢掉，理由写在旧注释里："只有另一条命令会填它"）：
/// `workspace.searchEverywhere` 的 `"type"` / `"symbol"` 命中带它
/// （`project/search_index.rs:462` 的 `symbol_name: Some(indexed.name.clone())`），
/// 而真源结果行画的就是 `result.symbolName ?? url.lastPathComponent`
/// （`macos/Sources/Lithe/Views/Search/SearchEverywhereView.swift:376`）——
/// 丢掉它，符号行就只能画出文件名，**画不出符号名**。
/// `workspace.search` 的两处构造恒为 `None`（`files.rs:227,258`），所以对 Find in Files 无影响。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    /// `kind`。
    pub(crate) kind: MatchKind,
    /// `path`：**工作区相对**、`/` 分隔（`application-boundary.md` 的路径口径）。
    pub(crate) path: String,
    /// `line`：1 基行号；路径命中为 `None`（Core 序列化成 `null`）。
    pub(crate) line: Option<usize>,
    /// `preview`：命中的那一行正文（Core 已经 `trim` 过，`files.rs:257`）。
    pub(crate) preview: String,
    /// `symbolName`：只有 `"type"` / `"symbol"` 命中带它（见本类型的文档）。
    pub(crate) symbol_name: Option<String>,
}

/// 响应解析失败。
///
/// **只给稳定 token，不给句子**：这句话最终会显示在失败态里，而界面文案一律走 `tr`
/// （`shared/src/i18n.rs` 的规矩），宿主不许自己编一句中文/英文。token 与 Core 的错误码
/// 同一个口径（机器可读、可 grep、可进诊断行），显示时由
/// `lithe.search.gpui.failedDetail` 的 `{reason}` 承接。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    /// `ok:true` 但没有 `data`。`workspace.search` 一定回 `data`（`dispatcher` 的
    /// `SearchResponse` 分支），所以这是**契约异常**而不是"没有结果" —— 两者必须分开。
    MissingData,
    /// `data` 不是对象、缺 `matches` / `files` 数组，或某个元素缺
    /// `kind` / `preview` / `before` / `after` / `line`。
    Malformed,
    /// `matches[i]` / `files[i]` 缺必填的 `path`（两类响应长得很像，共用这一个 token）。
    MissingPath(usize),
}

impl ParseError {
    /// 稳定 token（进诊断行，也进失败态）。
    pub(crate) fn token(self) -> String {
        match self {
            Self::MissingData => "missing_data".to_string(),
            Self::Malformed => "malformed_response".to_string(),
            Self::MissingPath(index) => format!("missing_path_at_{index}"),
        }
    }
}

/// 把 `CoreClient::execute` 的 `data` 解析成命中列表。
///
/// **空结果与失败是两件事**：`Ok(vec![])` 是"真的没有命中"，`Err(..)` 是"这条响应不可用"。
/// 面板必须把两者画成不同的东西（`lithe.search.noResultsFor` vs
/// `lithe.search.gpui.failedDetail`）。
pub(crate) fn parse_matches(data: Option<&Value>) -> Result<Vec<Match>, ParseError> {
    let Some(data) = data else {
        return Err(ParseError::MissingData);
    };
    let Some(items) = data.get("matches").and_then(Value::as_array) else {
        return Err(ParseError::Malformed);
    };
    let mut matches = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(path) = item.get("path").and_then(Value::as_str) else {
            return Err(ParseError::MissingPath(index));
        };
        let (Some(kind), Some(preview)) = (
            item.get("kind").and_then(Value::as_str),
            item.get("preview").and_then(Value::as_str),
        ) else {
            return Err(ParseError::Malformed);
        };
        matches.push(Match {
            kind: MatchKind::parse(kind),
            path: path.to_string(),
            line: item.get("line").and_then(Value::as_u64).map(|l| l as usize),
            preview: preview.to_string(),
            // `symbolName` 缺失（或不是字符串）就是 `None`：`workspace.search` 恒缺它，
            // 那是**正常**形状，不是契约异常（所以不在这里报 `Malformed`）。
            symbol_name: item
                .get("symbolName")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    Ok(matches)
}

/// 结果列表的一行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchRow {
    /// 文件头：这个文件有命中（或**路径本身**命中了查询）。
    ///
    /// `hits` = 这个文件下的命中**行**数；`0` 只可能出现在"路径命中"的
    /// 文件上（查询匹配了文件名，但正文里没有那一行）。
    File {
        /// 工作区相对路径（`/` 分隔）。
        path: String,
        /// 这个文件下的命中行数。
        hits: usize,
    },
    /// 一条命中行。
    Hit {
        /// 工作区相对路径（与它上面那个文件头同值）。
        path: String,
        /// 1 基行号；Core 没给就是 `None`。
        line: Option<usize>,
        /// 命中行的预览正文。
        preview: String,
    },
}

/// 把 Core 的顺序折成"文件头 + 命中行"。
///
/// ## 为什么不重排
///
/// Core 的输出顺序是：**先**全部 `kind: "file"` 的路径命中（按索引 id，`files.rs:209-231`）、
/// **再**按索引 id + 行号输出的内容命中（`files.rs:233-266`）。本函数只做**折叠**：
///
/// - 文件之间的顺序 = 该文件**第一次出现**的顺序；
/// - 同一文件内命中行的顺序 = Core 给出的顺序（即行号升序）。
///
/// 两者都不引入第二套排序口径（"确定性逻辑属于 Core"）。分组是**展示**需要：
/// IDEA 与真机的结果区都是"文件头 + 该文件的命中行"，而不是把路径与正文平铺成一行一条。
///
/// 只有内容命中的文件也会有文件头（否则那些行没有归属）；只有路径命中的文件得到一个
/// `hits: 0` 的文件头。**不会**出现的路径不会凭空建头。
pub(crate) fn group(matches: &[Match]) -> Vec<SearchRow> {
    // 路径 → 在 `paths` / `hits` 里的下标。用线性查找而不是 `HashMap`：
    // 上界是 `MAX_RESULTS`（200），而且**顺序即输出顺序**，线性表最直白。
    let mut paths: Vec<String> = Vec::new();
    let mut hits: Vec<Vec<SearchRow>> = Vec::new();
    for item in matches {
        let index = match paths.iter().position(|path| path == &item.path) {
            Some(index) => index,
            None => {
                paths.push(item.path.clone());
                hits.push(Vec::new());
                paths.len() - 1
            }
        };
        if !item.kind.is_file() {
            hits[index].push(SearchRow::Hit {
                path: item.path.clone(),
                line: item.line,
                preview: item.preview.clone(),
            });
        }
    }

    let mut rows = Vec::with_capacity(paths.len() + matches.len());
    for (index, path) in paths.into_iter().enumerate() {
        rows.push(SearchRow::File {
            path: path.clone(),
            hits: hits[index].len(),
        });
        rows.extend(hits[index].iter().cloned());
    }
    rows
}

// ---------------------------------------------------------------------------
// Quick Open（`Ctrl+P`；`workspace.searchEverywhere` 的 Files scope）
// ---------------------------------------------------------------------------

/// Quick Open 的一行：**一个文件**（Core 的路径命中）。
///
/// 只有 [`MatchKind::File`] 会变成行 —— 这是 macos 真源里 `.files` scope 的取法
/// （`SearchEverywhereView.swift:132-133` 把 `.files` 映射到
/// `feature.searchEverywhereResults.fileMatches`），**不是**宿主侧的第二套匹配：
/// "哪一条算文件命中、按什么顺序回来"全在 Core（`files.rs:209-231`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuickOpenRow {
    /// `path`：工作区相对、`/` 分隔。**这一行的身份**（`ElementId` 用它，不用下标）。
    pub(crate) path: String,
    /// 文件名（最后一段）。真源画的是 `url.lastPathComponent`
    /// （`SearchEverywhereView.swift:376`）。
    pub(crate) name: String,
    /// 所在目录（去掉文件名那一段）；文件直接在工作区根下时为空串。真源的
    /// `containerPath(for:)` 就是 `deletingLastPathComponent`（同文件 `:446-450`）。
    pub(crate) parent: String,
}

impl QuickOpenRow {
    /// 从工作区相对路径拆出「文件名 + 所在目录」。
    fn new(path: String) -> Self {
        let (parent, name) = split_path(&path);
        Self { path, name, parent }
    }
}

/// 工作区相对路径 → `(所在目录, 最后一段)`。
///
/// 按 `/` 拆（契约口径：相对路径一律 `/` 分隔），**不碰**平台分隔符。三个投影
/// （Quick Open 的文件行、Search Everywhere 的文件/符号/正文行）共用这一份，
/// 免得"名字怎么取"在两处漂移。
fn split_path(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent.to_string(), name.to_string()),
        None => (String::new(), path.to_string()),
    }
}

/// 把 Core 的命中折成 Quick Open 要画的文件行。
///
/// 三件事，各有出处：
///
/// 1. **只留路径命中**（`kind == "file"`）—— Files scope（见 [`QuickOpenRow`]）；
/// 2. **保持 Core 的顺序**：不排序、不按相关度重排（"排序 / 匹配 / 模糊度属于 Core"）；
/// 3. **同一路径只出现一次**（保留第一次出现的那一条）：与 [`group`] 同一个折叠口径。
///    Core 今天对同一个文件只会给一条路径命中（`files.rs:209-231` 每个索引 id 一条），
///    所以它是防御性的"一个文件不许占两行"，不是第二套去重策略。
pub(crate) fn quick_open_rows(matches: &[Match]) -> Vec<QuickOpenRow> {
    let mut rows: Vec<QuickOpenRow> = Vec::new();
    for item in matches {
        if !item.kind.is_file() {
            continue;
        }
        if rows.iter().any(|row| row.path == item.path) {
            continue;
        }
        rows.push(QuickOpenRow::new(item.path.clone()));
    }
    rows
}

// ---------------------------------------------------------------------------
// Search Everywhere（`workspace.searchEverywhere` 的四个 Core scope + Actions）
// ---------------------------------------------------------------------------

/// Search Everywhere 的 scope。
///
/// ## 真源六个，本侧六个（`Actions` 是 U17 后续切片补上的）
///
/// 真源 `allCases`：`All` / `Classes` / `Files` / `Symbols` / `Actions` / `Text`
/// （`macos/Sources/Lithe/Views/Search/SearchEverywhereView.swift:5-14`）。本侧逐格照抄，
/// 一个不少、相对顺序也不变。
///
/// ## 两种来源，判据是 [`Self::is_core_backed`]
///
/// | scope | 数据源 | 真源出处 |
/// | --- | --- | --- |
/// | `All` / `Classes` / `Files` / `Symbols` / `Text` | Core `workspace.searchEverywhere` 的命中投影 | `SearchEverywhereView.swift:120-137` |
/// | `Actions` | **宿主命令目录**（[`action_rows`]），与 Core 无关 | 同文件 `:138-139`（`actionMatches(query)`） |
///
/// ⚠️ `Actions` **不是工作区搜索**：真源那一档的候选是 `LitheActionRegistry`
/// （`macos/Sources/Lithe/Models/LitheAction.swift:56-102` 的 35 条动作），本侧的对应物是
/// [`crate::command_palette::CommandId`] 的 **24 个变体**。所以这一档显示的**永远是子集**
/// （宿主目录 24 条 vs 真源命令目录 `LitheCommandCatalog.swift:129-182` 的 39 条）——
/// 不假装等价，UI-MAP 里同样写明。
///
/// ## scope ↔ Core `kind`（只对五个 Core scope 成立）
///
/// | scope | Core `kind` | 真源出处 |
/// | --- | --- | --- |
/// | `All` | `file` + `type` + `symbol`（**不含 `content`**） | `SearchEverywhereView.swift:122-129` |
/// | `Classes` | `type` | 同文件 `:130-131` |
/// | `Files` | `file` | 同文件 `:132-133` |
/// | `Symbols` | `symbol` | 同文件 `:134-135` |
/// | `Text` | `content` | 同文件 `:136-137` |
///
/// ⚠️ **All 不含 `content`** 是真源明写的取舍：正文命中只在 Text 标签页出现，
/// 避免与 Find in Files 重叠（`SearchEverywhereView.swift:123-125` 的注释）。
/// 这一条**没有**因为新增 Classes 或 Actions 而改变：All 仍是 `file + type + symbol` 三段。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchEverywhereScope {
    /// 名字类命中三段拼接：文件 + 类型 + 符号。
    All,
    /// 类型声明名命中（Core 的 `kind == "type"`）。
    Classes,
    /// 文件路径命中。
    Files,
    /// 非类型符号名命中。
    Symbols,
    /// **宿主命令目录**里的动作（**不发 Core 调用**，见 [`Self::is_core_backed`]）。
    Actions,
    /// 文件正文行命中。
    Text,
}

impl SearchEverywhereScope {
    /// 标签行里的顺序 = Tab / `←` / `→` 环绕的顺序（真源 `allCases` 的顺序，
    /// `SearchEverywhereView.swift:5-14`）。
    ///
    /// 真源六个，本侧六个、相对顺序逐格照抄 —— 所以 `Actions` 排在 **`Symbols` 之后、
    /// `Text` 之前**（下标 4），这是本切片最容易写错的一格。
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::Classes,
        Self::Files,
        Self::Symbols,
        Self::Actions,
        Self::Text,
    ];

    /// 稳定 token：诊断行与 `ElementId` 用它，**不用**显示名（显示名随语言变）。
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Classes => "classes",
            Self::Files => "files",
            Self::Symbols => "symbols",
            Self::Actions => "actions",
            Self::Text => "text",
        }
    }

    /// Tab 标签的文案键（`gpui 自有键`，出处见 `extract-locale.mjs`）。
    pub(crate) fn label_key(self) -> &'static str {
        match self {
            Self::All => "lithe.searchEverywhere.scope.all",
            Self::Classes => "lithe.searchEverywhere.scope.classes",
            Self::Files => "lithe.searchEverywhere.scope.files",
            Self::Symbols => "lithe.searchEverywhere.scope.symbols",
            Self::Actions => "lithe.searchEverywhere.scope.actions",
            Self::Text => "lithe.searchEverywhere.scope.text",
        }
    }

    /// 这个 scope 没有命中时的空态键（真源的 `emptyResultsMessage`，
    /// `SearchEverywhereView.swift:75-92`）。
    pub(crate) fn empty_key(self) -> &'static str {
        match self {
            Self::All => "lithe.searchEverywhere.noMatches.all",
            Self::Classes => "lithe.searchEverywhere.noMatches.classes",
            Self::Files => "lithe.searchEverywhere.noMatches.files",
            Self::Symbols => "lithe.searchEverywhere.noMatches.symbols",
            Self::Actions => "lithe.searchEverywhere.noMatches.actions",
            Self::Text => "lithe.searchEverywhere.noMatches.text",
        }
    }

    /// 这一档的行**是不是来自 Core**（`workspace.searchEverywhere`）。
    ///
    /// 只有 `Actions` 是 `false`：它的行来自宿主命令目录（真源 `:138-139`），与工作区搜索
    /// 无关。这个谓词是"**Actions 档不发 Core 调用**"这条约束的**单一判据**
    /// —— 浮层的查询 / 防抖 / 采纳三条路径都只看它（配合 [`issues_core_request`]），
    /// 而不各自 `match scope` 一遍。
    pub(crate) fn is_core_backed(self) -> bool {
        !matches!(self, Self::Actions)
    }

    /// 在六个 scope 上移动 `offset` 格并**环绕**。
    ///
    /// 真源 `moveScope(by:)` 就是 `(currentIndex + offset + count) % count`
    /// （`SearchEverywhereView.swift:542-547`），Tab / Shift-Tab 与 `←` / `→` 共用它
    /// （同文件 `:523-529`）。用 `rem_euclid` 保证负偏移也落在 `0..count`。
    pub(crate) fn moved(self, offset: isize) -> Self {
        let count = Self::ALL.len() as isize;
        let index = Self::ALL
            .iter()
            .position(|scope| *scope == self)
            .unwrap_or(0) as isize;
        Self::ALL[(index + offset).rem_euclid(count) as usize]
    }
}

/// Search Everywhere 结果列表的一行（**Core 命中**的行）。
///
/// **一种行型承载五个 Core scope**，因为真源就是一行型：`resultRow` 画的是
/// 「名字 → `:行号` → 所在目录 → 预览（只有 content）」
/// （`SearchEverywhereView.swift:364-397`），而"名字"取 `symbolName ?? 文件名`
/// （同文件 `:376`）。所以这里不按 scope 分三种行型 —— 那会在视图层生出一层无用的 `match`。
///
/// ⚠️ `Actions` 档的行**不是这一种**：它没有路径 / 行号 / 目录，画的是
/// 「图标 + title + subtitle + group」（同文件 `:457-495` 的 `actionRow`），
/// 行型是宿主命令目录的 `CommandAction`（见 [`action_rows`]）。两种行型在视图层由一个
/// 二选一的枚举装在一起（与真源 `SearchItem` 的 `case result / case action`
/// 同构，同文件 `:115-118`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EverywhereRow {
    /// 这一条来自哪一类命中（决定画不画行号、画不画预览）。
    pub(crate) kind: MatchKind,
    /// 工作区相对路径（`/` 分隔）。**这一行的身份**（`ElementId` 用它拼，不用下标）。
    pub(crate) path: String,
    /// 行首那一段：符号 / 类型命中用 `symbolName`，其余用路径最后一段。
    pub(crate) name: String,
    /// 所在目录（去掉最后一段）；文件直接在工作区根下时为空串。
    pub(crate) parent: String,
    /// 1 基行号（路径命中为 `None`）。
    pub(crate) line: Option<usize>,
    /// 命中行正文（Core 已 `trim`）；只有 [`MatchKind::Content`] 会画它。
    pub(crate) preview: String,
}

impl EverywhereRow {
    /// 画不画 `:行号`：真源是 `result.kind != .file`（`SearchEverywhereView.swift:349`）。
    pub(crate) fn shows_line(&self) -> bool {
        !self.kind.is_file()
    }

    /// 画不画预览正文：真源只在 `result.kind == .content` 时画（同文件 `:392-397`）。
    pub(crate) fn shows_preview(&self) -> bool {
        self.kind.is_content()
    }
}

/// 把一条 Core 命中翻成一行。
fn everywhere_row(item: &Match) -> EverywhereRow {
    let (parent, file_name) = split_path(&item.path);
    EverywhereRow {
        kind: item.kind.clone(),
        path: item.path.clone(),
        // 真源 `SearchEverywhereView.swift:376` 的 `result.symbolName ?? url.lastPathComponent`。
        name: item.symbol_name.clone().unwrap_or(file_name),
        parent,
        line: item.line,
        preview: item.preview.clone(),
    }
}

/// 按 scope 从**同一份** Core 命中里投影出要画的行。
///
/// 三件事的判据都在这里，而且都是"取子集"，**没有任何重排 / 重筛 / 重排相关度**：
/// 顺序 = Core 给的顺序（`files.rs:326-330` 的 file → types → symbols → content）。
///
/// - `Files`：复用 [`quick_open_rows`]（只留 `kind == "file"`、同一路径只占一行），
///   Quick Open 与这个 scope 是**同一份投影**，不写第二份；
/// - `Classes` / `Symbols` / `Text`：各按一个 `kind` 过滤，不排序、不去重
///   （Core 每个符号给一条，`search_index.rs:456-464`）；
/// - `All`：`Files` 那一段 + `type` 一段 + `symbol` 一段，**不含 `content`**（见
///   [`SearchEverywhereScope`] 的表）；
/// - `Actions`：**恒为空**。它的行来自宿主命令目录（[`action_rows`]），与 Core 命中无关 ——
///   真源在 `resultSource` 那一层就把两条来源分开了（`SearchEverywhereView.swift:120-141`：
///   `case .actions/.commands` 走 `actionMatches`，其余走 `feature.searchEverywhereResults`）。
///   这里给一个空分支而不是 `unreachable!()`：调用方可能遍历 [`SearchEverywhereScope::ALL`]
///   （诊断行就是这么数的），那时 `actions=` 报 0 是**正确**的读数（本函数不产生动作行）。
pub(crate) fn everywhere_rows(
    scope: SearchEverywhereScope,
    matches: &[Match],
) -> Vec<EverywhereRow> {
    match scope {
        SearchEverywhereScope::Files => quick_open_rows(matches)
            .into_iter()
            .map(|row| EverywhereRow {
                kind: MatchKind::File,
                path: row.path,
                name: row.name,
                parent: row.parent,
                // 路径命中没有行号可跳：Core 那条的 `line` 恒为 `null`（`files.rs:225`）。
                line: None,
                preview: String::new(),
            })
            .collect(),
        SearchEverywhereScope::Classes => named_rows(matches, MatchKind::is_type),
        SearchEverywhereScope::Symbols => named_rows(matches, MatchKind::is_symbol),
        SearchEverywhereScope::Text => content_rows(matches),
        SearchEverywhereScope::Actions => Vec::new(),
        SearchEverywhereScope::All => {
            let mut rows = everywhere_rows(SearchEverywhereScope::Files, matches);
            rows.extend(named_rows(matches, MatchKind::is_type));
            rows.extend(named_rows(matches, MatchKind::is_symbol));
            rows
        }
    }
}

/// `type` / `symbol` 两段的投影（它们只差一个谓词）。
fn named_rows(matches: &[Match], keep: fn(&MatchKind) -> bool) -> Vec<EverywhereRow> {
    matches
        .iter()
        .filter(|item| keep(&item.kind))
        .map(everywhere_row)
        .collect()
}

/// 正文行的投影（Text scope）。
fn content_rows(matches: &[Match]) -> Vec<EverywhereRow> {
    matches
        .iter()
        .filter(|item| item.kind.is_content())
        .map(everywhere_row)
        .collect()
}

// ---------------------------------------------------------------------------
// Actions scope 与 `/` 命令模式：候选目录在**宿主**，不在 Core
// ---------------------------------------------------------------------------

/// `/` 命令模式的空态键（真源 `.commands` 的 `emptyResultsMessage`，
/// `SearchEverywhereView.swift:77-78`）。
///
/// **不是**任何一个 scope 的空态：真源给命令模式单独一句 "No matching commands"。
pub(crate) const COMMANDS_EMPTY_KEY: &str = "lithe.searchEverywhere.noMatchingCommands";

/// 这一串查询是不是真源的 `/` 命令模式
/// （`SearchEverywhereView.swift:20-26` 的 `query.hasPrefix("/")`）。
///
/// 判据在**原始**查询串上（不去空白）：真源也是先 `hasPrefix` 再 `dropFirst`。
pub(crate) fn is_command_query(query: &str) -> bool {
    query.starts_with('/')
}

/// `/` 之后的命令查询串；不是命令模式时 `None`
/// （`SearchEverywhereView.swift:20-26` 的 `String(query.dropFirst())`）。
pub(crate) fn command_query(query: &str) -> Option<&str> {
    query.strip_prefix('/')
}

/// 这一档 + 这个查询串走的是**宿主命令目录**吗（`Actions` 档或 `/` 命令模式）。
///
/// 真源的判据就是这两条（`SearchEverywhereView.swift:59-63` 先判 `commandQuery`、
/// `:70` 再按 `scope == .actions`），本侧把它们合成一个谓词，浮层拿它决定"行从哪来"。
///
/// ⚠️ 它与 [`issues_core_request`] 的否定**不等价**：空查询同样不发 Core，但那是 Core 档
/// 自己的 `idle` 态（列表空着、占位符说话），与命令目录无关 —— 所以两个谓词分开写，
/// 免得"空查询"被当成"命令目录"。
pub(crate) fn uses_command_catalog(scope: SearchEverywhereScope, query: &str) -> bool {
    !scope.is_core_backed() || is_command_query(query)
}

/// 这一刻该不该发 `workspace.searchEverywhere`（**单一判据**）。
///
/// 三条一起成立才发：
///
/// 1. 这一档是 Core 档（[`SearchEverywhereScope::is_core_backed`]）—— `Actions` 的行来自
///    宿主命令目录（真源 `:138-139`），一个 Core 请求都不该发；
/// 2. 不是 `/` 命令模式 —— 真源把 `/` 判在 scope **之前**（`:59-63` 的 `commandQuery`
///    非 nil 就直接走 `.commands`），那一档的候选同样是宿主命令目录；
/// 3. 查询串不是空白 —— `search_everywhere` 对空查询直接回空（`files.rs:274-279`），
///    发出去只是白跑一次跨进程往返。
///
/// ⚠️ 浮层的查询 / 防抖 / 采纳三条路径**都调它**（而不是各自 `match scope` 一遍），
/// 所以"Actions 档不发 Core"这条约束只有一个判据、一处可测。
pub(crate) fn issues_core_request(scope: SearchEverywhereScope, query: &str) -> bool {
    !uses_command_catalog(scope, query) && !query.trim().is_empty()
}

/// 真源 `LitheAction.matches` 的逐条翻译：**去空白 + 转小写后的子序列匹配**。
///
/// ## 规则（照抄，不发明）
///
/// 出处 `macos/Sources/Lithe/Models/LitheAction.swift:37-51`：
///
/// 1. `query.lowercased().filter { !$0.isWhitespace }`（`:38-40`）；
/// 2. 空 query **恒真**（`:41`）—— 所以 `/` 后面什么都不写时列出全部命令；
/// 3. 候选串 `searchText = "\(title) \(subtitle) \(group.rawValue)"`，再 `lowercased()`
///    （`:35`、`:42`）；
/// 4. 顺着候选串走一遍，能按顺序把查询串的每个字符都匹配掉就成立（`:43-50`）。
///
/// ⚠️ 这里是**子序列**而不是子串：`"ose"` 能命中 `"Open Settings"`（`O`…`S`…`E`）。
/// 真源就是这条规则，所以不许"顺手换成更聪明的模糊度打分" —— 那是另一件事、另一个切片。
///
/// ## 为什么它不能复用上游那两套（见模块文档的同名小节）
///
/// Core 的匹配只认工作区内容；命令面板的 `item_matches`
/// （`gpui-component-0.7.0/src/command/state.rs:332`）是**私有** `fn`，宿主拿不到，
/// 而且它匹配的是 `CommandItem` 的 label / keywords。所以这一套只能落在宿主侧。
pub(crate) fn action_matches(query: &str, title: &str, subtitle: &str, group: &str) -> bool {
    let normalized: Vec<char> = query
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if normalized.is_empty() {
        return true;
    }
    // 候选串与真源 `searchText` 逐字同构：三段用空格连起来（`LitheAction.swift:35`）。
    let candidates = format!("{title} {subtitle} {group}").to_lowercase();
    let mut wanted = normalized.into_iter().peekable();
    for character in candidates.chars() {
        match wanted.peek() {
            // 查询串匹配完了：后面的字符不用再看。
            None => break,
            Some(&expected) if character == expected => {
                wanted.next();
            }
            Some(_) => {}
        }
    }
    wanted.peek().is_none()
}

/// Actions scope 的行：宿主命令目录按真源规则过滤后的那一段。
///
/// 三件事，全都不发明：
///
/// 1. **顺序 = 目录顺序**（`catalog` 进来的顺序；宿主侧是
///    `crate::command_palette::CommandId::ALL` 的顺序）—— 真源 `actionMatches` 也只是
///    `.map(SearchItem.action)`，不排序（`SearchEverywhereView.swift:138-139`）；
/// 2. **过滤只看 [`action_matches`]**（候选串 = `label` + `description` + 分类名，对应真源的
///    title + subtitle + group），空 query 恒真；
/// 3. **不重排、不去重、不截断**（宿主目录只有 24 条，截断与否由目录自己决定）。
///
/// 行型**直接复用**命令面板的 [`CommandAction`]：同一个动作在两个入口里是同一份数据
/// （label / description / category / icon 全都不复制），所以这里不为 Actions 档另定义一份
/// 字段重复的行型，也不建第二份目录。
///
/// ⚠️ `catalog` 按值收：目录每帧由 [`crate::workspace::ShellWorkspace::action_catalog`] 现取
/// （朝向 / 主题会变，缓存一份就会过期），过滤后直接交给视图，中间不做第二次克隆。
pub(crate) fn action_rows(catalog: Vec<CommandAction>, query: &str) -> Vec<CommandAction> {
    catalog
        .into_iter()
        .filter(|action| {
            action_matches(
                query,
                &action.label,
                &action.description,
                &action.category_label(),
            )
        })
        .collect()
}

/// Search Everywhere 的四态（**只描述 Core 那一侧**）。
///
/// 与 [`QuickOpenState`] 的差别只有一处：`ready` / `loading` 装的是 **Core 的原始命中**
/// 而不是投影后的行 —— 因为 scope 是**本地**开关（真源的搜索任务只按查询串作 key，
/// `SearchEverywhereView.swift:207` 的 `.task(id: "\(queryMode.workspaceSearchTaskID)|…")`
/// **不含 scope**），切 Core scope 不该再打一次 Core。
///
/// ⚠️ `Actions` 档 / `/` 命令模式**不用这个枚举**：它们的行由 [`action_rows`] 现算
/// （宿主命令目录，没有"在飞 / 过期 / 失败"这三个 Core 才有的状态）。
/// 进 Actions 时这里的状态**保持不动**（上一次 Core 查询的命中留着），
/// 这样切回 Core 档如果是同一个查询串就能立刻复用 —— 与真源"任务不因切 scope 重跑"同效。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EverywhereState {
    /// 查询串为空：**不发 Core 调用**（`search_everywhere` 对空查询直接回空，`files.rs:274-279`）。
    Idle,
    /// 已发出查询、结果还没回来；`matches` 是**上一次**的结果（继续可见，理由同
    /// [`QuickOpenState::Loading`]）。
    Loading {
        /// 这一次查询的串。
        query: String,
        /// 上一次查询的命中。
        matches: Vec<Match>,
    },
    /// 结果到了；`matches` 可能是空的（"这个 scope 没有匹配"）。
    Ready {
        /// 这一次查询的串。
        query: String,
        /// Core 的命中（五个 Core scope 共用这一份）。
        matches: Vec<Match>,
    },
    /// 失败了；`reason` 是可显示的原因（Core 的 `message` 或 [`ParseError::token`]）。
    Failed {
        /// 这一次查询的串。
        query: String,
        /// 可显示的原因。
        reason: String,
    },
}

impl EverywhereState {
    /// 进 `loading` 态（带上一次的命中）。
    pub(crate) fn loading(query: impl Into<String>, previous: &[Match]) -> Self {
        Self::Loading {
            query: query.into(),
            matches: previous.to_vec(),
        }
    }

    /// 进 `ready` 态（空 `matches` = 空结果，不是失败）。
    pub(crate) fn ready(query: impl Into<String>, matches: &[Match]) -> Self {
        Self::Ready {
            query: query.into(),
            matches: matches.to_vec(),
        }
    }

    /// 进 `failed` 态。
    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    /// 当前查询串（`idle` 时为空串）。
    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query, .. }
            | Self::Ready { query, .. }
            | Self::Failed { query, .. } => query,
        }
    }

    /// 手上这一份 Core 命中（`failed` / `idle` 为空）。
    ///
    /// `loading` **也算有**：上一次的结果在等新结果时继续可见（见该变体）。
    pub(crate) fn matches(&self) -> &[Match] {
        match self {
            Self::Loading { matches, .. } | Self::Ready { matches, .. } => matches,
            Self::Idle | Self::Failed { .. } => &[],
        }
    }

    /// 这个 scope 下要画的行（**纯投影**，每次渲染现算 —— 与真源的 `visibleItems` 同构）。
    ///
    /// ⚠️ `Actions` 恒返回空（见 [`everywhere_rows`]）：那一档的行来自 [`action_rows`]。
    pub(crate) fn rows(&self, scope: SearchEverywhereScope) -> Vec<EverywhereRow> {
        everywhere_rows(scope, self.matches())
    }

    /// 手上这份命中**就是这个查询串**的 `ready` 结果吗。
    ///
    /// 用途只有一个：从 `Actions` / `/` 模式切**回** Core 档时，决定要不要补发一次查询。
    ///
    /// - 同一个查询串 + 已经是 `Ready` ⇒ 复用，不重发（真源的搜索任务不因切 scope 重跑，
    ///   `SearchEverywhereView.swift:207` 的任务 id 只有查询串，没有 scope）；
    /// - `Loading` 不算：进 `Actions` 时在飞的请求已经被 `timeline.abandon()` 撤掉
    ///   （`Actions` 档不发 Core），那一份**没有**在跑的请求，必须重发；
    /// - `Idle` / `Failed` 也不算（手上的 `query()` 分别是空串与失败的那个串）。
    pub(crate) fn is_ready_for(&self, query: &str) -> bool {
        matches!(self, Self::Ready { query: ready, .. } if ready == query.trim())
    }
}

/// 四态（与 [`SearchState`] 同一份契约：`idle` / `loading` / `ready` / `failed`）。
///
/// **为什么不直接复用 [`SearchState`]**：它的 `ready` 装的是查找面板的 [`SearchRow`]
/// （文件头 + 命中行**两层**），而 Quick Open 只有文件行**一层**；硬塞进去会让查找面板的
/// 渲染多一层无用 `match`。四态的**语义**与 [`SearchState`] 逐条一致 ——
/// 尤其"空结果是 `Ready`，不是 `Failed`"这一条。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QuickOpenState {
    /// 查询串为空（或只有空白）：**不发 Core 调用**（`search_everywhere` 对空查询直接回空，
    /// `files.rs:274-279`），列表空着，占位符负责说"输入以搜索文件"。
    Idle,
    /// 已发出查询、结果还没回来。
    ///
    /// ⚠️ `Loading` 与 `Ready { rows: [] }` **必须分开**：`Command` 只在非 loading 且零行时
    /// 才画空态（`command/state.rs:989-993`），否则"在路上"会被画成"没有匹配"。
    ///
    /// `rows` = **上一次查询**留下的结果，继续留在屏幕上直到新结果到达（真机同样不清空
    /// `visibleItems`，见 `SearchEverywhereView.swift:302-328`；打一个字就闪一下空列表是更差的观感）。
    Loading {
        /// 这一次查询的串。
        query: String,
        /// 上一次查询的文件行（可能为空）。
        rows: Vec<QuickOpenRow>,
    },
    /// 结果到了；`rows` 可能是空的（"没有匹配的文件"）。
    Ready {
        /// 这一次查询的串。
        query: String,
        /// 文件行。
        rows: Vec<QuickOpenRow>,
    },
    /// 失败了；`reason` 是可直接显示的原因（Core 的 `message` 或 [`ParseError::token`]）。
    Failed {
        /// 这一次查询的串。
        query: String,
        /// 可直接显示的原因。
        reason: String,
    },
}

impl QuickOpenState {
    /// 进 `loading` 态，**带上上一次的行**（它们继续可见，见 [`QuickOpenState::Loading`]）。
    pub(crate) fn loading(query: impl Into<String>, previous: &[QuickOpenRow]) -> Self {
        Self::Loading {
            query: query.into(),
            rows: previous.to_vec(),
        }
    }

    /// 进 `ready` 态（`matches` 在这里过 [`quick_open_rows`]；空列表 = 空结果，不是失败）。
    pub(crate) fn ready(query: impl Into<String>, matches: &[Match]) -> Self {
        Self::Ready {
            query: query.into(),
            rows: quick_open_rows(matches),
        }
    }

    /// 进 `failed` 态。
    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    /// 当前查询串（`idle` 时为空串）。
    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query, .. }
            | Self::Ready { query, .. }
            | Self::Failed { query, .. } => query,
        }
    }

    /// 要画的文件行。
    ///
    /// `loading` **也算有行**：上一次的结果在等新结果时继续可见（见该变体的文档）。
    /// `failed` 没有行 —— 失败态整块换成原因（`Command` 的空态槽）。
    pub(crate) fn rows(&self) -> &[QuickOpenRow] {
        match self {
            Self::Loading { rows, .. } | Self::Ready { rows, .. } => rows,
            Self::Idle | Self::Failed { .. } => &[],
        }
    }
}

/// 面板的四态（契约要求的 `idle` / `loading` / `ready` / `failed`）。
///
/// ⚠️ **空结果属于 `Ready`**（`rows` 为空），不是 `Failed`：契约要求"空结果与失败必须区分"。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchState {
    /// 没有查询串（或查询串只有空白）：画"在项目中搜索"的空态，**不发 Core 调用**。
    Idle,
    /// 已发出查询、结果还没回来。
    Loading {
        /// 这一次查询的串（诊断与"结果属于谁"的判据）。
        query: String,
    },
    /// 结果到了；`rows` 可能是空的（"没有命中"）。
    Ready {
        /// 这一次查询的串。
        query: String,
        /// 分组后的行。
        rows: Vec<SearchRow>,
    },
    /// 失败了；`reason` 是可直接显示的原因（Core 的 `message` 或 [`ParseError::token`]）。
    Failed {
        /// 这一次查询的串。
        query: String,
        /// 可直接显示的原因。
        reason: String,
    },
}

impl SearchState {
    /// 进 `loading` 态。
    pub(crate) fn loading(query: impl Into<String>) -> Self {
        Self::Loading {
            query: query.into(),
        }
    }

    /// 进 `ready` 态（`matches` 在这里分组；空 `matches` = 空结果，不是失败）。
    pub(crate) fn ready(query: impl Into<String>, matches: &[Match]) -> Self {
        Self::Ready {
            query: query.into(),
            rows: group(matches),
        }
    }

    /// 进 `failed` 态。
    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    /// 当前查询串（`idle` 时为空串）。
    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query } | Self::Ready { query, .. } | Self::Failed { query, .. } => {
                query
            }
        }
    }

    /// 分组后的行（只有 `ready` 有；其余态画别的东西）。
    pub(crate) fn rows(&self) -> &[SearchRow] {
        match self {
            Self::Ready { rows, .. } => rows,
            _ => &[],
        }
    }

    /// `(文件数, 命中行数)`：文件数 = 文件头数。
    ///
    /// 给诊断行用（`S1_SEARCH complete files=… hits=…`）—— 那是这一件唯一的机器可核对锚点。
    pub(crate) fn counts(&self) -> (usize, usize) {
        let mut files = 0;
        let mut hits = 0;
        for row in self.rows() {
            match row {
                SearchRow::File { .. } => files += 1,
                SearchRow::Hit { .. } => hits += 1,
            }
        }
        (files, hits)
    }
}

// ===========================================================================
// Replace in Files（U17 第四件）：`workspace.replacePreview` 的纯逻辑
// ===========================================================================
//
// 契约出处（逐条行号，便于对着 Core 复核）：
//
// - 命令名 `workspace.replacePreview`：`rust/lithe-core/src/protocol/command.rs:362`
//   （枚举变体 `WorkspaceReplacePreview` 在 `:77-78`）；
// - 请求 `ReplacementPreviewRequest`：`rust/lithe-core/src/project/files.rs:110-136`
//   —— 必填只有 `root` / `query` / `replacement`，其余**全部** `#[serde(default)]`：
//   `caseSensitive` / `wholeWords` / `regularExpression` / `preserveCase` / `fileMask` /
//   `paths` / `textOverrides` / `hiddenDirectoryNames` / `hiddenFilePatterns`（camelCase）；
// - 处理函数 `replace_preview`：`files.rs:336-423`（**只读**：一行都不写盘）；
// - 响应 `ReplacementPreviewResponse { files }`：`protocol/contracts.rs:133-138`；
//   `ReplacementFile { path, matches, replacementText }`：`:124-131`；
//   `ReplacementMatch { line, before, after, occurrenceCount }`：`:114-122`。
//
// ⚠️ **契约里没有的东西**（所以界面也不画）：
//
// 1. **没有 apply / replace 命令**：本分支与替换有关的命令**只有** `workspace.replacePreview`
//    一条（`protocol/command.rs:77-78`）。所以"应用替换"在本分支**做不了**（等 main），
//    宿主也**不许**自己写盘 —— 那会绕开 Core 的排除规则 / 可见性判断，变成第二个真相源；
// 2. **没有上限**：请求里一个 `maxResults` 都没有（对比 `SearchRequest` 的 `files.rs:63`），
//    所以一次查询可能回整个工作区的命中。别在宿主里自己截断（那是第二套上限）；
// 3. **没有总数**：`files` / `matches` 都要宿主自己数（[`ReplaceState::counts`]）；
// 4. **没有列号**：`line` 是 1 基行号（与 `SearchMatch::line` 同一口径），没有列；
// 5. `operationId` 是**信封级**字段（`command.rs:14-15`），不在 payload 里 ——
//    与 [`SEARCH_COMMAND`] 的说明完全同一处。

/// `workspace.replacePreview`（`protocol/command.rs:362`）。
pub(crate) const REPLACE_PREVIEW_COMMAND: &str = "workspace.replacePreview";

/// Replace in Files 的选项：查找面板的四个选项 + `preserveCase`。
///
/// 为什么**包一层**而不是给 [`SearchOptions`] 加一个字段：`preserveCase` 只服务替换
/// （`files.rs:123-125`），而 `SearchRequest` 里根本没有它（`files.rs:47-74`）。
/// 加进 `SearchOptions` 会让 `workspace.search` 的 payload 多出一个 Core 不认的字段。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReplaceOptions {
    /// 复用的四个查询选项（大小写 / 全词 / 正则 / 文件掩码）。
    pub(crate) search: SearchOptions,
    /// `preserveCase`：**只在字面量替换时生效** —— 正则那一支直接返回
    /// （`files.rs:890-901` 的 `replace_with_options`），所以视图把这一格在正则打开时置灰。
    pub(crate) preserve_case: bool,
}

/// `workspace.replacePreview` 的 payload（**字段名是兼容面**）。
///
/// **故意不传** `paths` / `textOverrides`：真源 macOS 传的是编辑器里未保存的缓冲区
/// （`macos/Sources/Lithe/Models/AppModel/AppModel+SearchModule.swift:53-58` 的
/// `paths(root)` / `overrides(root)`），而 gpui 宿主今天没有"未保存缓冲区覆盖"这条通路。
/// 少传时 Core 自己从索引出候选（`files.rs:361-366`）、从磁盘读正文（`:391-396`），
/// 正是本切片要的语义：**预览的是磁盘上的内容**（有意的差异，记在 `UI-MAP.md` §2.2 ⑬）。
///
/// **不传** `hiddenDirectoryNames` / `hiddenFilePatterns`：与 [`payload`] 同一条论证
/// （少一个会漂移的字段，且与 `searchIndex.warm` 共用同一个索引缓存键）。
///
/// **不传上限**：契约里没有这个字段（见本节的契约说明第 2 条）—— 不要自己发明一个。
pub(crate) fn replace_payload(
    root: &str,
    query: &str,
    replacement: &str,
    options: &ReplaceOptions,
) -> Value {
    json!({
        "root": root,
        "query": query,
        "replacement": replacement,
        "caseSensitive": options.search.case_sensitive,
        "wholeWords": options.search.whole_words,
        "regularExpression": options.search.regular_expression,
        "preserveCase": options.preserve_case,
        "fileMask": options.search.file_mask,
    })
}

/// 一行上的替换预览（`contracts.rs:114-122` 的 `ReplacementMatch`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplacementLine {
    /// `line`：**1 基**行号（`files.rs:406` 的 `index + 1`）。
    pub(crate) line: usize,
    /// `before`：Core 给的原行正文（完整一行，不是片段）。
    pub(crate) before: String,
    /// `after`：**Core 算出的**替换后的行正文。宿主一个字符都不改它。
    pub(crate) after: String,
    /// `occurrenceCount`：这一行上被替换的次数（`files.rs:401-410`）。
    pub(crate) occurrences: usize,
}

/// 一个文件的替换预览（`contracts.rs:124-131` 的 `ReplacementFile`）。
///
/// ⚠️ 响应里还有 `replacementText`（整份替换后的文件正文，Core 真的给了），本切片
/// **不解析**它：真源那块是"文件预览编辑器"（`ProjectReplacementSourcePreview.swift:14-45`
/// 挂 Monaco 编辑器 + 保存按钮），gpui 侧没有对应元素，解析一份画不出来的全文只会白占内存。
/// 这是有意的省略，不是漏掉了 —— 见 `UI-MAP.md` §2.2 ⑬。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplacementFile {
    /// `path`：工作区相对、`/` 分隔（Core 已按路径升序排好，`files.rs:421`）。
    pub(crate) path: String,
    /// 这个文件里的命中行（行号升序，Core 给出的顺序）。
    pub(crate) lines: Vec<ReplacementLine>,
}

impl ReplacementFile {
    /// 这个文件里会被替换的总次数。
    ///
    /// **相加而不是重新匹配**：真源的 `matchCount` 就是同一个算法
    /// （`macos/Sources/LitheSearchModule/Models/ProjectReplacementModels.swift:38` 的
    /// `matches.reduce(0) { $0 + $1.occurrenceCount }`）。
    pub(crate) fn occurrences(&self) -> usize {
        self.lines.iter().map(|line| line.occurrences).sum()
    }
}

/// 把 `CoreClient::execute` 的 `data` 解析成按文件分组的替换预览。
///
/// **空结果与失败是两件事**（与 [`parse_matches`] 同一条）：`Ok(vec![])` 是"没有任何命中"，
/// `Err(..)` 是"这条响应不可用"。面板必须把两者画成不同的东西
/// （`lithe.search.noResultsFor` vs `lithe.search.failed`）。
///
/// `occurrenceCount` 缺失按 `0` 算：它只进"这个文件替换了几处"的计数，
/// 不影响这一行画不画（`before` / `after` 才是这一行的内容）。
pub(crate) fn parse_replace_preview(
    data: Option<&Value>,
) -> Result<Vec<ReplacementFile>, ParseError> {
    let Some(data) = data else {
        return Err(ParseError::MissingData);
    };
    let Some(items) = data.get("files").and_then(Value::as_array) else {
        return Err(ParseError::Malformed);
    };
    let mut files = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(path) = item.get("path").and_then(Value::as_str) else {
            return Err(ParseError::MissingPath(index));
        };
        let Some(matches) = item.get("matches").and_then(Value::as_array) else {
            return Err(ParseError::Malformed);
        };
        let mut lines = Vec::with_capacity(matches.len());
        for item in matches {
            let (Some(line), Some(before), Some(after)) = (
                item.get("line").and_then(Value::as_u64),
                item.get("before").and_then(Value::as_str),
                item.get("after").and_then(Value::as_str),
            ) else {
                return Err(ParseError::Malformed);
            };
            lines.push(ReplacementLine {
                line: line as usize,
                before: before.to_string(),
                after: after.to_string(),
                occurrences: item
                    .get("occurrenceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
            });
        }
        files.push(ReplacementFile {
            path: path.to_string(),
            lines,
        });
    }
    Ok(files)
}

/// 替换预览列表的一行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReplaceRow {
    /// 文件头：这个文件里有几处会被替换。
    File {
        /// 工作区相对路径（`/` 分隔）。
        path: String,
        /// 这个文件里会被替换的总次数。
        occurrences: usize,
    },
    /// 一条 before / after。
    Line {
        /// 与它上面那个文件头同值。
        path: String,
        /// 1 基行号。
        line: usize,
        /// 原行正文（Core 给的）。
        before: String,
        /// 替换后的行正文（Core 给的）。
        after: String,
        /// 这一行的替换次数。
        occurrences: usize,
    },
}

/// 把 Core 的分组摊成"文件头 + 命中行"。
///
/// **不重排、不重分组、不重算**：Core 已经给出 `files`（路径升序，`files.rs:421`）
/// 与每个文件里的行号升序（`:397-412`）。本函数只做展示需要的摊平，与 [`group`]
/// 同一条口径 —— 而且要的是"**按文件**的 before/after"（真源同样按文件列，
/// `ProjectReplaceView.swift:217-224` 是 `ForEach(files) { ForEach(matches) }`）。
pub(crate) fn replace_rows(files: &[ReplacementFile]) -> Vec<ReplaceRow> {
    let mut rows =
        Vec::with_capacity(files.len() + files.iter().map(|f| f.lines.len()).sum::<usize>());
    for file in files {
        rows.push(ReplaceRow::File {
            path: file.path.clone(),
            occurrences: file.occurrences(),
        });
        rows.extend(file.lines.iter().map(|line| ReplaceRow::Line {
            path: file.path.clone(),
            line: line.line,
            before: line.before.clone(),
            after: line.after.clone(),
            occurrences: line.occurrences,
        }));
    }
    rows
}

/// Replace in Files 的四态（与 [`SearchState`] 同一份契约：`idle` / `loading` / `ready` /
/// `failed`）。
///
/// **为什么不复用 [`SearchState`]**：它的 `ready` 装的是查找面板的 [`SearchRow`]
/// （`kind` 分出来的路径命中 / 正文命中），而这一层要留着 Core 的 `files`
/// —— before/after 是**成对的**、每个文件还带一个替换次数，行是渲染时现投影
/// （与 [`EverywhereState`] 同一取舍）。四态的语义逐条一致，尤其
/// "**空结果是 `Ready`，不是 `Failed`**"。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReplaceState {
    /// 还没有查询串（或查询串只有空白）：画空态，**不发 Core 调用**
    /// （`files.rs:341-344` 对空查询直接回空 —— 发过去只是白跑一趟信封）。
    Idle,
    /// 已发出预览请求、结果还没回来；`files` 是**上一次**的预览，继续可见
    /// （真源同样在结果到达前不清空 `projectReplacementFiles`，
    /// `ProjectReplaceView.swift:207-259`）。
    Loading {
        /// 这一次预览的查询串。
        query: String,
        /// 上一次的预览（可能为空）。
        files: Vec<ReplacementFile>,
    },
    /// 结果到了；`files` 可能是空的（"没有任何地方会变"）。
    Ready {
        /// 这一次预览的查询串。
        query: String,
        /// Core 给的分组。
        files: Vec<ReplacementFile>,
    },
    /// 失败了；`reason` 是可直接显示的原因（Core 的 `message` 或 [`ParseError::token`]）。
    Failed {
        /// 这一次预览的查询串。
        query: String,
        /// 可直接显示的原因。
        reason: String,
    },
}

impl ReplaceState {
    /// 进 `loading` 态（带上一次的预览）。
    pub(crate) fn loading(query: impl Into<String>, previous: &[ReplacementFile]) -> Self {
        Self::Loading {
            query: query.into(),
            files: previous.to_vec(),
        }
    }

    /// 进 `ready` 态（空 `files` = 空结果，不是失败）。
    pub(crate) fn ready(query: impl Into<String>, files: &[ReplacementFile]) -> Self {
        Self::Ready {
            query: query.into(),
            files: files.to_vec(),
        }
    }

    /// 进 `failed` 态。
    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    /// 当前查询串（`idle` 时为空串）。
    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query, .. }
            | Self::Ready { query, .. }
            | Self::Failed { query, .. } => query,
        }
    }

    /// 手上这一份 Core 分组（`failed` / `idle` 为空）。
    ///
    /// `loading` **也算有**：上一次的预览在新结果到达前继续可见（见该变体）。
    pub(crate) fn files(&self) -> &[ReplacementFile] {
        match self {
            Self::Loading { files, .. } | Self::Ready { files, .. } => files,
            Self::Idle | Self::Failed { .. } => &[],
        }
    }

    /// 要画的行（**纯投影**，每次渲染现算 —— 与 [`EverywhereState::rows`] 同构）。
    pub(crate) fn rows(&self) -> Vec<ReplaceRow> {
        replace_rows(self.files())
    }

    /// `(文件数, 替换次数)`：真源底栏那句 "N files, M matches" 的两个数
    /// （`ProjectReplaceView.swift:172`；`M` 是每个文件 `matchCount` 之和）。
    ///
    /// 给诊断行与底栏用 —— 这是这一件唯一的机器可核对锚点。**不是** `maxResults` 那种上限。
    pub(crate) fn counts(&self) -> (usize, usize) {
        let files = self.files();
        (
            files.len(),
            files.iter().map(ReplacementFile::occurrences).sum(),
        )
    }
}

/// 一次排定的查询：调用方拿这三个值去发 Core 调用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scheduled {
    /// 这一次查询的代次（结果回来时要原样交回 [`QueryTimeline::accepts`]）。
    pub(crate) generation: u64,
    /// 这一次查询的 `operationId`（信封级；同时是 Core 的取消句柄）。
    pub(crate) operation_id: String,
    /// **被这次取代**的在飞请求的 `operationId`：调用方要把它交给
    /// [`lithe_gpui_shared::cancel_operation`]，让那一次扫描尽早停下。
    pub(crate) superseded: Option<String>,
}

/// 查询代次：**stale 结果保护与"取消/取代前一次"的唯一判据**。
///
/// 形状与 `crate::search_index::SearchIndexWiring` 的代次闸门同源（那里管"索引状态属于哪个根"，
/// 这里管"结果属于哪一次查询"），只是这里的换代时机是**输入变化**而不是换根。
///
/// 两条不变量：
///
/// 1. 代次**只增不减**，`operationId` 由代次拼出来，所以同一次会话里不重复；
/// 2. 结果只有在 `accepts(issued)` 为真时才采纳 —— 旧查询的结果回来时**一律丢弃**
///    （打一行 `stale=1`，不采纳）。这一条不依赖"取消真的生效"：取消是尽力而为的
///    优化，代次闸门才是正确性保证（Core 的 `check()` 是协作式的，
///    见 `protocol/cancellation.rs:96`）。
#[derive(Debug, Default)]
pub(crate) struct QueryTimeline {
    generation: u64,
    /// 当前在飞请求的 `operationId`；结果落地或被放弃后清空。
    in_flight: Option<String>,
}

impl QueryTimeline {
    /// 排一次新查询：换代（旧结果从此一律丢弃），并把在飞的旧 `operationId` 交回来。
    pub(crate) fn schedule(&mut self) -> Scheduled {
        let superseded = self.in_flight.take();
        self.generation = self.generation.wrapping_add(1);
        let operation_id = operation_id(self.generation);
        self.in_flight = Some(operation_id.clone());
        Scheduled {
            generation: self.generation,
            operation_id,
            superseded,
        }
    }

    /// 这条完成于 `issued` 代次的结果还算不算当前查询。
    pub(crate) fn accepts(&self, issued: u64) -> bool {
        self.generation == issued
    }

    /// 当前代次的请求已经落地（成功或失败）：不再需要取消它。
    ///
    /// 只清当前代次的那一条：旧代次的 `in_flight` 早就在换代时被取走了。
    pub(crate) fn settle(&mut self, issued: u64) {
        if self.accepts(issued) {
            self.in_flight = None;
        }
    }

    /// 放弃当前查询（清空输入 / 面板被换掉）：换代（在飞结果从此作废）并把它的
    /// `operationId` 交回来交给 Core 取消。
    pub(crate) fn abandon(&mut self) -> Option<String> {
        self.generation = self.generation.wrapping_add(1);
        self.in_flight.take()
    }
}

/// `operationId` 的形状：代次单调递增，所以同一次会话里不重复。
pub(crate) fn operation_id(generation: u64) -> String {
    format!("lithe-gpui-search-{generation}")
}

/// 工作区相对路径（`/` 分隔）→ 绝对路径。
///
/// **逐段 `push`**，不写 `root.join(text)`：Windows 上 `join("src/a.java")` 恰好也能跑
/// （`/` 也被当分隔符），但那是平台旁路行为；逐段 push 让分隔符语义不依赖它，同时顺手
/// 挡掉空段。Core 的路径来自索引（`relative_path`），这里不做额外的安全假设 ——
/// 不在根之下的路径由 Core 保证不会出现。
pub(crate) fn absolute(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in relative.split('/').filter(|segment| !segment.is_empty()) {
        path.push(segment);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// payload 的字段名是**契约面**（`rename_all = "camelCase"`）：拼错一位 Core 就用
    /// `invalid_request` 拒掉整条命令，而且只有跑起来才知道。这里逐字钉住，并钉住
    /// "**故意不传**那些字段"这件事（它们会改变索引缓存键，见 `payload` 的文档）。
    #[test]
    fn payload_follows_the_core_contract_field_names() {
        let options = SearchOptions {
            case_sensitive: true,
            whole_words: true,
            regular_expression: true,
            file_mask: "*.java, *.kt".to_string(),
        };
        assert_eq!(
            payload("D:/proj", "needle", &options),
            json!({
                "root": "D:/proj",
                "query": "needle",
                "caseSensitive": true,
                "wholeWords": true,
                "regularExpression": true,
                "maxResults": 200,
                "fileMask": "*.java, *.kt",
            })
        );
    }

    /// 三个选项默认全关、掩码默认空：默认 payload 与 Core 的默认行为逐字段一致。
    #[test]
    fn default_options_match_the_core_defaults() {
        let value = payload("r", "q", &SearchOptions::default());
        assert_eq!(value["caseSensitive"], json!(false));
        assert_eq!(value["wholeWords"], json!(false));
        assert_eq!(value["regularExpression"], json!(false));
        assert_eq!(value["fileMask"], json!(""));
        assert_eq!(value["maxResults"], json!(MAX_RESULTS));
    }

    /// 可见性与三个 `max*Results` **不出现在** payload 里：
    /// 前者由 Core 的内置规则兜底（并且保证与 `searchIndex.warm` 同一个缓存键），
    /// 后者留空让 Core 回落到 `maxResults`。
    #[test]
    fn payload_omits_fields_that_would_drift_or_split_the_index_cache_key() {
        let value = payload("r", "q", &SearchOptions::default());
        for key in [
            "hiddenDirectoryNames",
            "hiddenFilePatterns",
            "maxFileResults",
            "maxContentResults",
            "maxSymbolResults",
        ] {
            assert!(value.get(key).is_none(), "payload 里不该有 {key}：{value}");
        }
        // 上限是显式传的（不是靠 Core 的 default），这是"面板说得清上限"的前提。
        assert_eq!(value["maxResults"], json!(MAX_RESULTS));
    }

    /// 正常响应：四种 `kind`、`line` 的 `null`、`symbolName` 的保留与缺失、以及
    /// **未认识的 kind 不丢条目**。
    #[test]
    fn parses_the_four_match_shapes_and_keeps_unknown_kinds() {
        let data = json!({
            "matches": [
                { "kind": "file", "path": "src/A.java", "line": null, "preview": "src/A.java" },
                { "kind": "content", "path": "src/A.java", "line": 12, "preview": "class A {" },
                { "kind": "type", "path": "src/A.java", "line": 1, "preview": "class A", "symbolName": "A" },
                { "kind": "symbol", "path": "src/B.java", "line": 3, "preview": "void b()", "symbolName": "b" },
                { "kind": "future", "path": "src/C.java", "line": 1, "preview": "x" },
            ]
        });
        let matches = parse_matches(Some(&data)).expect("必须解析成功");
        assert_eq!(
            matches.len(),
            5,
            "不认识的 kind 也要留下（丢的是归类，不是条目）"
        );
        assert_eq!(matches[0].kind, MatchKind::File);
        assert_eq!(matches[0].line, None);
        assert_eq!(matches[0].symbol_name, None, "路径命中没有符号名");
        assert_eq!(matches[1].kind, MatchKind::Content);
        assert_eq!(matches[1].line, Some(12));
        assert_eq!(matches[2].kind, MatchKind::Type);
        assert_eq!(matches[2].symbol_name.as_deref(), Some("A"), "类型名要留下");
        assert_eq!(matches[3].kind, MatchKind::Symbol);
        assert_eq!(
            matches[3].symbol_name.as_deref(),
            Some("b"),
            "符号行画的是 symbolName（真源 :376），丢了就画不出名字"
        );
        assert_eq!(matches[4].kind, MatchKind::Other("future".to_string()));
        assert_eq!(matches[1].preview, "class A {");
    }

    /// `ok:true` 但没有 `data`、`matches` 不是数组、元素缺 `path` —— 三种都要**报错**
    /// （不能悄悄当成"没有命中"），而且 token 稳定。
    #[test]
    fn contract_anomalies_are_reported_not_silently_read_as_no_matches() {
        assert_eq!(parse_matches(None), Err(ParseError::MissingData));
        assert_eq!(
            parse_matches(Some(&json!({ "total": 3 }))),
            Err(ParseError::Malformed)
        );
        assert_eq!(
            parse_matches(Some(&json!({ "matches": {} }))),
            Err(ParseError::Malformed)
        );
        assert_eq!(
            parse_matches(Some(
                &json!({ "matches": [ { "kind": "content", "preview": "x" } ] })
            )),
            Err(ParseError::MissingPath(0))
        );
        assert_eq!(
            parse_matches(Some(
                &json!({ "matches": [ { "path": "a.java", "preview": "x" } ] })
            )),
            Err(ParseError::Malformed)
        );

        assert_eq!(ParseError::MissingData.token(), "missing_data");
        assert_eq!(ParseError::Malformed.token(), "malformed_response");
        assert_eq!(ParseError::MissingPath(7).token(), "missing_path_at_7");
    }

    /// **空结果是 `Ok(vec![])`，不是错误** —— 这是"空结果与失败必须区分"的判据。
    #[test]
    fn an_empty_match_list_is_a_successful_empty_result() {
        assert_eq!(
            parse_matches(Some(&json!({ "matches": [] }))),
            Ok(Vec::new())
        );
        let state = SearchState::ready("needle", &[]);
        assert_eq!(state.counts(), (0, 0));
        assert!(state.rows().is_empty());
        assert!(matches!(state, SearchState::Ready { .. }), "{state:?}");
    }

    fn content(path: &str, line: usize, preview: &str) -> Match {
        Match {
            kind: MatchKind::Content,
            path: path.to_string(),
            line: Some(line),
            preview: preview.to_string(),
            symbol_name: None,
        }
    }

    fn path_match(path: &str) -> Match {
        Match {
            kind: MatchKind::File,
            path: path.to_string(),
            line: None,
            preview: path.to_string(),
            symbol_name: None,
        }
    }

    fn named(kind: MatchKind, name: &str, path: &str, line: usize) -> Match {
        Match {
            kind,
            path: path.to_string(),
            line: Some(line),
            preview: format!("{name} …"),
            symbol_name: Some(name.to_string()),
        }
    }

    /// 分组：文件头 + 命中行；文件顺序 = **第一次出现**的顺序（不重排）；
    /// 文件内顺序 = Core 给出的顺序。
    #[test]
    fn grouping_keeps_the_core_order_and_folds_it_by_file() {
        let rows = group(&[
            content("b.java", 10, "ten"),
            content("a.java", 3, "three"),
            content("b.java", 2, "two"),
        ]);
        assert_eq!(
            rows,
            vec![
                SearchRow::File {
                    path: "b.java".to_string(),
                    hits: 2
                },
                SearchRow::Hit {
                    path: "b.java".to_string(),
                    line: Some(10),
                    preview: "ten".to_string()
                },
                SearchRow::Hit {
                    path: "b.java".to_string(),
                    line: Some(2),
                    preview: "two".to_string()
                },
                SearchRow::File {
                    path: "a.java".to_string(),
                    hits: 1
                },
                SearchRow::Hit {
                    path: "a.java".to_string(),
                    line: Some(3),
                    preview: "three".to_string()
                },
            ],
            "文件按第一次出现的顺序、文件内按 Core 的顺序，都不许重排"
        );
    }

    /// 只有路径命中的文件得到一个 `hits: 0` 的文件头；同一路径的路径命中 + 内容命中
    /// **只出现一个文件头**（否则界面上会出现两个同名分组）。
    #[test]
    fn a_path_match_and_content_matches_share_one_file_header() {
        let rows = group(&[
            path_match("src/A.java"),
            content("src/A.java", 5, "five"),
            path_match("src/Other.java"),
        ]);
        assert_eq!(
            rows,
            vec![
                SearchRow::File {
                    path: "src/A.java".to_string(),
                    hits: 1
                },
                SearchRow::Hit {
                    path: "src/A.java".to_string(),
                    line: Some(5),
                    preview: "five".to_string()
                },
                SearchRow::File {
                    path: "src/Other.java".to_string(),
                    hits: 0
                },
            ]
        );
    }

    /// 结果里的文件头是**展示分组**，而 `counts` 报的也是这两个数 —— 诊断行依赖它。
    #[test]
    fn counts_report_files_and_hit_rows() {
        let state = SearchState::ready(
            "q",
            &[path_match("a"), content("a", 1, "x"), content("b", 2, "y")],
        );
        assert_eq!(state.counts(), (2, 2));
        assert_eq!(state.query(), "q");
        assert_eq!(SearchState::Idle.query(), "");
        assert!(SearchState::Idle.rows().is_empty());
        assert_eq!(SearchState::failed("q", "invalid_request").query(), "q");
    }

    /// 代次闸门：换代之后旧代次的结果**必须**被丢弃；当前代次的仍然采纳。
    #[test]
    fn a_superseded_generation_is_not_adopted() {
        let mut timeline = QueryTimeline::default();
        let first = timeline.schedule();
        assert_eq!(first.generation, 1);
        assert_eq!(first.operation_id, "lithe-gpui-search-1");
        assert_eq!(first.superseded, None);
        assert!(timeline.accepts(first.generation));

        let second = timeline.schedule();
        assert_eq!(second.generation, 2);
        assert_eq!(second.operation_id, "lithe-gpui-search-2");
        assert_eq!(
            second.superseded.as_deref(),
            Some("lithe-gpui-search-1"),
            "换代要把上一条在飞请求交出来取消"
        );
        assert!(!timeline.accepts(first.generation), "旧代次不能被采纳");
        assert!(timeline.accepts(second.generation));
    }

    /// 落地之后不再重复交出同一个 `operationId`；换代后旧的那条也不再被取消。
    #[test]
    fn a_settled_query_is_no_longer_handed_back_for_cancellation() {
        let mut timeline = QueryTimeline::default();
        let first = timeline.schedule();
        timeline.settle(first.generation);
        let second = timeline.schedule();
        assert_eq!(second.superseded, None, "已经落地的请求不需要取消");

        // 旧代次的 `settle` 不能清掉当前在飞的那一条。
        timeline.settle(first.generation);
        let third = timeline.schedule();
        assert_eq!(third.superseded.as_deref(), Some("lithe-gpui-search-2"));
    }

    /// 放弃（清空输入）：换代并把在飞请求交出来取消；之后再放弃就没有可取消的了。
    #[test]
    fn abandoning_a_query_supersedes_it_and_returns_the_operation_to_cancel() {
        let mut timeline = QueryTimeline::default();
        let scheduled = timeline.schedule();
        assert_eq!(
            timeline.abandon().as_deref(),
            Some(scheduled.operation_id.as_str())
        );
        assert!(!timeline.accepts(scheduled.generation));
        assert_eq!(timeline.abandon(), None, "没有在飞请求时不该凭空造一个");
    }

    /// `operationId` 由代次拼出：单调、不重复（Core 用它做取消与陈旧判定的句柄）。
    #[test]
    fn operation_ids_are_unique_per_generation() {
        let mut timeline = QueryTimeline::default();
        let mut seen = Vec::new();
        for _ in 0..5 {
            let scheduled = timeline.schedule();
            assert_eq!(scheduled.operation_id, operation_id(scheduled.generation));
            assert!(!seen.contains(&scheduled.operation_id), "{seen:?}");
            seen.push(scheduled.operation_id);
        }
    }

    /// 相对路径 → 绝对路径：`/` 分段、忽略空段（`a//b`）。
    /// 用 `join` 拼期望值，测试在 Windows / Unix 上都成立。
    #[test]
    fn relative_paths_become_absolute_by_segments() {
        let root = Path::new("proj-root");
        assert_eq!(
            absolute(root, "src/main/A.java"),
            root.join("src").join("main").join("A.java")
        );
        assert_eq!(
            absolute(root, "//src//A.java"),
            root.join("src").join("A.java")
        );
        assert_eq!(absolute(root, ""), root.to_path_buf());
    }

    // ---- Quick Open（`workspace.searchEverywhere` 的 Files scope） ----

    /// 两条命令是**同一个 payload 构造器**：Quick Open 换的只是命令名，请求形状一份。
    /// （`files.rs:272` 的 `search_everywhere(request: SearchRequest)` 收的就是
    /// `workspace.search` 那个类型。）
    #[test]
    fn quick_open_reuses_the_same_request_shape_as_find_in_files() {
        // 默认选项 = 不区分大小写 / 不整词 / 不正则 / 无掩码：真机 Quick Open 也没有这三个开关。
        assert_eq!(
            payload("D:/proj", "needle", &SearchOptions::default()),
            json!({
                "root": "D:/proj",
                "query": "needle",
                "caseSensitive": false,
                "wholeWords": false,
                "regularExpression": false,
                "maxResults": MAX_RESULTS,
                "fileMask": "",
            })
        );
        // 命令名是**兼容面**（`protocol/command.rs:360-361`）：拼错一位 Core 就用
        // `unknown_command` 拒掉，只有跑起来才发现。
        assert_eq!(SEARCH_COMMAND, "workspace.search");
        assert_eq!(EVERYWHERE_COMMAND, "workspace.searchEverywhere");
        assert_ne!(EVERYWHERE_COMMAND, SEARCH_COMMAND);
        // 上限与真机 `SearchEverywhereResults.matchLimit = 200` 同值
        // （`macos/Sources/LitheSearchModule/Models/SearchResults.swift:58`）。
        assert_eq!(MAX_RESULTS, 200);
    }

    /// Files scope：**只留路径命中**，`content` / `type` / `symbol` / 未认识的 kind 一律不成行
    /// —— 那四类属于别的 scope（`SearchEverywhereView.swift:130-137`）。
    #[test]
    fn quick_open_keeps_only_path_matches() {
        let matches = vec![
            path_match("src/A.java"),
            content("src/A.java", 12, "class A {"),
            named(MatchKind::Symbol, "b", "src/B.java", 3),
            named(MatchKind::Type, "A", "src/A.java", 1),
            path_match("src/C.kt"),
        ];
        assert_eq!(
            quick_open_rows(&matches),
            vec![
                QuickOpenRow {
                    path: "src/A.java".to_string(),
                    name: "A.java".to_string(),
                    parent: "src".to_string(),
                },
                QuickOpenRow {
                    path: "src/C.kt".to_string(),
                    name: "C.kt".to_string(),
                    parent: "src".to_string(),
                },
            ],
            "只有路径命中成行，且保持 Core 给出的顺序"
        );
    }

    /// 显示口径：文件名 = 最后一段、所在目录 = 前面的部分；根下的文件 `parent` 为空串
    /// （真源 `SearchEverywhereView.swift:446-450` 的 `deletingLastPathComponent`）。
    /// 深层路径只在**第一个** `/` 处留下全部前缀，不截断成"最后两段"。
    #[test]
    fn quick_open_splits_name_and_container_directory() {
        let matches = vec![
            path_match("A.java"),
            path_match("src/deep/nested/B.java"),
            path_match("src/C.java"),
        ];
        let rows = quick_open_rows(&matches);
        assert_eq!(rows[0].name, "A.java");
        assert_eq!(rows[0].parent, "", "根下的文件没有目录段");
        assert_eq!(rows[1].name, "B.java");
        assert_eq!(rows[1].parent, "src/deep/nested");
        assert_eq!(rows[2].name, "C.java");
        assert_eq!(rows[2].parent, "src");
    }

    /// 同一路径只占一行（保留第一次出现的那条）；`path` 是这一行的身份。
    #[test]
    fn quick_open_rows_are_unique_by_path() {
        let rows = quick_open_rows(&[path_match("src/A.java"), path_match("src/A.java")]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "src/A.java");
    }

    /// 四态：空结果是 `Ready`（0 行）而不是 `Failed`；`Loading` 与 `Ready { rows: [] }`
    /// 必须分得开（`Command` 只在非 loading 时画空态），而且 `Loading` **带着上一次的行**
    /// —— 搜索期间旧结果继续可见。
    #[test]
    fn quick_open_empty_result_is_ready_not_failed() {
        let empty = QuickOpenState::ready("zzz", &[]);
        assert!(matches!(empty, QuickOpenState::Ready { .. }), "{empty:?}");
        assert!(empty.rows().is_empty());
        assert_eq!(empty.query(), "zzz");
        assert_eq!(QuickOpenState::Idle.query(), "");
        assert_eq!(QuickOpenState::Idle.rows(), &[]);

        let previous = quick_open_rows(&[path_match("src/A.java")]);
        let loading = QuickOpenState::loading("q", &previous);
        assert_eq!(loading.query(), "q");
        assert_eq!(
            loading.rows(),
            previous.as_slice(),
            "在路上 时上一次的结果要继续可见（真机不清空 visibleItems）"
        );
        assert_ne!(
            loading,
            QuickOpenState::Ready {
                query: "q".to_string(),
                rows: Vec::new(),
            },
            "在路上 与 没有匹配 是两个状态"
        );
        assert_eq!(QuickOpenState::failed("q", "invalid_request").query(), "q");
        assert_eq!(
            QuickOpenState::failed("q", "invalid_request").rows(),
            &[],
            "失败态没有行：整块换成原因"
        );
    }

    /// `SearchEverywhere` 也会回符号命中：`quick_open_rows` 丢掉它们，但
    /// `parse_matches` 照旧**一条都不丢**（解析层不因为不认识就扔条目）。
    #[test]
    fn quick_open_parses_the_everywhere_response_without_losing_symbol_rows() {
        let data = json!({
            "matches": [
                { "kind": "file", "path": "src/A.java", "line": null, "preview": "src/A.java" },
                { "kind": "symbol", "path": "src/A.java", "line": 3, "preview": "void a()", "symbolName": "a" },
            ]
        });
        let matches = parse_matches(Some(&data)).expect("必须解析成功");
        assert_eq!(matches.len(), 2);
        assert_eq!(quick_open_rows(&matches).len(), 1);
    }

    // ---- Search Everywhere（五个 Core scope 的投影 + Actions 档；全是纯函数，无时钟 / 无 I/O） ----

    /// 一份典型的 `workspace.searchEverywhere` 响应，顺序就是 Core 的顺序
    /// （`files.rs:326-330`：file → types → symbols → content）。
    fn everywhere_fixture() -> Vec<Match> {
        vec![
            path_match("src/A.java"),
            named(MatchKind::Type, "Alpha", "src/A.java", 1),
            named(MatchKind::Symbol, "alphaRun", "src/A.java", 9),
            content("src/A.java", 12, "class Alpha {"),
            named(MatchKind::Symbol, "alphaOther", "src/B.java", 3),
            content("src/B.java", 4, "  alpha();"),
            path_match("docs/alpha.md"),
        ]
    }

    /// 五个 scope ↔ Core `kind` 的映射逐条钉住；**All 不含 content** 是真源明写的取舍
    /// （`SearchEverywhereView.swift:122-129`），也是本切片最容易写错的一格。
    #[test]
    fn scopes_project_the_core_kinds_exactly() {
        let matches = everywhere_fixture();

        let files = everywhere_rows(SearchEverywhereScope::Files, &matches);
        assert_eq!(
            files
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/A.java", "docs/alpha.md"]
        );
        assert!(files.iter().all(|row| row.kind == MatchKind::File));

        let classes = everywhere_rows(SearchEverywhereScope::Classes, &matches);
        assert_eq!(
            classes
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha"],
            "Classes 只收 kind == type（类 / 接口 / 枚举 / record），且保持 Core 顺序"
        );
        assert!(classes.iter().all(|row| row.kind.is_type()));

        let symbols = everywhere_rows(SearchEverywhereScope::Symbols, &matches);
        assert_eq!(
            symbols
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alphaRun", "alphaOther"],
            "Symbols 只收 kind == symbol，且保持 Core 顺序"
        );

        let text = everywhere_rows(SearchEverywhereScope::Text, &matches);
        assert_eq!(
            text.iter()
                .map(|row| (row.path.as_str(), row.line))
                .collect::<Vec<_>>(),
            vec![("src/A.java", Some(12)), ("src/B.java", Some(4))],
            "Text 只收 kind == content"
        );

        let all = everywhere_rows(SearchEverywhereScope::All, &matches);
        assert_eq!(
            all.iter()
                .map(|row| (row.kind.clone(), row.name.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (MatchKind::File, "A.java"),
                (MatchKind::File, "alpha.md"),
                (MatchKind::Type, "Alpha"),
                (MatchKind::Symbol, "alphaRun"),
                (MatchKind::Symbol, "alphaOther"),
            ],
            "All = file 段 + type 段 + symbol 段（三段拼接、段内保持 Core 顺序）"
        );
        assert!(
            !all.iter().any(|row| row.kind.is_content()),
            "All **不含** content：正文命中只在 Text 标签页出现，避免与 Find in Files 重叠"
        );

        assert!(
            everywhere_rows(SearchEverywhereScope::Actions, &matches).is_empty(),
            "Actions 不是 Core 投影：它的行来自宿主命令目录（`action_rows`），这里恒为空"
        );
    }

    /// 符号行画的是 `symbolName`，路径命中画的是文件名（真源 `:376`）；
    /// `:行号` 与预览的显示判据同源（同文件 `:349,392`）。
    #[test]
    fn symbol_rows_carry_the_symbol_name_and_the_line() {
        let matches = everywhere_fixture();
        let symbols = everywhere_rows(SearchEverywhereScope::Symbols, &matches);
        assert_eq!(symbols[0].name, "alphaRun");
        assert_eq!(symbols[0].parent, "src");
        assert_eq!(symbols[0].line, Some(9));
        assert!(symbols[0].shows_line(), "符号行要画 :行号");
        assert!(!symbols[0].shows_preview(), "符号行不画正文预览");

        let files = everywhere_rows(SearchEverywhereScope::Files, &matches);
        assert_eq!(files[0].name, "A.java", "路径命中没有符号名，回落到文件名");
        assert_eq!(files[0].parent, "src");
        assert_eq!(files[1].parent, "docs");
        assert!(!files[0].shows_line(), "路径命中不画 :行号");
        assert_eq!(files[0].line, None);

        let text = everywhere_rows(SearchEverywhereScope::Text, &matches);
        assert_eq!(text[0].name, "A.java");
        assert!(text[0].shows_line());
        assert!(text[0].shows_preview(), "只有正文行画预览");
    }

    /// 空结果与失败是**两个状态**：`ready` + 0 命中（每个 scope 都空）≠ `failed`。
    #[test]
    fn an_empty_everywhere_result_is_ready_not_failed() {
        let state = EverywhereState::ready("zzz", &[]);
        assert!(matches!(state, EverywhereState::Ready { .. }), "{state:?}");
        assert_eq!(state.query(), "zzz");
        for scope in SearchEverywhereScope::ALL {
            assert!(state.rows(scope).is_empty(), "{scope:?} 应当是空的");
        }
        assert_eq!(EverywhereState::Idle.query(), "");
        assert_eq!(
            EverywhereState::failed("q", "invalid_request").matches(),
            &[] as &[Match],
            "失败态没有命中：整块换成原因"
        );
    }

    /// `loading` 带着**上一次**的命中继续可见（真机不清空 `visibleItems`，
    /// `SearchEverywhereView.swift:302-328`），且它与"没有匹配"是两个状态。
    #[test]
    fn a_loading_everywhere_state_keeps_the_previous_rows_visible() {
        let previous = everywhere_fixture();
        let loading = EverywhereState::loading("alp", &previous);
        assert_eq!(loading.query(), "alp");
        assert_eq!(loading.matches().len(), previous.len());
        assert_eq!(
            loading.rows(SearchEverywhereScope::Symbols).len(),
            2,
            "在路上 时上一次的结果继续可见"
        );
        assert_ne!(
            loading,
            EverywhereState::Ready {
                query: "alp".to_string(),
                matches: Vec::new(),
            },
            "在路上 与 没有匹配 是两个状态"
        );
    }

    /// `symbolName` 的**缺失**是正常形状（`workspace.search` 恒缺它），不是契约异常；
    /// 但它一旦存在就必须原样留着。
    #[test]
    fn a_missing_symbol_name_is_normal_and_a_present_one_survives() {
        let data = json!({
            "matches": [
                { "kind": "type", "path": "src/A.java", "line": 1, "preview": "class A" },
                { "kind": "type", "path": "src/A.java", "line": 1, "preview": "class A", "symbolName": "A" },
            ]
        });
        let matches = parse_matches(Some(&data)).expect("symbolName 缺失不是错误");
        assert_eq!(matches[0].symbol_name, None);
        assert_eq!(matches[1].symbol_name.as_deref(), Some("A"));
    }

    /// Tab / `←` / `→` 的环绕：`(i + offset) mod 6`，两个方向都要绕回来
    /// （真源 `SearchEverywhereView.swift:542-547`）。
    ///
    /// 同时钉住**标签顺序**：真源 `allCases` 是
    /// `All` / `Classes` / `Files` / `Symbols` / `Actions` / `Text`（`:5-14`），
    /// 本侧六档逐格照抄 —— `Actions` 排**第五位**（下标 4，在 `Symbols` 之后、`Text` 之前）。
    #[test]
    fn scopes_cycle_with_wrap_around_in_both_directions() {
        assert_eq!(SearchEverywhereScope::ALL.len(), 6);
        assert_eq!(
            SearchEverywhereScope::ALL.map(|scope| scope.token()),
            ["all", "classes", "files", "symbols", "actions", "text"],
            "Tab 顺序照真源 allCases（Actions 在 Symbols 之后、Text 之前）"
        );
        assert_eq!(
            SearchEverywhereScope::All.moved(1),
            SearchEverywhereScope::Classes
        );
        assert_eq!(
            SearchEverywhereScope::Classes.moved(1),
            SearchEverywhereScope::Files
        );
        assert_eq!(
            SearchEverywhereScope::Symbols.moved(1),
            SearchEverywhereScope::Actions
        );
        assert_eq!(
            SearchEverywhereScope::Actions.moved(1),
            SearchEverywhereScope::Text
        );
        assert_eq!(
            SearchEverywhereScope::Text.moved(1),
            SearchEverywhereScope::All
        );
        assert_eq!(
            SearchEverywhereScope::Text.moved(-1),
            SearchEverywhereScope::Actions
        );
        assert_eq!(
            SearchEverywhereScope::All.moved(-1),
            SearchEverywhereScope::Text
        );
        assert_eq!(
            SearchEverywhereScope::All.moved(-7),
            SearchEverywhereScope::Text,
            "负偏移绕圈要落在合法档位上（rem_euclid，不是裸取模）"
        );
        assert_eq!(
            SearchEverywhereScope::Files.moved(6),
            SearchEverywhereScope::Files,
            "整圈回到原处"
        );
    }

    /// `Classes` 与 `Symbols` **不是同一个 kind 的两个视图**：Core 的符号扫描造两种 `kind`
    /// （`files.rs:976-981` 造 `"type"`、`:993-998` 造 `"symbol"`），所以 `Classes` 只出
    /// `kind == "type"`、`Symbols` 只出 `kind == "symbol"`，两边取值域不重叠；而 `All`
    /// 仍然同时含这两段、且仍然**不含 `content`** —— 新增这一档没有改 All 的口径。
    /// 每个 scope 的行数还要与 `All` 的计数对得上（诊断行 `S1_SEARCH_EVERYWHERE complete`
    /// 报的就是这几个数）。
    #[test]
    fn classes_scope_only_projects_type_rows_without_changing_all() {
        let matches = everywhere_fixture();
        let classes = everywhere_rows(SearchEverywhereScope::Classes, &matches);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].kind, MatchKind::Type);
        assert_eq!(classes[0].name, "Alpha");
        assert_eq!(classes[0].parent, "src");
        assert_eq!(classes[0].line, Some(1));
        assert!(
            classes.iter().all(|row| row.kind.is_type()),
            "Classes 的行全部是 kind == type"
        );

        let symbols = everywhere_rows(SearchEverywhereScope::Symbols, &matches);
        assert!(
            symbols.iter().all(|row| row.kind.is_symbol()),
            "Classes 与 Symbols 的 kind 取值域不重叠（一个只收 type、一个只收 symbol）"
        );

        let all = everywhere_rows(SearchEverywhereScope::All, &matches);
        assert!(all.iter().any(|row| row.kind.is_type()), "All 仍然含 type");
        assert!(
            !all.iter().any(|row| row.kind.is_content()),
            "All 仍然不含 content（真源 :123-125 明写的取舍）"
        );
        assert_eq!(
            all.len(),
            everywhere_rows(SearchEverywhereScope::Files, &matches).len()
                + classes.len()
                + symbols.len(),
            "All = Files 段 + Classes 段 + Symbols 段"
        );
    }

    // ---- Actions 档 / `/` 命令模式（U17 后续切片；候选目录在宿主，不在 Core）----
    //
    // 这一组全是纯函数、无时钟、无 I/O、**不依赖语言**：合成目录里的 label / description
    // 都是测试字面量（`action_rows` 会去取分类名，而那是 `tr(..)` 出来的 —— 所以断言只挑
    // 与语言无关的查询串，见每条测试的说明）。

    /// 一份**合成**的宿主命令目录（3 条，字段都是测试字面量）。
    ///
    /// 用合成数据而不是真源文案：`action_rows` 的过滤要读 `label` / `description` / 分类名，
    /// 真源文案会随 locale 变，测试就会"在英文下过、在中文下挂"。
    fn action_fixture() -> Vec<CommandAction> {
        use gpui_kit::SharedString;
        use gpui_kit::component::Icon;

        use crate::command_palette::{Category, CommandId};

        let entry = |id, category, label: &str, description: &str| CommandAction {
            id,
            category,
            label: SharedString::from(label),
            description: SharedString::from(description),
            icon: Icon::default(),
        };
        vec![
            entry(
                CommandId::OpenSettings,
                Category::Settings,
                "Open Settings",
                "Configure editor",
            ),
            entry(
                CommandId::ToggleTerminal,
                Category::View,
                "Toggle Terminal",
                "Show or hide terminal",
            ),
            entry(
                CommandId::SaveBuffer,
                Category::View,
                "Save Buffer",
                "Write the active buffer",
            ),
        ]
    }

    /// 真源 `LitheAction.matches`（`LitheAction.swift:37-51`）的五类用例：
    /// 大小写、空白、子序列、非子序列、空 query。
    #[test]
    fn action_matcher_is_a_case_insensitive_whitespace_stripped_subsequence() {
        // 1. 大小写不敏感（两边都 lowercased）。
        assert!(action_matches("OPENSET", "Open Settings", "", ""));
        assert!(action_matches("openseT", "Open Settings", "", ""));
        assert!(action_matches("open", "OPEN", "", ""));
        // 2. 查询串里的空白被去掉（`filter { !$0.isWhitespace }`）。
        assert!(action_matches("open set", "Open Settings", "", ""));
        assert!(action_matches("  o p e n  ", "Open Settings", "", ""));
        // 3. 子序列（不要求连续、不要求词边界）。
        assert!(action_matches("ose", "Open Settings", "", ""));
        assert!(action_matches("tgle", "Toggle Terminal", "", ""));
        // 4. 非子序列：缺字符 / 顺序反了 / 多一个字符。
        assert!(!action_matches("openz", "Open Settings", "", ""));
        assert!(
            !action_matches("gnittes", "Open Settings", "", ""),
            "顺序必须一致：反过来不算命中"
        );
        assert!(
            !action_matches("openq", "Open Settings", "", ""),
            "候选串里没有的字符永远不命中（`openn` 反而**会**命中：`settings` 里还有 n）"
        );
        // 5. 空 query 恒真（含全是空白的 query）—— `/` 后面什么都不写时列出全部命令。
        assert!(action_matches("", "Open Settings", "", ""));
        assert!(action_matches("   ", "Open Settings", "", ""));
        assert!(action_matches("\t\n", "any", "thing", "here"));

        // 候选串是 `title + subtitle + group` 三段拼接（真源 `searchText`，`:35`）：
        // 只写在 subtitle / group 里的词同样要能命中。
        assert!(action_matches("configure", "Open", "Configure editor", ""));
        assert!(action_matches("editor", "Open", "Configure editor", ""));
        assert!(action_matches("navigation", "Back", "", "Navigation"));
        assert!(
            !action_matches("navigation", "Back", "", ""),
            "group 真的进了候选串：去掉它之后同一个查询就不命中"
        );
        assert!(
            !action_matches("editor", "Open", "", ""),
            "subtitle 真的进了候选串：去掉它之后同一个查询就不命中"
        );
    }

    /// Actions 档的行 = 宿主目录按真源规则过滤后的那一段：**顺序照目录**、
    /// 空 query 恒真、命中的一定是子序列。
    #[test]
    fn action_rows_keep_the_catalog_order_and_the_source_matching() {
        let ids = |rows: &[CommandAction]| {
            rows.iter()
                .map(|action| action.id.id().to_string())
                .collect::<Vec<_>>()
        };

        // 空 query：三条全在，顺序 = 目录顺序（真源 `actionMatches` 也只 `.map`，不排序）。
        assert_eq!(
            ids(&action_rows(action_fixture(), "")),
            ["open-settings", "toggle-terminal", "save-buffer"]
        );

        // 命中的那条留在原位；`opense` 只可能是 "Open Settings"（另外两条的候选串里
        // 没有能在 `o` 之后接上 `p` 的字符，与分类名是中文还是英文无关）。
        assert_eq!(
            ids(&action_rows(action_fixture(), "opense")),
            ["open-settings"],
            "过滤不许改变顺序、也不许命中别的条目"
        );

        // 没有命中就是空表（不是"退回全部"）。
        assert!(action_rows(action_fixture(), "zzzz").is_empty());

        // 行型就是宿主目录的那一条本身：label / description / 分类都不复制、不重写。
        // 查询串刻意挑 `buffer`：它是第三条独有的词（子序列匹配会跨词、跨 subtitle 与
        // 分类名，所以"看起来只属于某一条"的词不一定真的只命中那一条 —— 例如 `save`
        // 也会被 "Toggle Terminal Show or hide terminal View" 命中）。
        let rows = action_rows(action_fixture(), "buffer");
        assert_eq!(rows.len(), 1, "只有 Save Buffer 的候选串里有 b…u…f…f…e…r");
        assert_eq!(rows[0].label.as_ref(), "Save Buffer");
        assert_eq!(rows[0].description.as_ref(), "Write the active buffer");
        assert_eq!(rows[0].id.id(), "save-buffer");
    }

    /// `/` 命令模式的判定在**原始**查询串上（真源 `hasPrefix("/")` → `dropFirst()`）。
    #[test]
    fn command_mode_is_decided_on_the_raw_query_before_the_scope() {
        assert!(is_command_query("/"));
        assert!(is_command_query("/open"));
        assert!(is_command_query("/ open"));
        assert!(!is_command_query("open"), "没有斜杠就是工作区查询");
        assert!(
            !is_command_query(" open"),
            "真源不去空白：前导空格之后就不再是命令模式"
        );
        assert!(!is_command_query(""));

        assert_eq!(command_query("/open"), Some("open"));
        assert_eq!(
            command_query("/"),
            Some(""),
            "只有斜杠时命令查询串是空的 ⇒ 空 query 恒真 ⇒ 列出全部命令"
        );
        assert_eq!(command_query("open"), None);
        assert_eq!(command_query(""), None);
    }

    /// **"Actions 档不发 Core"** 的纯函数判据：`Actions` 档（任何查询串）与 `/` 命令模式
    /// （任何 scope）都不发 Core 请求，其余五档 + 非空非 `/` 查询才发。
    #[test]
    fn actions_and_command_mode_never_issue_a_core_request() {
        // 只有 Actions 一档不是 Core 档。
        assert!(!SearchEverywhereScope::Actions.is_core_backed());
        assert_eq!(
            SearchEverywhereScope::ALL
                .iter()
                .filter(|scope| !scope.is_core_backed())
                .count(),
            1,
            "六个档里只有 Actions 走宿主命令目录"
        );

        // 1. Actions 档：**任何**查询串都不发（含非空、含 `/`、含空白）。
        for query in ["", "open", "/open", "   ", "open settings"] {
            assert!(
                !issues_core_request(SearchEverywhereScope::Actions, query),
                "Actions 档不许发 Core 请求（query={query:?}）"
            );
            assert!(
                uses_command_catalog(SearchEverywhereScope::Actions, query),
                "Actions 档的行一律来自宿主命令目录（query={query:?}）"
            );
        }

        // 2. `/` 命令模式：**任何** scope 下都不发（真源把 `/` 判在 scope 之前）。
        for scope in SearchEverywhereScope::ALL {
            for query in ["/", "/open", "/open settings"] {
                assert!(
                    !issues_core_request(scope, query),
                    "{scope:?} 在 `/` 模式下不许发 Core 请求（query={query:?}）"
                );
                assert!(uses_command_catalog(scope, query));
            }
        }

        // 3. 其余五档：非空、非 `/` 查询才发；空查询与全空白不发。
        for scope in SearchEverywhereScope::ALL {
            if !scope.is_core_backed() {
                continue;
            }
            assert!(issues_core_request(scope, "open"), "{scope:?}");
            assert!(issues_core_request(scope, "  open  "), "{scope:?}");
            assert!(!issues_core_request(scope, ""), "{scope:?} 空查询不发");
            assert!(!issues_core_request(scope, "   "), "{scope:?} 全空白不发");
            assert!(!uses_command_catalog(scope, "open"));
            assert!(
                !uses_command_catalog(scope, " "),
                "空查询是 Core 档自己的 idle 态，**不是**命令目录"
            );
        }
    }

    /// `Actions` 档的行与 Core 命中**完全无关**：Core 有命中时它也是空的，而且它不影响
    /// 其余五档的投影（`All` 仍是 file + type + symbol、仍不含 content）。
    #[test]
    fn the_actions_scope_never_projects_core_matches() {
        let matches = everywhere_fixture();
        assert!(everywhere_rows(SearchEverywhereScope::Actions, &matches).is_empty());
        // 即使 `Ready` 里躺着 Core 命中，Actions 档也一行都不取。
        assert!(rows_ready(SearchEverywhereScope::Actions).is_empty());
        // 其余五档不受影响（同一份命中，投影照旧）。
        assert_eq!(rows_ready(SearchEverywhereScope::Files).len(), 2);
        assert_eq!(rows_ready(SearchEverywhereScope::Classes).len(), 1);
        assert_eq!(rows_ready(SearchEverywhereScope::Symbols).len(), 2);
        assert_eq!(rows_ready(SearchEverywhereScope::Text).len(), 2);
    }

    /// `Ready` 态下某个 scope 能画出的行（`Actions` 恒空）。
    fn rows_ready(scope: SearchEverywhereScope) -> Vec<EverywhereRow> {
        EverywhereState::ready("alpha", &everywhere_fixture()).rows(scope)
    }

    /// 从 `Actions` 档切回 Core 档时"要不要补一次查询"的判据：只有**同一个查询串的
    /// `Ready`** 才能复用。
    #[test]
    fn a_ready_result_is_reused_only_for_the_same_query() {
        let state = EverywhereState::ready("alpha", &everywhere_fixture());
        assert!(state.is_ready_for("alpha"));
        assert!(
            state.is_ready_for("  alpha  "),
            "比较前 trim（查询一律 trim 后发出去）"
        );
        assert!(!state.is_ready_for("beta"), "换了查询串就得重发");
        assert!(!state.is_ready_for(""), "空查询不是这一次的命中");

        // `Loading` 不算：进 Actions 时在飞的请求已经被 `timeline.abandon()` 撤掉，
        // 那一份**没有**在跑的请求，切回来必须重发。
        assert!(!EverywhereState::loading("alpha", &[]).is_ready_for("alpha"));
        assert!(!EverywhereState::Idle.is_ready_for("alpha"));
        assert!(!EverywhereState::failed("alpha", "invalid_request").is_ready_for("alpha"));
    }

    /// Search Everywhere 的 `maxResults` **显式抬高**（B10 的符号饥饿绕行）：
    /// 两条命令必须共用同一个 `payload` 构造器，只有上限不同。
    #[test]
    fn search_everywhere_raises_the_result_budget_without_changing_the_request_shape() {
        let options = SearchOptions::default();
        let common = payload("D:/proj", "alpha", &options);
        let everywhere = payload_with_limit("D:/proj", "alpha", &options, EVERYWHERE_MAX_RESULTS);

        let mut same = common.clone();
        same["maxResults"] = json!(EVERYWHERE_MAX_RESULTS);
        assert_eq!(
            everywhere, same,
            "除了 maxResults，请求形状必须逐字相同（两条命令在 Core 里是同一个 SearchRequest）"
        );

        // 值本身也要钉住：它是"符号能不能活下来"的唯一旋钮（推导写在常量文档上）。
        assert_eq!(MAX_RESULTS, 200);
        assert_eq!(EVERYWHERE_MAX_RESULTS, 1000);
        assert!(
            EVERYWHERE_MAX_RESULTS > MAX_RESULTS + 2 * 50,
            "预留的符号预算（types 50 + symbols 50）必须真的落在抬高后的预算里"
        );
        assert!(EVERYWHERE_MAX_RESULTS <= 10_000, "Core 自己夹到 10_000");
        assert_eq!(everywhere["maxResults"], json!(1000));
    }

    /// Search Everywhere 与 Quick Open 用的是**同一个 `parse_matches`**：一份响应喂给两种
    /// 投影，符号行不会在解析层被丢掉（那正是"没有第二份解析"的判据）。
    #[test]
    fn everywhere_and_quick_open_share_one_parser() {
        let data = json!({
            "matches": [
                { "kind": "file", "path": "src/A.java", "line": null, "preview": "src/A.java" },
                { "kind": "content", "path": "src/A.java", "line": 12, "preview": "class A {" },
                { "kind": "type", "path": "src/A.java", "line": 1, "preview": "class A", "symbolName": "A" },
                { "kind": "symbol", "path": "src/B.java", "line": 3, "preview": "void b()", "symbolName": "b" },
            ]
        });
        let matches = parse_matches(Some(&data)).expect("必须解析成功");
        assert_eq!(matches.len(), 4, "解析层一条都不丢");
        assert_eq!(quick_open_rows(&matches).len(), 1, "Quick Open 只留文件行");
        assert_eq!(
            everywhere_rows(SearchEverywhereScope::All, &matches).len(),
            3,
            "All 拿 file + type + symbol 三条"
        );
        assert_eq!(
            everywhere_rows(SearchEverywhereScope::Text, &matches).len(),
            1
        );
    }

    // ---- Replace in Files（U17 第四件，`workspace.replacePreview`） ----

    /// 请求的字段名与**取值**逐字钉住：拼错一位 Core 就用 `invalid_request` 拒掉整条命令，
    /// 而这条命令**不写盘**（写盘的是不存在的 apply），所以"发出去的到底是什么"
    /// 只能靠这里与诊断行核对。
    #[test]
    fn replace_payload_follows_the_core_contract_field_names() {
        let options = ReplaceOptions {
            search: SearchOptions {
                case_sensitive: true,
                whole_words: true,
                regular_expression: false,
                file_mask: "*.java, *.kt".to_string(),
            },
            preserve_case: true,
        };
        assert_eq!(
            replace_payload("D:/proj", "needle", "replacement", &options),
            json!({
                "root": "D:/proj",
                "query": "needle",
                "replacement": "replacement",
                "caseSensitive": true,
                "wholeWords": true,
                "regularExpression": false,
                "preserveCase": true,
                "fileMask": "*.java, *.kt",
            })
        );
    }

    /// **故意不传**的字段：`paths` / `textOverrides`（宿主没有未保存缓冲区通路）、
    /// 可见性数组（索引缓存键）、以及**根本不存在的上限** —— 契约里没有 `maxResults`，
    /// 宿主也不许自己发明一个（那会变成第二套上限）。
    #[test]
    fn replace_payload_omits_paths_overrides_and_any_invented_limit() {
        let value = replace_payload("r", "q", "", &ReplaceOptions::default());
        for key in [
            "paths",
            "textOverrides",
            "hiddenDirectoryNames",
            "hiddenFilePatterns",
            "maxResults",
            "maxFileResults",
            "maxContentResults",
        ] {
            assert!(
                value.get(key).is_none(),
                "replacePreview 的 payload 里不该有 {key}：{value}"
            );
        }
        // 默认选项：四个布尔全关、掩码空串（Core 的 `#[serde(default)]` 同值）。
        assert_eq!(value["caseSensitive"], json!(false));
        assert_eq!(value["wholeWords"], json!(false));
        assert_eq!(value["regularExpression"], json!(false));
        assert_eq!(value["preserveCase"], json!(false));
        assert_eq!(value["fileMask"], json!(""));
    }

    /// 正常响应：按文件分组、`before` / `after` / 行号 / 次数全部原样带出，
    /// **顺序就是 Core 给的顺序**（宿主不重排）。
    #[test]
    fn parses_the_replace_preview_grouped_by_file() {
        let data = json!({
            "files": [
                {
                    "path": "src/A.java",
                    "matches": [
                        { "line": 12, "before": "  int a = 1;", "after": "  int b = 1;", "occurrenceCount": 1 },
                        { "line": 30, "before": "  int a = 2; int a = 3;", "after": "  int b = 2; int b = 3;", "occurrenceCount": 2 },
                    ],
                    "replacementText": "…整份文件正文（本层不解析，见类型文档）…",
                },
                { "path": "src/B.java", "matches": [], "replacementText": "" },
            ]
        });
        let files = parse_replace_preview(Some(&data)).expect("必须解析成功");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/A.java");
        assert_eq!(files[0].lines.len(), 2);
        assert_eq!(files[0].lines[0].line, 12);
        assert_eq!(files[0].lines[0].before, "  int a = 1;");
        assert_eq!(files[0].lines[0].after, "  int b = 1;");
        assert_eq!(files[0].lines[0].occurrences, 1);
        assert_eq!(files[0].lines[1].line, 30, "顺序照 Core，不重排");
        assert_eq!(files[0].occurrences(), 3, "1 + 2，与真源 matchCount 同算法");
        assert_eq!(files[1].path, "src/B.java");
        assert!(files[1].lines.is_empty());
    }

    /// 契约异常必须报出来，**不许被读成"没有命中"**（那会让界面画一句"没找到"，
    /// 而真相是这条响应不可用）。`occurrenceCount` 缺失是例外：它只影响计数。
    #[test]
    fn replace_contract_anomalies_are_reported_not_silently_read_as_no_matches() {
        assert_eq!(
            parse_replace_preview(None).unwrap_err(),
            ParseError::MissingData
        );
        assert_eq!(
            parse_replace_preview(Some(&json!({ "matches": [] }))).unwrap_err(),
            ParseError::Malformed,
            "缺 files 数组"
        );
        assert_eq!(
            parse_replace_preview(Some(&json!({ "files": [{ "matches": [] }] }))).unwrap_err(),
            ParseError::MissingPath(0)
        );
        assert_eq!(
            parse_replace_preview(Some(&json!({ "files": [{ "path": "a", "matches": {} }] })))
                .unwrap_err(),
            ParseError::Malformed
        );
        assert_eq!(
            parse_replace_preview(Some(&json!({
                "files": [{ "path": "a", "matches": [{ "line": 1, "before": "x" }] }]
            })))
            .unwrap_err(),
            ParseError::Malformed,
            "缺 after"
        );
        // 次数缺失：这一行照样画（before / after 才是内容），计数按 0。
        let files = parse_replace_preview(Some(&json!({
            "files": [{ "path": "a", "matches": [{ "line": 1, "before": "x", "after": "y" }] }]
        })))
        .expect("次数缺失不是契约异常");
        assert_eq!(files[0].lines[0].occurrences, 0);
    }

    /// **空结果不是失败**：`files: []` 是 `Ready` 且零行（界面画"没找到"），
    /// 不是 `Failed`（界面画 Core 的原因）。
    #[test]
    fn an_empty_replace_preview_is_ready_not_failed() {
        let files = parse_replace_preview(Some(&json!({ "files": [] }))).expect("空结果是成功");
        let state = ReplaceState::ready("needle", &files);
        assert_eq!(state.rows(), Vec::new());
        assert_eq!(state.counts(), (0, 0));
        assert!(!matches!(state, ReplaceState::Failed { .. }));
    }

    /// 摊平只做"文件头 + 命中行"，顺序 = Core 的顺序；计数是 `occurrenceCount` 之和。
    #[test]
    fn replace_rows_fold_each_file_into_a_header_plus_lines() {
        let files = vec![
            ReplacementFile {
                path: "src/B.java".to_string(),
                lines: vec![ReplacementLine {
                    line: 7,
                    before: "b".to_string(),
                    after: "B".to_string(),
                    occurrences: 2,
                }],
            },
            ReplacementFile {
                path: "src/A.java".to_string(),
                lines: vec![
                    ReplacementLine {
                        line: 1,
                        before: "a".to_string(),
                        after: "A".to_string(),
                        occurrences: 1,
                    },
                    ReplacementLine {
                        line: 2,
                        before: "aa".to_string(),
                        after: "AA".to_string(),
                        occurrences: 1,
                    },
                ],
            },
        ];
        // Core 给的就是这个顺序（`files.rs:421` 按路径升序），本层**不重排**：
        // 换了顺序这里就该红，提示有人加了第二套排序。
        let rows = replace_rows(&files);
        assert_eq!(rows.len(), 5, "2 个文件头 + 3 条命中行");
        assert_eq!(
            rows,
            vec![
                ReplaceRow::File {
                    path: "src/B.java".to_string(),
                    occurrences: 2,
                },
                ReplaceRow::Line {
                    path: "src/B.java".to_string(),
                    line: 7,
                    before: "b".to_string(),
                    after: "B".to_string(),
                    occurrences: 2,
                },
                ReplaceRow::File {
                    path: "src/A.java".to_string(),
                    occurrences: 2,
                },
                ReplaceRow::Line {
                    path: "src/A.java".to_string(),
                    line: 1,
                    before: "a".to_string(),
                    after: "A".to_string(),
                    occurrences: 1,
                },
                ReplaceRow::Line {
                    path: "src/A.java".to_string(),
                    line: 2,
                    before: "aa".to_string(),
                    after: "AA".to_string(),
                    occurrences: 1,
                },
            ]
        );
    }

    /// 四态：`idle` 没有查询串；`loading` **带着上一次的预览**继续可见（换了查询就闪一下
    /// 空列表是更差的观感）；`failed` 没有行；`counts()` 只在有结果时有数。
    #[test]
    fn replace_state_keeps_the_previous_preview_while_loading() {
        let files = vec![ReplacementFile {
            path: "src/A.java".to_string(),
            lines: vec![ReplacementLine {
                line: 3,
                before: "x".to_string(),
                after: "y".to_string(),
                occurrences: 4,
            }],
        }];

        assert_eq!(ReplaceState::Idle.query(), "");
        assert!(ReplaceState::Idle.files().is_empty());
        assert_eq!(ReplaceState::Idle.counts(), (0, 0));

        let loading = ReplaceState::loading("next", &files);
        assert_eq!(loading.query(), "next");
        assert_eq!(loading.files().len(), 1, "上一次的预览继续可见");
        assert_eq!(loading.counts(), (1, 4));

        let ready = ReplaceState::ready("next", &files);
        assert_eq!(ready.rows().len(), 2);
        assert_eq!(ready.counts(), (1, 4));

        let failed = ReplaceState::failed("next", "invalid_request: bad regex");
        assert_eq!(failed.query(), "next");
        assert!(failed.rows().is_empty());
        assert_eq!(failed.counts(), (0, 0));
    }
}
