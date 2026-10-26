; Runs inside the installer that Tauri builds (bundle.windows.nsis.installerHooks).
; The entry that starts the DockMax with Windows (spec 0008) belongs to the
; user, in their own Run key: it goes away with the program, so a removed
; DockMax does not leave a broken startup entry behind. The user's data
; (settings, notes) stays unless they ask the uninstaller to delete it.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "DockMax"
  ; The copy of the program that watches the taskbar while the DockMax runs
  ; (ADR 0009) lives in the local data folder, outside the install folder.
  Delete "$LOCALAPPDATA\dev.maxsueleinstein.dockmax\dockmax-guard.exe"
!macroend
