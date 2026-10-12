# GPUI 统一前端 · 核心实现层优化分析（娄码通）

**基线**：`3085f5d1`，分支 `feat/menu-bar-opaque`，Windows 主机，cargo 1.97.0 / rustc 1.97.0
**范围**：`rust/lithe-gpui`（14 crate / 123 个 `.rs`）+ 平级 `rust/lithe-ui`（组件库）
**口径**：只做分析与方案产出。**未改任何生产代码、未建分支、未提交。**

> 本报告中的所有数字均来自本次实测或可复查的静态检索。凡未实测的，逐条标注「未实测」。

---

## 0. 先说三条会改变排期结论的实测

台账与既有平替计划把「构建一次约 15 分钟」当成每日 3～4 次的硬约束。本次实测表明，
**这条约束只在「全量重建」时成立；日常改一个业务 crate 的增量构建是秒级**：

| 场景 | 命令 | 实测耗时 | 重编单元数 |
|---|---|---|---|
| 全量冷构建（无任何缓存） | `cargo check -p lithe-gpui-app -j 4` | **547.1 s ≈ 9m07s** | 549 |
| 全量重建（`target` 被清/污染后） | `cargo build -p lithe-gpui-app -j 4` | **548.8 s ≈ 9m09s** | 399 |
| 增量：无改动 | `cargo check -p lithe-gpui-app` | **1.7 s / 1.4 s** | 0 |
| 增量：无改动 | `cargo build -p lithe-gpui-app` | **1.9 s** | 0 |
| 增量：改 `terminal/src/lib.rs` | `cargo build` | **8.6 s** | 3 |
| 增量：改 `workbench/src/lib.rs` | `cargo build` | **8.7 s** | 2 |
| 增量：改 `settings/src/lib.rs` | `cargo check` | **3.6 s** | 3 |
| 增量：改 `shared/src/lib.rs` | `cargo build` | **281.0 s ≈ 4m41s** | 12 |
| 增量：改 `lithe-ui/src/lib.rs` | `cargo build` | **671.4 s ≈ 11m11s** | 15 |

**结论**：把「15 分钟」当成每次迭代的成本是**高估**。真实成本曲线是三档：

- **改 `workbench` / `terminal` / `settings` / `git` 等宿主 crate 的私有代码 → 3～9 秒**
- **改 `shared`（11 个 crate 依赖它）→ 约 4 分 41 秒**
- **改 `lithe-ui` 或全量重建 → 9～11 分钟**

这条区分直接决定排期：**只要把「改 shared」和「改 lithe-ui」的批次单独排、成批做，
日常迭代次数可以从「每天 3～4 次」提到「每天几十次」**。

台账 §3 记的「2026-10-11 实测 14m48s」与我实测的 11m11s（lithe-ui）/ 9m09s（全量）
同量级，差异来自 `CARGO_PROFILE_DEV_DEBUG=0`（本次全程开启，既有配方也建议开）。

### 0.1 一条与台账不符的事实：工作区内的 target 是能用的

台账 §3 与平替计划硬约束 2 都写：**工作区被 ACL 限制，其中的可执行文件运行时被拒绝一切写入，
构建必须绕开工作区**。本次实测**不支持**这条的一般形式：

- `rust/lithe-gpui` 内新建 crate、`CARGO_TARGET_DIR` 指向工作区内 → `cargo run` **成功**（exit=0，2 s）
- 该 exe 在自身目录旁 `File::create` → **`BESIDE_WRITE_OK`**
- 该 exe 在自身目录旁 `create_dir_all` → **`BESIDE_MKDIR_OK`**
- 该 exe `Command::new("cmd")` 起子进程并读 stdout → **`CHILD_OK child-ok`**

也就是说，在**我这次会话的沙盒策略下**，工作区内的产物写入与子进程都是通的。
我**无法**复核台账当时遇到的拒绝——它可能是更早的沙盒策略、或特定的 `explorer.exe` 状态所致。
**这条差异必须由主理人确认，因为它直接决定构建配方是否还需要 `CARGO_TARGET_DIR` 绕行。**

> ⚠️ 我自己的沙盒**不能写** `D:/developmentProjects/rust/lithe-gpui-target`（已存在的 11.88 GB 目录）：
> `error: failed to open: ...\debug\.cargo-build-lock` / `拒绝访问 (os error 5)`。
> 因此本次所有构建实测都在 `rust/lithe-gpui/target`（已被 `.gitignore:6` 忽略）中完成，
> **与仓库既有配方用的目录不同**，结论的可迁移性见 §5 风险。

---

## 1. `#[allow(dead_code)]` 精确清点

全目录共 **18 处**，分布在 6 个文件。逐条核对「实现是否已在、只缺调用点」：

