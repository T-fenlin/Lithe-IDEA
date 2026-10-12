# 构建与质量实测原始记录（娄码通）

配套报告：`reports/gpui-core-impl-optimization-2026-10.md`
环境：Windows，8 逻辑核，cargo 1.97.0 / rustc 1.97.0，基线 `3085f5d1`
目标目录：`D:\developmentProjects\rust\Lithe-IDEA\rust\lithe-gpui\target`（`.gitignore:6` 已忽略）
环境变量：`RUST_MIN_STACK=67108864`、`CARGO_PROFILE_DEV_DEBUG=0`、`-j 4`

---

## 1. 构建耗时

命令模板：
```powershell
$env:CARGO_TARGET_DIR='D:\developmentProjects\rust\Lithe-IDEA\rust\lithe-gpui\target'
$env:RUST_MIN_STACK='67108864'; $env:CARGO_PROFILE_DEV_DEBUG='0'
cargo <check|build> -p lithe-gpui-app -j 4
```
`workdir` = `rust/lithe-gpui`

| 标签 | 命令 | 秒 | 重编单元数 | 第一方重编 crate |
|---|---|---|---|---|
| 冷全量 | `cargo check -p lithe-gpui-app -j 4` | 547.1 | 549 | 全部 13 |
| 全量重建 | `cargo build -p lithe-gpui-app -j 4` | 548.8 | 399 | 全部 13 |
| 增量无改动 A | `cargo check` | 1.7 | 0 | — |
| 增量无改动 B | `cargo check` | 1.4 | 0 | — |
| 增量无改动 | `cargo build` | 1.9 | 0 | — |
| 增量无改动（build 预热后） | `cargo build` | 1.9 | 0 | — |
| 改 `terminal/src/lib.rs` | `cargo build` | **8.6** | 3 | app, terminal, workbench |
| 改 `workbench/src/lib.rs` | `cargo build` | **8.7** | 2 | app, workbench |
| 改 `settings/src/lib.rs` | `cargo check` | **3.6** | 3 | app, settings, workbench |
| 改 `shared/src/lib.rs` | `cargo check` | **221.2** | 12 | 全部 12（除 languages） |
| 改 `shared/src/lib.rs` | `cargo build` | **281.0** | 12 | 全部 12 |
| 改 `../lithe-ui/src/lib.rs` | `cargo check` | **231.7** | 14 | 全部 13 + lithe-ui |
| 改 `../lithe-ui/src/lib.rs` | `cargo build` | **671.4** | 15 | 全部 13 + lithe-ui |

「重编单元数」= `cargo` 输出里 `Compiling`/`Checking` 行的去重 crate 数。

---

## 2. 历史构建时间线（从既有 target 的产物 mtime 重建）

既有目录 `D:/developmentProjects/rust/lithe-gpui-target`（11.88 GB，**只读**）：

- 首个重建产物：`2026-10-12 07:08:24`
- `Lithe.exe` 落盘：`2026-10-12 07:23:38`
- **墙钟 = 15.25 min**，重编 1077 个 rlib/rmeta（该目录内 100%，即全量重建）

关键单元耗时（首→末产物 mtime 之差）：

| 单元 | 起 | 止 | 秒 |
|---|---|---|---|
| `windows_sys` | 07:08:44 | 07:16:08 | 444 |
| `windows` | 07:13:36 | 07:14:18 | 42（产物 242 MB） |
| `tokio` | 07:10:03 | 07:10:12 | 9（产物 48 MB） |
| `gpui` | 07:14:07 | 07:18:08 | **241**（产物 202 MB） |
| `gpui_windows` | 07:14:12 | 07:14:26 | 14（产物 92 MB） |
| **`lithe_ui`** | 07:16:26 | 07:18:55 | **149**（rlib 1222.5 MB @ debug=1） |
| `lithe_core` | 07:17:33 | 07:17:33 | —（rlib 285.7 MB） |
| `lithe_gpui_shared` | 07:21:55 | 07:22:02 | 7 |
| `lithe_gpui_workbench` | 07:22:36 | 07:22:48 | 12（rlib 228.9 MB） |
| `Lithe.exe` | — | 07:23:38 | —（89.3 MB） |

