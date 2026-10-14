/** Panels the dock can open; must match `PanelId` in the Rust side. */
export type PanelId = "settings" | "about";

/** Rectangle in CSS pixels, relative to the window's client area. */
export interface LogicalRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Event the panel window receives right before it is shown. */
export const PANEL_SHOW_EVENT = "panel-show";

export interface PanelShowPayload {
  panelId: PanelId;
}