| # | 位置 | 对象 | 判定 | 说明 |
|---|---|---|---|---|
| 1 | `crates/explorer/src/explorer_view.rs:85` | 字段 `search_open: bool` | **实现已在，只缺调用点** | 同文件 `:348 toggle_search` 写它、`:363 clear_search` 清它，两个方法本身也是 `dead_code` |
| 2 | `crates/explorer/src/explorer_view.rs:348` | `fn toggle_search` | **实现已在，只缺调用点** | 完整实现（含 focus 逻辑），无调用者 ⇒ 项目树搜索框点不出来 |
| 3 | `crates/explorer/src/explorer_view.rs:363` | `fn clear_search` | **实现已在，只缺调用点** | 同上 |
| 4 | `crates/editor/src/navigation.rs:291` | `JumpHistory::can_go_back` | **实现已在，只缺调用点** | `pub(crate)`，无调用者 |
| 5 | `crates/editor/src/navigation.rs:296` | `JumpHistory::can_go_forward` | **实现已在，只缺调用点** | 同上 |
| 6 | `crates/editor/src/editor_view.rs:1082` | `EditorView::go_back` | **实现已在，只缺调用点** | 完整实现；台账 §2.2 已指出「无 action 定义与键位绑定」 |
| 7 | `crates/editor/src/editor_view.rs:1093` | `EditorView::go_forward` | **实现已在，只缺调用点** | 同上 |
| 8 | `crates/workbench/src/workspace.rs:1532` | `ShellWorkspace::clear_breakpoints` | **实现已在，只缺调用点** | `pub(crate)`，转发到 `debug` 面板 |
| 9 | `crates/git/src/identity.rs:104` | `parse_setup` | **实现已在，只缺调用点** | 与台账 §2.2「Git 身份配置页」同源：解析器齐备，无 UI 消费者 |
| 10 | `crates/workbench/src/debug.rs:24` | `const SET_BREAKPOINTS_COMMAND` | **实现已在，只缺调用点** | 常量定义在、无使用点 |
| 11 | `crates/workbench/src/debug.rs:358` | `SteppingFilter::set` | **实现已在，只缺调用点** | 步进过滤器选项的 setter，无调用者 |
| 12–14 | `crates/terminal/src/session.rs:298 / :312 / :321` | `profiles()` / `replace_profiles()` / `add_profile()` | **实现已在，只缺调用点** | 终端 profile 的读/换/增三件套，全部无调用者 |
| 15 | `crates/terminal/src/profile.rs:56` | `SHELL_IDS` | **实现已在，只缺调用点** | shell id 白名单，无读取点 |
| 16–18 | `crates/terminal/src/constants.rs:77 / :79 / :86` | `CHROME_RADIUS` / `STATUS_LINE_HEIGHT` / `TERMINAL_PADDING_INLINE` | **实现已在，只缺调用点** | `:82-85` 的注释自己写明「**当前没有读点**……迁到组件库后逐行内衬归库自己」——**这是迁移遗留，不是缺口** |

**统计口径**：18 处中 **17 处属于「已实现但零调用点」**，**1 处（`TERMINAL_PADDING_INLINE`）是组件库迁移遗留的死常量**。
**没有任何一处是「完全未实现」。**

**这条对排期有意义**：`dead_code` 不是「欠实现」的信号，而是**接线债务**的清单。
它和台账 §2 的 44 个 `NotWired` 菜单入口指向同一类问题——**代码写完了，入口没接**。

补充：全目录 `todo!()` / `unimplemented!()` / `unreachable!()` **各 0 处**，
`TODO` / `FIXME` / `HACK` / `XXX` 注释 **0 处**。没有「留坑待填」的代码标记，
说明缺口是**入口级**的，不是实现级的。

---

## 2. `unwrap()` / `expect()` 分布：假设被推翻

我原本的假设是「`keymap_io.rs` 等高计数主要来自 `#[cfg(test)]` 模块」。**实测推翻了这个假设的方向，但结论反过来**：

按「首个 `#[cfg(test)]` 行」切分生产代码与测试代码后：

| 口径 | `.unwrap()` + `.expect()` 总数 |
|---|---|
| **全部代码** | **541** |
| **生产代码**（`#[cfg(test)]` 之前） | **40** |
| 测试代码 | 501 |

**生产代码只占 7.4%**。任务简报里点名的四个文件，实测结果如下：

| 文件 | 简报给的数 | 实测总数 | **实测生产数** | 判定 |
|---|---|---|---|---|
| `crates/settings/src/keymap_io.rs` | 12 | 12 | **0** | **全部在 `#[cfg(test)] mod tests`（`:220` 起）内** |
| `crates/settings/src/recent_projects.rs` | 10 | 18 | **0** | 同上 |
| `crates/run/src/plan.rs` | 4 | 4 | **0** | 同上 |
| `crates/git/src/changes.rs` | 1 | 3 | **0** | 同上 |

**结论：简报里作为「非测试代码 unwrap 分布」给出的四个证据，实际上全部是测试代码。**
这不是「已经拖后腿的部分」。我复核了 `keymap_io.rs` 全文 435 行：`:220` 是
`#[cfg(test)] mod tests` 的起点，其后的 12 处 `unwrap` 全在该模块内（例如 `:254`
`serde_json::to_string_pretty(...).unwrap()`）。**这是一条需要主理人回传给简报来源的更正。**

### 2.1 生产代码 40 处 unwrap/expect 的实际构成

| 文件 | 生产数 | 性质 |
|---|---|---|
| `crates/workbench/tests/*.rs`（4 个文件） | 29 | **集成测试**（`tests/` 目录，非 `#[cfg(test)]` 切分口径能覆盖） |
| `crates/shared/src/workspace_config/test_repo.rs` | 4 | 测试辅助设施 |
| `crates/app/src/main.rs` | 2 | 启动期 |
| `crates/app/build.rs` | 1 | 构建脚本 |
| `crates/workbench/src/project_menu.rs` | 1 | 生产 |
| `crates/workbench/src/menu_bar.rs` | 1 | 生产 |
| `crates/settings/src/dialog.rs` | 1 | 生产 |

**真正的生产路径 unwrap/expect 只有 4 处**（`project_menu.rs` 1、`menu_bar.rs` 1、
`dialog.rs` 1、`main.rs` 2 —— `main.rs` 的 2 处属启动期，启动期 panic 是可接受的失败模式）。

**结论：错误处理方面，「不静默吞错」与「避免 panic」两条规则在 GPUI 侧基本达标。**
`unwrap()` 集中在测试，是正常的测试写法。

### 2.2 但「不静默吞错」在另一处有真实的债

`let _ = <expr>;` 共 **325 处**。其中右侧是 Result 返回型调用（`core_json` / `std::fs::` /
`File::` / `write` / `remove` / `create_dir` / `rename` / `copy`）的共 **78 处**。

抽样核对后，78 处的构成是：

- **约 60 处是测试清理**（`let _ = std::fs::remove_dir_all(&dir);` 集中在 `settings/src/persistence.rs`、
  `settings/src/project.rs`、`shared/src/workspace_config/*.rs` 的 `#[cfg(test)]` 段）——**合理**
