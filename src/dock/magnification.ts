/** How much a dock item grows: its tile is scaled by `scale` from the
 * middle of its screen-side edge, then moved `lift` pixels away from the
 * screen edge. The item's box widens by the same `scale`, so the
 * magnification pushes the neighbours apart instead of covering them. */
export interface Magnification {
  scale: number;
  lift: number;
}

export type MagnifyRole = "hovered" | "neighbor";

/** Single source of the magnification values (DESIGN.md "Layout",
 * Ampliação B1): Dock.tsx hands them to Dock.css per item (`--s`,
 * `--lift`), and the layout sent to Rust is computed from them, so the hold
 * region and the window size match what the dock paints. */
export const MAGNIFICATION: Record<MagnifyRole, Magnification> = {
  hovered: { scale: 1.4, lift: 6 },
  neighbor: { scale: 1.15, lift: 0 },
};

const REST: Magnification = { scale: 1, lift: 0 };

/** The role of the item at `index` while the item at `magnified` is
 * magnified: the item itself, a direct neighbour, or none. */
export function magnifyRole(
  index: number,
  magnified: number | null,
): MagnifyRole | null {
  if (magnified === null) return null;
  if (index === magnified) return "hovered";
  return Math.abs(index - magnified) === 1 ? "neighbor" : null;
}

/** The magnification of the item at `index`; scale 1 without lift when it
 * has no role. */
export function magnificationOf(
  index: number,
  magnified: number | null,
): Magnification {
  const role = magnifyRole(index, magnified);
  return role ? MAGNIFICATION[role] : REST;
}
