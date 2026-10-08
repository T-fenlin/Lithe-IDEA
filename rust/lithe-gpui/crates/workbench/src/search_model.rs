use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::command_palette::CommandAction;

pub(crate) const SEARCH_COMMAND: &str = "workspace.search";

pub(crate) const EVERYWHERE_COMMAND: &str = "workspace.searchEverywhere";

pub(crate) const MAX_RESULTS: usize = 200;

pub(crate) const EVERYWHERE_MAX_RESULTS: usize = 1000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchOptions {
    pub(crate) case_sensitive: bool,
    pub(crate) whole_words: bool,
    pub(crate) regular_expression: bool,
    pub(crate) file_mask: String,
}

pub(crate) fn payload(root: &str, query: &str, options: &SearchOptions) -> Value {
    payload_with_limit(root, query, options, MAX_RESULTS)
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MatchKind {
    File,
    Content,
    Type,
    Symbol,
    Other(String),
}

impl MatchKind {
    fn parse(value: &str) -> Self {
        match value {
            "file" => Self::File,
            "content" => Self::Content,
            "type" => Self::Type,
            "symbol" => Self::Symbol,
            other => Self::Other(other.to_string()),
        }
    }

    pub(crate) fn is_file(&self) -> bool {
        matches!(self, Self::File)
    }

    pub(crate) fn is_content(&self) -> bool {
        matches!(self, Self::Content)
    }

    pub(crate) fn is_type(&self) -> bool {
        matches!(self, Self::Type)
    }

    pub(crate) fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    pub(crate) kind: MatchKind,
    pub(crate) path: String,
    pub(crate) line: Option<usize>,
    pub(crate) preview: String,
    pub(crate) symbol_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    MissingData,
    Malformed,
    MissingPath(usize),
}

impl ParseError {
    pub(crate) fn token(self) -> String {
        match self {
            Self::MissingData => "missing_data".to_string(),
            Self::Malformed => "malformed_response".to_string(),
            Self::MissingPath(index) => format!("missing_path_at_{index}"),
        }
    }
}

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
            symbol_name: item
                .get("symbolName")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    Ok(matches)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchRow {
    File {
        path: String,
        hits: usize,
    },
    Hit {
        path: String,
        line: Option<usize>,
        preview: String,
    },
}

pub(crate) fn group(matches: &[Match]) -> Vec<SearchRow> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuickOpenRow {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) parent: String,
}

impl QuickOpenRow {
    fn new(path: String) -> Self {
        let (parent, name) = split_path(&path);
        Self { path, name, parent }
    }
}

fn split_path(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent.to_string(), name.to_string()),
        None => (String::new(), path.to_string()),
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchEverywhereScope {
    All,
    Classes,
    Files,
    Symbols,
    Actions,
    Text,
}

impl SearchEverywhereScope {
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::Classes,
        Self::Files,
        Self::Symbols,
        Self::Actions,
        Self::Text,
    ];

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

    pub(crate) fn is_core_backed(self) -> bool {
        !matches!(self, Self::Actions)
    }

    pub(crate) fn moved(self, offset: isize) -> Self {
        let count = Self::ALL.len() as isize;
        let index = Self::ALL
            .iter()
            .position(|scope| *scope == self)
            .unwrap_or(0) as isize;
        Self::ALL[(index + offset).rem_euclid(count) as usize]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EverywhereRow {
    pub(crate) kind: MatchKind,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) parent: String,
    pub(crate) line: Option<usize>,
    pub(crate) preview: String,
}

impl EverywhereRow {
    pub(crate) fn shows_line(&self) -> bool {
        !self.kind.is_file()
    }

    pub(crate) fn shows_preview(&self) -> bool {
        self.kind.is_content()
    }
}

fn everywhere_row(item: &Match) -> EverywhereRow {
    let (parent, file_name) = split_path(&item.path);
    EverywhereRow {
        kind: item.kind.clone(),
        path: item.path.clone(),
        name: item.symbol_name.clone().unwrap_or(file_name),
        parent,
        line: item.line,
        preview: item.preview.clone(),
    }
}

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

fn named_rows(matches: &[Match], keep: fn(&MatchKind) -> bool) -> Vec<EverywhereRow> {
    matches
        .iter()
        .filter(|item| keep(&item.kind))
        .map(everywhere_row)
        .collect()
}

fn content_rows(matches: &[Match]) -> Vec<EverywhereRow> {
    matches
        .iter()
        .filter(|item| item.kind.is_content())
        .map(everywhere_row)
        .collect()
}

pub(crate) const COMMANDS_EMPTY_KEY: &str = "lithe.searchEverywhere.noMatchingCommands";

pub(crate) fn is_command_query(query: &str) -> bool {
    query.starts_with('/')
}

