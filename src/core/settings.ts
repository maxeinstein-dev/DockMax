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

/** The look of the dock bar (spec 0005 §3.2); must match `DockStyle` in
 * `src-tauri/src/domain/settings.rs`. */
export type DockStyle =
  | "native"
  | "liquidGlass"
  | "midnight"
  | "rocket"
  | "aquaShelf"
  | "neon"
  | "canary"
  | "paper"
  | "floating";

/** Every style, in the order the Settings panel lists them. */
export const DOCK_STYLES: readonly DockStyle[] = [
  "native",
  "liquidGlass",
  "midnight",
  "rocket",
  "aquaShelf",
  "neon",
  "canary",
  "paper",
  "floating",
];

/** What the user changed in one style; a field left out keeps the style's
 * own value. */
export interface StyleTweaks {
  border?: boolean;
  /** How round the bar is, `MIN_CORNERS`–`MAX_CORNERS` percent. */
  corners?: number;
  reflections?: boolean;
}

export const MIN_CORNERS = 0;
export const MAX_CORNERS = 100;

/** What an item does under the pointer. */
export type HoverEffect = "none" | "scale" | "magnify";

/** Growth of the hovered item with `scale`, in percent. */
export const MIN_HOVER_SCALE = 100;
export const MAX_HOVER_SCALE = 160;
export const DEFAULT_HOVER_SCALE = 110;

/** The mark under an open item. */
export type Indicator = "dot" | "dash";

/** Colors offered for the indicator, `#RRGGBB`; `null` is the style's own
 * (spec 0004 §3.1: the default plus fifteen). */
export const INDICATOR_COLORS = [
  "#FFFFFF",
  "#E8E8EC",
  "#C7C7CC",
  "#8E8E93",
  "#2C3A5A",
  "#0B1020",
  "#000000",
  "#0A84FF",
  "#64D2FF",
  "#00E5FF",
  "#30D158",
  "#1E8A3C",
  "#FFD60A",
  "#FF9F0A",
  "#FF453A",
  "#FF2D95",
  "#BF5AF2",
  "#9DB4FF",
] as const;

/** One of the colors offered. */
export type IndicatorColor = (typeof INDICATOR_COLORS)[number];

/** How a pinned program's icon sits in its tile; must match `IconShape` in
 * `src-tauri/src/domain/settings.rs`. */
export type IconShape = "plate" | "uniform" | "original";

/** What the middle button (the wheel) does on a program of the dock; must
 * match `MiddleClick` in `src-tauri/src/domain/settings.rs`. */
export type MiddleClick = "newInstance" | "none";

/** What a program's icon does while it opens; must match `OpenAnimation`. */
export type OpenAnimation = "bounce" | "pulse" | "shake";

/** What the dock does with the Windows taskbar (ADR 0009); must match
 * `TaskbarMode` in `src-tauri/src/domain/settings.rs`. */
export type TaskbarMode = "visible" | "autohide" | "hidden";

/** Most tray icons the dock shows itself, beside the battery, the network
 * and the volume; `MAX_TRAY_IN_DOCK` in `src-tauri/src/domain/settings.rs`. */
export const MAX_TRAY_IN_DOCK = 6;

/** Distance between the dock and the screen edge, in logical pixels. */
export const MAX_BOTTOM_SPACE = 64;
export const DEFAULT_BOTTOM_SPACE = 2;
/** The times of a dock hiding on its own, in milliseconds: on the screen
 * edge before it comes up, and away before it goes down. */
export const DEFAULT_AUTO_HIDE_REVEAL = 300;
export const MAX_AUTO_HIDE_REVEAL = 2000;
export const DEFAULT_AUTO_HIDE_DELAY = 700;
export const MAX_AUTO_HIDE_DELAY = 5000;

/** The look fields of `DockSettings` before the user changes anything;
 * equal to `DockSettings::default()` in `src-tauri/src/domain/settings.rs`. */
export const DEFAULT_LOOK = {
  style: "native",
  tweaks: {},
  hover: "scale",
  hoverScale: DEFAULT_HOVER_SCALE,
  indicator: "dot",
  indicatorColor: null,
  iconShape: "plate",
  iconPack: null,
  openAnimation: "bounce",
  showOpenApps: true,
  middleClick: "newInstance",
  taskbar: "hidden",
  everyMonitor: true,
  autoHide: false,
  autoHideReveal: DEFAULT_AUTO_HIDE_REVEAL,
  autoHideDelay: DEFAULT_AUTO_HIDE_DELAY,
  bottomSpace: DEFAULT_BOTTOM_SPACE,
} as const satisfies Pick<
  DockSettings,
  | "style"
  | "tweaks"
  | "hover"
  | "hoverScale"
  | "indicator"
  | "indicatorColor"
  | "iconShape"
  | "iconPack"
  | "openAnimation"
  | "showOpenApps"
  | "middleClick"
  | "taskbar"
  | "everyMonitor"
  | "autoHide"
  | "autoHideReveal"
  | "autoHideDelay"
  | "bottomSpace"
>;

