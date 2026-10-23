/* Type-level test of the locale contract, checked by `pnpm typecheck` (and
 * `tsc` in `pnpm build`); Vitest does not run it. Each `@ts-expect-error`
 * must match a real error, or the check fails. */
import { ptBR, type Messages } from "./pt-BR";

/** A complete locale may use any text. */
export const otherText: Messages = {
  dock: {
    label: "Dock",
    items: { settings: "Settings", about: "About" },
  },
  panel: {
    titles: { settings: "Settings", about: "About" },
    failed: "This panel could not be shown.",
    closeHint: { key: "Esc", action: "closes" },
  },
  settings: {
    groups: {
      theme: "Theme",
      edge: "Position",
      size: "Size",
      opacity: "Dock opacity",
    },
    theme: { system: "System", light: "Light", dark: "Dark" },
    edge: { bottom: "Bottom", top: "Top" },
    size: { small: "Small", medium: "Medium", large: "Large" },
    opacity: { translucent: "Translucent", solid: "Solid" },
    footer: "Saved automatically",
    errors: {
      iconSizeOutOfRange: "That size is not accepted.",
      unsupportedVersion: "These settings are from another version.",
      writeFailed: "The change could not be saved.",
      notReady: "Settings are not available right now.",
    },
  },
  about: {
    description: "Windows 11 dock, built with Tauri.",
    labels: { version: "Version", license: "License", code: "Code" },
    licenseValue: "Personal use",
    repository: "github.com/maxeinstein-dev/DockMax",
    footer: "Independent project",
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
