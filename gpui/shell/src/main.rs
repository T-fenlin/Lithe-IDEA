//! P1 冒烟宿主：验证 GPUI 宿主能否在进程内直接驱动 Rust Core。
//!
//! 只做两件事：
//! 1. 用 GPUI Kit 打开一个真实窗口；
//! 2. 调用 `lithe_core::execute_json` 执行 `core.ping` 与 `workspace.snapshot`，
//!    把结果显示在窗口里，并把原始响应写到证据目录。
//!
//! 工作区根与证据目录都从命令行参数取，不硬编码任何机器路径。
//! 命令的字段形状取自 Core 源码（契约文档只列出了命令名，没有给字段表）：
//! `core.ping` 忽略 payload，返回 `{protocolVersion, coreVersion}`；
//! `workspace.snapshot` 的 payload 为 `{root, hiddenDirectoryNames?, hiddenFilePatterns?}`，
//! 返回 `{root: WorkspaceNode, files: string[]}`。

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use gpui_kit::component::{ActiveTheme as _, Root};
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
};

/// 参数缺失或非法时打印的用法说明。
const USAGE: &str = "\
lithe-gpui-shell <workspace-root> [--evidence-dir DIR] [--max-rows N]

  workspace-root   传给 workspace.snapshot 的工作区根目录
  --evidence-dir   原始响应与摘要的输出目录（默认 .artifacts/p1）
  --max-rows       窗口里最多显示多少条文件路径（默认 200）
";

/// 一次运行的输入参数。
struct Options {
    /// 工作区根，直接作为 `workspace.snapshot` 的 `root` 字段。
    workspace_root: PathBuf,
    /// 证据输出目录，写入原始响应与摘要，便于人工核对。
    evidence_dir: PathBuf,
    /// 窗口里最多渲染多少条路径，避免用数万行撑爆界面。
    max_rows: usize,
}

impl Options {
    fn parse() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
        let first = args.next().ok_or_else(|| USAGE.to_string())?;
        if first == "-h" || first == "--help" {
            return Err(USAGE.to_string());
        }

        let mut evidence_dir = PathBuf::from(".artifacts/p1");
        let mut max_rows = 200usize;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--evidence-dir" => {
                    let value = args.next().ok_or("--evidence-dir 缺少值")?;
                    evidence_dir = PathBuf::from(value);
                }
                "--max-rows" => {
                    let value = args.next().ok_or("--max-rows 缺少值")?;
                    max_rows = value
                        .parse()
                        .map_err(|error| format!("--max-rows 解析失败：{error}"))?;
                }
                other => return Err(format!("未知参数 {other}\n\n{USAGE}")),
            }
        }

        Ok(Self {
            workspace_root: PathBuf::from(first),
            evidence_dir,
            max_rows,
        })
    }
}

/// `core.ping` 的结论。
struct PingSummary {
    protocol_version: i64,
    core_version: String,
}

/// `workspace.snapshot` 的结论。
struct SnapshotSummary {
    file_count: usize,
    rows: Vec<String>,
    raw_bytes: usize,
}

/// 一次探测的完整结果。与 GPUI 无关，便于单独推理与复用。
struct ProbeOutcome {
    ping: Result<PingSummary, String>,
    snapshot: Result<SnapshotSummary, String>,
    elapsed_millis: u128,
}

/// 组装一条带信封字段的请求。P1 就带上 `operationId` 与超时，避免 P2 再补。
fn envelope(id: &str, timeout_millis: u64, command: &str, payload: serde_json::Value) -> String {
    serde_json::json!({
        "id": id,
        "operationId": id,
        "timeoutMilliseconds": timeout_millis,
        "command": command,
        "payload": payload,
    })
    .to_string()
}

/// 执行一条命令并做信封级校验，返回解析后的响应与原始文本。
fn execute(request: &str) -> Result<(serde_json::Value, String), String> {
    let raw = lithe_core::execute_json(request);
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("响应不是合法 JSON：{error}"))?;

    if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        return Ok((value, raw));
    }

    let code = value
        .pointer("/error/code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let message = value
        .pointer("/error/message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(无消息)");
    Err(format!("{code}: {message}"))
}

/// 把证据写入目录；返回写入的字节数。
fn write_evidence(dir: &Path, name: &str, content: &str) -> Result<usize, String> {
    fs::create_dir_all(dir).map_err(|error| format!("创建证据目录失败：{error}"))?;
    let path = dir.join(name);
    fs::write(&path, content).map_err(|error| format!("写入 {path:?} 失败：{error}"))?;
    Ok(content.len())
}

