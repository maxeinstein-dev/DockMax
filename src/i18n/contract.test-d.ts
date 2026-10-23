/* Type-level test of the locale contract, checked by `pnpm typecheck` (and
 * `tsc` in `pnpm build`); Vitest does not run it. Each `@ts-expect-error`
 * must match a real error, or the check fails. */
import { ptBR, type Messages } from "./pt-BR";

/** A complete locale may use any text. */
export const otherText: Messages = {
  dock: {
    label: "Dock",
    items: { settings: "Settings", about: "About", tray: "System tray" },
    status: { calendar: "Clock and calendar" },
  },
  panel: {
    titles: { settings: "Settings", about: "About", tray: "System tray" },
    failed: "This panel could not be shown.",
    closeHint: { key: "Esc", action: "closes" },
  },
  settings: {
    groups: {
      theme: "Theme",
      edge: "Position",
      size: "Size",
      opacity: "Dock opacity",
      taskbar: "Windows taskbar",
    },
    theme: { system: "System", light: "Light", dark: "Dark" },
    edge: { bottom: "Bottom", top: "Top" },
    size: { small: "Small", medium: "Medium", large: "Large" },
    opacity: { translucent: "Translucent", solid: "Solid" },
    taskbar: { visible: "Visible", autohide: "Automatic", hidden: "Hidden" },
    look: {
      title: "Dock look",
      style: "Style",
      styles: {
        native: "Native",
        liquidGlass: "Liquid Glass",
        midnight: "Midnight",
        rocket: "Rocket",
        aquaShelf: "Aqua shelf",
        neon: "Neon",
        canary: "Canary",
        paper: "Paper",
        floating: "Floating",
      },
      tweaks: "Customize this style",
      border: "Border",
      corners: "Corners",
      cornersHint: "How round the bar is",
      reflections: "Reflections",
      reflectionsHint: "A mirrored image under each icon",
      restore: "Restore",
      restoreHint: "Discards the changes of this style",
      hover: "Hover effect",
      hoverOptions: { none: "None", scale: "Scale", magnify: "Magnify" },
      hoverScale: "Hover scale",
      indicator: "Indicator",
      indicatorOptions: { dot: "Dot", dash: "Dash" },
      indicatorColor: "Indicator color",
      indicatorDefault: "Default",
      colors: {
        "#FFFFFF": "White",
        "#E8E8EC": "Very light gray",
        "#C7C7CC": "Light gray",
        "#8E8E93": "Gray",
        "#2C3A5A": "Blue graphite",
        "#0B1020": "Night blue",
        "#000000": "Black",
        "#0A84FF": "Blue",
        "#64D2FF": "Light blue",
        "#00E5FF": "Cyan",
        "#30D158": "Green",
        "#1E8A3C": "Dark green",
        "#FFD60A": "Yellow",
        "#FF9F0A": "Orange",
        "#FF453A": "Red",
        "#FF2D95": "Pink",
        "#BF5AF2": "Purple",
        "#9DB4FF": "Lilac",
      },
    },
    footer: "Saved automatically",
    errors: {
      iconSizeOutOfRange: "That size is not accepted.",
      lookOutOfRange: "That look setting is not accepted.",
      unsupportedVersion: "These settings are from another version.",
      writeFailed: "The change could not be saved.",
      notReady: "Settings are not available right now.",
    },
  },
  tray: {
    empty: "No program has put an icon here yet.",
    emptyHint: "Some only show up after they are opened again.",
    unnamed: "Unnamed program",
    footer: "Right button opens the program's menu",
  },
  about: {
    description: "Windows 11 dock, built with Tauri.",
    labels: { version: "Version", license: "License", code: "Code" },
    licenseValue: "Personal use",
    repository: "github.com/maxeinstein-dev/DockMax",
    footer: "Independent project",
  },
  apps: {
    title: "Dock apps",
    count: "{count} of {max}",
    unavailable: "Unavailable",
    listLabel: "Apps in dock order",
    empty: "No apps in the dock. Search for one below.",
    add: "Add an app",
    limit: "Limit of {max} apps",
    searchLabel: "Search installed apps",
    searchPlaceholder: "Search by name",
    catalogLabel: "Installed apps",
    loading: "Reading the installed apps…",
    noMatch: "No app with that name.",
    catalogFailed: "The installed apps could not be read.",
    retry: "Try again",
    pin: "Pin {name}",
    pinned: "{name} (pinned)",
    moveUp: "Move {name} up",
    moveDown: "Move {name} down",
    remove: "Remove {name} from the dock",
    footer: "The order is the dock's",
    footerUnavailable: "Settings and About keep working",
    unavailableNotice: {
      title: "Dock apps are unavailable.",
      detail: "Detail.",
    },
    errors: {
      limitReached: "a",
      notFound: "a",
      alreadyPinned: "a",
      invalidApp: "a",
      launchFailed: "a",
      unavailable: "a",
      writeFailed: "a",
      notReady: "a",
    },
  },
  widgets: ptBR.widgets,
};

/** A locale missing a key does not compile. */
export const missingKey: Messages = {
  ...ptBR,
  panel: {
    ...ptBR.panel,
    // @ts-expect-error -- `about` is missing.
    titles: { settings: "Settings" },
  },
};

/** A locale missing the message of an error code does not compile. */
export const missingErrorMessage: Messages = {
  ...ptBR,
  settings: {
    ...ptBR.settings,
    // @ts-expect-error -- `notReady` is missing.
    errors: {
      iconSizeOutOfRange: "That size is not accepted.",
      lookOutOfRange: "That look setting is not accepted.",
      unsupportedVersion: "These settings are from another version.",
      writeFailed: "The change could not be saved.",
    },
  },
};

/** `value` without `key`, typed so. */
function without<T extends object, K extends keyof T>(
  value: T,
  key: K,
): Omit<T, K> {
  const copy: Partial<T> = { ...value };
  delete copy[key];
  return copy as Omit<T, K>;
}

/** A locale missing the message of a widget error code does not compile. */
export const missingWidgetErrorMessage: Messages = {
  ...ptBR,
  widgets: {
    ...ptBR.widgets,
    // @ts-expect-error -- `notReady` is missing.
    errors: without(ptBR.widgets.errors, "notReady"),
  },
};

/** A locale missing the name of a widget kind does not compile. */
export const missingKindName: Messages = {
  ...ptBR,
  widgets: {
    ...ptBR.widgets,
    // @ts-expect-error -- `files` is missing.
    kinds: without(ptBR.widgets.kinds, "files"),
  },
};

/** A locale missing a whole area does not compile. */
// @ts-expect-error -- `settings` is missing.
export const missingArea: Messages = {
  dock: ptBR.dock,
  panel: ptBR.panel,
  about: ptBR.about,
  apps: ptBR.apps,
  widgets: ptBR.widgets,
};

/** A locale with a key the PT-BR messages do not have does not compile. */
export const extraKey: Messages = {
  ...ptBR,
  panel: {
    ...ptBR.panel,
    // @ts-expect-error -- unknown key.
    subtitle: "Extra",
  },
};
