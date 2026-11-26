#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Opens a GUI program where the user cannot see or focus it, screenshots its
  window, then ends it.

.DESCRIPTION
  Two ways, both leaving the user's foreground window alone:

  desktop     The program's windows live on a desktop of their own
              (CreateDesktop). Nothing appears on the user's desktop, and no
              window of it can take focus. The frame is asked of the window
              itself with PrintWindow(PW_RENDERFULLCONTENT), never from the
              screen.
  noactivate  The program's windows are on the user's desktop but shown with
              SW_SHOWNOACTIVATE and pushed to the bottom of the z-order: visible
              only where nothing else covers that place, never focused.

  auto (the default) uses desktop. A blank frame is an error, never a silent
  switch to a visible window: the report says to try noactivate by name.

  Green itself is told ELY_SMOKE=hold (the product's own no-activate start) and
  given a data folder of its own, so it neither takes focus nor meets the copy
  the user is running.

.PARAMETER Exe
  The program to start, by path.

.PARAMETER Arguments
  Its command line, such as a Markdown file to open.

.PARAMETER Screenshot
  Where the window's PNG goes.

.PARAMETER Mode
  auto (desktop), desktop, or noactivate.

.PARAMETER Theme
  Sets ELY_APPEARANCE when given (light, dark, system).

.PARAMETER Environment
  Variables for the program; ELY_SMOKE=hold by default.

.PARAMETER DataRoot
  Its data folder; a fresh temporary one by default.

.PARAMETER Report
  Writes what happened as JSON: the foreground window before and after, which
  frame came back, and how many of the program's windows were on the user's
  desktop.

.PARAMETER Screen
  Also saves a picture of the user's screen while the program runs.

.PARAMETER KeepData
  Keeps the temporary data folder.

.OUTPUTS
  Exit code 0 when a non-blank frame of the window was saved; 1 otherwise.

.EXAMPLE
  ./scripts/launch-quiet.ps1 -Exe target/debug/green.exe -Arguments fixtures/smoke.md `
      -Screenshot .amp/in/artifacts/green-quiet.png -Report .amp/in/artifacts/report.json
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$Arguments = '',
    [Parameter(Mandatory = $true)][string]$Screenshot,
    [ValidateSet('auto', 'desktop', 'noactivate')][string]$Mode = 'auto',
    [ValidateSet('light', 'dark', 'system')][string]$Theme,
    [hashtable]$Environment = @{ ELY_SMOKE = 'hold' },
    [string]$DataRoot,
    [string]$Report,
    [string]$Screen,
    [int]$TimeoutSeconds = 60,
    [int]$SettleMs = 2500,
    [switch]$KeepData
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not ('Quiet.Native' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Quiet
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct STARTUPINFO
    {
        public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
        public int dwX; public int dwY; public int dwXSize; public int dwYSize;
        public int dwXCountChars; public int dwYCountChars; public int dwFillAttribute; public int dwFlags;
        public short wShowWindow; public short cbReserved2; public IntPtr lpReserved2;
        public IntPtr hStdInput; public IntPtr hStdOutput; public IntPtr hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct PROCESS_INFORMATION
    {
        public IntPtr hProcess; public IntPtr hThread; public int dwProcessId; public int dwThreadId;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    public static class Native
    {
        public delegate bool EnumProc(IntPtr hwnd, IntPtr data);

        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern IntPtr CreateDesktopW(string name, IntPtr device, IntPtr devmode, int flags, uint access, IntPtr attributes);
        [DllImport("user32.dll", SetLastError = true)] public static extern bool CloseDesktop(IntPtr desktop);
        [DllImport("user32.dll", SetLastError = true)] public static extern bool EnumDesktopWindows(IntPtr desktop, EnumProc callback, IntPtr data);
        [DllImport("user32.dll", SetLastError = true)] public static extern bool EnumWindows(EnumProc callback, IntPtr data);
        [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hwnd, StringBuilder text, int count);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hwnd, StringBuilder text, int count);
        [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
        [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
        [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
        [DllImport("user32.dll")] public static extern IntPtr GetWindowDC(IntPtr hwnd);
        [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hwnd, IntPtr hdc);
        [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr dst, int x, int y, int width, int height, IntPtr src, int srcX, int srcY, uint rop);
        [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
        [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
        [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
        [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
        [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);

        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        static extern bool CreateProcessW(string application, string commandLine, IntPtr processAttributes,
            IntPtr threadAttributes, bool inheritHandles, uint flags, IntPtr environment, string directory,
            ref STARTUPINFO startup, out PROCESS_INFORMATION information);
        [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll")] public static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
        [DllImport("kernel32.dll")] public static extern bool TerminateProcess(IntPtr process, uint code);

        public const uint GENERIC_ALL = 0x10000000;
        public const uint STARTF_USESHOWWINDOW = 0x00000001;
        public const short SW_SHOWNOACTIVATE = 4;
        public const uint HWND_BOTTOM = 1;
        public const uint SWP_NOSIZE = 0x0001;
        public const uint SWP_NOMOVE = 0x0002;
        public const uint SWP_NOACTIVATE = 0x0010;
        public const uint WM_CLOSE = 0x0010;
        public const uint PW_RENDERFULLCONTENT = 0x00000002;
        public const uint SRCCOPY = 0x00CC0020;

        /// Every rectangle in pixels, not scaled points.
        public static void PerMonitor()
        {
            // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
            if (!SetProcessDpiAwarenessContext(new IntPtr(-4))) SetProcessDPIAware();
        }

        /// The whole virtual screen: all monitors, in pixels.
        public static void ScreenBox(out int x, out int y, out int width, out int height)
        {
            x = GetSystemMetrics(76);      // SM_XVIRTUALSCREEN
            y = GetSystemMetrics(77);      // SM_YVIRTUALSCREEN
            width = GetSystemMetrics(78);  // SM_CXVIRTUALSCREEN
            height = GetSystemMetrics(79); // SM_CYVIRTUALSCREEN
        }

        /// A desktop of this window station, or an exception.
        public static IntPtr NewDesktop(string name)
        {
            var desktop = CreateDesktopW(name, IntPtr.Zero, IntPtr.Zero, 0, GENERIC_ALL, IntPtr.Zero);
            if (desktop == IntPtr.Zero)
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "CreateDesktop " + name);
            return desktop;
        }

        /// Starts the program, on `desktop` when one is named; its process handle comes back.
        public static IntPtr Start(string exe, string arguments, string desktop, out int pid)
        {
            var startup = new STARTUPINFO();
            startup.cb = Marshal.SizeOf(typeof(STARTUPINFO));
            startup.lpDesktop = desktop;
            startup.dwFlags = (int)STARTF_USESHOWWINDOW;
            startup.wShowWindow = SW_SHOWNOACTIVATE;
            var command = "\"" + exe + "\"" + (arguments.Length == 0 ? "" : " " + arguments);
            PROCESS_INFORMATION information;
            // The environment is this process's, which the caller has set.
            if (!CreateProcessW(exe, command, IntPtr.Zero, IntPtr.Zero, false, 0, IntPtr.Zero,
                    System.IO.Path.GetDirectoryName(exe), ref startup, out information))
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "CreateProcess " + exe);
            pid = information.dwProcessId;
            CloseHandle(information.hThread);
            return information.hProcess;
        }

        /// The program's windows on `desktop`, the user's own when IntPtr.Zero.
        public static IntPtr[] Windows(IntPtr desktop, int process)
        {
            var found = new List<IntPtr>();
            EnumProc callback = (hwnd, data) =>
            {
                uint pid;
                GetWindowThreadProcessId(hwnd, out pid);
                if (pid == (uint)process) found.Add(hwnd);
                return true;
            };
            if (desktop == IntPtr.Zero) EnumWindows(callback, IntPtr.Zero);
            else EnumDesktopWindows(desktop, callback, IntPtr.Zero);
            return found.ToArray();
        }

        /// The program's largest visible window, or IntPtr.Zero.
        public static IntPtr Main(IntPtr desktop, int process)
        {
            IntPtr best = IntPtr.Zero;
            long area = -1;
            foreach (var hwnd in Windows(desktop, process))
            {
                if (!IsWindowVisible(hwnd)) continue;
                RECT rect;
                if (!GetWindowRect(hwnd, out rect)) continue;
                long size = (long)(rect.Right - rect.Left) * (rect.Bottom - rect.Top);
                if (size > area) { area = size; best = hwnd; }
            }
            return best;
        }

        public static string Title(IntPtr hwnd)
        {
            var text = new StringBuilder(512);
            GetWindowTextW(hwnd, text, text.Capacity);
            return text.ToString();
        }

        public static string Class(IntPtr hwnd)
        {
            var text = new StringBuilder(256);
            GetClassNameW(hwnd, text, text.Capacity);
            return text.ToString();
        }

        public static bool Frame(IntPtr hwnd, out int width, out int height)
        {
            RECT rect;
            GetWindowRect(hwnd, out rect);
            width = rect.Right - rect.Left;
            height = rect.Bottom - rect.Top;
            return width > 0 && height > 0;
        }

        /// Behind every other window, without taking focus.
        public static void ToBottom(IntPtr hwnd)
        {
            SetWindowPos(hwnd, new IntPtr((long)HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }

        public static void AskToClose(IntPtr hwnd) { PostMessageW(hwnd, WM_CLOSE, IntPtr.Zero, IntPtr.Zero); }
    }
}
'@
}
[Quiet.Native]::PerMonitor()

function Note([string]$text) { Write-Host $text }

# A folder of its own: a running Green is untouched and the log is ours to read.
$madeData = $false
if (-not $DataRoot) {
    $DataRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('green-quiet-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $madeData = $true
}
[void](New-Item -ItemType Directory -Force -Path $DataRoot)
$env:ELY_DATA_ROOT = (Resolve-Path $DataRoot).Path
foreach ($name in $Environment.Keys) { Set-Item -Path "env:$name" -Value $Environment[$name] }
if ($Theme) { $env:ELY_APPEARANCE = $Theme }
if (-not $env:ELY_LOG) { $env:ELY_LOG = 'info' }
# The log's place follows the data folder; names come from the app's own folder.
$Exe = (Resolve-Path $Exe).Path

function Log-File { Get-ChildItem -Path $env:ELY_DATA_ROOT -Recurse -Filter 'ely.log' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName }

$Screenshot = [System.IO.Path]::GetFullPath($Screenshot)
$Screen = if ($Screen) { [System.IO.Path]::GetFullPath($Screen) } else { $null }
$Report = if ($Report) { [System.IO.Path]::GetFullPath($Report) } else { $null }
foreach ($file in @($Screenshot, $Screen, $Report)) {
    if ($file) {
        $dir = [System.IO.Path]::GetDirectoryName($file)
        if ($dir -and -not (Test-Path $dir)) { [void](New-Item -ItemType Directory -Force -Path $dir) }
    }
}

function Foreground-Now {
    $hwnd = [Quiet.Native]::GetForegroundWindow()
    [pscustomobject]@{
        handle = ('0x{0:X}' -f $hwnd.ToInt64())
        title  = [Quiet.Native]::Title($hwnd)
        class  = [Quiet.Native]::Class($hwnd)
    }
}

function Measure-Frame($bitmap) {
    # A grid of pixels: a picture holds many colours, a blank frame one or two.
    $counts = @{}
    $total = 0
    $stepX = [Math]::Max(1, [int]($bitmap.Width / 40))
    $stepY = [Math]::Max(1, [int]($bitmap.Height / 40))
    for ($y = 0; $y -lt $bitmap.Height; $y += $stepY) {
        for ($x = 0; $x -lt $bitmap.Width; $x += $stepX) {
            $pixel = $bitmap.GetPixel($x, $y)
            $key = "$($pixel.R),$($pixel.G),$($pixel.B)"
            $counts[$key] = 1 + ($counts[$key] -as [int])
            $total++
        }
    }
    $dominant = 0
    foreach ($key in $counts.Keys) { if ($counts[$key] -gt $dominant) { $dominant = $counts[$key] } }
    [pscustomobject]@{
        distinct = $counts.Count
        dominant = [Math]::Round($dominant / [Math]::Max(1, $total), 4)
        blank    = ($counts.Count -le 4) -or (($dominant / [Math]::Max(1, $total)) -ge 0.995)
    }
}

function Save-Frame([IntPtr]$hwnd, [string]$path) {
    $width = 0; $height = 0
    if (-not [Quiet.Native]::Frame($hwnd, [ref]$width, [ref]$height)) { throw "window $hwnd has no rectangle" }
    $bitmap = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        # Asked of the window itself; the user's screen is never read.
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $hdc = $graphics.GetHdc()
        try { $drawn = [Quiet.Native]::PrintWindow($hwnd, $hdc, [Quiet.Native]::PW_RENDERFULLCONTENT) }
        finally { $graphics.ReleaseHdc($hdc); $graphics.Dispose() }
        $measured = Measure-Frame $bitmap
        $used = if ($drawn) { 'printwindow' } else { 'none' }
        if (-not $measured.blank) {
            $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
            return [pscustomobject]@{ taken = $used; width = $width; height = $height; distinct = $measured.distinct; dominant = $measured.dominant; blank = $false }
        }
        # The window's device context, for a window that does not answer PrintWindow.
        $bitmap.Dispose()
        $bitmap = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $target = $graphics.GetHdc()
        $source = [Quiet.Native]::GetWindowDC($hwnd)
        try { $copied = [Quiet.Native]::BitBlt($target, 0, 0, $width, $height, $source, 0, 0, [Quiet.Native]::SRCCOPY) }
        finally {
            if ($source -ne [IntPtr]::Zero) { [void][Quiet.Native]::ReleaseDC($hwnd, $source) }
            $graphics.ReleaseHdc($target); $graphics.Dispose()
        }
        $second = Measure-Frame $bitmap
        $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        if ($copied -and $second.distinct -gt $measured.distinct) { $used = 'bitblt' }
        return [pscustomobject]@{ taken = $used; width = $width; height = $height; distinct = $second.distinct; dominant = $second.dominant; blank = $second.blank }
    }
    finally { $bitmap.Dispose() }
}

function Save-Screen([string]$path) {
    $x = 0; $y = 0; $width = 0; $height = 0
    [Quiet.Native]::ScreenBox([ref]$x, [ref]$y, [ref]$width, [ref]$height)
    $bitmap = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try { $graphics.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size($width, $height))) }
        finally { $graphics.Dispose() }
        $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $bitmap.Dispose() }
    Note "launch-quiet: the user's screen at $path"
}

function Wait-Window([IntPtr]$desktop, [int]$childPid, [int]$seconds) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    while ($watch.Elapsed.TotalSeconds -lt $seconds) {
        $hwnd = [Quiet.Native]::Main($desktop, $childPid)
        if ($hwnd -ne [IntPtr]::Zero) { return $hwnd }
        Start-Sleep -Milliseconds 50
    }
    throw "no visible window of process $childPid appeared within ${seconds}s"
}

function Wait-FirstFrame([int]$seconds) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    while ($watch.Elapsed.TotalSeconds -lt $seconds) {
        $file = Log-File
        if ($file) {
            $text = Get-Content -Path $file -Raw -ErrorAction SilentlyContinue
            if ($text -and $text.Contains('ui: first frame presented')) { return $true }
        }
        Start-Sleep -Milliseconds 100
    }
    return $false
}

function Run-Quiet([string]$how) {
    $desktop = [IntPtr]::Zero
    $desktopName = $null
    $process = [IntPtr]::Zero
    $childPid = 0
    $hwnd = [IntPtr]::Zero
    try {
        if ($how -eq 'desktop') {
            $desktopName = 'GreenQuiet-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
            $desktop = [Quiet.Native]::NewDesktop($desktopName)
            Note "launch-quiet: desktop $desktopName created"
        }
        $process = [Quiet.Native]::Start($Exe, $Arguments, $desktopName, [ref]$childPid)
        Note "launch-quiet: $Exe started as $childPid ($how)"
        $hwnd = Wait-Window $desktop $childPid $TimeoutSeconds
        if ($how -eq 'noactivate') { [Quiet.Native]::ToBottom($hwnd) }
        Note ("launch-quiet: window {0} '{1}' class {2}" -f ('0x{0:X}' -f $hwnd.ToInt64()), [Quiet.Native]::Title($hwnd), [Quiet.Native]::Class($hwnd))
        if (Wait-FirstFrame $TimeoutSeconds) { Note 'launch-quiet: first frame presented' }
        else { Note 'launch-quiet: no first-frame line in the log; the window had its time to draw' }
        Start-Sleep -Milliseconds $SettleMs
        $frame = Save-Frame $hwnd $Screenshot
        Note ("launch-quiet: frame {0}x{1} by {2}, {3} colours, dominant {4}" -f $frame.width, $frame.height, $frame.taken, $frame.distinct, $frame.dominant)
        if ($frame.blank) { Note 'launch-quiet: the frame is blank' }
        if ($Screen) { Save-Screen $Screen }
        $frame | Add-Member -NotePropertyName desktop -NotePropertyValue $desktopName
        $frame | Add-Member -NotePropertyName handle -NotePropertyValue ('0x{0:X}' -f $hwnd.ToInt64())
        $frame | Add-Member -NotePropertyName windowTitle -NotePropertyValue ([Quiet.Native]::Title($hwnd))
        $frame | Add-Member -NotePropertyName windowsOnUserDesktop -NotePropertyValue ([Quiet.Native]::Windows([IntPtr]::Zero, $childPid).Count)
        $frame | Add-Member -NotePropertyName pid -NotePropertyValue $childPid
        return $frame
    }
    finally {
        if ($hwnd -ne [IntPtr]::Zero) { [Quiet.Native]::AskToClose($hwnd) }
        if ($process -ne [IntPtr]::Zero) {
            if ([Quiet.Native]::WaitForSingleObject($process, 5000) -ne 0) { [void][Quiet.Native]::TerminateProcess($process, 1) }
            [void][Quiet.Native]::CloseHandle($process)
        }
        if ($desktop -ne [IntPtr]::Zero) { [void][Quiet.Native]::CloseDesktop($desktop) }
    }
}

$before = Foreground-Now
Note ("launch-quiet: foreground before {0} '{1}'" -f $before.handle, $before.title)
$mode = $Mode
$frame = $null
try {
    if ($mode -eq 'auto') { $mode = 'desktop' }
    $frame = Run-Quiet $mode
}
finally {
    Start-Sleep -Milliseconds 200
    $after = Foreground-Now
    Note ("launch-quiet: foreground after  {0} '{1}'" -f $after.handle, $after.title)
}

$kept = ($before.handle -eq $after.handle)
$outcome = [pscustomobject]@{
    exe              = $Exe
    arguments        = $Arguments
    mode             = $mode
    screenshot       = $Screenshot
    screen           = $Screen
    frame            = $frame
    foregroundBefore = $before
    foregroundAfter  = $after
    foregroundKept   = $kept
    dataRoot         = $env:ELY_DATA_ROOT
}
if ($Report) {
    $outcome | ConvertTo-Json -Depth 6 | Set-Content -Path $Report -Encoding utf8
    Note "launch-quiet: report at $Report"
}
if (-not $KeepData -and $madeData) { Remove-Item -Recurse -Force $DataRoot -ErrorAction SilentlyContinue }
if (-not $kept) { Note 'launch-quiet: WARNING the foreground window changed' }
if (-not $frame -or $frame.blank) { exit 1 }
exit 0
