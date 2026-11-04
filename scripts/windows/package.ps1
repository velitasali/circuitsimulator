# Bundle circuitsimulator.exe with the Qt libraries it loads.
# Run from anywhere:
#   pwsh scripts/windows/package.ps1
#   pwsh scripts/windows/package.ps1 -Configuration debug
#   pwsh scripts/windows/package.ps1 -BuildInstaller
#
# The folder is dist/windows. scripts/windows/circuitsimulator.iss packs that
# folder into a setup program using Inno Setup (ISCC.exe).

param(
    [ValidateSet("release", "debug")]
    [string]$Configuration = "release",
    [switch]$BuildInstaller
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$ExeName = "circuitsimulator.exe"
$Built = Join-Path $Root "target\$Configuration\$ExeName"
if (-not (Test-Path $Built)) {
    throw "No binary at $Built. Build it with: cargo build -p cs-app $(if ($Configuration -eq 'release') { '--release' })"
}

$Dest = Join-Path $Root "dist\windows"
if (Test-Path $Dest) {
    Remove-Item -Recurse -Force $Dest
}
New-Item -ItemType Directory -Path $Dest | Out-Null
Copy-Item $Built (Join-Path $Dest $ExeName)

$Qmake = Get-Command qmake -ErrorAction SilentlyContinue
if (-not $Qmake) {
    throw "qmake is not on PATH. Install Qt 6 and add its bin directory before running this script."
}
$QtBin = (& qmake -query QT_INSTALL_BINS).Trim()
$Deploy = Join-Path $QtBin "windeployqt.exe"
if (-not (Test-Path $Deploy)) {
    throw "windeployqt was not next to qmake ($QtBin)."
}

$QmlDir = Join-Path $Root "qml"
& $Deploy --compiler-runtime --qmldir $QmlDir (Join-Path $Dest $ExeName)
if ($LASTEXITCODE -ne 0) {
    throw "windeployqt failed with exit code $LASTEXITCODE"
}

# The MinGW runtime that linked the exe has to be the one the process loads.
# windeployqt copies it when it can see gcc. Copy the usual trio if gcc is
# on PATH and a name is still missing.
$Gcc = Get-Command gcc -ErrorAction SilentlyContinue
if ($Gcc) {
    $GccDir = Split-Path $Gcc.Source -Parent
    foreach ($Dll in @("libstdc++-6.dll", "libgcc_s_seh-1.dll", "libwinpthread-1.dll")) {
        $From = Join-Path $GccDir $Dll
        $To = Join-Path $Dest $Dll
        if ((Test-Path $From) -and -not (Test-Path $To)) {
            Copy-Item $From $To
        }
    }
}

@"
Circuit Simulator
Run circuitsimulator.exe in this folder.
Qt libraries were copied by windeployqt and must stay next to the exe.
"@ | Set-Content -Encoding utf8 (Join-Path $Dest "README.txt")

Write-Host "Bundle written to $Dest"

$IssScript = Join-Path $PSScriptRoot "circuitsimulator.iss"

function Find-Iscc {
    $cmd = Get-Command iscc -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $candidates = @(
        (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    foreach ($path in $candidates) {
        if ($path -and (Test-Path $path)) { return $path }
    }
    return $null
}

$Iscc = Find-Iscc

if ($BuildInstaller) {
    if (-not $Iscc) {
        throw "Inno Setup compiler (ISCC.exe) was not found on PATH or standard installation directories. Install Inno Setup 6 (winget install JRSoftware.InnoSetup or choco install innosetup) to compile the installer."
    }
    Write-Host "Compiling Inno Setup installer..."
    & $Iscc $IssScript
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
    }
    $InstallerExe = Join-Path $Root "dist\CircuitSimulator-setup.exe"
    Write-Host "Installer created successfully: $InstallerExe"
} elseif ($Iscc) {
    Write-Host "Inno Setup compiler found at: $Iscc"
    Write-Host "To compile the installer, re-run with -BuildInstaller or execute: & `"$Iscc`" `"$IssScript`""
} else {
    Write-Host "To create an installer executable, install Inno Setup 6 and run: iscc `"$IssScript`""
}
