# GPUI 前端功能平替冲刺计划（2026-10-12 → 2026-10-31）

## 先说结论

统一 GPUI 前端（`rust/lithe-gpui`）相对旧 Windows 编辑器（`windows/tauri`）的平替基准是
**92 项**用户能力，不是功能矩阵登记的 112 项 —— 旧端
`windows/tauri/src/config/backend-capabilities.ts` 把 `agent/database/docker/github/remote/wsl/runActions`
硬编码为 `false`，那 20 项 UI 有入口、点下去显示「待开发」，矩阵却记成 `implemented`。

当前水位：已具备 23 项 / 降级或未接线 23 项 / 完全缺失 46 项。

本月目标：**已具备 23 → 68，完全缺失 46 → 8，另有 19 项显式排除**。

---

## 0. 决定排期的五条硬约束

这几条不是注意事项，是会直接改变工作量的事实，排期必须按它们算。

1. **构建一次约 15 分钟。** `cargo build -p lithe-gpui-app` 会级联重编整条依赖链
   （2026-10-11 实测 14m48s，`lithe-ui` 一改就全链路重编）。因此**每天有效构建次数只有 3～4 次**，
   功能必须成批写完再一起验证，不能一项一构建。
2. **构建必须绕开工作区。** 工作区被沙盒施加 ACL，位于其中的可执行文件运行时被拒绝一切写入：
   ```
   cd rust/lithe-gpui
   CARGO_TARGET_DIR=D:/developmentProjects/rust/lithe-gpui-target cargo build -p lithe-gpui-app
   ```
3. **分支与流程。** 每个阶段单独从最新 `preview` 建分支（见根 `AGENTS.md`「开发与协作」）；
   GitHub 相关操作一律用 `gh` CLI。
4. **验收方式是启动日志的 `S1_*` 打点**，不是截图。按根 `AGENTS.md`，验证结束后必须关闭
   Lithe 进程并确认无残留。
5. **每批功能必须同步功能矩阵。** `shared/platform-feature-matrix/features/<id>.json` 是唯一数据源，
   CI 门禁会比较能力 JSON 的实际内容；只改代码不改矩阵，PR 会挂。改完跑
   `node scripts/generate-platform-feature-matrix.mjs` 与 `./scripts/verify-platform-feature-matrix.sh`。

**环境前置（不解决则阶段 3、5 无法验收）：只需跑两条仓库自带脚本，不改系统环境。**

2026-10-12 更正：早前判断「本机 JDK 1.8 ⇒ 得先升 JDK」是**错的**，它把两套 JDK 混为一谈了。
实际分层如下，互不影响：

| 用途 | JDK | 来源 | 可否覆盖 |
|---|---|---|---|
| **跑 jdtls（语言服务运行时）** | Temurin **21.0.12.1+1** | 仓库自带，随安装包打包 | `LITHE_JDTLS_JAVA` / `JAVA_HOME` / 设置页覆盖（Override 优先级最高） |
| **用户项目编译与运行（Project SDK）** | 默认 **JDK 8** | 用户自持，不随应用打包 | 项目页 JDK 覆盖 |

`scripts/prepare-jdk.ps1` 头部注释即为官方口径：*「This JDK only runs the Java language
server. Project SDKs stay user-owned.」* 所以本机 `java -version` 是 1.8.0_221 **不影响
jdtls** —— jdtls 用的是 `third_party/jdk/manifest.json` 钉的 21，`MINIMUM_JAVA_MAJOR=21`
那条门槛只对「jdtls 的运行时」生效。

仓库自带的打包链路（全部已核实存在）：

```
third_party/jdtls/manifest.json    jdtls 1.61.0 + lombok 1.18.46
                                   + java-debug 0.59.0 + java-test 0.46.0
third_party/jdk/manifest.json      Temurin 21.0.12.1+1（windows-x86_64 URL + sha256 齐备）
scripts/prepare-jdtls.ps1          下载并校验到 .artifacts/jdtls
scripts/prepare-jdk.ps1            下载并校验到 .artifacts/jdk
scripts/build-windows.ps1:118/132  复制为 <安装根>/LanguageServers/jdtls 与 LanguageServers/jdk
```

