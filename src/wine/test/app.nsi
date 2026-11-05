; A small installer like the ones on the web: 32-bit stub, Start Menu shortcuts, uninstaller.
Unicode true
Name "MAN Test App"
OutFile "${OUT}"
InstallDir "$PROGRAMFILES\MAN Test App"
RequestExecutionLevel admin
Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles
Section
  SetOutPath "$INSTDIR"
  File "${APP}"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\MAN Test App"
  CreateShortcut "$SMPROGRAMS\MAN Test App\MAN Test App.lnk" "$INSTDIR\pick64.exe" "open"
  CreateShortcut "$SMPROGRAMS\MAN Test App\Uninstall MAN Test App.lnk" "$INSTDIR\uninstall.exe"
SectionEnd
Section "Uninstall"
  Delete "$INSTDIR\pick64.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\MAN Test App\MAN Test App.lnk"
  Delete "$SMPROGRAMS\MAN Test App\Uninstall MAN Test App.lnk"
  RMDir "$SMPROGRAMS\MAN Test App"
SectionEnd
