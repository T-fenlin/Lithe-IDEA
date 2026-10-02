# GPUI 宿主：会话交接（HANDOFF）

> **用法**：新会话开场把本文件给代理，例如
> 「读 `rust/lithe-gpui/HANDOFF.md`、`rust/lithe-gpui/UI-MAP.md` 与 `docs/tasks/gpui-core-first.md`，
> 先跑一遍状态核查（`git log` / `git status` / 构建），再按任务表顶部「范围约束」挑一条做。」
>
> **分工**：本文件 = **操作手册**（怎么跑、坑在哪、现状在哪）。
> 界面规格与官方条文 = `UI-MAP.md`。任务清单与优先级 = `docs/tasks/gpui-core-first.md`。
> 已知卡点 = `BLOCKERS.md`。设计取舍 = `.agents/notes/`。**不要在本文件重复这些。**

---

## 0. 现状（2026-10-02）

| 项 | 值 |
| --- | --- |
| 分支 | `chore/sync-macos-and-lithe-core-from-main`（领先 main 约 215 个提交，**未推送**） |
| 最近提交 | `bedb41ca` BLOCKERS 复测更正 · `f5ae19d7` 工具链事实 · `a3abc3b6` HANDOFF/README 重写 · `f6973789` 任务表 · `d6997d34` UI-MAP 按 0.7 重写 |
| 工作区 | 干净（截至 `bedb41ca`；文档改动均已成提交，无未提交文件） |
| 构建 | `cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe` 通过（约 6m26s，debug exe ≈ 102 MB） |
| `lithe-core` 测试 | 762 条 **761 通过**，1 条 `git::history` 分页测试因本机临时目录权限失败（既有问题） |
| 界面宿主 | 12 个 crate；设置 8 页已实现，其余分类是明确空态 |
| UI 测试 | `cargo test -p lithe-gpui-workbench --test ui` → 1 条通过（`D26` 的测试层，见 §4） |
| 验证 | **只有 Windows**。macOS / Linux 未验证 —— 不要声称 macOS 已验证 |

⛔ **本分支不改 `rust/lithe-core/`**（它从 main 同步进来）。范围约束与任务归属见任务表顶部。

## 1. 硬规则（维护者明确要求过，违反会被打回）

1. **cargo 只跑改动范围**：`cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe`；
   测试用 `cargo test --manifest-path rust/Cargo.toml -p <只动过的 crate>`，能按测试名过滤就加过滤。
   **不要** `-p a -p b` 连带，更不要跑 workspace 全量。
   改完 Rust 必须跑 **`cargo fmt --manifest-path rust/Cargo.toml --all -- --check`**
   （`develop-lithe` 对 `rust/lithe-gpui/crates/*` 的必需验证项；rustfmt 现已可用，见 §2）。
2. **绝不要把 cargo / 任何原生进程的输出接进 PowerShell 管道。**
   `cargo build … | Select-String`、`cargo test … 2>&1 | Select-Object -First N`、`| Tee-Object` **全算违规**
   （慢消费者会给仍在写输出的原生进程发停止信号，表现为"像卡了十分钟"，且 `$LASTEXITCODE` 不可信）。
   固定两步：**先重定向到文件，再只对文件过滤**。

   ```powershell
   # ✅ 正确：原生进程只重定向；管道左手边是 cmdlet（读文件）
   cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe *> .artifacts\build.log
   Write-Output "exit=$LASTEXITCODE"
   Select-String -Path .artifacts\build.log -Pattern "^error|error\[|^warning: unused|Finished" -Context 0,6
   ```
   ```powershell
   # 🚫 禁止：原生进程直接被接进管道（本仓库最常见的自伤）
   cargo build --bin Lithe 2>&1 | Select-String "error"
   cargo test -p lithe-gpui-settings | Select-Object -Last 5
   ```
   汇报里贴**日志文件原文 + `exit=`**，不要贴管道截断的残留。
   ⚠️ 写给子代理的提示词里，**不要把"安全用例"和"禁止用例"放在一起** —— 实测会诱导代理把 cargo 也接进管道。
