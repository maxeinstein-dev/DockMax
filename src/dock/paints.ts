import type { DockStyle } from "../core/settings";
import classic98Json from "../../src-tauri/skins/classic98/skin.json";
import classic98Png from "../../src-tauri/skins/classic98/skin.png";
import phosphorJson from "../../src-tauri/skins/phosphor/skin.json";
import phosphorPng from "../../src-tauri/skins/phosphor/skin.png";
import retroJson from "../../src-tauri/skins/retro/skin.json";
import retroPng from "../../src-tauri/skins/retro/skin.png";
import tacticalJson from "../../src-tauri/skins/tactical/skin.json";
import tacticalPng from "../../src-tauri/skins/tactical/skin.png";
import medievalJson from "../../src-tauri/skins/medieval/skin.json";
import medievalPng from "../../src-tauri/skins/medieval/skin.png";
import hunterJson from "../../src-tauri/skins/hunter/skin.json";
import hunterPng from "../../src-tauri/skins/hunter/skin.png";

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
  /** The bar is an image (spec 0020 §3): its skin, else null. */
  skin: SkinPaint | null;
}

/** A skin of the bar, from its `skin.json` and `skin.png` in
 * `src-tauri/skins/<id>/`: the image goes to the bar's height, its ends as
 * they are and the middle repeated (or stretched) across. */
export interface SkinPaint {
  url: string;
  width: number;
  height: number;
  left: number;
  right: number;
  repeat: boolean;
  pixelated: boolean;
}

interface SkinJson {
  width: number;
  height: number;
  slice: { top: number; right: number; bottom: number; left: number };
  middle: string;
  pixelated: boolean;
}

function skinPaint(url: string, json: SkinJson): SkinPaint {
  return {
    url,
    width: json.width,
    height: json.height,
    left: json.slice.left,
    right: json.slice.right,
    repeat: json.middle === "repeat",
    pixelated: json.pixelated,
  };
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
  skin: null,
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
  retro: {
    ...PLAIN,
    indicator: [0xff, 0x00, 0x4d, 1],
    skin: skinPaint(retroPng, retroJson),
  },
  classic98: {
    ...PLAIN,
    indicator: [0x00, 0x00, 0x80, 1],
    skin: skinPaint(classic98Png, classic98Json),
  },
  phosphor: {
    ...PLAIN,
    indicator: [0x3c, 0xff, 0x6e, 1],
    skin: skinPaint(phosphorPng, phosphorJson),
  },
  tactical: {
    ...PLAIN,
    indicator: [0xff, 0x8a, 0x1f, 1],
    skin: skinPaint(tacticalPng, tacticalJson),
  },
  medieval: {
    ...PLAIN,
    indicator: [0xe8, 0xb8, 0x4a, 1],
    skin: skinPaint(medievalPng, medievalJson),
  },
  hunter: {
    ...PLAIN,
    indicator: [0xf2, 0xa3, 0x3a, 1],
    skin: skinPaint(hunterPng, hunterJson),
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
