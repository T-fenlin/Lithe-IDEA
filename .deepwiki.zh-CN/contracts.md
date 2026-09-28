# 契约与 Fixture

`shared/` 不是编译目标，而是兼容性表面：它保存了协议文本、JSON schema 和大约 90 个 fixture，用于锁定跨进程的数据形状。

这些内容是防止静默破坏的最后一道防线。只要改动了 Core command、ErrorCode 或工作区配置，通常都必须同步更新 fixture 和协议文档。

## 契约来源

| 关注点 | 真源 |
| --- | --- |
| 应用行为、数据规则、所有权、生命周期 | `shared/contracts/application-boundary.md` |
| Core JSON 命令面、事件 schema、协议版本 | `shared/contracts/rust-core-api.md` |
| Git rebase / patch exchange / repository setup | 对应的 Git contract 文档 |
| GitHub / AI commit / terminal profile | 各自独立的 contract 文档 |
| 更新状态机 | `update-v1` 相关 schema |

## 核心规则

`application-boundary.md` 中的几条规则特别重要：

1. 每个进程与语言边界都必须使用 UTF-8 JSON。
2. 工作区路径以相对路径表示，并统一使用 `/`。
3. 绝对路径可以出现在本地编辑器和 LSP URI 中，但不能作为跨平台持久标识。
4. 各类产品行号都基于 1-based 语义。
5. 编辑器和 LSP 的位置使用 zero-based line + UTF-16 column。
6. 缺失位置用 `null` 表示。
7. 列表必须按确定顺序输出，方便直接比较 fixture。
8. 异步调用必须暴露明确的状态.
9. 失败必须有稳定 `code` 和用户可理解的 `message`。

## JSON schema

仓库中有多个 schema，包括：

- run configuration
- project manifest
- Maven 配置
- toolchain 配置
- theme 定义
- update state

这些 schema 本身就是兼容性面，并且在变更时需要同步更新读写实现。

## Fixture

fixture 负责锁定“真实输出的型和值”。大多数测试会通过 `include_str!` 或 manifest 目录读取 fixture，而不是直接依赖运行时生成值。

像以下几类对象都属于契约表面：

- Core command 返回值
- Git 结果
- Maven / Java workspace 发现
- diagnostics / completion 结果
- 工作区配置结构

## 约束与治理

如果改动了以下内容，就等同于改了兼容性表面：

- command variant
- error code
- request / response 字段
- `.lithe/` 配置结构
- JSON schema

这些修改都必须同步更新：

- 文档
- reader / writer
- fixture
- 相关单测

## 结论

Lithe 的稳定性来自 contracts、schemas、fixtures 这套“可验证的兼容性地基”。它让 UI 侧与 Core 侧不再默契依赖。
