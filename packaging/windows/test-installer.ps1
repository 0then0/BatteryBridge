# Run only in the disposable GitHub Actions Windows VM after building NSIS.
$ErrorActionPreference = 'Stop'
function Invoke-Installer {
    param([string]$FilePath)
    $process = Start-Process -FilePath $FilePath -ArgumentList '/S' -PassThru
    if (-not $process.WaitForExit(180000)) {
        & taskkill.exe /PID $process.Id /T /F | Out-Host
        throw "Installer timed out after 180 seconds: $FilePath"
    }
    if ($process.ExitCode -ne 0) { throw "Installer failed ($($process.ExitCode)): $FilePath" }
}
# The NSIS runtime is 32-bit. Check which PowerShell can load the task cmdlets.
foreach ($directory in @('SysWOW64', 'System32')) {
    Write-Output "ScheduledTasks availability in $directory PowerShell:"
    & "$env:WINDIR/$directory/WindowsPowerShell/v1.0/powershell.exe" -NoProfile -NonInteractive -Command 'Get-Command Register-ScheduledTask -ErrorAction Stop | Select-Object Name,Source'
}
$setup = Get-ChildItem target/release/bundle/nsis/*-setup.exe | Select-Object -First 1
if (-not $setup) { throw 'NSIS installer missing' }
try { Invoke-Installer -FilePath $setup.FullName } catch {
    # Diagnose on this disposable VM without turning a failed installer into a pass.
    $failure = $_
    $installed = Join-Path $env:ProgramFiles 'BatteryBridge'
    if (Test-Path "$installed/service/install-packaged.ps1") {
        & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$installed/service/install-packaged.ps1"
    } else {
        & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File ./packaging/windows/prepare-install.ps1 -Destination $installed
    }
    throw $failure
}
$installed = Join-Path $env:ProgramFiles 'BatteryBridge'
$client = Join-Path $installed 'service/batterybridge.exe'
& $client config --json
if ($LASTEXITCODE -ne 0) { throw 'Installed service did not answer IPC' }
$task = Get-ScheduledTask -TaskName 'BatteryBridge'
if ($task.Principal.UserId -notin @('SYSTEM','S-1-5-18')) { throw 'Task must run as SYSTEM' }
Get-Process batterybridge-gui -ErrorAction SilentlyContinue | Stop-Process -Force
Invoke-Installer -FilePath (Join-Path $installed 'uninstall.exe')
if (Get-ScheduledTask -TaskName 'BatteryBridge' -ErrorAction SilentlyContinue) { throw 'Background task survived removal' }
if (Test-Path (Join-Path $installed 'service/batterybridge-service.exe')) { throw 'Daemon executable survived removal' }
if (Test-Path (Join-Path $env:ProgramData 'BatteryBridge')) { throw 'Saved settings survived removal' }
Write-Output 'Windows NSIS install / IPC / uninstall integration: passed'
