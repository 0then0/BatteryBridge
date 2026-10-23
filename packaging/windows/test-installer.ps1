# Run only in the disposable GitHub Actions Windows VM after building NSIS.
$ErrorActionPreference = 'Stop'
$setup = Get-ChildItem target/release/bundle/nsis/*-setup.exe | Select-Object -First 1
if (-not $setup) { throw 'NSIS installer missing' }
$process = Start-Process -FilePath $setup.FullName -ArgumentList '/S' -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
$installed = Join-Path $env:ProgramFiles 'BatteryBridge'
$client = Join-Path $installed 'service/batterybridge.exe'
& $client config --json
if ($LASTEXITCODE -ne 0) { throw 'Installed service did not answer IPC' }
$task = Get-ScheduledTask -TaskName 'BatteryBridge'
if ($task.Principal.UserId -notin @('SYSTEM','S-1-5-18')) { throw 'Task must run as SYSTEM' }
Get-Process batterybridge-gui -ErrorAction SilentlyContinue | Stop-Process -Force
$process = Start-Process -FilePath (Join-Path $installed 'uninstall.exe') -ArgumentList '/S' -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Uninstaller failed: $($process.ExitCode)" }
if (Get-ScheduledTask -TaskName 'BatteryBridge' -ErrorAction SilentlyContinue) { throw 'Background task survived removal' }
if (Test-Path (Join-Path $installed 'service/batterybridge-service.exe')) { throw 'Daemon executable survived removal' }
if (Test-Path (Join-Path $env:ProgramData 'BatteryBridge')) { throw 'Saved settings survived removal' }
Write-Output 'Windows NSIS install / IPC / uninstall integration: passed'
