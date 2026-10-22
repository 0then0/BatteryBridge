# Run explicitly from an elevated PowerShell. Installs a machine-wide scheduled
# SYSTEM task; this executable does not implement the Windows SCM service API.
#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
$Destination = Join-Path $env:ProgramFiles 'BatteryBridge'
$State = Join-Path $env:ProgramData 'BatteryBridge'
. "$PSScriptRoot\permissions.ps1"
$Source = "$PSScriptRoot\..\..\batterybridge-service.exe"
if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw 'Daemon executable is missing from the release archive' }
Protect-BatteryBridgeDirectory -LiteralPath $Destination -ReadUsers
Protect-BatteryBridgeDirectory -LiteralPath $State
# Stop an existing task before replacing its executable on Windows.
$existing = Get-ScheduledTask -TaskName 'BatteryBridge' -ErrorAction SilentlyContinue
if ($existing) {
    Stop-ScheduledTask -TaskName 'BatteryBridge'
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((Get-ScheduledTask -TaskName 'BatteryBridge').State -eq 'Running') {
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Existing BatteryBridge task did not stop' }
        Start-Sleep -Milliseconds 200
    }
}
Copy-Item -LiteralPath $Source -Destination "$Destination\batterybridge-service.exe"
Protect-BatteryBridgeDirectory -LiteralPath $Destination -ReadUsers
$Action = New-ScheduledTaskAction -Execute "$Destination\batterybridge-service.exe"
$Triggers = @((New-ScheduledTaskTrigger -AtStartup), (New-ScheduledTaskTrigger -AtLogOn))
$Settings = New-ScheduledTaskSettingsSet -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
$Principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -LogonType ServiceAccount -RunLevel Highest
Register-ScheduledTask -TaskName 'BatteryBridge' -Action $Action -Trigger $Triggers -Settings $Settings -Principal $Principal -Description 'Device-aware battery charge policy' -Force | Out-Null
Start-ScheduledTask -TaskName 'BatteryBridge'
