; Green's Windows installer: one user's, one folder, no elevation.
;
; What this writes is the install folder, a Start menu shortcut and the
; uninstall entry, all per user, and the uninstaller takes every one of them
; away again — a leftover key is a bug.
;
; 1.0.0 deliberately writes nothing under HKCU\Software\Classes: a `.md`
; association was never verified end to end, and half-cleaned associations are
; the kind of residue nobody finds later. See later.md.
;
; release.yml fills @VERSION@ and @OUTFILE@ and runs makensis from the folder
; that holds green.exe, fonts/ and the licences.

Unicode true
!define APP "Green"
!define COMPANY "Green"
!define EXE "green.exe"
!define VERSION "@VERSION@"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Green"

!include "MUI2.nsh"

Name "${APP} ${VERSION}"
OutFile "@OUTFILE@"
; The default a user keeps: their own programs folder, no UAC prompt.
InstallDir "$LOCALAPPDATA\Programs\${APP}"
InstallDirRegKey HKCU "Software\${APP}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "CompanyName" "${COMPANY}"
VIAddVersionKey "FileDescription" "Green, an offline Markdown notes editor"
VIAddVersionKey "LegalCopyright" "GPL-3.0-or-later"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

Section "Green" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "green.exe"
  File /r "fonts"
  File /r "LICENSES"
  File "LICENSE"
  File "THIRD-PARTY-NOTICES.txt"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APP}"
  CreateShortcut "$SMPROGRAMS\${APP}\${APP}.lnk" "$INSTDIR\${EXE}"

  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${COMPANY}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE}"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  WriteRegStr HKCU "Software\${APP}" "InstallDir" "$INSTDIR"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD-PARTY-NOTICES.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir /r "$INSTDIR\fonts"
  RMDir /r "$INSTDIR\LICENSES"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APP}\${APP}.lnk"
  RMDir "$SMPROGRAMS\${APP}"

  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\${APP}"
SectionEnd
