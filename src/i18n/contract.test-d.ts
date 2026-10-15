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
    underConstruction: "Under construction.",
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
    repository: "github.com/maxeinstein-dev/DockMax",
    footer: "Independent project",
  },
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

/** A locale missing a whole area does not compile. */
// @ts-expect-error -- `settings` is missing.
export const missingArea: Messages = {
  dock: ptBR.dock,
  panel: ptBR.panel,
  about: ptBR.about,
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
