# All task scheduler calls and hardware clients are mocked; only disposable files/ACLs change.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\task-lifecycle.ps1"
$root = Join-Path ([IO.Path]::GetTempPath()) ("batterybridge-lifecycle-" + [Guid]::NewGuid())
$script:task = $null
$script:events = [Collections.Generic.List[string]]::new()
$script:version = 1
$script:limit = 80
$script:failRestore = $false
function Get-ScheduledTask { param($TaskName, $ErrorAction); return $script:task }
function Stop-ScheduledTask { param($TaskName); $script:events.Add('stop'); $script:task.State = 'Ready' }
function Start-ScheduledTask { param($TaskName); $script:events.Add('start'); $script:task.State = 'Running' }
function Unregister-ScheduledTask { param($TaskName, $Confirm); $script:events.Add('remove'); $script:task = $null }
function New-ScheduledTaskAction { param($Execute); return @{ Execute = $Execute } }
function New-ScheduledTaskTrigger { return @{} }
function New-ScheduledTaskSettingsSet { return @{} }
function New-ScheduledTaskPrincipal { return @{} }
function Register-ScheduledTask {
    param($TaskName, $Action, $Trigger, $Settings, $Principal, $Description, [switch]$Force)
    $script:events.Add('register')
    $script:task = [pscustomobject]@{ State = 'Ready'; Action = $Action }
}
function Read-BatteryBridgeConfig { param($Client); $script:events.Add('probe'); return @{ version = $script:version; charge_limit = $script:limit } }
function Restore-BatteryBridgeCharging {
    param($Client, $Config)
    $script:events.Add('restore')
    if ($script:failRestore) { throw 'fixture: restore failed' }
    $script:limit = $null
}
try {
    $source = Join-Path $root 'source'
    $destination = Join-Path $root 'installed'
    $state = Join-Path $root 'state'
    New-Item -ItemType Directory -Path $source,$state -Force | Out-Null
    foreach ($name in @('batterybridge.exe','batterybridge-service.exe')) {
        [IO.File]::WriteAllText((Join-Path $source $name), 'fixture')
    }
    $config = Join-Path $state 'config.toml'
    [IO.File]::WriteAllText($config, 'saved policy fixture')
    Install-BatteryBridgeTask -SourceDirectory $source -Destination $destination -StateDirectory $state
    if ($script:task.State -ne 'Running') { throw 'Task not started' }
    if ($script:task.Action.Execute -ne (Join-Path $destination 'batterybridge-service.exe')) { throw 'Wrong privileged executable' }
    if (-not (Test-Path -LiteralPath (Join-Path $destination 'batterybridge.exe'))) { throw 'Client not installed' }
    Install-BatteryBridgeTask -SourceDirectory $source -Destination $destination -StateDirectory $state
    if ([IO.File]::ReadAllText($config) -ne 'saved policy fixture') { throw 'Update changed saved settings' }
    $script:events.Clear()
    $script:failRestore = $true
    $failed = $false
    try { Remove-BatteryBridgeTask -Destination $destination } catch { $failed = $true }
    if (-not $failed -or $script:events.Contains('stop') -or $script:events.Contains('remove')) { throw 'Failed restore did not prevent removal' }
    $script:events.Clear()
    $script:failRestore = $false
    Remove-BatteryBridgeTask -Destination $destination
    if ($script:events.IndexOf('restore') -ge $script:events.IndexOf('stop')) { throw 'Task stopped before restoring charging' }
    if ($null -ne $script:task) { throw 'Task still registered' }
    Remove-BatteryBridgeTask -Destination $destination # Already absent is harmless.
    # A format-2 installation can have pending schedules even with its limit off.
    $script:version = 2
    $script:limit = $null
    $script:task = [pscustomobject]@{ State = 'Running' }
    $script:events.Clear()
    Remove-BatteryBridgeTask -Destination $destination
    if (-not $script:events.Contains('restore')) { throw 'Care schedules were not reset before removal' }
    if ($script:events.IndexOf('restore') -ge $script:events.IndexOf('stop')) { throw 'Schedule reset happened after task stop' }
    Write-Output 'Windows lifecycle checks: passed'
} finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
