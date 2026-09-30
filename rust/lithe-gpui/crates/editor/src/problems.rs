//! 问题列表（底部工具窗的「诊断」页）：把服务端报的**全部**诊断按文件列出来。
//!
//! ## 它解决的是"错误在没打开的文件里"这一档
//!
//! 编辑器里的波浪线只覆盖**打开着的** buffer。而 Java 项目最常见的失败恰恰不在这里：
//! 一个多模块 Maven 工程里，报错的是没打开的模块，用户在界面上什么都看不到
//! （`java/src/service.rs` 的 `workspace_diagnostics` 记着这件事的来龙去脉，
//! 决策笔记是 `.agents/notes/implemented/bug-fix/2026-09-20-workspace-diagnostics-visibility.md`）。
//! 这个面板就是那些诊断的落点：**判据是"在不在工作区内"，不是"有没有打开"**。
//!
//! ## 数据从哪来：一次查询，不是订阅
//!
//! [`ProblemsPane::refresh`] 直接问 [`JavaLanguageService::workspace_diagnostics`]
//! （纯查询：不起会话、不阻塞），把结果整份换掉。**没有**去订阅事件泵：
//!
//! 1. 事件泵是 `java` crate 内部的（`SessionEvents` 是 `pub(crate)`），订阅要么开一个新的
//!    回调通道，要么把泵的锁暴露出去 —— 而"问到什么就画什么"已经够用；
//! 2. 刷新时机是**确定的**（见下表），不需要靠事件去猜"什么时候可能变了"。
//!
//! | 刷新时机 | 谁触发 | 为什么够 |
//! | --- | --- | --- |
//! | Java 服务起来之后 | [`crate::EditorPane::prepare_java`] | 首次有数据可查 |
//! | 每个 buffer 的诊断落地之后 | [`crate::EditorPane::apply_diagnostics`] | 波浪线与列表**同一份快照**，不会一个新一个旧 |
//! | 面板被切到前台 | 外壳（`workbench` 的底部工具窗切换处） | 长时间没打开过面板时补一次 |
//!
//! 代价：刷新是"整份重建"（复制全部诊断 + 排序）。它发生在**防抖之后**（每个 buffer
//! 一次取回只有一次落地），而上限由 Core 兜着（未打开文件最多 2000 个，
//! `rust/lithe-core/src/lsp/interface/client.rs` 的 `WORKSPACE_DIAGNOSTIC_FILE_LIMIT`），
//! 所以量级是"几千条诊断的一次排序"，不是每帧。
//!
//! ## 点一行 = 打开那个文件并落到那一行
//!
//! 面板自己**开不了文件**（buffer 归 [`crate::EditorPane`]），所以走的是与标签右键菜单
//! 同一个"宿主回调"形状（[`OpenLocation`]）：由 `EditorPane` 在装配时登记，
//! 落点是 `EditorPane::open_at`。
//!
//! ## 与旧前端的关系
//!
//! 视觉规格取自 Windows 前端的 `features/diagnostics/components/diagnostics-pane.tsx`
//! （按文件成组、组头可折叠、行内是"严重度图标 + 文案 + 位置"）。本侧的**界面文案全部复用
//! 那份前端已经在 locale 里的键**（`lithe.diagnostics.*`，54 条，见 `shared/locales/`），
//! 一条新键都不加。真源里还有的那几档（按严重度分组 / 排序切换 / 全文搜索 / 来源筛选 /
//! 快速修复菜单 / 复制动作 / 全屏）本批**没做** —— 空格子不画，因为画一个点不动的控件
//! 比没有更糟。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
// 可点的 `div()` 必须先 `.id(..)`（`on_click` 在 `StatefulInteractiveElement` 上，
// 细节见 `workbench/src/notifications.rs:57-61` 的同一条注释）。
// `overflow_y_scroll` 来自 `InteractiveElement`（gpui 自己那个"要挂 `ScrollHandle`"的版本），
// 不是 gpui-component 的 `ScrollableElement::overflow_y_scrollbar`。
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, rems,
};

use lithe_gpui_java::{JavaDiagnostic, JavaFileDiagnostics, JavaLanguageService};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{tr, tr_args};

use crate::diagnostics::Level;

