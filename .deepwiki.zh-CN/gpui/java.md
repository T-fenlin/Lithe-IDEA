# Language Service (`lithe-gpui-java`)

`rust/gpui/crates/java/` 拥有**所有与平台相关的 Eclipse JDT LS 细节**。它故意**没有** `gpui-kit` 依赖（`Cargo.toml:7-15`）——它是纯数据 / 请求构造层，这也是它能持有阻塞式 `std::thread` 事件泵的关键。

分层设计是整个点：`lib.rs:8-26` 中写得很清楚：Core 负责子进程、stdin/stdout、帧协议、JSON-RPC id、文档版本、等待超时、能力声明、诊断以及优雅/强制终止；这个 crate 则**不包含任何 LSP 协议代码，也不含 Java 语法实现**。它承担四类职责：

| 模块 | 职责 |
| --- | --- |
| `jdtls.rs` | JDT LS 载荷和运行 JDK 的平台发现 |
| `workspace.rs` | 工作区索引缓存 key、缓存目录、过期回收、Maven 上下文 |
| `session.rs` | Core LSP 命令的同步信封 |
| `service.rs` | `JavaLanguageService` 门面 |
| `events.rs` | 事件泵——**唯一消费 `lsp.waitEvents` 的地方**——以及分发表和诊断存储 |
| `toolchain.rs` | `javaHomePath` 和 `mavenSettingsPath` 的宿主注入点 |

## State

```
JavaLanguageService                     service.rs:187-194
  workspace_root: PathBuf
  state: Mutex<State>
      Idle              从未尝试，或策略判断“不是 Java 工作区”
      Ready(Box<Session>)
      Failed(String)    失败后不会重试
      Closed            app 正在退出
  installation: Mutex<Option<(JdtlsInstallation, JavaRuntime)>>

Session                                   session.rs:101-107
  id, events: Arc<SessionEvents>, pump: EventPump

SessionEvents                             events.rs:139-161
  pending: Mutex<HashMap<operationId, Sender<Result<Value,String>>>>   # in-flight waiters
  lifecycle: Mutex<Lifecycle> + Condvar                               # queued Vec<Value> + terminal
  diagnostics: Mutex<HashMap<uri, Vec<JavaDiagnostic>>>               # 最新快照
```

它在 `EditorPane` 内被持有为 `Option<Arc<JavaLanguageService>>`（`editor/src/editor_view.rs:374`），因此所有 gpui 交互都通过 `prepare_java` / `shutdown` / `Drop` 进行。

## JDT LS 载荷发现

`jdtls::resolve(workspace_root)`（`jdtls.rs:104-153`）：

1. `find_executable`（`:336-350`）——优先 `is_file()`，顺序为：
   - **内置根**（`:356-374`）：`current_exe().parent()/LanguageServers/jdtls`，再从 exe 目录向上探 12 层，寻找 `.artifacts/jdtls`。使用 `current_exe` 而非资源目录，是让 `cargo run` 能工作。
   - `PATH` 中每个目录 × `['jdtls.bat', 'jdtls.cmd', 'jdtls.exe', 'jdtls']`
   - **外部安装根**（`:377-409`）：`JDTLS_HOME`，`LITHE_JDTLS_ROOT`，`<LOCALAPPDATA|ProgramFiles|ProgramFiles(x86)|XDG_DATA_HOME>` 变体，`<USERPROFILE|HOME>/{.jdtls, scoop/...}`，以及 **`<workspace_root>/.lithe/toolchains/jdtls`**
