#Requires -Version 5.1
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
<#
.SYNOPSIS
    Sets up x-link-collector on Windows: builds the receiver, installs the
    downloader's tools, and optionally starts the receiver at logon.

.EXAMPLE
    .\scripts\setup.ps1
    .\scripts\setup.ps1 -Autostart
#>
[CmdletBinding()]
param(
    # Register a logon task so the receiver (and its panel) is always up.
    [switch]$Autostart
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$panel = 'http://127.0.0.1:9876/'

function Need($name, $hint) {
    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        throw "$name not found. $hint"
    }
}

# The py launcher first: a bare python.exe may be the Microsoft Store
# placeholder, which answers --version with an offer to install Python.
function Find-Python {
    foreach ($candidate in @(@('py', '-3'), @('python'), @('python3'))) {
        if (-not (Get-Command $candidate[0] -ErrorAction SilentlyContinue)) { continue }
        $rest = @($candidate | Select-Object -Skip 1)
        $version = & $candidate[0] @rest --version 2>&1
        if ($LASTEXITCODE -eq 0 -and "$version" -match '^Python 3\.') { return , $candidate }
    }
    return $null
}

Write-Host '==> checking prerequisites'
Need 'cargo' 'Install Rust from https://rustup.rs'
$python = Find-Python
if (-not $python) {
    throw 'Python 3 not found. Install it from https://python.org and tick "Add python.exe to PATH".'
}

Write-Host '==> building the receiver'
cargo build --release --manifest-path "$root\receiver\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
$binary = Join-Path $root 'receiver\target\release\x-link-receiver.exe'

Write-Host '==> installing yt-dlp and gallery-dl'
$rest = @($python | Select-Object -Skip 1)
& $python[0] @rest "$root\downloader\x-download" --setup
if ($LASTEXITCODE -ne 0) { throw 'installing the download tools failed' }

if ($Autostart) {
    Write-Host '==> starting it at every sign-in'
    # The same entry the panel's "Start when you sign in to Windows" switch
    # writes, so either one can turn it off again. --no-open: signing in is
    # not the moment for a browser tab to appear.
    $run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
    Set-ItemProperty -Path $run -Name 'x-link-receiver' -Value "`"$binary`" --no-open"
    # An older setup registered a scheduled task instead; one is enough.
    schtasks /Query /TN 'x-link-receiver' 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { schtasks /Delete /TN 'x-link-receiver' /F | Out-Null }
    Start-Process $binary -ArgumentList '--no-open'
    Write-Host '    running now, and at every sign-in (switch it off in the panel: Settings)'
    Start-Sleep -Seconds 2
    Start-Process $panel
}

Write-Host ''
Write-Host 'Done.' -ForegroundColor Green
Write-Host "  receiver : $binary"
Write-Host "  panel    : $panel"
Write-Host ''
Write-Host 'Next:'
Write-Host '  1. Load the extension: open vivaldi://extensions (or chrome://extensions),'
Write-Host "     turn on Developer mode, Load unpacked -> $root\extension"
if (-not $Autostart) {
    Write-Host "  2. Start the receiver by double-clicking $binary"
    Write-Host '     The panel opens in your browser. (Or re-run this script with -Autostart.)'
}
Write-Host '  3. Download from the panel, or with: python downloader\x-download'
