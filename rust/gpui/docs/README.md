# `gpui/docs/` — 文档索引

这个目录放**非源码**资料。分两层看：`gpui/` 根下是**活着的工作文档**（每次开工先读），
`gpui/docs/` 与 `gpui/research/` 是**参考资料**。

## 一张表看懂放哪儿

| 路径 | 是什么 | 什么时候读 |
| --- | --- | --- |
| `gpui/README.md` | 怎么跑、已核实的接口事实、硬约束 | 第一次进来 |
| `gpui/PLAN.md` | **唯一的工作计划**：下一步做什么、怎么算做完、当前差距与历史轮次结论 | 每次开工 |
| `gpui/BLOCKERS.md` | 卡住的问题登记册（同一问题试 3 次即登记，不再原地打转） | 卡住时 |
| `gpui/UI-MAP.md` | **实现硬规则**：单位、gpui-kit 版本陷阱、实测踩过的坑（§1） | 写代码前 |
| `gpui/UI-MAP-WINDOWS.md` | **界面规格真源**：Windows 元素 → gpui-kit 0.6.6 的逐区域对应表（数值 + 文案 + 行号） | 做某个区域前 |
| `gpui/docs/gpui-kit/0.6.6/` | **上游官方文档镜像**（见下） | 查 API / 架构 / i18n / 主题 |
| `gpui/research/windows/01..06-*.md` | Windows 前端 6 份区域调研（`UI-MAP-WINDOWS.md` 的索引来源，被它逐条引用行号） | 需要回到原始出处时 |
| `gpui/research/gpui-kit-0.6.6-api.md` | 已发布 0.6.6 的 API 事实清单（123 个 ThemeColor token、101 字形、28 条"错误写法→正确写法"） | 不确定某个 API 是否存在时 |
| `gpui/research/gpui-kit-overlay-howto.md` | 浮层（Dialog / Sheet / Notification）调研与实测陷阱 | 做浮层前 |
| `gpui/docs/grill.md` | 本轮重写计划的**设计树质询**：12 条 frontier 决策 + 证据 | 想知道"为什么这么定"时 |
| `gpui/docs/archive/` | 已降级、仅留档的资料 | 需要历史时 |

## 上游文档镜像：`gpui/docs/gpui-kit/0.6.6/`

抓的是 `https://gpui-kit.com/zh-CN/...`（**默认版、未版本化**路径）下的 zh-CN 文档全量
**151 页**，`obscura fetch <url> --dump original` 直取原始 markdown。

### 为什么是"默认版路径"而不是 `/versions/...`

- 站点上**不存在** `/versions/0.6.6/...` 或 `/versions/0.6/...`（实测 404）；版本化的只有
  `/versions/main/...`。
- **默认版文档描述的就是 0.6**：它的建窗口写法是
  `cx.open_window(WindowOptions::default(), |window, cx| …)` + `cx.new(|cx| Root::new(view, window, cx))`，
  与已发布的 0.6.6 完全一致；依赖那行写 `gpui-kit = "0.6"`。
- 因此 **`gpui-kit/0.6.6/` = 默认版文档 = 本项目遵守的规格**。

> **维护者决定（2026-09-25）：规格一律以 0.6.6 的在线文档为准，不使用其它版本的文档。**
> 早先抓过一套 `versions/main/`（160 页）用于对比，已按此决定删除，不要再去抓。

### 为什么 `versions/main` 不能用（留记录，免得以后又有人去抓）

`versions/main` 描述的是**未发布**的 API，最明显的一处是它用
`gpui_kit::open_window(WindowOptions::default(), cx, …)`（由它自己把视图包进 `Root`），
而 **0.6.6 里没有这个函数**：`gpui-kit-0.6.6/src` 里 grep `pub fn open_window` 零命中，
它自己的文档注释（`lib.rs:37`、`:132`）用的就是 `cx.open_window`；0.6.6 里也没有应用层的
`WindowExt`。要按 main 写就必须把依赖换成 gpui-kit 的 git main 分支，而本机 TLS 坏了
（`schannel: SEC_E_NO_CREDENTIALS`）、`cargo/config.toml` 里注释掉的本地代理也没在跑 —— 做不到。