- **少数是生产路径**，例如 `crates/settings/src/theme.rs:169` `let _ = std::fs::create_dir_all(store_dir);`
  ——创建主题目录失败被静默丢弃，用户会看到「主题保存了但没生效」而没有任何诊断

**这是一条真实但量级很小的债**（生产路径约 5～10 处），建议按「有诊断日志兜底」的标准逐处过一遍，
不适合单独立项，适合**随相关功能改动顺带清理**。

同时，全目录 `eprintln!` 有 **336 处**，`S1_*` 打点体系（`S1_MENU`、`S1_DEBUG_TRANSPORT`、
`S1_WORKSPACE_CONFIG`、`S1_JAVA_*`）覆盖了关键路径，**观测性设计本身是合格的**。

---

## 3. unsafe 与并发

### 3.1 unsafe：生产代码 **0 处**

`rust/lithe-gpui` 全目录 `unsafe` 共 12 处，**全部在 `crates/settings/src/paths.rs` 的
`#[cfg(test)] mod tests` 内**（`:156`、`:157`、`:183`、`:198`、`:206`、`:212`、`:223`、`:229`、
`:230`、`:241`、`:242`、`:248`）。

生产代码里那 2 处是 `EnvGuard::drop`（`:156-157`）——它本身也在 `#[cfg(test)]` 模块里：

```rust
// crates/settings/src/paths.rs:153-160
impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.1 {
            Some(value) => unsafe { std::env::set_var(self.0, value) },
            None => unsafe { std::env::remove_var(self.0) },
        }
    }
}
```

**安全不变量核对**：Rust 2024 起 `std::env::set_var` / `remove_var` 是 `unsafe fn`，
前提是「没有其他线程在并发读写环境」。该模块用 `static ENV_LOCK: Mutex<()>`（`:143`）
串行化所有用例，每个用例开头 `lock_env()`（`:162-177`）取锁并捕获三个环境变量的原值，
`Drop` 时恢复。**这个不变量成立，写法正确。**

> 一处**未修**的小瑕疵：`unsafe` 块内**没有 `// SAFETY:` 注释**说明「为何此时无并发访问」。
> `develop-lithe` 的 Rust Core 注释规范只强制要求 `rust/lithe-core/` 下的 unsafe 带
> `# Safety`，`lithe-gpui` 不在其管辖内，所以**不构成规则违反**，但作为可维护性项建议补。

**`rust/lithe-ui` 侧 unsafe 有 33 处**，其中生产代码在 `shell/native.rs`（10 处）、
`forms/color/dropper.rs`（5 处）、`buttons/share.rs`（2 处），全部是 macOS 的 Objective-C
桥接（`msg_send!` / `cocoa` / `objc`）。这些在 **`[target.'cfg(target_os = "macos")']` 条件下**
（`rust/lithe-ui/Cargo.toml:57`），**Windows 构建不编译**，因此不影响本次 Windows 侧性能与安全结论。
macOS 侧的 FFI 安全论证需要 macOS 实机复核，**本次未做**。

**结论：GPUI 统一前端在核心实现层**没有** unsafe 边界风险。**

### 3.2 并发原语清点

| 原语 | 命中数 | 备注 |
|---|---|---|
| `cx.spawn` | 60 | GPUI 任务模型，主流 |
| `Arc<` | 47 | |
| `Rc<` | 78 | GPUI 实体句柄，单线程 UI 模型，正常 |
| `RefCell` | 19 | 同上 |
| `Mutex` | 69 | |
| **`RwLock`** | **0** | 无读写锁 |
| `std::thread::spawn` | 9 | |
| `mpsc` | 16 | |
| `std::thread::sleep` | 25（`blocking_` 口径） | 见 §3.4 |
| `tokio::spawn` | **0** | 未引入 tokio 运行时 |
| `block_on` | **0** | **好：没有在 UI 线程上跑阻塞执行器** |
| `oneshot` | **0** | |

**并发模型是自洽的**：GPUI 单线程 UI + `cx.spawn`/`background_spawn` 做后台 + `std::sync::mpsc`
做跨线程回传，**没有混入 tokio 运行时**，也没有 `block_on` 把异步执行器堵在 UI 线程上。

### 3.3 `core_json`：一个同步的 C ABI 调用面，但调用点基本正确

`lithe_gpui_shared::core_json`（`crates/shared/src/core_client.rs:144`）是**同步**的：

```rust
// crates/shared/src/core_client.rs:114-115
pub fn execute(&self, request: &CoreRequest) -> Result<Option<Value>, CoreError> {
    let raw = lithe_core::execute_json(&request.to_request_json());
```

全目录 **40 个调用点**。逐一核对后，**绝大多数被正确地放在 `background_spawn` 里**。
典型正确写法在 `crates/explorer/src/explorer_view.rs:206-214`：

```rust
cx.spawn(async move |this, cx| {
    let snapshot_root = root.clone();
    let (snapshot, status) = cx
        .background_spawn(async move {
            (load_snapshot(&snapshot_root),   // ← 内部走 core_json，在后台线程
             load_git_status(&root))
        })
```

**没有发现「在 render 里直接 core_json」的阻塞 UI 线程写法。**

> **未实测**：我没有对每个调用点做「是否在 UI 线程」的动态验证，以上是静态读码结论。
> 若要升级为 L3，需要在实机用 `S1_*` 日志 + 帧耗时采样交叉验证。

### 3.4 唯一的结构性并发债：`debug_transport.rs` 的轮询线程

`crates/workbench/src/debug_transport.rs` 是全目录 `std::thread::spawn` 最集中的文件
（12 处，含测试）。生产侧有三处**轮询等待**：

```rust
// :280-291  recv_bounded
loop {
    if let Some(chunk) = self.try_recv() { return Some(chunk); }
    if Instant::now() >= deadline { return None; }
    std::thread::sleep(Duration::from_millis(5));   // ← 5ms 轮询
}
// :293-311  wait_exit
Ok(None) if Instant::now() < deadline => {
    std::thread::sleep(Duration::from_millis(10));  // ← 10ms 轮询
}
```