/// 同步执行完整探测。刻意不做任何 GPUI 调用，方便在后台线程运行。
fn run_probe(options: &Options) -> ProbeOutcome {
    let started = Instant::now();

    let ping = (|| -> Result<PingSummary, String> {
        let request = envelope("p1-ping", 30_000, "core.ping", serde_json::json!({}));
        let (value, raw) = execute(&request)?;
        write_evidence(&options.evidence_dir, "ping.json", &raw)?;
        Ok(PingSummary {
            protocol_version: value
                .pointer("/data/protocolVersion")
                .and_then(serde_json::Value::as_i64)
                .ok_or("响应缺少 data.protocolVersion")?,
            core_version: value
                .pointer("/data/coreVersion")
                .and_then(serde_json::Value::as_str)
                .ok_or("响应缺少 data.coreVersion")?
                .to_string(),
        })
    })();

    let snapshot = (|| -> Result<SnapshotSummary, String> {
        let root = options
            .workspace_root
            .to_str()
            .ok_or("工作区根不是合法 UTF-8 路径")?
            .to_string();
        let request = envelope(
            "p1-snapshot",
            120_000,
            "workspace.snapshot",
            serde_json::json!({ "root": root }),
        );
        let (value, raw) = execute(&request)?;
        let raw_bytes = write_evidence(&options.evidence_dir, "snapshot.json", &raw)?;

        let files = value
            .pointer("/data/files")
            .and_then(serde_json::Value::as_array)
            .ok_or("响应缺少 data.files 数组")?;

        let rows = files
            .iter()
            .filter_map(serde_json::Value::as_str)
            .take(options.max_rows)
            .map(str::to_string)
            .collect();

        Ok(SnapshotSummary {
            file_count: files.len(),
            rows,
            raw_bytes,
        })
    })();

    ProbeOutcome {
        ping,
        snapshot,
        elapsed_millis: started.elapsed().as_millis(),
    }
}

impl ProbeOutcome {
    fn headline(&self) -> String {
        let ping = if self.ping.is_ok() { "ok" } else { "FAILED" };
        let snapshot = if self.snapshot.is_ok() { "ok" } else { "FAILED" };
        format!(
            "P1 探测完成：core.ping={ping}，workspace.snapshot={snapshot}，耗时 {} ms",
            self.elapsed_millis
        )
    }

    /// 打印给终端的一行机器可读结论，便于外部判断成败。
    fn report(&self) {
        println!("{}", self.headline());
        match &self.ping {
            Ok(ping) => println!(
                "P1_PING_OK protocolVersion={} coreVersion={}",
                ping.protocol_version, ping.core_version
            ),
            Err(error) => println!("P1_PING_FAILED {error}"),
        }
        match &self.snapshot {
            Ok(snapshot) => println!(
                "P1_SNAPSHOT_OK files={} rawBytes={}",
                snapshot.file_count, snapshot.raw_bytes
            ),
            Err(error) => println!("P1_SNAPSHOT_FAILED {error}"),
        }
    }
}

/// 承载探测状态的窗口视图。
struct CoreProbe {
    status: SharedString,
    ping_line: SharedString,
    snapshot_line: SharedString,
    rows: Vec<SharedString>,
}

impl CoreProbe {
    fn new(options: Options, cx: &mut Context<Self>) -> Self {
        let view = Self {
            status: "正在探测 Rust Core…".into(),
            ping_line: String::new().into(),
            snapshot_line: String::new().into(),
            rows: Vec::new(),
        };

        // 重活在后台线程跑，结果回到前台更新实体：窗口在命令执行期间保持可交互。
        // 注意：gpui-kit 技能文档里的 `.then(cx.spawn(..))` 写法属于尚未发布的 0.7.0；
        // 0.6.6 的 `Task` 没有 `then`，正确写法是在前台任务里 await 后台任务。
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_this, cx| {
            let outcome = cx.background_spawn(async move { run_probe(&options) }).await;
            let _ = entity.update(cx, |view, cx| {
                view.apply(outcome);
                cx.notify();
            });
        })
        .detach();

        view
    }

    fn apply(&mut self, outcome: ProbeOutcome) {
        outcome.report();

        self.status = outcome.headline().into();

        self.ping_line = match &outcome.ping {
            Ok(ping) => format!(
                "core.ping → protocolVersion {}, coreVersion {}",
                ping.protocol_version, ping.core_version
            )
            .into(),
            Err(error) => format!("core.ping → 失败：{error}").into(),
        };

        self.snapshot_line = match &outcome.snapshot {
            Ok(snapshot) => format!(
                "workspace.snapshot → 共 {} 个文件，原始响应 {} 字节，下面显示前 {} 条",
                snapshot.file_count,
                snapshot.raw_bytes,
                snapshot.rows.len()
            )
            .into(),
            Err(error) => format!("workspace.snapshot → 失败：{error}").into(),
        };

        if let Ok(snapshot) = &outcome.snapshot {
            self.rows = snapshot.rows.iter().map(|row| row.as_str().into()).collect();
        }
    }
}

impl Render for CoreProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_2()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.status.clone())
            .child(self.ping_line.clone())
            .child(self.snapshot_line.clone())
            .child(
                div()
                    .id("rows")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_1()
                    .overflow_y_scroll()
                    .children(self.rows.iter().cloned()),
            )
    }
}

fn main() {
    let options = match Options::parse() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    // 0.6.6 的官方启动顺序（见 gpui-kit 的 lib.rs 文档示例）：
    // application().run() -> init(cx) -> 在 App 上 spawn 异步任务 -> open_window()。
    // 视图必须用 Root::new 包起来，Root 才提供对话框、浮层与通知能力。
    // 注意 `with_assets` 在 0.6.6 不存在（同样属于 0.7.0 文档），不要照抄技能里的那段。
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(gpui_kit::WindowOptions::default(), move |window, cx| {
                let view = cx.new(|cx| CoreProbe::new(options, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
