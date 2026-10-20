import type { WidgetId } from "./widgets";

/** The system panels: always on the dock, whatever the widgets. */
export type SystemPanelId = "settings" | "about";

/** The panel of a widget instance. */
export interface WidgetPanelId {
  widget: WidgetId;
}

/** Panels the dock can open; must match `PanelId` in
 * `src-tauri/src/windows/panel.rs`. In JSON: `"settings"`, `"about"` or
 * `{ "widget": "<id>" }`. Rust refuses (`toggle_panel` fails, nothing
 * opens) a widget ID that isn't an instance the dock has. An object, so
 * compare two with `samePanel`, never with `===`. */
export type PanelId = SystemPanelId | WidgetPanelId;

/** The panel of the instance `id`. */
export function widgetPanel(id: WidgetId): WidgetPanelId {
  return { widget: id };
}

/** Whether `id` is a widget instance's panel. */
export function isWidgetPanel(id: PanelId): id is WidgetPanelId {
  return typeof id === "object";
}

/** Whether `a` and `b` name the same panel. */
export function samePanel(a: PanelId | null, b: PanelId | null): boolean {
  if (a === null || b === null) return a === b;
  if (isWidgetPanel(a)) return isWidgetPanel(b) && a.widget === b.widget;
  return a === b;
}

/** A string that names the panel, for React keys and `resetKey`s:
 * `settings`, `about` or `widget:<id>`. Never shown and never sent. */
export function panelKey(id: PanelId): string {
  return isWidgetPanel(id) ? `widget:${id.widget}` : id;
}

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
