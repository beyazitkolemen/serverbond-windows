param([string]$NsisDir = "$env:LOCALAPPDATA\tauri\NSIS")
$ErrorActionPreference = 'Stop'

# Execute the actual WebView2 section with registry reads, embedded files and
# process execution replaced by local fixtures. Never install a runtime or
# touch its registry keys. Cases include updates, stale registry entries and
# bootstrapper failure, without a working WebView being necessary.
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$template = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'src-tauri/windows/installer.nsi')
$section = [regex]::Match($template, '(?ms)^Section WebView2\r?\n.*?^SectionEnd').Value
if (!$section) { throw 'WebView2 installer section not found.' }
$section = [regex]::Replace($section, '(?m)^\s*ReadRegStr \$4 HKLM .*$', '    StrCpy $4 "${TEST_MACHINE}"')
$section = [regex]::Replace($section, '(?m)^\s*ReadRegStr \$4 HKCU .*$', '    StrCpy $4 "${TEST_USER}"')
$section = [regex]::Replace($section, '(?m)^\s*File .*$', '        ; Embedded fixture is not extracted.')
$section = [regex]::Replace($section, '(?m)^\s*Delete .*$', '        ; Temporary system paths are not touched.')
$section = [regex]::Replace($section, '(?m)^\s*ExecWait .*$', @'
        StrCpy $Installed 1
        StrCpy $1 ${TEST_EXIT}
        !if ${TEST_LAUNCH_ERROR} == 1
          SetErrors
        !endif
'@)
# A future template change must not accidentally execute a real OS operation.
if ($section -match '(?m)^\s*(ReadRegStr|ExecWait|Delete|File)\s') { throw 'Unmocked OS operation in WebView2 section.' }
$section = $section.Replace('SectionEnd', @'
  FileOpen $0 "${TEST_RESULT}" w
  FileWrite $0 $Installed
  FileClose $0
SectionEnd
'@)

$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("serverbond-webview-tests-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot | Out-Null
$cases = @(
    @{ Name='missing'; Machine=''; User=''; Update=0; Exit=0; Error=0; Installed=1; Status=0 },
    @{ Name='missing-during-update'; Machine=''; User=''; Update=1; Exit=0; Error=0; Installed=1; Status=0 },
    @{ Name='zero-version'; Machine='0.0.0.0'; User=''; Update=1; Exit=0; Error=0; Installed=1; Status=0 },
    @{ Name='machine-current'; Machine='141.0.3537.71'; User=''; Update=0; Exit=0; Error=0; Installed=0; Status=0 },
    @{ Name='user-current'; Machine=''; User='141.0.3537.71'; Update=0; Exit=0; Error=0; Installed=0; Status=0 },
    @{ Name='zero-machine-user-current'; Machine='0.0.0.0'; User='141.0.3537.71'; Update=0; Exit=0; Error=0; Installed=0; Status=0 },
    @{ Name='minimum'; Machine='109.0.1518.0'; User=''; Update=1; Exit=0; Error=0; Installed=0; Status=0 },
    @{ Name='old-without-edgeupdate-path'; Machine='109.0.1517.99'; User=''; Update=0; Exit=0; Error=0; Installed=1; Status=0 },
    @{ Name='old-during-silent-update'; Machine='108.0.0.0'; User=''; Update=1; Exit=0; Error=0; Installed=1; Status=0 },
    @{ Name='installer-failure'; Machine=''; User=''; Update=0; Exit=1603; Error=0; Installed=1; Status=1603 },
    @{ Name='launch-failure'; Machine=''; User=''; Update=1; Exit=0; Error=1; Installed=1; Status=1603 }
)
$header = @'
Unicode true
RequestExecutionLevel user
SilentInstall silent
!include LogicLib.nsh
!include x64.nsh
!include WordFunc.nsh
!define INSTALLWEBVIEW2MODE "embedBootstrapper"
!define WEBVIEW2INSTALLERARGS "/silent"
!define WEBVIEW2BOOTSTRAPPERPATH "unused"
!define WEBVIEW2INSTALLERPATH "unused"
!define WEBVIEW2APPGUID "unused"
!define MINIMUMWEBVIEW2VERSION "109.0.1518.0"
Name "ServerBond WebView2 fixture"
Var Installed
Var UpdateMode
Function .onInit
  StrCpy $Installed 0
  StrCpy $UpdateMode ${TEST_UPDATE}
FunctionEnd
LangString webview2Downloading 1033 "fixture"
LangString webview2DownloadSuccess 1033 "fixture"
LangString webview2DownloadError 1033 "fixture"
LangString webview2AbortError 1033 "fixture abort"
LangString installingWebview2 1033 "fixture"
LangString webview2InstallSuccess 1033 "fixture"
LangString webview2InstallError 1033 "fixture error"
'@
foreach ($case in $cases) {
    $result = Join-Path $testRoot ($case.Name + '.txt')
    $exe = Join-Path $testRoot ($case.Name + '.exe')
    $source = Join-Path $testRoot ($case.Name + '.nsi')
    $defines = @(
        "!define TEST_MACHINE `"$($case.Machine)`"",
        "!define TEST_USER `"$($case.User)`"",
        "!define TEST_UPDATE $($case.Update)",
        "!define TEST_EXIT $($case.Exit)",
        "!define TEST_LAUNCH_ERROR $($case.Error)",
        "!define TEST_RESULT `"$result`"",
        "OutFile `"$exe`""
    ) -join "`r`n"
    "$defines`r`n$header`r`n$section" | Set-Content -Encoding utf8 -LiteralPath $source
    & (Join-Path $NsisDir 'makensis.exe') /V2 $source
    if ($LASTEXITCODE -ne 0) { throw "NSIS fixture did not compile: $($case.Name)" }
    $process = Start-Process -FilePath $exe -ArgumentList '/S' -WindowStyle Hidden -PassThru
    if (!$process.WaitForExit(15000)) { $process.Kill(); throw "NSIS fixture timed out: $($case.Name)" }
    if ($process.ExitCode -ne $case.Status) { throw "Unexpected status $($process.ExitCode): $($case.Name)" }
    if ($case.Status -eq 0) {
        $actual = Get-Content -Raw -LiteralPath $result
        if ($actual -ne [string]$case.Installed) { throw "Unexpected bootstrapper count $actual : $($case.Name)" }
    }
    Write-Output "PASS: $($case.Name)"
}
Write-Output "All $($cases.Count) NSIS WebView2 fixtures passed. Artifacts: $testRoot"
