# GPUI 架构缺口 · 第 1 步：lithe-core LSP 方法面穷尽核对

**取证脚本**：`reports/gpui-arch-s1-probe.ps1`（只读检索，已实跑）
**运行方式**：仓库根目录 `pwsh -NoProfile -File reports/gpui-arch-s1-probe.ps1`
**基线**：提交 `3085f5d1`，分支 `feat/menu-bar-opaque`
**运行状态**：**已实跑**（2026-10-12，Windows PowerShell 7）。本文所有命中数来自该次输出，未做二次推测。

---

## 结论

7 个 `MISSING_LSP_REQUESTS` 入口分成两类，**分界线不在 UI，在 Core**：

| # | 菜单入口 | menu_bar.rs | Core 有命令？ | 判定 |
|---|---|---|---|---|
| 1 | 格式化文档 `formatDocument` | :711 | **有** —— `LspSemanticOperation::Formatting` → `textDocument/formatting` | UI 缺接线 |
| 2 | 重命名符号 `renameSymbol` | :810 | **有** —— `Rename` → `textDocument/rename` | UI 缺接线 |
| 3 | 转到引用 `goToReferences` | :805 | **有** —— `References` → `textDocument/references` | UI 缺接线 |
| 4 | 转到实现 `goToImplementation` | :795 | **有** —— `Implementation` → `textDocument/implementation` | UI 缺接线 |
| 5 | 转到类型定义 `goToTypeDefinition` | :800 | **有** —— `TypeDefinition` → `textDocument/typeDefinition` | UI 缺接线 |
| 6 | 格式化选区 `formatSelection` | :716 | **无** —— 无 `rangeFormatting`，且 formatting 参数不带 `range` | **Core 缺命令** |
| 7 | 参数提示 `triggerParameterHints` | :684 | **无** —— 全目录 `signatureHelp` 命中 0 | **Core 缺命令** |

**决策含义**：第 1～5 项是「接线活」，不需要动 `lithe-core`（该目录按选型笔记 §9 默认冻结）；第 6、7 项**必须先在 Core 补命令**，界面无论怎么写都补不上。排期时这两项要单列，且不能排在同一批构建里等 UI 改完就验收。

---

## 证据与定位

### 1. Core 的操作变体面（脚本 §A、§B）

`LspSemanticOperation` 定义在 `rust/lithe-core/src/lsp/interface/engine.rs:276-324`，共 **22 个变体**（脚本 §A 中 278–323 行那一段；该段之前的 `Created/Ready/Stopped` 等属于同文件的另外两个枚举，不是本枚举）。

`semantic_method` 映射在 `engine.rs:4222-4246`。实测覆盖（脚本 §B）：

```
4226: Definition       => "textDocument/definition"
4228: TypeDefinition   => "textDocument/typeDefinition"
4229: References       => "textDocument/references"
4230: Implementation   => "textDocument/implementation"
4232: Rename           => "textDocument/rename"
4233: Formatting       => "textDocument/formatting"
```

⇒ 第 1～5 项在 Core 侧**四处都齐**：枚举有变体、`semantic_method` 有映射、`validate_lsp_method` 放行、`feature_request_params` 有参数构造。

### 2. Core 的 method 白名单（脚本 §C）

`validate_lsp_method` 在 `rust/lithe-core/src/lsp/interface/client.rs:673-699`，放行 18 个 method（脚本 §C 的 675–692 行）。其中**没有** `textDocument/signatureHelp`、`textDocument/rangeFormatting`、`textDocument/documentSymbol`、`textDocument/prepareRename`。

同一份白名单的镜像 `feature_request_params`（`client.rs:701-`）在 704–742 行逐 method 构造参数，同样没有这两个分支。

### 3. formatting 参数构造不带 range（脚本 §D）

`client.rs:723-732` 实测原文：

