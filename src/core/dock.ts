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

/** Command the dock calls whenever its layout changes (size, edge,
 * magnification, label). Replaces `set_dock_hit_region`. */
export const SET_DOCK_LAYOUT_COMMAND = "set_dock_layout";

/** What the dock occupies, in logical pixels: where it paints (as in
 * `DockHitRegion`, relative to the client area) and the size its content
 * needs (bar + magnification + label), which Rust gives the window before
 * positioning it on the edge. */
export interface DockLayout {
  bar: LogicalRect;
  hold: LogicalRect[];
  window: { width: number; height: number };
}

export interface SetDockLayoutArgs {
  layout: DockLayout;
}
