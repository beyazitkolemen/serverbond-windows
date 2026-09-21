!include "WinVer.nsh"

; Rust and the bundled Go-based services require NT 10.0 or later.
; Run before copying application files, including silent updates.
!macro NSIS_HOOK_PREINSTALL
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "ServerBond requires Windows 10 or Windows Server 2016 (64-bit), or later." /SD IDOK
    SetErrorLevel 1633
    Quit
  ${EndIf}
!macroend
