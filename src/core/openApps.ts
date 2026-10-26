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
  /** It has a badge for its icon (the photo of a Chrome profile):
   * `useOpenBadge` reads it. */
  badge: boolean;
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
  launchOpen: "launch_open",
  closeApp: "close_app",
  closeOpen: "close_open",
  pinOpen: "pin_open",
  badge: "open_badge",
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
  /** Rust has answered at least once: before that, an empty list means
   * nothing yet. */
  loaded: boolean;
}

const NONE: OpenNow = { apps: new Map(), others: [], loaded: false };

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
    loaded: true,
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

/** Opens a new instance of the open program that is not pinned, named by
 * `key` (the middle button). Rejects when it is no longer open or could not be
 * opened. */
export function launchOpen(key: string): Promise<void> {
  return invoke<void>(OPEN_APPS_COMMANDS.launchOpen, { key }).catch(
    (error: unknown) => {
      log.warn(`apps: launch open failed: ${errorKind(error)}`);
      throw error;
    },
  );
}

/** Asks every window of the open pinned program `id` to close, as the close
 * button of each does (spec 0009 D3). Rejects with the app error code. */
export function closeApp(id: WidgetId): Promise<void> {
  return invoke<number>(OPEN_APPS_COMMANDS.closeApp, { id }).then(
    () => undefined,
    (error: unknown) => {
      log.warn(`apps: close failed: ${errorKind(error)}`);
      throw error;
    },
  );
}

/** The same for an open program that is not pinned, by its `key`. */
export function closeOpen(key: string): Promise<void> {
  return invoke<number>(OPEN_APPS_COMMANDS.closeOpen, { key }).then(
    () => undefined,
    (error: unknown) => {
      log.warn(`apps: close open failed: ${errorKind(error)}`);
      throw error;
    },
  );
}

/** Pins the open program `key` that is not pinned, at the end of the pinned
 * ones. Rejects with the app error code. */
export function pinOpen(key: string): Promise<void> {
  return invoke<unknown>(OPEN_APPS_COMMANDS.pinOpen, { key }).then(
    () => undefined,
    (error: unknown) => {
      log.warn(`apps: pin open failed: ${errorKind(error)}`);
      throw error;
    },
  );
}

const badges = new Map<string, Promise<string | null>>();

/** The badge of the open program `key` as an image URL, or null; asked once
 * per program and page. */
export function openBadge(key: string): Promise<string | null> {
  let badge = badges.get(key);
  if (badge === undefined) {
    badge = invoke<string | null>(OPEN_APPS_COMMANDS.badge, { key }).then(
      (url) => (typeof url === "string" ? url : null),
      (error: unknown) => {
        log.warn(`apps: badge failed: ${errorKind(error)}`);
        return null;
      },
    );
    badges.set(key, badge);
  }
  return badge;
}

/** Forgets the badges read (for the tests). */
export function forgetBadges(): void {
  badges.clear();
}

/** The badge of the open program `key`, as it arrives; null while it is read,
 * and for a program with none (`has` false: nothing is asked). */
export function useOpenBadge(key: string, has: boolean): string | null {
  const [badge, setBadge] = useState<{
    key: string;
    url: string | null;
  } | null>(null);
  useEffect(() => {
    if (!has) return;
    let active = true;
    void openBadge(key).then((url) => {
      if (active) setBadge({ key, url });
    });
    return () => {
      active = false;
    };
  }, [key, has]);
  return has && badge !== null && badge.key === key ? badge.url : null;
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
