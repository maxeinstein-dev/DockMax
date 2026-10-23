import { describe, expect, it } from "vitest";
import { DEFAULT_LOOK, DOCK_STYLES, type DockSettings } from "../core/settings";
import { STYLE_LOOKS, dockLookOf, styleLookOf } from "./look";

describe("styleLookOf", () => {
  it("is the style's own look without changes", () => {
    for (const style of DOCK_STYLES) {
      expect(styleLookOf(style, undefined)).toEqual(STYLE_LOOKS[style]);
      expect(styleLookOf(style, {})).toEqual(STYLE_LOOKS[style]);
    }
  });

  it("puts each change over the style's own value, field by field", () => {
    expect(styleLookOf("neon", { corners: 10 })).toEqual({
      ...STYLE_LOOKS.neon,
      corners: 10,
    });
    expect(styleLookOf("floating", { border: true })).toEqual({
      ...STYLE_LOOKS.floating,
      border: true,
    });
    // A change back to false is a change, not "left out".
    expect(styleLookOf("floating", { reflections: false })).toEqual({
      ...STYLE_LOOKS.floating,
      reflections: false,
    });
    expect(styleLookOf("native", { corners: 0 }).corners).toBe(0);
  });

  it("has a look for every style, with corners in 0–100", () => {
    expect(Object.keys(STYLE_LOOKS).sort()).toEqual([...DOCK_STYLES].sort());
    for (const { corners } of Object.values(STYLE_LOOKS)) {
      expect(corners).toBeGreaterThanOrEqual(0);
      expect(corners).toBeLessThanOrEqual(100);
    }
  });

  it("draws the shelf and the floating dock with reflections", () => {
    expect(STYLE_LOOKS.aquaShelf.reflections).toBe(true);
    expect(STYLE_LOOKS.floating.reflections).toBe(true);
    expect(STYLE_LOOKS.native.reflections).toBe(false);
  });
});

describe("dockLookOf", () => {
  const dock = (change: Partial<DockSettings>): DockSettings => ({
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
    ...change,
  });

  it("takes the changes of the chosen style only", () => {
    const settings = dock({
      style: "rocket",
      tweaks: { rocket: { corners: 5 }, neon: { corners: 90, border: false } },
    });

    expect(dockLookOf(settings)).toEqual({
      ...STYLE_LOOKS.rocket,
      corners: 5,
      style: "rocket",
      indicator: "dot",
      indicatorColor: null,
    });
  });

  it("carries the indicator and its color", () => {
    const look = dockLookOf(
      dock({ indicator: "dash", indicatorColor: "#FF0000" }),
    );

    expect(look.indicator).toBe("dash");
    expect(look.indicatorColor).toBe("#FF0000");
  });
});
