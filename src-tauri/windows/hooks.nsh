; Runs inside the installer that Tauri builds (bundle.windows.nsis.installerHooks).
; The entry that starts the Dockyard with Windows (spec 0008) belongs to the
; user, in their own Run key: it goes away with the program, so a removed
; Dockyard does not leave a broken startup entry behind. The user's data
; (settings, notes) stays unless they ask the uninstaller to delete it.
;
; The Dockyard was called DockMax: the entry and the copy of the watchdog of
; that name go too, in case the program never ran after the rename (it is
; the first run that takes them over). The identifier did not change, so the
; data folder is the same.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Dockyard"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "DockMax"
  ; The copy of the program that watches the taskbar while the Dockyard runs
  ; (ADR 0009) lives in the local data folder, outside the install folder.
  Delete "$LOCALAPPDATA\dev.maxsueleinstein.dockmax\dockyard-guard.exe"
  Delete "$LOCALAPPDATA\dev.maxsueleinstein.dockmax\dockmax-guard.exe"
!macroend
