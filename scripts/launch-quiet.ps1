#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Starts a GUI program without giving it the user's focus, screenshots its
  window, then ends it.

.DESCRIPTION
  TEMPORARY. `green --capture` (docs/capture-mode.md) shows no window at all and
  writes the same pixels byte for byte; use it instead once it has landed. This
  script stays only for pictures of a real window (decorations, dragging, real
  input focus), which `--capture` cannot express.

  The point here is one thing: GetForegroundWindow() reads the same before and
  after. The picture comes from the window itself (PrintWindow), never from the
  screen, so it works whether or not the user is looking at that desktop.

  A new window lands on whichever virtual desktop is active when it appears,
  and stays on it afterwards; the report says which one that was, so a run can
  show whether the window turned up in front of the user's work. Moving a
  window to another desktop is not offered: the documented interface refuses a
  window owned by another process (E_ACCESSDENIED).

  Green is told ELY_SMOKE=hold, the product's own no-activate start, and given
  a data folder of its own, so it neither takes focus nor meets the copy the
  user is running.

.PARAMETER Exe
  The program to start, by path.

.PARAMETER Arguments
  Its command line, such as a Markdown file to open.

.PARAMETER Screenshot
  Where the window's PNG goes.

.PARAMETER Theme
  Sets ELY_APPEARANCE when given (light, dark, system).

.PARAMETER Environment
  Variables for the program; ELY_SMOKE=hold by default.

.PARAMETER DataRoot
  Its data folder; a fresh temporary one by default.

.PARAMETER Report
  Writes what happened as JSON: the foreground window before and after, which
  frame came back, which virtual desktop the window landed on, and how many of
  the program's windows were on the user's desktop.

.PARAMETER Screen
  Also saves a picture of the user's screen while the program runs.

.PARAMETER TimeoutSeconds
  How long to wait for the window, and for the log's first-frame line.

