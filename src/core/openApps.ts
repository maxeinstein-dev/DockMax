import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";
import type { WidgetId } from "./widgets";

/* The pinned programs that are open (spec 0009). Rust follows the windows
 * and keeps the handles; the dock only hears who is open and whether the user
 * is in it, and asks to bring a program to the front by the id of its pinned
 * item. */

/** A pinned program with windows open. */
export interface OpenApp {
  /** The pinned item's id: `PinnedApp.id`. */
  id: WidgetId;
  /** How many windows it has. */
  count: number;
  /** The user is working in one of them. */
  active: boolean;
}

export const OPEN_APPS_COMMANDS = {
  list: "list_open_apps",
  activate: "activate_app",
} as const;

/** Event every window receives with the `OpenApp[]` after a change. */
export const OPEN_APPS_CHANGED_EVENT = "open-windows-changed";

/** Brings the pinned program `id` to the front, minimizes it if the user is
 * in it, or passes to its next window. Resolves `false` when it has no
 * window (or the call failed): the caller opens it instead. */
export function activateApp(id: WidgetId): Promise<boolean> {
  return invoke<boolean>(OPEN_APPS_COMMANDS.activate, { id }).then(
    (done) => done === true,
    (error: unknown) => {
      log.warn(`apps: activate failed: ${errorKind(error)}`);
      return false;
    },
  );
}

function byId(open: readonly OpenApp[]): ReadonlyMap<string, OpenApp> {
  return new Map(open.map((app) => [app.id, app]));
}

const NONE: ReadonlyMap<string, OpenApp> = new Map();

/** The open programs by the id of their pinned item, kept current by
 * `open-windows-changed`; empty while loading and if they can't be read (the
 * dock then shows no program as open). */
export function useOpenApps(): ReadonlyMap<string, OpenApp> {
  const [open, setOpen] = useState<ReadonlyMap<string, OpenApp>>(NONE);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers; that read is older
    // then, so it is dropped.
    let changed = false;

    const unlisten = listen<OpenApp[]>(OPEN_APPS_CHANGED_EVENT, (event) => {
      if (!active || !Array.isArray(event.payload)) return;
      changed = true;
      setOpen(byId(event.payload));
    });

    invoke<OpenApp[]>(OPEN_APPS_COMMANDS.list).then(
      (current) => {
        if (active && !changed && Array.isArray(current)) {
          setOpen(byId(current));
        }
      },
      (error: unknown) => {
        log.warn(`apps: open programs read failed: ${errorKind(error)}`);
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return open;
}
