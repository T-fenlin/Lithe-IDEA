[CmdletBinding()]
param(
    # 干什么：build=只编译；test=跑测试；run=编译后启动；launch=直接启动已有产物。
    [ValidateSet("build", "test", "run", "launch")]
    [string]$Action = "build",

    # build/run 编哪个包；test 用 -TestPackage。
    [string]$Package = "lithe-gpui-app",

    # test 要跑的包（可重复传，例如 lithe-gpui-settings,lithe-gpui-shared）。
    [string[]]$TestPackage = @(),

    # 并行度。默认 2：内存紧张时 linker 会 LNK1102，见 docs/development/gpui-parity-plan-2026-10.md。
    [int]$Jobs = 2,

    # release 档构建（默认 debug，与 sign-windows.ps1 的默认目录一致）。
    [switch]$Release,

    # Cargo target 目录。默认取 <仓库父目录>/lithe-gpui-target：工作区被沙盒施加 ACL，
    # 产物必须落在工作区外，否则生成的可执行文件运行时被拒绝一切写入。
    [string]$TargetDirectory = "",

    # 关掉调试符号以降低内存与耗时（代价是断点调试无符号）。
    # 注意：这个开关会改变 cargo profile 指纹，与已有产物不兼容时触发一次全量重编。
    [switch]$Lean,

    # 传给被启动应用的额外参数（--open-settings / --menu-probe ... 等）。
    [string[]]$AppArgument = @()
)

# 统一 GPUI 前端的本地编译 / 测试 / 启动入口。
#
# 这个脚本存在的唯一理由：把「怎么编」从每个会话的记忆里挪进仓库。此前每次都要
# 重新试出同一套配方（外部 CARGO_TARGET_DIR、-j 2、RUST_MIN_STACK、
# CARGO_PROFILE_DEV_DEBUG、LITHE_JDTLS_ROOT），试错成本全部浪费在重复劳动上。
#
# 配方依据（不是这里发明的）：
#   - docs/development/gpui-parity-plan-2026-10.md:20-34  「构建必须绕开工作区」
#   - docs/development/gpui-platform-gap-and-schedule-2026-10.md:240-249  内存配方
#   - docs/ci-builds.md:226-233  CI 的等价命令（--locked + 显式 --target）
#
# 与 CI 的差别是刻意的：本地用外部 target 目录、不带 --locked（允许更新 lock），
# 且默认关调试符号换构建速度。要完全对齐 CI，加 -Release 并自行带 --locked。

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "rust/lithe-gpui/Cargo.toml"
if (-not (Test-Path -LiteralPath $manifest)) {
    throw "找不到 GPUI workspace 清单：$manifest"
}

if ([string]::IsNullOrWhiteSpace($TargetDirectory)) {
    if (-not [string]::IsNullOrWhiteSpace($env:LITHE_GPUI_TARGET_DIR)) {
        $TargetDirectory = $env:LITHE_GPUI_TARGET_DIR
    } else {
        $TargetDirectory = Join-Path (Split-Path -Parent $root) "lithe-gpui-target"
    }
}
$TargetDirectory = [System.IO.Path]::GetFullPath($TargetDirectory)

# 产物落在工作区内会被沙盒 ACL 拒绝写入（可执行文件运行时被拒绝一切写入），直接拦住。
$rootPrefix = $root.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if ($TargetDirectory.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Cargo target 目录必须在工作区之外（当前：$TargetDirectory）。用 -TargetDirectory 指定工作区外的路径。"
}

$profileName = if ($Release) { "release" } else { "debug" }
$env:CARGO_TARGET_DIR = $TargetDirectory
$env:RUST_MIN_STACK = "67108864"
# 默认**不动** CARGO_PROFILE_DEV_DEBUG：改它会换掉 profile 指纹，把已有增量产物全部作废。
# 内存吃紧时再加 -Lean（等价于文档里的 CARGO_PROFILE_DEV_DEBUG=0）。
if ($Lean) {
    $env:CARGO_PROFILE_DEV_DEBUG = "0"
}

Write-Host "build-gpui action=$Action package=$Package target=$TargetDirectory profile=$profileName jobs=$Jobs lean=$Lean"

# cargo 的 `--manifest-path` 必须跟在子命令之后（与 CI 的写法一致）。
function Invoke-Cargo {
    param([string]$Subcommand, [string[]]$CargoArgument)
    $argumentList = @($Subcommand, "--manifest-path", $manifest) + $CargoArgument
    Write-Host ("cargo " + ($argumentList -join " "))
    & cargo @argumentList
    if ($LASTEXITCODE -ne 0) { throw "cargo 退出码 $LASTEXITCODE" }
}

$binaryName = "Lithe.exe"
$binaryDirectory = Join-Path $TargetDirectory $profileName
$binaryPath = Join-Path $binaryDirectory $binaryName

# 正在运行的应用会锁住 exe，cargo 只会报一句「拒绝访问。(os error 5)」，看不出是谁占着。
# 与其让下一个人重新试一遍，不如在这里直接说清楚。
if ($Action -ne "launch") {
    $running = @(Get-Process -Name ([System.IO.Path]::GetFileNameWithoutExtension($binaryName)) -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and [System.IO.Path]::GetFullPath($_.Path) -ieq $binaryPath })
    if ($running.Count -gt 0) {
        throw "构建目标正被运行中的实例占用（PID $($running.Id -join ", ")）：$binaryPath`n先关掉它，或用 -Action launch 复用已有产物。"
    }
}

switch ($Action) {
    "build" {
        $profileArgument = if ($Release) { @("--release") } else { @() }
        Invoke-Cargo "build" (@("-p", $Package, "-j", "$Jobs") + $profileArgument)
        Write-Host "产物：$binaryPath"
    }
    "test" {
        $packages = if ($TestPackage.Count -gt 0) { $TestPackage } else { @($Package) }
        foreach ($name in $packages) {
            Invoke-Cargo "test" @("-p", $name, "--lib", "-j", "$Jobs")
        }
    }
    "run" {
        $profileArgument = if ($Release) { @("--release") } else { @() }
        Invoke-Cargo "build" (@("-p", $Package, "-j", "$Jobs") + $profileArgument)
    }
    "launch" { }
}

if ($Action -in @("run", "launch")) {
    if (-not (Test-Path -LiteralPath $binaryPath)) {
        throw "找不到可执行文件：$binaryPath（先跑 -Action build）"
    }
    # 开发期 exe 在 target 目录下，向上找 12 层也够不到仓库根的 .artifacts/jdtls
    # （jdtls.rs 的 bundled_roots/search_roots），所以必须显式指路；正式打包后
    # LanguageServers/jdtls 就在 exe 旁边，不需要这个变量。
    $jdtlsRoot = Join-Path $root ".artifacts/jdtls"
    if (Test-Path -LiteralPath $jdtlsRoot) {
        $env:LITHE_JDTLS_ROOT = $jdtlsRoot
        Write-Host "LITHE_JDTLS_ROOT=$jdtlsRoot"
    } else {
        Write-Warning "没有 $jdtlsRoot：Java 语义与调试会退化成内置回退。先跑 ./scripts/prepare-jdtls.ps1"
    }

    $launchArgument = @()
    if ($AppArgument.Count -gt 0) { $launchArgument = $AppArgument } else { $launchArgument = @($root) }
    Write-Host ("启动 " + $binaryPath + " " + ($launchArgument -join " "))
    Push-Location $binaryDirectory
    try {
        & $binaryPath @launchArgument
    } finally {
        Pop-Location
    }
}