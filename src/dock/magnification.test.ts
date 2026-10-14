import { describe, expect, it } from "vitest";
import css from "./Dock.css?raw";
import {
  MAGNIFICATION,
  MAGNIFICATION_VARIABLES,
  holdRects,
  magnifiedRect,
  magnifyRole,
} from "./magnification";

function expectRect(
  actual: { x: number; y: number; width: number; height: number },
  expected: { x: number; y: number; width: number; height: number },
) {
  expect(actual.x).toBeCloseTo(expected.x, 9);
  expect(actual.y).toBeCloseTo(expected.y, 9);
  expect(actual.width).toBeCloseTo(expected.width, 9);
  expect(actual.height).toBeCloseTo(expected.height, 9);
}

describe("magnification", () => {
  it("uses the dock's magnification values", () => {
    expect(MAGNIFICATION.hovered).toEqual({ scale: 1.4, lift: 6 });
    expect(MAGNIFICATION.neighbor).toEqual({ scale: 1.15, lift: 0 });
  });

  it("exposes the values to Dock.css as custom properties", () => {
    expect(MAGNIFICATION_VARIABLES).toEqual({
      "--dock-hovered-scale": "1.4",
      "--dock-hovered-lift": "6",
      "--dock-neighbor-scale": "1.15",
      "--dock-neighbor-lift": "0",
    });
  });

  it("keeps Dock.css on the shared values instead of its own numbers", () => {
    for (const name of Object.keys(MAGNIFICATION_VARIABLES)) {
      expect(css).toContain(`var(${name})`);
    }
    expect(css).toContain("transform-origin: center bottom");
    expect(css).not.toMatch(/scale\(\s*\d/);
    expect(css).not.toMatch(/translateY\(\s*-?\d/);
  });

  it("names the hovered item and its direct neighbours", () => {
    expect(magnifyRole(1, 1)).toBe("hovered");
    expect(magnifyRole(0, 1)).toBe("neighbor");
    expect(magnifyRole(2, 1)).toBe("neighbor");
    expect(magnifyRole(3, 1)).toBeNull();
    expect(magnifyRole(0, null)).toBeNull();
  });

  it("grows the hovered rect from its bottom center and lifts it", () => {
    const rect = { x: 16, y: 12, width: 100, height: 48 };

    // 100 * 1.4 = 140 wide, centered on x = 66; 48 * 1.4 = 67.2 tall,
    // bottom at 60 - 6 = 54.
    expectRect(magnifiedRect(rect, MAGNIFICATION.hovered), {
      x: -4,
      y: 54 - 67.2,
      width: 140,
      height: 67.2,
    });
  });

  it("grows a neighbour without lifting it", () => {
    const rect = { x: 124, y: 12, width: 60, height: 48 };

    expectRect(magnifiedRect(rect, MAGNIFICATION.neighbor), {
      x: 154 - 34.5,
      y: 60 - 55.2,
      width: 69,
      height: 55.2,
    });
  });

  it("leaves a rect at scale 1 without lift unchanged", () => {
    const rect = { x: 3.5, y: 7.25, width: 10, height: 20 };

    expect(magnifiedRect(rect, { scale: 1, lift: 0 })).toEqual(rect);
  });

  it("holds the hovered item and its neighbours only", () => {
    const first = { x: 0, y: 10, width: 50, height: 48 };
    const second = { x: 58, y: 10, width: 50, height: 48 };
    const third = { x: 116, y: 10, width: 50, height: 48 };
    const fourth = { x: 174, y: 10, width: 50, height: 48 };

    expect(holdRects([first, second, third, fourth], 1)).toEqual([
      magnifiedRect(first, MAGNIFICATION.neighbor),
      magnifiedRect(second, MAGNIFICATION.hovered),
      magnifiedRect(third, MAGNIFICATION.neighbor),
    ]);
  });

  it("holds nothing while no item is hovered", () => {
    expect(holdRects([{ x: 0, y: 0, width: 10, height: 10 }], null)).toEqual(
      [],
    );
  });

  it("holds only the existing neighbour at the ends", () => {
    const items = [
      { x: 0, y: 0, width: 10, height: 10 },
      { x: 20, y: 0, width: 10, height: 10 },
    ];

    expect(holdRects(items, 1)).toHaveLength(2);
    expect(holdRects(items, 0)).toHaveLength(2);
  });
});
