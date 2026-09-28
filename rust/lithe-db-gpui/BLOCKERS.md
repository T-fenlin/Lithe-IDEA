# GPUI 重写：卡住的问题登记册（BLOCKERS）

> **这个文件存在的唯一理由**：停止"原地打转"。
>
> 规则（维护者 2026-09-25 定，替代 `PLAN.md` 里"同一问题最多试 5 次"的旧口径）：
>
> 1. **同一个问题最多试 3 次**。一次 = 一次 `cargo check`，或一次"翻 gpui-kit 源码找 API"未果。
> 2. 3 次之后**不再继续试**，把现象、已经排除的可能、下一步该怎么查，写进下面这张表，然后**继续做别的**。
> 3. 表里的条目只有在**有新的外部输入**时才重新捡起来（拿到了源码结论、拿到了维护者决策、上游换了版本）。
> 4. 条目状态只有三种：`已登记`（没人接手）/ `已决策`（维护者拍板了怎么做）/ `已解`（做完并留了证据）。

配套：`PLAN.md` 讲"下一步做什么"，`UI-MAP.md` §1 讲硬规则，本文件讲"哪些做不了、为什么"。

---

## 登记表

| # | 问题 | 现象 / 已经排除的可能 | 试过几次 | 建议的下一步 | 状态 |
| --- | --- | --- | --- | --- | --- |
| B1 | **本机没有 `gh` CLI，`git` 也连不上 origin** | ① `Get-Command gh` 无结果；② `C:\Program Files\GitHub CLI\gh.exe`、`%LOCALAPPDATA%\Programs\GitHub CLI\gh.exe`、chocolatey/scoop scoop shims 都不存在；③ 无 `GH_*` / `GITHUB_*` 环境变量、无 `~/.config/gh/hosts.yml`；④ `winget` 也不可用（exit 1）；⑤ `git ls-remote origin preview` 报 `schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS`，`credential.helper = manager` 但拿不到凭证。**结论：`AGENTS.md` 要求的"从最新 preview 开分支 + `gh` 提 PR"在当前环境无法执行。** | 1 | 维护者决定：继续在 `main` 上做、之后再拆分支；或在主环境提供 `gh` + 凭证；或本轮只在本地提交 + 附上 PR 描述草稿。 | 已登记 |
| B2 | **两份文档引用的文件不存在（命名漂移）** | `lithe-db-gpui/UI-MAP.md:11` 与 `lithe-db-gpui/PLAN.md:12` 引用 `docs/development/gpui-ui-windows-prompt.md`，实际文件名是 `docs/development/gpui-ui-windows-rewrite-prompt.md`；两份文档都引用 `lithe-db-gpui/UI-MAP-WINDOWS.md`，该文件**不存在**（正在由子代理产出）；`lithe-db-gpui/docs/archive/ui-map-macos.md` 已被工作区删除（仅存于 git 历史），但 `UI-MAP.md:31` 仍把它列为配套文件。 | 1 | **已处理**：`UI-MAP.md:11` 与 `PLAN.md:12` 的提示词路径已改对；`lithe-db-gpui/docs/archive/ui-map-macos.md` 已从 git 历史恢复（它的消失属于"清空重写"时误删，计划里它是配套的行为对照文档）；`UI-MAP-WINDOWS.md` 由子代理产出中。 | 已解 |
| B3 | **`lithe-db-gpui/target/` 不存在 → 首次构建是完全冷构建** | 上一轮会话把 `target/` 一起清掉了（`lithe-db-gpui/` 下 `Cargo.toml`、`Cargo.lock`、整个 `shell/` 都处于工作区删除状态）。已从 `HEAD` 恢复清单，冷构建已在后台跑。**风险：`cargo check` 会独占 `lithe-db-gpui/target` 锁，多个子代理同时构建会互相阻塞。** | — | 构建与 `cargo check` **只由主代理执行**，区域子代理只写代码不构建。 | 已登记 |
| B4 | **提交行文字顶部被裁掉约 1/4 行高**（历史遗留） | 上一轮已用掉 5 次尝试上限。已排除：行高（`h(22)`→`min_h(22)`）、`line_height(22)`。当时判断是行盒小于字体 ascent+descent，`overflow_hidden` 裁掉了字顶。**该项属于旧实现（工作区已删），重写后需重新确认是否复现。** | 5（旧口径） | 重写后若复现：试"`line_height` 给到比行高更大"或"去掉 `overflow_hidden` 只留 `text_ellipsis`"。 | 已登记 |
| B5 | **本环境无法合成鼠标事件** | `SetCursorPos` 后光标停在 (0,0)、`GetForegroundWindow()` 返回 0（上一轮记录）。**结论：所有点击/悬停/拖拽类交互只能由维护者在真机验证，代理只能验证"启动路径 + 静态布局 + 截图"。** 本轮实测印证：`capture-screenshot.ps1` 只调 `SetForegroundWindow`（`capture-screenshot.ps1:75`）、**不合成输入**，但截图里编辑区出现了被打开的文件 —— 那是维护者本人点击项目树的结果，也顺带证明了"点文件 → 开标签 → 显示真实内容"这条链路是通的。 | 1 | 需要交互验证的验收项一律标注"待维护者实机确认"，不要反复尝试注入事件。 | 已解 |
| B6 | **`cargo build` 报的 1 个 warning 不是代码 warning** | `warning: linker stdout: 正在创建库 ...dll.lib 和对象 ...dll.exp`，来自 Rust 的 `#[warn(linker_messages)]`，是 MSVC 链接器的正常信息被打成 warning；`lithe-core` 同样报这一条。**强制全量重编（`cargo clean -p lithe-db-gpui-shell` + `cargo check`）后本 crate 是 0 code warning。** | 1 | 验收口径改为"**0 code warning**；`linker_messages` 不计入"（或以后统一加 `-A linker_messages`）。不要为它改代码。 | 已决策 |

---

## 登记格式（新条目照抄）

```
| B<n> | **<一句话问题>** | <现象 + 已经排除的可能 + 已核对的证据> | <次数> | <建议下一步> | 已登记 |
```
