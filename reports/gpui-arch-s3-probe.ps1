# Step 3 取证脚本：DAP / Git 命令差集 + 证据台账计数复核
#
# 用法（仓库根目录）：
#   pwsh -NoProfile -File reports/gpui-arch-s3-probe.ps1
#
# 目的：
#   A. lithe-core 已实现的 debug.* 命令 vs GPUI 引用 —— 找出「Core 有、UI 零引用」
#   B. lithe-core 已实现的 git.* 命令 vs GPUI 引用 —— 同上
#   C. 复核证据台账 §2 的三处计数与一处「零调用」断言
# 只读检索，不改仓库。

$ErrorActionPreference = 'Stop'

# 只扫源码：rust/lithe-gpui/target/ 下有构建期生成的 .rs（14 个），
# 混进来会让命令名与关键词的命中数虚高。
$Sources = Get-ChildItem -Path "rust/lithe-gpui" -Recurse -Filter *.rs |
  Where-Object { $_.FullName -notmatch '\\target\\' }

function Section($title) { Write-Output ""; Write-Output "=== $title ===" }
"（纳入扫描的 GPUI 源码文件数：$($Sources.Count)）"

$dispatcher = "rust/lithe-core/src/runtime/dispatcher.rs"
$commandRs  = "rust/lithe-core/src/protocol/command.rs"

Section "A1. lithe-core dispatcher 里实现的 Debug 命令分支"
Select-String -Path $dispatcher -Pattern 'CoreCommand::Debug[A-Za-z]+ =>' |
  ForEach-Object { $_.LineNumber.ToString().PadLeft(5) + ": " + $_.Line.Trim() }

Section "A2. 每个 debug.* 命令名在 GPUI 的引用数"
$debugNames = Select-String -Path $commandRs -Pattern '"(debug\.[a-zA-Z]+)"' -AllMatches |
  ForEach-Object { $_.Matches | ForEach-Object { $_.Groups[1].Value } } | Sort-Object -Unique
foreach ($c in $debugNames) {
  $n = ($Sources | Select-String -Pattern ([regex]::Escape('"' + $c + '"')) | Measure-Object).Count
  "{0,-34} {1}" -f $c, $n
}

Section "B1. 每个 git.* 命令名在 GPUI 的引用数"
$gitNames = Select-String -Path $commandRs -Pattern '"(git\.[a-zA-Z]+)"' -AllMatches |
  ForEach-Object { $_.Matches | ForEach-Object { $_.Groups[1].Value } } | Sort-Object -Unique
foreach ($c in $gitNames) {
  $n = ($Sources | Select-String -Pattern ([regex]::Escape('"' + $c + '"')) | Measure-Object).Count
  "{0,-34} {1}" -f $c, $n
}

Section "B2. git crate 的 IntegrationOperation / integration_gate 消费者"
$Sources | Select-String -Pattern 'IntegrationOperation|integration_gate|integrationPreflight' |
  ForEach-Object { $_.Path.Replace((Get-Location).Path + "\", "") + ":" + $_.LineNumber + ": " + $_.Line.Trim() }

Section "C1. 台账 §2：MENUS 边界与条目计数"
$mb = "rust/lithe-gpui/crates/workbench/src/menu_bar.rs"
$lines = Get-Content $mb
$start = 594; $depth = 0; $end = -1
for ($i = $start; $i -lt $lines.Count; $i++) {
  $depth += ([regex]::Matches($lines[$i], '\[')).Count - ([regex]::Matches($lines[$i], '\]')).Count
  if ($depth -le 0 -and $i -gt $start) { $end = $i + 1; break }
}
"MENUS 数组边界        : 行 595 .. $end"
"NotWired (数组内)      : " + (Select-String -Path $mb -Pattern 'MenuItem::NotWired' | Where-Object { $_.LineNumber -ge 595 -and $_.LineNumber -le $end }).Count
"Action   (数组内)      : " + (Select-String -Path $mb -Pattern 'MenuItem::Action' | Where-Object { $_.LineNumber -ge 595 -and $_.LineNumber -le $end }).Count
"Action   (全文件)      : " + (Select-String -Path $mb -Pattern 'MenuItem::Action' | Measure-Object).Count
"NOT_WIRED_OUTSIDE_MENUS: 2（声明见 :172）"

Section "C2. 台账 §2.2 断言：Git 身份配置页「set_git_identity_host 生产代码零调用」"
$Sources | Select-String -Pattern 'set_git_identity_host' |
  ForEach-Object { $_.Path.Replace((Get-Location).Path + "\", "") + ":" + $_.LineNumber + ": " + $_.Line.Trim() }
"--- workspace.rs 首个 #[cfg(test)] 行号（判断调用点是否在生产代码里）---"
$ws = "rust/lithe-gpui/crates/workbench/src/workspace.rs"
$cfgTest = (Select-String -Path $ws -Pattern '^\s*#\[cfg\(test\)\]' | Select-Object -First 1).LineNumber
"first #[cfg(test)] at: $cfgTest ; total lines: " + (Get-Content $ws).Count

Section "C3. 台账 §2.2 断言：go_back / go_forward 是否仍标 dead_code"
Select-String -Path "rust/lithe-gpui/crates/editor/src/editor_view.rs" -Pattern 'allow\(dead_code\)' -Context 0,3 |
  ForEach-Object { $_.LineNumber.ToString().PadLeft(5) + ": " + $_.Line.Trim() + "  ->  " + (($_.Context.PostContext -join ' ').Trim()) }

Section "C4. 台账 §2.3：GPUI 关键词命中复核（只扫源码，排除 target/ 构建产物）"
$sources = Get-ChildItem -Path "rust/lithe-gpui" -Recurse -Filter *.rs |
  Where-Object { $_.FullName -notmatch '\\target\\' }
"（纳入扫描的源码文件数：$($sources.Count)）"
foreach ($kw in @('mybatis','viewer','local-history','run-actions','runActions','onboarding',
                  'logging','host-api','HostApi','community','Community','mcp','MCP',
                  'ai_chat','AiChat','notebook')) {
  $n = ($sources | Select-String -Pattern $kw -SimpleMatch | Measure-Object).Count
  "{0,-16} {1}" -f $kw, $n
}
