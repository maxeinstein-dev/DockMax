import { describe, expect, it } from "vitest";
import type { DockSize } from "../core/appearance";
import { MAX_DOCK_BAR_HEIGHT, MAX_DOCK_WINDOW_HEIGHT } from "../core/dock";
import type { LogicalRect } from "../core/panels";
import type { DockEdge } from "../core/settings";
import tokens from "../themes/tokens.css?raw";
import css from "./Dock.css?raw";
import {
  ROOM,
  SIZE_GEOMETRY,
  TIP_GAP,
  TIP_HEIGHT,
  dockLayout,
  dockWindowSize,
  placeDock,
  separatorWidth,
  tipAnchor,
} from "./layout";

/** Fase 1: Configurações and Sobre, one group. */
const GROUPS = [2];
/** Label pill widths used in the numeric examples (Configurações, Sobre). */
const LABELS = [110, 60];

function expectClose(actual: unknown, expected: unknown, path = "") {
  if (typeof expected === "number") {
    expect(actual, path).toBeTypeOf("number");
    expect(actual as number, path).toBeCloseTo(expected, 6);
    return;
  }
  if (Array.isArray(expected)) {
    expect(actual, path).toHaveLength(expected.length);
    expected.forEach((value, index) =>
      expectClose((actual as unknown[])[index], value, `${path}[${index}]`),
    );
    return;
  }
  const object = expected as Record<string, unknown>;
  expect(Object.keys(actual as object).sort(), path).toEqual(
    Object.keys(object).sort(),
  );
  for (const key of Object.keys(object)) {
    expectClose(
      (actual as Record<string, unknown>)[key],
      object[key],
      `${path}.${key}`,
    );
  }
}

const rect = (x: number, y: number, width: number, height: number) => ({
  x,
  y,
  width,
  height,
});

