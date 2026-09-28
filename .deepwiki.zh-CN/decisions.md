# 决策与说明

这个仓库的架构决策不是散落在文档里，而是维护为 Agent Notes。它们以中文、生命周期标记和结构化 headings 的形式保存在 `.agents/notes/` 下，是理解“为什么这样做”的真实来源。

## 注意力分配

在动手改代码前，先看：

- `AGENTS.md`
- 相关技能说明
- 目标生命周期下的 note

这比直接看具体接口更重要，因为很多设计决策都体现在所有权边界与兼容性约束中，而不是某个函数里。

## Note 分类

- `implemented`：已落地，默认优先阅读
- `proposed`：未落地
- `rejected`：已否决，避免重复争论
- `archived`：历史背景，供追溯

这些 notes 的名称通常是：

`yyyy-mm-dd-<english-slug>.md`

并且要求固定 headings，以便脚本校验。

## 关键架构 note

最值得优先阅读的几个决定包括：

- repository ownership and sharing boundaries
- language tooling and LSP runtime ownership
- Java build and launch boundary
- Git execution and project console
- UI performance boundaries
- configuration semantics

这些文档通常说明：谁拥有什么进程、谁负责什么缓存、哪些字段必须保持稳定，什么情况属于“跨层接口”。

## 规则

- 任何实现、重构、测试、验证都应先确认对应 note。
- 若涉及兼容性边界、项目的 Ownership 或运行时细节，不要凭印象修改。
- 更新前先看“为什么”，而不是“代码现在是这样写的”。

## 结论

Lithe 的工程真实知识并不只在代码里，Agent Notes 才是历次决策和边界取舍的可追溯来源。
