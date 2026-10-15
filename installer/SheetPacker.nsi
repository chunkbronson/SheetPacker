; SheetPacker installer (per-user, no admin required).
Unicode true
Name "SheetPacker"
OutFile "SheetPacker-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\SheetPacker"
RequestExecutionLevel user
SetCompressor /SOLID lzma

Icon "SheetPacker.ico"
!define MUI_ICON   "SheetPacker.ico"
!define MUI_UNICON "SheetPacker.ico"

VIProductVersion "0.2.0.0"
VIAddVersionKey "ProductName"     "SheetPacker"
VIAddVersionKey "FileDescription" "SheetPacker installer"
VIAddVersionKey "FileVersion"     "0.2.0"
VIAddVersionKey "ProductVersion"  "0.2.0"
VIAddVersionKey "LegalCopyright"  "SheetPacker"

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\SheetPacker.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Run SheetPacker"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

!define UNKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\SheetPacker"

Section "SheetPacker" SecMain
  SetOutPath "$INSTDIR"
  File "SheetPacker.exe"
  File "README.txt"

  CreateDirectory "$SMPROGRAMS\SheetPacker"
  CreateShortcut "$SMPROGRAMS\SheetPacker\SheetPacker.lnk" "$INSTDIR\SheetPacker.exe"
  CreateShortcut "$SMPROGRAMS\SheetPacker\Uninstall.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut "$DESKTOP\SheetPacker.lnk" "$INSTDIR\SheetPacker.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "${UNKEY}" "DisplayName"     "SheetPacker"
  WriteRegStr HKCU "${UNKEY}" "DisplayVersion"  "0.2.0"
  WriteRegStr HKCU "${UNKEY}" "Publisher"       "SheetPacker"
  WriteRegStr HKCU "${UNKEY}" "DisplayIcon"     "$INSTDIR\SheetPacker.exe"
  WriteRegStr HKCU "${UNKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNKEY}" "NoModify" "1"
  WriteRegStr HKCU "${UNKEY}" "NoRepair" "1"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\SheetPacker.exe"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\SheetPacker\SheetPacker.lnk"
  Delete "$SMPROGRAMS\SheetPacker\Uninstall.lnk"
  RMDir  "$SMPROGRAMS\SheetPacker"
  Delete "$DESKTOP\SheetPacker.lnk"

  DeleteRegKey HKCU "${UNKEY}"
SectionEnd
