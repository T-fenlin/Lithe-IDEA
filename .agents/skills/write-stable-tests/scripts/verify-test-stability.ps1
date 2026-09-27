[CmdletBinding()]
param(
    [string]$BaseRevision,
    [string]$HeadRevision = "HEAD",
    [switch]$All
)

# 闸门只剩 Rust 一条通道（Swift / TypeScript 规则随旧前端删除），所以没有 `-Platform`。
$ErrorActionPreference = "Stop"
$arguments = @((Join-Path $PSScriptRoot "verify-test-stability.mjs"))
if ($All) { $arguments += "--all" }
if (-not [string]::IsNullOrWhiteSpace($BaseRevision)) {
    $arguments += @("--base", $BaseRevision, "--head", $HeadRevision)
}

& node @arguments
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
