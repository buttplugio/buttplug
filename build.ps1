#requires -version 5.1
[CmdletBinding()]
param(
    [ValidateSet('debug','engine','test','all')]
    [string]$Target = 'debug',
    [int]$Jobs = 0
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Step($m) { Write-Host "`n==> $m" -ForegroundColor Cyan }
function Write-Ok($m)   { Write-Host "    OK  $m" -ForegroundColor Green }
function Write-Warn2($m){ Write-Host "    !!  $m" -ForegroundColor Yellow }

function Fail($msg, $fix) {
    Write-Host "`nFAILED: $msg" -ForegroundColor Red
    if ($fix) { Write-Host "FIX:    $fix" -ForegroundColor Yellow }
    exit 1
}

function Invoke-Native {
    param([string]$Exe, [string[]]$ArgList, [switch]$Quiet)
    if (-not $Quiet) { Write-Host "    $Exe $($ArgList -join ' ')" -ForegroundColor DarkGray }
    & $Exe @ArgList
    if ($LASTEXITCODE -ne 0) { Fail "'$Exe' exited with code $LASTEXITCODE" }
}

function Find-Exe {
    param([string]$Name, [string[]]$Candidates)
    foreach ($c in $Candidates) {
        if ($c -and (Test-Path -LiteralPath $c)) { return $c }
    }
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

$repoRoot = $PSScriptRoot
if (-not $repoRoot) { $repoRoot = (Get-Location).Path }

$manifest = Join-Path $repoRoot 'Cargo.toml'
if (-not (Test-Path -LiteralPath $manifest)) {
    Fail "no Cargo.toml in $repoRoot" "cd to the buttplug repo root and re-run."
}
$manifestText = Get-Content -LiteralPath $manifest -Raw
if ($manifestText -notmatch 'default-members' -or $manifestText -notmatch 'buttplug_server_hwmgr_sdl_gamepad') {
    Fail "$repoRoot does not look like the buttplug workspace root" "cd to the folder that contains Cargo.toml with [workspace]."
}
Write-Ok "workspace root: $repoRoot"

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$haveVsWhere = Test-Path -LiteralPath $vswhere

Write-Step 'Locating MSVC toolchain'
$vsPath = $null
if ($haveVsWhere) {
    $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    if ($vsPath) { $vsPath = $vsPath.Trim() }
}
if (-not $vsPath) {
    Fail 'no Visual Studio install with the MSVC x64 compiler' 'install "Desktop development with C++" in the VS Installer.'
}
Write-Ok "VS at $vsPath"

if ($env:VSCMD_VER) {
    Write-Ok "MSVC env already active (VSCMD_VER=$env:VSCMD_VER)"
} else {
    $devShell = Join-Path $vsPath 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll'
    if (-not (Test-Path -LiteralPath $devShell)) {
        Fail "DevShell module missing at $devShell" 'repair VS Build Tools, or run from "x64 Native Tools Command Prompt for VS".'
    }
    Import-Module $devShell
    Enter-VsDevShell -VsInstallPath $vsPath -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
    if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
        Fail 'Enter-VsDevShell ran but link.exe is unresolved' 'open "x64 Native Tools Command Prompt for VS" and run the script there.'
    }
    Write-Ok 'MSVC x64 environment initialized'
}

Write-Step 'Locating CMake'
$cmake = Find-Exe 'cmake' @((Join-Path $vsPath 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe'))
if (-not $cmake) {
    Fail 'cmake not found' 'install the "C++ CMake tools for Windows" component in the VS Installer.'
}
Write-Ok (& $cmake --version | Select-Object -First 1)
$env:PATH = (Split-Path -Parent $cmake) + ';' + $env:PATH

$ninja = Find-Exe 'ninja' @((Join-Path $vsPath 'Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe'))
if ($ninja) {
    $env:PATH = (Split-Path -Parent $ninja) + ';' + $env:PATH
    Write-Ok 'ninja available'
} else {
    Write-Warn2 'ninja not found, SDL3 will use the Visual Studio generator'
}

Write-Step 'Checking Rust toolchain'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail 'cargo not on PATH' 'close this window, open a new one.'
}
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $active = (& rustup show active-toolchain) 2>$null
    if ($active -notmatch 'msvc') {
        Write-Warn2 "active toolchain is not MSVC ($active); switching"
        Invoke-Native 'rustup' @('default','stable-x86_64-pc-windows-msvc') -Quiet
    }
}
$rustcLine = (& rustc -V) 2>$null
if ($rustcLine -notmatch 'rustc (\d+)\.(\d+)\.(\d+)') {
    Fail "could not parse rustc version from '$rustcLine'" 'reinstall the Rust MSVC toolchain via rustup.'
}
$ver = [version]("{0}.{1}.{2}" -f $Matches[1], $Matches[2], $Matches[3])
if ($ver -lt [version]'1.85.0') { Fail "rustc $ver is below 1.85" 'rustup update stable' }
Write-Ok "rustc $ver"

if ($Jobs -le 0) { $Jobs = [Environment]::ProcessorCount }

$lp = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem' -Name LongPathsEnabled -ErrorAction SilentlyContinue).LongPathsEnabled
if ($lp -ne 1) { Write-Warn2 'LongPathsEnabled off; using short CARGO_TARGET_DIR' }
$env:CARGO_TARGET_DIR = Join-Path $env:SystemDrive 'bpt'
Write-Ok "CARGO_TARGET_DIR=$env:CARGO_TARGET_DIR"

function Build-Debug {
    Write-Step 'cargo build (default-members)'
    Invoke-Native 'cargo' @('build','--jobs',"$Jobs")
    Write-Ok "output in $env:CARGO_TARGET_DIR\debug"
}
function Build-Engine {
    Write-Step 'cargo build --release -p intiface-engine'
    Invoke-Native 'cargo' @('build','--release','-p','intiface-engine','--jobs',"$Jobs")
    $exe = Join-Path $env:CARGO_TARGET_DIR 'release\intiface-engine.exe'
    if (Test-Path -LiteralPath $exe) {
        Write-Ok "built $exe"
        Write-Host "`n    run:  & '$exe' --websocket-port 12345 --use-bluetooth-le" -ForegroundColor Gray
    }
}

Push-Location $repoRoot
try {
    switch ($Target) {
        'debug'  { Build-Debug }
        'engine' { Build-Engine }
        'test'   { Write-Step 'cargo test'; Invoke-Native 'cargo' @('test','--jobs',"$Jobs") }
        'all'    { Build-Debug; Build-Engine }
    }
} finally { Pop-Location }

Write-Host "`nDone ($Target)." -ForegroundColor Green