3. **同一时刻只有一个代码写者**（`rust/target` 是排他锁；验证脚本还会 `Stop-Process Lithe` 互相杀掉对方的实例）。
   **文档/审计类任务可以并行**（只读源码 + 各写各的文件）。
4. **只读区**：`macos/`（规格真源）、`shared/`、`.agents/`、`third_party/` 对实现代理是只读。
   `rust/lithe-core/` 见顶部范围约束 —— 本分支**不改**。
5. **子代理不 commit / 不 `git add`**，由主代理复核后提交。
   主代理提交时：**只 `git add` 需要的那几个路径**（`git commit` 不带路径会提交整个索引），
   提交前先看 `git status --short`。
6. **文案**：界面不许出现中英文字面量（注释除外），走 `lithe_gpui_shared::tr`；新键**同时**进两份 yml +
   `i18n.rs` 的 `WIRED` 清单。**yml 由 `lithe-gpui/tools/extract-locale.mjs` 生成，不要手改**；
   Windows 缺的键加它的 `GPUI_ONLY_KEYS`。
7. **颜色只走 `cx.theme()`；布局一律 rem 档位 helper**（`p_2()` / `gap_3()` / `text_sm()`…），
   档位外的值用 `rems(P / 16.)`（**不是 `/4.`**，1 rem = 16px）并写注释。
   守卫：`node lithe-gpui/tools/check-ui-px.mjs`（扫 `crates/*/src/**/*.rs` 的直接 `px(...)`，白名单外命中 → 退出码 1）。
8. **诊断走 stderr（`eprintln!`）**：stdout 重定向到文件是**块缓冲**，进程还在跑时日志里可能一行都没有。
   所以"读日志验证"要读 `.err`，或把断言结果写到文件再从文件读。
9. **验证要到交互级**：真实鼠标/键盘注入 + 截图 + 像素/几何测量 + 在 PowerShell 侧独立核对副作用
   （例如 stage 后 `git diff --cached --name-only`、提交后 `git log -1 --oneline`）。
   只看"界面看起来对"不算完成。**某条路 3 次没弄好就停下**，把现象/已试方案/证据写进 `BLOCKERS.md` 并汇报。
10. **结束时杀掉自己启动的 `Lithe`**（含 `java` 子进程与终端的 powershell 子进程）。
    ⚠️ **强杀 Lithe 不走 `lsp.stopServer`** ⇒ JDTLS 的 `java.exe` 会留在后台，要单独 `Stop-Process`。

## 2. 环境事实（不遵守会得出错误结论）

- **显示器 125% DPI：截图是物理像素，逻辑值 × 1.25 = 截图像素**。窗口截图约 1823×1024。
  例：对话框 820×620 逻辑 = 1025×775 物理；左栏 190 逻辑 = 237 物理。
- **工作站会锁屏。** 锁屏时 `SetForegroundWindow` 被拒 → 真实注入失效。
  锁屏时优先做**启动态旗标验证**：加一个 `--xxx-probe` 开关，与点击回调**走同一段代码**，首帧后自动打开，
  无需注入即可截到新帧。
- **`PrintWindow` 在无人值守下只返回"上次绘制那一帧"**：窗口不再重绘时两张截图可能 SHA 相同却已生效。
  所以"投递点击触发重绘再截图"**不可靠**，用启动态探针拿新帧。
- `Lithe.exe` 是**控制台子系统**程序（debug 保留控制台）→ 窗口枚举**必须排除控制台窗口**；
  gpui 的窗口类名是 **`Zed::Window`**。
