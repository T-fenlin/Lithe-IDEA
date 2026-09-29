# Builds the Lithe host binary (the GPUI Kit application) from the rust/ workspace.
#
# Usage:
#   ./scripts/build-lithe.ps1                                 # debug build
#   ./scripts/build-lithe.ps1 -Configuration Release          # release build
#   ./scripts/build-lithe.ps1 -Run                            # build, then launch the binary
#   ./scripts/build-lithe.ps1 -Jobs 4 -CargoArguments --locked
#
# Why TEMP/TMP are redirected into the repository:
#   MSVC's cl.exe writes its own scratch files (the "debug record") through %TEMP%
#   before it can start its front end c1.dll. When the process tree that runs cargo
#   cannot write to the system %TEMP% (restricted sandbox, locked-down terminal, ...),
#   cl.exe exits with code 2 and prints no source diagnostic at all:
#       cl: Command line error D8050 : cannot execute '...\c1.dll' ...
#   which cargo reports as "failed to run custom build command for `ring`".
#   Pointing TEMP/TMP at .artifacts/cargo-tmp (already git-ignored) avoids that.
#   Pass -KeepHostTemp to keep the inherited system temp directory instead.

# PositionalBinding is off so that bare tokens such as "--locked" reach $CargoArguments
# instead of being bound to the first declared parameter.
[CmdletBinding(PositionalBinding = $false)]
param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug",

    # Parallelism for cargo -j. 0 keeps cargo's default.
    [int]$Jobs = 0,

    # Launch the built binary once the build succeeds.
    [switch]$Run,

    # Run `cargo clean` first. Wipes rust/target and forces a full rebuild (slow).
    [switch]$Clean,

    # Keep the inherited TEMP/TMP instead of using .artifacts/cargo-tmp.
    [switch]$KeepHostTemp,

    # Extra arguments forwarded to cargo, for example: -CargoArguments --locked
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArguments
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "rust/Cargo.toml"
if (-not (Test-Path -LiteralPath $manifest)) {
    throw "Workspace manifest not found: $manifest (this script must stay in the repository's scripts/ directory)."
}

if (-not $KeepHostTemp) {
    $scratch = Join-Path $root ".artifacts/cargo-tmp"
    New-Item -ItemType Directory -Force -Path $scratch | Out-Null
    $env:TEMP = $scratch
    $env:TMP = $scratch
    Write-Host "TEMP/TMP -> $scratch  (pass -KeepHostTemp to keep the system temp directory)"
}

if ($Clean) {
    Write-Host "cargo clean --manifest-path $manifest"
    & cargo clean --manifest-path $manifest
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# Careful: PowerShell variable names are case-insensitive, so this must not be called
# $cargoArguments - that would alias the $CargoArguments parameter above.
$cargoInvocation = @("build", "--manifest-path", $manifest, "--bin", "Lithe")
if ($Configuration -eq "Release") { $cargoInvocation += "--release" }
if ($Jobs -gt 0) { $cargoInvocation += @("-j", "$Jobs") }
if ($CargoArguments) { $cargoInvocation += $CargoArguments }

Write-Host ("cargo " + ($cargoInvocation -join " "))

# Native stderr is merged below so the failure hint can inspect it; that must not
# trip $ErrorActionPreference = "Stop", so relax it for the build only.
$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& cargo @cargoInvocation 2>&1 | Tee-Object -Variable cargoOutput | Out-Host
$exitCode = $LASTEXITCODE
$ErrorActionPreference = $previousErrorActionPreference

if ($exitCode -ne 0) {
    $cargoText = $cargoOutput | Out-String
    if (($cargoText -match "D8050") -or ($cargoText -match "c1\.dll")) {
        Write-Warning ("cl.exe could not start c1.dll (D8050): the process tree running cargo cannot write to " +
            "its TEMP/TMP. Re-run without -KeepHostTemp, or point TEMP/TMP at a writable directory.")
    }
    exit $exitCode
}

$profileDirectory = if ($Configuration -eq "Release") { "release" } else { "debug" }
$binaryName = if ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT) { "Lithe.exe" } else { "Lithe" }
$binary = Join-Path $root ("rust/target/" + $profileDirectory + "/" + $binaryName)

if (-not (Test-Path -LiteralPath $binary)) {
    Write-Warning "cargo succeeded but the expected binary is missing: $binary"
    exit 0
}

$built = Get-Item -LiteralPath $binary
Write-Host ""
Write-Host ("Built " + $Configuration + " binary: " + $built.FullName)
Write-Host ("  " + [Math]::Round($built.Length / 1MB, 2) + " MB, " + $built.LastWriteTime)

if ($Run) {
    $started = Start-Process -FilePath $built.FullName -PassThru
    Write-Host ("  launched PID " + $started.Id)
}

exit 0