```rust
"textDocument/formatting" => Ok(json!({
    "textDocument": text_document,
    "options": {
        "tabSize": 4,
        "insertSpaces": true,
        "trimTrailingWhitespace": true,
        "insertFinalNewline": true,
        "trimFinalNewlines": true
    }
})),
```

**关键**：`SemanticRequest` 结构体本身**有** `range` 字段（`engine.rs:343`），`inlayHint`（:735）与 `codeAction`（:744）都用 `lsp_range_json(required_range(request)?)` 读它；但 `formatting` 分支**既不读 range，也没有 `required_range` 校验**。

⇒ 「格式化选区」不能靠复用 `Formatting` 操作 + 传 range 实现——即使 UI 传了 range，Core 也会把它丢掉。**这是一处比「缺 method」更隐蔽的缺口**：如果只补 `rangeFormatting` 的 method 而不改参数构造，症状会是「格式化选区和格式化文档行为一样」，不会报错。

### 4. 关键词命中数（脚本 §E，lithe-core 全目录）

| 关键词 | 命中 |
|---|---|
| `signatureHelp` | **0** |
| `SignatureHelp` | **0** |
| `signature_help` | **0** |
| `rangeFormatting` | **0** |
| `RangeFormatting` | **0** |
| `range_formatting` | **0** |
| `documentSymbol` | **0** |
| `DocumentSymbol` | **0** |
| `prepareRename` | **0** |
| `PrepareRename` | **0** |
| `workspace/symbol` | 3 |

**附带发现**：`workspace/symbol` 命中 3 处，全在 `client.rs:1617`（能力协商映射）与 `lsp/tests.rs:2143,2193`（测试 fixture）。它**不在** `validate_lsp_method` 白名单里 ⇒ Core 能协商出 `workspaceSymbols` 能力，却发不出这个请求。这与「大纲/工作区符号」缺口同源，属同一类「Core 半通」状态。

### 5. GPUI 侧实际发出的 operation（脚本 §F）

`rust/lithe-gpui/crates/java/src/service.rs` 全文只有 **5 个** `session.request(...)` 调用点：

| 行 | operation |
|---|---|
| :146 | `definition` |
| :199 | `hover` |
| :253 | `completion` |
| :318 | `codeActions` |
| :671 | `virtualDocument` |

⇒ 即使 Core 支持 `references`/`implementation`/`typeDefinition`/`rename`/`formatting`，GPUI 侧**一个都没发过**。这是「已实现零调用点」的直接证据。

### 6. 7 个入口的菜单落点（脚本 §G）

`rust/lithe-gpui/crates/workbench/src/menu_bar.rs`：

| 行 | label_key | 所属菜单 |
|---|---|---|
| :684 | `lithe.menu.triggerParameterHints` | edit |
| :711 | `lithe.menu.formatDocument` | edit |
| :716 | `lithe.menu.formatSelection` | edit |
| :795 | `lithe.menu.goToImplementation` | go |
| :800 | `lithe.menu.goToTypeDefinition` | go |
| :805 | `lithe.menu.goToReferences` | go |
| :810 | `lithe.menu.renameSymbol` | go |

---

## 风险与条件

| 风险 | 影响 | 触发条件 | 缓解 |
|---|---|---|---|
| **只补 method 不补参数构造** | 「格式化选区」静默退化成「格式化文档」，无报错 | 有人按第 6 项排期时只改 `validate_lsp_method` | 补 `rangeFormatting` 时**必须**同时改 `feature_request_params`，并在 `lsp/tests.rs` 加一条断言「请求体含 range」 |
| **Core 冻结口径** | 第 6、7 项无法在本方向内闭环 | 选型笔记 §9「`lithe-core` 默认不改，缺口走 main 侧补」 | 按 §9 上报，前端先交付不依赖它的部分；或在界面上不画入口（禁止画假按钮） |
| `workspace/symbol` 半通 | 大纲/工作区符号若按「Core 已有」排期会落空 | 有人读到 `feature_name_for_method` 有 `workspaceSymbols` 就以为可用 | 判定口径统一为「`validate_lsp_method` 放行才算 Core 有命令」 |
| 对照端其实也没做 | 第 6、7 项在两端都缺，平替无参照基准 | 排期把它们当「平替 Windows/macOS」 | 见下节限制 |

