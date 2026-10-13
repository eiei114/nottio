param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\nottio.exe'),
    [int]$AppPid = 0,
    [string]$PreviewPath
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class NottioIconCheck {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryExW(string file, IntPtr reserved, uint flags);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr FindResourceW(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll")]
    public static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")]
    public static extern IntPtr LockResource(IntPtr resource);
    [DllImport("kernel32.dll")]
    public static extern bool FreeLibrary(IntPtr module);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern uint ExtractIconExW(string file, int index, out IntPtr large, out IntPtr small, uint count);
    [DllImport("user32.dll")]
    public static extern bool DestroyIcon(IntPtr icon);
    [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW")]
    public static extern IntPtr GetClassLongPtr(IntPtr window, int index);
}
'@
$Exe = (Resolve-Path $Exe).Path
# Load as data only: inspecting resources never executes the application.
$module = [NottioIconCheck]::LoadLibraryExW($Exe, [IntPtr]::Zero, 0x22)
if ($module -eq [IntPtr]::Zero) { throw 'Could not load executable resources' }
try {
    $resource = [NottioIconCheck]::FindResourceW($module, [IntPtr]1, [IntPtr]14)
    if ($resource -eq [IntPtr]::Zero) { throw 'Missing RT_GROUP_ICON resource #1' }
    $data = [NottioIconCheck]::LockResource([NottioIconCheck]::LoadResource($module, $resource))
    if ($data -eq [IntPtr]::Zero) { throw 'Could not read icon group' }
    $count = [Runtime.InteropServices.Marshal]::ReadInt16($data, 4)
    if ($count -ne 10) { throw "Expected 10 icon resolutions; found $count" }
} finally {
    [void][NottioIconCheck]::FreeLibrary($module)
}
$large = [IntPtr]::Zero
$small = [IntPtr]::Zero
try {
    $extracted = [NottioIconCheck]::ExtractIconExW($Exe, 0, [ref]$large, [ref]$small, 1)
    if ($extracted -lt 1 -or $large -eq [IntPtr]::Zero -or $small -eq [IntPtr]::Zero) {
        throw 'Windows shell could not extract both application icons'
    }
    if ($PreviewPath) {
        $icon = [Drawing.Icon]::FromHandle($large).Clone()
        try {
            $bitmap = $icon.ToBitmap()
            try { $bitmap.Save($PreviewPath, [Drawing.Imaging.ImageFormat]::Png) }
            finally { $bitmap.Dispose() }
        } finally { $icon.Dispose() }
    }
} finally {
    if ($large -ne [IntPtr]::Zero) { [void][NottioIconCheck]::DestroyIcon($large) }
    if ($small -ne [IntPtr]::Zero) { [void][NottioIconCheck]::DestroyIcon($small) }
}
$windowIcon = $null
if ($AppPid) {
    $process = Get-Process -Id $AppPid
    if ($process.Path -ne $Exe -or $process.MainWindowHandle -eq [IntPtr]::Zero) {
        throw 'PID must belong to a visible window of this executable'
    }
    $windowIcon = [NottioIconCheck]::GetClassLongPtr($process.MainWindowHandle, -14)
    if ($windowIcon -eq [IntPtr]::Zero) { throw 'GPUI window class has no application icon' }
}
[pscustomobject]@{
    exe = $Exe
    group_resource_id = 1
    resolutions = $count
    shell_icons = $extracted
    window_icon_present = ($null -ne $windowIcon)
} | ConvertTo-Json -Compress
