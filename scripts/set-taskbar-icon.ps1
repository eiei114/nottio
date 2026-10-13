param([switch]$Apply, [switch]$RefreshShell)
$ErrorActionPreference = 'Stop'
if ($RefreshShell -and -not $Apply) {
    throw '-RefreshShell requires -Apply; read-only mode never changes shell state.'
}
function Get-IconHash([string]$Path) {
    $sha = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($Path)
    try { [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $sha.Dispose() }
}
$root = Split-Path $PSScriptRoot -Parent
$exe = (Resolve-Path (Join-Path $root 'target\release\nottio.exe')).Path
$icon = (Resolve-Path (Join-Path $root 'assets\nottio.ico')).Path
# A new path per artwork revision prevents a pinned icon from retaining the
# previous image cached under an unchanged ICO filename. Never purge global cache.
$hash = Get-IconHash $icon
$iconDirectory = Join-Path $env:APPDATA 'nottio\icons'
$cachedIcon = Join-Path $iconDirectory ("nottio-" + $hash.Substring(0, 16) + '.ico')
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
$expected = "${cachedIcon},0"
$backup = $null
if ($Apply) {
    New-Item -ItemType Directory -Path $iconDirectory -Force | Out-Null
    if (-not (Test-Path -LiteralPath $cachedIcon) -or
        (Get-IconHash $cachedIcon) -ne $hash) {
        Copy-Item -LiteralPath $icon -Destination $cachedIcon -Force
    }
    if ((Get-IconHash $cachedIcon) -ne $hash) {
        throw 'Cached taskbar artwork differs from the committed ICO.'
    }
}
if ($Apply -and $before -ne $expected) {
    $backup = Join-Path $env:APPDATA ("nottio\backups\" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-taskbar-icon')
    New-Item -ItemType Directory -Path $backup | Out-Null
    Copy-Item -LiteralPath $pinned -Destination (Join-Path $backup 'nottio.lnk')
    # Only icon metadata changes; retain launch target, arguments and workdir.
    $link.IconLocation = $expected
    $link.Save()
}
if ($Apply) {
    if (-not ('NottioShortcutNotify' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class NottioShortcutNotify {
    [DllImport("shell32.dll")]
    public static extern void SHChangeNotify(uint eventId, uint flags, IntPtr item1, IntPtr item2);
}
'@
    }
    $path = [Runtime.InteropServices.Marshal]::StringToHGlobalUni($pinned)
    try {
        # SHCNE_UPDATEITEM, SHCNF_PATHW | SHCNF_FLUSH: refresh this link only.
        [NottioShortcutNotify]::SHChangeNotify(0x2000, 0x1005, $path, [IntPtr]::Zero)
    } finally {
        [Runtime.InteropServices.Marshal]::FreeHGlobal($path)
    }
    if ($RefreshShell) {
        # A taskbar group may retain its old image despite the link update.
        # Standard redraw notification only: no association writes, cache-file
        # deletion, pin changes or Explorer restart. Other icons may redraw too.
        [NottioShortcutNotify]::SHChangeNotify(0x08000000, 0x1000, [IntPtr]::Zero, [IntPtr]::Zero)
    }
}
$after = $shell.CreateShortcut($pinned)
if ($after.TargetPath -ne $target -or $after.Arguments -ne $arguments -or $after.WorkingDirectory -ne $workingDirectory) {
    throw 'Launch settings changed unexpectedly; original shortcut was backed up.'
}
if ($Apply -and $after.IconLocation -ne $expected) {
    throw 'Shortcut did not retain the configured application icon.'
}
$cacheMatches = (Test-Path -LiteralPath $cachedIcon) -and (Get-IconHash $cachedIcon) -eq $hash
[pscustomobject]@{
    shortcut = $pinned
    target = $after.TargetPath
    icon_before = $before
    icon_after = $after.IconLocation
    configured = ($after.IconLocation -eq $expected -and $cacheMatches)
    artwork_sha256 = $hash
    applied = [bool]$Apply
    shell_refresh_notified = [bool]$RefreshShell
    backup = $backup
} | ConvertTo-Json -Compress