2. `installation_roots(executable)`（`:256-278`）——如果父目录名为 `bin`，那么 root 是其父目录；另外也会把父目录加入队列；`canonicalize` 后的变体追加到**最后**。

   > 不要排序这个列表。Windows 上 `canonicalize` 返回原始 `\\?\D:\...`；`
`（0x5C）排序在 `D`（0x44）之前，所以排序后会优先选 verbatim 变体，所有资源路径都带 `\\?\`——JVM 随后无法找到 launcher jar（观察到 `initialize` 会立刻失败，错误为 `Unable to add \\?\...lombok.jar`）。文档在 `:250-255`。
3. 对每个 root，`resources_for_root`（`:166-188`）执行 all-or-nothing 校验：
   `plugins/org.eclipse.equinox.launcher_*.jar`（必需，按排序文件名取第一个）、`java-debug/...debug.plugin-*.jar`（可选）、`java-test/extensions.txt` **以及其列出的每个文件**（必需——声明顺序正是 OSGi 解析顺序，且缺失一个 listed bundle 就算错误，因为部分加载会破坏整个扩展）、平台专属 `config_*` 目录、`lombok/lombok.jar`。任何含 `/`、`\` 或 `..` 的路径都会在这里被拒绝为 path traversal（`:230-235`）。
4. 版本（`:282-333`）：`manifest.json` -> 否则 core plugin 文件名 `org.eclipse.jdt.ls.core_1.61.0.<ts>.jar` 取**前三段点分割** -> 否则内嵌 `BUNDLED_MANIFEST`。它只影响缓存 key，不影响可用性。

## JDK 解析

`resolve_runtime` 的判定顺序就是枚举顺序（`:512-548`）：

1. `Override` —— 设置页 `javaHomePath`（用户显式选择）
2. `ExplicitEnv` —— `LITHE_JDTLS_JAVA`
3. `Bundled` —— `current_exe()/LanguageServers/jdk`，向上寻找 `.artifacts/jdk`
4. `JdtlsBundled` —— `<jdtls_root>/jre`
5. `JavaHome` —— `JAVA_HOME`
6. `Path` —— 每个 `PATH` 目录
7. `InstalledRoot` —— 常见安装根目录

> override 被刻意排在 `LITHE_JDTLS_JAVA` 之前，并由测试固定（`:1013-1042`）。环境变量被说明为“只覆盖推导过程，不覆盖常规入口”，而设置值则是产品层的用户选择，settings 页面标记为“已选择”。允许校验开关覆盖用户选择会使 settings 页面“已存但未生效”再次出现（`:498-508`）。

`MINIMUM_JAVA_MAJOR = 21`（`:80`）。`probe_java` 会执行 `java -version`，合并 stdout + stderr（现代 JDK 在 stderr 上输出版本）。`major_version` 处理 `1.8.0_402 -> 8`、`21.0.2 -> 21`、`25 -> 25`。被拒绝的候选会被记录，并继续试下一个——**一个不可用的 override 不会导致启动失败**（`:630-631`）。

只有 `Override` 分支接受无扩展名的 `java`，因为这是 settings 页面接受的形式（`java/src/service.rs:720-722`）。

`S1_JAVA_TOOLCHAIN` 有四种输出形态，`reason=` 始终放在最后，因为它可能含空格（`:577-623`）：

```
S1_JAVA_TOOLCHAIN override=-        state=absent  result=automatic java=… javaVersion=21.0.8
S1_JAVA_TOOLCHAIN override=<path>  state=used    result=used      java=… javaVersion=21.0.8
S1_JAVA_TOOLCHAIN override=<path>  state=invalid fallback=automatic result=automatic … reason=版本 1.8.0_221 低于 21
S1_JAVA_TOOLCHAIN override=<path>  state=invalid fallback=automatic result=failed reason=找不到 java.exe
```

### 注入链

```
settings file javaHomePath  ─┐
                             ├─▶ project::resolve_overrides ──▶ set_java_toolchain_override
.lithe/run.local.json      ─┘   (shared::workspace_config::ToolchainPaths::resolve)
                                        │
                                        ▼
                          static OVERRIDE: Mutex<Option<…>>     toolchain.rs:79
                                        ▼
                          jdtls::resolve_runtime re-reads it      jdtls.rs:635
