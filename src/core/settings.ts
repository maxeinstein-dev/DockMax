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

/** Event every window receives with the full settings after a change. */
export const SETTINGS_CHANGED_EVENT = "settings-changed";

/** The settings in use. */
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/** Saves `settings` and returns the ones now in use. Takes the full object:
 * Rust rejects a missing or unknown field, unlike when it reads the file.
 * Rejects with a message safe to show when they are invalid or can't be
 * saved; nothing changes then. */
export function updateSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("update_settings", { settings });
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
