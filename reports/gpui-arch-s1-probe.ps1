# Step 1 取证脚本：lithe-core LSP 方法面穷尽核对
#
# 用法（仓库根目录）：
#   pwsh -NoProfile -File reports/gpui-arch-s1-probe.ps1
#
# 目的：判定 7 个 MISSING_LSP_REQUESTS 入口各自是「Core 有命令」还是「Core 缺命令」。
# 判据来自四处交叉：
#   A. engine.rs 的 LspSemanticOperation 枚举（有哪些操作变体）
#   B. engine.rs 的 semantic_method 映射（变体 → LSP method）
#   C. client.rs 的 validate_lsp_method 白名单（哪些 method 放行）
#   D. client.rs 的 feature_request_params（参数怎么构造，是否带 range）
# 只读检索，不改仓库。

$ErrorActionPreference = 'Stop'
$root = (Get-Location).Path

# 只扫源码：rust/lithe-core 当前没有 target/ 目录，但脚本要在任何检出状态下都能给对读数，
# 所以显式排除构建产物。
$CoreSources = Get-ChildItem -Path "rust/lithe-core" -Recurse -Filter *.rs |
  Where-Object { $_.FullName -notmatch '\\target\\' }

function Section($title) { Write-Output ""; Write-Output "=== $title ===" }
"（lithe-core 纳入扫描的源码文件数：$($CoreSources.Count)）"

Section "A. LspSemanticOperation 变体清单（engine.rs:276 起）"
$engine = "rust/lithe-core/src/lsp/interface/engine.rs"
Select-String -Path $engine -Pattern '^\s{4}([A-Z][A-Za-z]+),$' |
  ForEach-Object { $_.LineNumber.ToString().PadLeft(5) + ": " + $_.Line.Trim() }

Section "B. semantic_method 映射（engine.rs:4222 起）"
Select-String -Path $engine -Pattern 'LspSemanticOperation::[A-Za-z]+ => "' |
  ForEach-Object { $_.LineNumber.ToString().PadLeft(5) + ": " + $_.Line.Trim() }

Section "C. validate_lsp_method 白名单（client.rs:673 起）"
$client = "rust/lithe-core/src/lsp/interface/client.rs"
Select-String -Path $client -Pattern '^\s+\|? "(textDocument|completionItem|codeAction|java|workspace)/[A-Za-z/]+"' |
  ForEach-Object { $_.LineNumber.ToString().PadLeft(5) + ": " + $_.Line.Trim() }

Section "D. formatting 的参数构造（client.rs:723 起，看是否带 range）"
Get-Content $client | Select-Object -Skip 722 -First 11 |
  ForEach-Object -Begin { $i = 723 } -Process { $i.ToString().PadLeft(5) + ": " + $_; $i++ }

Section "E. 关键词命中数（lithe-core 源码，排除 target/）"
foreach ($kw in @('signatureHelp', 'SignatureHelp', 'signature_help',
                  'rangeFormatting', 'RangeFormatting', 'range_formatting',
                  'documentSymbol', 'DocumentSymbol',
                  'prepareRename', 'PrepareRename',
                  'workspace/symbol')) {
  $n = ($CoreSources | Select-String -Pattern $kw -SimpleMatch | Measure-Object).Count
  "{0,-22} {1}" -f $kw, $n
}

Section "F. GPUI 侧 crates/java 实际发出的 operation 名（service.rs）"
Select-String -Path "rust/lithe-gpui/crates/java/src/service.rs" -Pattern 'session\.request\(\s*$' -Context 0,2 |
  ForEach-Object { $_.LineNumber.ToString().PadLeft(5) + ": " + ($_.Context.PostContext -join ' ').Trim() }

Section "G. MISSING_LSP_REQUESTS 的 7 个使用点（menu_bar.rs）"
Select-String -Path "rust/lithe-gpui/crates/workbench/src/menu_bar.rs" -Pattern 'MISSING_LSP_REQUESTS' -Context 2,0 |
  Where-Object { $_.LineNumber -gt 200 } |
  ForEach-Object {
    $label = ($_.Context.PreContext | Where-Object { $_ -match 'label_key' }) -join ''
    $_.LineNumber.ToString().PadLeft(5) + ": " + $label.Trim()
  }
