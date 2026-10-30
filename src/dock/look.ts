import type {
  DockSettings,
  DockStyle,
  Indicator,
  StyleTweaks,
} from "../core/settings";

/* What the dock bar looks like once the style and the user's changes to it
 * are put together (spec 0005 §3.2). The settings keep only what the user
 * changed; the rest is each style's own look, here. */

/** The parts of a style the user can change. */
export interface StyleLook {
  /** The outline around the bar. */
  border: boolean;
  /** How round the bar is, 0–100 % of the most it can be (a pill). */
  corners: number;
  /** A mirrored image under each tile. */
  reflections: boolean;
}

/** The look each style has before any change; the CSS of the style in
 * `Dock.css` is drawn for these values. */
export const STYLE_LOOKS: Readonly<Record<DockStyle, StyleLook>> = {
  native: { border: true, corners: 65, reflections: false },
  liquidGlass: { border: true, corners: 100, reflections: false },
  midnight: { border: true, corners: 25, reflections: false },
  rocket: { border: true, corners: 25, reflections: false },
  aquaShelf: { border: false, corners: 0, reflections: true },
  neon: { border: true, corners: 70, reflections: false },
  paper: { border: false, corners: 40, reflections: false },
  floating: { border: false, corners: 40, reflections: true },
  // The skins: the image is the bar, with no border or corners to change.
  retro: { border: false, corners: 0, reflections: false },
  classic98: { border: false, corners: 0, reflections: false },
  phosphor: { border: false, corners: 0, reflections: false },
  tactical: { border: false, corners: 0, reflections: false },
  medieval: { border: false, corners: 0, reflections: false },
  hunter: { border: false, corners: 0, reflections: false },
};

/** The look a style has when `tweaks` is applied over its own. */
export function styleLookOf(
  style: DockStyle,
  tweaks: StyleTweaks | undefined,
): StyleLook {
  const own = STYLE_LOOKS[style];
  return {
    border: tweaks?.border ?? own.border,
    corners: tweaks?.corners ?? own.corners,
    reflections: tweaks?.reflections ?? own.reflections,
  };
}

/** Everything the bar's CSS reads. */
export interface DockLook extends StyleLook {
  style: DockStyle;
  indicator: Indicator;
  /** `#RRGGBB`, or null for the style's own color. */
  indicatorColor: string | null;
}

/** The look of the dock for these settings. */
export function dockLookOf(
  dock: Pick<DockSettings, "style" | "tweaks" | "indicator" | "indicatorColor">,
): DockLook {
  return {
    ...styleLookOf(dock.style, dock.tweaks[dock.style]),
    style: dock.style,
    indicator: dock.indicator,
    indicatorColor: dock.indicatorColor,
  };
}