`:1561` 的测试辅助 `loopback_server` 里还有 `sleep(200ms)`。

**评估**：这些是**有界等待**，符合 `write-stable-tests` 的「有界等待」要求，不会挂死。
代价是**每次等待最坏引入 5～10 ms 延迟**，对调试会话的启动/步进响应是可感知的（人眼阈值约 100 ms，
单次不致命，但调试协议一次往返可能叠加多次）。**改进方向**：把 `mpsc::Receiver` 换成
`std::sync::mpsc::Receiver::recv_timeout` 或 `Condvar`，把「轮询」变成「阻塞唤醒」。

**未实测**：我没有测量调试会话的实际响应延迟。这是**静态推断的优化机会**，不是已证实的性能问题。

### 3.5 锁的使用

生产代码的 `Mutex` 共约 15 个字段级实例，集中在 `java/src/events.rs`（5 个）、
`java/src/service.rs`（2 个）、`run/src/session.rs`（2 个）、`run/src/pane.rs`、`settings/src/*`。

**中毒处理是一致的**，且方向正确——都选择「恢复而非 panic」：

```rust
// crates/java/src/service.rs:130-131
self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
// crates/run/src/session.rs:387
// crates/java/src/events.rs:427-429
```

唯一用 `.ok()?` 直接放弃的是 `service.rs:102` 的 `lsp_session_id()`——中毒时返回 `None`，
调用方会得到「Java 服务未启动」的降级路径。**可接受**。

**`RwLock` 为 0**：所有共享可变状态都是独占锁。对 `java/src/events.rs` 的
`diagnostics: Mutex<HashMap<String, Vec<JavaDiagnostic>>>`（`:65`）这类**读多写少**的映射，
换成 `RwLock` 或 `ArcSwap` 是有收益的——但**未实测**读竞争程度，属推断。

### 3.6 `.lock().expect("锁")` 的 11 处

全部在测试代码（`debug_transport.rs` 的 `#[cfg(test)]` 段、`workbench/tests/*.rs`）。
生产路径没有 `.lock().unwrap()`。**合规。**

---

## 4. 性能：一个已实测的算法热点

### 4.1 `search_model.rs:152 group()` —— O(n²)，实测最慢 57×

```rust
// crates/workbench/src/search_model.rs:152-171
pub(crate) fn group(matches: &[Match]) -> Vec<SearchRow> {
    let mut paths: Vec<String> = Vec::new();
    let mut hits: Vec<Vec<SearchRow>> = Vec::new();
    for item in matches {
        let index = match paths.iter().position(|path| path == &item.path) {  // ← 线性扫描
            Some(index) => index,
            None => { paths.push(item.path.clone()); hits.push(Vec::new()); paths.len() - 1 }
        };
        ...
```

`paths.iter().position(...)` 在每次迭代做一次线性查找 ⇒ 整体 **O(n²)**。
同样形状的问题在 `:205 quick_open_rows()` 的 `rows.iter().any(|row| row.path == item.path)`（`:211`）。

**这是被真实调用的**：
- `:547` `SearchState::ready` → `group(matches)`
- `:501` `QuickOpenState::ready` → `quick_open_rows(matches)`
- `search_panel.rs` / `quick_open.rs:188,321` / `search_everywhere.rs:189,550` 消费这些行

**可达规模**（`search_model.rs:11,13`）：

```rust
pub(crate) const MAX_RESULTS: usize = 200;          // 面板搜索
pub(crate) const EVERYWHERE_MAX_RESULTS: usize = 1000;  // 随处搜索
```

Core 侧上限更高（`rust/lithe-core/src/project/files.rs:204` `min(10_000)`）。

**实测**（复刻 `group()` 与 `quick_open_rows()` 的确切算法，`opt-level=3`，输入形状为
「每 10 条命中一条文件行」，与搜索结果的实际形状一致）：

| N（matches） | `group` 线性 ms | `group` HashMap ms | 加速 | `quick_open` 线性 ms | HashMap ms | 加速 |
|---|---|---|---|---|---|---|
| 1,000 | 0.49 | 0.07 | 7.2× | 0.00 | 0.01 | 0.4× |
| 5,000 | 6.96 | 0.23 | **29.8×** | 0.06 | 0.04 | 1.6× |
| 10,000 | 22.43 | 0.47 | **47.6×** | 0.21 | 0.05 | 3.9× |
| 20,000 | 98.87 | 1.73 | **57.1×** | 1.34 | 0.26 | 5.2× |

两个实现**结果一致**（基准里 `assert_eq!` 钉住了语义等价）。

**读法**：
- 在 `EVERYWHERE_MAX_RESULTS = 1000` 的**默认上限**下，`group` 约 **0.5 ms** ——**当前不构成用户可感问题**
- 只有在 Core 被要求返回**上万条**时（`min(10_000)` 允许，`MAX_RESULTS` 可被调用方覆盖）
  才会到 **22～99 ms**，那是**可感知的一帧卡顿**
- `quick_open_rows` 在默认规模下 0.21 ms，影响更小

**所以这是一条「随规模恶化」的债，不是当前热点。** 诚实地说：**它今天不拖后腿，
但它是唯一一条我实测到的、有明确量级结论的算法债**，改动成本极低（引入 `HashMap<&str, usize>`
索引），建议顺手做掉，并加一条 n=10_000 的基准测试防回归。

> **注意**：我没有把 `group()` 单独拿出来跑真实的 Core 返回数据，基准是**算法复刻**。
> 真实耗时还取决于 `Match` 的字符串分配（`:127` `path.to_string()`、`:129` `preview.to_string()`
> 每条命中 2 次堆分配）。**这部分未实测**。

### 4.2 `search_model.rs` 的其他可优化点（未实测，静态读码）

