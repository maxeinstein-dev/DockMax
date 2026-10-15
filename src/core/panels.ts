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
  /** Numbers this show; sent back in `panel_ready`, so Rust can tell the
   * answer to this show from a late answer to an earlier one. */
  wait: number;
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
 * rendered and been laid out: once per `panel-show`, switching panels
 * included. Rust sizes the panel window to `size`, places it so the visible
 * card (the window minus `inset` on every side) sits 8 px from the dock bar,
 * and only then shows it (or after a time limit, if this never comes). */
export const PANEL_READY_COMMAND = "panel_ready";

/** Width and height in logical (CSS) pixels. */
export interface LogicalSize {
  width: number;
  height: number;
}

export interface PanelReadyArgs {
  /** The panel whose content rendered; never the one shown before it. */
  panelId: PanelId;
  /** The `wait` of the `panel-show` this answers. */
  wait: number;
  /** Full size the panel window must have, in logical pixels: the visible
   * card plus `inset` on every side, rounded up to whole pixels. */
  size: LogicalSize;
  /** Transparent margin around the visible card, the same on every side, in
   * logical pixels: room for the card's shadow. */
  inset: number;
}
