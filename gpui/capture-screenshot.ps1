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
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd, System.Text.StringBuilder name, int max);
    public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lParam);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    /// 该进程里**客户区最大**的可见顶层窗口。
    ///
    /// 为什么不用 `Process.MainWindowHandle`：`Lithe.exe` 是控制台子系统程序，独立控制台
    /// 启动时 `MainWindowHandle` 会指到那个黑底控制台窗口上（实测抓到 1239x647 的控制台，
    /// 而不是 1823x1024 的 GPUI 窗口，看起来就像"界面没渲染"）。控制台窗口按类名排掉。
    public static IntPtr FindLargestWindow(uint processId) {
        IntPtr best = IntPtr.Zero;
        long bestArea = 0;
        EnumWindows((hwnd, _) => {
            uint pid;
            GetWindowThreadProcessId(hwnd, out pid);
            if (pid != processId || !IsWindowVisible(hwnd)) return true;
            var name = new System.Text.StringBuilder(256);
            GetClassName(hwnd, name, name.Capacity);
            string cls = name.ToString();
            if (cls == "ConsoleWindowClass" || cls == "PseudoConsoleWindow") return true;
            RECT rect;
            if (!GetClientRect(hwnd, out rect)) return true;
            long area = (long)(rect.Right - rect.Left) * (rect.Bottom - rect.Top);
            if (area > bestArea) { bestArea = area; best = hwnd; }
            return true;   // ⚠️ 这个 lambda 必须每条路径都返回值，漏掉会让 Add-Type 直接编译失败
        }, IntPtr.Zero);
        return best;
    }

    public static string ClassNameOf(IntPtr hwnd) {
        var name = new System.Text.StringBuilder(256);
        GetClassName(hwnd, name, name.Capacity);
        return name.ToString();
    }
}
"@

function Find-WindowHandle([string]$name, [int]$timeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        foreach ($process in @(Get-Process -Name $name -ErrorAction SilentlyContinue)) {
            $found = [WindowCapture]::FindLargestWindow([uint32]$process.Id)
            if ($found -ne [IntPtr]::Zero) {
                # ⚠️ 必须 Write-Host：Write-Output 会混进本函数的返回值，调用方拿到的就是
                # "字符串 + IntPtr" 的数组，传给 SetForegroundWindow 会报 Object[] 无法转换。
                Write-Host ("选中窗口：pid={0} class={1} hwnd={2}" -f $process.Id, [WindowCapture]::ClassNameOf($found), $found)
                return $found
            }
        }
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
# 打印窗口矩形原点：调用方（点击/坐标换算脚本）用它把"截图坐标"换算成"客户区坐标"，
# 因为截图是窗口矩形，而点击注入用的是客户区坐标，两者差一个边框内缩量。
Write-Host ("窗口矩形：left={0} top={1} {2}x{3}" -f $rect.Left, $rect.Top, $width, $height)

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