.PARAMETER SettleMs
  How long the window draws before the picture is taken. Typst sets formulas in
  the background, so the first frame is often not the finished one.

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

    /// The documented virtual-desktop interface: reading only, never moving.
    [ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("A5CD92FF-29BE-454C-8D04-D82879FB3F1B")]
    interface IVirtualDesktopManager
    {
        int IsWindowOnCurrentVirtualDesktop(IntPtr topLevelWindow, out int onCurrentDesktop);
        int GetWindowDesktopId(IntPtr topLevelWindow, out Guid desktopId);
        int MoveWindowToDesktop(IntPtr topLevelWindow, ref Guid desktopId);
    }

    public static class Native
    {
        public delegate bool EnumProc(IntPtr hwnd, IntPtr data);

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

        /// Starts the program without asking to be activated; its process handle comes back.
        public static IntPtr Start(string exe, string arguments, out int pid)
        {
            var startup = new STARTUPINFO();
            startup.cb = Marshal.SizeOf(typeof(STARTUPINFO));
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

        /// The program's largest visible window, or IntPtr.Zero.
        public static IntPtr Main(int process)
        {
            IntPtr best = IntPtr.Zero;
            long area = -1;
            EnumWindows((hwnd, data) =>
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner != (uint)process || !IsWindowVisible(hwnd)) return true;
                RECT rect;
                if (!GetWindowRect(hwnd, out rect)) return true;
                long size = (long)(rect.Right - rect.Left) * (rect.Bottom - rect.Top);
                if (size > area) { area = size; best = hwnd; }
                return true;
            }, IntPtr.Zero);
            return best;
        }

        /// The program's windows on the user's own desktop, visible or not.
        public static int Count(int process)
        {
            int count = 0;
            EnumWindows((hwnd, data) =>
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner == (uint)process) count++;
                return true;
            }, IntPtr.Zero);
            return count;
        }

        /// The virtual desktop a window is on, and whether that is the current one; "" when the
        /// shell keeps no record of the window, which happens right after it appears.
        public static string WindowDesktop(IntPtr hwnd, out int onCurrentDesktop)
        {
            onCurrentDesktop = -1;
            try
            {
                var manager = (IVirtualDesktopManager)Activator.CreateInstance(
                    Type.GetTypeFromCLSID(new Guid("AA509086-5CA9-4C25-8F95-589D3C07B48A")));
                int on;
                manager.IsWindowOnCurrentVirtualDesktop(hwnd, out on);
                onCurrentDesktop = on;
                Guid id;
                return manager.GetWindowDesktopId(hwnd, out id) == 0 ? id.ToString() : "";
            }
            catch
            {
                return "";
            }
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

$Exe = (Resolve-Path $Exe).Path
$Screenshot = [System.IO.Path]::GetFullPath($Screenshot)
$Screen = if ($Screen) { [System.IO.Path]::GetFullPath($Screen) } else { $null }
$Report = if ($Report) { [System.IO.Path]::GetFullPath($Report) } else { $null }
foreach ($file in @($Screenshot, $Screen, $Report)) {
    if ($file) {
        $dir = [System.IO.Path]::GetDirectoryName($file)
        if ($dir -and -not (Test-Path $dir)) { [void](New-Item -ItemType Directory -Force -Path $dir) }
    }
}

function Log-File { Get-ChildItem -Path $env:ELY_DATA_ROOT -Recurse -Filter 'ely.log' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName }

function Foreground-Now {
    $hwnd = [Quiet.Native]::GetForegroundWindow()
    [pscustomobject]@{
        handle = ('0x{0:X}' -f $hwnd.ToInt64())
        title  = [Quiet.Native]::Title($hwnd)
        class  = [Quiet.Native]::Class($hwnd)
    }
}

function Virtual-Desktops {
    # The shell's own list, in display order; each id is 16 packed bytes.
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops')
    if (-not $key) { return $null }
    $raw = $key.GetValue('VirtualDesktopIDs')
    if (-not $raw) { return $null }
    $list = @()
    for ($i = 0; $i -lt $raw.Length; $i += 16) { $list += [Guid]::new([byte[]]$raw[$i..($i + 15)]).ToString() }
    $current = $key.GetValue('CurrentVirtualDesktop')
    [pscustomobject]@{
        list    = $list
        current = if ($current) { [Guid]::new([byte[]]$current).ToString() } else { $null }
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

function Wait-Window([int]$childPid, [int]$seconds) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    while ($watch.Elapsed.TotalSeconds -lt $seconds) {
        $hwnd = [Quiet.Native]::Main($childPid)
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

$before = Foreground-Now
Note ("launch-quiet: foreground before {0} '{1}'" -f $before.handle, $before.title)
$process = [IntPtr]::Zero
$childPid = 0
$hwnd = [IntPtr]::Zero
$frame = $null
try {
    $process = [Quiet.Native]::Start($Exe, $Arguments, [ref]$childPid)
    Note "launch-quiet: $Exe started as $childPid"
    $hwnd = Wait-Window $childPid $TimeoutSeconds
    [Quiet.Native]::ToBottom($hwnd)
    Note ("launch-quiet: window {0} '{1}' class {2}" -f ('0x{0:X}' -f $hwnd.ToInt64()), [Quiet.Native]::Title($hwnd), [Quiet.Native]::Class($hwnd))
    if (Wait-FirstFrame $TimeoutSeconds) { Note 'launch-quiet: first frame presented' }
    else { Note 'launch-quiet: no first-frame line in the log; the window had its time to draw' }
    Start-Sleep -Milliseconds $SettleMs
    $frame = Save-Frame $hwnd $Screenshot
    Note ("launch-quiet: frame {0}x{1} by {2}, {3} colours, dominant {4}" -f $frame.width, $frame.height, $frame.taken, $frame.distinct, $frame.dominant)
    if ($frame.blank) { Note 'launch-quiet: the frame is blank' }
    if ($Screen) { Save-Screen $Screen }
    # Where the window landed, read and never moved.
    $onCurrent = -1
    $windowDesktop = [Quiet.Native]::WindowDesktop($hwnd, [ref]$onCurrent)
    $desktops = Virtual-Desktops
    $indexOf = { param($id) if ($desktops -and $id) { $desktops.list.IndexOf($id) } else { -1 } }
    $virtualDesktop = [pscustomobject]@{
        window           = $windowDesktop
        windowIndex      = (& $indexOf $windowDesktop)
        current          = if ($desktops) { $desktops.current } else { $null }
        currentIndex     = (& $indexOf $(if ($desktops) { $desktops.current } else { $null }))
        onCurrentDesktop = ($onCurrent -eq 1)
        desktops         = if ($desktops) { $desktops.list } else { @() }
    }
    Note ("launch-quiet: virtual desktop: window on '{0}' (index {1}), current '{2}' (index {3}), onCurrent={4}" -f
        $virtualDesktop.window, $virtualDesktop.windowIndex, $virtualDesktop.current, $virtualDesktop.currentIndex, $virtualDesktop.onCurrentDesktop)
    $frame | Add-Member -NotePropertyName handle -NotePropertyValue ('0x{0:X}' -f $hwnd.ToInt64())
    $frame | Add-Member -NotePropertyName windowTitle -NotePropertyValue ([Quiet.Native]::Title($hwnd))
    $frame | Add-Member -NotePropertyName windowsOnUserDesktop -NotePropertyValue ([Quiet.Native]::Count($childPid))
    $frame | Add-Member -NotePropertyName virtualDesktop -NotePropertyValue $virtualDesktop
    $frame | Add-Member -NotePropertyName pid -NotePropertyValue $childPid
}
finally {
    if ($hwnd -ne [IntPtr]::Zero) { [Quiet.Native]::AskToClose($hwnd) }
    if ($process -ne [IntPtr]::Zero) {
        if ([Quiet.Native]::WaitForSingleObject($process, 5000) -ne 0) { [void][Quiet.Native]::TerminateProcess($process, 1) }
        [void][Quiet.Native]::CloseHandle($process)
    }
    Start-Sleep -Milliseconds 200
    $after = Foreground-Now
    Note ("launch-quiet: foreground after  {0} '{1}'" -f $after.handle, $after.title)
}

$kept = ($before.handle -eq $after.handle)
$outcome = [pscustomobject]@{
    exe              = $Exe
    arguments        = $Arguments
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