即 `third_party/` 只存 manifest 与空目录骨架（`third_party/README.md` 明令禁止把上游
发布包进仓），产物在构建时下载。**当前 `.artifacts/jdtls` 与 `.artifacts/jdk` 都不存在**
—— 这才是启动时 `S1_JAVA_FAILED: 未找到 jdtls` 的真正根因，不是没装 jdtls。

因此阶段 2 的动作是**跑两条脚本 + 一个环境变量**，不需要动系统 JDK：

```powershell
./scripts/prepare-jdtls.ps1 -OutputDirectory .artifacts/jdtls   # 下载 jdtls 全家桶
./scripts/prepare-jdk.ps1                                       # 下载 JDK 21 到 .artifacts/jdk
```

开发期额外一步：调试产物 `lithe-gpui-target/debug/Lithe.exe` 不在仓库内，
`jdtls.rs:275 bundled_roots()` 向上 12 层找 `.artifacts/jdtls` 够不到仓库根，所以启动时带：

```bash
LITHE_JDTLS_ROOT=D:/developmentProjects/rust/Lithe-IDEA/.artifacts/jdtls ./Lithe.exe
```

（`jdtls.rs:302` 的 `search_roots()` 会读这个变量。正式打包后 `LanguageServers/jdtls`
就在 exe 旁边，无需此变量。）

唯一仍需本机自备的是 **Maven**：仓库不打包 Maven，靠 `MAVEN_HOME` 探测
（`settings/src/project.rs:11`），当前 `mvn` 报 `classworlds` 错误需自行修复或指向可用的
Maven 安装。它只阻塞阶段 4 的 Maven 目标执行，不阻塞阶段 2 的语义与导航。

**每天的构建还受本机内存制约（2026-10-12 实测）**：`explorer.exe` 会泄漏到 17 GB 以上，
32 GB 内存被占掉 28 GB 后，rustc 会崩成 `STATUS_STACK_BUFFER_OVERRUN` 或 ICE，
linker 报 `LNK1102 内存不足`；而崩溃会污染 target 缓存，后续每次重试都变成
`E0786 invalid metadata files` / `only metadata stub found`，必须 `cargo clean` 才能摆脱。
所以每天开工先跑一次内存检查，紧张时的构建配方是：

```bash
tasklist | sed 's/,//g' | awk 'NR>3 && $(NF-1) ~ /^[0-9]+$/ {printf "%.0f MB  %s\n", $(NF-1)/1024, $1}' | sort -rn | head
export CARGO_TARGET_DIR=D:/developmentProjects/rust/lithe-gpui-target
export RUST_MIN_STACK=67108864     # 缓解 rustc 宏展开栈溢出
export CARGO_PROFILE_DEV_DEBUG=0   # 降低内存与耗时，代价是断点调试无符号
cargo build -p lithe-gpui-app -j 2 # 并行度不高于 4，否则 linker OOM
```

根本解法是释放 explorer 的内存（重启 `explorer.exe`），属于桌面进程，需要用户确认后再动。

---

## 1. 阶段 0：止血（10/12，1 天）

四件「改动小、感知极强」的事，先做掉。0.4 不写代码，但它是阶段 2、4 的验收开关。

### 0.1 会话保存被 exclude 失败一票否决 —— 体验事故，最高优先级

现象：每次开项目都记不住上次打开的文件，且每次弹一条 Error 通知。

根因链条（已定位到行，这两个现象是同一个 bug，不是一个功能问题加一个报错）：

```
shared/src/workspace_config/session.rs:139
  let exclusion = sharing::ensure_project_dir_excluded(root).map_err(SessionError::Exclude)?;
                                                                                          ^ 这里
sharing.rs:113  write_ignore_patterns(...)  被 Core 的 git 只读接口拒绝
sharing.rs:126  S1_WORKSPACE_CONFIG exclude_failed ... error=Unsupported Git write operation
workbench/src/workspace.rs:1364  S1_WORKSPACE_CONFIG session_flush ... wrote=false
```

