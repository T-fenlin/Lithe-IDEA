# 复现 scripts/classify-ci-changes.sh 的 glob 分类逻辑（PowerShell 版）。
#
# 为什么需要它：本工作区 bash 不可用（`bash -c "echo OK"` 返回乱码且 exit=1），
# 无法直接运行分类器。这里按 PowerShell 的 -like 通配符重列 case 分支的判定，
# 用来对「给定改动路径会命中哪条分支」做可复跑的独立核对。
#
# 口径：只覆盖与 Rust 路径判定相关的分支（rust/* 兜底及其前面的特例）。
# 完整 13 个开关的语义以 scripts/classify-ci-changes.sh 为准。

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string[]]$Path
)

# 与 classify-ci-changes.sh 中 rust 相关 case 分支一一对应（大小写不敏感，脚本里做了 ,, 小写化）
$rustBranches = [ordered]@{
    'rust/lithe-core/src/*.rs'                     = @{ rust_core = $true; macos_release = $true; windows = $true; windows_rust = $true }
    'rust/lithe-core/tests/*'                      = @{ rust_core = $true }
    'rust/lithe-core/cargo.toml'                   = @{ rust_core = $true; macos_release = $true; windows = $true; windows_rust = $true }
    'rust/lithe-core/include/*'                    = @{ rust_core = $true; macos_release = $true; windows = $true; windows_rust = $true }
    'rust/cargo.toml'                              = @{ rust_core = $true; rust_database = $true; macos_release = $true; windows = $true; windows_rust = $true }
    'rust/cargo.lock'                              = @{ rust_core = $true; rust_database = $true; macos_release = $true; windows = $true; windows_rust = $true }
    'rust/lithe-db-mcp/*'                          = @{ rust_database = $true; macos_release = $true }
    'rust/lithe-db-sidecar/*'                      = @{ rust_database = $true; macos_release = $true }
    # 兜底：任何未在上面命中的 rust/ 路径
    'rust/*'                                       = @{ rust_core = $true; rust_database = $true; macos_release = $true; windows = $true; windows_rust = $true }
}

$flags = [ordered]@{
    rust_core = $false; rust_database = $false; macos_release = $false
    windows = $false; windows_rust = $false
}
$claimed = @{}

foreach ($raw in $Path) {
    # 脚本对路径做了 .lower()（bash 的 ,,）后再匹配
    $p = $raw.ToLowerInvariant()
    $hit = $null
    foreach ($pattern in $rustBranches.Keys) {
        if ($p -like $pattern) { $hit = $pattern; break }
    }
    if ($hit) {
        foreach ($k in $rustBranches[$hit].Keys) { $flags[$k] = $true }
        Write-Output ("{0,-55} -> {1}" -f $p, $hit)
    } else {
        Write-Output ("{0,-55} -> (未命中任何 rust 分支)" -f $p)
    }
}

Write-Output ""
Write-Output "分类结果："
foreach ($k in $flags.Keys) { Write-Output ("  {0}={1}" -f $k, $flags[$k].ToString().ToLowerInvariant()) }