- `:152-182 group()` 里 `item.path.clone()` 对**每一条命中**都克隆路径字符串（`:159`、`:166`），
  同文件路径被重复分配 N 次。改为 `Arc<str>` 或索引到 `paths` 可消除。
- `:381-402 action_matches()` 对**每个候选**做 `format!("{title} {subtitle} {group}").to_lowercase()`
  （`:390`）——每次匹配分配一个 String。命令目录若上百条，每次输入一个字符就分配上百个 String。
  可改为在 `CommandAction` 上缓存小写形式。
- `:404-416 action_rows()` 接收 `catalog: Vec<CommandAction>` **按值**，调用方
  `command_palette.rs:174-180 visible_rows()` 每次 render 都 `shell.read(cx).action_catalog(cx)`
  重建整个目录，然后 `action_rows` 又把它整体消费掉。**每次按键/每帧重建命令目录**是明显的可优化点。

> `search_everywhere.rs:550` 的 `let rows: Vec<EverywhereItem> = self.items(cx);` 在 `render()` 里，
> `items()` → `rows()` → `everywhere_rows()` → `quick_open_rows()`。**这是每帧执行的**，
> 所以 §4.1 的复杂度确实落在渲染路径上。**未实测帧耗时**，但路径是确认的。

### 4.3 `debug_transport.rs`（2264 行）与 `search_index.rs`（455 行）

- `debug_transport.rs`：`format!` **33 处**、`.clone()` 11 处、`to_vec()` 7 处、
  `Vec::with_capacity` **0 处**。该文件承担 DAP 帧的编解码与线程搬运。
  **没有发现 O(n²) 形状**；主要开销是每帧的字符串格式化与拷贝。
  **未实测**：没有对它做基准或 flamegraph。**我不下「它慢」的结论。**
- `search_index.rs`：455 行，职责是「索引就绪状态 + 代数（generation）防陈旧」，
  结构清晰，**未发现热点**。它的 9 处 unwrap 全在测试段。

**诚实结论：我只证实了 `search_model.rs` 一处算法债。`debug_transport.rs` 与 `search_index.rs`
的「明显热点」在本轮检索中**没有找到**——不是「确认没有」，是「按静态读码没找到」。

---

## 5. 构建效率：可测量的优化方案

### 5.1 依赖分层与级联面（`cargo metadata` 实测）

第一方 crate 的反向依赖数：

| crate | 被谁依赖（反向依赖数） | 改它的实测代价 |
|---|---|---|
| **`lithe-gpui-shared`** | **11**（除自己外全部） | **281 s**（build）/ 221 s（check） |
| `lithe-gpui-settings` | 2（workbench, app） | **3.6 s**（check） |
| `lithe-gpui-java` | 2（workbench, editor） | 未单测 |
| `lithe-gpui-run` | 2（settings, workbench） | 未单测 |
| `lithe-gpui-notify` | 2（workbench, editor） | 未单测 |
| `lithe-gpui-languages` | 2（app, editor） | 未单测 |
| `lithe-gpui-git` / `explorer` / `terminal` / `editor` / `extensions` | 各 1（workbench） | **8.6 s**（terminal, build） |
| `lithe-gpui-workbench` | 1（app） | **8.7 s**（build） |
| `lithe-gpui-app` | 0（根） | — |
| **`lithe-ui`**（平级 workspace） | 由 9 个宿主 crate 引入 | **671 s**（build）/ 232 s（check） |

**「改 `workbench` 之外的 crate 减少级联」这条既有建议是错的**：
`workbench` 的反向依赖数只有 1，改它 **8.7 秒**。真正的放大器是
**`shared`（11 个反向依赖，281 s）和 `lithe-ui`（671 s）**。
排期必须按这个真实分层来分批，否则会把「改 workbench」当成高代价操作而错误地攒批。

### 5.2 `lithe-ui` 解耦：量化后的可行性

`lithe-ui` 是 45 个顶层模块、8.32 MB 源码、约 1090 个 `.rs` 的组件库，
`src/lib.rs:1-49` 把 **45 个模块全部 `pub mod` 无条件导出**，`Cargo.toml` 的 `[features]`
只有 `runtime_shaders` 与 `test-support`，**没有任何模块级 feature 门**。

**宿主实际只用到 17 个模块**：

```
theme(52) layout(30) forms(24) buttons(18) editor(17) shell(14) primitives(13)
overlays(9) menus(9) feedback(8) lists(7) navigation(5) language(5) init(4)
motion(4) devtools(1) data_display(1)
```

**未被 `rust/lithe-gpui` 引用的 29 个模块**（对 `lithe_ui::terminal` / `::git` / `::debug` /
`::tables` / `::charts` 逐个检索，命中均为 **0**）：

```
account agent calendar canvas charts chat collab dashboard debug documents
files finance generative git i18n interaction mail maps media messaging
misc onboarding project rendering settings tables terminal tooling typography
```

**规模对比**：

| | 模块数 | 源码 |
|---|---|---|
| 被宿主使用 | 17 | 2698 KB |
| **未被宿主使用** | **29** | **4189 KB** |

即 **约 61% 的 `lithe-ui` 源码（4.19 MB）每次都在为 GPUI 宿主编译**，
而宿主一行都不引用。

`lithe_ui` 的编译产物实测 **299.7 MB rlib + 25.9 MB rmeta**（`debug=0` 下）；
仓库既有 target（`debug=1`）里是 **1222.5 MB rlib**。它在重建时间线里占用
`07:16:26 → 07:18:55` 共 **149 秒**，是单 crate 最长的编译单元。

**可行的解耦手段（按成本递增）**：

1. **加模块级 feature（最小改动，收益最大）**
   在 `rust/lithe-ui/Cargo.toml` 增加 `default = ["runtime_shaders", "full"]` 与
   `full = [...]`，把 29 个未使用模块的 `pub mod` 用 `#[cfg(feature = "...")]` 门起来；
   `lithe-gpui` 侧只启用需要的 17 个。
   **风险**：`lithe-ui` 是「逐文件移植自 Ely GPUI Components」（`Cargo.toml:7-8`、`UPSTREAM.md`），
   加 feature 门会让后续 rebase 上游变难。**这是架构边界决策，必须由主理人转交架构师确认，
   我不擅自定。**

