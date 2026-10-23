import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";

/* The notification area, hosted by the dock while the Windows taskbar is
 * hidden (ADR 0009, Fase E). Rust collects the icons programs send; the
 * panel lists them and clicks them. */

/** An icon of the notification area; must match `TrayIcon` in
 * `src-tauri/src/commands/tray.rs`. */
export interface TrayIcon {
  /** Names the icon for a click. */
  key: string;
  /** The program's tooltip; may be empty. */
  tip: string;
  /** A PNG as a `data:` URL, or null when the image could not be read. */
  image: string | null;
  /** The program asked not to show it. */
  hidden: boolean;
  /** Whether it takes clicks. */
  clickable: boolean;
}

/** Event the windows receive with the full list when it changes. */
export const TRAY_CHANGED_EVENT = "tray-changed";

export const LIST_TRAY_COMMAND = "list_tray";
export const TRAY_CLICK_COMMAND = "tray_click";

export type TrayButton = "left" | "right";

export interface TrayClickArgs {
  key: string;
  button: TrayButton;
}

/** The icons to show: the ones the programs did not ask to hide. */
export function visibleIcons(icons: readonly TrayIcon[]): TrayIcon[] {
  return icons.filter((icon) => !icon.hidden);
}

/** Clicks an icon as the taskbar would. Failures are logged by kind; the
 * panel closes either way. */
export function clickIcon(key: string, button: TrayButton): Promise<void> {
  const args: TrayClickArgs = { key, button };
  return invoke<void>(TRAY_CLICK_COMMAND, { ...args }).catch(
    (error: unknown) => {
      log.warn(`tray: click failed: ${errorKind(error)}`);
    },
  );
}

/** The icons in the notification area, kept current by `tray-changed`;
 * null while loading. */
export function useTray(): TrayIcon[] | null {
  const [icons, setIcons] = useState<TrayIcon[] | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers: the read is older
    // then, and is dropped.
    let changed = false;

    const unlisten = listen<TrayIcon[]>(TRAY_CHANGED_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setIcons(event.payload);
    });

    invoke<TrayIcon[]>(LIST_TRAY_COMMAND).then(
      (current) => {
        if (active && !changed) setIcons(current);
      },
      (error: unknown) => {
        if (active) {
          log.warn(`tray: read failed: ${errorKind(error)}`);
          setIcons((current) => current ?? []);
        }
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return icons;
}
