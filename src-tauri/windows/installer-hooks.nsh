!macro NSIS_HOOK_PREINSTALL
  ; The sidecar is transient and read-only. Stop an older copy before replacement.
  nsExec::ExecToLog 'taskkill /F /IM forgeos-mcp.exe'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'taskkill /F /IM forgeos-mcp.exe'
  ; The version-pinned Tauri template passes /FORGEOS_UPGRADE=1 during replacement.
  ; A normal Control Panel uninstall has no marker and performs owned cleanup.
  ${GetParameters} $R0
  ${GetOptions} $R0 "/FORGEOS_UPGRADE=" $R1
  ${If} $R1 != "1"
    nsExec::ExecToLog '"$INSTDIR\forgeos.exe" --cleanup-owned-codex-mcp'
  ${EndIf}
!macroend