2. **把 `lithe-ui` 的 examples 排除出常规构建**
   `Cargo.toml:86-124` 声明了 **9 个 `[[example]]`**（gallery、docs_*）。
   `cargo build -p lithe-gpui-app` 不编 examples，所以**这条对本次实测的 671 s 无影响**，
   但 `cargo test --workspace` 会带上。属于次要项。

3. **共享 `target` 目录**
   当前 `lithe-ui` 与 `lithe-gpui` 是两个独立 workspace（`lithe-ui/Cargo.toml:128`
   有 `[workspace]`）。若把 `CARGO_TARGET_DIR` 统一，两者的依赖（`gpui-pre` 等）可共享编译产物。
   **未实测收益**，需要评估是否会引入 feature 解析冲突。

### 5.3 三条硬约束的复核结果

| 台账/计划的约束 | 本次实测 | 判定 |
|---|---|---|
| 构建一次约 15 分钟，每天 3～4 次 | 全量 9～11 分钟；**改宿主 crate 私有代码 3～9 秒** | **部分不成立**——约束只对 `shared`/`lithe-ui`/全量成立 |
| 构建必须绕开工作区（工作区 ACL 拒绝写入） | 工作区内 `cargo run` + 写文件 + 起子进程**全部成功** | **本次未能复现**，需主客确认 |
| `explorer.exe` 泄漏到 17 GB，rustc 崩 | 本机 `explorer` **未在运行**；`Available MBytes` 读数异常（0.00，perf counter 在此环境不可信）；Committed 21.57 GB / Limit 32.52 GB | **本次未复现**；32 GB 物理内存、commit limit 32.5 GB 与该说法一致 |
| `RUST_MIN_STACK=67108864` | 本次全程设置 | 沿用 |
| `CARGO_PROFILE_DEV_DEBUG=0` | 本次全程设置，全量 9m07s 完成无崩溃 | 有效，建议保留 |
| `-j 2` | 本次用 `-j 4`（8 逻辑核），无 OOM | `-j 2` 可能过于保守；建议 `-j 4` 起步 |

**对排期的直接含义**：
- 把「改 `shared`」「改 `lithe-ui`」的改动**单独成批**，每批只触发一次 4～11 分钟重建
- 改 `workbench` / `editor` / `git` / `explorer` / `terminal` / `settings` 的改动可以**高频迭代（秒级）**
- 因此「每天 3～4 次构建」不应作为排期系数；真正的系数是**「每批涉及 `shared`/`lithe-ui` 的重建次数」**

---

## 6. 优化项清单（本维度）

| # | 项 | 现状证据 | 目标 | 测量方法 | 人日 | 依赖 |
|---|---|---|---|---|---|---|
| **O1** | `group()` / `quick_open_rows()` 改哈希索引 | `search_model.rs:152-171`（`paths.iter().position`）、`:205-217`（`rows.iter().any`）；实测 20k 时 57.1× / 5.2× | n=10k 时 `group` 从 22.4 ms 降到 <1 ms | 加 `criterion` 基准：n ∈ {1k, 5k, 10k, 20k}，断言 HashMap 版 ≤ 线性版 1/10 | **0.5** | 无 |
| **O2** | 命令目录按帧重建 | `command_palette.rs:174-180` 每次 `visible_rows` 都 `action_catalog(cx)` 重建 Vec 并整体消费；`search_everywhere.rs:196-201` 同 | 目录缓存，仅在命令集变化时重建 | 加基准：`action_rows(catalog.clone(), q)` 的分配次数；用 `dhat` 或手数 `Vec` 长度 | **1** | 无 |
| **O3** | `action_matches` 每条候选一次 `format!+to_lowercase` | `search_model.rs:390` | 零分配匹配 | 基准 + 分配计数 | **0.5** | 无 |
| **O4** | `debug_transport` 轮询改阻塞唤醒 | `:280-291` 5 ms、`:293-311` 10 ms 轮询 | 去掉轮询延迟 | 测量 DAP 一次往返耗时（未实测）；改后对比 | **2** | 需要能跑调试会话（依赖 jdtls 资产） |
| **O5** | `java/events.rs` 读多写少锁换 `RwLock` | `:62-67` 5 个 `Mutex` 字段 | 降低诊断更新时的串行化 | `perf` / 帧耗时采样 | **1.5** | 需实机 Java 会话 |
| **O6** | `lithe-ui` 模块级 feature 门 | `lithe-ui/src/lib.rs:1-49` 45 个无条件 `pub mod`；宿主只用 17 个；未使用 29 个 / 4189 KB | 重建时间从 671 s 降到与使用面成正比 | 改后实测 `touch lithe-ui/src/lib.rs` 的 `cargo build` 耗时 | **3～5**（含回归） | **需架构师确认**（见 §5.2 风险） |
| **O7** | 统一 `CARGO_TARGET_DIR` 让两个 workspace 共享依赖产物 | `lithe-ui/Cargo.toml:128` 独立 workspace | 减少 `gpui-pre` 等公共依赖的重复编译 | 改后实测全量重建耗时 | **1** | 需评估 feature 解析冲突 |
| **O8** | 生产路径 `let _ =` 补诊断 | 78 处中约 5～10 处在生产路径，如 `settings/src/theme.rs:169` | 失败有 `S1_*` 打点 | 逐处读码 + 触发失败路径看日志 | **1** | 无 |
| **O9** | `paths.rs` 的 unsafe 补 `// SAFETY:` | `paths.rs:156-157` | 安全不变量成文 | 评审 | **0.2** | 无 |
| **O10** | 清理 3 个终端死常量 | `terminal/src/constants.rs:77/79/86`；`:82-85` 注释自述「当前没有读点」 | 删掉或补读点 | 评审 | **0.2** | 与终端面板功能改动合并 |