### 重新抓取（幂等，已有非空文件会跳过）

```powershell
$exe  = "C:\Program Files\obscura-x86_64-windows\obscura.exe"
$llms = ".artifacts\docs-probe\llms-default.txt"   # https://gpui-kit.com/llms.txt
$dest = "gpui\docs\gpui-kit\0.6.6"
# 默认版清单里的 zh-CN 链接是 /zh-CN/...（**不带**版本前缀），所以 URL = 站点根 + 该路径。
$paths = [regex]::Matches((Get-Content $llms -Raw), '\((/zh-CN/[^)]+?\.md)\)') |
    ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
foreach ($p in $paths) {
    $file = Join-Path $dest (($p.TrimStart('/')) -replace '/', '\')
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $file) | Out-Null
    if ((Test-Path $file) -and (Get-Item $file).Length -gt 500 -and
        ((Get-Content $file -TotalCount 1) -notmatch '<!DOCTYPE')) { continue }
    & $exe fetch "https://gpui-kit.com$p" --dump original |
        Set-Content -Path $file -Encoding utf8
}
```

先取清单：

```powershell
& $exe fetch "https://gpui-kit.com/llms.txt" --dump original |
    Set-Content -Path ".artifacts\docs-probe\llms-default.txt" -Encoding utf8
```

### 抓取时踩过的两个坑（都别再犯）

1. **路径前缀**：`llms.txt` 里的链接**带不带版本前缀是不一定的** —— 默认版清单是
   `/zh-CN/...`，`versions/main` 的清单一度是 `/versions/main/zh-CN/...`。
   把捕获到的路径直接当 URL 路径用（站点根 + 该路径），**不要**再拼一次版本前缀，
   也不要用它当"相对仓库根"去建本地路径；否则会得到
   `.../0.6.6/versions/main/zh-CN/...` 这种双前缀，**全部 404**。
2. **不要"失败就删文件"**：站点对未知路径返回一页 404 HTML。如果脚本把 404 内容当抓取结果、
   或者失败后 `Remove-Item`，现象就会看起来像"下完又删了"（本轮真的踩到过）。
   判据要用**内容**：正文 `.md` 以 `---` frontmatter 开头、长度 > 500 B；
   **失败要保留现场**并只记录清单。

### 内容约定

- 每页是原始 markdown（`---` frontmatter + 正文），没有经过浏览器渲染或 HTML→markdown 转换，
  所以与站点上的 `.md` 逐字一致。
- 页面清单来自 `https://gpui-kit.com/llms.txt`（**只在站点根有一份**；
  `/zh-CN/llms.txt` 与 `/versions/main/zh-CN/llms.txt` 都是 404）。

## 已删除 / 已归档（2026-09-25 文档分层）

| 文件 | 处置 | 理由 |
| --- | --- | --- |
| `gpui/PLUGINS.md` | **删除** | Zed 式 WASM 插件方向调研。维护者已定"插件暂时不弄，等 gpui-kit 的插件出来"；而 0.6.6 官方文档里已经有 gpui-shell 那套真实的插件机制（`zh-CN/shell.md`、`shell/host-module.md`、`shell/capabilities.md`），这份调研已被取代。仓库里 **0 处引用**；需要时从 git 历史取。 |
| `gpui/UI-MAP-macos.md` | **移到 `docs/archive/`** | 界面规格来源早已改为 Windows + 0.6.6 文档，macOS 降级为"行为对照"。仍有 8 处引用，且 `macos/` 前端尚未删除，所以**归档而不是删除**；等 `macos/` 删掉时一并处理。 |
| `gpui/GRILL.md` | **移到 `docs/grill.md`** | 一次性产物（本轮计划的设计树质询），不属于每次开工必读的工作文档。 |

`gpui/research/` **没有动**：它已经是独立一层，而且 `UI-MAP-WINDOWS.md` 逐条引用其中的
`文件:行号`，`UI-MAP.md` 与 `PLAN.md` 也引用它 —— 挪位置只会制造几十处失效引用，没有收益。