`.lithe/` 没能写进 `.git/info/exclude`，`?` 把这个失败升级成整个会话保存失败。

修复分两步：
- **止血（本次做）**：把 `session.rs:139` 的失败从「中断」降级为「记录并继续」。
  `.lithe/` 没被 gitignore 只是会让工作区变脏，不该让会话保存整体失败。
- **正修（登记为独立条目）**：补 Core 侧 git 写操作能力，让 `write_ignore_patterns` 真正可用。

验收：`S1_WORKSPACE_CONFIG session_flush ... wrote=true`；重启后 `session_restored opened>0`；
不再出现 Error 通知。

### 0.2 Spring 面板：代码写完了，只差一个入口

`workbench/src/right_tool_window.rs` 里 `RightToolWindowView::Spring` 的 `id()`、`from_id()`、
`title()`、`icon()` **四个分支都已实现**，但：

```rust
// right_tool_window.rs:47
pub const fn from_rail_index(index: usize) -> Option<Self> {
    match index {
        1 => Some(Self::Notifications),
        2 => Some(Self::Maven),
        _ => None,          // Spring 没有映射，右栏图标点不出来
    }
}
```

工作：`from_rail_index` 补 `3 => Some(Self::Spring)`；确认右侧 activity rail
（`workspace.rs:3053` 的 `right_activity_items`）渲染到第 3 位；同步改
`right_tool_window.rs:502-511` 的单元测试（现有用例断言 `from_rail_index(3) == None`）。

验收：右栏出现 Spring 图标；点击后渲染 `spring_panel.rs` 的端点列表。

### 0.3 在文件中替换：缺的不是按钮，是 Core 命令

`replace_in_files.rs` 只有 `REPLACE_PREVIEW_COMMAND`（`workspace.replacePreview`），
**Core 侧没有对应的 apply 命令** —— `protocol/command.rs` 里 workspace 域只有
`snapshot / repositories / searchIndex.* / search / searchEverywhere / replacePreview`，
加上 `file.read`、`file.write`。

所以这项不能只加按钮：需要**先在 Core 增 `workspace.replaceApply`**（或复用 `file.write`
在 gpui 侧批量落盘），再补 UI 的应用入口。工作量按 1 天算，放在阶段 0 末尾；
若 Core 侧阻力大则顺延到阶段 1，不阻塞后续。

### 0.4 点亮 Java 语义资产（10 分钟，解锁阶段 2 与阶段 4 的验收）

不属于代码改动，但**不做则阶段 2、4 写完无法验证**，所以排在阶段 0。

当前 `.artifacts/jdtls` 与 `.artifacts/jdk` 均不存在，于是 `resolve()` 找不到可执行文件、
启动打 `S1_JAVA_FAILED: 未找到 jdtls`。这不是「机器没装 jdtls」，而是**仓库自带的资产
还没下载**。两条脚本带 sha256 校验，跑一次即可：

```powershell
./scripts/prepare-jdtls.ps1 -OutputDirectory .artifacts/jdtls
./scripts/prepare-jdk.ps1
```

下载物落在 `.artifacts/`（已被忽略，不进仓），CI 在 `.github/workflows/ci-windows.yml:595`
走的正是同一条命令，有 `.artifacts/jdtls-downloads` 缓存。

**2026-10-12 实测的两个坑**：

1. `prepare-jdtls.ps1` 会串行下载 7 个文件，其中 `java-test` 的 vsix 走
   `open-vsx.org` → 302 到 `openvsx.eclipsecontent.org`，**本机实测只有 2.9 KB/s**，
   4.99 MB 要下 20~30 分钟。PowerShell 工具的默认超时是 120 秒、上限 600 秒，**会中途被杀**。
   办法：用 Git Bash 的 `curl` 后台下载到 `.artifacts/jdtls-downloads/`，校验 sha256 后改名为
   脚本期望的缓存名，再重跑脚本 —— `Get-VerifiedDownload`（`prepare-jdtls.ps1:96`）命中缓存会直接跳过。
2. GitHub raw 会偶发 `502 Bad Gateway`（首次跑就挂在 java-debug 的 LICENSE.txt）。
   重试即可，已下好的文件靠缓存复用，不会重来。

