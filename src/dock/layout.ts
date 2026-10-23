import type { DockSize } from "../core/appearance";
import type { DockLayout } from "../core/dock";
import type { LogicalRect } from "../core/panels";
import type { DockEdge } from "../core/settings";
import {
  MAGNIFY_HOVER,
  magnificationOf,
  peakMagnification,
  type HoverConfig,
} from "./magnification";

/* The dock's geometry, computed instead of measured: the rects sent to Rust
 * and the label position are the final ones, never a transition caught
 * halfway. Dock.css lays the dock out by the same rules, from the same
 * numbers (tokens.css size presets, pinned by layout.test.ts):
 *
 * - the bar is `icon + 2 × pad` tall, flush with the window edge that faces
 *   the screen edge and centered across the window;
 * - each item is `icon × scale` wide (the magnification pushes its
 *   neighbours apart), `gap` apart, `pad` inside the bar;
 * - a tile grows from the middle of its screen-side edge and moves `lift`
 *   away from it;
 * - the label pill sits `TIP_GAP` beyond the tile, on the far side. */

export interface SizeGeometry {
  icon: number;
  pad: number;
  gap: number;
}

/** DESIGN.md "Layout" table; equal to the `data-size` blocks of
 * tokens.css. */
export const SIZE_GEOMETRY: Readonly<Record<DockSize, SizeGeometry>> = {
  s: { icon: 40, pad: 6, gap: 6 },
  m: { icon: 48, pad: 7, gap: 7 },
  l: { icon: 64, pad: 9, gap: 9 },
};

/** Distance between a tile and its label pill (DESIGN.md "Tooltip"). */
export const TIP_GAP = 10;

/** Height of the label pill: a 16 px line plus 4 px of padding above and
 * below (Dock.css `.dock-tip`). */
export const TIP_HEIGHT = 24;

/** Transparent room the window keeps beyond the label and on both sides of
 * the widest bar, for the pill's ring and shadow and the bar's shadow. */
export const ROOM = 8;

/** Width a separator takes between two groups, its margins included
 * (`margin: 0 gap × 0.4` around a 1 px line), besides the flex gap that
 * follows it. */
export function separatorWidth(gap: number): number {
  return 1 + gap * 0.8;
}

/** What the dock shows: its size, the number of items in each group (a
 * separator stands between groups) and what an item does under the pointer
 * (the magnification of Fase 1 when left out). */
export interface DockShape {
  size: DockSize;
  groups: readonly number[];
  hover?: HoverConfig;
}

interface Row {
  /** Outer width of the bar. */
  width: number;
  /** Each item's left edge (from the bar's outer left edge), width, and
   * magnification. */
  items: { x: number; width: number; scale: number; lift: number }[];
}

/** Lays the items out along the bar while the item at `magnified` is
 * magnified (null: at rest). */
function row(
  { size, groups, hover = MAGNIFY_HOVER }: DockShape,
  magnified: number | null,
): Row {
  const { icon, pad, gap } = SIZE_GEOMETRY[size];
  const items: Row["items"] = [];
  let x = pad;
  let first = true;
  groups.forEach((count, group) => {
    if (count <= 0) return;
    if (group > 0 && !first) x += separatorWidth(gap) + gap;
    for (let k = 0; k < count; k++) {
      const { scale, lift } = magnificationOf(items.length, magnified, hover);
      const width = icon * scale;
      items.push({ x, width, scale, lift });
      x += width + gap;
      first = false;
    }
  });
  const width = items.length ? x - gap + pad : 2 * pad;
  return { width, items };
}

/** Rounds up to whole pixels, ignoring floating-point noise (67.2 comes
 * out as 67.19999999999999). */
function ceil(value: number): number {
  return Math.ceil(value - 1e-6);
}

export interface WindowSizeInput extends DockShape {
  /** False with "reduce motion": nothing is ever magnified. */
  motion: boolean;
  /** Measured width of each item's label pill, in item order. */
  labelWidths: readonly number[];
}

