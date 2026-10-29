; Ask Explorer to refresh cached icons so the desktop shortcut shows the
; new Cove icon right away (SHCNE_ASSOCCHANGED, SHCNF_IDLIST).
!macro NSIS_HOOK_POSTINSTALL
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
