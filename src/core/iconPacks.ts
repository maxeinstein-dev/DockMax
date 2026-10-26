import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";

/* The icon packs in the packs folder (spec 0007). Rust reads the folder and
 * checks every pack; the page only lists them, opens the folder and takes
 * the icons through `app_icon`, never a path. */

/** Why a pack is refused; must match `PackError` in
 * `src-tauri/src/domain/icon_pack.rs`. */
export type PackErrorCode =
  | "manifestMissing"
  | "manifestTooBig"
  | "manifestInvalid"
  | "noName"
  | "textTooLong"
  | "badAccent"
  | "tooManyEntries"
  | "badKey"
  | "badPath";

export const PACK_ERROR_CODES = Object.keys({
  manifestMissing: 0,
  manifestTooBig: 0,
  manifestInvalid: 0,
  noName: 0,
  textTooLong: 0,
  badAccent: 0,
  tooManyEntries: 0,
  badKey: 0,
  badPath: 0,
} satisfies Record<PackErrorCode, 0>) as PackErrorCode[];

/** A folder of the packs folder: a pack, or why it can't be used. */
export interface IconPackInfo {
  /** The name of the folder: what `dock.iconPack` holds. */
  id: string;
  name: string;
  author: string;
  version: string;
  description: string;
  /** `#RRGGBB`. */
  accent: string | null;
  iconCount: number;
  /** How many of the pinned programs the pack has an image for, of `total`. */
  covers: number;
  total: number;
  error: PackErrorCode | null;
}

export const ICON_PACK_COMMANDS = {
  list: "list_icon_packs",
  reload: "reload_icon_packs",
  openFolder: "open_icon_packs_folder",
} as const;

/** Event every window receives when the icons the dock shows may have
 * changed (a pack was added, removed or replaced). */
export const ICONS_CHANGED_EVENT = "icons-changed";

/** Reads the packs folder again and lists what is in it. */
export function listIconPacks(): Promise<IconPackInfo[]> {
  return invoke<IconPackInfo[]>(ICON_PACK_COMMANDS.list).catch(
    (error: unknown) => {
      log.warn(`icons: list failed: ${errorKind(error)}`);
      throw error;
    },
  );
}

/** Like `listIconPacks`, and the dock reads its icons again whatever
 * changed: an image of a pack may have been replaced. */
export function reloadIconPacks(): Promise<IconPackInfo[]> {
  return invoke<IconPackInfo[]>(ICON_PACK_COMMANDS.reload).catch(
    (error: unknown) => {
      log.warn(`icons: reload failed: ${errorKind(error)}`);
      throw error;
    },
  );
}

/** Opens the packs folder in File Explorer. */
export function openIconPacksFolder(): Promise<void> {
  return invoke<void>(ICON_PACK_COMMANDS.openFolder).catch((error: unknown) => {
    log.warn(`icons: open folder failed: ${errorKind(error)}`);
    throw error;
  });
}
