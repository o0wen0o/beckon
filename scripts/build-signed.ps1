<#
.SYNOPSIS
    Bundles Beckon with the updater signing key, without putting the key or its
    password into the user environment.

.DESCRIPTION
    Since ADR-0022 `tauri build` refuses to bundle without a signing key:
    createUpdaterArtifacts is on, so an unsigned bundle is an error rather than
    an artifact. The two variables the bundler reads are
    TAURI_SIGNING_PRIVATE_KEY_PATH and TAURI_SIGNING_PRIVATE_KEY_PASSWORD.

    They are set here for *this process only*. Nothing is written to the user
    environment, because a password in HKCU\Environment is a password every
    process that account runs can read, forever, for the sake of one command.

    The password is asked for once and - on Windows only - can be remembered in
    ~/.tauri/beckon.pass, encrypted with DPAPI under the current user account.
    That file is useless to another account and useless copied to another
    machine, which is what separates it from writing the password to disk.
    PowerShell on macOS has no DPAPI, so there it asks every time.

    For a one-off there is no need for this script at all:

        $env:TAURI_SIGNING_PRIVATE_KEY_PATH = "$HOME\.tauri\beckon.key"
        $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = 'the password'
        npm run tauri build

    Those two assignments die with the shell, which is the whole point.

    Arguments are passed through to tauri build, so
    `scripts/build-signed.ps1 --target universal-apple-darwin` works.

    PowerShell rather than Node for the same reason gen-icons.ps1 is: it is what
    the repo already had. `pwsh scripts/build-signed.ps1` runs it on macOS.
#>
[CmdletBinding()]
param(
    # Ask for the password again, discarding any remembered copy.
    [switch]$Forget,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$BuildArgs
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$keyPath = Join-Path $HOME '.tauri/beckon.key'
$passPath = Join-Path $HOME '.tauri/beckon.pass'

if (-not (Test-Path $keyPath)) {
    throw "No signing key at $keyPath. Generate one with: npx tauri signer generate -w $keyPath -- or point this script at the maintainer's key. Without it, tauri build cannot produce updater artifacts (ADR-0022)."
}

if ($Forget -and (Test-Path $passPath)) {
    Remove-Item $passPath
    Write-Host 'Forgot the remembered password.'
}

# DPAPI is Windows-only. $IsWindows does not exist in Windows PowerShell 5.1,
# where the answer is always yes - hence the null check rather than the value.
$canRemember = ($null -eq $IsWindows) -or $IsWindows

$secure = $null
if ((Test-Path $passPath) -and -not $Forget) {
    try {
        $secure = ConvertTo-SecureString (Get-Content $passPath -Raw).Trim()
    }
    catch {
        # A blob written by another account, or a corrupted file. Not fatal:
        # asking is always available.
        Write-Warning "Could not read $passPath ($($_.Exception.Message)); asking instead."
        $secure = $null
    }
}

if (-not $secure) {
    $secure = Read-Host -AsSecureString "Password for $keyPath"
    if ($canRemember) {
        $answer = Read-Host "Remember it in $passPath, encrypted for this account? [y/N]"
        if ($answer -match '^[Yy]') {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $passPath) | Out-Null
            ConvertFrom-SecureString $secure | Set-Content $passPath -Encoding utf8
            Write-Host 'Remembered. Pass -Forget to clear it.'
        }
    }
}

# SecureString to plaintext the way that works in both 5.1 and 7: -AsPlainText
# on ConvertFrom-SecureString is 7-only, and the bundler wants a string in an
# environment variable either way.
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
try {
    $password = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
}
finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
}

$code = 1
Push-Location $root
try {
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $keyPath
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $password
    & npm run tauri build @BuildArgs
    $code = $LASTEXITCODE
}
finally {
    # Process-scoped either way, so this is housekeeping rather than a control -
    # but the shell that ran the script may be interactive and outlive it.
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    Pop-Location
}

exit $code
