# Fixed ACL policy for the SYSTEM task. This file performs no task registration.
# NSIS can inherit PowerShell 7's module search path while running Windows PowerShell.
# Load the security cmdlets belonging to this runtime, not a parent's module copy.
Import-Module "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" -ErrorAction Stop
function Protect-BatteryBridgeDirectory {
    param([Parameter(Mandatory)][string]$LiteralPath, [switch]$ReadUsers)
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $trustedOwners = @('S-1-5-18', 'S-1-5-32-544', $identity.User.Value)
    if (Test-Path -LiteralPath $LiteralPath) {
        $root = Get-Item -LiteralPath $LiteralPath -Force
        if (-not $root.PSIsContainer -or ($root.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw "Refusing non-directory or reparse point: $LiteralPath"
        }
        $nodes = @($root) + @(Get-ChildItem -LiteralPath $LiteralPath -Force -Recurse)
        # Validate the complete tree before modifying any ACL or copying code.
        foreach ($node in $nodes) {
            if ($node.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing reparse point: $($node.FullName)" }
            $owner = (Get-Acl -LiteralPath $node.FullName).GetOwner([System.Security.Principal.SecurityIdentifier]).Value
            if ($owner -notin $trustedOwners) { throw "Refusing directory tree owned by another user: $($node.FullName)" }
        }
    } else {
        $nodes = @(New-Item -ItemType Directory -Path $LiteralPath -Force)
    }
    foreach ($node in $nodes) {
        if ($node.PSIsContainer) { $acl = [System.Security.AccessControl.DirectorySecurity]::new(); $inherit = 'OICI' }
        else { $acl = [System.Security.AccessControl.FileSecurity]::new(); $inherit = '' }
        # Replace the DACL entirely: icacls /grant:r would retain unrelated explicit ACEs.
        $sddl = "O:BAG:BAD:P(A;$inherit;FA;;;SY)(A;$inherit;FA;;;BA)"
        if ($ReadUsers) { $sddl += "(A;$inherit;GRGX;;;BU)" }
        $acl.SetSecurityDescriptorSddlForm($sddl)
        Set-Acl -LiteralPath $node.FullName -AclObject $acl
    }
}
