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
  /** The name that survives a restart (`path:<program>#<number>`), what the
   * settings keep the icons shown in the dock by; null when the program's
   * file can't be read. */
  id: string | null;
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

/** Command the tray panel calls when an icon dragged out of it is let go,
 * with the point on the screen in physical pixels: on the dock, the dock
 * shows the icon itself from then on. Answers whether it does. */
export const TRAY_TO_DOCK_COMMAND = "tray_to_dock";

/** How far the pointer goes before a press on an icon becomes a drag. */
export const TRAY_DRAG_THRESHOLD = 6;

export type TrayButton = "left" | "right";

export interface TrayClickArgs {
  key: string;
  button: TrayButton;
}

/** The icons to show: the ones the programs did not ask to hide. */
export function visibleIcons(icons: readonly TrayIcon[]): TrayIcon[] {
  return icons.filter((icon) => !icon.hidden);
}

/** The icons the tray panel shows: the visible ones the dock does not show
 * itself (`inDock`, their ids). */
export function panelIcons(
  icons: readonly TrayIcon[],
  inDock: readonly string[],
): TrayIcon[] {
  return visibleIcons(icons).filter(
    (icon) => icon.id === null || !inDock.includes(icon.id),
  );
}

/** `inDock` with `id` added at its end (`on`) or taken out. */
export function withInDock(
  inDock: readonly string[],
  id: string,
  on: boolean,
): string[] {
  const others = inDock.filter((kept) => kept !== id);
  return on ? [...others, id] : others;
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

/** An icon dragged out of the panel and let go at `screen` (CSS pixels on
 * the screen, `PointerEvent.screenX/Y`): Rust keeps it in the dock when it was
 * let go there. Failures are logged by kind. */
export function dropOnDock(
  key: string,
  screen: { x: number; y: number },
  scale: number,
): Promise<boolean> {
  return invoke<boolean>(TRAY_TO_DOCK_COMMAND, {
    key,
    x: Math.round(screen.x * scale),
    y: Math.round(screen.y * scale),
  }).catch((error: unknown) => {
    log.warn(`tray: drop failed: ${errorKind(error)}`);
    return false;
  });
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
        if (active && !changed) setIcons(Array.isArray(current) ? current : []);
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