/// 点一行要做的事：打开 `path` 并落到服务端给的位置（UTF-16 列口径）。
///
/// `&Path` 而不是 `PathBuf`：回调在 `&self` 上取用，路径由面板的列表持有，
/// 每次点击拷一份路径没有意义（同一形状见 [`crate::TabMenuHostActions`]）。
pub type OpenLocation = Arc<dyn Fn(&Path, u32, u32, &mut Window, &mut App) + 'static>;

/// 列表里的一条问题。
///
/// 位置按服务端口径**原样存**（0 基行 + UTF-16 列）：换算成编辑器的字符列要那份正文，
/// 而列表没有正文 —— 那是 [`crate::EditorPane::open_at`] 落地时的事。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Problem {
    /// 0 基行号。
    pub(crate) line: u32,
    /// 该行的 UTF-16 码元列（LSP 的口径）。
    pub(crate) utf16_column: u32,
    /// 严重度档位。
    pub(crate) level: Level,
    /// 服务端给的消息（可能是多行的）。
    pub(crate) message: String,
    /// 发布来源（JDT 给的是 `Java`）。
    pub(crate) source: Option<String>,
    /// 服务端的稳定码。
    pub(crate) code: Option<String>,
}

/// 按文件成组的一段。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemsFile {
    /// 绝对路径（打开文件要用它）。
    pub(crate) path: PathBuf,
    /// 组头里显示的文件名。
    pub(crate) name: String,
    /// 组头里弱化显示的那一段（工作区相对目录；为空表示文件就在工作区根上）。
    pub(crate) directory: String,
    /// 这一组的问题，已按行、列排好。
    pub(crate) problems: Vec<Problem>,
}

/// 严重度筛选：三档开关（错误 / 警告 / 信息）。
///
/// 为什么 "hint" 归在信息里：info 与 hint 在 IDEA 的这个问题列表里是同一行
/// （都算"不是问题的问题"），拆成四个开关只会让工具条更长而没有新的判别力。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SeverityFilter {
    pub(crate) errors: bool,
    pub(crate) warnings: bool,
    pub(crate) infos: bool,
}

impl SeverityFilter {
    /// 默认三档全开（真源的初始状态：`severityFilter = Set(allCases)`）。
    pub(crate) const ALL: Self = Self {
        errors: true,
        warnings: true,
        infos: true,
    };

    /// 这一档现在是否显示。
    pub(crate) fn shows(self, level: Level) -> bool {
        match level {
            Level::Error => self.errors,
            Level::Warning => self.warnings,
            Level::Info | Level::Hint => self.infos,
        }
    }

    /// 取反一档。
    pub(crate) fn toggled(self, level: Level) -> Self {
        match level {
            Level::Error => Self {
                errors: !self.errors,
                ..self
            },
            Level::Warning => Self {
                warnings: !self.warnings,
                ..self
            },
            Level::Info | Level::Hint => Self {
                infos: !self.infos,
                ..self
            },
        }
    }
}

/// 一次刷新的统计（给工具条与诊断行用）。
///
/// `pub`：外壳可能拿它做角标之类的呈现（[`ProblemsPane::counts`] 的返回类型）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// 错误档条数。
    pub errors: usize,
    /// 警告档条数。
    pub warnings: usize,
    /// 信息档（含 hint）条数。
    pub infos: usize,
}

impl Counts {
    /// 全部条数（三档之和）。
    pub(crate) fn total(self) -> usize {
        self.errors + self.warnings + self.infos
    }
}

/// 全部问题的统计（**不**受筛选影响：工具条上的计数是"事实"，不是"筛后剩多少"）。
pub(crate) fn problem_counts(files: &[ProblemsFile]) -> Counts {
    let mut counts = Counts::default();
    for file in files {
        for problem in &file.problems {
            match problem.level {
                Level::Error => counts.errors += 1,
                Level::Warning => counts.warnings += 1,
                Level::Info | Level::Hint => counts.infos += 1,
            }
        }
    }
    counts
}

/// 筛选之后还剩多少条（判"没有匹配"用）。
pub(crate) fn visible_count(files: &[ProblemsFile], filter: SeverityFilter) -> usize {
    files
        .iter()
        .flat_map(|file| &file.problems)
        .filter(|problem| filter.shows(problem.level))
        .count()
}

