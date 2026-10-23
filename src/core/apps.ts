import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";
import type { MoveDirection, StorageStatus, WidgetId } from "./widgets";

/* The programs pinned to the dock (spec 0006). Rust owns the list: the dock
 * only reads it (and the icons) and opens a program; every change goes
 * through the panel. */

/** The most programs the dock holds pinned. */
export const MAX_APPS = 40;

/** One pinned program. */
export interface PinnedApp {
  /** The dock's own id for the item: what the commands take. */
  id: WidgetId;
  /** The program's name in the Windows Apps folder: what opens it. */
  appId: string;
  /** The name shown for it. */
  name: string;
}

/** The pinned programs in dock order, and the state of the storage (the
 * widgets' database holds both). */
export interface AppsSnapshot {
  storage: StorageStatus;
  apps: PinnedApp[];
}

/** One program of the catalog the panel offers. */
export interface CatalogEntry {
  appId: string;
  name: string;
}

/** One entry of the menu of a program: the key names it for
 * `runAppEntry`. */
export interface MenuEntry {
  key: number;
  title: string;
}

export type MenuSectionKind = "tasks" | "category" | "recent" | "frequent";

export const MENU_SECTION_KINDS = [
  "tasks",
  "category",
  "recent",
  "frequent",
] as const satisfies readonly MenuSectionKind[];

export interface MenuSection {
  kind: MenuSectionKind;
  /** The name of the program's own category; null for the others. */
  title: string | null;
  entries: MenuEntry[];
}

/** The menu of a pinned program, as the Windows taskbar shows it (its Jump
 * List): `canReveal` says whether its file can be shown in Explorer. */
export interface AppMenu {
  sections: MenuSection[];
  canReveal: boolean;
}

export type AppErrorCode =
  | "limitReached"
  | "notFound"
  | "alreadyPinned"
  | "invalidApp"
  | "launchFailed"
  | "unavailable"
  | "writeFailed"
  | "notReady";

export const APP_ERROR_CODES: readonly AppErrorCode[] = Object.keys({
  limitReached: 0,
  notFound: 0,
  alreadyPinned: 0,
  invalidApp: 0,
  launchFailed: 0,
  unavailable: 0,
  writeFailed: 0,
  notReady: 0,
} satisfies Record<AppErrorCode, 0>) as AppErrorCode[];

const ERROR_CODES: ReadonlySet<string> = new Set(APP_ERROR_CODES);

/** How an app command rejects. Carries no id and no name. */
export interface AppCommandError {
  code: AppErrorCode;
}

export function isAppCommandError(error: unknown): error is AppCommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    ERROR_CODES.has(error.code)
  );
}

export const APP_COMMANDS = {
  list: "list_apps",
  catalog: "list_catalog",
  icon: "app_icon",
  pin: "pin_app",
  unpin: "unpin_app",
  move: "move_app",
  launch: "launch_app",
  menu: "app_menu",
  runEntry: "run_app_entry",
  reveal: "reveal_app",
} as const;

/** Event every window receives with an `AppsSnapshot` after a change. */
export const APPS_CHANGED_EVENT = "apps-changed";

/** The sizes of an icon Rust reads: the panel's list and the dock. */
export const ICON_SIZES = { list: 32, dock: 96 } as const;

/** Runs `command`. A known `{ code }` from Rust passes through; any other
 * failure has no code of its own: it is logged by kind and rejects with
 * `notReady`. */
function run<T>(
  command: string,
  args: Record<string, unknown>,
  logFailure: (error: unknown) => void,
): Promise<T> {
  return invoke<T>(command, args).catch((error: unknown) => {
    if (isAppCommandError(error)) throw error;
    logFailure(error);
    const notReady: AppCommandError = { code: "notReady" };
    throw notReady;
  });
}

/** The pinned programs in dock order. */
export function listApps(): Promise<AppsSnapshot> {
  return run(APP_COMMANDS.list, {}, (error) =>
    log.warn(`apps: list failed: ${errorKind(error)}`),
  );
}

/** Every program of the Windows Apps folder, by name. Takes about a second
 * the first time. */
export function listCatalog(): Promise<CatalogEntry[]> {
  return run(APP_COMMANDS.catalog, {}, (error) =>
    log.warn(`apps: catalog failed: ${errorKind(error)}`),
  );
}

/** Pins a program of the catalog at the end of the dock. */
export function pinApp(appId: string): Promise<PinnedApp> {
  return run(APP_COMMANDS.pin, { appId }, (error) =>
    log.warn(`apps: pin failed: ${errorKind(error)}`),
  );
}