export interface DockSettings {
  edge: DockEdge;
  /** Icon size in logical pixels, 32–96; the size presets are values of
   * this field. */
  iconSize: number;
  opacity: DockOpacity;
  style: DockStyle;
  /** Per-style changes; a style without an entry keeps its own look. */
  tweaks: Partial<Record<DockStyle, StyleTweaks>>;
  hover: HoverEffect;
  /** Growth of the hovered item with `scale`, in percent. */
  hoverScale: number;
  indicator: Indicator;
  /** `#RRGGBB`, or null for the style's own color. */
  indicatorColor: string | null;
  iconShape: IconShape;
  /** The icon pack every program wears (spec 0007): the id of a folder of
   * the packs folder, or null for the icons Windows gives. */
  iconPack: string | null;
  openAnimation: OpenAnimation;
  /** Whether the open programs that are not pinned show on the dock, after
   * the widgets (spec 0009 D2). */
  showOpenApps: boolean;
  /** What the middle button does on a program of the dock. */
  middleClick: MiddleClick;
  /** The tray icons the dock shows itself, by their lasting `id`
   * (`TrayIcon.id`); the others are behind the arrow. Rust leaves the field
   * out while it is empty. */
  trayInDock?: string[];
  taskbar: TaskbarMode;
  /** A dock on every monitor, each holding its own edge, or only on the
   * primary one (spec 0010 D4b). */
  everyMonitor: boolean;
  /** The dock goes off the screen when the pointer leaves it and comes back
   * at the screen edge, giving its band to the programs. */
  autoHide: boolean;
  /** Milliseconds on the screen edge before it comes up,
   * 0–`MAX_AUTO_HIDE_REVEAL`. */
  autoHideReveal: number;
  /** Milliseconds away before it goes down, 0–`MAX_AUTO_HIDE_DELAY`. */
  autoHideDelay: number;
  /** Distance between the dock and the screen edge, in logical pixels,
   * 0–`MAX_BOTTOM_SPACE`. */
  bottomSpace: number;
}

export interface Settings {
  /** Format version; 1 in this build. */
  version: number;
  theme: Theme;
  dock: DockSettings;
  /** Whether the DockMax opens when the user signs in to Windows. */
  launchAtLogin: boolean;
  /** Whether Alt+Space opens the launcher (spec 0012); the menus open it
   * either way. */
  launcher: boolean;
  /** The launcher's web search by keyword (spec 0012 L2); the first is the
   * default search. */
  launcherKeywords: WebKeyword[];
}

/** A keyword of the launcher's web search; must match `WebKeyword` in
 * `src-tauri/src/domain/launcher_web.rs`. */
export interface WebKeyword {
  /** What is typed before the term (`g`). */
  keyword: string;
  /** The search's name (`Google`). */
  name: string;
  /** The address, with `%s` where the term goes. */
  template: string;
}

/** The limits of the keywords, as Rust checks them (in characters, not
 * UTF-16 units). */
export const MAX_KEYWORDS = 20;
export const MAX_KEYWORD = 16;
export const MAX_KEYWORD_NAME = 40;
export const MAX_KEYWORD_TEMPLATE = 500;

/** What is wrong with a keyword's field, if anything. */
export type KeywordProblem =
  | "keywordEmpty"
  | "keywordSpaces"
  | "keywordTooLong"
  | "keywordRepeated"
  | "nameEmpty"
  | "nameTooLong"
  | "templateNotHttps"
  | "templateNoTerm"
  | "templateSpaces"
  | "templateTooLong";

export interface KeywordProblems {
  keyword?: KeywordProblem;
  name?: KeywordProblem;
  template?: KeywordProblem;
}

const chars = (text: string) => [...text].length;
// Any control character, as Rust's `char::is_control`.
// eslint-disable-next-line no-control-regex
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

/** Why Rust would refuse `draft` among `others` (the other keywords), field
 * by field; empty when it would keep it. Mirrors `launcher_web::is_valid`. */
export function keywordProblems(
  draft: WebKeyword,
  others: readonly WebKeyword[],
): KeywordProblems {
  const problems: KeywordProblems = {};
  const word = draft.keyword;
  if (chars(word) === 0) problems.keyword = "keywordEmpty";
  else if (/\s/.test(word) || CONTROL.test(word))
    problems.keyword = "keywordSpaces";
  else if (chars(word) > MAX_KEYWORD) problems.keyword = "keywordTooLong";
  else if (
    others.some((other) => other.keyword.toLowerCase() === word.toLowerCase())
  )
    problems.keyword = "keywordRepeated";

  if (draft.name.trim() === "" || CONTROL.test(draft.name))
    problems.name = "nameEmpty";
  else if (chars(draft.name) > MAX_KEYWORD_NAME) problems.name = "nameTooLong";

  const template = draft.template;
  const host = template.startsWith("https://")
    ? (template.slice("https://".length).split(/[/?#]/)[0] ?? "")
    : "";
  if (host === "" || host.includes("%s"))
    problems.template = "templateNotHttps";
  else if (/\s/.test(template) || CONTROL.test(template))
    problems.template = "templateSpaces";
  else if (!template.includes("%s")) problems.template = "templateNoTerm";
  else if (chars(template) > MAX_KEYWORD_TEMPLATE)
    problems.template = "templateTooLong";
  return problems;
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
  /** A value of the dock's look is outside its range. */
  | "lookOutOfRange"
  /** A keyword of the launcher's web search is malformed or repeated. */
  | "launcherKeywordsInvalid"
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
    lookOutOfRange: true,
    launcherKeywordsInvalid: true,
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
