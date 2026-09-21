param([string]$NsisDir = "$env:LOCALAPPDATA\tauri\NSIS")
$ErrorActionPreference = 'Stop'

# Exercise the real template's initialization without installing the app,
# touching its registry keys, or starting/stopping services.
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$template = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'src-tauri/windows/installer.nsi')
$onInit = [regex]::Match($template, '(?ms)^Function \.onInit\r?\n.*?^FunctionEnd').Value
$restore = [regex]::Match($template, '(?ms)^Function RestorePreviousInstallLocation\r?\n.*?^FunctionEnd').Value
if (!$onInit -or !$restore) { throw 'Installer initialization functions were not found.' }
$testId = [guid]::NewGuid().ToString('N')
$testRoot = Join-Path ([IO.Path]::GetTempPath()) "serverbond-installer-test-$testId"
$registryKey = "Software\ServerBondTests\Installer-$testId"
$registryPath = "HKCU:\$registryKey"
New-Item -ItemType Directory -Path $testRoot | Out-Null
$harness = @'
Unicode true
RequestExecutionLevel user
SilentInstall silent
!include LogicLib.nsh
!include FileFunc.nsh
!include x64.nsh
!define INSTALLMODE "currentUser"
!define ARCH "x64"
!define PRODUCTNAME "ServerBond"
!define DISPLAYLANGUAGESELECTOR "false"
!define PLACEHOLDER_INSTALL_DIR "placeholder\ServerBond"
!define MANUPRODUCTKEY "__REGISTRY__"
!macro SetContext
  SetShellVarContext current
!macroend
Name "ServerBond path test"
OutFile "__EXE__"
InstallDir "${PLACEHOLDER_INSTALL_DIR}"
Var PassiveMode
Var NoShortcutMode
Var UpdateMode
__ONINIT__
__RESTORE__
Section
  FileOpen $0 "__RESULT__" w
  FileWrite $0 $INSTDIR
  FileClose $0
SectionEnd
'@
$exe = Join-Path $testRoot 'path-test.exe'
$resultFile = Join-Path $testRoot 'result.txt'
$source = Join-Path $testRoot 'path-test.nsi'
$harness.Replace('__REGISTRY__', $registryKey).Replace('__EXE__', $exe).Replace('__RESULT__', $resultFile).Replace('__ONINIT__', $onInit).Replace('__RESTORE__', $restore) | Set-Content -Encoding utf8 -LiteralPath $source

function Assert-InstallPath([string]$Expected, [string[]]$Arguments) {
    if (Test-Path -LiteralPath $resultFile) { Remove-Item -LiteralPath $resultFile }
    $process = Start-Process -FilePath $exe -ArgumentList $Arguments -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Path test exited with $($process.ExitCode)." }
    $actual = Get-Content -Raw -LiteralPath $resultFile
    if ($actual -ne $Expected) { throw "Expected '$Expected', got '$actual'." }
    Write-Output "PASS: $Expected"
}

try {
    & (Join-Path $NsisDir 'makensis.exe') /V2 $source
    if ($LASTEXITCODE -ne 0) { throw 'NSIS path harness did not compile.' }
    Assert-InstallPath 'C:\ServerBond' @('/S')
    New-Item -Path $registryPath -Force | Out-Null
    Set-Item -Path $registryPath -Value 'D:\Existing ServerBond'
    Assert-InstallPath 'D:\Existing ServerBond' @('/S', '/UPDATE')
    $custom = Join-Path $testRoot 'custom home'
    Assert-InstallPath $custom @('/S', "/D=$custom")
} finally {
    if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath }
    $resolved = [IO.Path]::GetFullPath($testRoot)
    if ([IO.Path]::GetDirectoryName($resolved) -ne [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') -or [IO.Path]::GetFileName($resolved) -ne "serverbond-installer-test-$testId") {
        throw 'Refusing cleanup outside the temporary test directory.'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
