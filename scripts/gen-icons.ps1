<#
.SYNOPSIS
    Regenerates every raster in src-tauri/icons from the SVG sources in assets/.

.DESCRIPTION
    assets/logo.svg is the source of truth for the app icon. The tray icons come
    from assets/tray-normal.svg and assets/tray-error.svg instead of from a 32px
    render of the logo: the logo leans on gradients, a glow filter and a drop
    shadow, all of which collapse into a smear at 32px, so the tray sources are
    a flattened redraw of the same mark.

    Rasterizing is done by `tauri icon`, which is already a devDependency, so
    this adds no toolchain. Two passes are needed for the app icon, because only
    the default pass emits icon.ico and icon.icns, and only a --png pass
    can ask for 256.

    Everything else `tauri icon` writes (Microsoft Store logos, android, ios) is
    for platforms Beckon does not target and is left in the temp dir.

    PowerShell rather than Node only because that is what the repo already had;
    `pwsh scripts/gen-icons.ps1` runs it on macOS, and both bundlers' icons come
    out of the one pass.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root 'src-tauri\icons'

$sources = [ordered]@{
    logo  = Join-Path $root 'assets\logo.svg'
    tray  = Join-Path $root 'assets\tray-normal.svg'
    error = Join-Path $root 'assets\tray-error.svg'
}
foreach ($src in $sources.Values) {
    if (-not (Test-Path $src)) { throw "Missing icon source: $src" }
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) "beckon-icons-$([System.Guid]::NewGuid().ToString('N'))"
$full = Join-Path $work 'full'
$px256 = Join-Path $work '256'
$tray = Join-Path $work 'tray'
$trayError = Join-Path $work 'tray-error'

function Invoke-TauriIcon {
    param([string]$Source, [string]$Output, [string[]]$Sizes)

    $iconArgs = @('tauri', 'icon', $Source, '-o', $Output)
    foreach ($size in $Sizes) { $iconArgs += @('-p', $size) }

    # tauri icon logs font-loading warnings to stderr on some machines and still
    # succeeds, so trust the exit code rather than the stream.
    & npx @iconArgs | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "tauri icon failed for $Source (exit $LASTEXITCODE)" }
}

try {
    New-Item -ItemType Directory -Force -Path $full, $px256, $tray, $trayError | Out-Null

    Invoke-TauriIcon -Source $sources.logo -Output $full -Sizes @()
    Invoke-TauriIcon -Source $sources.logo -Output $px256 -Sizes @('256')
    Invoke-TauriIcon -Source $sources.tray -Output $tray -Sizes @('32')
    Invoke-TauriIcon -Source $sources.error -Output $trayError -Sizes @('32')

    $copies = [ordered]@{
        '32x32.png'       = Join-Path $full '32x32.png'
        '128x128.png'     = Join-Path $full '128x128.png'
        '128x128@2x.png'  = Join-Path $full '128x128@2x.png'
        '256x256.png'     = Join-Path $px256 '256x256.png'
        'icon.ico'        = Join-Path $full 'icon.ico'
        'icon.icns'       = Join-Path $full 'icon.icns'
        'tray-normal.png' = Join-Path $tray '32x32.png'
        'tray-error.png'  = Join-Path $trayError '32x32.png'
    }

    foreach ($name in $copies.Keys) {
        $from = $copies[$name]
        if (-not (Test-Path $from)) { throw "tauri icon did not produce $from" }
        Copy-Item -Path $from -Destination (Join-Path $dest $name) -Force
        Write-Host "  wrote icons\$name"
    }
}
finally {
    if (Test-Path $work) { Remove-Item -Recurse -Force $work }
}

Write-Host "Icons regenerated into src-tauri\icons."
