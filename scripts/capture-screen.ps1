# Captures a region of the primary screen (the taskbar, a toast) to a PNG.
#
#   powershell -File scripts/capture-screen.ps1 -Out shot.png [-Bottom 400] [-Minimize lovelyDL]
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [int]$Bottom = 0,
    [string]$Minimize = ""
)
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -Name W -Namespace U -MemberDefinition @"
[DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int cmd);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
"@
[U.W]::SetProcessDPIAware() | Out-Null
if ($Minimize) {
    $p = Get-Process | Where-Object { $_.MainWindowTitle -eq $Minimize } | Select-Object -First 1
    if ($p) { [U.W]::ShowWindow($p.MainWindowHandle, 6) | Out-Null; Start-Sleep -Milliseconds 300 }
    Write-Output "minimized"
    exit 0
}
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$top = if ($Bottom -gt 0) { $screen.Height - $Bottom } else { 0 }
$h = $screen.Height - $top
$bmp = New-Object System.Drawing.Bitmap $screen.Width, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen(0, $top, 0, 0, (New-Object System.Drawing.Size $screen.Width, $h))
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "Saved $Out"
