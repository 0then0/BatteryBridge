Unicode true
RequestExecutionLevel admin
ManifestDPIAware true
SetCompressor /SOLID lzma
!include MUI2.nsh
!include LogicLib.nsh
!include x64.nsh
!include "installer-hooks.nsh"

Name "BatteryBridge"
OutFile "${OUTPUT}"
InstallDir "$PROGRAMFILES64\BatteryBridge"
VIProductVersion "${VERSION}.${BUILD_NUMBER}"
VIAddVersionKey "ProductName" "BatteryBridge"
VIAddVersionKey "FileDescription" "BatteryBridge installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "BatteryBridge contributors"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\BatteryBridge"
!define MUI_ICON "..\..\apps\gui\icons\icon.ico"
!define MUI_UNICON "..\..\apps\gui\icons\icon.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "French"
!insertmacro MUI_LANGUAGE "Japanese"
!insertmacro MUI_LANGUAGE "PortugueseBR"

Function .onInit
  SetRegView 64
FunctionEnd
Function un.onInit
  SetRegView 64
FunctionEnd

Section
  !insertmacro NSIS_HOOK_PREINSTALL
  ; Prepare stops the previous task and hardens these paths before replacing code.
  SetOutPath "$INSTDIR"
  File "${BUILD_DIR}\batterybridge-gui.exe"
  SetOutPath "$INSTDIR\service"
  File "${BUILD_DIR}\batterybridge.exe"
  File "${BUILD_DIR}\batterybridge-service.exe"
  File "permissions.ps1"
  File "task-lifecycle.ps1"
  File "install-packaged.ps1"
  File "remove-packaged.ps1"
  SetOutPath "$INSTDIR\licenses"
  File "..\..\LICENSE"
  File "..\..\assets\licenses\*.txt"
  SetShellVarContext all
  CreateShortcut "$SMPROGRAMS\BatteryBridge.lnk" "$INSTDIR\batterybridge-gui.exe"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "BatteryBridge"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "BatteryBridge contributors"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '$\"$INSTDIR\uninstall.exe$\"'
  WriteRegStr HKLM "${UNINSTALL_KEY}" "QuietUninstallString" '$\"$INSTDIR\uninstall.exe$\" /S'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
  !insertmacro NSIS_HOOK_POSTINSTALL
SectionEnd

Section "Uninstall"
  !insertmacro NSIS_HOOK_PREUNINSTALL
  ; Restore charging and stop the privileged task before removing its files.
  nsExec::Exec '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "Get-Process batterybridge-gui -ErrorAction SilentlyContinue | Stop-Process -Force"'
  Pop $0
  SetShellVarContext all
  Delete "$SMPROGRAMS\BatteryBridge.lnk"
  Delete "$INSTDIR\batterybridge-gui.exe"
  Delete "$INSTDIR\service\batterybridge.exe"
  Delete "$INSTDIR\service\batterybridge-service.exe"
  Delete "$INSTDIR\service\permissions.ps1"
  Delete "$INSTDIR\service\task-lifecycle.ps1"
  Delete "$INSTDIR\service\install-packaged.ps1"
  Delete "$INSTDIR\service\remove-packaged.ps1"
  RMDir "$INSTDIR\service"
  Delete "$INSTDIR\licenses\LICENSE"
  Delete "$INSTDIR\licenses\fastframe-MIT.txt"
  Delete "$INSTDIR\licenses\Inter-OFL.txt"
  RMDir "$INSTDIR\licenses"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
SectionEnd
