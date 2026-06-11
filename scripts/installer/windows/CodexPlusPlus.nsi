Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."

Name "Luoda-Codex"
OutFile "${ROOT}\dist\windows\CodexPlusPlus-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Luoda-Codex"
InstallDirRegKey HKCU "Software\Luoda-Codex" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!define MUI_ICON "${ROOT}\apps\codex-plus-manager\src-tauri\icons\icon.ico"
!define MUI_UNICON "${ROOT}\apps\codex-plus-manager\src-tauri\icons\icon.ico"

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

  Delete "$DESKTOP\Luoda-Codex 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\Luoda-Codex\Luoda-Codex 绠＄悊宸ュ叿.lnk"

  CreateShortcut "$DESKTOP\Luoda-Codex.lnk" "$INSTDIR\luoda-codex.exe" "" "$INSTDIR\luoda-codex.exe"
  CreateShortcut "$DESKTOP\Luoda-Codex 管理工具.lnk" "$INSTDIR\luoda-codex-manager.exe" "" "$INSTDIR\luoda-codex-manager.exe"
  CreateDirectory "$SMPROGRAMS\Luoda-Codex"
  CreateShortcut "$SMPROGRAMS\Luoda-Codex\Luoda-Codex.lnk" "$INSTDIR\luoda-codex.exe" "" "$INSTDIR\luoda-codex.exe"
  CreateShortcut "$SMPROGRAMS\Luoda-Codex\Luoda-Codex 管理工具.lnk" "$INSTDIR\luoda-codex-manager.exe" "" "$INSTDIR\luoda-codex-manager.exe"
  CreateShortcut "$SMPROGRAMS\Luoda-Codex\卸载 Luoda-Codex.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\luoda-codex-manager.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Luoda-Codex" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex" "DisplayName" "Luoda-Codex"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex" "Publisher" "luoda2023"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex" "DisplayIcon" "$INSTDIR\luoda-codex-manager.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM luoda-codex.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM luoda-codex-manager.exe /F'
  Pop $0

  Delete "$DESKTOP\Luoda-Codex.lnk"
  Delete "$DESKTOP\Luoda-Codex 管理工具.lnk"
  Delete "$DESKTOP\Luoda-Codex 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\Luoda-Codex\Luoda-Codex.lnk"
  Delete "$SMPROGRAMS\Luoda-Codex\Luoda-Codex 管理工具.lnk"
  Delete "$SMPROGRAMS\Luoda-Codex\Luoda-Codex 绠＄悊宸ュ叿.lnk"
  Delete "$SMPROGRAMS\Luoda-Codex\卸载 Luoda-Codex.lnk"
  RMDir "$SMPROGRAMS\Luoda-Codex"

  Delete "$INSTDIR\luoda-codex.exe"
  Delete "$INSTDIR\luoda-codex-manager.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Luoda-Codex"
  DeleteRegKey HKCU "Software\Luoda-Codex"
SectionEnd