**07:18:55 → 07:21:55 有约 3 分钟的间隙**，期间只有 67 个文件变动，均为 `lithe_ui` 的
增量编译中间产物（`.o`、`query-cache.bin`、`dep-graph.bin`），无其他 crate 产物。
即该段是 `lithe_ui` 编译的延续（其 rlib 于 07:18:55 落盘，`output-lib-lithe_ui` 于 07:19:00 落盘）。

---

## 3. 依赖分层（`cargo metadata --offline --format-version 1 --no-deps`）

| crate | 第一方直接依赖 | 反向依赖数 | 反向依赖者 |
|---|---|---|---|
| lithe-gpui-shared | lithe-ui, lithe-core | **11** | settings, git, explorer, workbench, java, terminal, run, app, editor, extensions, notify |
| lithe-gpui-settings | lithe-ui, shared, run | 2 | workbench, app |
| lithe-gpui-languages | lithe-ui | 2 | app, editor |
| lithe-gpui-java | shared | 2 | workbench, editor |
| lithe-gpui-run | lithe-ui, shared | 2 | settings, workbench |
| lithe-gpui-notify | shared | 2 | workbench, editor |
| lithe-gpui-git | lithe-ui, shared | 1 | workbench |
| lithe-gpui-explorer | lithe-ui, shared | 1 | workbench |
| lithe-gpui-workbench | 10 个（见下） | 1 | app |
| lithe-gpui-terminal | lithe-ui, shared | 1 | workbench |
| lithe-gpui-editor | lithe-ui, notify, shared, java | 1 | workbench |
| lithe-gpui-extensions | shared | 1 | workbench |
| lithe-gpui-app | languages, lithe-ui, settings, shared, workbench | 0 | — |

`workbench` 的直接依赖：editor, explorer, extensions, git, java, run, notify, settings, shared, terminal。

---

## 4. 静态计数

### 4.1 unwrap/expect（按首个 `#[cfg(test)]` 行切分）

| 口径 | 计数 |
|---|---|
| 全部 | **541** |
| 生产（`#[cfg(test)]` 之前） | **40** |
| 测试 | 501 |

生产 40 处的构成：`workbench/tests/*.rs` 4 文件共 29、`shared/src/workspace_config/test_repo.rs` 4、
`app/src/main.rs` 2、`app/build.rs` 1、`workbench/src/project_menu.rs` 1、
`workbench/src/menu_bar.rs` 1、`settings/src/dialog.rs` 1。

简报点名的四个文件：

| 文件 | 总数 | 生产数 | 首个 `#[cfg(test)]` 行 |
|---|---|---|---|
| `settings/src/keymap_io.rs` | 12 | **0** | 220 |
| `settings/src/recent_projects.rs` | 18 | **0** | — |
| `run/src/plan.rs` | 4 | **0** | — |
| `git/src/changes.rs` | 3 | **0** | — |

### 4.2 其他静态计数

| 项 | 计数 |
|---|---|
| `#[allow(dead_code)]` | **18** |
| `#[allow(unused*)]` | 0 |
| `#[allow(never*)]` | 0 |
| `todo!()` / `unimplemented!()` / `unreachable!()` | 0 / 0 / 0 |
| `TODO`/`FIXME`/`HACK`/`XXX` 注释 | **0** |
| `panic!(` | 71（含测试） |
| `let _ = ` | 325（其中右侧为 Result 型调用 78） |
| `.ok()` | 51 |
| `unwrap_or_default()` | 119 |
| `eprintln!` | 336 |
| `unsafe`（lithe-gpui） | **12，全部在 `settings/src/paths.rs` 的 `#[cfg(test)]` 段** |
| `unsafe`（lithe-ui） | 33（生产 17：`shell/native.rs` 10、`forms/color/dropper.rs` 5、`buttons/share.rs` 2；均 macOS-gated） |
| `.lock().unwrap()/.expect()` | 11，**全部在测试代码** |

### 4.3 并发原语

| 原语 | 计数 |
|---|---|
| `cx.spawn` | 60 |
| `Arc<` | 47 |
| `Rc<` | 78 |
| `RefCell` | 19 |
| `Mutex` | 69 |
| **`RwLock`** | **0** |
| `std::thread::spawn` | 9 |
| `mpsc` | 16 |
| `std::thread::sleep` | 25 |
| `tokio::spawn` | **0** |
| **`block_on`** | **0** |
| `oneshot` | **0** |
| `std::fs::` | 267 |
| `std::process::Command` | 4 |

### 4.4 测试

