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

/** An open program that is not pinned (spec 0009 D2). */
export interface OpenOther {
  /** What `activateOpen` takes: opaque, never a path. */
  key: string;
  /** What `app_icon` takes for its icon. */
  appId: string;
  name: string;
  count: number;
  active: boolean;
}

/** Who is open: the pinned programs, and the others. */
export interface OpenSnapshot {
  apps: OpenApp[];
  others: OpenOther[];
}

export const OPEN_APPS_COMMANDS = {
  list: "list_open_apps",
  activate: "activate_app",
  activateOpen: "activate_open",
} as const;

/** Event every window receives with the `OpenSnapshot` after a change. */
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

/** What the dock keeps of who is open: the pinned programs by the id of
 * their item, and the others in the order they are shown. */
export interface OpenNow {
  apps: ReadonlyMap<string, OpenApp>;
  others: readonly OpenOther[];
}

const NONE: OpenNow = { apps: new Map(), others: [] };

function validSnapshot(value: unknown): value is OpenSnapshot {
  return (
    typeof value === "object" &&
    value !== null &&
    "apps" in value &&
    Array.isArray(value.apps) &&
    "others" in value &&
    Array.isArray(value.others)
  );
}

function nowOf(snapshot: OpenSnapshot): OpenNow {
  return {
    apps: new Map(snapshot.apps.map((app) => [app.id, app])),
    others: snapshot.others,
  };
}

/** Brings the open program that is not pinned, named by `key`, to the front,
 * minimizes it, or passes to its next window. Resolves `false` when it is
 * not open any more (or the call failed). */
export function activateOpen(key: string): Promise<boolean> {
  return invoke<boolean>(OPEN_APPS_COMMANDS.activateOpen, { key }).then(
    (done) => done === true,
    (error: unknown) => {
      log.warn(`apps: activate open failed: ${errorKind(error)}`);
      return false;
    },
  );
}

/** The open programs, kept current by `open-windows-changed`; none while
 * loading and if they can't be read (the dock then shows no program as
 * open). */
export function useOpenApps(): OpenNow {
  const [open, setOpen] = useState<OpenNow>(NONE);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers; that read is older
    // then, so it is dropped.
    let changed = false;

    const unlisten = listen<OpenSnapshot>(OPEN_APPS_CHANGED_EVENT, (event) => {
      if (!active || !validSnapshot(event.payload)) return;
      changed = true;
      setOpen(nowOf(event.payload));
    });

    invoke<OpenSnapshot>(OPEN_APPS_COMMANDS.list).then(
      (current) => {
        if (active && !changed && validSnapshot(current)) {
          setOpen(nowOf(current));
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
