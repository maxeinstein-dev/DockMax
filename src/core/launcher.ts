import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";

/* The launcher (spec 0012 L1): a native window Rust opens with Alt+Space.
 * Configurações only switches it, says when another program holds the
 * hotkey, and forgets what it opened. */

/** What Configurações shows about the launcher; must match `LauncherStatus`
 * in `src-tauri/src/commands/launcher.rs`. */
export interface LauncherStatus {
  /** Another program holds Alt+Space: the launcher opens only from the
   * menus ("Buscar apps…"). */
  hotkeyTaken: boolean;
}

/** Event the windows receive when the hotkey's state changes. */
export const LAUNCHER_STATUS_EVENT = "launcher-status";

export const LAUNCHER_STATUS_COMMAND = "launcher_status";
export const CLEAR_LAUNCHER_HISTORY_COMMAND = "clear_launcher_history";

/** Forgets what the launcher opened. Resolves whether it did; failures are
 * logged by kind. */
export function clearLauncherHistory(): Promise<boolean> {
  return invoke<void>(CLEAR_LAUNCHER_HISTORY_COMMAND).then(
    () => true,
    (error: unknown) => {
      log.warn(`launcher: clearing the history failed: ${errorKind(error)}`);
      return false;
    },
  );
}

/** The launcher's status, kept current by `launcher-status`; null while
 * loading, and for good if it can't be read. */
export function useLauncherStatus(): LauncherStatus | null {
  const [status, setStatus] = useState<LauncherStatus | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers: the read is older
    // then, and is dropped.
    let changed = false;

    const unlisten = listen<LauncherStatus>(LAUNCHER_STATUS_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setStatus(event.payload);
    });

    invoke<LauncherStatus>(LAUNCHER_STATUS_COMMAND).then(
      (current) => {
        if (active && !changed) setStatus(current);
      },
      (error: unknown) => {
        if (active)
          log.warn(`launcher: status read failed: ${errorKind(error)}`);
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return status;
}
