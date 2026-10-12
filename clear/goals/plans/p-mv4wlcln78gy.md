# 阶段归档 · p-mv4wlcln78gy

- 收束时间:2026-10-12T07:12:31.466Z
- 所属目标:g-mv4wev7xqmar
- 收束之辞:四步全部交付，产出 4 份报告 + 3 个可重跑取证脚本（均已实跑）。

**拿到的结论**：
1. **LSP 面**：Core 有 22 个语义操作，GPUI 只发 5 个。7 个 MISSING_LSP_REQUESTS 入口中，5 项（格式化文档/重命名/引用/转到实现/转到类型定义）是纯 UI 接线缺口；2 项（参数提示、格式化选区）是 **Core 缺命令** —— `signatureHelp` 与 `rangeFormatting` 在 lithe-core 命中 0，且 `formatting` 的参数构造（client.rs:723-732）不读 `request.range`（对照 inlayHint:735、codeAction:744 都读），只补白名单会造成「格式化选区=格式化文档」的静默退化。
2. **接线缺口量化**：组件库 lithe-ui 已有 SymbolOutline/GoToSymbol/CallHierarchy/RenameInput/CodeActionMenu/SignatureHelp/FindWidget 等，宿主零引用；并发现双实现（宿主自建 ProblemsPane、自组 Menu）。
3. **DAP/Git**：Core 18 个 debug 命令里 8 个 GPUI 零引用；取消语义（debug.cancelOperation + lsp.cancelOperation）整体缺失，属一个架构缺口而非两个功能。Core git.write 接受 26 种 operation，GPUI 只发 8 种。
4. **文本模型**：两处 O(n) 每次编辑 —— Buffer::replace 整篇重建行表（buffer.rs:265-278），Highlighter::reparse 传 None 旧树放弃增量解析（language/mod.rs:259-289）。

**台账复核发现 2 处不一致**（已交给人）：§2.2 Git 身份配置页「生产代码零调用」不成立（workspace.rs:883 有生产调用）；§2 Action 计数 52 vs 实测 51。

**自查缺陷 1 处已修复**：三个脚本首版未排除 target/，致 viewer 虚报 1，已加排除并重跑。

**工作量**：架构维度 13.0 人日，分 10 项，含依赖顺序与构建编批建议。

**未运行**：任何 cargo 命令、Lithe 进程、JDTLS 能力协商、真实 git 写操作、macOS/Linux 验证 —— 全部结论建立在静态检索与源码阅读之上。

## 步骤
- [advanced] 1. 穷尽核对 lithe-core 的 LSP 方法面：从 LspSemanticOperation 枚举、semantic_method 映射、validate_lsp_method 白名单、feature_request_params 参数构造四处交叉，逐条判定 7 个 MISSING_LSP_REQUESTS 入口各自是「Core 有命令」还是「Core 缺命令」。写一个可重跑的 pwsh 脚本落到 reports/，把检索命令原文与命中数一起留档。 → reports/gpui-arch-s1-core-lsp-surface.md
- [advanced] 2. 量化 GPUI 侧的接线缺口：把 rust/lithe-ui 的编辑器 widget 导出清单（SymbolOutline/GoToSymbol/CallHierarchy/RenameInput/CodeActionMenu/FindWidget/SignatureHelp/ProblemsPanel 等）与 rust/lithe-gpui 宿主的实际引用做差集，逐项给出「组件库有 + 宿主零引用」或「两侧都没有」。同时核对 crates/editor 9 个模块的现状与 menu_bar 里 15 个 MISSING_* 分组的对应关系。 → reports/gpui-arch-s2-wiring-diff.md
- [advanced] 3. 核 DAP 与 Git 两侧的命令差集：把 lithe-core 的 debug.* 与 git.* 命令全集分别与 GPUI 侧引用做差集，标出「Core 已实现但 GPUI 零引用」。同时复核证据台账 §2 的计数：MENUS 数组边界（实测 595–935）、Action 条数（台账 52 vs 实测 51），以及台账 §2.2 称 Git 身份配置页「生产代码零调用」是否仍成立。 → reports/gpui-arch-s3-dap-git-diff.md
- [advanced] 4. 汇编交付文档：按任务书要求的格式写「结论 / 证据与定位 / 风险与条件 / 建议与验收 / 交接给主理人」，含模块划分与依赖方向、关键数据结构与接口签名现状、文本模型增量性结论、风险清单、以及对我负责维度的工作量估算（人日）与先后顺序。明确标注未运行的验证。 → reports/gpui-arch-gap-analysis.md
