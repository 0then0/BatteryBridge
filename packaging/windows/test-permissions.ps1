# Disposable ACL/junction regression checks; never registers a service or writes hardware.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\permissions.ps1"
$root = Join-Path ([IO.Path]::GetTempPath()) ("batterybridge-acl-" + [Guid]::NewGuid())
try {
    $state = Join-Path $root 'state'
    New-Item -ItemType Directory -Path $state -Force | Out-Null
    $acl = Get-Acl -LiteralPath $state
    $everyone = [Security.Principal.SecurityIdentifier]::new('S-1-1-0')
    $rule = [Security.AccessControl.FileSystemAccessRule]::new($everyone, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath $state -AclObject $acl
    Protect-BatteryBridgeDirectory -LiteralPath $state
    $secured = Get-Acl -LiteralPath $state
    if (-not $secured.AreAccessRulesProtected) { throw 'State directory still inherits ACLs' }
    $rules = $secured.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])
    foreach ($entry in $rules) {
        if ($entry.IdentityReference.Value -notin @('S-1-5-18','S-1-5-32-544')) { throw 'Untrusted explicit ACE survived' }
    }
    $target = Join-Path $root 'target'
    New-Item -ItemType Directory -Path $target | Out-Null
    $before = (Get-Acl -LiteralPath $target).Sddl
    $junction = Join-Path $root 'junction'
    New-Item -ItemType Junction -Path $junction -Target $target | Out-Null
    $rejected = $false
    try { Protect-BatteryBridgeDirectory -LiteralPath $junction } catch { $rejected = $true }
    if (-not $rejected) { throw 'Reparse point was accepted' }
    if ((Get-Acl -LiteralPath $target).Sddl -ne $before) { throw 'Junction target ACL was changed' }
    Remove-Item -LiteralPath $junction -Force
    Write-Output 'Windows ACL regression checks: passed'
} finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