export function unpinApp(id: WidgetId): Promise<void> {
  return run(APP_COMMANDS.unpin, { id }, (error) =>
    log.warn(`apps: unpin failed: ${errorKind(error)}`),
  );
}

export function moveApp(id: WidgetId, direction: MoveDirection): Promise<void> {
  return run(APP_COMMANDS.move, { id, direction }, (error) =>
    log.warn(`apps: move failed: ${errorKind(error)}`),
  );
}

/** Opens a pinned program. */
export function launchApp(id: WidgetId): Promise<void> {
  return run(APP_COMMANDS.launch, { id }, (error) =>
    log.warn(`apps: launch failed: ${errorKind(error)}`),
  );
}

/** The menu of a pinned program: its categories, recent items and tasks.
 * Reads the program's list from disk. */
export function appMenu(id: WidgetId): Promise<AppMenu> {
  return run(APP_COMMANDS.menu, { id }, (error) =>
    log.warn(`apps: menu failed: ${errorKind(error)}`),
  );
}

/** Runs an entry of the menu `appMenu` last returned for `id`. */
export function runAppEntry(id: WidgetId, key: number): Promise<void> {
  return run(APP_COMMANDS.runEntry, { id, key }, (error) =>
    log.warn(`apps: entry failed: ${errorKind(error)}`),
  );
}

/** Shows the program's file in File Explorer. */
export function revealApp(id: WidgetId): Promise<void> {
  return run(APP_COMMANDS.reveal, { id }, (error) =>
    log.warn(`apps: reveal failed: ${errorKind(error)}`),
  );
}

/** At most this many icons are read at once: each is a few milliseconds of
 * the shell's time, and the panel's catalog asks for a few hundred. */
const ICON_CONCURRENCY = 4;

const icons = new Map<string, Promise<string | null>>();
const waiting: (() => void)[] = [];
let reading = 0;

function pump() {
  while (reading < ICON_CONCURRENCY) {
    const start = waiting.shift();
    if (!start) return;
    reading += 1;
    start();
  }
}

/** The icon of a program as an image URL, or null when Windows has none (or
 * it can't be read). Each program and size is asked once per page; the
 * requests go a few at a time. */
export function appIcon(
  appId: string,
  size: number = ICON_SIZES.dock,
): Promise<string | null> {
  const key = `${size} ${appId}`;
  let icon = icons.get(key);
  if (icon === undefined) {
    icon = new Promise<string | null>((resolve) => {
      waiting.push(() => {
        invoke<string | null>(APP_COMMANDS.icon, { appId, size })
          .then(
            (url) => resolve(typeof url === "string" ? url : null),
            (error: unknown) => {
              log.warn(`apps: icon failed: ${errorKind(error)}`);
              resolve(null);
            },
          )
          .finally(() => {
            reading -= 1;
            pump();
          });
      });
      pump();
    });
    icons.set(key, icon);
  }
  return icon;
}

/** Forgets the icons read (for the tests). */
export function forgetIcons(): void {
  icons.clear();
  waiting.length = 0;
  reading = 0;
}

/** The pinned programs and the storage state, kept current by
 * `apps-changed`; null while loading, and for good if they can't be read (the
 * dock then has no apps). */
export function useApps(): AppsSnapshot | null {
  const [snapshot, setSnapshot] = useState<AppsSnapshot | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers; that read is older
    // then, so it is dropped.
    let changed = false;

    const unlisten = listen<AppsSnapshot>(APPS_CHANGED_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setSnapshot(event.payload);
    });

    listApps().then(
      (current) => {
        if (active && !changed && Array.isArray(current?.apps)) {
          setSnapshot(current);
        }
      },
      () => {
        if (active) log.warn("apps: read failed");
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return snapshot;
}

/** The icon of a program, as it arrives: null until it is read, and when
 * the program has none. */
export function useAppIcon(
  appId: string,
  size: number = ICON_SIZES.dock,
): string | null {
  const [icon, setIcon] = useState<{ key: string; url: string | null } | null>(
    null,
  );
  const key = `${size} ${appId}`;
  useEffect(() => {
    let active = true;
    void appIcon(appId, size).then((url) => {
      if (active) setIcon({ key, url });
    });
    return () => {
      active = false;
    };
  }, [appId, size, key]);
  return icon !== null && icon.key === key ? icon.url : null;
}
