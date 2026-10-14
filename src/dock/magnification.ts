import type { LogicalRect } from "../core/panels";

/** How much a dock item grows: `scale` from its bottom center
 * (`transform-origin: center bottom` in Dock.css), then raised `lift`
 * pixels away from the screen edge. */
export interface Magnification {
  scale: number;
  lift: number;
}

export type MagnifyRole = "hovered" | "neighbor";

/** Single source of the magnification values: Dock.css reads them through
 * `MAGNIFICATION_VARIABLES`, and the hold region sent to Rust is computed
 * from them, so the region matches what the dock paints. */
export const MAGNIFICATION: Record<MagnifyRole, Magnification> = {
  hovered: { scale: 1.4, lift: 6 },
  neighbor: { scale: 1.15, lift: 0 },
};

/** CSS custom properties set on the bar for Dock.css. */
export const MAGNIFICATION_VARIABLES: Record<string, string> = {
  "--dock-hovered-scale": String(MAGNIFICATION.hovered.scale),
  "--dock-hovered-lift": String(MAGNIFICATION.hovered.lift),
  "--dock-neighbor-scale": String(MAGNIFICATION.neighbor.scale),
  "--dock-neighbor-lift": String(MAGNIFICATION.neighbor.lift),
};

/** The role of the item at `index` while the item at `hovered` is hovered:
 * the hovered item itself, a direct neighbour, or none. */
export function magnifyRole(
  index: number,
  hovered: number | null,
): MagnifyRole | null {
  if (hovered === null) return null;
  if (index === hovered) return "hovered";
  return Math.abs(index - hovered) === 1 ? "neighbor" : null;
}

/** Where an item laid out at `rect` is painted once its magnification
 * finishes: `translateY(-lift) scale(scale)` around its bottom center.
 * Computed from the layout, not measured, so it never catches a transition
 * halfway. */
export function magnifiedRect(
  rect: LogicalRect,
  { scale, lift }: Magnification,
): LogicalRect {
  const width = rect.width * scale;
  const height = rect.height * scale;
  const bottom = rect.y + rect.height - lift;
  return {
    x: rect.x + (rect.width - width) / 2,
    y: bottom - height,
    width,
    height,
  };
}

/** Final rects of the magnified items, given every item's layout rect in
 * order. Empty while nothing is hovered. */
export function holdRects(
  items: LogicalRect[],
  hovered: number | null,
): LogicalRect[] {
  return items.flatMap((rect, index) => {
    const role = magnifyRole(index, hovered);
    return role ? [magnifiedRect(rect, MAGNIFICATION[role])] : [];
  });
}
