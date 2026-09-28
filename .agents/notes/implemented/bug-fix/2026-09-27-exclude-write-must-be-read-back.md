# Agent 笔记：本机排除写入必须回读校验（守卫不能宣称没发生的成功）

状态：已实现

## 先说结论

Lithe 在写 `.lithe/` 之前会把 `.lithe/` 追加进 Git 的本机排除文件（`.git/info/exclude`），这样
工作区配置**默认不与团队共享**。原来的实现只要 Core 命令返回成功就打印"排除成功"——而真机上出现过
**命令成功、文件却没变**的情况（写入被环境静默丢弃），于是产品一边宣称"已经排除"，一边让 `.lithe/`
暴露在 `git add .` 的路径上。

现在**写完必须回读**：从排除文件里读回那一行，没读到就报结构化失败
（`SharingError::NotPersisted`）并留诊断，**绝不**再打印成功。失败还会在界面上以**常驻红条**出现，
不再是只有 stderr 的一行。

开发者需要记住两条：**新增任何"写外部文件"的动作都要有回读判据**（"调用成功"不等于"磁盘变了"）；
**测试这条路径不需要真的把磁盘写坏**——把回读判据抽成纯函数直测即可，判据本身才是要守的东西。

## 问题

实测现象（父代理在一台会把工作区之外写入静默丢弃的机器上跑真实应用）：

```text
S1_WORKSPACE_CONFIG excluded root=D:\…\dbx-plugin-k8s pattern=.lithe/
S1_WORKSPACE_CONFIG shell_identity_failed root=D:\…\dbx-plugin-k8s error=清单文件读写失败：拒绝访问。 (os error 5)
```

第一行是**谎报**：随后读那个仓库的 `.git/info/exclude`，里面**仍然是 Git 默认的模板注释，一行
`.lithe/` 都没有**。

危险之处不在"没写成功"，而在**判据反了**：

- 用户以为工作区配置默认不共享，于是放心地 `git add .`；
- 实际上 `.lithe/` 那一行根本没进排除文件，个人状态（本机路径、会话、缓存）会被提交进团队仓库；
- 整个过程只有一行 stderr，界面上什么都不显示。

旧实现为什么察觉不到：`write_git_paths` 把任何成功响应 `.map(|_| ())`，**从不回读**。Core 只负责
"把这一行写进那个文件"，它返回成功就代表它的工作做完了；"文件最终是否含这一行"是**本层**的预期，
不是 Core 的错误。

## 决策

### 一、判据抽成纯函数，位置由 Core 给

- `pattern_is_present(exclude_file, pattern) -> io::Result<bool>`（`sharing.rs`）：**文件不存在 →
  `false`**；逐行 `trim` 后逐字比较（与 Core 写入时的 trim 口径一致）；**注释行不算命中**
  （`# .lithe/` 不是规则）。抽成纯函数是为了能脱离 Core、脱离真实仓库直测。
- `local_exclude_path(root)` 走 **Core 的 `git.watchContext`**，取响应里的 `gitCommonDirectory` 再拼
  `info/exclude`；返回 `null` 表示"这个根不是 Git 仓库"。

### 二、写与删两侧都回读

- `ensure_project_dir_excluded`：写 `excludePatterns` 之后确认那一行**在**；
- `share_project_config`：移除 `unexcludePatterns` 之后确认那一行**不在** —— 这一步不能省，否则后面的
  `stage` 会踩在"文件仍被排除"之上，暂存出一份空的可共享集合；
- 不通过就返回 `SharingError::NotPersisted { operation, pattern, path, reason }` + 诊断
  `S1_WORKSPACE_CONFIG exclude_not_persisted …`，**绝不打 `excluded`**。

### 三、`NotPersisted` 是独立错误类型，不是 `CoreError`

回读失败**不是 Core 报的错**（Core 说成功），是本层发现"预期状态没出现"。所以
`WorkspaceConfigError::Exclude` 从装 `CoreError` 改成装 `SharingError`；`SharingError::Git(CoreError)`
保留 Core 侧失败，并提供与 `CoreError` **同名**的 `is_not_a_repository()`，让"非仓库静默跳过"这条
既有语义的调用方写法不变。顺带把"路径身份也拿不到"从借用 `Exclude` 拆成独立的
`PathIdentity(CoreError)`——原来那个借用语义不对。

### 四、失败必须让用户看见

`ShellWorkspace` 新增字段 `workspace_config_error: Option<SharedString>`，渲染成**常驻红条**
（可手动关，范式照 `rust/gpui/crates/git/src/changes_view.rs` 的 `render_write_error`），插在项目标签条
之下、工作区之上。

**刻意不用状态栏那条 4 秒自动消失的提示，也不用通知/对话框**：

