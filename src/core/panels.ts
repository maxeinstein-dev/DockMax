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

/** Event the dock receives when the open panel changes; drives the open
 * panel indicator. Must match `CHANGED_EVENT` in
 * `src-tauri/src/windows/panel.rs`. */
export const PANEL_CHANGED_EVENT = "panel-changed";

export interface PanelChanged {
  /** The panel now open, or null when none is. */
  panelId: PanelId | null;
}

/** Command the panel calls once the content `panel-show` asked for has
 * rendered; Rust shows the window only then (or after a time limit). */
export const PANEL_READY_COMMAND = "panel_ready";

export interface PanelReadyArgs {
  /** The panel whose content rendered. */
  panelId: PanelId;
}
