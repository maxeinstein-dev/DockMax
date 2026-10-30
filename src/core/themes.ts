import type { DockSettings, DockStyle } from "./settings";

/* The themes of the dock (spec 0020): a shortcut that applies, at once, a
 * style of the bar, an icon pack and the color of the mark under an open
 * program. Nothing new is saved: the theme in use is the one whose three
 * choices are the dock's now, and none when the user changed one of them
 * ("Personalizado"). */

export type ThemeId = "standard" | "neon" | "glaze" | "glass" | "midnight";

export interface Theme {
  id: ThemeId;
  style: DockStyle;
  /** The id of a bundled icon pack (`src-tauri/icon-packs/<id>`); null for
   * the programs' own icons. */
  iconPack: string | null;
  /** `#RRGGBB`; null for the style's own color. */
  indicatorColor: string | null;
}

/** The themes, in the order of their cards. */
export const THEMES: readonly Theme[] = [
  { id: "standard", style: "native", iconPack: null, indicatorColor: null },
  { id: "neon", style: "neon", iconPack: "neon", indicatorColor: null },
  { id: "glaze", style: "native", iconPack: "esmalte", indicatorColor: null },
  {
    id: "glass",
    style: "liquidGlass",
    iconPack: "vidro",
    indicatorColor: null,
  },
  {
    id: "midnight",
    style: "midnight",
    iconPack: "traco",
    indicatorColor: null,
  },
];

type ThemeChoices = Pick<DockSettings, "style" | "iconPack" | "indicatorColor">;

/** The three choices `theme` makes, as a change to the dock's settings. */
export function choicesOf(theme: Theme): ThemeChoices {
  return {
    style: theme.style,
    iconPack: theme.iconPack,
    indicatorColor: theme.indicatorColor,
  };
}

/** The theme the dock shows now: the one whose three choices are the
 * dock's; null when none is ("Personalizado"). */
export function themeOf(dock: ThemeChoices): ThemeId | null {
  const same = (a: string | null, b: string | null) =>
    (a ?? "").toUpperCase() === (b ?? "").toUpperCase();
  return (
    THEMES.find(
      (theme) =>
        theme.style === dock.style &&
        same(theme.iconPack, dock.iconPack) &&
        same(theme.indicatorColor, dock.indicatorColor),
    )?.id ?? null
  );
}