/// 服务端的快照 → 列表数据（纯函数，可直接单测）。
///
/// 三件事，每件都是"用户看得见"的：
///
/// 1. **跳过空快照**：服务端撤回过诊断的文件不该在列表上占一行 —— 这一层是"最后一次防线"，
///    `Service::workspace_diagnostics` 已经滤过一遍，但纯函数不该假设调用方滤过；
/// 2. **排序**：文件按路径、文件内按（行，列）。服务端的推送顺序不是排序顺序，
///    照抄会让同一份问题在不同刷新里跳来跳去；
/// 3. **算显示名**：`root` 之内显示成工作区相对路径（分成"文件名 + 目录"两段，
///    组头要弱化显示目录那一段），之外显示绝对路径 —— 那个文件不在工作区里，
///    假装它相对谁都是错的。
pub(crate) fn build_files(files: &[JavaFileDiagnostics], root: Option<&Path>) -> Vec<ProblemsFile> {
    let mut built: Vec<ProblemsFile> = files
        .iter()
        .filter(|file| !file.diagnostics.is_empty())
        .map(|file| {
            let (name, directory) = display_parts(root, &file.path);
            let mut problems: Vec<Problem> = file.diagnostics.iter().map(Problem::from).collect();
            problems.sort_by_key(|problem| (problem.line, problem.utf16_column));
            ProblemsFile {
                path: file.path.clone(),
                name,
                directory,
                problems,
            }
        })
        .collect();
    built.sort_by(|left, right| left.path.cmp(&right.path));
    built
}

impl From<&JavaDiagnostic> for Problem {
    fn from(diagnostic: &JavaDiagnostic) -> Self {
        Self {
            line: diagnostic.range.start.line,
            utf16_column: diagnostic.range.start.utf16_column,
            level: Level::from_lsp(diagnostic.severity),
            message: diagnostic.message.clone(),
            source: diagnostic.source.clone(),
            code: diagnostic.code.clone(),
        }
    }
}

/// 路径 → 组头要显示的两段（文件名，目录）。
///
/// 工作区之内的文件：目录是**工作区相对**的（`src/main/java/demo`），用 `/` 分隔 ——
/// 契约里跨平台标识符一律用 `/`（`develop-lithe`「Keep shared contracts deterministic」），
/// 显示上也一样，免得同一个文件在 Windows 上写成反斜杠、在别处写成斜杠。
/// 工作区之外（或没有工作区）的：目录是绝对路径，文件名照旧。
pub(crate) fn display_parts(root: Option<&Path>, path: &Path) -> (String, String) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let directory = root
        .and_then(|root| path.strip_prefix(root).ok())
        .and_then(|relative| relative.parent())
        .map(relative_components)
        .unwrap_or_else(|| {
            path.parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default()
        });
    (name, directory)
}