| crate | `#[test]` |
|---|---|
| workbench | 244 |
| settings | 216 |
| shared | 80 |
| editor | 76 |
| git | 49 |
| java | 45 |
| notify | 32 |
| run | 16 |
| explorer | 13 |
| app | 12 |
| terminal | 8 |
| extensions | 6 |
| languages | 6 |
| **合计** | **803** |

`#[gpui::test]`：15。

---

## 5. 规模

| crate | 源码 KB（`*.rs`+`*.toml`+`*.json`） |
|---|---|
| workbench | 968.8 |
| settings | 550.2 |
| shared | 248.0 |
| git | 223.0 |
| editor | 217.2 |
| java | 147.8 |
| run | 71.0 |
| app | 62.3 |
| terminal | 53.2 |
| explorer | 46.4 |
| notify | 28.9 |
| languages | 12.5 |
| extensions | 10.9 |

最大单文件（行数）：`settings/src/dialog.rs` 4036、`workbench/src/workspace.rs` 3975、
`workbench/src/debug.rs` 3287、`editor/src/editor_view.rs` 2949、`workbench/src/menu_bar.rs` 2766、
`workbench/src/debug_transport.rs` 2264、`settings/src/project.rs` 2116、
`git/src/log_view.rs` 2015、`workbench/src/search_model.rs` 1959。

`rust/lithe-ui`：45 个顶层模块，约 1090 个 `.rs`，8.32 MB。
宿主实际引用 17 个模块（2698 KB）；未被引用 29 个（**4189 KB**，即 61%）。

---

## 6. 基准：`group()` / `quick_open_rows()` 线性 vs 哈希

复刻 `crates/workbench/src/search_model.rs:152-171` 与 `:205-217` 的确切算法，
`opt-level=3`，输入形状「每 10 条命中含 1 条文件行」，两个实现用 `assert_eq!` 钉住语义等价。

```
       N group_linear_ms  group_hash_ms    speedup   qopen_lin_ms  qopen_hash_ms    speedup
    1000           0.49           0.07       7.2x           0.00           0.01       0.4x
    5000           6.96           0.23      29.8x           0.06           0.04       1.6x
   10000          22.43           0.47      47.6x           0.21           0.05       3.9x
   20000          98.87           1.73      57.1x           1.34           0.26       5.2x
```

对照可达规模：`MAX_RESULTS = 200`（`search_model.rs:11`）、
`EVERYWHERE_MAX_RESULTS = 1000`（`:13`）、Core 上限 `min(10_000)`（`lithe-core/src/project/files.rs:204`）。

---

## 7. 沙盒探测记录

| 探测 | 结果 |
|---|---|
| 读 `D:/developmentProjects/rust/lithe-gpui-target` | **可读**（8250 文件 / 11.88 GB / 723 个 fingerprint 目录） |
| 写 `D:/developmentProjects/rust/lithe-gpui-target/__probe.txt` | **拒绝访问** |
| `CARGO_TARGET_DIR` 指向该目录跑 cargo | `error: failed to open: ...\debug\.cargo-build-lock` / `拒绝访问 (os error 5)` |
| 在 `rust/lithe-gpui` 内建 crate + 工作区内 target | `cargo run` **成功**，exit=0，2 s |
| 该 exe 在自身目录旁 `File::create` | **`BESIDE_WRITE_OK`** |
| 该 exe 在自身目录旁 `create_dir_all` | **`BESIDE_MKDIR_OK`** |
| 该 exe `Command::new("cmd")` 读 stdout | **`CHILD_OK child-ok`** |
| `git check-ignore -v rust/lithe-gpui/target` | `.gitignore:6:rust/lithe-gpui/target/` ✓ |

探测产物已全部清理（`__probe_crate`、`__probe2`、`reports/gpui-core-impl-evidence/_bench`）。

## 8. 机器状态

| 项 | 值 |
|---|---|
| 逻辑处理器 | 8 |
| Committed Bytes | 21.57 GB |
| Commit Limit | 32.52 GB |
| `Available MBytes` | 0.00（该 perf counter 在此环境不可信） |
| `explorer.exe` | **未在运行** |
| 内存占用前 12 进程 | DeepSeek Harness 583 MB、QQ 368 MB、msedgewebview2 321 MB… |
| 磁盘 | C: 23.4 GB 可用；D: 158.2 GB 可用 |