/**
 * The window the dock needs, whatever is magnified or labelled: the bar,
 * the magnified tile and the label pill beyond it, plus `ROOM`; and, across,
 * the widest the bar gets (the magnification pushes it wider) or the
 * farthest a label reaches past it, plus `ROOM` on each side. The same for
 * both edges (the dock is mirrored on top).
 */
export function dockWindowSize({
  motion,
  labelWidths,
  ...shape
}: WindowSizeInput): DockLayout["window"] {
  const { icon, pad } = SIZE_GEOMETRY[shape.size];
  const peak = peakMagnification(shape.hover ?? MAGNIFY_HOVER);
  // Nothing grows with "reduce motion" or without a hover effect.
  const grows = motion && peak.scale !== 1;
  const { scale, lift } = grows ? peak : { scale: 1, lift: 0 };
  const tileReach = pad + icon * scale + lift;
  const height = tileReach + TIP_GAP + TIP_HEIGHT + ROOM;

  // At rest any item may show its label (keyboard focus, or a panel open);
  // magnified, only the magnified item does.
  const count = row(shape, null).items.length;
  const states: (number | null)[] = [null];
  if (grows) for (let i = 0; i < count; i++) states.push(i);

  let half = 0;
  for (const state of states) {
    const { width, items } = row(shape, state);
    half = Math.max(half, width / 2);
    items.forEach((item, index) => {
      if (state !== null && index !== state) return;
      const center = item.x + item.width / 2 - width / 2;
      half = Math.max(half, Math.abs(center) + (labelWidths[index] ?? 0) / 2);
    });
  }

  return { width: ceil(2 * half + 2 * ROOM), height: ceil(height) };
}

export interface PlacementInput extends DockShape {
  edge: DockEdge;
  /** The magnified item, or null at rest. */
  magnified: number | null;
  /** The window's client area as it is now. */
  viewport: { width: number; height: number };
}

export interface Placement {
  /** The visible bar. */
  bar: LogicalRect;
  /** Each item's box (the button). */
  items: LogicalRect[];
  /** Each item's tile, magnified. */
  tiles: LogicalRect[];
  /** Each item's magnification scale. */
  scales: number[];
}

/** Where the bar, the items and the tiles end up in the current window
 * once the magnification settles. */
export function placeDock({
  edge,
  magnified,
  viewport,
  ...shape
}: PlacementInput): Placement {
  const { icon, pad } = SIZE_GEOMETRY[shape.size];
  const { width, items } = row(shape, magnified);
  const height = icon + 2 * pad;
  const bar = {
    x: (viewport.width - width) / 2,
    y: edge === "bottom" ? viewport.height - height : 0,
    width,
    height,
  };
  const top = bar.y + pad;
  return {
    bar,
    items: items.map((item) => ({
      x: bar.x + item.x,
      y: top,
      width: item.width,
      height: icon,
    })),
    tiles: items.map(({ x, width: side, lift }) => ({
      x: bar.x + x,
      y: edge === "bottom" ? top + icon - lift - side : top + lift,
      width: side,
      height: side,
    })),
    scales: items.map((item) => item.scale),
  };
}

/** Where the label pill of `tile` is anchored: the middle of its edge
 * nearest the tile, `TIP_GAP` beyond the tile on the far side. */
export function tipAnchor(
  tile: LogicalRect,
  edge: DockEdge,
): { x: number; y: number } {
  return {
    x: tile.x + tile.width / 2,
    y: edge === "bottom" ? tile.y - TIP_GAP : tile.y + tile.height + TIP_GAP,
  };
}

/** What the dock reports to Rust: the bar, the magnified tiles to keep the
 * mouse on, and the window it needs. */
export function dockLayout(
  placement: Placement,
  window: DockLayout["window"],
): DockLayout {
  return {
    bar: placement.bar,
    hold: placement.tiles.filter((_, index) => placement.scales[index] !== 1),
    window,
  };
}
