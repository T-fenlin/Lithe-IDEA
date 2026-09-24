# `gpui/docs/` — gpui-kit **0.6.6** 在线文档的本地镜像

这些目录是**抓下来的上游文档**，不是本项目的文档。放在这里有两个原因：

1. **本机 shell 没有可用的 TLS**：`cargo` / `Invoke-RestMethod` 一律报
   `schannel: AcquireCredentialsHandle failed: SEC_NO_CREDENTIALS`，所以子代理与后续会话
   **读不到在线文档**，只能读本地文件。用 `obscura` 抓一次、落到仓库里，大家就都能读。
2. 文档是**规格真源之一**（另一部分是 gpui-kit 已发布源码）。两者不一致时**以源码为准**。

## 目录

| 目录 | 来源 | 对应版本 |
| --- | --- | --- |
| `gpui-kit/0.6.6/` | `https://gpui-kit.com/zh-CN/...`（**默认版、未版本化**的路径） | **已发布 0.6.6** |

### 为什么是"默认版路径"而不是 `/versions/...`

- 站点上**不存在** `/versions/0.6.6/...` 或 `/versions/0.6/...`（实测 404）；版本化的只有
  `/versions/main/...`。
- **默认版（未版本化）文档描述的就是 0.6**：它的建窗口写法是
  `cx.open_window(WindowOptions::default(), |window, cx| …)` + `cx.new(|cx| Root::new(view, window, cx))`，
  与已发布的 0.6.6 完全一致；依赖那行写 `gpui-kit = "0.6"`。
- 因此 **`gpui-kit/0.6.6/` = 默认版文档 = 我们要遵守的规格**。

> **维护者决定（2026-09-25）：规格一律以 0.6.6 的在线文档为准，不使用其它版本的文档。**
> 早先抓过一套 `versions/main/`（160 页）用于对比，已按此决定删除，不要再去抓。

### 为什么 `versions/main` 不能用（留个记录，免得以后又有人去抓）

`versions/main` 描述的是**未发布**的 API，最明显的一处是它用
`gpui_kit::open_window(WindowOptions::default(), cx, …)`（由它自己把视图包进 `Root`），
而 **0.6.6 里没有这个函数**：`gpui-kit-0.6.6/src` 里 grep `pub fn open_window` 零命中，
它自己的文档注释（`lib.rs:37`、`:132`）用的就是 `cx.open_window`；0.6.6 里也没有应用层的
`WindowExt`。要按 main 写就必须把依赖换成 gpui-kit 的 git main 分支，而本机 TLS 坏了、
`cargo/config.toml` 里注释掉的本地代理也没在跑 —— 做不到，所以不采用。

## 重新抓取（幂等，已有非空文件会跳过）

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

## 抓取时踩过的两个坑（都别再犯）

1. **路径前缀**：`llms.txt` 里的链接**带不带版本前缀是不一定的** —— 默认版清单是
   `/zh-CN/...`，`versions/main` 的清单一度是 `/versions/main/zh-CN/...`。
   把捕获到的路径直接当 URL 路径用（站点根 + 该路径），**不要**再拼一次版本前缀，
   也不要用它当"相对仓库根"去建本地路径；否则会得到
   `.../0.6.6/versions/main/zh-CN/...` 这种双前缀，**全部 404**。
2. **不要"失败就删文件"**：站点对未知路径返回一页 404 HTML。如果脚本把 404 内容当抓取结果、
   或者失败后 `Remove-Item`，现象就会看起来像"下完又删了"（本轮真的踩到过）。
   判据要用**内容**：正文 `.md` 以 `---` frontmatter 开头、长度 > 500 B；
   **失败要保留现场**并只记录清单。

## 内容约定

- 每页是原始 markdown（`---` frontmatter + 正文），由 `obscura fetch --dump original` 直取，
  没有经过浏览器渲染或 HTML→markdown 转换，所以与站点上的 `.md` 逐字一致。
- 页面清单来自 `https://gpui-kit.com/llms.txt`（**只在站点根有一份**；
  `/zh-CN/llms.txt`、`/versions/main/zh-CN/llms.txt` 都是 404）。
