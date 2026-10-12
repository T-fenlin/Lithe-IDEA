# Step 2 取证脚本：lithe-ui 编辑器 widget 导出 vs lithe-gpui 宿主引用 差集
#
# 用法（仓库根目录）：
#   pwsh -NoProfile -File reports/gpui-arch-s2-probe.ps1
#
# 目的：把「接线缺口」量化成可清点的条数 —— 组件库有、宿主零引用的 widget 清单。
# 只读检索，不改仓库。

$ErrorActionPreference = 'Stop'

# 只扫源码：rust/lithe-gpui/target/ 下有构建期生成的 .rs（14 个），
# 混进来会让关键词命中数虚高（实测 "viewer" 就是被 target 里的 mime_guess 生成文件顶起来的）。
$Sources = Get-ChildItem -Path "rust/lithe-gpui" -Recurse -Filter *.rs |
  Where-Object { $_.FullName -notmatch '\\target\\' }

function Section($title) { Write-Output ""; Write-Output "=== $title ===" }
"（纳入扫描的源码文件数：$($Sources.Count)）"

$uiMod = "rust/lithe-ui/src/editor/mod.rs"

Section "A. lithe-ui 编辑器模块 pub use 导出清单（mod.rs）"
Select-String -Path $uiMod -Pattern '^pub use ' | ForEach-Object {
  $_.LineNumber.ToString().PadLeft(5) + ": " + $_.Line.Trim()
}

Section "B. 逐个 widget 名在 lithe-gpui 宿主的引用数"
# 只取编辑器相关 widget 名，避免 git 的 Reference / 通用词噪声
$widgets = @(
  'SymbolOutline','GoToSymbol','CallHierarchy','RenameInput','CodeActionMenu',
  'FindWidget','SignatureHelp','CompletionMenu','CompletionItem','ProblemsPanel',
  'Problem','HoverInfo','CodeLens','InlayHint','GhostText','DiffHunk','GitMark',
  'ReadOnlyBanner','VimMode','VimModeIndicator','ExtensionState','ExtensionsPanel',
  'KeybindingsEditor','Keybinding','SearchPanel','ReferencesPanel','SearchFilters',
  'SettingsEditor','TaskRunner','TaskState','ConsoleEntry','DebugConsole','OutputPanel',
  'ThemePreview','ProjectSwitcher','RecentProjects','WelcomePage','Symbol','SymbolKind',
  'LspStatus','LspState','Encoding','LineEnding','IndentSettings','LanguageMode',
  'BranchIndicator','NotificationBell','CursorShape','LineNumbers','FindOptions'
)
"{0,-22} {1,6}  {2}" -f 'widget', 'hits', 'first hit (path:line)'
foreach ($w in $widgets) {
  $hits = $Sources | Select-String -Pattern ("\b" + $w + "\b")
  $first = if ($hits) { $hits[0].Path.Replace((Get-Location).Path + "\", "") + ":" + $hits[0].LineNumber } else { "" }
  "{0,-22} {1,6}  {2}" -f $w, $hits.Count, $first
}

Section "C. crates/editor 的 9 个模块与职责（lib.rs 声明）"
Get-Content "rust/lithe-gpui/crates/editor/src/lib.rs" | Select-Object -First 16 |
  ForEach-Object -Begin { $i = 1 } -Process { $i.ToString().PadLeft(4) + ": " + $_; $i++ }

Section "D. menu_bar.rs 的 15 个 MISSING_* 分组定义与命中次数"
$mb = "rust/lithe-gpui/crates/workbench/src/menu_bar.rs"
$groups = Select-String -Path $mb -Pattern '^pub const (MISSING_[A-Z_]+): Missing = Missing \{' |
  ForEach-Object { $_.Matches[0].Groups[1].Value }
foreach ($g in $groups) {
  $n = (Select-String -Path $mb -Pattern ("\b" + $g + "\b") | Measure-Object).Count
  # 命中含定义行与 MISSING_GROUPS 汇总行各 1 次，故使用点 = 命中 - 2
  "{0,-34} hits={1,3}  使用点≈{2}" -f $g, $n, ($n - 2)
}

Section "E. MENUS 数组边界与数组内条目计数"
$lines = Get-Content $mb
$start = 594
$depth = 0; $end = -1
for ($i = $start; $i -lt $lines.Count; $i++) {
  $depth += ([regex]::Matches($lines[$i], '\[')).Count - ([regex]::Matches($lines[$i], '\]')).Count
  if ($depth -le 0 -and $i -gt $start) { $end = $i + 1; break }
}
"MENUS 数组: 行 595 .. $end"
$inArray = Select-String -Path $mb -Pattern 'MenuItem::NotWired' | Where-Object { $_.LineNumber -ge 595 -and $_.LineNumber -le $end }
"MenuItem::NotWired (数组内): " + $inArray.Count
$actIn = Select-String -Path $mb -Pattern 'MenuItem::Action' | Where-Object { $_.LineNumber -ge 595 -and $_.LineNumber -le $end }
"MenuItem::Action   (数组内): " + $actIn.Count
"MenuItem::Action   (全文件): " + (Select-String -Path $mb -Pattern 'MenuItem::Action' | Measure-Object).Count
"NOT_WIRED_OUTSIDE_MENUS 条目: " + ((Select-String -Path $mb -Pattern 'MISSING_[A-Z_]+,$' | Where-Object { $_.LineNumber -ge 172 -and $_.LineNumber -le 179 }).Count)

Section "F. editor crate 的 9 个模块文件大小"
Get-ChildItem "rust/lithe-gpui/crates/editor/src" -Filter *.rs |
  ForEach-Object { "{0,-20} {1,7:N1} KB" -f $_.Name, ($_.Length / 1024) }

Section "G. 关键缺口关键词在 lithe-gpui 的命中数"
foreach ($kw in @('outline','breadcrumb','Breadcrumb','EditorGroup','PaneGroup','splitEditor',
                  'SplitEditor','inlay','Inlay','foldingRange','documentSymbol','documentSymbols')) {
  $n = ($Sources | Select-String -Pattern $kw -SimpleMatch | Measure-Object).Count
  "{0,-18} {1}" -f $kw, $n
}
