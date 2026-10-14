import type { LogicalRect } from "./panels";

/** Where the dock paints, relative to its client area: the visible bar and
 * the final rects of the magnified items. Must match `HitRegion` in the
 * Rust side. */
export interface DockHitRegion {
  bar: LogicalRect;
  hold: LogicalRect[];
}

/** Event the dock receives when Rust stops capturing the mouse: the page
 * gets no more pointer events, so it clears its hover state. */
export const DOCK_RELEASED_EVENT = "dock-released";
