# 支撑 crate 与上游锁定

这部分内容描述的是项目中一些“非核心协议层，但对产品机制很关键”的组件：Git 进程适配器、数据库 sidecar、MCP 适配器，以及 `third_party` 的上游管理约束。

## `lithe-git-host`

它负责原生 Git 进程的启动、管道读写、清理和认证处理。它不是 Git 语义层，而是“进程入口与清理适配层”。

它定义了：

- 流式输出读取
- 退出状态
- cancel / cleanup timeout
- askpass 与 credentials 处理

这是为了让 `lithe-core` 保持确定性的命令和结果处理。

## `lithe-db-sidecar`

这是数据库侧车，可作为一个独立子进程运行，提供一个 JSON 请求 / JSON 响应协议。它负责把 SQL / 数据库能力和驱动细节从宿主隔离出去。

特点：

- 一次请求一次响应
- 明确的协议边界
- 写入保护与审计要求
- 只做 database runtime 能力，不负责 UI

## `lithe-db-mcp`

这是一个 MCP stdio Server，它不直接接数据库驱动，而是通过 sidecar 进行调用，负责工具暴露和审计策略。

它的目标是：

- 让官方 MCP 协议接口可用
- 把能力暴露给外部工具
- 通过 policy 和 confirmed 标志约束危险操作

## `third_party`

`third_party/` 的规则非常明确：保留的是清单、版本、URL 和 checksum，而不是完整上下游代码仓库。只在必要时保留受限补丁，并要求版权与许可匹配。

目前主要管理：

- JDT LS
- Temurin JDK
- dbx 相关参考资源

## 结论

这些 crate 的作用不是对本体功能做“第二份实现”，而是维持稳定边界：让 Core 关注协议、Host 关注展示、外围运行时关注实际子进程和资源。
