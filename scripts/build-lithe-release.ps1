# Release build entry point (the official binary). This build uses lto plus a single
# codegen unit, so expect a long compile.
# Equivalent to: ./scripts/build-lithe.ps1 -Configuration Release
# Switches -Run/-Clean/-KeepHostTemp/-Jobs are forwarded, and any extra argument is
# forwarded to cargo, for example: ./scripts/build-lithe-release.ps1 --locked

[CmdletBinding(PositionalBinding = $false)]
param(
    [int]$Jobs = 0,
    [switch]$Run,
    [switch]$Clean,
    [switch]$KeepHostTemp,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArguments
)

$ErrorActionPreference = "Stop"

# A hashtable splat binds by name; an array splat would bind positionally.
$forwarded = @{ Configuration = "Release" }
if ($Jobs -gt 0) { $forwarded.Jobs = $Jobs }
if ($Run) { $forwarded.Run = $true }
if ($Clean) { $forwarded.Clean = $true }
if ($KeepHostTemp) { $forwarded.KeepHostTemp = $true }

$script = Join-Path $PSScriptRoot "build-lithe.ps1"
if ($CargoArguments) {
    & $script @forwarded @CargoArguments
} else {
    & $script @forwarded
}
exit $LASTEXITCODE
