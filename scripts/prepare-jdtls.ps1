#!/usr/bin/env pwsh
# Prepares the JDT LS smoke-test root consumed by
# rust/lithe-core/src/lsp/interface/engine_real_jdt_tests.rs, which is gated by
# LITHE_JDTLS_SMOKE_ROOT / LITHE_JDTLS_SMOKE_JAVA / LITHE_JDTLS_SMOKE_PROJECT_JDK.
#
# It does NOT download anything: it verifies and lays out the payloads described by
# third_party/jdtls/manifest.json (JDT LS archive, java-debug VSIX, java-test VSIX,
# lombok jar), so a machine or CI job that already fetched them can run the smoke test.
# Fetch URLs and SHA256 values live in the manifest and are the single source of truth.
#
# Usage:
#   pwsh scripts/prepare-jdtls.ps1 -Root third_party/jdtls
#   pwsh scripts/prepare-jdtls.ps1 -Root third_party/jdtls -JavaHome D:\ProgramData\java\openjdk-21 `
#       -ProjectJdk D:\ProgramData\java\open-25 -Maven D:\ProgramData\maven\apache-maven-3.6.3\bin\mvn.cmd
#
# Exits non-zero with an actionable message when a payload is missing, instead of
# letting the smoke test fail later on a confusing assertion.

[CmdletBinding()]
param(
    [string]$Root = 'third_party/jdtls',
    [string]$JavaDebugVsix,
    [string]$JavaTestVsix,
    [string]$JavaHome,
    [string]$ProjectJdk,
    [string]$Maven
)

$ErrorActionPreference = 'Stop'

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

function Write-JavaTestExtensionList([string]$JavaTestDirectory) {
    $extensions = Join-Path $JavaTestDirectory 'extensions'
    if (-not (Test-Path $extensions)) { return $false }
    $names = Get-ChildItem $extensions -Filter '*.jar' | Sort-Object Name | ForEach-Object { $_.Name }
    if ($names.Count -eq 0) { return $false }
    $list = Join-Path $JavaTestDirectory 'extensions.txt'
    Set-Content -Path $list -Value ($names -join "`n") -Encoding utf8
    Write-Host "java-test extensions.txt: $($names.Count) jar(s)"
    return $true
}

function Expand-VsixInto([string]$Vsix, [string]$Destination, [string]$JarFilter, [string]$TargetDirectory) {
    if (-not (Test-Path $Vsix)) { Fail "payload missing: $Vsix" }
    $expanded = "$Destination"
    if (Test-Path $expanded) { Remove-Item -Recurse -Force $expanded }
    Expand-Archive -Path $Vsix -DestinationPath $expanded -Force
    New-Item -ItemType Directory -Force $TargetDirectory | Out-Null
    $jars = Get-ChildItem $expanded -Recurse -Filter $JarFilter
    if ($jars.Count -eq 0) { Fail "no $JarFilter inside $Vsix" }
    foreach ($jar in $jars) {
        Copy-Item $jar.FullName $TargetDirectory -Force
        Write-Host "placed $($jar.Name)"
    }
}

if (-not (Test-Path (Join-Path $Root 'manifest.json'))) {
    Fail "manifest not found at $Root/manifest.json (third_party/jdtls is the expected root)"
}

$launcher = Get-ChildItem (Join-Path $Root 'plugins') -Filter 'org.eclipse.equinox.launcher_*.jar' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $launcher) { Fail "JDT LS payload missing: $Root/plugins/org.eclipse.equinox.launcher_*.jar (fetch the archive named in the manifest)" }

$configuration = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'config_win' }
elseif ($IsMacOS) { 'config_mac' }
else { 'config_linux' }
if (-not (Test-Path (Join-Path $Root $configuration))) { Fail "JDT LS payload missing: $Root/$configuration" }

if ($JavaDebugVsix) {
    Expand-VsixInto -Vsix $JavaDebugVsix -Destination (Join-Path $Root '.java-debug-vsix') `
        -JarFilter 'com.microsoft.java.debug.plugin-*.jar' -TargetDirectory (Join-Path $Root 'java-debug')
}
$debugBundle = Get-ChildItem (Join-Path $Root 'java-debug') -Filter '*.jar' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $debugBundle) { Fail "java-debug payload missing in $Root/java-debug (fetch the vscjava.vscode-java-debug VSIX named in the manifest)" }
Write-Host "java-debug bundle: $($debugBundle.Name)"

$lombok = Join-Path $Root 'lombok/lombok.jar'
if (-not (Test-Path $lombok)) { Fail "lombok payload missing: $lombok (fetch lombok.jar named in the manifest)" }
Write-Host "lombok: $((Get-Item $lombok).Length) bytes"

if ($JavaTestVsix) {
    Expand-VsixInto -Vsix $JavaTestVsix -Destination (Join-Path $Root '.java-test-vsix') `
        -JarFilter '*.jar' -TargetDirectory (Join-Path $Root 'java-test/extensions')
}
if (-not (Write-JavaTestExtensionList (Join-Path $Root 'java-test'))) {
    Fail "java-test payload missing in $Root/java-test/extensions (fetch the vscjava.vscode-java-test VSIX named in the manifest)"
}

Write-Host ''
Write-Host 'Smoke-test environment:'
Write-Host "  LITHE_JDTLS_SMOKE_ROOT=$((Resolve-Path $Root).Path)"
if ($JavaHome) { Write-Host "  LITHE_JDTLS_SMOKE_JAVA=$((Join-Path $JavaHome 'bin/java.exe'))" }
else { Write-Host '  LITHE_JDTLS_SMOKE_JAVA=<a JDK 21+ java executable that runs JDT LS>' }
if ($ProjectJdk) { Write-Host "  LITHE_JDTLS_SMOKE_PROJECT_JDK=$ProjectJdk" }
else { Write-Host '  LITHE_JDTLS_SMOKE_PROJECT_JDK=<a JDK 25 home for the java25 fixture project>' }
if ($Maven) { Write-Host "  LITHE_JDTLS_SMOKE_MAVEN=$Maven" }
Write-Host ''
Write-Host 'Then: cargo test --manifest-path rust/Cargo.toml -p lithe-core --lib real_jdtls -- --nocapture'
Write-Host 'Known gap: the fixture project created by that test still fails to build on some machines'
Write-Host '(javaBuildCompilationErrors with 2 markers); see'
Write-Host '.agents/notes/proposed/architecture/2026-10-04-gpui-breakpoints-and-java-debugging.md.'