/// 相对目录的显示串：`/` 分隔，且**不含**文件名那一段。
fn relative_components(relative: &Path) -> String {
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 底部工具窗「诊断」页的实体。
///
/// 自带视图状态（筛选、哪些文件折叠了）而不是交给外壳：它们只在这个面板里有意义，
/// 换根时本来就该一起没（与通知中心面板同一条口径）。
pub struct ProblemsPane {
    /// Java 语言服务。`None` = 外壳还没起服务（启动早期，或根本不是 Java 项目）。
    java: Option<Arc<JavaLanguageService>>,
    /// 工作区根：只用来算显示用的相对路径。
    root: Option<PathBuf>,
    /// 当前列表（已排序、已成组）。
    files: Vec<ProblemsFile>,
    /// 严重度筛选。
    filter: SeverityFilter,
    /// 折叠起来的文件（存路径不存下标：刷新会整份重建列表）。
    collapsed: HashSet<PathBuf>,
    /// 点一行的落地动作（由 `EditorPane` 登记）。
    open: Option<OpenLocation>,
}

impl ProblemsPane {
    /// 建一个空列表（外壳装配时创建，服务起来之前[`Self::refresh`]什么都不会做）。
    pub fn new() -> Self {
        Self {
            java: None,
            root: None,
            files: Vec::new(),
            filter: SeverityFilter::ALL,
            collapsed: HashSet::new(),
            open: None,
        }
    }

    /// 登记 Java 服务与工作区根，并立刻刷新一次。
    ///
    /// 由 [`crate::EditorPane::prepare_java`] 调（与波浪线、补全同一个调用点）。
    pub(crate) fn set_service(
        &mut self,
        service: Arc<JavaLanguageService>,
        root: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.java = Some(service);
        self.root = Some(root);
        self.refresh(cx);
    }

    /// 登记"点一行"的落地动作（见 [`OpenLocation`]）。
    pub(crate) fn set_open(&mut self, open: OpenLocation) {
        self.open = Some(open);
    }

    /// 重新问一次服务端并整份重建列表。
    ///
    /// 服务还没起来时把列表清空（而不是留着上一次的结果）：那可能是**另一个项目**的，
    /// 换根之后留着比空着更糟。这里不 panic、不报错 —— 空列表就是这个面板的"还没到"。
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        let files = match (&self.java, &self.root) {
            (Some(service), Some(root)) => {
                build_files(&service.workspace_diagnostics(), Some(root.as_path()))
            }
            _ => Vec::new(),
        };
        let counts = problem_counts(&files);
        self.files = files;
        println!(
            "S1_EDITOR_PROBLEMS files={} problems={} errors={} warnings={} infos={}",
            self.files.len(),
            counts.total(),
            counts.errors,
            counts.warnings,
            counts.infos,
        );
        cx.notify();
    }

    /// 工具条上的统计（不受筛选影响）。
    pub fn counts(&self) -> Counts {
        problem_counts(&self.files)
    }

    /// 取反一档筛选。
    fn toggle(&mut self, level: Level, cx: &mut Context<Self>) {
        self.filter = self.filter.toggled(level);
        cx.notify();
    }

    /// 重置筛选（"没有匹配"空态里的那个按钮）。
    fn reset_filter(&mut self, cx: &mut Context<Self>) {
        self.filter = SeverityFilter::ALL;
        cx.notify();
    }

    /// 折叠 / 展开一个文件。
    fn toggle_collapsed(&mut self, path: &Path, cx: &mut Context<Self>) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_path_buf());
        }
        cx.notify();
    }

    /// 点一条问题：打开文件并落到它的位置。
    ///
    /// 没登记落地动作时（组件在测试宿主里单独跑）只留一行诊断，不 panic。
    fn open_problem(&self, path: &Path, problem: &Problem, window: &mut Window, cx: &mut App) {
        let Some(open) = self.open.clone() else {
            println!("S1_EDITOR_PROBLEMS open=unwired file={}", path.display());
            return;
        };
        open(path, problem.line, problem.utf16_column, window, cx);
    }
}

impl Default for ProblemsPane {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for ProblemsPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let counts = problem_counts(&self.files);
        let visible = visible_count(&self.files, self.filter);
        // 两档空态分得很清楚（真源同样分）：一条问题都没有 vs 筛选之后一条都不剩。
        // 合成一档的话，"筛没了"会看起来像"项目干净了"。
        let body: AnyElement = if self.files.is_empty() {
            self.empty_state(tr("lithe.diagnostics.empty"), false, cx)
        } else if visible == 0 {
            self.empty_state(tr("lithe.diagnostics.noMatch"), true, cx)
        } else {
            v_flex()
                .w_full()
                .children(self.file_groups(cx))
                .into_any_element()
        };

        v_flex()
            .size_full()
            .child(self.toolbar(counts, cx))
            .child(
                v_flex()
                    .id("lithe-problems-list")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    // 滚动容器无条件挂（`StatefulInteractiveElement::overflow_y_scroll`）：
                    // 内容不满时它什么都不画，而放进分支里会让两边的元素类型不同。
                    .overflow_y_scroll()
                    .child(body),
            )
            .into_any_element()
    }
}

