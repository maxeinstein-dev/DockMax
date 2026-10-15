import type { LogicalRect, PanelId } from "./panels";

/** Event the dock receives when Rust stops capturing the mouse: the page
 * gets no more pointer events, so it clears its hover state. */
export const DOCK_RELEASED_EVENT = "dock-released";

/** Command the dock calls whenever its layout changes (size, edge,
 * magnification, label). Replaces `set_dock_hit_region`. */
export const SET_DOCK_LAYOUT_COMMAND = "set_dock_layout";

/** What the dock occupies, in logical pixels: where it paints, relative to
 * the client area (the visible bar and the final rects of the magnified
 * icons, which keep the mouse captured), and the size its content needs
 * (bar + magnification + label), which Rust gives the window before
 * positioning it on the edge. The bar sits on the window edge that faces
 * the screen edge; magnification and label grow toward the other side. */
export interface DockLayout {
  bar: LogicalRect;
  hold: LogicalRect[];
  window: { width: number; height: number };
}

export interface SetDockLayoutArgs {
  layout: DockLayout;
}

/** Event the dock receives when something outside it (the tray's
 * "Configurações") asks for a panel: the dock opens it anchored to its item,
 * as a click would, unless it is already open. */
export const DOCK_REQUEST_PANEL_EVENT = "dock-request-panel";

export interface DockRequestPanel {
  panelId: PanelId;
}

/** The tallest bar and dock window Rust accepts in `set_dock_layout`
 * (`MAX_BAR_HEIGHT` and `MAX_WINDOW_HEIGHT` in
 * src-tauri/src/windows/dock.rs, derived there from the largest icon size).
 * A layout beyond them is rejected and the dock stops resizing, so
 * layout.test.ts keeps every preset within them and a Rust test keeps both
 * sides equal. */
export const MAX_DOCK_BAR_HEIGHT = 126;
export const MAX_DOCK_WINDOW_HEIGHT = 198;
