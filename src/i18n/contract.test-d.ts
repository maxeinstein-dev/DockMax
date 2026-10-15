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
  },
};

/** A locale missing a key does not compile. */
export const missingKey: Messages = {
  dock: ptBR.dock,
  panel: {
    // @ts-expect-error -- `about` is missing.
    titles: { settings: "Settings" },
    underConstruction: "Under construction.",
  },
};

/** A locale missing a whole area does not compile. */
// @ts-expect-error -- `panel` is missing.
export const missingArea: Messages = {
  dock: ptBR.dock,
};

/** A locale with a key the PT-BR messages do not have does not compile. */
export const extraKey: Messages = {
  dock: ptBR.dock,
  panel: {
    ...ptBR.panel,
    // @ts-expect-error -- unknown key.
    subtitle: "Extra",
  },
};
