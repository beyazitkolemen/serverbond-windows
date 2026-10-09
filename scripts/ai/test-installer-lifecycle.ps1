param(
    [string]$RenderedDir = (Join-Path $PSScriptRoot '../../target/release/nsis/x64'),
    [string]$NsisDir = (Join-Path $env:LOCALAPPDATA 'tauri/NSIS'),
    [string]$ArtifactsRoot = 'D:\Temp\F4'
)
$ErrorActionPreference = 'Stop'
# This runs the actual rendered installer/uninstaller with an isolated identity.
# It never starts ServerBond or installs Microsoft prerequisites. Missing-runtime
# and elevated/non-admin cases belong in a disposable Windows guest.
$RenderedDir = [IO.Path]::GetFullPath($RenderedDir)
$ArtifactsRoot = [IO.Path]::GetFullPath($ArtifactsRoot)
if (-not $ArtifactsRoot.StartsWith('D:\', [StringComparison]::OrdinalIgnoreCase) -and $env:GITHUB_ACTIONS -ne 'true') { throw 'Local artifacts must stay on D:; CI may use its explicit ephemeral runner temp.' }
if (-not (Test-Path -LiteralPath $ArtifactsRoot)) { New-Item -ItemType Directory -Path $ArtifactsRoot | Out-Null }
$makensis = Join-Path $NsisDir 'makensis.exe'
if (-not (Test-Path -LiteralPath $makensis)) { throw "Missing NSIS: $makensis" }
$source = [IO.File]::ReadAllText((Join-Path $RenderedDir 'installer.nsi'))
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$config = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$renderedVersion = [regex]::Match($source, '(?m)^!define VERSION "([^"]+)"').Groups[1].Value
if ($renderedVersion -ne $config.version) { throw 'Rendered installer version is stale; rebuild Tauri before testing.' }
$renderedTime = (Get-Item -LiteralPath (Join-Path $RenderedDir 'installer.nsi')).LastWriteTimeUtc
foreach ($inputFile in @('src-tauri/windows/installer.nsi', 'src-tauri/windows/installer-hooks.nsh', 'src-tauri/tauri.conf.json')) {
    if ((Get-Item -LiteralPath (Join-Path $repoRoot $inputFile)).LastWriteTimeUtc -gt $renderedTime) { throw "Rendered installer predates $inputFile; rebuild Tauri before testing." }
}
if ($source -notmatch '(?m)^!define INSTALLMODE "currentUser"\s*$') { throw 'Fixture supports only currentUser installers.' }
$payloadMatch = [regex]::Match($source, '(?m)^!define MAINBINARYSRCPATH "([^"]+)"')
if (-not $payloadMatch.Success -or -not (Test-Path -LiteralPath $payloadMatch.Groups[1].Value)) { throw 'Rendered production payload missing; build first.' }
$payload = $payloadMatch.Groups[1].Value
$payloadHash = (Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash

function Read-WebViewVersion {
    foreach ($hive in @([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryHive]::CurrentUser)) {
        $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($hive, [Microsoft.Win32.RegistryView]::Registry32)
        try {
            $key = $base.OpenSubKey('Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}')
            if ($key) {
                try { $pv = [string]$key.GetValue('pv', '') } finally { $key.Dispose() }
                if ($pv -and $pv -ne '0.0.0.0') { return [version]$pv }
            }
        } finally { $base.Dispose() }
    }
    return [version]'0.0.0.0'
}
$minimum = [regex]::Match($source, '(?m)^!define MINIMUMWEBVIEW2VERSION "([^"]+)"').Groups[1].Value
if (-not $minimum -or (Read-WebViewVersion) -lt [version]$minimum) { throw 'Host WebView2 is absent/outdated. Use a disposable Windows guest; fixture refuses prerequisite installation.' }

function Registry-Snapshot([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) { return '<absent>' }
    $items = @((Get-Item -LiteralPath $path)) + @(Get-ChildItem -LiteralPath $path -Recurse)
    return (($items | Sort-Object Name | ForEach-Object {
        $key = $_
        $values = @($key.GetValueNames() | Sort-Object | ForEach-Object {
            [pscustomobject]@{ Name = $_; Kind = [string]$key.GetValueKind($_); Value = $key.GetValue($_, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) }
        })
        [pscustomobject]@{ Key = $key.Name; Values = $values }
    }) | ConvertTo-Json -Depth 12 -Compress)
}
function Host-Snapshot {
    $uninstall = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ServerBond'
    $install = if (Test-Path -LiteralPath $uninstall) { (Get-ItemProperty -LiteralPath $uninstall).InstallLocation } else { '' }
    $files = @((Join-Path ([Environment]::GetFolderPath('Desktop')) 'ServerBond.lnk'), (Join-Path ([Environment]::GetFolderPath('Programs')) 'ServerBond.lnk'))
    if ($install) { $files += Join-Path $install.Trim('"') 'serverbond-desktop.exe' }
    $run = if (Test-Path -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run') { Get-Item 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' } else { $null }
    return ([ordered]@{
        Uninstall = Registry-Snapshot $uninstall
        Product = Registry-Snapshot 'HKCU:\Software\ServerBond'
        WebViewMachine = Registry-Snapshot 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
        WebViewUser = Registry-Snapshot 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
        Autostart = if ($run) { $run.GetValue('ServerBond', '<absent>') } else { '<absent>' }
        Files = @($files | ForEach-Object { [pscustomobject]@{ Path = $_; Hash = if (Test-Path -LiteralPath $_) { (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash } else { '<absent>' } } })
        Services = @(Get-Service | Where-Object { $_.Name -like '*serverbond*' } | Sort-Object Name | Select-Object Name, Status)
    } | ConvertTo-Json -Depth 15 -Compress)
}
$originalRunExists = Test-Path -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$before = Host-Snapshot
$id = 'ServerBondInstallerTest-' + [guid]::NewGuid().ToString('N')
$testRoot = Join-Path $ArtifactsRoot $id
$defaultHome = Join-Path $testRoot 'default home'
$unicodeHome = Join-Path $testRoot "Türkçe öçşığ – O’Brien"
$binary = $id.ToLowerInvariant()
$bundle = 'com.serverbond.installertest.' + $id.Substring(24).ToLowerInvariant()
$productKey = 'HKCU:\Software\' + $id + '\' + $id
$manufacturerKey = 'HKCU:\Software\' + $id
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' + $id
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$desktopLink = Join-Path ([Environment]::GetFolderPath('Desktop')) ($id + '.lnk')
$startLink = Join-Path ([Environment]::GetFolderPath('Programs')) ($id + '.lnk')
$results = [Collections.Generic.List[object]]::new()
$probe = $null

function Assert([bool]$ok, [string]$message) { if (-not $ok) { throw $message } }
function NsIS-Literal([string]$value) { return $value.Replace('$', '$$').Replace('"', '$\"') }
function Set-Define([string]$text, [string]$name, [string]$value) {
    $pattern = '(?m)^!define ' + [regex]::Escape($name) + ' "[^"\r\n]*"'
    Assert ([regex]::Matches($text, $pattern).Count -eq 1) "Expected exactly one $name define."
    $replacement = '!define ' + $name + ' "' + (NsIS-Literal $value) + '"'
    return [regex]::Replace($text, $pattern, [Text.RegularExpressions.MatchEvaluator]{ param($m) $replacement })
}
function Run-Exe([string]$path, [string]$arguments, [int]$timeoutSeconds = 45) {
    # No /R flag: neither installed production payload nor UI is launched.
    $process = Start-Process -FilePath $path -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit($timeoutSeconds * 1000)) {
        $process.Kill(); $process.WaitForExit()
        throw "Fixture timed out: $path. Artifacts retained at $testRoot; inspect test-only processes before retry."
    }
    return $process.ExitCode
}
function Record([string]$name, [bool]$passed = $true) {
    $results.Add([pscustomobject]@{ Case = $name; Passed = $passed })
    if ($passed) { Write-Host "PASS $name" } else { Write-Host "FAIL $name" }
}
function Check-Payload([string]$installHome) {
    Assert ((Get-FileHash -LiteralPath (Join-Path $installHome ($binary + '.exe')) -Algorithm SHA256).Hash -eq $payloadHash) 'Installed production bytes differ.'
}
function Check-Data([string]$installHome) {
    foreach ($entry in $sentinels.GetEnumerator()) { Assert ([IO.File]::ReadAllText((Join-Path $installHome $entry.Key)) -eq $entry.Value) "Data changed: $($entry.Key)" }
}
function Uninstall([string]$installHome) {
    # NSIS normally copies its uninstaller to TEMP before removing its installed
    # copy. Do the identical-byte copy explicitly so we can synchronously observe
    # the actual uninstall process and its exit code without an orphaned child.
    $installedUninstaller = Join-Path $installHome 'uninstall.exe'
    $runner = Join-Path $testRoot ('uninstall-runner-' + [guid]::NewGuid().ToString('N') + '.exe')
    Copy-Item -LiteralPath $installedUninstaller -Destination $runner
    Assert ((Get-FileHash -LiteralPath $runner).Hash -eq (Get-FileHash -LiteralPath $installedUninstaller).Hash) 'Uninstaller runner bytes differ.'
    Assert ((Run-Exe $runner ('/S _?=' + $installHome)) -eq 0) 'Uninstall failed.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $installHome ($binary + '.exe')))) 'Uninstall retained executable.'
    Assert (-not (Test-Path -LiteralPath $installedUninstaller)) 'Uninstall retained installed uninstaller.'
    Assert (-not (Test-Path -LiteralPath $uninstallKey)) 'Uninstall registry retained.'
}
try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    foreach ($file in @('utils.nsh', 'FileAssociation.nsh')) { Copy-Item -LiteralPath (Join-Path $RenderedDir $file) -Destination $testRoot }
    $fixturePayload = Join-Path $testRoot ($binary + '.exe')
    Copy-Item -LiteralPath $payload -Destination $fixturePayload
    $fixture = $source
    foreach ($entry in @{
        MANUFACTURER = $id; PRODUCTNAME = $id; BUNDLEID = $bundle
        MAINBINARYNAME = $binary; MAINBINARYSRCPATH = $fixturePayload
    }.GetEnumerator()) { $fixture = Set-Define $fixture $entry.Key $entry.Value }
    # Keep real hooks/prerequisite detection and all file/registry/shortcut logic.
    $defaultPattern = '(?m)^\s*StrCpy \$INSTDIR "C:\\ServerBond"\s*$'
    Assert ([regex]::Matches($fixture, $defaultPattern).Count -eq 1) 'Unexpected default install path; safety rewrite rejected.'
    $newDefault = '      StrCpy $INSTDIR "' + (NsIS-Literal $defaultHome) + '"'
    $fixture = [regex]::Replace($fixture, $defaultPattern, [Text.RegularExpressions.MatchEvaluator]{ param($m) $newDefault })
    Assert ($fixture -notmatch '(?m)^!define (PRODUCTNAME|MANUFACTURER) "ServerBond"') 'Production identity remained.'
    Assert ($fixture -notmatch '(?m)^!define BUNDLEID "com\.serverbond\.desktop"') 'Production bundle remained.'
    Assert ($fixture -notmatch '(?m)^!define MAINBINARYNAME "serverbond-desktop"') 'Production process name remained.'
    $installers = @()
    foreach ($version in @('99.1.0', '99.1.1')) {
        $installer = Join-Path $testRoot ('install-' + $version + '.exe')
        $script = Set-Define $fixture OUTFILE $installer
        $script = Set-Define $script VERSION $version
        $script = Set-Define $script VERSIONWITHBUILD ($version + '.0')
        $scriptPath = Join-Path $testRoot ('install-' + $version + '.nsi')
        [IO.File]::WriteAllText($scriptPath, $script, [Text.UTF8Encoding]::new($true))
        & $makensis /V2 $scriptPath 2>&1 | Out-File -LiteralPath (Join-Path $testRoot ('compile-' + $version + '.log'))
        Assert ($LASTEXITCODE -eq 0) 'NSIS fixture compile failed.'
        $installers += $installer
    }
    Assert ((Run-Exe $installers[0] '/S /NS') -eq 0) 'Fresh default install failed.'
    Check-Payload $defaultHome
    Assert (-not (Test-Path -LiteralPath $desktopLink) -and -not (Test-Path -LiteralPath $startLink)) '/NS created shortcuts.'
    Assert ((Get-Item -LiteralPath $productKey).GetValue('') -eq $defaultHome) 'Default path registry wrong.'
    Record 'fresh-default-real-payload-no-shortcuts'
    Uninstall $defaultHome

    Assert ((Run-Exe $installers[0] ('/S /D=' + $unicodeHome)) -eq 0) 'Unicode custom install failed.'
    Check-Payload $unicodeHome
    Assert ((Test-Path -LiteralPath $desktopLink) -and (Test-Path -LiteralPath $startLink)) 'Normal silent install missing shortcuts.'
    $sentinels = @{ '.env' = 'fixture-secret=preserve'; 'config\settings.json' = '{"fixture":"preserve"}'; 'data\app.db' = 'fixture-db-preserve' }
    foreach ($entry in $sentinels.GetEnumerator()) {
        $path = Join-Path $unicodeHome $entry.Key
        [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path)) | Out-Null
        [IO.File]::WriteAllText($path, $entry.Value)
    }
    if (-not (Test-Path -LiteralPath $runKey)) { New-Item -Path $runKey -Force | Out-Null }
    New-ItemProperty -LiteralPath $runKey -Name $id -Value 'fixture-autostart-preserve' -PropertyType String -Force | Out-Null
    $shortcutHash = (Get-FileHash -LiteralPath $desktopLink).Hash
    Record 'custom-unicode-spaces-real-shortcuts'

    $lockedPath = Join-Path $unicodeHome ($binary + '.exe')
    $lock = [IO.File]::Open($lockedPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try { $code = Run-Exe $installers[1] '/S /UPDATE /NS' } finally { $lock.Dispose() }
    Check-Payload $unicodeHome; Check-Data $unicodeHome
    $versionPreserved = (Get-ItemProperty -LiteralPath $uninstallKey).DisplayVersion -eq '99.1.0'
    Record 'locked-upgrade-nonzero-preserves-old-version-and-data' ($code -ne 0 -and $versionPreserved)

    $uninstallerPath = Join-Path $unicodeHome 'uninstall.exe'
    $uninstallerHash = (Get-FileHash -LiteralPath $uninstallerPath).Hash
    $registryBefore = Registry-Snapshot $uninstallKey
    $lock = [IO.File]::Open($uninstallerPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try { $code = Run-Exe $installers[1] '/S /UPDATE /NS' } finally { $lock.Dispose() }
    Check-Payload $unicodeHome; Check-Data $unicodeHome
    $metadataPreserved = (Registry-Snapshot $uninstallKey) -eq $registryBefore
    $uninstallerPreserved = (Get-FileHash -LiteralPath $uninstallerPath).Hash -eq $uninstallerHash
    Record 'locked-uninstaller-upgrade-nonzero-preserves-metadata-and-files' ($code -ne 0 -and $metadataPreserved -and $uninstallerPreserved)

    Assert ((Run-Exe $installers[1] '/S /UPDATE /NS') -eq 0) 'Registry-path update failed.'
    Check-Payload $unicodeHome; Check-Data $unicodeHome
    Assert ((Get-ItemProperty -LiteralPath $uninstallKey).DisplayVersion -eq '99.1.1') 'Version registry not updated.'
    Assert ((Get-Item -LiteralPath $productKey).GetValue('') -eq $unicodeHome) 'Update changed registered path.'
    Assert ((Get-Item -LiteralPath $runKey).GetValue($id) -eq 'fixture-autostart-preserve') 'Update removed autostart.'
    Assert ((Get-FileHash -LiteralPath $desktopLink).Hash -eq $shortcutHash) 'Update modified existing shortcut.'
    Record 'update-registered-unicode-home-preserves-data-shortcuts-autostart'

    # Exercise actual FindProcess/KillProcess with a separate inert NSIS process
    # sharing ONLY the fixture basename; production app is never started.
    $probeDir = Join-Path $testRoot 'inert-running-process'
    [IO.Directory]::CreateDirectory($probeDir) | Out-Null
    $probePath = Join-Path $probeDir ($binary + '.exe')
    $probeScript = "Unicode true`nRequestExecutionLevel user`nSilentInstall silent`nOutFile `"$probePath`"`nSection`nSleep 30000`nSectionEnd`n"
    $probeNsi = Join-Path $probeDir 'probe.nsi'
    [IO.File]::WriteAllText($probeNsi, $probeScript, [Text.UTF8Encoding]::new($true))
    & $makensis /V2 $probeNsi 2>&1 | Out-File -LiteralPath (Join-Path $testRoot 'compile-probe.log')
    Assert ($LASTEXITCODE -eq 0) 'Inert process compile failed.'
    $probe = Start-Process -FilePath $probePath -WindowStyle Hidden -PassThru
    Start-Sleep -Milliseconds 700
    Assert (-not $probe.HasExited) 'Inert process exited before test.'
    Assert ((Run-Exe $installers[1] '/S /UPDATE /NS') -eq 0) 'Running fixture update failed.'
    Assert ($probe.WaitForExit(3000)) 'Installer did not stop test process.'
    Check-Payload $unicodeHome; Check-Data $unicodeHome
    Record 'running-unique-inert-process-stopped-by-real-installer'

    $uninstallRegistryBefore = Registry-Snapshot $uninstallKey
    $lock = [IO.File]::Open($lockedPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try { $code = Run-Exe (Join-Path $unicodeHome 'uninstall.exe') ('/S _?=' + $unicodeHome) } finally { $lock.Dispose() }
    Check-Payload $unicodeHome; Check-Data $unicodeHome
    $registryPreserved = (Registry-Snapshot $uninstallKey) -eq $uninstallRegistryBefore
    Record 'locked-uninstall-nonzero-preserves-registry-executable-data' ($code -ne 0 -and $registryPreserved)
    # The failure must also leave its uninstaller available for a later retry.
    Assert (Test-Path -LiteralPath (Join-Path $unicodeHome 'uninstall.exe')) 'Failed uninstall removed its own retry executable.'

    Uninstall $unicodeHome
    Check-Data $unicodeHome
    Assert (-not (Test-Path -LiteralPath $desktopLink) -and -not (Test-Path -LiteralPath $startLink)) 'Uninstall retained fixture shortcuts.'
    Assert ((Get-Item -LiteralPath $runKey).GetValue($id, '<absent>') -eq '<absent>') 'Uninstall retained fixture autostart.'
    Record 'uninstall-preserves-env-config-db-removes-shortcuts-registry-autostart'
    Assert ((Run-Exe $installers[1] '/S /NS') -eq 0) 'Reinstall using retained path failed.'
    Check-Payload $unicodeHome; Check-Data $unicodeHome
    Uninstall $unicodeHome
    Check-Data $unicodeHome
    Record 'reinstall-retained-path-preserves-data'
} finally {
    if ($probe -and -not $probe.HasExited) { $probe.Kill(); $probe.WaitForExit() }
    # Cleanup only the GUID identity. Keep D: artifacts, sentinels and logs for review.
    foreach ($path in @($desktopLink, $startLink)) { if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force } }
    foreach ($path in @($uninstallKey, $manufacturerKey)) { if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force } }
    Remove-ItemProperty -LiteralPath $runKey -Name $id -ErrorAction SilentlyContinue
    if (-not $originalRunExists -and (Test-Path -LiteralPath $runKey)) {
        $runAfter = Get-Item -LiteralPath $runKey
        if ($runAfter.ValueCount -eq 0 -and $runAfter.SubKeyCount -eq 0) { Remove-Item -LiteralPath $runKey -Force }
    }
    $after = Host-Snapshot
    Assert ($before -eq $after) 'Production installation/runtime/services snapshot changed! Inspect host before continuing.'
    if (Test-Path -LiteralPath $testRoot) {
        [IO.File]::WriteAllText((Join-Path $testRoot 'results.json'), (@{ Cases = @($results.ToArray()); HostUnchanged = ($before -eq $after); PayloadSHA256 = $payloadHash; Real = 'Full rendered NSIS install/update/uninstall, production EXE byte-copy, real isolated registry/shortcuts/process'; NotCovered = 'Production app launch, missing WebView2/VC install, elevation, non-admin Windows user, checked delete-app-data UI, MSI migration' } | ConvertTo-Json -Depth 6))
    }
    Write-Host "Artifacts: $testRoot"
}
$failed = @($results.ToArray() | Where-Object { -not $_.Passed })
Assert ($failed.Count -eq 0) ("Installer lifecycle failed: " + (($failed | ForEach-Object { $_.Case }) -join ', '))
Write-Host "$($results.Count) installer lifecycle cases passed; host production identity unchanged."
