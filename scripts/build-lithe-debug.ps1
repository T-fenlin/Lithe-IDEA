# Debug build entry point.
# Equivalent to: ./scripts/build-lithe.ps1 -Configuration Debug
# Switches -Run/-Clean/-KeepHostTemp/-Jobs are forwarded, and any extra argument is
# forwarded to cargo, for example: ./scripts/build-lithe-debug.ps1 -Run --locked

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
$forwarded = @{ Configuration = "Debug" }
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