describe("size geometry", () => {
  it("matches the data-size presets of tokens.css", () => {
    for (const size of ["s", "m", "l"] as const) {
      const block = new RegExp(
        `:root\\[data-size="${size}"\\]\\s*\\{([^}]*)\\}`,
      ).exec(tokens)?.[1];
      expect(block, size).toBeDefined();
      const value = (name: string) =>
        Number(new RegExp(`--${name}:\\s*(\\d+)px`).exec(block ?? "")?.[1]);
      expect(SIZE_GEOMETRY[size]).toEqual({
        icon: value("icon"),
        pad: value("pad"),
        gap: value("gap"),
      });
    }
  });

  it("keeps the label pill as tall as TIP_HEIGHT in Dock.css", () => {
    const tip = /\.dock-tip \{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(tip).toContain("line-height: 16px");
    expect(tip).toContain("padding: 4px 10px");
    expect(TIP_HEIGHT).toBe(16 + 2 * 4);
    expect(TIP_GAP).toBe(10);
  });
});

describe("dockWindowSize", () => {
  // Height: pad + icon × 1.4 + lift 6 + TIP_GAP 10 + TIP_HEIGHT 24 + ROOM 8.
  // Width: the farthest the label of the magnified outer item reaches from
  // the center, plus ROOM on each side. Same on both edges.
  it.each([
    ["s", { width: 178, height: 110 }],
    ["m", { width: 189, height: 123 }],
    ["l", { width: 209, height: 147 }],
  ] as const)("fits size %s magnified, with its labels", (size, expected) => {
    expect(
      dockWindowSize({
        size,
        groups: GROUPS,
        motion: true,
        labelWidths: LABELS,
      }),
    ).toEqual(expected);
  });

  // Height: pad + icon + TIP_GAP + TIP_HEIGHT + ROOM; width from the labels
  // at rest.
  it.each([
    ["s", { width: 172, height: 88 }],
    ["m", { width: 181, height: 97 }],
    ["l", { width: 199, height: 115 }],
  ] as const)("fits size %s at rest with reduced motion", (size, expected) => {
    expect(
      dockWindowSize({
        size,
        groups: GROUPS,
        motion: false,
        labelWidths: LABELS,
      }),
    ).toEqual(expected);
  });

  it("fits the widest bar when the labels are narrow", () => {
    // Medium, magnified: 2 × 7 + 7 + 48 × (1.4 + 1.15) = 143.4, + 2 × 8.
    expect(
      dockWindowSize({
        size: "m",
        groups: GROUPS,
        motion: true,
        labelWidths: [0, 0],
      }),
    ).toEqual({ width: 160, height: 123 });
  });

  it("grows with a long label", () => {
    const narrow = dockWindowSize({
      size: "m",
      groups: GROUPS,
      motion: true,
      labelWidths: LABELS,
    });
    const wide = dockWindowSize({
      size: "m",
      groups: GROUPS,
      motion: true,
      labelWidths: [200, 60],
    });

    expect(wide.width - narrow.width).toBe(90);
    expect(wide.height).toBe(narrow.height);
  });
});

describe("placeDock", () => {
  const viewport = { width: 189, height: 123 };
  const medium = { size: "m" as const, groups: GROUPS, viewport };

  it("puts the bar at rest on the bottom edge, centered", () => {
    const placement = placeDock({ ...medium, edge: "bottom", magnified: null });

    // 2 × 7 + 2 × 48 + 7 = 117 wide, 48 + 2 × 7 = 62 tall.
    expectClose(placement, {
      bar: rect(36, 61, 117, 62),
      items: [rect(43, 68, 48, 48), rect(98, 68, 48, 48)],
      tiles: [rect(43, 68, 48, 48), rect(98, 68, 48, 48)],
      scales: [1, 1],
    });
  });

  it("pushes the neighbour apart and lifts the magnified tile", () => {
    const placement = placeDock({ ...medium, edge: "bottom", magnified: 0 });

    // Bar: 2 × 7 + 7 + 67.2 + 55.2 = 143.4 wide.
    expectClose(placement, {
      bar: rect(22.8, 61, 143.4, 62),
      items: [rect(29.8, 68, 67.2, 48), rect(104, 68, 55.2, 48)],
      tiles: [rect(29.8, 42.8, 67.2, 67.2), rect(104, 60.8, 55.2, 55.2)],
      scales: [1.4, 1.15],
    });
  });

  it("mirrors everything on the top edge", () => {
    const placement = placeDock({ ...medium, edge: "top", magnified: 0 });

    expectClose(placement, {
      bar: rect(22.8, 0, 143.4, 62),
      items: [rect(29.8, 7, 67.2, 48), rect(104, 7, 55.2, 48)],
      tiles: [rect(29.8, 13, 67.2, 67.2), rect(104, 7, 55.2, 55.2)],
      scales: [1.4, 1.15],
    });
  });

  it("leaves room for a separator between groups", () => {
    const placement = placeDock({
      size: "m",
      groups: [2, 1],
      viewport: { width: 400, height: 123 },
      edge: "bottom",
      magnified: null,
    });

    // The third item comes after a gap, the separator and another gap.
    const third = 7 + 2 * (48 + 7) + separatorWidth(7) + 7;
    expect(separatorWidth(7)).toBeCloseTo(6.6, 9);
    expect(placement.bar.width).toBeCloseTo(third + 48 + 7, 9);
    expect(placement.items[2]?.x).toBeCloseTo(placement.bar.x + third, 9);
  });
});

describe("tipAnchor", () => {
  const tile = rect(29.8, 42.8, 67.2, 67.2);

  it("hangs the label TIP_GAP above the tile on the bottom edge", () => {
    expectClose(tipAnchor(tile, "bottom"), { x: 63.4, y: 32.8 });
  });

  it("puts the label TIP_GAP below the tile on the top edge", () => {
    expectClose(tipAnchor(rect(29.8, 13, 67.2, 67.2), "top"), {
      x: 63.4,
      y: 90.2,
    });
  });
});

describe("dockLayout", () => {
  const viewport = { width: 189, height: 123 };
  const window = { width: 189, height: 123 };

  it("holds nothing at rest", () => {
    const placement = placeDock({
      size: "m",
      groups: GROUPS,
      viewport,
      edge: "bottom",
      magnified: null,
    });

    expect(dockLayout(placement, window)).toEqual({
      bar: placement.bar,
      hold: [],
      window,
    });
  });

  it("holds the magnified tiles", () => {
    const placement = placeDock({
      size: "m",
      groups: GROUPS,
      viewport,
      edge: "bottom",
      magnified: 1,
    });

    expect(dockLayout(placement, window).hold).toEqual(placement.tiles);
  });
});

describe("the window fits everything the dock paints", () => {
  const within = (
    inner: LogicalRect,
    size: { width: number; height: number },
  ) =>
    inner.x >= 0 &&
    inner.y >= 0 &&
    inner.x + inner.width <= size.width + 1e-9 &&
    inner.y + inner.height <= size.height + 1e-9;

  for (const size of ["s", "m", "l"] as DockSize[]) {
    for (const edge of ["bottom", "top"] as DockEdge[]) {
      for (const motion of [true, false]) {
        it(`size ${size}, ${edge}, motion ${motion}`, () => {
          const window = dockWindowSize({
            size,
            groups: GROUPS,
            motion,
            labelWidths: LABELS,
          });
          const states = motion ? [null, 0, 1] : [null];
          for (const magnified of states) {
            const placement = placeDock({
              size,
              groups: GROUPS,
              edge,
              magnified,
              viewport: window,
            });
            // The bar is flush with the screen-side edge.
            expect(placement.bar.y).toBe(
              edge === "bottom" ? window.height - placement.bar.height : 0,
            );
            expect(within(placement.bar, window)).toBe(true);
            placement.tiles.forEach((tile, index) => {
              expect(within(tile, window)).toBe(true);
              if (magnified !== null && index !== magnified) return;
              const at = tipAnchor(tile, edge);
              const width = LABELS[index] ?? 0;
              const pill = rect(
                at.x - width / 2,
                edge === "bottom" ? at.y - TIP_HEIGHT : at.y,
                width,
                TIP_HEIGHT,
              );
              // The pill keeps ROOM from the far edge and the sides.
              expect(
                within(
                  rect(
                    pill.x - ROOM,
                    edge === "bottom" ? pill.y - ROOM : pill.y,
                    pill.width + 2 * ROOM,
                    pill.height + ROOM,
                  ),
                  window,
                ),
                `item ${index}, magnified ${magnified}`,
              ).toBe(true);
            });
          }
        });
      }
    }
  }
});

describe("dock size limits shared with Rust", () => {
  it.each(["s", "m", "l"] as const)(
    "keeps the %s preset within the heights Rust accepts",
    (size) => {
      const { icon, pad } = SIZE_GEOMETRY[size];
      expect(icon + 2 * pad).toBeLessThanOrEqual(MAX_DOCK_BAR_HEIGHT);
      for (const motion of [true, false]) {
        const { height } = dockWindowSize({
          size,
          groups: GROUPS,
          motion,
          labelWidths: LABELS,
        });
        expect(height).toBeLessThanOrEqual(MAX_DOCK_WINDOW_HEIGHT);
      }
    },
  );
});