缓存名就是 `manifest.json` 里的 sha256，例如 java-test vsix 为
`vscode-java-test-0.46.0-56c1e14d...255.zip`（完整值见 manifest）。

开发期启动调试产物时补一个环境变量，否则 exe 在 `lithe-gpui-target` 下够不到仓库根的
`.artifacts`（`jdtls.rs:275` 只向上找 12 层且只认祖先目录）：

```bash
LITHE_JDTLS_ROOT=D:/developmentProjects/rust/Lithe-IDEA/.artifacts/jdtls ./Lithe.exe
```

验收：启动日志 `S1_JAVA_*` 不再 FAILED；打开 .java 能出诊断；下断点能停住。

**2026-10-12 已执行完毕 ✅**

| 资产 | 结果 |
|---|---|
| `.artifacts/jdtls` | 已组装，`prepare-jdtls.ps1` 的 `Assert-JdtlsOutput` 全绿（114 plugins、config_win、lombok、java-debug、java-test 27 bundle + runner） |
| `.artifacts/jdk` | 已解压，`java -version` → `openjdk version "21.0.12.1" Temurin-21.0.12.1+1` |

开发期启动命令（exe 够不到仓库根，必须带变量）：

```bash
LITHE_JDTLS_ROOT=D:/developmentProjects/rust/Lithe-IDEA/.artifacts/jdtls \
  ./Lithe.exe
```

`jdtls.rs:584 bundled_jdk_roots()` 会找到 `.artifacts/jdk`；若想让 jdtls 用别的 JDK，
设 `LITHE_JDTLS_JAVA` 或在项目页覆盖（Override 优先级最高）。

**剩下的验证卡在构建**：当前 `Lithe.exe` 是 10-11 22:41 编的，不含阶段 0.1 的 4 个文件改动，
且构建仍受 `explorer.exe` 占 17 GB 的制约（见上文内存小节）。内存释放后重编一次，
即可用上面的命令实机确认 `S1_JAVA_*` 是否转绿。

---

## 2. 阶段 1：编辑器基本功（10/13 → 10/16，4 天）

这一阶段是把「能打开文件」变成「能正常写东西」，优先级高于一切外围集成。

| 能力 | 现状 | 关键落点 |
|---|---|---|
| 新建文件 / 文件夹 / 重命名 / 删除 | 缺失 | `explorer` crate 现在只有 Open / 复制路径 / 刷新；菜单 `MISSING_FILE_LIFECYCLE` |
| 编辑器内查找 / 查找替换 | 未接线 | `lithe-ui/src/editor/find.rs:60` 有 `FindWidget`，`CodeEditor` 没接；菜单 `Find`/`FindAndReplace` 的 `keybind_action` 返回 None |
| 保存全部 / 另存为 / 还原文件 | 未接线 | `menu_bar.rs:620/625/630` |
| 跳转历史（前进/后退） | 未接线 | `editor_view.rs:1083/1094` 的 `go_back`/`go_forward` **已实现但零调用点** |
| 大纲 / 结构面板、面包屑 | 缺失 | editor crate 无对应文件 |
| 缩略图、空白字符、行号开关 | 未接线 | `menu_bar.rs:747/753/758`；`lithe-ui` 有 `minimap()` builder，宿主没开 |
| 文件编码 / 行尾符切换 | 缺失 | 状态栏 UTF-8 是硬编码字符串 |

验收：新建→编辑→保存→重命名→删除全链路走通；Ctrl+F 能在当前文件内查找替换；
关掉再开项目，上次打开的文件还在。

---

## 3. 阶段 2：语义与导航（10/19 → 10/21，3 天）

**强依赖环境前置**：`.artifacts/jdtls` 与 `.artifacts/jdk` 就位（见上文两条脚本）。
这两项是仓库自带的、随安装包发布的资产，开发期只需下载一次，不是装系统级 JDK。

