#Requires -RunAsAdministrator
param([string]$SourceDirectory = "$PSScriptRoot\..\..")
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\task-lifecycle.ps1"
Install-BatteryBridgeTask -SourceDirectory $SourceDirectory -Destination (Join-Path $env:ProgramFiles 'BatteryBridge')
