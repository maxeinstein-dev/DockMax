import type { DockStyle } from "../core/settings";

/* The colors each style paints the bar with, for the miniatures of
 * Configurações › Aparência (spec 0014 R1). A mirror of
 * `src-tauri/src/windows/native_dock/look.rs`, which paints the real dock;
 * paints.test.ts keeps the two equal. Colors as `[r, g, b, alpha]`. */

export type Rgba = readonly [number, number, number, number];

export interface StylePaint {
  /** The bar, from the top: one color, or the stops of a vertical gradient
   * (`over` drawn over `stops`, both over `base`). Null: the theme's own
   * surface, as the Nativo style. */
  fill:
    | { solid: Rgba }
    | {
        base: Rgba | null;
        stops: readonly (readonly [number, Rgba])[];
        over: readonly (readonly [number, Rgba])[];
      }
    | null;
  /** Soft light around the bar: color and reach in px of the real dock. */
  glows: readonly { color: Rgba; reach: number }[];
  /** The light edge; null for the theme's own, "none" for no edge. */
  hi: Rgba | "none" | null;
  /** A gradient outline from left to right, instead of the hairline. */
  outline: readonly [Rgba, Rgba] | null;
  /** No shadow under the bar (the floating styles). */
  flat: boolean;
  /** The silver shelf under the tiles. */
  shelf: boolean;
  /** The style's own color of the mark under an open program; null for the
   * accent. */
  indicator: Rgba | null;
}

const WHITE = (alpha: number): Rgba => [255, 255, 255, alpha];

const PLAIN: StylePaint = {
  fill: null,
  glows: [],
  hi: null,
  outline: null,
  flat: false,
  shelf: false,
  indicator: null,
};

export const STYLE_PAINTS: Readonly<Record<DockStyle, StylePaint>> = {
  native: PLAIN,
  liquidGlass: {
    ...PLAIN,
    fill: {
      base: [32, 38, 56, 0.5],
      stops: [
        [0, WHITE(0.24)],
        [1, WHITE(0.07)],
      ],
      over: [],
    },
    glows: [{ color: WHITE(0.18), reach: 6 }],
    indicator: WHITE(1),
  },
  midnight: {
    ...PLAIN,
    fill: { solid: [8, 10, 16, 0.93] },
    indicator: WHITE(1),
  },
  rocket: {
    ...PLAIN,
    fill: { solid: [9, 14, 32, 0.95] },
    glows: [{ color: [80, 130, 255, 0.6], reach: 7 }],
    indicator: [0x8f, 0xb0, 0xff, 1],
  },
  neon: {
    ...PLAIN,
    fill: { solid: [6, 6, 12, 0.96] },
    glows: [
      { color: [0, 229, 255, 0.55], reach: 6 },
      { color: [255, 43, 214, 0.35], reach: 7 },
    ],
    outline: [
      [0x00, 0xe5, 0xff, 1],
      [0xff, 0x2b, 0xd6, 1],
    ],
    indicator: [0x00, 0xe5, 0xff, 1],
  },
  canary: {
    ...PLAIN,
    fill: {
      base: null,
      stops: [
        [0, [0xf3, 0xef, 0x3d, 1]],
        [0.58, [0x8e, 0xd1, 0x2c, 1]],
        [1, [0x34, 0xa5, 0x2c, 1]],
      ],
      over: [
        [0, WHITE(0.42)],
        [0.46, WHITE(0)],
      ],
    },
    indicator: [0x1c, 0x4a, 0x12, 1],
  },
  paper: {
    ...PLAIN,
    fill: { solid: [247, 247, 243, 1] },
    hi: "none",
    indicator: [0x2c, 0x3a, 0x5a, 1],
  },
  floating: {
    ...PLAIN,
    fill: { solid: [0, 0, 0, 0] },
    hi: "none",
    flat: true,
    indicator: WHITE(1),
  },
  aquaShelf: {
    ...PLAIN,
    fill: { solid: [0, 0, 0, 0] },
    hi: "none",
    flat: true,
    shelf: true,
    indicator: [0x2c, 0x3a, 0x5a, 1],
  },
};

/** The stops of the silver shelf, from the side away from the screen edge
 * (`fn shelf` in `native_dock/paint.rs`). */
export const SHELF_STOPS: readonly (readonly [number, Rgba])[] = [
  [0, [0xf6, 0xf8, 0xfc, 1]],
  [0.55, [0xb4, 0xba, 0xc6, 1]],
  [1, [0x85, 0x8c, 0x9b, 1]],
];

/** A color as CSS. */
export function css([r, g, b, a]: Rgba): string {
  return `rgb(${r} ${g} ${b} / ${a})`;
}

function gradient(
  stops: readonly (readonly [number, Rgba])[],
  direction = "180deg",
): string {
  const list = stops.map(([at, color]) => `${css(color)} ${at * 100}%`);
  return `linear-gradient(${direction}, ${list.join(", ")})`;
}

/** The CSS `background` of the bar; null for the theme's own surface. */
export function fillCss(paint: StylePaint): string | null {
  const fill = paint.fill;
  if (fill === null) return null;
  if ("solid" in fill) return css(fill.solid);
  const layers = [fill.over, fill.stops]
    .filter((layer) => layer.length > 0)
    .map((layer) => gradient(layer));
  if (fill.base !== null) layers.push(gradient([[0, fill.base]]));
  return layers.join(", ");
}

/** The glows as a CSS `box-shadow` list, scaled to the miniature. */
export function glowCss(paint: StylePaint, scale: number): string | null {
  if (paint.glows.length === 0) return null;
  return paint.glows
    .map(({ color, reach }) => `0 0 ${reach * scale}px ${css(color)}`)
    .join(", ");
}

/** The outline as a horizontal CSS gradient. */
export function outlineCss(paint: StylePaint): string | null {
  return paint.outline === null
    ? null
    : gradient(
        [
          [0, paint.outline[0]],
          [1, paint.outline[1]],
        ],
        "90deg",
      );
}

export function shelfCss(): string {
  return gradient(SHELF_STOPS);
}