| 能力 | 现状 | 说明 |
|---|---|---|
| 重命名符号 / 查找引用 / 转到实现 / 转到类型定义 | 未接线 | `menu_bar.rs:793/798/803/808` |
| 格式化文档 / 选区 | 未接线 | `menu_bar.rs:709/714` |
| 参数提示 | 未接线 | `menu_bar.rs:682` |
| 诊断波浪线 / 代码动作 / 快速修复 | 骨架 | 依赖 jdtls；缺 jdtls 时诊断与代码动作失效，但补全/hover/跳转有 `lsp.builtin*` 回退仍工作 |

一个必须记住的降级事实：**缺 jdtls 不等于 Java 全线不可用**。
`editor/src/completion.rs:69`、`hover.rs:103`、`navigation.rs:22` 都有内置回退。
被整体拦住的只有调试 —— `start_debug_session`（`workspace.rs:1388`）第一步就要
`service.lsp_session_id()`，拿不到就 `status=blocked reason=no-java-service`。

验收：日志不再出现 `S1_JAVA_FAILED`；打开一个 .java 文件能看到诊断、能 F2 重命名；
在 Spring Boot 工程里下一个断点能真的停住。

断点的前提只是「jdtls 起了」，即 `.artifacts/jdk` 里的 21 被 `jdtls.rs:584 bundled_jdk_roots()`
找到 —— **与本机 `JAVA_HOME` 是 1.8 无关**，不要再去升系统 JDK。

---

## 4. 阶段 3：Git 完整化（10/22 → 10/23、10/26，3 天）

`git` crate 已有 6,633 行，提交与日志图谱是完整的，缺的是主动操作类。

| 能力 | 现状 | 关键落点 |
|---|---|---|
| 新建分支 | 未接线 | `log_view.rs:1048/1099` 工具栏按钮 `enabled=false` 且 handler 是空闭包 |
| 提交差异（diff）查看 | 未接线 | `log_view.rs:1512` 图标按钮 + 空 handler |
| 主动 merge / rebase / cherry-pick | 未接线 | `remote.rs:45/407` 的 `IntegrationOperation` + `integration_gate` 齐备，**全仓无 UI 消费者** |
| 标签管理 | 缺失 | 需新增 |
| Stash | 缺失 | 只有 checkout 的 `autoStash` 开关（`remote.rs:88`） |
| Git 身份配置页 | 未接线 | `identity.rs:173` 的 `set_git_identity_host` 生产代码里从未被调用 |

验收：能建分支、能看某个提交的 diff、能发起一次 merge 并在冲突时看到进行中横幅、
能 stash 再 pop。

---

## 5. 阶段 4：Java 工具链（10/27 → 10/29，3 天）

**依赖 `mvn` 修复**（当前 classworlds 报错，Maven 根本没跑起来）。

| 能力 | 现状 | 说明 |
|---|---|---|
| 执行 Maven 目标 | 缺失 | 设置项文案已写明「gpui 侧还没有执行 Maven 的通路」 |
| 依赖树与 Profile | 缺失 | Maven 工具窗现在只有 reactor 扫描与模块树 |
| Maven settings 直达语言服务 | 缺失 | 需打通 |
| MyBatis 导航 | 缺失 | 旧端 `mybatis` feature 是可用级 |
| Java main / 测试运行 | 缺失 | Core 有 `debug.javaTestLaunch`，gpui 侧零引用 |
| Spring 端点导航 | 未接线 | 面板已在阶段 0.2 点亮，导航（跳转到控制器方法）仍需接 |

验收：Maven 工具窗能执行 `clean install` 并看到输出；能从 Spring 端点跳到控制器方法。

---

## 6. 阶段 5：收尾（10/30 → 10/31，2 天）

- 逐批回填 `shared/platform-feature-matrix/features/*.json`（**这是 CI 门禁，不是可选项**）
- 全量 `cargo test` 与 `./scripts/verify-agent-notes.sh`
- 实机跑一遍完整流程：开项目 → 编辑 → 搜索替换 → 提交 → 运行 → 调试
- 关闭所有 Lithe 进程并确认无残留

---

## 7. 排除线：本月明确不做（19 项）

理由统一为一条：**旧端自己也是「待开发」，平替没有参照价值**，做了也没有验收基准。

