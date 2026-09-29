# GPUI Kit 0.7.0 文档镜像（zh-CN）

本目录是 GPUI Kit 官方中文文档的**逐字离线镜像**，只用于本仓库内的离线查阅与检索：它不参与构建，也不是运行时输入，页面内容未做任何修改。

## 来源与版本

| 项 | 值 |
| --- | --- |
| 来源站点 | <https://gpui-kit.com/zh-CN/docs/> |
| 站点版本 | v0.7.0（抓取时站点标注为最新版本） |
| 抓取方式 | 站点 `sitemap.xml` → 逐页请求 Markdown 端点 `https://gpui-kit.com/zh-CN/<path>.md` |
| 抓取日期 | 2026-09-29 |
| 页面数量 | 183（`docs` 41 / `component` 78 / `base` 49 / `shell` 15） |
| 依赖口径 | `rust/lithe-gpui/` 各 crate 声明 `gpui-kit = "0.7"`，本镜像与该依赖口径一致 |

镜像范围限定为文档四层：`docs`（GPUI 核心概念与应用指南）、`component`（带样式的组件库）、`base`（无样式行为与基础设施）、`shell`（JavaScript 扩展层）。`/zh-CN/apps`、`/zh-CN/contributors`、`/zh-CN/releases`、`/zh-CN/skills` 等非文档页面不在镜像内。

## 目录映射

- 页面路径去掉 `/zh-CN/` 前缀后逐级落到本地目录，并补 `.md` 扩展名：
  `/zh-CN/docs/entity` → [`docs/entity.md`](docs/entity.md)；
  `/zh-CN/component/button` → [`component/button.md`](component/button.md)。
- 四层章节首页与中间层索引页保存为 `index.md`：
  `/zh-CN/docs` → [`docs/index.md`](docs/index.md)；
  `/zh-CN/base/primitives` → [`base/primitives/index.md`](base/primitives/index.md)。
  上游正文使用相对链接（如 `./accordion.md`），按此映射可直接解析。
- 每页头部的 frontmatter `url` 字段记录了原始页面路径，例如 `url: /zh-CN/component/button.md`。

链接完整性已检查：751 条站内相对链接全部可解析，333 条外部链接保持原样。仅两处例外，均为上游文本本身而非镜像缺陷：

- [`component/text-view.md`](component/text-view.md) 中的 `![alt](src)` 是 Markdown 语法示例，不是链接。
- [`docs/comparison.md`](docs/comparison.md) 指向的 `/gallery/` 是站点绝对路径（交互式 gallery），不在文档镜像范围内。

## 许可与署名

依照上游声明：GPUI Kit 有权授权的原创正文与图示以 **CC BY 4.0** 提供，代码示例与软件源码采用 **Apache-2.0**，第三方材料保留各自许可。本目录按 CC BY 4.0 的条件转载，署名 GPUI Kit，链接原文（<https://gpui-kit.com/zh-CN/docs>）与许可条款（<https://creativecommons.org/licenses/by/4.0/>），并声明内容为未修改的逐字镜像。每页正文末尾保留了上游自带的许可说明。
