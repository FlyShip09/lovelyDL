# Performance numbers for PROGRESS.md: cold start to a visible window, idle CPU, memory.
#
#   powershell -File scripts/measure.ps1 -Exe target\release\lovelyDL.exe -AppHome .dev\home-perf
#   powershell -File scripts/measure.ps1 -Sample 20      # sample an already running app
#
# "Cold start" ends when the window is visible, which the page asks for only after its first
# paint. CPU and memory add up lovelyDL and the WebView2 processes it started.
param(
    [string]$Exe = "",
    [string]$AppHome = "",
    [int]$Sample = 30,
    [switch]$KeepRunning
)
$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Wnd {
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc proc, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    public static bool Visible(string title) {
        bool found = false;
        EnumWindows((h, l) => {
            var sb = new StringBuilder(256);
            GetWindowText(h, sb, 256);
            if (IsWindowVisible(h) && sb.ToString() == title) { found = true; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

function AppProcesses($since) {
    $app = Get-Process lovelydl -ErrorAction SilentlyContinue
    $views = Get-Process msedgewebview2 -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -ge $since.AddSeconds(-1) }
    @($app) + @($views) | Where-Object { $_ }
}

if ($Exe) {
    if ($AppHome) { $env:LOVELYDL_HOME = (Resolve-Path $AppHome).Path }
    $since = Get-Date
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process $Exe -PassThru
    while (-not [Wnd]::Visible("lovelyDL") -and $watch.ElapsedMilliseconds -lt 20000) { Start-Sleep -Milliseconds 5 }
    Write-Output ("cold start to visible window: {0} ms" -f $watch.ElapsedMilliseconds)
    Start-Sleep -Seconds 5
} else {
    $proc = Get-Process lovelydl | Select-Object -First 1
    $since = $proc.StartTime
}

$cores = [Environment]::ProcessorCount
# Process properties are read live, so CPU times are summed now, not after the sleep.
function CpuMs($procs) { $sum = 0; foreach ($p in $procs) { $sum += $p.TotalProcessorTime.TotalMilliseconds }; $sum }
function Tools($since) { Get-Process yt-dlp, ffmpeg, ffprobe, deno -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -ge $since } }
$cpuBefore = CpuMs (AppProcesses $since)
$toolsBefore = @{}
foreach ($p in Tools $since) { $toolsBefore[$p.Id] = $p.TotalProcessorTime.TotalMilliseconds }
$t0 = Get-Date
Start-Sleep -Seconds $Sample
$after = AppProcesses $since
$cpuAfter = CpuMs $after
$wall = ((Get-Date) - $t0).TotalMilliseconds
$percent = 100 * ($cpuAfter - $cpuBefore) / ($wall * $cores)
$ws = ($after | Measure-Object -Property WorkingSet64 -Sum).Sum / 1MB
$private = ($after | Measure-Object -Property PrivateMemorySize64 -Sum).Sum / 1MB
Write-Output ("processes: {0} (lovelyDL + WebView2)" -f $after.Count)
Write-Output ("CPU over {0} s: {1:N2} % of all {2} cores ({3:N0} ms of CPU time)" -f $Sample, $percent, $cores, ($cpuAfter - $cpuBefore))
Write-Output ("memory: {0:N0} MB working set, {1:N0} MB private" -f $ws, $private)
$tools = @(Tools $since | Where-Object { $toolsBefore.ContainsKey($_.Id) })
if ($tools.Count) {
    $toolCpu = 0
    foreach ($p in $tools) { $toolCpu += $p.TotalProcessorTime.TotalMilliseconds - $toolsBefore[$p.Id] }
    $toolWs = ($tools | Measure-Object -Property WorkingSet64 -Sum).Sum / 1MB
    Write-Output ("yt-dlp and helpers: {0} processes, {1:N2} % CPU, {2:N0} MB working set" -f $tools.Count, (100 * $toolCpu / ($wall * $cores)), $toolWs)
}
if ($Exe -and -not $KeepRunning) { Stop-Process -Id $proc.Id -ErrorAction SilentlyContinue }
