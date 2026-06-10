Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."

Name "LuodaCodex"
OutFile "${ROOT}\dist\windows\LuodaCodex-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\LuodaCodex"
InstallDirRegKey HKCU "Software\LuodaCodex" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!define MUI_ICON "${ROOT}\apps\luoda-codex-manager\src-tauri\icons\icon.ico"
!define MUI_UNICON "${ROOT}\apps\luoda-codex-manager\src-tauri\icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"

  nsExec::ExecToLog 'taskkill /IM luoda-codex.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM luoda-codex-manager.exe /F'
  Pop $0

  File "${ROOT}\dist\windows\app\luoda-codex.exe"
  File "${ROOT}\dist\windows\app\luoda-codex-manager.exe"

  Delete "$DESKTOP\LuodaCodex 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\LuodaCodex\LuodaCodex 绠＄悊宸ュ叿.lnk"

  CreateShortcut "$DESKTOP\LuodaCodex.lnk" "$INSTDIR\luoda-codex.exe" "" "$INSTDIR\luoda-codex.exe"
  CreateShortcut "$DESKTOP\LuodaCodex.lnk" "$INSTDIR\luoda-codex-manager.exe" "" "$INSTDIR\luoda-codex-manager.exe"
  CreateDirectory "$SMPROGRAMS\LuodaCodex"
  CreateShortcut "$SMPROGRAMS\LuodaCodex\LuodaCodex.lnk" "$INSTDIR\luoda-codex.exe" "" "$INSTDIR\luoda-codex.exe"
  CreateShortcut "$SMPROGRAMS\LuodaCodex\LuodaCodex.lnk" "$INSTDIR\luoda-codex-manager.exe" "" "$INSTDIR\luoda-codex-manager.exe"
  CreateShortcut "$SMPROGRAMS\LuodaCodex\卸载 LuodaCodex.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\luoda-codex-manager.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\LuodaCodex" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex" "DisplayName" "LuodaCodex"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex" "Publisher" "BigPizzaV3"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex" "DisplayIcon" "$INSTDIR\luoda-codex-manager.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM luoda-codex.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM luoda-codex-manager.exe /F'
  Pop $0

  Delete "$DESKTOP\LuodaCodex.lnk"
  Delete "$DESKTOP\LuodaCodex.lnk"
  Delete "$DESKTOP\LuodaCodex 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\LuodaCodex\LuodaCodex.lnk"
  Delete "$SMPROGRAMS\LuodaCodex\LuodaCodex.lnk"
  Delete "$SMPROGRAMS\LuodaCodex\LuodaCodex 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\LuodaCodex\卸载 LuodaCodex.lnk"
  RMDir "$SMPROGRAMS\LuodaCodex"

  Delete "$INSTDIR\luoda-codex.exe"
  Delete "$INSTDIR\luoda-codex-manager.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LuodaCodex"
  DeleteRegKey HKCU "Software\LuodaCodex"
SectionEnd
