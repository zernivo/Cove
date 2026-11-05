# Visual check on the runner (see .github/workflows/visual.yml).
$ErrorActionPreference = "Continue"
New-Item -ItemType Directory -Force shots | Out-Null
$shots = (Resolve-Path shots).Path

Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class W {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hh, bool r);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern int GetSystemMetrics(int i);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int SystemParametersInfo(int a, int b, string c, int d);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, int flags, IntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
"@
[W]::SetProcessDPIAware() | Out-Null

function Shot($name) {
  $name = "$($env:COVE_MODE)-$name"
  $w = [W]::GetSystemMetrics(0); $h = [W]::GetSystemMetrics(1)
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen(0, 0, 0, 0, $bmp.Size)
  $bmp.Save("$shots\$name.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  "shot $name ($w x $h)"
}
function Focus($p) {
  # Alt tap so this process may hand the foreground over.
  [W]::keybd_event(0x12, 0, 0, [IntPtr]::Zero); [W]::keybd_event(0x12, 0, 2, [IntPtr]::Zero)
  [W]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
}

# Screen, theme, transparency, wallpaper.
try { Set-DisplayResolution -Width 1920 -Height 1080 -Force -ErrorAction Stop } catch { "Set-DisplayResolution: $_" }
$light = if ($env:COVE_MODE -eq "light") { 1 } else { 0 }
$pers = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"
New-Item -Force $pers | Out-Null
Set-ItemProperty $pers EnableTransparency 1 -Type DWord
Set-ItemProperty $pers AppsUseLightTheme $light -Type DWord
Set-ItemProperty $pers SystemUsesLightTheme $light -Type DWord
$wall = (Resolve-Path "ui\public\wallpapers\dune-light.jpg").Path
[W]::SystemParametersInfo(0x14, 0, $wall, 3) | Out-Null
"screen: $([W]::GetSystemMetrics(0)) x $([W]::GetSystemMetrics(1))"
Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version | Format-List | Out-String

# Close File Explorer windows left from a previous pass (not the shell).
try { (New-Object -ComObject Shell.Application).Windows() | ForEach-Object { $_.Quit() } } catch { }
# Install (once).
Get-Process cove, notepad -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep 2
if (-not (Test-Path setup.exe)) {
Copy-Item (Get-ChildItem src-tauri\target\release\bundle\nsis\*-setup.exe | Select-Object -First 1).FullName setup.exe
Start-Process .\setup.exe -ArgumentList "/S" -Wait
}
$exe = Get-ChildItem "$env:LOCALAPPDATA\Cove","$env:ProgramFiles\Cove" -Filter *.exe -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.Name -notmatch "uninstall" } | Select-Object -First 1
"exe: $($exe.FullName)"

# Settings: Cove desktop on, traffic lights on, intro + tour seen, dock always shown.
$ver = (Get-Content src-tauri\tauri.conf.json | ConvertFrom-Json).version
$short = ($env:GITHUB_SHA).Substring(0, 12)
$dir = "$env:APPDATA\app.cove.desktop"
New-Item -ItemType Directory -Force $dir | Out-Null
$mode = if ($light -eq 1) { "light" } else { "dark" }
@{ dock = $true; takeover = $true; traffic = $true; tour_done = $true; intro_seen = "$ver+$short"; dock_hide = "never"; menu_bar = "off"; appearance = $mode; tray_icons = $true; last_version = $ver } | ConvertTo-Json | Set-Content "$dir\settings.json" -Encoding utf8
Get-Content "$dir\settings.json"

# A text file for Notepad, so the glass has detail to blur.
$txt = "$env:TEMP\cove-glass.txt"
(1..60 | ForEach-Object { "Line $_  The quick brown fox jumps over the lazy dog. 0123456789 ABCDEFGHIJ" }) | Set-Content $txt

$cove = Start-Process $exe.FullName -PassThru
Start-Sleep 18
Shot "00-desktop"

$np = Start-Process notepad.exe -ArgumentList $txt -PassThru
Start-Sleep 3
$np.Refresh()
[W]::ShowWindow($np.MainWindowHandle, 1) | Out-Null
[W]::MoveWindow($np.MainWindowHandle, 980, 380, 900, 560, $true) | Out-Null
Start-Process explorer.exe -ArgumentList "C:\Windows"
Start-Sleep 4
$ex = Get-Process explorer | Where-Object { $_.MainWindowTitle -ne "" } | Select-Object -First 1
if ($ex) { [W]::MoveWindow($ex.MainWindowHandle, 60, 60, 860, 520, $true) | Out-Null }
Start-Sleep 1
Focus $np
[W]::SetCursorPos(500, 700) | Out-Null
Start-Sleep 2
Shot "01-notepad-focused"

foreach ($k in @("cc", "menu", "wifi")) {
  Start-Process $exe.FullName -ArgumentList "--cove-open=$k"
  Start-Sleep 3
  Shot "02-$k"
  # Toggle it closed again (same item), then make sure.
  Start-Process $exe.FullName -ArgumentList "--cove-open=$k"
  Start-Sleep 1
  [W]::keybd_event(0x1B, 0, 0, [IntPtr]::Zero); [W]::keybd_event(0x1B, 0, 2, [IntPtr]::Zero)
  Start-Sleep 1
}

if ($ex) { Focus $ex; Start-Sleep 2; Shot "03-explorer-focused" }
Focus $np
Start-Sleep 2
Shot "04-notepad-again"

$alive = -not $cove.HasExited
"cove alive: $alive"
Get-Process | Where-Object { $_.Name -match "cove|notepad|explorer" } | Format-Table Name, Id, MainWindowTitle -AutoSize | Out-String
Copy-Item "$dir\cove.log" "$shots\cove-$($env:COVE_MODE).log" -ErrorAction SilentlyContinue
Copy-Item "$dir\settings.json" "$shots\settings-$($env:COVE_MODE).json" -ErrorAction SilentlyContinue
if (-not $alive) { exit 1 }