**合计约 11～14 人日**（不含 O6 的架构决策等待）。

**建议顺序**：`O1 → O3 → O2`（同文件、同一次重建，半天内完成，纯收益无风险）
→ `O8 → O9 → O10`（清理类，随相关功能顺带）
→ `O7`（独立可测）
→ **`O6` 待架构师确认后再排**
→ `O4 → O5`（依赖 jdtls 资产与实机会话，放在能跑调试的阶段）。

**依赖关系**：
- O4、O5 的**验收**依赖 `.artifacts/jdtls` + `.artifacts/jdk` 就位与实机调试会话，
  在环境前置没解决前**做不了验收**（可以写代码，不能验证）
- O6、O7 都动构建配置，**必须串行**，且各自需要一次全量重建做基线
- O1～O3 只动 `workbench` 私有代码 ⇒ **8.7 秒**即可验证，无阻塞

---

## 6.5 三条判断的裁决

立题时登记了三条候选判断。下面把每条对着它**事先写下的推翻条件**逐条对齐，
读数全部取自本报告前文与 `reports/gpui-core-impl-evidence/`。

### 判断 1 · 「unwrap 集中在测试代码」→ **支持**

> 主张：`keymap_io.rs` 等文件的高 unwrap 计数主要来自 `#[cfg(test)]` 模块，
> 生产路径的 unwrap 密度显著更低。
> **推翻条件**：统计出这些文件在 `cfg(test)` 分界线之前的 unwrap 数仍占多数。

| 对照项 | 读数 | 出处 |
|---|---|---|
| `keymap_io.rs` 生产 / 全部 | **0 / 12** | §2 表；分界线在 `keymap_io.rs:220` |
| `recent_projects.rs` 生产 / 全部 | **0 / 18** | §2 表 |
| `run/plan.rs` 生产 / 全部 | **0 / 4** | §2 表 |
| `git/changes.rs` 生产 / 全部 | **0 / 3** | §2 表 |
| 全目录生产 / 全部 | **40 / 541**（7.4%） | §2；`measurements.md` §4.1 |

**分界线之前的 unwrap 不占多数——恰恰是 0。推翻条件未触发。** 判断成立。

### 判断 2 · 「workbench 是级联重编放大器」→ **推翻**

> 主张：`workbench` 与 `settings` 位于依赖链中段，改动它们会级联重编 app 及其下游；
> 改动叶子 crate（`terminal`/`notify`/`extensions`）级联面更小。
> **推翻条件**：实测显示改叶子 crate 与改 `workbench` 的重编 crate 数相近。

| 对照项 | 读数 | 出处 |
|---|---|---|
| 改 `terminal`（叶子）重编 crate 数 | **3**（terminal → workbench → app） | §0 表 |
| 改 `workbench` 重编 crate 数 | **2**（workbench → app） | §0 表 |
| `terminal` 反向依赖数 | 1 | §5.1；`dep-layers.md` |
| `workbench` 反向依赖数 | **1** | §5.1；`dep-layers.md` |
| 真正的放大器 | `shared` 反向依赖 **11**（281 s）、`lithe-ui`（671 s） | §5.1 |

**叶子与 workbench 的重编 crate 数相近（3 vs 2），推翻条件被触发。** 判断被推翻。

需要如实说明推翻的方向：主张隐含「`workbench` 是放大器、叶子更轻」，实测**两者相当**，
且 `workbench` 的反向依赖数只有 1 —— 它依赖 10 个第一方 crate（扇入大），
但**只被 app 依赖**（扇出小）。**依赖多 ≠ 被依赖多**，这是原主张的推理错误。
真正的放大器是 `shared`（11 个反向依赖）与 `lithe-ui`。

这条推翻**改变了排期结论**：既有建议「优先改 `workbench` 之外的 crate 减少级联」方向是反的，
排期应改为「只有 `shared` 与 `lithe-ui` 的改动需要攒批」。

### 判断 3 · 「构建瓶颈是依赖链规模而非增量机制」→ **支持**

> 主张：15 分钟主要来自首次/全链编译与上游 gpui 依赖，日常改一个业务 crate 的
> 增量构建远低于 15 分钟。
> **推翻条件**：实测增量改一个 crate 仍需 10 分钟以上。

| 对照项 | 读数 | 出处 |
|---|---|---|
| 改 `workbench` | **8.7 s** | §0 表 |
| 改 `terminal` | **8.6 s** | §0 表 |
| 改 `settings` | **3.6 s** | §0 表 |
| 无改动 | **1.4 ～ 1.9 s** | §0 表 |
| 改 `shared` | 221.2 ～ 281.0 s | §0 表（分钟级，但 < 10 min） |
| 改 `lithe-ui` | 231.7 ～ 671.4 s（11m11s，接近但未达 10 分钟阈值的仅此一项） | §0 表 |
| 全量重建 | 547.1 ～ 548.8 s | §0 表 |

**改宿主 crate 的增量构建是 3.6～8.7 秒，远低于 10 分钟。推翻条件未触发。** 判断成立。

补一句边界：判断原文说「日常改**一个业务 crate**」，实测中改 `shared` 与 `lithe-ui`
确实进入分钟级——但这两个不是「业务 crate」，而是被 11 个 / 9 个 crate 依赖的公共层。
判断在其**自身声明的范围内**成立，且 §0 已把边界显式写成三档曲线。

### 裁决小结

| 判断 | 裁决 | 是否改变排期 |
|---|---|---|
| unwrap 集中在测试代码 | **支持** | 是——「错误处理是拖累项」的前提不成立 |
| workbench 是级联重编放大器 | **推翻** | 是——攒批对象从 workbench 改为 `shared`/`lithe-ui` |
| 构建瓶颈是依赖链规模而非增量机制 | **支持** | 是——「每天 3～4 次构建」不应作为排期系数 |

---

## 7. 风险与条件

