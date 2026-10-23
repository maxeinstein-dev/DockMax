import { describe, expect, it } from "vitest";
import css from "./Dock.css?raw";
import { DOCK_STYLES } from "../core/settings";
import { STATUS_WIDTH } from "./layout";
import {
  DEFAULT_HOVER,
  MAGNIFICATION,
  hoverConfigOf,
  magnificationOf,
  magnifyRole,
  peakMagnification,
} from "./magnification";

describe("magnification", () => {
  it("uses the DESIGN.md values: 1.4 lifted 6 px, neighbours 1.15", () => {
    expect(MAGNIFICATION.hovered).toEqual({ scale: 1.4, lift: 6 });
    expect(MAGNIFICATION.neighbor).toEqual({ scale: 1.15, lift: 0 });
  });

  it("names the magnified item and its direct neighbours", () => {
    expect(magnifyRole(1, 1)).toBe("hovered");
    expect(magnifyRole(0, 1)).toBe("neighbor");
    expect(magnifyRole(2, 1)).toBe("neighbor");
    expect(magnifyRole(3, 1)).toBeNull();
    expect(magnifyRole(0, null)).toBeNull();
  });

  it("gives every item its scale and lift", () => {
    expect(magnificationOf(1, 1)).toEqual(MAGNIFICATION.hovered);
    expect(magnificationOf(0, 1)).toEqual(MAGNIFICATION.neighbor);
    expect(magnificationOf(3, 1)).toEqual({ scale: 1, lift: 0 });
    expect(magnificationOf(0, null)).toEqual({ scale: 1, lift: 0 });
  });
});

describe("hover effects", () => {
  const scale = { effect: "scale", scale: 1.25 } as const;
  const none = { effect: "none", scale: 1.25 } as const;
  const magnify = { effect: "magnify", scale: 1.25 } as const;

  it("does nothing with Nenhum", () => {
    expect(magnifyRole(1, 1, "none")).toBeNull();
    expect(magnifyRole(0, 1, "none")).toBeNull();
    expect(magnificationOf(1, 1, none)).toEqual({ scale: 1, lift: 0 });
    expect(peakMagnification(none)).toEqual({ scale: 1, lift: 0 });
  });

  it("grows only the hovered item, by the chosen scale and without lift, with Escala", () => {
    expect(magnifyRole(1, 1, "scale")).toBe("hovered");
    expect(magnifyRole(0, 1, "scale")).toBeNull();
    expect(magnifyRole(2, 1, "scale")).toBeNull();
    expect(magnificationOf(1, 1, scale)).toEqual({ scale: 1.25, lift: 0 });
    expect(magnificationOf(0, 1, scale)).toEqual({ scale: 1, lift: 0 });
    expect(peakMagnification(scale)).toEqual({ scale: 1.25, lift: 0 });
  });

  it("keeps the Fase 1 values with Ampliação, whatever the scale", () => {
    expect(magnificationOf(1, 1, magnify)).toEqual(MAGNIFICATION.hovered);
    expect(magnificationOf(0, 1, magnify)).toEqual(MAGNIFICATION.neighbor);
    expect(peakMagnification(magnify)).toEqual(MAGNIFICATION.hovered);
  });

  it("reads the effect and the scale from the settings, in percent", () => {
    expect(hoverConfigOf({ hover: "scale", hoverScale: 110 })).toEqual({
      effect: "scale",
      scale: 1.1,
    });
    expect(hoverConfigOf({ hover: "magnify", hoverScale: 160 }).scale).toBe(
      1.6,
    );
  });

  it("starts as the defaults of the settings: Escala, 110 %", () => {
    expect(hoverConfigOf(undefined)).toEqual(DEFAULT_HOVER);
    expect(DEFAULT_HOVER).toEqual({ effect: "scale", scale: 1.1 });
  });
});

describe("Dock.css", () => {
  const rule = (selector: string) => {
    const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return new RegExp(`(^|\\n)${escaped} \\{([^}]*)\\}`).exec(css)?.[2] ?? "";
  };

  it("takes the magnification from the per-item values, not its own numbers", () => {
    expect(rule(".dock-item")).toContain(
      "width: calc(var(--icon) * var(--s, 1))",
    );
    expect(rule(".dock-tile")).toContain(
      "transform: translateY(calc(var(--lift, 0px) * -1)) scale(var(--s, 1))",
    );
    expect(css).not.toMatch(/scale\(\s*\d/);
    expect(css).not.toMatch(/translateY\(\s*-?\d/);
  });

  it("grows the tiles from the screen edge, mirrored on top", () => {
    expect(rule(".dock-tile")).toContain("transform-origin: 50% 100%");
    expect(rule(':root[data-edge="top"] .dock-tile')).toContain(
      "transform-origin: 50% 0",
    );
    expect(rule(':root[data-edge="top"] .dock-tile')).toContain(
      "transform: translateY(var(--lift, 0px)) scale(var(--s, 1))",
    );
  });

  it("animates with the motion tokens and stops with reduced motion", () => {
    expect(rule(".dock-item")).toContain(
      "transition: width var(--t-magnify) var(--ease)",
    );
    expect(rule(".dock-tile")).toContain(
      "transition: transform var(--t-magnify) var(--ease)",
    );
    const reduced =
      /@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?)\n\}/.exec(
        css,
      )?.[1];
    expect(reduced).toMatch(
      /\.dock-item,\s*\.dock-tile,\s*\.dock-dot \{\s*transition: none;/,
    );
  });

  it("lays the bar out as layout.ts computes it", () => {
    const bar = rule(".dock-bar");
    expect(bar).toContain("height: calc(var(--icon) + 2 * var(--pad))");
    expect(bar).toContain("gap: var(--gap)");
    expect(bar).toContain("box-sizing: border-box");
    // 1 px border + pad − 1 px of padding: the items sit pad inside.
    expect(bar).toContain("padding: calc(var(--pad) - 1px)");
    expect(bar).toContain("border: 1px solid var(--hair)");
  });

  it("draws every style of the bar, Nativo being the base look", () => {
    const others = DOCK_STYLES.filter((style) => style !== "native");
    expect(others).toHaveLength(DOCK_STYLES.length - 1);
    expect(css).not.toContain('data-style="native"');
    for (const style of others) {
      const block = new RegExp(
        ':root\\[data-style="' + style + '"\\] \\{([^}]*)\\}',
      ).exec(css)?.[1];
      expect(block, style).toBeDefined();
      // Every style names its bar color and its indicator color.
      expect(block, style).toContain("--bar-bg:");
      expect(block, style).toContain("--ind-default:");
    }
  });

  it("takes the corners, the outline and the reflections from the page attributes", () => {
    expect(rule(":root[data-style] .dock-bar")).toContain(
      "var(--corners, 0.65)",
    );
    expect(rule(':root[data-border="off"] .dock-bar')).toContain(
      "border-color: transparent",
    );
    expect(css).toContain(':root[data-reflect="on"] .dock-tile');
    expect(css).toContain(
      ':root[data-edge="top"][data-reflect="on"] .dock-tile',
    );
  });

  it("draws the dash indicator and takes its color from the page", () => {
    expect(rule(".dock-dot")).toContain(
      "background: var(--indicator-color, var(--ind-default, var(--accent)))",
    );
    expect(rule(':root[data-indicator="dash"] .dock-dot')).toContain(
      "height: 3px",
    );
  });

  it("gives the clock the width layout.ts reserves for it", () => {
    expect(rule(".dock-status")).toContain(`width: ${STATUS_WIDTH}px`);
    expect(rule(".dock-status")).toContain("height: var(--icon)");
  });
});
