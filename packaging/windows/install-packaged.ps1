#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\task-lifecycle.ps1"
Install-BatteryBridgeTask -SourceDirectory $PSScriptRoot -Destination $PSScriptRoot
