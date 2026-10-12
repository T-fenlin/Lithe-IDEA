[CmdletBinding()]
param(
    [string]$BinaryDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) ".." | Join-Path -ChildPath "lithe-gpui-target/debug"),
    [string]$BinaryName = "Lithe.exe",
    [string]$CertificateThumbprint = $env:LITHE_WINDOWS_CERTIFICATE_THUMBPRINT,
    [string]$TimestampServer = $env:LITHE_WINDOWS_TIMESTAMP_SERVER,
    [switch]$AllowSelfSigned,
    [switch]$RequireSignature
)

# Signs the unified GPUI front-end binary (crates/app -> Lithe.exe) and the DLLs
# that sit next to it. The legacy Tauri product has its own path inside
# package-windows.ps1; this script exists because rust/lithe-gpui ships straight
# out of cargo and previously had no signing step at all.

$ErrorActionPreference = "Stop"

function Resolve-SignTool {
    $candidates = @()
    $kitsRoot = "C:\Program Files (x86)\Windows Kits\10\bin"
    if (Test-Path -LiteralPath $kitsRoot) {
        $candidates += Get-ChildItem -LiteralPath $kitsRoot -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^\d' } |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName "x64\signtool.exe" }
    }
    $candidates += "C:\Program Files (x86)\Windows Kits\10\App Certification Kit\signtool.exe"
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    throw "signtool.exe was not found. Install the Windows SDK (Signing Tools)."
}

function Resolve-Certificate {
    param([string]$Thumbprint, [switch]$SelfSignedAllowed)

    if (-not [string]::IsNullOrWhiteSpace($Thumbprint)) {
        $found = Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -CodeSigningCert -ErrorAction SilentlyContinue |
            Where-Object { $_.Thumbprint -ieq $Thumbprint } |
            Select-Object -First 1
        if ($null -eq $found) { throw "Configured certificate thumbprint was not found: $Thumbprint" }
        return $found
    }

    if ($SelfSignedAllowed) {
        # Local development: reuse the certificate created by New-SelfSignedCertificate
        # for CN=Lithe Development (Local Build). It is trusted on this machine only.
        $found = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert -ErrorAction SilentlyContinue |
            Where-Object { $_.Subject -like "*Lithe Development*" } |
            Sort-Object NotAfter -Descending |
            Select-Object -First 1
        if ($null -eq $found) {
            throw "No local development certificate found. Create one with New-SelfSignedCertificate -Type CodeSigningCert."
        }
        Write-Output "Using local development certificate: $($found.Thumbprint)"
        return $found
    }

    throw "No certificate configured. Pass -CertificateThumbprint (or -AllowSelfSigned for local builds)."
}

$directory = [System.IO.Path]::GetFullPath($BinaryDirectory)
if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "Binary directory does not exist: $directory"
}

$signtool = Resolve-SignTool
Write-Output "signtool : $signtool"
Write-Output "directory: $directory"

$certificate = Resolve-Certificate -Thumbprint $CertificateThumbprint -SelfSignedAllowed:$AllowSelfSigned
$thumbprint = $certificate.Thumbprint

# The executable first, then every DLL beside it. Signing only the .exe leaves the
# side-by-side assemblies unsigned, which is enough to re-trigger warnings.
$targets = @()
$exe = Join-Path $directory $BinaryName
if (Test-Path -LiteralPath $exe) { $targets += $exe }
$targets += Get-ChildItem -LiteralPath $directory -Filter *.dll -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName }

if ($targets.Count -eq 0) {
    $message = "No binaries to sign under $directory (expected $BinaryName)."
    if ($RequireSignature) { throw $message }
    Write-Output $message
    return
}

Write-Output "Signing $($targets.Count) file(s) with certificate $thumbprint"

foreach ($target in $targets) {
    $arguments = @("sign", "/fd", "SHA256", "/sha1", $thumbprint)
    if (-not [string]::IsNullOrWhiteSpace($TimestampServer)) {
        $arguments += @("/tr", $TimestampServer, "/td", "SHA256")
    }
    $arguments += $target

    & $signtool $arguments 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) {
        throw "signtool failed for $target (exit $LASTEXITCODE)."
    }
}

$failures = @()
foreach ($target in $targets) {
    & $signtool verify /pa $target 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { $failures += $target }
}

if ($failures.Count -gt 0) {
    throw "Signature verification failed for: $($failures -join ', ')"
}

Write-Output "All $($targets.Count) file(s) signed and verified."
$directory
