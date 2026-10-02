# GPUI 宿主：卡点登记册（BLOCKERS）

> **这个文件存在的唯一理由**：停止"原地打转"。
>
> 规则（维护者 2026-09-25 定）：
>
> 1. **同一个问题最多试 3 次**。一次 = 一次 `cargo check`，或一次"翻 gpui-kit 源码找 API"未果。
> 2. 3 次之后**不再继续试**，把现象、已经排除的可能、下一步该怎么查写进下表，然后**继续做别的**。
> 3. 表里的条目只有在**有新的外部输入**时才重新捡起来（拿到源码结论、拿到维护者决策、上游换版本）。
> 4. 状态只有三种：`已登记`（没人接手）/ `已决策`（维护者或实现已拍板怎么做）/ `已解`（做完并留了证据）。

**配套**：任务清单与范围约束 = `docs/tasks/gpui-core-first.md`；界面规格与硬规则 = `UI-MAP.md` §1；
操作手册、环境事实、已丢失的验证工具 = `HANDOFF.md`。**本文件只讲"哪些做不了、为什么"。**

---

## 登记表

| # | 问题 | 现象 / 已经排除的可能 / 已核对的证据 | 试过几次 | 建议的下一步 | 状态 |
| --- | --- | --- | --- | --- | --- |
| B1 | **本机没有 `gh` CLI（但 origin 读方向已可连）** | 2026-10-02 复测：① `Get-Command gh` 无结果；② `git ls-remote --heads origin` **exit=0 且列出全部远端分支** ⇒ 早先记的"git 也连不上 origin（schannel `SEC_E_NO_CREDENTIALS`）"**已不成立**。所以缺的只是"提 PR / 查 CI"这一步，fetch 不受影响。**推方向未验证**（需要凭证，可能仍不可用）。 | 2 | 维护者定：(a) 在主环境提供 `gh` + 凭证；(b) 只 push 分支 + 附 PR 描述草稿交由人工贴；(c) 继续只在本地提交。 | 已登记 |
| B2 | **Core 的两处缺口在本分支修不了** | Core 缺 `plugins.listCatalog`（`plugins/mod.rs` 还是 schema v1）与 `git.write` 的 `excludePatterns` / `unexcludePatterns`；而本分支**不改 `rust/lithe-core/`**（它从 `main` 同步进来，改了会被下次同步覆盖或冲突）。后果：**扩展页恒显"加载失败"**、**`.lithe/` 共享守卫静默失效**。 | — | 由 main 侧修，本分支只上报（材料 = 任务表 §0 的三处回归 + 两处契约超前）。前端能做的只是让这两处**可见、可解释**，不是让它们"好"。 | 已决策 |
| B3 | **仓库的 `scripts/verify-*.sh` 在本机跑不了** | 2026-10-02 复测：`rg` / `ruby` / `awk` 都**不在 PATH**，`find` 还是 `C:\WINDOWS\system32\find.exe`。Node 写的 `scripts/verify-agent-notes.mjs` 可以正常跑。 | 1 | 需要某条 `.sh` 的结论时：改写成 Node / 内联等价检查，或到有这些工具的环境跑。**不要声称跑过的 `.sh` 检查通过。** | 已登记 |
| B4 | **提交行文字顶部会被裁掉约 1/4 行高** | 行盒小于字体 ascent+descent 时 `overflow_hidden` 会裁掉字顶。已排除：行高（`h(22)`→`min_h(22)`）、单独调 `line_height(22)`。**当前实现的定案：提交信息只用 `text_ellipsis()` + `min_h`，不加 `overflow_hidden`。** ⚠️ **本编号被源码引用**：`crates/git/src/log_view.rs:958` 按 `B4` 引用这条（"不要给提交信息加 `overflow_hidden`"）—— **不要改编号、不要删本条**。 | 5（旧口径） | 若在某处复现：先确认那里没加 `overflow_hidden`；仍复现再试 `line_height` 给到比行高更大。 | 已决策 |
| B5 | **合成鼠标/键盘事件只在工作站未锁屏时可用** | 早先记的"本环境无法合成鼠标事件"**已被推翻** —— 后续会话用注入脚本做过大量交互级验证。准确表述：**锁屏时 `SetForegroundWindow` 被拒 ⇒ 真实注入失效**；退路是 `PostMessage`（`WM_LBUTTONDOWN/UP`、`WM_CHAR`、`AttachThreadInput`+`SetKeyboardState`），但它**弱一档**（不过输入队列与前台判定）。另：维护者已确认早期那些"杂散点击"**是他本人手动操作**，不是环境缺陷 —— 所以**不要**据此绕开 GUI 验证、**不要**去"修"一个不存在的派发缺陷、**不要**给面板加防抖。 | 1 | 锁屏时改用**启动态探针**（`--xxx-probe`，与点击回调**同一段代码**，首帧后自动打开），无需注入即可截到新帧。注入类脚本已丢失，要重写见 `HANDOFF.md` §3。 | 已解 |
| B6 | **`cargo build` 报的 1 个 warning 不是代码 warning** | `warning: linker stdout: 正在创建库 …dll.lib 和对象 …dll.exp`，来自 Rust 的 `#[warn(linker_messages)]`，是 MSVC 链接器的正常信息被打成 warning；`lithe-core` 同样报这一条。强制全量重编后本 crate 是 0 code warning。 | 1 | 验收口径改为"**0 code warning**；`linker_messages` 不计入"（或以后统一加 `-A linker_messages`）。**不要为它改代码。** | 已决策 |

---

## 已从本表移除（留一行说明去向，避免有人再翻旧版）

早期条目 B2（两份文档引用的文件不存在，Windows 时代命名漂移）与 B3（`lithe-gpui/target` 不存在 → 冷构建）
**均已随 `UI-MAP.md` 重写与宿主迁入 `rust/` 工作区而消解**，不再登记。
target 锁那条并发约束仍在，已并入 `HANDOFF.md` §1 硬规则 3。

## 登记格式（新条目照抄）

```
| B<n> | **<一句话问题>** | <现象 + 已经排除的可能 + 已核对的证据> | <次数> | <建议下一步> | 已登记 |
```
