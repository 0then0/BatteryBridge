# Shared by the graphical installer and the administrator's manual setup scripts.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\permissions.ps1"

function Stop-BatteryBridgeTask {
    $task = Get-ScheduledTask -TaskName 'BatteryBridge' -ErrorAction SilentlyContinue
    if (-not $task) { return }
    Stop-ScheduledTask -TaskName 'BatteryBridge'
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((Get-ScheduledTask -TaskName 'BatteryBridge').State -eq 'Running') {
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Existing BatteryBridge task did not stop' }
        Start-Sleep -Milliseconds 200
    }
}
function Read-BatteryBridgeConfig {
    param([string]$Client)
    $output = & $Client config --json
    if ($LASTEXITCODE -ne 0) { throw 'BatteryBridge service did not respond' }
    return ($output | ConvertFrom-Json)
}
function Wait-BatteryBridgeService {
    param([string]$Client)
    for ($attempt = 0; $attempt -lt 10; $attempt++) {
        try { return Read-BatteryBridgeConfig -Client $Client } catch {
            if ($attempt -eq 9) { throw }
            Start-Sleep -Seconds 1
        }
    }
}
function Install-BatteryBridgeTask {
    param([string]$SourceDirectory, [string]$Destination, [string]$StateDirectory = (Join-Path $env:ProgramData 'BatteryBridge'))
    foreach ($name in @('batterybridge.exe', 'batterybridge-service.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $SourceDirectory $name) -PathType Leaf)) {
            throw "Required executable is missing: $name"
        }
    }
    Protect-BatteryBridgeDirectory -LiteralPath $Destination -ReadUsers
    Protect-BatteryBridgeDirectory -LiteralPath $StateDirectory
    Stop-BatteryBridgeTask
    foreach ($name in @('batterybridge.exe', 'batterybridge-service.exe')) {
        $source = [IO.Path]::GetFullPath((Join-Path $SourceDirectory $name))
        $target = [IO.Path]::GetFullPath((Join-Path $Destination $name))
        if ($source -ne $target) { Copy-Item -LiteralPath $source -Destination $target -Force }
    }
    Protect-BatteryBridgeDirectory -LiteralPath $Destination -ReadUsers
    $action = New-ScheduledTaskAction -Execute (Join-Path $Destination 'batterybridge-service.exe')
    $triggers = @((New-ScheduledTaskTrigger -AtStartup), (New-ScheduledTaskTrigger -AtLogOn))
    $settings = New-ScheduledTaskSettingsSet -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    $principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -LogonType ServiceAccount -RunLevel Highest
    Register-ScheduledTask -TaskName 'BatteryBridge' -Action $action -Trigger $triggers -Settings $settings -Principal $principal -Description 'Device-aware battery charge policy' -Force | Out-Null
    Start-ScheduledTask -TaskName 'BatteryBridge'
    Wait-BatteryBridgeService -Client (Join-Path $Destination 'batterybridge.exe') | Out-Null
}
function Restore-BatteryBridgeCharging {
    param([string]$Client)
    & $Client limit off
    if ($LASTEXITCODE -ne 0) { throw 'Could not restore normal charging; removal stopped' }
}
function Remove-BatteryBridgeTask {
    param([string]$Destination)
    $task = Get-ScheduledTask -TaskName 'BatteryBridge' -ErrorAction SilentlyContinue
    if (-not $task) { return }
    $client = Join-Path $Destination 'batterybridge.exe'
    if ($task.State -ne 'Running') { Start-ScheduledTask -TaskName 'BatteryBridge' }
    $config = Wait-BatteryBridgeService -Client $client
    # Windows currently supports only persistent conservation mode, no ephemeral modes.
    # A monitoring-only installation with an off policy has no charging state to restore.
    if ($null -ne $config.charge_limit) {
        Restore-BatteryBridgeCharging -Client $client
    }
    Stop-BatteryBridgeTask
    Unregister-ScheduledTask -TaskName 'BatteryBridge' -Confirm:$false
}