impl ProblemsPane {
    /// 工具条：统计 + 三档开关。
    ///
    /// 统计数字取**未筛选**的总数（"还有 3 个错误"不该因为关了错误那一档就变成 0 ——
    /// 那会让用户以为错误没了）。开关本身用档位色 / 弱化色区分开与关，
    /// **颜色之外还有数字**：色盲用户与灰度截图下都读得出来。
    fn toolbar(&self, counts: Counts, cx: &Context<Self>) -> AnyElement {
        let total = counts.total().to_string();
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().foreground)
                    .child(tr_args(
                        "lithe.diagnostics.problemCount",
                        &[("count", total.as_str())],
                    )),
            );

        for (level, count, key) in [
            (Level::Error, counts.errors, "lithe.diagnostics.errors"),
            (
                Level::Warning,
                counts.warnings,
                "lithe.diagnostics.warnings",
            ),
            (Level::Info, counts.infos, "lithe.diagnostics.info"),
        ] {
            let shown = self.filter.shows(level);
            let color = if shown {
                severity_color(level, cx)
            } else {
                cx.theme().muted_foreground
            };
            // `tr_args` 收的是 `&str`，所以三样都要先落到局部变量上（临时值的借用活不到调用结束）。
            let label = tr(key).to_string();
            let count = count.to_string();
            row = row.child(
                h_flex()
                    .id(SharedString::from(format!(
                        "problems-filter-{}",
                        level.id()
                    )))
                    .gap_1()
                    .px_1()
                    .rounded_sm()
                    .when(shown, |this| this.bg(cx.theme().accent))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle(level, cx)))
                    .child(idea_icon_image(severity_icon(level), cx, rems(0.75)))
                    .child(div().text_xs().text_color(color).child(tr_args(
                        "lithe.diagnostics.severityCount",
                        &[
                            ("label", label.as_str()),
                            ("visible", count.as_str()),
                            ("total", count.as_str()),
                        ],
                    ))),
            );
        }
        row.into_any_element()
    }

    /// 按文件成组的那一串。
    fn file_groups(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut groups = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let problems: Vec<&Problem> = file
                .problems
                .iter()
                .filter(|problem| self.filter.shows(problem.level))
                .collect();
            if problems.is_empty() {
                continue;
            }
            let collapsed = self.collapsed.contains(&file.path);
            let path = file.path.clone();
            let mut group = v_flex().w_full().child(
                h_flex()
                    .id(SharedString::from(format!(
                        "problems-file-{}",
                        file.path.display()
                    )))
                    .w_full()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_collapsed(&path, cx)))
                    .child(idea_icon_image(
                        if collapsed {
                            &idea::CARET_RIGHT_ICON
                        } else {
                            &idea::CARET_DOWN_ICON
                        },
                        cx,
                        rems(0.75),
                    ))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(file.name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(file.directory.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("({})", problems.len())),
                    ),
            );
            if !collapsed {
                for problem in problems {
                    group = group.child(self.problem_row(&file.path, problem, cx));
                }
            }
            groups.push(group.into_any_element());
        }
        groups
    }

    /// 一条问题：严重度图标 + 文案 + `行:列`（+ 来源）。
    fn problem_row(&self, path: &Path, problem: &Problem, cx: &Context<Self>) -> AnyElement {
        // 显示用 1 基（`S1_EDITOR_CURSOR` 与状态栏都是 1 基，用户看的也是 1 基）；
        // 落地时要的是 0 基，所以换算只发生在 `EditorPane::open_at` 里 —— 这里不动数据。
        let location = format!("{}:{}", problem.line + 1, problem.utf16_column + 1);
        let source = problem
            .source
            .clone()
            .filter(|source| !source.is_empty())
            .or_else(|| problem.code.clone())
            .unwrap_or_default();
        let path = path.to_path_buf();
        let entry = problem.clone();

        h_flex()
            .id(SharedString::from(format!(
                "problems-row-{}-{}-{}",
                path.display(),
                problem.line,
                problem.utf16_column
            )))
            .w_full()
            .items_start()
            .gap_2()
            .px_2()
            .py_1()
            .cursor_pointer()
            // 点击：打开文件并落到这一行（`EditorPane::open_at` 负责列口径换算）。
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_problem(&path, &entry, window, cx);
            }))
            .child(idea_icon_image(
                severity_icon(problem.level),
                cx,
                rems(0.75),
            ))
            .child(
                v_flex()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(problem.message.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(location),
                            )
                            .when(!source.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(source),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    /// 两档空态：一条问题都没有，或筛选之后一条都不剩（后者带「重置筛选」）。
    ///
    /// 空态只占**列表区**（工具条照画）：否则用户关掉三档筛选之后就再也点不回来 ——
    /// 这正是通知中心面板踩过的那个坑（`workbench/src/notifications.rs:571-574`）。
    fn empty_state(&self, title: SharedString, reset: bool, cx: &Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .items_center()
            .justify_center()
            .gap_1()
            .py_6()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(title),
            )
            .when(reset, |this| {
                this.child(
                    div()
                        .id("problems-reset-filters")
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.reset_filter(cx)))
                        .child(tr("lithe.diagnostics.resetFilters")),
                )
            })
            .into_any_element()
    }
}