### 假设与限制（可能推翻结论的证据）

1. **`signatureHelp` 的对照端事实**：Windows 侧 `windows/tauri/src/platform/lsp-core-adapter.ts:1252-1259` 把 `lsp_get_signature_help` 明确列进 `LSP_EXPLICITLY_UNAVAILABLE_COMMANDS`（6 条显式不可用命令之一），macOS 侧全目录检索 `signatureHelp` **命中 0**。⇒ **参数提示在两端都没有实现**。所以第 7 项不是「平替缺口」，是「三端都缺」。若排期把它当平替项，工作量与验收基准都会错。这一条只在「两端现状」范围内成立，不排除 IDEA 规格本身要求它。
2. **Windows 侧同样缺 rangeFormatting**：`lsp_get_document_symbols`、`lsp_get_workspace_symbols`、`lsp_format_range`、`lsp_prepare_rename` 也在那份不可用清单里。⇒ 第 6 项同样是三端都缺。
3. **本文未运行任何 Core 测试**。判定全部来自静态检索与源码阅读。若 `lithe-core` 有运行时能力协商会拒绝某些 method（例如 JDTLS 未声明 `documentFormattingProvider`），实际可用性还会更窄——那属于运行验证，本次**未做**。

---

## 建议与验收

| 优先级 | 行动 | 前置 | 完成标准 |
|---|---|---|---|
| P0 | 补 5 个 UI 接线（文档格式化、重命名、引用、实现、类型定义） | 无（Core 已就绪） | `crates/java` 新增 5 个方法；菜单 5 条 `NotWired` 换成 `Action`；启动日志出现对应 `S1_JAVA_*` 打点；**删除对应 `MISSING_LSP_REQUESTS` 使用点** |
| P1 | 上报 Core 缺口：`rangeFormatting` + `signatureHelp` | 走 main 侧 | Core 侧 `validate_lsp_method` 放行且 `feature_request_params` 构造正确参数；`lsp/tests.rs` 有断言 |
| P2 | 清 `MISSING_LSP_REQUESTS` 分组本身 | P0 完成 | 该分组从 `MISSING_GROUPS`（`menu_bar.rs:154`）移除，`Missing` 常量删除 |
| P2 | 上报 `workspace/symbol` 白名单缺失 | 走 main 侧 | 与大纲/工作区符号排期合并评估 |

**验收方式**：按仓库既有口径，用启动日志的 `S1_*` 打点，不是截图。每接一根线必须同步删掉对应的 `MenuItem::NotWired` 与 `MISSING_*` 分组——留假入口会违反「降级可见」纪律。

**工作量（我负责的架构维度，不含 UI 设计与实现工时）**：
- 第 1～5 项接线：**1.5 人日**（`crates/java` 加 5 个薄封装 + `crates/editor` 加 5 个入口 + 菜单改 5 处 + 各 1 条单测）
- 第 6、7 项 Core 侧设计（接口与参数契约、测试断言）：**1 人日**，实现另计，且**不计入本方向排期**（Core 冻结）
- 依赖：5 项接线之间无依赖，可同一批写完再一起构建（构建 15 分钟/次，必须成批）

---

## 未运行验证清单

- 未运行任何 `cargo` 命令（未构建、未跑测试）。
- 未启动 Lithe 进程，无 `S1_*` 日志。
- 未验证 JDTLS 实际声明的能力集（`documentFormattingProvider` / `renameProvider` / `referencesProvider` 等）——若某 provider 未声明，接线完成后仍会拿不到结果。
- 未在 macOS 或 Linux 上验证（本次为 Windows 主机静态分析）。