```

这里用的是 `Mutex`，不是 `thread_local!`：读取发生在 `start_locked` 上，而它运行在 gpui 后台执行线程上；`thread_local!` 会永远读到 `None`，重现“一旦保存就无效”的 bug（`toolchain.rs:20-28`）。

**生效时机：重启。** 因为临时槽会在每次 `resolve_runtime` 时重读，但 session 是按工作区持有的，而且 `EditorPane::prepare_java` 是幂等的，因此 JDK 改变在**下一次语言服务启动时**生效。中途切换 JDK 会被拒绝，因为索引和 JVM 版本会失配（`toolchain.rs:53-59`）。

五个 toolchain key 中，只注入了两项：`javaHomePath` 和 `mavenSettingsPath`。`mavenExecutablePath` / `mavenJavaHomePath` / `mavenLocalRepositoryPath` 被刻意不注入，因为 gpui 侧并不执行 Maven。

## 工作区索引缓存

`workspace::plan(root, jdtls_version)`（`workspace.rs:62-79`）：

1. `collect_fingerprint_inputs`（`:136-198`）——只收集**平台观察到**的输入：对根目录中 `['pom.xml', 'build.gradle', 'build.gradle.kts']` 中每项记录 `{path, modifiedUnixMilliseconds, sizeBytes}`；另外还记录**直接子目录中包含 `pom.xml` 的目录名**（非递归——递归交给 JDT）。
2. `resolve_fingerprint` → **`java.jdtWorkspaceFingerprint`**。规范化、排序、去重和哈希都由 Core 负责。
3. `resolve_workspace_key` → **`lsp.jdtWorkspaceKey`**。会校验为 64 位小写 hex，因为它会作为目录名（`:388-393`）。
4. `state_directory() = cacheDirectory/jdtls/<workspaceKey>`（`:54-58`）。`state_directory_existed` 是“重新打开复用索引”的直接证据。
5. `cleanup_expired(cache_directory, active_workspace_key)`（`:260-385`）回收过期项；失败仅记录 `S1_JAVA_CACHE_RECLAIM_FAILED`，**不阻断启动**——回收只是为了节省磁盘。

平台缓存根：`%LOCALAPPDATA%\Lithe\cache` / `~/Library/Caches/Lithe` / `$XDG_CACHE_HOME/lithe` -> `~/.cache/lithe`，并可由 `LITHE_GPUI_CACHE_DIR` 覆盖（`:90-123`）。

遗留的“仅基于路径的 key”路径**不用**：必须总是带 fingerprint，否则工作区结构变更后会静默复用陈旧 project model（`:12-13`）。

## 会话生命周期

### `lsp.startServer` payload

`Session::start`（`session.rs:127-214`）构造的是完全结构化的**直接启动**——没有 wrapper 参数。信封超时为 **30 s**（`:181-185`），只覆盖“启动进程并写入一帧”的时间，因此 UI 背景任务不会因为 `initialize` 超时而卡住。所有路径都经过 `jdtls::normalize_path`（`:882-890`），它会把 `\\?\UNC\server\share` -> `//server/share`，去掉 verbatim `\\?\`，并将 `\` 转 `/` —— 这是必需的，因为 JVM 会拒绝 verbatim 路径。

pump 会在 `startServer` 成功后**立即启动**（`:210-211`）：`stateChanged` / `log` / `serverInfoChanged` 等握手事件已经排队，如若延后启动，可能错过 ready signal。pump 启动失败会导致整个 start 失败。

### 事件泵

```
loop {
  if stopping -> break
  match wait_events(session_id, PUMP_SLICE = 1s) {
    Ok(batch)  -> dispatch each; if terminal_reason().is_some() -> break
    Err(error) -> break   // 任何错误都表示 session 不可用
  }
}
events.mark_terminal("事件泵已退出（{reason}）")
```

`PUMP_SLICE` **不是轮询周期**：pump 会阻塞在 Core 的 condvar 上，发生活动时即返回。它的意义只是限定空闲 session 询问 Core 的频率，以及 `stop()` 的 join 时间（`events.rs:51-56`）。

| 事件 | 处理 |
| --- | --- |
| `requestCompleted` | 以 `operationId` 删除 waiter 并 `send` 结果；未知 id 会累积 `unclaimed` 并打日志 `S1_JAVA_PUMP unclaimed` |
| `diagnostics` | 每个 `uri` 只保留最新快照，并打印 `S1_JAVA_JAVA_DIAGNOSTICS uri=… version=… count=… dropped=…` |
| `log` | `S1_JAVA_LOG level=… message=…`，连续重复会被抑制 |
| `stateChanged` / `serverInfoChanged` | 推入**队列** + `Condvar::notify_all`；terminal state 也会 `mark_terminal` |
| 其他 | 计数，并在第一次看到该类型时打印 `S1_JAVA_PUMP ignored type=…` |

**单消费者原则。** `lsp.waitEvents` 具有 drain 语义，因此第二个消费者会偷走另一个消费者的事件。`wait_events` 因此被设计成一个**自由函数**，接收 `session_id` 而不是 `&Session`，这样没人能不费力地拿到第二个读 handle（`session.rs:442-446`）。

`Lifecycle::queued` 是一个 **queue，不是 cell**（`:168-169`）：如果它是“最新状态”的 cell，那么一个 `stateChanged: ready` 在调用方开始等待前到达就可能被覆盖，从而让 `wait_ready` 永远卡住，直到绝对超时。

锁顺序是 `pending → lifecycle`，并且 `mark_terminal` 会故意 **先释放 `lifecycle` 再拿 `pending`**，这样二者永远不会嵌套（`:383-394`）。

## 参考资料

- `rust/gpui/crates/java/` 下全部文件
- `rust/gpui/crates/editor/src/editor_view.rs`
- `rust/lithe-core/src/lsp/` 全部内容
- `shared/contracts/application-boundary.md`
- `third_party/jdtls/manifest.json`
