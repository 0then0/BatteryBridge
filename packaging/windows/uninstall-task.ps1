#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
# First use batterybridge limit off and verify normal charging. Stop-ScheduledTask
# forcibly terminates the process; it does not deliver a graceful console signal.
Stop-ScheduledTask -TaskName 'BatteryBridge'
Unregister-ScheduledTask -TaskName 'BatteryBridge' -Confirm:$false
# Retain binaries and policy for deliberate removal/inspection by the administrator.
