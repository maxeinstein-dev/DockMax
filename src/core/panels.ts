import type { WidgetId } from "./widgets";

/** The system panels: always on the dock, whatever the widgets. */
export type SystemPanelId = "settings" | "about" | "tray";

/** The panel of a widget instance. */
export interface WidgetPanelId {
  widget: WidgetId;
}

/** The menu of a pinned program (spec 0006 §6): `app` is the pinned
 * item's id. */
export interface AppPanelId {
  app: WidgetId;
}

/** The menu of an open program that is not pinned (spec 0009 D2 and D3):
 * `open` is the opaque key Rust gave it. */
export interface OpenPanelId {
  open: string;
}

/** Panels the dock can open; must match `PanelId` in
 * `src-tauri/src/windows/panel.rs`. In JSON: `"settings"`, `"about"`,
 * `"tray"`, `{ "widget": "<id>" }`, `{ "app": "<id>" }` or
 * `{ "open": "<key>" }`. Rust refuses
 * (`toggle_panel` fails, nothing opens) an ID that isn't an instance, a
 * pinned program or an open program the dock has. An object, so compare two with
 * `samePanel`, never with `===`. */
export type PanelId = SystemPanelId | WidgetPanelId | AppPanelId | OpenPanelId;

/** The item of the open program `key`. */
export function otherPanel(key: string): OpenPanelId {
  return { open: key };
}

/** Whether `id` names an open program that is not pinned. */
export function isOpenPanel(id: PanelId): id is OpenPanelId {
  return typeof id === "object" && "open" in id;
}

/** The panel of the instance `id`. */
export function widgetPanel(id: WidgetId): WidgetPanelId {
  return { widget: id };
}

/** The menu panel of the pinned program `id`. */
export function appPanel(id: WidgetId): AppPanelId {
  return { app: id };
}

/** Whether `id` is a widget instance's panel. */
export function isWidgetPanel(id: PanelId): id is WidgetPanelId {
  return typeof id === "object" && "widget" in id;
}

/** Whether `id` is the menu of a pinned program. */
export function isAppPanel(id: PanelId): id is AppPanelId {
  return typeof id === "object" && "app" in id;
}

/** Whether `a` and `b` name the same panel. */
export function samePanel(a: PanelId | null, b: PanelId | null): boolean {
  if (a === null || b === null) return a === b;
  if (isWidgetPanel(a)) return isWidgetPanel(b) && a.widget === b.widget;
  if (isAppPanel(a)) return isAppPanel(b) && a.app === b.app;
  if (isOpenPanel(a)) return isOpenPanel(b) && a.open === b.open;
  return a === b;
}

/** A string that names the panel, for React keys and `resetKey`s:
 * `settings`, `about`, `widget:<id>` or `app:<id>`. Never shown and never
 * sent. */
export function panelKey(id: PanelId): string {
  if (isWidgetPanel(id)) return `widget:${id.widget}`;
  if (isAppPanel(id)) return `app:${id.app}`;
  if (isOpenPanel(id)) return `open:${id.open}`;
  return id;
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

/** Command the panel calls once the content `panel-show` asked for has
 * rendered and been laid out: once per `panel-show`, switching panels
 * included. Rust sizes the panel window to `size`, places it so the visible
 * card (the window minus `inset` on every side) sits 8 px from the dock bar,
 * and only then shows it (or after a time limit, if this never comes). */
export const PANEL_READY_COMMAND = "panel_ready";

/** Sent by the panel's page once it listens for `panel-show` (spec 0011 H2):
 * the window is made on demand, and a show asked for while it was being made
 * waits for this. */
export const PANEL_MOUNTED_COMMAND = "panel_mounted";

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
