<#
.SYNOPSIS
    捕获 GPUI 窗口（或整屏）的截图，作为视觉证据。

.DESCRIPTION
    默认只捕获目标进程的主窗口（不把桌面上的其他内容录进证据）；`-WholeScreen` 时抓整个
    虚拟屏幕，用来验证窗口在屏幕上的大小与位置。

    ⚠️ **本进程必须先声明 DPI 感知**，否则在 125% 缩放的机器上会踩两个坑（本项目实测）：
    - `GetWindowRect` 返回的是被系统虚拟化后的尺寸（1536×864 而不是真实的 1920×1080 区域）；
    - `PrintWindow` 抓到的位图只有虚拟化尺寸，**真实画面会被裁掉一大截**，
      看起来就像"内容没铺满 / 状态栏被挤出去"，其实是截图工具的问题。
    因此这里在创建任何窗口之前先调 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`。

.EXAMPLE
    pwsh -File capture-screenshot.ps1 -OutputPath ..\.artifacts\p1\window.png -ProcessName Lithe
    pwsh -File capture-screenshot.ps1 -OutputPath ..\.artifacts\p1\screen.png -ProcessName Lithe -WholeScreen
#>
param(
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [string]$ProcessName = "Lithe",
    [int]$TimeoutSeconds = 20,
    # 抓整个虚拟屏幕（验证窗口在屏幕上的大小/位置），而不是只抓窗口。
    [switch]$WholeScreen
)

$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class DpiAwareness {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
    // PER_MONITOR_AWARE_V2 = -4
    public static bool EnablePerMonitorV2() {
        try { return SetProcessDpiAwarenessContext(new IntPtr(-4)); } catch { return false; }
    }
}
"@

# 必须在任何窗口/GDI 调用之前设置；失败不致命，但要在输出里说出来。
$dpiAware = [DpiAwareness]::EnablePerMonitorV2()

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class WindowCapture {
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

function Find-WindowHandle([string]$name, [int]$timeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $process = Get-Process -Name $name -ErrorAction SilentlyContinue |
            Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
        if ($process) { return $process.MainWindowHandle }
        Start-Sleep -Milliseconds 250
    }
    return [IntPtr]::Zero
}

$handle = Find-WindowHandle $ProcessName $TimeoutSeconds
if ($handle -eq [IntPtr]::Zero) {
    throw "在 $TimeoutSeconds 秒内没有找到 $ProcessName 的可见窗口"
}

[void][WindowCapture]::SetForegroundWindow($handle)
Start-Sleep -Milliseconds 500

$directory = Split-Path -Parent $OutputPath
if ($directory -and -not (Test-Path $directory)) {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
}

if ($WholeScreen) {
    # 整屏：抓虚拟屏幕，用来核对窗口占屏幕的比例与位置。
    $left = [DpiAwareness]::GetSystemMetrics(76)   # SM_XVIRTUALSCREEN
    $top = [DpiAwareness]::GetSystemMetrics(77)    # SM_YVIRTUALSCREEN
    $width = [DpiAwareness]::GetSystemMetrics(78)  # SM_CXVIRTUALSCREEN
    $height = [DpiAwareness]::GetSystemMetrics(79) # SM_CYVIRTUALSCREEN
    if ($width -le 0 -or $height -le 0) { throw "拿不到虚拟屏幕尺寸" }

    $bitmap = New-Object System.Drawing.Bitmap $width, $height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($left, $top, 0, 0, (New-Object System.Drawing.Size $width, $height))
    } finally {
        $graphics.Dispose()
    }
    $bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    Write-Output "已保存整屏截图：$OutputPath (${width}x${height}, dpiAware=$dpiAware)"
    exit 0
}

$rect = New-Object WindowCapture+RECT
if (-not [WindowCapture]::GetWindowRect($handle, [ref]$rect)) {
    throw "GetWindowRect 失败，拿不到窗口尺寸"
}
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) { throw "窗口尺寸非法：${width}x${height}" }

$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$hdc = $graphics.GetHdc()
try {
    # flags = 2（PW_RENDERFULLCONTENT）：抓取包括 GPU 合成内容在内的完整窗口。
    if (-not [WindowCapture]::PrintWindow($handle, $hdc, 2)) {
        throw "PrintWindow 失败"
    }
} finally {
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
}

$bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()

Write-Output "已保存窗口截图：$OutputPath (${width}x${height}, dpiAware=$dpiAware)"