GitHub PR / 评论审查 · 数据库工具窗（连接、CRUD 导出、查询历史、schema）· Docker / Compose ·
远程工作区 / WSL 路径与边界 · AI 聊天与 Agent（含 MCP 连接面）· 插件市场安装 ·
更新器 · 内存监视器 · 诊断导出 · IntelliJ Islands 外观与项目颜色渐变 · 任务栏项目标题 ·
PHP 可选插件 · Notebook run actions

---

## 8. 风险登记

| 风险 | 影响 | 应对 |
|---|---|---|
| **`.artifacts/jdtls`、`jdk` 未下载** | Java 语义与调试全部只能写不能验 | 10/12 跑两条 `prepare-*.ps1`；这是唯一硬前置，且是仓库自带脚本，不是装系统 JDK |
| **开发期 exe 找不到 jdtls** | 即使下载了仍报 `S1_JAVA_FAILED` | 启动时带 `LITHE_JDTLS_ROOT` 指向仓库 `.artifacts/jdtls`（正式打包无此问题） |
| **`mvn` 不可用** | 仅阶段 4 的 Maven 目标执行阻塞 | 修 `MAVEN_HOME` 指向可用 Maven；修不好就只做不依赖 Maven 的 Spring 端点导航 |
| 构建 15 分钟/次 | 每天迭代次数被限死 | 功能成批提交，一天最多 3 次构建；优先改 `workbench` 之外的 crate 减少级联 |
| 菜单 `notWired=42` 面太大 | 容易做成「菜单亮了但点了没反应」 | 每接一根线必须同步删掉对应的 `MenuItem::NotWired` 与 `MISSING_*` 分组，不留假入口 |
| 矩阵回填遗漏 | PR 被 CI 门禁挂 | 每阶段结束当天回填，不要攒到最后 |

## 9. 决策记录

| 决策项 | 结论 | 时间 |
|---|---|---|
| Java 语义线是否纳入本月 | **纳入**（阶段 2、4）。前提不是装系统 JDK，而是跑 `prepare-jdtls.ps1` + `prepare-jdk.ps1`；开发期再带 `LITHE_JDTLS_ROOT` 启动 | 2026-10-12 |
| 构建验证受阻时 | **暂停构建，先交付计划**；代码改动保留在工作树，环境恢复后再验证 | 2026-10-12 |
| 矩阵如何登记新前端 | **待定** —— 见下 | — |

**仍待确认**：`shared/platform-feature-matrix/features/*.json` 现在只有 `macos` 与 `windows`
两个平台字段，没有 gpui 端。新统一前端的状态该新增一个平台字段，还是暂记到 `windows` 端？
这决定了阶段 5 回填时的写法，需要早点定，否则返工。

## 10. 阶段 0 进度（截至 2026-10-12 00:30）

| 项 | 状态 | 说明 |
|---|---|---|
| 0.1 会话保存被 exclude 失败一票否决 | **代码已改，未编译验证** | 改了 4 个文件，见下 |
| 0.2 Spring 面板入口 | 未开始 | 落点已定位到行 |
| 0.3 在文件中替换 | 未开始 | 需先在 Core 加 `workspace.replaceApply` |

0.1 的改动（工作树未提交，共 4 个文件）：

- `shared/src/workspace_config/sharing.rs`：`EnsureExcluded` 增加 `Failed` 变体并补文档；
  新增 `degrade_exclusion_failure()`（把失败降级为 `Failed` 并打 `exclusion_degraded` 诊断日志）；
  新增单测 `exclusion_failure_degrades_instead_of_blocking_the_save`。
- `session.rs:139`、`project.rs:149`、`toolchain.rs:246`：三处调用点从 `?` 硬失败改为降级。
  即**会话恢复、项目清单、JDK/Maven 路径覆盖**三处保存不再被 exclude 失败一票否决。

规范依据：`develop-lithe` SKILL 的「Handle failures explicitly」——sharing 层如实返回 `Err`，
由有上下文的调用方决定容忍度并留诊断日志，不静默吞错。
正修（补 Core 的 git 写能力，让 `write_ignore_patterns` 真正可用）另立条目。
