# Run only on a disposable Windows CI VM. Exercise the real NSIS boundary before
# compiling Rust, so hook failures retain their actual subprocess diagnostics.
$ErrorActionPreference = 'Stop'
$work = Join-Path $env:RUNNER_TEMP 'batterybridge-nsis-test'
New-Item -ItemType Directory -Force $work | Out-Null
python ./packaging/windows/build.py --prepare-tools
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare NSIS toolchain' }
$compiler = Get-Item target/tools/nsis-3.13/makensis.exe
$hooks = (Resolve-Path "$PSScriptRoot/installer-hooks.nsh").Path
$output = Join-Path $work 'prepare.exe'
@"
Unicode true
RequestExecutionLevel admin
!include LogicLib.nsh
!include x64.nsh
!include "$hooks"
OutFile "$output"
Section
  StrCpy `$INSTDIR "`$PROGRAMFILES64\BatteryBridge"
  !insertmacro NSIS_HOOK_PREINSTALL
  SetErrorLevel 0
SectionEnd
"@ | Set-Content "$work/prepare.nsi"
& $compiler.FullName /V2 "$work/prepare.nsi"
if ($LASTEXITCODE -ne 0) { throw 'Could not compile NSIS hook test' }
$process = Start-Process $output -ArgumentList '/S' -PassThru
if (-not $process.WaitForExit(120000)) { & taskkill.exe /PID $process.Id /T /F; throw 'NSIS hook timed out' }
foreach ($view in @([Microsoft.Win32.RegistryView]::Registry32, [Microsoft.Win32.RegistryView]::Registry64)) {
    $root = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, $view)
    $key = $root.OpenSubKey('Software\BatteryBridge\Installer')
    if ($key) { Write-Output ($key.GetValue('Error')); $key.Dispose() }
    $root.Dispose()
}
if ($process.ExitCode -ne 0) { throw "NSIS preparation failed: $($process.ExitCode)" }
# The test only prepares empty protected directories, never a task or daemon.
Remove-Item (Join-Path $env:ProgramFiles 'BatteryBridge'),(Join-Path $env:ProgramData 'BatteryBridge') -Recurse -Force
Write-Output 'Native NSIS preparation: passed'