pub(crate) fn command_query(query: &str) -> Option<&str> {
    query.strip_prefix('/')
}

pub(crate) fn uses_command_catalog(scope: SearchEverywhereScope, query: &str) -> bool {
    !scope.is_core_backed() || is_command_query(query)
}

pub(crate) fn issues_core_request(scope: SearchEverywhereScope, query: &str) -> bool {
    !uses_command_catalog(scope, query) && !query.trim().is_empty()
}

pub(crate) fn action_matches(query: &str, title: &str, subtitle: &str, group: &str) -> bool {
    let normalized: Vec<char> = query
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if normalized.is_empty() {
        return true;
    }
    let candidates = format!("{title} {subtitle} {group}").to_lowercase();
    let mut wanted = normalized.into_iter().peekable();
    for character in candidates.chars() {
        match wanted.peek() {
            None => break,
            Some(&expected) if character == expected => {
                wanted.next();
            }
            Some(_) => {}
        }
    }
    wanted.peek().is_none()
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EverywhereState {
    Idle,
    Loading { query: String, matches: Vec<Match> },
    Ready { query: String, matches: Vec<Match> },
    Failed { query: String, reason: String },
}

impl EverywhereState {
    pub(crate) fn loading(query: impl Into<String>, previous: &[Match]) -> Self {
        Self::Loading {
            query: query.into(),
            matches: previous.to_vec(),
        }
    }

    pub(crate) fn ready(query: impl Into<String>, matches: &[Match]) -> Self {
        Self::Ready {
            query: query.into(),
            matches: matches.to_vec(),
        }
    }

    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query, .. }
            | Self::Ready { query, .. }
            | Self::Failed { query, .. } => query,
        }
    }

    pub(crate) fn matches(&self) -> &[Match] {
        match self {
            Self::Loading { matches, .. } | Self::Ready { matches, .. } => matches,
            Self::Idle | Self::Failed { .. } => &[],
        }
    }

    pub(crate) fn rows(&self, scope: SearchEverywhereScope) -> Vec<EverywhereRow> {
        everywhere_rows(scope, self.matches())
    }

    pub(crate) fn is_ready_for(&self, query: &str) -> bool {
        matches!(self, Self::Ready { query: ready, .. } if ready == query.trim())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QuickOpenState {
    Idle,
    Loading {
        query: String,
        rows: Vec<QuickOpenRow>,
    },
    Ready {
        query: String,
        rows: Vec<QuickOpenRow>,
    },
    Failed {
        query: String,
        reason: String,
    },
}

impl QuickOpenState {
    pub(crate) fn loading(query: impl Into<String>, previous: &[QuickOpenRow]) -> Self {
        Self::Loading {
            query: query.into(),
            rows: previous.to_vec(),
        }
    }

    pub(crate) fn ready(query: impl Into<String>, matches: &[Match]) -> Self {
        Self::Ready {
            query: query.into(),
            rows: quick_open_rows(matches),
        }
    }

    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query, .. }
            | Self::Ready { query, .. }
            | Self::Failed { query, .. } => query,
        }
    }

    pub(crate) fn rows(&self) -> &[QuickOpenRow] {
        match self {
            Self::Loading { rows, .. } | Self::Ready { rows, .. } => rows,
            Self::Idle | Self::Failed { .. } => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchState {
    Idle,
    Loading { query: String },
    Ready { query: String, rows: Vec<SearchRow> },
    Failed { query: String, reason: String },
}

impl SearchState {
    pub(crate) fn loading(query: impl Into<String>) -> Self {
        Self::Loading {
            query: query.into(),
        }
    }

    pub(crate) fn ready(query: impl Into<String>, matches: &[Match]) -> Self {
        Self::Ready {
            query: query.into(),
            rows: group(matches),
        }
    }

    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query } | Self::Ready { query, .. } | Self::Failed { query, .. } => {
                query
            }
        }
    }

    pub(crate) fn rows(&self) -> &[SearchRow] {
        match self {
            Self::Ready { rows, .. } => rows,
            _ => &[],
        }
    }

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

