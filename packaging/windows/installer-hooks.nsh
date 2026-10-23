; NSIS is a 32-bit process; Sysnative selects the native 64-bit PowerShell.
!define BB_PACKAGING_DIR "${__FILEDIR__}"

!macro NSIS_HOOK_PREINSTALL
  ; The privileged task must never execute code from a user-writable install path.
  ${If} $INSTDIR != "$PROGRAMFILES64\BatteryBridge"
    WriteRegStr HKLM "Software\BatteryBridge\Installer" "Error" "BatteryBridge must be installed in $PROGRAMFILES64\BatteryBridge. (process: $0)"
    MessageBox MB_ICONSTOP "BatteryBridge must be installed in $PROGRAMFILES64\BatteryBridge." /SD IDOK
    SetErrorLevel 10
    Abort
  ${EndIf}
  InitPluginsDir
  File /oname=$PLUGINSDIR\permissions.ps1 "${BB_PACKAGING_DIR}\permissions.ps1"
  File /oname=$PLUGINSDIR\task-lifecycle.ps1 "${BB_PACKAGING_DIR}\task-lifecycle.ps1"
  File /oname=$PLUGINSDIR\prepare-install.ps1 "${BB_PACKAGING_DIR}\prepare-install.ps1"
  nsExec::ExecToStack /TIMEOUT=90000 '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\prepare-install.ps1" -Destination "$INSTDIR"'
  Pop $0
  Pop $1
  ${If} $0 != "0"
    WriteRegStr HKLM "Software\BatteryBridge\Installer" "Error" "Could not prepare BatteryBridge installation: $1 (process: $0)"
    MessageBox MB_ICONSTOP "Could not prepare BatteryBridge installation: $1" /SD IDOK
    SetErrorLevel 11
    Abort
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToStack /TIMEOUT=90000 '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\service\install-packaged.ps1"'
  Pop $0
  Pop $1
  ${If} $0 != "0"
    WriteRegStr HKLM "Software\BatteryBridge\Installer" "Error" "BatteryBridge service installation failed: $1 (process: $0)"
    MessageBox MB_ICONSTOP "BatteryBridge service installation failed: $1" /SD IDOK
    SetErrorLevel 12
    Abort
  ${EndIf}
  DeleteRegKey HKLM "Software\BatteryBridge\Installer"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToStack /TIMEOUT=90000 '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\service\remove-packaged.ps1"'
  Pop $0
  Pop $1
  ${If} $0 != "0"
    WriteRegStr HKLM "Software\BatteryBridge\Installer" "Error" "BatteryBridge removal stopped: $1 (process: $0)"
    MessageBox MB_ICONSTOP "BatteryBridge removal stopped: $1" /SD IDOK
    SetErrorLevel 13
    Abort
  ${EndIf}
  DeleteRegKey HKLM "Software\BatteryBridge\Installer"
!macroend
