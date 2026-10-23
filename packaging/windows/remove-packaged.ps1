#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\task-lifecycle.ps1"
Remove-BatteryBridgeTask -Destination $PSScriptRoot
$state = Join-Path $env:ProgramData 'BatteryBridge'
if (Test-Path -LiteralPath $state) {
    Protect-BatteryBridgeDirectory -LiteralPath $state
    Remove-Item -LiteralPath $state -Recurse -Force
}