- 自动消失等于把"`.lithe` 没建出来"这件事藏起来，而它是需要用户处理的失败；
- 异步结果回来时只保证**外壳实体还活着**（换根会重建外壳），而对话框与通知要求窗口与 `Root` 就位，
  所以回填走 `this.update(…)` + `notify`。

## 考虑过的备选方案

### 信任 Core 的成功响应（保持原状）
实现成本为零。不采用的原因：真机上"命令成功、文件没变"已经发生过（见问题一节），而信任它会把
"默认不共享"这条产品承诺变成一个不成立的承诺。

### 自己拼 `<root>/.git/info/exclude` 做回读
少一次 Core 往返。不采用的原因：linked worktree 与 submodule 下 `info/exclude` 落在**共享的 common
目录**，`<root>/.git/info/exclude` 根本不存在——回读会永远失败，或者在错误的文件上"成功"。这也是
写入侧一直不硬编码这个路径的同一个理由。

### 把回读失败归进 `CoreError`
少一个类型。不采用的原因：语义不对——Core 会说"我成功了"，而问题恰恰是本层发现预期没出现；混进
Core 的错误分类会让"非仓库静默跳过"的判据（按错误码与消息匹配）产生歧义。

### 用自动消失的提示或系统通知报失败
更轻、代码更少。不采用的原因见第四节：这类失败需要用户处理，而且异步回填时窗口与 `Root` 不一定就位。

### 只测判据、不测回读是否真的接上
更省事。不采用的原因：那样的测试在"有人把回读删掉"时仍然全绿。所以额外补了一条闭环测试：
真实临时仓库里先写别人的规则 → 守卫返回 `Ensured` → **`pattern_is_present` 必须为 true**
（判据写反或回读被删，两条断言不可能同时成立）。

## 后果

**收益**：

- "默认不共享"从**信任调用结果**变成**由磁盘状态证明**；
- 静默丢弃、只读文件、权限策略挡住等所有"写了却不算数"的形态，统一变成一次**看得见的失败**；
- 判据是纯函数，能脱离 Core 与真实仓库直测；回读还顺带证明了回读动作本身不改写那个文件
  （别人的规则逐字保留）。

**代价**：

- 每次写/删排除多一次 Core 只读往返（`git.watchContext` + 读一次文件），发生在打开项目的后台任务里，
  不在 UI 线程；
- 真实权限错误（EACCES）与"静默丢弃"现在表现为同一种失败文案。这是有意的：对用户来说两者都是
  "没做成"，区分细节只进诊断。

## 验证

- `cargo test --workspace`（gpui）：**393 通过 / 0 失败**，其中 `shared` 52 条
  （含 `pattern_is_present_reads_the_documented_cases` 的五档语义、
  `ensure_excluded_verifies_its_own_write_by_reading_back` 的回读闭环）。
- `cargo check --workspace --all-targets`：exit=0（只有两条既存 dead_code 警告）。
- `./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1`：通过。
- `node scripts/verify-agent-notes.mjs`：通过。
- **失败路径的真实复现**（这条以前被记为"无法确定性构造"，实际可以）：把应用的可执行文件放在一个
  **会被围栏丢弃工作区之外写入**的位置，指向工作区之外的仓库，清掉排除文件里的 `.lithe/` 后启动。
  观察到的日志（stdout 里**没有** `excluded`；下面两行在 stderr）：

  ```text
  S1_WORKSPACE_CONFIG exclude_not_persisted operation=excludePatterns pattern=.lithe/ path=…\.git\info\exclude reason=回读时文件里没有该行
  S1_WORKSPACE_CONFIG shell_identity_failed root=… error=确保本机排除失败：excludePatterns 之后回读 … 时没能确认 `.lithe/` 的状态（回读时文件里没有该行）——写入可能被静默丢弃，拒绝宣称成功
  ```

  磁盘真相与日志一致：排除文件里 `.lithe/` **0 行**、`.lithe/` **没有**被创建。
- **正常路径的真实端到端**（可写环境、工作区之外的仓库）：`excluded` → `manifest_saved …
  exclusion=Ensured ignore=Created` → `project_id_created`；`.lithe/project.json` 有 UUID、
  `.lithe/.gitignore` 恰好三行、排除文件里 `.lithe/` **恰好一行**、`git status --porcelain` **为空**。

## 适用范围

- `rust/gpui/crates/shared/src/workspace_config/sharing.rs`
- `rust/gpui/crates/shared/src/workspace_config/project.rs`
- `rust/gpui/crates/shared/src/workspace_config/mod.rs`
- `rust/gpui/crates/workbench/src/workspace.rs`
- `rust/gpui/crates/shared/locales/lithe.zh-CN.yml`
- `rust/gpui/tools/extract-locale.mjs`
- `rust/gpui/PLAN.md`
- `rust/gpui/HANDOFF.md`
