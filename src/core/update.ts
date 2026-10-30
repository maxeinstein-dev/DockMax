import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { errorKind, log } from "./log";

/* The automatic update (spec 0019 G3). Rust looks for a new version a
 * moment after the app opens and every 6 hours; Sobre shows what it found
 * and installs it only on Atualizar. */

/** What Sobre shows: `UpdateState` in `src-tauri/src/domain/update.rs`. */
export type UpdateState =
  | { state: "none" }
  | { state: "available"; version: string; notes: string | null }
  | {
      state: "downloading";
      version: string;
      downloaded: number;
      total: number | null;
    }
  | { state: "installing"; version: string };

/** Sent to every window when the state changes. */
export const UPDATE_CHANGED_EVENT = "update-changed";

export function updateStatus(): Promise<UpdateState> {
  return invoke("update_status");
}

/** Looks now; rejects with the reason it could not. */
export function checkForUpdate(): Promise<UpdateState> {
  return invoke("check_for_update");
}

/** Downloads and installs: the app closes as Sair closes it and the
 * installer opens it again, so on success it never answers. Rejects with
 * the reason when something fails before that. */
export function installUpdate(): Promise<void> {
  return invoke("install_update");
}

/** The download's progress in whole percent, or null without a size. */
export function percent(
  downloaded: number,
  total: number | null,
): number | null {
  if (!total || total <= 0) return null;
  return Math.min(100, Math.floor((downloaded / total) * 100));
}

/** The update state, kept current by Rust's event. */
export function useUpdate(): [UpdateState, (state: UpdateState) => void] {
  const [update, setUpdate] = useState<UpdateState>({ state: "none" });

  useEffect(() => {
    let active = true;
    // An event may arrive before the first read answers; that read is
    // older then, so it is dropped.
    let changed = false;

    const unlisten = listen<UpdateState>(UPDATE_CHANGED_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setUpdate(event.payload);
    }).catch((error: unknown) => {
      log.warn(`update: listen failed: ${errorKind(error)}`);
      return undefined;
    });

    updateStatus().then(
      (current) => {
        if (active && !changed && typeof current?.state === "string") {
          setUpdate(current);
        }
      },
      (error: unknown) => {
        if (active) log.warn(`update: read failed: ${errorKind(error)}`);
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop?.());
    };
  }, []);

  return [update, setUpdate];
}
