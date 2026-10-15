import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";

/* The user settings, `settings.json` v1 (spec 0002 §2.3 item 3). Must match
 * `Settings` in `src-tauri/src/domain/settings.rs`. Rust is the source of
 * truth: the windows read them with `get_settings`, apply what
 * `settings-changed` carries, and only the panel changes them. */

/** Follows the Windows light/dark setting, or forces one. */
export type Theme = "system" | "light" | "dark";

/** Screen edge the dock sits on. */
export type DockEdge = "bottom" | "top";

/** Dock background: simulated glass or opaque. Panels are always opaque. */
export type DockOpacity = "translucent" | "solid";

export interface DockSettings {
  edge: DockEdge;
  /** Icon size in logical pixels, 32–96; the size presets are values of
   * this field. */
  iconSize: number;
  opacity: DockOpacity;
}

export interface Settings {
  /** Format version; 1 in this build. */
  version: number;
  theme: Theme;
  dock: DockSettings;
}

/** Named icon sizes offered by the Settings panel; must match `SizePreset`
 * in `src-tauri/src/domain/settings.rs`. */
export type SizePreset = "small" | "medium" | "large";

/** Icon size of each preset, in logical pixels (`DESIGN.md`). */
export const SIZE_PRESETS: Readonly<Record<SizePreset, number>> = {
  small: 40,
  medium: 48,
  large: 64,
};

/** Why `update_settings` failed; must match `SettingsErrorCode` in
 * `src-tauri/src/domain/settings.rs`. Each code has a message in
 * `settings.errors` of the i18n dictionary. */
export type SettingsErrorCode =
  | "iconSizeOutOfRange"
  | "unsupportedVersion"
  | "writeFailed"
  /** The settings can't be changed in this run (no config folder), or the
   * command never ran. */
  | "notReady";

/** What `updateSettings` rejects with. No path, no value: only the code. */
export interface SettingsCommandError {
  code: SettingsErrorCode;
}

/** Every code, from a record so a new one can't be left out. */
const ERROR_CODES: ReadonlySet<string> = new Set(
  Object.keys({
    iconSizeOutOfRange: true,
    unsupportedVersion: true,
    writeFailed: true,
    notReady: true,
  } satisfies Record<SettingsErrorCode, true>),
);

/** Whether `error` is the `{ code }` the Rust command rejects with. */
export function isSettingsCommandError(
  error: unknown,
): error is SettingsCommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    ERROR_CODES.has(error.code)
  );
}

/** Event every window receives with the full settings after a change. */
export const SETTINGS_CHANGED_EVENT = "settings-changed";

/** The settings in use. */
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/** Saves `settings` and returns the ones now in use. Takes the full object:
 * Rust rejects a missing or unknown field, unlike when it reads the file.
 *
 * Always rejects with a `SettingsCommandError` when the settings are invalid
 * or can't be saved; nothing changes then. A failure before the command runs
 * (payload refused by Tauri, settings state missing) has no code of its own:
 * it is logged by kind and rejects with `notReady`. */
export function updateSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("update_settings", { settings }).catch(
    (error: unknown) => {
      if (isSettingsCommandError(error)) throw error;
      log.warn(`settings: change failed: ${errorKind(error)}`);
      const notReady: SettingsCommandError = { code: "notReady" };
      throw notReady;
    },
  );
}

/** The settings in use, kept current by `settings-changed`; null while
 * loading, and for good if they can't be read (the window then keeps its
 * defaults). */
export function useSettings(): Settings | null {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the initial read answers; the read is
    // older then, so it is dropped.
    let changed = false;

    const unlisten = listen<Settings>(SETTINGS_CHANGED_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setSettings(event.payload);
    });

    getSettings().then(
      (current) => {
        if (active && !changed) setSettings(current);
      },
      (error: unknown) => {
        if (active) log.warn(`settings: read failed: ${errorKind(error)}`);
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return settings;
}
