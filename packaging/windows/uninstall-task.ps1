#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\task-lifecycle.ps1"
Remove-BatteryBridgeTask -Destination (Join-Path $env:ProgramFiles 'BatteryBridge')
# Manual installation retains files and configuration for deliberate removal.
