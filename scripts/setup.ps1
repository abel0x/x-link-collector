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
    # Register a logon task so the receiver is always up.
    [switch]$Autostart
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Need($name, $hint) {
    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        throw "$name not found. $hint"
    }
}

Write-Host '==> checking prerequisites'
Need 'cargo'  'Install Rust from https://rustup.rs'
Need 'python' 'Install Python 3.9+ from https://python.org'

Write-Host '==> building the receiver'
cargo build --release --manifest-path "$root\receiver\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
$binary = Join-Path $root 'receiver\target\release\x-link-receiver.exe'

Write-Host '==> installing yt-dlp and gallery-dl'
$venv = Join-Path $root 'downloader\.venv'
if (-not (Test-Path $venv)) { python -m venv $venv }
& "$venv\Scripts\python.exe" -m pip install --quiet --upgrade pip yt-dlp gallery-dl
if ($LASTEXITCODE -ne 0) { throw 'pip install failed' }

if ($Autostart) {
    Write-Host '==> registering the logon task'
    $task = 'x-link-receiver'
    schtasks /Query /TN $task 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { schtasks /Delete /TN $task /F | Out-Null }
    schtasks /Create /TN $task /TR "`"$binary`"" /SC ONLOGON /RL LIMITED /F | Out-Null
    schtasks /Run /TN $task | Out-Null
    Write-Host "    running now, and at every logon (remove: schtasks /Delete /TN $task /F)"
}

Write-Host ''
Write-Host 'Done.' -ForegroundColor Green
Write-Host "  receiver : $binary"
Write-Host "  links go to  $env:USERPROFILE\Desktop\links.txt"
Write-Host ''
Write-Host 'Next:'
Write-Host '  1. Load the extension: open vivaldi://extensions (or chrome://extensions),'
Write-Host "     turn on Developer mode, Load unpacked -> $root\extension"
if (-not $Autostart) {
    Write-Host "  2. Start the receiver:  $binary"
    Write-Host '     (or re-run this script with -Autostart)'
}
Write-Host "  3. Download the media:  python downloader\x-download"