- **rustfmt 可用**（2026-10-02 复测；早先记的"没有 rustfmt"已作废）：
  `rustfmt` / `cargo fmt` 均为 **1.9.0-stable**，在 `D:\ProgramData\rust\cargo\bin\`。
  `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` 实测 **exit 0 / 零输出**
  （整个 workspace，含 `lithe-core`，当前完全没有格式差异）。
  ⚠️ 但**不要对全 workspace 跑 `cargo fmt`**：它会一并重写 `rust/lithe-core/`，而本分支不改那里。
  只对自己的 crate 跑（`cargo fmt --manifest-path rust/Cargo.toml -p <crate>`），或跑 `--check` 后手工处理。
- `scripts/verify-*.sh` 系列（除 Node 写的）**本机跑不了**：依赖 `rg`、`ruby`、`awk`，本机三者都缺
  （`find` 还是 Windows 自带的 `find.exe`）。`node scripts/verify-agent-notes.mjs` 可以跑。
- `verify-rust-core.sh` 是 **zsh/macOS** 脚本（Swift 桥 / C ABI 部分需 macOS 侧）。
- **JDTLS 载荷**：`.artifacts/jdtls`（61.7 MB，`bin/jdtls.bat`）；跑它要 **JDK 21+**
  （`D:\ProgramData\java\openjdk-21\bin\java.exe`，PATH 上那个是 1.8 不能用）。
- **跑 gpui 测试要把 `TEMP`/`TMP` 指到工作区内可写目录**（本机 `%TEMP%` 对 Rust 测试进程返回
  `PermissionDenied`）；副作用是"临时目录"落在工作树内，所以测试夹具一律先自己 `git init`，
  否则守卫会去改**真实检出**的 `.git/info/exclude`。
- **已验证的环境边界**：工作区内的可执行文件写不出工作区（`Lithe.exe` 连 `%APPDATA%\Lithe` 都写不了）；
  **把同一个 exe 复制到工作区之外就正常**。GUI 端到端要这么跑：工作区外的 exe + 工作区外的临时 Git 仓库 +
  `LITHE_GPUI_SETTINGS_FILE` 指向工作区外。

## 3. ⚠️ 验证工具**已经不在了**

早期会话的注入/截图脚本与阶段目录**都不在当前工作区**（`.artifacts/` 现在只剩构建日志、`probe/`、
`ui-review/`、`icon-set/`、`recon/` 等）。**以下全部丢失，不要去找**：

`lithe-gpui/capture-screenshot.ps1` · `.artifacts/ui-click.ps1` · `.artifacts/ui-type.ps1` ·
`.artifacts/verify-visual.ps1` · `.artifacts/session-e2e.ps1` · `.artifacts/p5…p15/`（各阶段验证脚本与 NOTES）·
`lithe-gpui/PLAN.md` · `lithe-gpui/research/**` · `lithe-gpui/docs/gpui-kit/0.6.6/**`

需要时**重新写**，或从 git 历史取回。重写注入脚本要记住的坐标换算：
**客户区坐标 = 截图坐标 − (客户区屏幕左上 − 窗口矩形原点)**（实测内缩 ≈ (9,0)）。

查 API 不要去翻已经删掉的 0.6.6 镜像，用**仓库内的 0.7.0 镜像 `docs/gpui kit/0.7.0/`**（184 份）或
registry 里已解析的源码。

## 4. 已知缺口（别当已解决；逐区域细节见 `UI-MAP.md` §2.2）

**与 Core 有关（本分支不动，等 main）**：

- **扩展页恒显"加载失败"**：Core 缺 `plugins.listCatalog`（schema 还是 v1），v2 fixture 也没内嵌。
- **`.lithe/` 共享守卫静默失效**：Core 缺 `git.write` 的 `excludePatterns` / `unexcludePatterns`，
  调用方拿 `invalid_request: "Unsupported Git write operation"`。
- 前端能做的只是**让这两处可见、可解释**，不是让它们"好"。上报材料见任务表 §0。

**gpui 侧**：

- **终端不是真终端**：`std::process::Command` + 三管道 + 丢控制序列，**无 PTY、无 resize、无 Ctrl+C、无 SGR**；
  shell 探测硬编码 Windows `["powershell","cmd"]` ⇒ macOS 上不可用。
- ✅ **运行启动不再阻塞 UI 线程**（D10）：`preLaunchSteps`（含 `javac`）移进 `background_spawn`，
  回前台只剩 `spawn` + 建读线程。
- ✅ **状态栏两格接上真值**（D13）：缩进取 `EditorPane::tab_size()`、只读态取
  `EditorPane::active_buffer_is_readonly()`（图标在 `LOCK_ICON` / `LOCK_OPEN_ICON` 间切换）。
  ✅ **分支项不再每帧读 `.git/HEAD`**（D11）：改读 Git 视图已持有的 `git.status` 快照
  （`ChangesView::branch()`），worktree / 分离 HEAD / 仓库根≠工作区根都不再读错。
  状态栏**仍剩**两处假值：内存 `"总计 0.0 MB · Lithe 0.0 MB"`、Git 更改数空格；
  底部工具窗的拖拽热区仍是死的 `div`。
- **主菜单栏没有键盘路径**（Tab 聚焦 + Enter 未实现；gpui 无 mnemonic 概念，`Alt` 助记键做不到）。
- **分支弹窗两处偏差**：页签行在搜索框**上方**（真源在下方）、搜索行右侧没有关闭按钮。
- **项目下拉**面板右侧有一条常显细滚动条（`scrollable(true)` 的 16 逻辑 px 占位）。
- **文件类型图标只部分验证**：`pom.xml` 画出了 Maven 图标；`.gitignore` / `README.md` / **文件夹图标**未确证。
- **设置页剩下的**：`autoCompletion` 热生效没人实机点过；`mavenExecutablePath` / `mavenJavaHomePath` /
  `mavenLocalRepositoryPath` **无消费方**；运行配置页**没有运行按钮**（执行未接入）；LSP 页缺 jdtls 运行时路径。
- **会话状态**：`.lithe/session.local.json` 已落地，但**没有 watcher**（外部手改要重开项目）、
  没有 `directoryMarks`（gpui 侧没有"标记目录"这个交互）。
- **`Ctrl+,` / `Ctrl+Shift+P` 的键盘入口始终没机器验证过**（注入侧 `SetForegroundWindow` 常被系统拒绝）；鼠标路径都验过。
  ⚠️ **这条现在有解了**：`D26` 已接上 UI 集成测试层（`crates/workbench/tests/`），
  `window.press(..)` / `window.click(..)` 走 GPUI 自己的事件派发，**不依赖窗口拿到前台焦点** ——
  那批"只能靠真机手测"的条目（菜单键盘路径、底部工具窗拖拽热区、浮层焦点归还）应当改用这一层来钉。

**技术债（小改动、高收益，可插队，见任务表 §4.3）**：✅ `rem_px` 已由 5 份收敛为 `shared` 一份（`D16`）；
✅ 生产代码的直接 `px()` 已清零（`D17`），守卫实测 **exit 0 / 违规 0 处**（余下 11 处是白名单里的 7 条平台/token 例外）；
✅ `D26` 已接上 UI 集成测试层（见 §4 的 `ui` 测试）；

## 5. 文档索引（按此顺序读）

| 文件 | 内容 |
| --- | --- |
| `rust/lithe-gpui/UI-MAP.md` | **接手第一份**：§1.1 度量规则 · §1.4 官方指南 27 组条文 + 逐条现状核查 · §2 17 个区域对照 · §3 验收清单 |
| `docs/tasks/gpui-core-first.md` | 任务清单（69 条）+ **顶部范围约束** + §6 开工顺序 |
| `rust/lithe-gpui/README.md` | 怎么跑、12 个 crate 的职责与依赖、已验证接口事实、硬约束 |
| `rust/lithe-gpui/BLOCKERS.md` | 卡点登记册（哪些做不了、为什么）。⚠️ **B4 的编号被 `crates/git/src/log_view.rs:958` 引用，不要改编号或删该条** |
| `docs/gpui kit/0.7.0/` | 官方文档镜像 184 份（`docs/` 41 · `component/` 78 · `base/` 49 · `shell/` 15） |
| `.agents/notes/` | 架构决策真源（中文）；`.agents/skills/develop-lithe/SKILL.md` 是编码与验证规则真源 |
