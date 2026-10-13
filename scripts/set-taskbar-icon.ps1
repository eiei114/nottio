param([switch]$Apply)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = (Resolve-Path (Join-Path $root 'target\release\nottio.exe')).Path
$icon = (Resolve-Path (Join-Path $root 'assets\nottio.ico')).Path
$pinned = Join-Path $env:APPDATA 'Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\nottio.lnk'
if (-not (Test-Path -LiteralPath $pinned)) {
    throw 'nottio is not pinned here. Pin the app first; this script never pins unrelated applications.'
}
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($pinned)
if ($link.TargetPath -ne $exe) {
    throw 'Pinned shortcut targets another executable; refusing to modify it.'
}
$before = $link.IconLocation
$target = $link.TargetPath
$arguments = $link.Arguments
$workingDirectory = $link.WorkingDirectory
$expected = "${icon},0"
$backup = $null
if ($Apply -and $before -ne $expected) {
    $backup = Join-Path $env:APPDATA ("nottio\backups\" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-taskbar-icon')
    New-Item -ItemType Directory -Path $backup | Out-Null
    Copy-Item -LiteralPath $pinned -Destination (Join-Path $backup 'nottio.lnk')
    # Only icon metadata changes; retain launch target, arguments and workdir.
    $link.IconLocation = $expected
    $link.Save()
}
if ($Apply) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class NottioShortcutNotify {
    [DllImport("shell32.dll")]
    public static extern void SHChangeNotify(uint eventId, uint flags, IntPtr item1, IntPtr item2);
}
'@
    $path = [Runtime.InteropServices.Marshal]::StringToHGlobalUni($pinned)
    try {
        # SHCNE_UPDATEITEM, SHCNF_PATHW | SHCNF_FLUSH: refresh this link only.
        [NottioShortcutNotify]::SHChangeNotify(0x2000, 0x1005, $path, [IntPtr]::Zero)
    } finally {
        [Runtime.InteropServices.Marshal]::FreeHGlobal($path)
    }
}
$after = $shell.CreateShortcut($pinned)
if ($after.TargetPath -ne $target -or $after.Arguments -ne $arguments -or $after.WorkingDirectory -ne $workingDirectory) {
    throw 'Launch settings changed unexpectedly; original shortcut was backed up.'
}
if ($Apply -and $after.IconLocation -ne $expected) {
    throw 'Shortcut did not retain the configured application icon.'
}
[pscustomobject]@{
    shortcut = $pinned
    target = $after.TargetPath
    icon_before = $before
    icon_after = $after.IconLocation
    configured = ($after.IconLocation -eq $expected)
    applied = [bool]$Apply
    backup = $backup
} | ConvertTo-Json -Compress