| 风险 | 影响 | 触发条件 | 应对 |
|---|---|---|---|
| **「15 分钟」约束被误当系数** | 排期严重高估工作量，或错误地攒批导致反馈周期过长 | 沿用既有计划的口径 | 按 §0 的三档曲线重算：只有 `shared`/`lithe-ui`/全量才是分钟级 |
| **工作区 ACL 结论无法复现** | 若仓库配方确实需要 `CARGO_TARGET_DIR` 绕行，则 §0/§5 的部分实测环境与生产环境不同 | 主理人复核时发现 | 我的实测**全部在 `rust/lithe-gpui/target`** 完成；若必须绕行，结论的量级仍成立，但绝对秒数会变 |
| **我无法写 `lithe-gpui-target`** | 无法在仓库配方目录下复现基线 | 沙盒策略 | 已如实标注；建议主理人在能写的环境重跑 §0 表格里的 `shared` 与 `lithe-ui` 两行做交叉验证 |
| **O6 触碰移植边界** | 给 `lithe-ui` 加 feature 门会让上游 rebase 变难，属架构边界变更 | 实施 O6 时 | **必须经架构师确认**；我建议先做 O1～O3、O7 等不触碰边界的项 |
| **`group()` 热点在当前规模下不可感** | 若把它当成「用户体验问题」立项，是夸大 | MAX_RESULTS=200 时约 0.5 ms | 定位为「随规模恶化的算法债」，成本 0.5 人日顺手做掉，不单独立项 |
| **基准是算法复刻，非真实 Core 数据** | 绝对秒数可能与真实路径有差 | 真实 `Match` 含字符串分配 | 结论只用**相对加速比**（57×），不引用绝对 ms 作为用户指标 |
| **`Available MBytes` perf counter 读数异常** | 内存结论无法用该计数器支撑 | 本机 perf counter 返回 0.00 | 改用 `Committed Bytes` / `Commit Limit` 两个可用读数 |
| **macOS 侧 FFI 未审** | `lithe-ui` 33 处 unsafe 中的 macOS 部分未做安全论证 | 跨平台验收时 | 明确标注为**未覆盖**，需 macOS 实机复核 |

---

## 8. 建议与验收

### 立即（本批，1 天，只动 `workbench` 私有代码 ⇒ 每次验证 8.7 s）

- **O1 + O3 + O2** 一次改完，加 `criterion` 基准与一条 n=10_000 的回归断言
- **完成标准**：`cargo test -p lithe-gpui-workbench` 全绿；
  `cargo build -p lithe-gpui-app` 实测 ≤ 15 s；基准输出写入 `reports/`；
  `shared/platform-feature-matrix/features/*.json` **无需改动**（纯内部优化，无用户可观察变化）

### 次批（1～2 天）

- **O8 + O9 + O10**：清理类，随相关功能合并
- **完成标准**：`./scripts/verify-rust-core-comments.sh` 不涉及（改动不在 `lithe-core`）；
  `cargo clippy -p lithe-gpui-workbench -- -D warnings` 通过

### 架构确认后（3～5 天）

- **O6**：`lithe-ui` 模块级 feature 门
- **前置**：架构师确认「不破坏与 Ely 上游的 rebase 路径」
- **完成标准**：实测 `touch lithe-ui/src/lib.rs` 后的 `cargo build -p lithe-gpui-app` 耗时
  从 671 s 显著下降；**报告优化前后对比**；`lithe-ui` 的 9 个 example 仍可构建

### 环境前置就绪后

- **O4 + O5**：需要 jdtls 资产 + 实机调试会话
- **完成标准**：DAP 一次往返延迟有实测前后对比；Java 诊断更新期间无 UI 卡顿

### 全局验收

- 每批改动后：`cargo fmt` → `cargo clippy` → 受影响 crate 的 `cargo test`
- **不跑** `./scripts/verify-rust-core.sh`（本次不触碰 `rust/lithe-core`）
- 涉及 `shared/` 改动时：`./scripts/verify-shared-contracts.sh`
- 启动验证按台账 §0 的 `S1_*` 打点方式，验证后**关闭所有 Lithe 进程并确认无残留**

---

## 9. 交接给主理人

1. **需要更正简报**：任务简报列出的四处 unwrap 证据
   （`keymap_io.rs` 12 / `recent_projects.rs` 10 / `run/plan.rs` 4 / `git/changes.rs` 1）
   **实测全部位于 `#[cfg(test)]` 模块内**，生产路径 unwrap/expect 合计仅 4 处。
   「错误处理是拖后腿项」这条前提需要重新评估。
2. **需要主理人裁决的架构边界**：`lithe-ui` 加模块级 feature 门（O6）
   是否可接受「与 Ely 上游 rebase 成本上升」。**我不擅自决定。**
3. **需要与其他职责核对**：
   - 与**闻本立（架构师）**核对：`shared` crate 的 11 个反向依赖是否有拆分空间
     （`shared` 281 s 的级联是最大单点）；`lithe-ui` feature 门的架构可接受性
   - 与**常通关（构建发布）**核对：`CARGO_TARGET_DIR` 绕行是否仍是必需；
     CI 里的构建配方是否也需要按三档耗时重算
   - 与**陆井然/颜思齐**核对：`dead_code` 清单里 `explorer` 搜索框、`terminal` profile 三件套
     是否就是他们要补的界面入口（若是，属接线而非实现）
4. **未覆盖 / 未实测**：
   - `debug_transport.rs`（2264 行）与 `search_index.rs`（455 行）**未找到明显热点**，
     但也**未做 flamegraph / 基准**——不能读作「确认无热点」
   - `lithe-ui` 的 macOS FFI unsafe（33 处）**未做安全论证**，需 macOS 实机
   - 全部 40 个 `core_json` 调用点**未做运行时线程归属验证**（仅静态读码）
   - 调试会话延迟、Java 诊断更新延迟、UI 帧耗时**均未实测**
5. **无其他缺失资料**。
