import { describe, expect, it } from "vitest";
import {
  appPanel,
  isAppPanel,
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

  it("names the menu of a program as Rust reads it", () => {
    // `{ "app": "<id>" }`, as `PanelId::App` in panel.rs.
    expect(JSON.stringify(appPanel(CLOCK))).toBe(`{"app":"${CLOCK}"}`);
  });

  it("tells the menu of a program from a widget and from the system panels", () => {
    expect(isAppPanel(appPanel(CLOCK))).toBe(true);
    expect(isAppPanel(widgetPanel(CLOCK))).toBe(false);
    expect(isAppPanel("settings")).toBe(false);
    expect(isWidgetPanel(appPanel(CLOCK))).toBe(false);
  });

  it("does not take a program's menu for the widget with the same id", () => {
    expect(samePanel(appPanel(CLOCK), appPanel(CLOCK))).toBe(true);
    expect(samePanel(appPanel(CLOCK), appPanel(NOTE))).toBe(false);
    expect(samePanel(appPanel(CLOCK), widgetPanel(CLOCK))).toBe(false);
    expect(samePanel(widgetPanel(CLOCK), appPanel(CLOCK))).toBe(false);
    expect(samePanel(appPanel(CLOCK), "settings")).toBe(false);
    expect(samePanel(null, appPanel(CLOCK))).toBe(false);
    expect(panelKey(appPanel(CLOCK))).toBe(`app:${CLOCK}`);
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
