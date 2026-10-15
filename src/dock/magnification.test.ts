import { describe, expect, it } from "vitest";
import css from "./Dock.css?raw";
import { MAGNIFICATION, magnificationOf, magnifyRole } from "./magnification";

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
});