pub(crate) const REPLACE_PREVIEW_COMMAND: &str = "workspace.replacePreview";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReplaceOptions {
    pub(crate) search: SearchOptions,
    pub(crate) preserve_case: bool,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplacementLine {
    pub(crate) line: usize,
    pub(crate) before: String,
    pub(crate) after: String,
    pub(crate) occurrences: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplacementFile {
    pub(crate) path: String,
    pub(crate) lines: Vec<ReplacementLine>,
}

impl ReplacementFile {
    pub(crate) fn occurrences(&self) -> usize {
        self.lines.iter().map(|line| line.occurrences).sum()
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReplaceRow {
    File {
        path: String,
        occurrences: usize,
    },
    Line {
        path: String,
        line: usize,
        before: String,
        after: String,
        occurrences: usize,
    },
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReplaceState {
    Idle,
    Loading {
        query: String,
        files: Vec<ReplacementFile>,
    },
    Ready {
        query: String,
        files: Vec<ReplacementFile>,
    },
    Failed {
        query: String,
        reason: String,
    },
}

impl ReplaceState {
    pub(crate) fn loading(query: impl Into<String>, previous: &[ReplacementFile]) -> Self {
        Self::Loading {
            query: query.into(),
            files: previous.to_vec(),
        }
    }

    pub(crate) fn ready(query: impl Into<String>, files: &[ReplacementFile]) -> Self {
        Self::Ready {
            query: query.into(),
            files: files.to_vec(),
        }
    }

    pub(crate) fn failed(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Failed {
            query: query.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn query(&self) -> &str {
        match self {
            Self::Idle => "",
            Self::Loading { query, .. }
            | Self::Ready { query, .. }
            | Self::Failed { query, .. } => query,
        }
    }

    pub(crate) fn files(&self) -> &[ReplacementFile] {
        match self {
            Self::Loading { files, .. } | Self::Ready { files, .. } => files,
            Self::Idle | Self::Failed { .. } => &[],
        }
    }

    pub(crate) fn rows(&self) -> Vec<ReplaceRow> {
        replace_rows(self.files())
    }

    pub(crate) fn counts(&self) -> (usize, usize) {
        let files = self.files();
        (
            files.len(),
            files.iter().map(ReplacementFile::occurrences).sum(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scheduled {
    pub(crate) generation: u64,
    pub(crate) operation_id: String,
    pub(crate) superseded: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct QueryTimeline {
    generation: u64,
    in_flight: Option<String>,
}

impl QueryTimeline {
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

    pub(crate) fn accepts(&self, issued: u64) -> bool {
        self.generation == issued
    }

    pub(crate) fn settle(&mut self, issued: u64) {
        if self.accepts(issued) {
            self.in_flight = None;
        }
    }

    pub(crate) fn abandon(&mut self) -> Option<String> {
        self.generation = self.generation.wrapping_add(1);
        self.in_flight.take()
    }
}

pub(crate) fn operation_id(generation: u64) -> String {
    format!("lithe-gpui-search-{generation}")
}

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

    #[test]
    fn default_options_match_the_core_defaults() {
        let value = payload("r", "q", &SearchOptions::default());
        assert_eq!(value["caseSensitive"], json!(false));
        assert_eq!(value["wholeWords"], json!(false));
        assert_eq!(value["regularExpression"], json!(false));
        assert_eq!(value["fileMask"], json!(""));
        assert_eq!(value["maxResults"], json!(MAX_RESULTS));
    }

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
        assert_eq!(value["maxResults"], json!(MAX_RESULTS));
    }

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

    #[test]
    fn a_settled_query_is_no_longer_handed_back_for_cancellation() {
        let mut timeline = QueryTimeline::default();
        let first = timeline.schedule();
        timeline.settle(first.generation);
        let second = timeline.schedule();
        assert_eq!(second.superseded, None, "已经落地的请求不需要取消");

        timeline.settle(first.generation);
        let third = timeline.schedule();
        assert_eq!(third.superseded.as_deref(), Some("lithe-gpui-search-2"));
    }

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

    #[test]
    fn quick_open_reuses_the_same_request_shape_as_find_in_files() {
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
        assert_eq!(SEARCH_COMMAND, "workspace.search");
        assert_eq!(EVERYWHERE_COMMAND, "workspace.searchEverywhere");
        assert_ne!(EVERYWHERE_COMMAND, SEARCH_COMMAND);
        assert_eq!(MAX_RESULTS, 200);
    }

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

    #[test]
    fn quick_open_rows_are_unique_by_path() {
        let rows = quick_open_rows(&[path_match("src/A.java"), path_match("src/A.java")]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "src/A.java");
    }

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

    fn action_fixture() -> Vec<CommandAction> {
        use gpui::SharedString;
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

    #[test]
    fn action_matcher_is_a_case_insensitive_whitespace_stripped_subsequence() {
        assert!(action_matches("OPENSET", "Open Settings", "", ""));
        assert!(action_matches("openseT", "Open Settings", "", ""));
        assert!(action_matches("open", "OPEN", "", ""));
        assert!(action_matches("open set", "Open Settings", "", ""));
        assert!(action_matches("  o p e n  ", "Open Settings", "", ""));
        assert!(action_matches("ose", "Open Settings", "", ""));
        assert!(action_matches("tgle", "Toggle Terminal", "", ""));
        assert!(!action_matches("openz", "Open Settings", "", ""));
        assert!(
            !action_matches("gnittes", "Open Settings", "", ""),
            "顺序必须一致：反过来不算命中"
        );
        assert!(
            !action_matches("openq", "Open Settings", "", ""),
            "候选串里没有的字符永远不命中（`openn` 反而**会**命中：`settings` 里还有 n）"
        );
        assert!(action_matches("", "Open Settings", "", ""));
        assert!(action_matches("   ", "Open Settings", "", ""));
        assert!(action_matches("\t\n", "any", "thing", "here"));

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

    #[test]
    fn action_rows_keep_the_catalog_order_and_the_source_matching() {
        let ids = |rows: &[CommandAction]| {
            rows.iter()
                .map(|action| action.id.id().to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids(&action_rows(action_fixture(), "")),
            ["open-settings", "toggle-terminal", "save-buffer"]
        );

        assert_eq!(
            ids(&action_rows(action_fixture(), "opense")),
            ["open-settings"],
            "过滤不许改变顺序、也不许命中别的条目"
        );

        assert!(action_rows(action_fixture(), "zzzz").is_empty());

        let rows = action_rows(action_fixture(), "buffer");
        assert_eq!(rows.len(), 1, "只有 Save Buffer 的候选串里有 b…u…f…f…e…r");
        assert_eq!(rows[0].label.as_ref(), "Save Buffer");
        assert_eq!(rows[0].description.as_ref(), "Write the active buffer");
        assert_eq!(rows[0].id.id(), "save-buffer");
    }

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

    #[test]
    fn actions_and_command_mode_never_issue_a_core_request() {
        assert!(!SearchEverywhereScope::Actions.is_core_backed());
        assert_eq!(
            SearchEverywhereScope::ALL
                .iter()
                .filter(|scope| !scope.is_core_backed())
                .count(),
            1,
            "六个档里只有 Actions 走宿主命令目录"
        );

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

        for scope in SearchEverywhereScope::ALL {
            for query in ["/", "/open", "/open settings"] {
                assert!(
                    !issues_core_request(scope, query),
                    "{scope:?} 在 `/` 模式下不许发 Core 请求（query={query:?}）"
                );
                assert!(uses_command_catalog(scope, query));
            }
        }

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

    #[test]
    fn the_actions_scope_never_projects_core_matches() {
        let matches = everywhere_fixture();
        assert!(everywhere_rows(SearchEverywhereScope::Actions, &matches).is_empty());
        assert!(rows_ready(SearchEverywhereScope::Actions).is_empty());
        assert_eq!(rows_ready(SearchEverywhereScope::Files).len(), 2);
        assert_eq!(rows_ready(SearchEverywhereScope::Classes).len(), 1);
        assert_eq!(rows_ready(SearchEverywhereScope::Symbols).len(), 2);
        assert_eq!(rows_ready(SearchEverywhereScope::Text).len(), 2);
    }

    fn rows_ready(scope: SearchEverywhereScope) -> Vec<EverywhereRow> {
        EverywhereState::ready("alpha", &everywhere_fixture()).rows(scope)
    }

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

        assert!(!EverywhereState::loading("alpha", &[]).is_ready_for("alpha"));
        assert!(!EverywhereState::Idle.is_ready_for("alpha"));
        assert!(!EverywhereState::failed("alpha", "invalid_request").is_ready_for("alpha"));
    }

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

        assert_eq!(MAX_RESULTS, 200);
        assert_eq!(EVERYWHERE_MAX_RESULTS, 1000);
        assert!(
            EVERYWHERE_MAX_RESULTS > MAX_RESULTS + 2 * 50,
            "预留的符号预算（types 50 + symbols 50）必须真的落在抬高后的预算里"
        );
        assert!(EVERYWHERE_MAX_RESULTS <= 10_000, "Core 自己夹到 10_000");
        assert_eq!(everywhere["maxResults"], json!(1000));
    }

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
        assert_eq!(value["caseSensitive"], json!(false));
        assert_eq!(value["wholeWords"], json!(false));
        assert_eq!(value["regularExpression"], json!(false));
        assert_eq!(value["preserveCase"], json!(false));
        assert_eq!(value["fileMask"], json!(""));
    }

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
        let files = parse_replace_preview(Some(&json!({
            "files": [{ "path": "a", "matches": [{ "line": 1, "before": "x", "after": "y" }] }]
        })))
        .expect("次数缺失不是契约异常");
        assert_eq!(files[0].lines[0].occurrences, 0);
    }

    #[test]
    fn an_empty_replace_preview_is_ready_not_failed() {
        let files = parse_replace_preview(Some(&json!({ "files": [] }))).expect("空结果是成功");
        let state = ReplaceState::ready("needle", &files);
        assert_eq!(state.rows(), Vec::new());
        assert_eq!(state.counts(), (0, 0));
        assert!(!matches!(state, ReplaceState::Failed { .. }));
    }

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
