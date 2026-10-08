use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, StyledExt as _};
use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, rems};

use lithe_gpui_java::{JavaDiagnostic, JavaFileDiagnostics, JavaLanguageService};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{tr, tr_args};

use crate::diagnostics::Level;

pub type OpenLocation = Arc<dyn Fn(&Path, u32, u32, &mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Problem {
    pub(crate) line: u32,
    pub(crate) utf16_column: u32,
    pub(crate) level: Level,
    pub(crate) message: String,
    pub(crate) source: Option<String>,
    pub(crate) code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProblemsFile {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) directory: String,
    pub(crate) problems: Vec<Problem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SeverityFilter {
    pub(crate) errors: bool,
    pub(crate) warnings: bool,
    pub(crate) infos: bool,
}

impl SeverityFilter {
    pub(crate) const ALL: Self = Self {
        errors: true,
        warnings: true,
        infos: true,
    };

    pub(crate) fn shows(self, level: Level) -> bool {
        match level {
            Level::Error => self.errors,
            Level::Warning => self.warnings,
            Level::Info | Level::Hint => self.infos,
        }
    }

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl Counts {
    pub(crate) fn total(self) -> usize {
        self.errors + self.warnings + self.infos
    }
}

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

pub(crate) fn visible_count(files: &[ProblemsFile], filter: SeverityFilter) -> usize {
    files
        .iter()
        .flat_map(|file| &file.problems)
        .filter(|problem| filter.shows(problem.level))
        .count()
}

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

fn relative_components(relative: &Path) -> String {
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub struct ProblemsPane {
    java: Option<Arc<JavaLanguageService>>,
    root: Option<PathBuf>,
    files: Vec<ProblemsFile>,
    filter: SeverityFilter,
    collapsed: HashSet<PathBuf>,
    open: Option<OpenLocation>,
}

impl ProblemsPane {
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

    pub(crate) fn set_open(&mut self, open: OpenLocation) {
        self.open = Some(open);
    }

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

    pub fn counts(&self) -> Counts {
        problem_counts(&self.files)
    }

    fn toggle(&mut self, level: Level, cx: &mut Context<Self>) {
        self.filter = self.filter.toggled(level);
        cx.notify();
    }

    fn reset_filter(&mut self, cx: &mut Context<Self>) {
        self.filter = SeverityFilter::ALL;
        cx.notify();
    }

    fn toggle_collapsed(&mut self, path: &Path, cx: &mut Context<Self>) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_path_buf());
        }
        cx.notify();
    }

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
                    .overflow_y_scroll()
                    .child(body),
            )
            .into_any_element()
    }
}

impl ProblemsPane {
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

    fn problem_row(&self, path: &Path, problem: &Problem, cx: &Context<Self>) -> AnyElement {
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

fn severity_icon(level: Level) -> &'static idea::IdeaIcon {
    match level {
        Level::Error => &idea::XCIRCLE_ICON,
        Level::Warning => &idea::WARNING_CIRCLE_ICON,
        Level::Info | Level::Hint => &idea::INFO_ICON,
    }
}

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

    #[test]
    fn display_names_are_workspace_relative_when_possible() {
        let (name, directory) = display_parts(
            Some(Path::new("/ws")),
            Path::new("/ws/src/main/java/demo/App.java"),
        );
        assert_eq!(name, "App.java");
        assert_eq!(directory, "src/main/java/demo");

        let (name, directory) = display_parts(Some(Path::new("/ws")), Path::new("/ws/pom.xml"));
        assert_eq!(name, "pom.xml");
        assert_eq!(directory, "");

        let (name, directory) = display_parts(
            Some(Path::new("/ws")),
            Path::new("/elsewhere/lib/Sources.java"),
        );
        assert_eq!(name, "Sources.java");
        assert!(directory.ends_with("lib"), "实际是 {directory}");

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

        let no_warnings = SeverityFilter {
            warnings: false,
            ..SeverityFilter::ALL
        };
        assert_eq!(problem_counts(&built).total(), 3);
        assert_eq!(visible_count(&built, no_warnings), 2);

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

    #[test]
    fn toggling_a_severity_twice_restores_the_filter() {
        let filter = SeverityFilter::ALL.toggled(Level::Error);
        assert!(!filter.shows(Level::Error));
        assert!(filter.shows(Level::Hint), "只动点的那一档");
        assert_eq!(filter.toggled(Level::Error), SeverityFilter::ALL);
    }

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
