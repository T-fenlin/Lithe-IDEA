# 热点排查（娄码通）

**口径声明**：本文件区分两类结论。
- **【已测量】** = 本次会话跑了基准或计时，给出数字
- **【静态读码】** = 只读代码推断，**未跑基准**，不作为性能结论使用

---

## 1. `search_model.rs:152 group()` — O(n²)【已测量】

### 证据

```rust
// crates/workbench/src/search_model.rs:152-171
pub(crate) fn group(matches: &[Match]) -> Vec<SearchRow> {
    let mut paths: Vec<String> = Vec::new();
    let mut hits: Vec<Vec<SearchRow>> = Vec::new();
    for item in matches {
        let index = match paths.iter().position(|path| path == &item.path) {  // ← 线性扫描
            Some(index) => index,
            None => {
                paths.push(item.path.clone());
                hits.push(Vec::new());
                paths.len() - 1
            }
        };
        if !item.kind.is_file() {
            hits[index].push(SearchRow::Hit {
                path: item.path.clone(),   // ← 每条命中克隆一次路径
                line: item.line,
                preview: item.preview.clone(),
            });
        }
    }
    ...
```

同形状问题在 `:205-217 quick_open_rows()`：

```rust
if rows.iter().any(|row| row.path == item.path) {   // :211 ← 线性去重
    continue;
}
```

### 可达性（确认，非推断）

| 调用链 | 位置 |
|---|---|
| `SearchState::ready` → `group(matches)` | `search_model.rs:547` |
| `QuickOpenState::ready` → `quick_open_rows(matches)` | `search_model.rs:501` |
| `EverywhereState::rows` → `everywhere_rows` → `quick_open_rows` | `search_model.rs:465` → `:323` |
| 渲染路径 | `search_everywhere.rs:550` `self.items(cx)` 在 `render()` 内 |
| | `quick_open.rs:188, 321` |
| | `search_panel.rs:507` render |

即 **`group()` 落在渲染路径上**。

### 规模上限

| 常量 | 值 | 位置 |
|---|---|---|
| `MAX_RESULTS` | 200 | `search_model.rs:11` |
| `EVERYWHERE_MAX_RESULTS` | 1000 | `search_model.rs:13` |
| Core 侧钳位 | `min(10_000)` | `lithe-core/src/project/files.rs:204` |

### 基准（复刻确切算法，`opt-level=3`，语义等价由 `assert_eq!` 钉住）

```
       N group_linear_ms  group_hash_ms    speedup   qopen_lin_ms  qopen_hash_ms    speedup
    1000           0.49           0.07       7.2x           0.00           0.01       0.4x
    5000           6.96           0.23      29.8x           0.06           0.04       1.6x
   10000          22.43           0.47      47.6x           0.21           0.05       3.9x
   20000          98.87           1.73      57.1x           1.34           0.26       5.2x
```

### 结论

**在当前默认上限（1000）下 `group` 约 0.5 ms，不构成用户可感问题。**
只有在 Core 被要求返回上万条时才到 22～99 ms（可感知的一帧卡顿）。

⇒ 定位为**「随规模恶化的算法债」**，不是当前热点。改动成本极低，建议顺手做掉并加
n=10_000 的回归基准。**不应作为「用户体验问题」单独立项。**

**限制**：基准是**算法复刻**，不是真实 Core 返回数据。真实路径还含每条命中 2 次字符串
分配（`path.to_string()` `:127`、`preview.to_string()` `:129`）。**该部分未测量**，
因此只引用相对加速比，不引用绝对 ms 作为用户指标。

---

## 2. `search_model.rs` 其他可优化点【静态读码，未跑基准】

| 位置 | 问题 | 推断代价 |
|---|---|---|
| `:152-182` | `item.path.clone()` 对**每条命中**克隆路径（`:159`、`:166`），同文件路径重复分配 N 次 | 与 N 成正比 |
| `:390` | `format!("{title} {subtitle} {group}").to_lowercase()` 对**每个候选**分配一个 String | 命令目录若上百条，每输入一个字符分配上百个 String |
| `:404-416` | `action_rows(catalog: Vec<CommandAction>)` **按值**接收目录 | 见下 |

### 命令目录按帧重建（推断的更大问题）

```rust
// crates/workbench/src/command_palette.rs:174-180
fn visible_rows(query: &str, cx: &App) -> Vec<CommandAction> {
    let catalog = shell()
        .and_then(|shell| shell.upgrade())
        .map(|shell| shell.read(cx).action_catalog(cx))   // ← 每次调用重建整个目录
        .unwrap_or_default();
    crate::search_model::action_rows(catalog, query)      // ← 按值消费
}
```

同形状在 `search_everywhere.rs:196-201` 的 `action_catalog(cx)`。

**推断**：每次按键/每帧重建整个命令目录 Vec，然后整体消费。**未测量**目录大小与重建耗时。

---

## 3. `debug_transport.rs`（2264 行）【未找到热点】

分配模式统计：

| 模式 | 计数 |
|---|---|
| `format!` | 33 |
| `.clone()` | 11 |
| `to_vec()` | 7 |
| `Vec::with_capacity` | **0** |

**未发现 O(n²) 形状**。主要开销是 DAP 帧编解码的字符串格式化与拷贝。

### 但发现一处结构性的等待延迟【静态读码】

```rust
// :280-291  recv_bounded
loop {
    if let Some(chunk) = self.try_recv() { return Some(chunk); }
    if Instant::now() >= deadline { return None; }
    std::thread::sleep(Duration::from_millis(5));    // ← 5 ms 轮询
}

// :293-311  wait_exit
Ok(None) if Instant::now() < deadline => {
    std::thread::sleep(Duration::from_millis(10));   // ← 10 ms 轮询
}
```

这是**有界等待**（符合 `write-stable-tests` 要求，不会挂死），但每次等待最坏引入
5～10 ms 延迟。调试协议一次往返可能叠加多次。

**改进方向**：`mpsc::Receiver::recv_timeout` 或 `Condvar`，把轮询变成阻塞唤醒。

**未测量**：没有测调试会话的实际响应延迟。这是**推断的优化机会，不是已证实的性能问题**。

---

## 4. `search_index.rs`（455 行）【未找到热点】

职责是「索引就绪状态 + 代数（generation）防陈旧」：

- `:100 next_generation` / `:105 accepts` / `:171 adopt` —— 代数校验，O(1)
- `:109 schedule_warm` / `:125 schedule_update` / `:149 schedule_invalidate` —— 调度，非热路径
- 6 处 `core_json` 调用，均在 `cx.spawn` 内

结构清晰，**未发现热点**。9 处 unwrap 全在 `#[cfg(test)]` 段（`:226` 起）。

---

## 5. 诚实边界

- `debug_transport.rs` 与 `search_index.rs` 的「明显热点」**在本轮检索中没有找到**。
  这是「按静态读码没找到」，**不是「确认没有」**——两者都**未做 flamegraph / criterion 基准**。
- 未测量项清单：UI 帧耗时、调试会话往返延迟、Java 诊断更新延迟、
  真实 Core 搜索返回数据的处理耗时、命令目录大小与重建耗时。
- 若要升级为可对外引用的性能结论，需要：`cargo flamegraph -p lithe-gpui-app` 实机采样，
  或对上述函数加 `criterion` 基准。