/// 严重度 → 图标（与通知中心同一套字形：错误 `CircleX`、警告 `TriangleAlert`、信息 `Info`）。
fn severity_icon(level: Level) -> &'static idea::IdeaIcon {
    match level {
        Level::Error => &idea::XCIRCLE_ICON,
        Level::Warning => &idea::WARNING_CIRCLE_ICON,
        Level::Info | Level::Hint => &idea::INFO_ICON,
    }
}

/// 严重度 → 颜色（主题 token，不写裸色值）。
fn severity_color(level: Level, cx: &Context<ProblemsPane>) -> gpui_kit::Hsla {
    match level {
        Level::Error => cx.theme().danger,
        Level::Warning => cx.theme().warning,
        _ => cx.theme().info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lithe_gpui_java::{JavaDiagnosticRange, JavaPosition};

    /// 造一条诊断：`severity` 是 LSP 的数值（`None` = 服务端没说）。
    fn diagnostic(line: u32, column: u32, severity: Option<i64>, message: &str) -> JavaDiagnostic {
        JavaDiagnostic {
            range: JavaDiagnosticRange {
                start: JavaPosition {
                    line,
                    utf16_column: column,
                },
                end: JavaPosition {
                    line,
                    utf16_column: column + 1,
                },
            },
            severity,
            message: message.to_string(),
            source: Some("Java".to_string()),
            code: Some("1234".to_string()),
            tags: Vec::new(),
        }
    }

    fn file(path: &str, diagnostics: Vec<JavaDiagnostic>) -> JavaFileDiagnostics {
        JavaFileDiagnostics {
            path: PathBuf::from(path),
            diagnostics,
        }
    }

    /// 列表是**按文件成组、文件内按位置**排的：服务端的推送顺序不参与排序，
    /// 否则同一份问题会在两次刷新之间跳来跳去。
    #[test]
    fn files_and_problems_are_sorted_deterministically() {
        let built = build_files(
            &[
                file(
                    "/ws/src/Beta.java",
                    vec![
                        diagnostic(9, 0, Some(1), "late"),
                        diagnostic(2, 4, Some(2), "early"),
                    ],
                ),
                file(
                    "/ws/src/Alpha.java",
                    vec![diagnostic(3, 0, Some(1), "only")],
                ),
            ],
            Some(Path::new("/ws")),
        );

        let names: Vec<&str> = built.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha.java", "Beta.java"]);
        let messages: Vec<&str> = built[1]
            .problems
            .iter()
            .map(|problem| problem.message.as_str())
            .collect();
        assert_eq!(
            messages,
            vec!["early", "late"],
            "同一文件内按（行，列）排，与推送顺序无关"
        );
    }

    /// 空快照的文件不进列表：服务端撤回诊断之后，列表上留一行 0 条问题没有意义。
    #[test]
    fn files_without_diagnostics_are_dropped() {
        let built = build_files(
            &[
                file("/ws/src/Empty.java", Vec::new()),
                file("/ws/src/Real.java", vec![diagnostic(0, 0, Some(1), "boom")]),
            ],
            Some(Path::new("/ws")),
        );
        assert_eq!(built.len(), 1);
        assert_eq!(built[0].name, "Real.java");
    }

    /// 显示名：工作区之内给**工作区相对**的目录（`/` 分隔），之外给绝对目录 ——
    /// 不在工作区里的文件假装它相对谁是错的。
    #[test]
    fn display_names_are_workspace_relative_when_possible() {
        let (name, directory) = display_parts(
            Some(Path::new("/ws")),
            Path::new("/ws/src/main/java/demo/App.java"),
        );
        assert_eq!(name, "App.java");
        assert_eq!(directory, "src/main/java/demo");

        // 工作区根上的文件：没有目录那一段。
        let (name, directory) = display_parts(Some(Path::new("/ws")), Path::new("/ws/pom.xml"));
        assert_eq!(name, "pom.xml");
        assert_eq!(directory, "");

        // 工作区之外：绝对目录照原样。
        let (name, directory) = display_parts(
            Some(Path::new("/ws")),
            Path::new("/elsewhere/lib/Sources.java"),
        );
        assert_eq!(name, "Sources.java");
        assert!(directory.ends_with("lib"), "实际是 {directory}");

        // 同名前缀的目录不算"在工作区内"（与 Core 的包含判据同一口径）。
        let (name, directory) = display_parts(
            Some(Path::new("/ws")),
            Path::new("/ws-archive/src/App.java"),
        );
        assert_eq!(name, "App.java");
        assert!(
            !directory.is_empty() && directory.starts_with('/'),
            "同名前缀目录必须按绝对路径显示，实际是 {directory}"
        );
    }

    /// 严重度分档：LSP 的四个数值各归各位，**缺字段与未知值算信息**（与波浪线同一个判据）。
    #[test]
    fn severity_levels_follow_lsp_numbers() {
        assert_eq!(Level::from_lsp(Some(1)), Level::Error);
        assert_eq!(Level::from_lsp(Some(2)), Level::Warning);
        assert_eq!(Level::from_lsp(Some(3)), Level::Info);
        assert_eq!(Level::from_lsp(Some(4)), Level::Hint);
        assert_eq!(Level::from_lsp(None), Level::Info);
        assert_eq!(Level::from_lsp(Some(0)), Level::Info);
        assert_eq!(Level::from_lsp(Some(99)), Level::Info);
    }

    /// 工具条上的计数是**事实**（未筛选），筛选只决定列表里画哪些。
    #[test]
    fn counts_are_unfiltered_and_the_filter_only_gates_rows() {
        let built = build_files(
            &[file(
                "/ws/src/App.java",
                vec![
                    diagnostic(0, 0, Some(1), "error"),
                    diagnostic(1, 0, Some(2), "warning"),
                    diagnostic(2, 0, Some(4), "hint"),
                ],
            )],
            Some(Path::new("/ws")),
        );

        let counts = problem_counts(&built);
        assert_eq!(counts.errors, 1);
        assert_eq!(counts.warnings, 1);
        assert_eq!(counts.infos, 1, "hint 与 info 归同一档");
        assert_eq!(counts.total(), 3);
        assert_eq!(visible_count(&built, SeverityFilter::ALL), 3);

        // 关掉警告：计数不变，可见条数少一条。这正是"关掉一档不等于问题没了"。
        let no_warnings = SeverityFilter {
            warnings: false,
            ..SeverityFilter::ALL
        };
        assert_eq!(problem_counts(&built).total(), 3);
        assert_eq!(visible_count(&built, no_warnings), 2);

        // 三档全关是"没有匹配"，不是"项目干净了"（两档空态的判据）。
        let none = SeverityFilter {
            errors: false,
            warnings: false,
            infos: false,
        };
        assert!(
            !none.shows(Level::Error) && !none.shows(Level::Warning) && !none.shows(Level::Hint),
            "三档全关时什么都不显示"
        );
        assert_eq!(visible_count(&built, none), 0);
    }

    /// 取反是幂等的自反操作：连按两次回到原样（工具条上的开关就是 `toggle`）。
    #[test]
    fn toggling_a_severity_twice_restores_the_filter() {
        let filter = SeverityFilter::ALL.toggled(Level::Error);
        assert!(!filter.shows(Level::Error));
        assert!(filter.shows(Level::Hint), "只动点的那一档");
        assert_eq!(filter.toggled(Level::Error), SeverityFilter::ALL);
    }

    /// 一条诊断的映射：消息 / 来源 / 码 / 位置都落到列表结构上，位置保持服务端口径
    /// （0 基行 + UTF-16 列）—— 换算是打开文件那一步的事，列表不做。
    #[test]
    fn problems_keep_server_side_positions() {
        let built = build_files(
            &[file(
                "/ws/src/App.java",
                vec![diagnostic(0, 10, Some(1), "boom")],
            )],
            Some(Path::new("/ws")),
        );
        let problem = &built[0].problems[0];
        assert_eq!(problem.line, 0);
        assert_eq!(problem.utf16_column, 10);
        assert_eq!(problem.message, "boom");
        assert_eq!(problem.source.as_deref(), Some("Java"));
        assert_eq!(problem.code.as_deref(), Some("1234"));
    }
}
