import { describe, expect, it } from "vitest";
import {
  isWidgetPanel,
  panelKey,
  samePanel,
  widgetPanel,
  type PanelId,
} from "./panels";

const CLOCK = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071";
const NOTE = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6075";

describe("panel IDs", () => {
  it("names a widget panel as Rust reads it", () => {
    // `{ "widget": "<id>" }`, as `PanelId::Widget` in panel.rs.
    expect(JSON.stringify(widgetPanel(CLOCK))).toBe(`{"widget":"${CLOCK}"}`);
    expect(JSON.stringify("settings" satisfies PanelId)).toBe('"settings"');
  });

  it("tells widget panels from the system ones", () => {
    expect(isWidgetPanel(widgetPanel(CLOCK))).toBe(true);
    expect(isWidgetPanel("settings")).toBe(false);
    expect(isWidgetPanel("about")).toBe(false);
  });

  it("compares panels by what they name, not by reference", () => {
    expect(samePanel(widgetPanel(CLOCK), widgetPanel(CLOCK))).toBe(true);
    expect(samePanel(widgetPanel(CLOCK), widgetPanel(NOTE))).toBe(false);
    expect(samePanel("settings", "settings")).toBe(true);
    expect(samePanel("settings", "about")).toBe(false);
    expect(samePanel("settings", widgetPanel(CLOCK))).toBe(false);
    expect(samePanel(widgetPanel(CLOCK), "about")).toBe(false);
    expect(samePanel(null, null)).toBe(true);
    expect(samePanel(null, "about")).toBe(false);
    expect(samePanel(widgetPanel(CLOCK), null)).toBe(false);
  });

  it("gives each panel its own key", () => {
    const keys = [
      "settings",
      "about",
      widgetPanel(CLOCK),
      widgetPanel(NOTE),
    ].map((id) => panelKey(id as PanelId));
    expect(keys).toEqual([
      "settings",
      "about",
      `widget:${CLOCK}`,
      `widget:${NOTE}`,
    ]);
  });
});
