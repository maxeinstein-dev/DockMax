import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";
import type { MoveDirection } from "./widgets";

/* The workspaces (spec 0013): each one is a dock of its own, with a name, a
 * color and the programs and widgets it shows, in order. Rust owns them;
 * Configurações edits any of them, whichever the docks show. The programs
 * and widgets of a workspace change through the apps and widgets commands,
 * which take its id (`workspace`). */

/** The most workspaces: Ctrl+Alt+1…9 reaches each one. */
export const MAX_WORKSPACES = 9;

/** The longest name, in characters. */
export const MAX_WORKSPACE_NAME = 32;

/** The colors a workspace can have, in the order new ones take them (keep
 * in step with `PALETTE` in `src-tauri/src/domain/workspaces.rs`). */
export const WORKSPACE_COLORS = [
  "#0A84FF",
  "#FF9F0A",
  "#30D158",
  "#BF5AF2",
  "#FF375F",
  "#FF453A",
  "#FFD60A",
  "#8E8E93",
] as const;

export interface Workspace {
  id: string;
  name: string;
  /** One of `WORKSPACE_COLORS`. */
  color: string;
  /** Its items in dock order: `app:<id>` and `widget:<id>`. */
  items: string[];
}

/** Every workspace in order, and the one in use (the dock used last). */
export interface WorkspacesSnapshot {
  workspaces: Workspace[];
  inUse: string | null;
}

export type WorkspaceErrorCode =
  | "notFound"
  | "limitReached"
  | "invalidName"
  | "invalidColor"
  | "lastWorkspace"
  | "unavailable"
  | "writeFailed"
  | "notReady";

export interface WorkspaceCommandError {
  code: WorkspaceErrorCode;
}

const ERROR_CODES: ReadonlySet<string> = new Set<WorkspaceErrorCode>([
  "notFound",
  "limitReached",
  "invalidName",
  "invalidColor",
  "lastWorkspace",
  "unavailable",
  "writeFailed",
  "notReady",
]);

export function isWorkspaceCommandError(
  error: unknown,
): error is WorkspaceCommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    ERROR_CODES.has(error.code)
  );
}

export const WORKSPACE_COMMANDS = {
  list: "list_workspaces",
  create: "create_workspace",
  rename: "rename_workspace",
  recolor: "recolor_workspace",
  move: "move_workspace",
  delete: "delete_workspace",
  moveAppWindows: "move_app_windows",
  moveOpenWindows: "move_open_windows",
} as const;

/** Event every window receives with a `WorkspacesSnapshot` after a change. */
export const WORKSPACES_CHANGED_EVENT = "workspaces-changed";

/** Runs `command`. A known `{ code }` from Rust passes through; any other
 * failure is logged by kind and rejects with `notReady`. */
function run<T>(command: string, args: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args).catch((error: unknown) => {
    if (isWorkspaceCommandError(error)) throw error;
    log.warn(`workspaces: a command failed: ${errorKind(error)}`);
    const notReady: WorkspaceCommandError = { code: "notReady" };
    throw notReady;
  });
}

export function listWorkspaces(): Promise<WorkspacesSnapshot> {
  return run(WORKSPACE_COMMANDS.list, {});
}

/** Makes a workspace at the end, empty, with the next free color. */
export function createWorkspace(name?: string): Promise<Workspace> {
  return run(WORKSPACE_COMMANDS.create, name === undefined ? {} : { name });
}

export function renameWorkspace(id: string, name: string): Promise<void> {
  return run(WORKSPACE_COMMANDS.rename, { id, name });
}

export function recolorWorkspace(id: string, color: string): Promise<void> {
  return run(WORKSPACE_COMMANDS.recolor, { id, color });
}

export function moveWorkspace(
  id: string,
  direction: MoveDirection,
): Promise<void> {
  return run(WORKSPACE_COMMANDS.move, { id, direction });
}

/** Deletes a workspace (never the last): the programs and widgets no other
 * workspace has go with it. */
export function deleteWorkspace(id: string): Promise<void> {
  return run(WORKSPACE_COMMANDS.delete, { id });
}

/** "Mover para <espaço>" (spec 0013 C3): the windows of the pinned program
 * `id` in the workspace in use go to `workspace`. How many moved. */
export function moveAppWindows(id: string, workspace: string): Promise<number> {
  return run(WORKSPACE_COMMANDS.moveAppWindows, { id, workspace });
}

/** The same for the open program `key` that is not pinned. */
export function moveOpenWindows(
  key: string,
  workspace: string,
): Promise<number> {
  return run(WORKSPACE_COMMANDS.moveOpenWindows, { key, workspace });
}

/** A name the user may give: one line, no spaces at the ends, 1 to
 * `MAX_WORKSPACE_NAME` characters (Rust checks it again). */
export function isValidWorkspaceName(name: string): boolean {
  const length = [...name].length;
  return (
    length >= 1 &&
    length <= MAX_WORKSPACE_NAME &&
    name.trim() === name &&
    !/\p{Cc}/u.test(name)
  );
}

/** Of `list` (programs or widgets, by `id`), those `workspace` shows, in its
 * order; the whole list when there is no workspace (they can't be read). */
export function inWorkspace<T extends { id: string }>(
  list: readonly T[],
  workspace: Workspace | null | undefined,
  kind: "app" | "widget",
): T[] {
  if (!workspace) return [...list];
  const byKey = new Map(list.map((item) => [`${kind}:${item.id}`, item]));
  return workspace.items.flatMap((key) => {
    const item = byKey.get(key);
    return item ? [item] : [];
  });
}

/** The workspaces as they are, following `workspaces-changed`. null while
 * loading, and for good when they can't be read. */
export function useWorkspaces(): WorkspacesSnapshot | null {
  const [snapshot, setSnapshot] = useState<WorkspacesSnapshot | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers; that read is older
    // then, so it is dropped.
    let changed = false;

    const unlisten = listen<WorkspacesSnapshot>(
      WORKSPACES_CHANGED_EVENT,
      (event) => {
        if (!active) return;
        changed = true;
        setSnapshot(event.payload);
      },
    );

    listWorkspaces().then(
      (current) => {
        if (active && !changed && Array.isArray(current?.workspaces)) {
          setSnapshot(current);
        }
      },
      () => {
        if (active) log.warn("workspaces: read failed");
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return snapshot;
}
