import {
  DEFAULT_HOVER_SCALE,
  type DockSettings,
  type HoverEffect,
} from "../core/settings";

/** How much a dock item grows: its tile is scaled by `scale` from the
 * middle of its screen-side edge, then moved `lift` pixels away from it. The
 * item's box widens by the same `scale`, so the growth pushes the neighbours
 * apart instead of covering them. */
export interface Magnification {
  scale: number;
  lift: number;
}

export type MagnifyRole = "hovered" | "neighbor";

/** What an item does under the pointer, and how much the "scale" effect
 * grows it (a ratio: 1.1 is 110 %). */
export interface HoverConfig {
  effect: HoverEffect;
  scale: number;
}

/** The "magnify" effect: single source of its values (DESIGN.md "Layout",
 * Ampliação B1). Dock.tsx hands them to Dock.css per item (`--s`, `--lift`),
 * and the layout sent to Rust is computed from them, so the hold region and
 * the window size match what the dock paints. */
export const MAGNIFICATION: Record<MagnifyRole, Magnification> = {
  hovered: { scale: 1.4, lift: 6 },
  neighbor: { scale: 1.15, lift: 0 },
};

/** What a caller that does not choose gets: the magnification of Fase 1. */
export const MAGNIFY_HOVER: HoverConfig = { effect: "magnify", scale: 1.4 };

/** The hover of the dock before its settings are read: the settings'
 * defaults. */
export const DEFAULT_HOVER: HoverConfig = {
  effect: "scale",
  scale: DEFAULT_HOVER_SCALE / 100,
};

/** The hover the settings ask for; the default while they are not read. */
export function hoverConfigOf(
  dock: Pick<DockSettings, "hover" | "hoverScale"> | undefined,
): HoverConfig {
  return dock
    ? { effect: dock.hover, scale: dock.hoverScale / 100 }
    : DEFAULT_HOVER;
}

const REST: Magnification = { scale: 1, lift: 0 };

/** The role of the item at `index` while the item at `magnified` is hovered:
 * the item itself, a direct neighbour (only with "magnify"), or none. */
export function magnifyRole(
  index: number,
  magnified: number | null,
  effect: HoverEffect = "magnify",
): MagnifyRole | null {
  if (magnified === null || effect === "none") return null;
  if (index === magnified) return "hovered";
  if (effect === "scale") return null;
  return Math.abs(index - magnified) === 1 ? "neighbor" : null;
}

/** The growth of the hovered item at its peak: what the window must leave
 * room for. */
export function peakMagnification(hover: HoverConfig): Magnification {
  switch (hover.effect) {
    case "none":
      return REST;
    case "scale":
      return { scale: hover.scale, lift: 0 };
    case "magnify":
      return MAGNIFICATION.hovered;
  }
}

/** The growth of the item at `index`; scale 1 without lift when it has no
 * role. */
export function magnificationOf(
  index: number,
  magnified: number | null,
  hover: HoverConfig = MAGNIFY_HOVER,
): Magnification {
  const role = magnifyRole(index, magnified, hover.effect);
  if (role === null) return REST;
  return role === "hovered" ? peakMagnification(hover) : MAGNIFICATION[role];
}
