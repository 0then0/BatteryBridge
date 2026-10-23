#Requires -RunAsAdministrator
param([Parameter(Mandatory)][string]$Destination)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\task-lifecycle.ps1"
Protect-BatteryBridgeDirectory -LiteralPath $Destination -ReadUsers
Protect-BatteryBridgeDirectory -LiteralPath (Join-Path $env:ProgramData 'BatteryBridge')
Stop-BatteryBridgeTask

# Updating replaces the GUI executable as well as the stopped service.
Get-Process batterybridge-gui -ErrorAction SilentlyContinue | Stop-Process -Force
