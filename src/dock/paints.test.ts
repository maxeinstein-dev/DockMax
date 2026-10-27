import { describe, expect, it } from "vitest";
import lookRs from "../../src-tauri/src/windows/native_dock/look.rs?raw";
import paintRs from "../../src-tauri/src/windows/native_dock/paint.rs?raw";
import { DOCK_STYLES, type DockStyle } from "../core/settings";
import {
  SHELF_STOPS,
  STYLE_PAINTS,
  fillCss,
  glowCss,
  outlineCss,
  type Rgba,
  type StylePaint,
} from "./paints";

// The miniatures must paint each style as the real dock does: every color of
// a style here is a color of that style in look.rs, and the other way round.
// The Rust is read, never restated.

const RUST_NAMES: Readonly<Record<DockStyle, string>> = {
  native: "Native",
  liquidGlass: "LiquidGlass",
  midnight: "Midnight",
  rocket: "Rocket",
  aquaShelf: "AquaShelf",
  neon: "Neon",
  canary: "Canary",
  paper: "Paper",
  floating: "Floating",
};

/** `r g b` of every color written in `rust`, as a set. */
function rustColors(rust: string): Set<string> {
  const found = new Set<string>();
  const number = (text: string) =>
    Number.parseInt(text, text.startsWith("0x") ? 16 : 10);
  for (const match of rust.matchAll(
    /Color::rgba?\(\s*(0x[0-9a-f]+|\d+),\s*(0x[0-9a-f]+|\d+),\s*(0x[0-9a-f]+|\d+)/gi,
  )) {
    found.add(
      [match[1], match[2], match[3]]
        .map((part) => number(part ?? ""))
        .join(" "),
    );
  }
  if (/WHITE/.test(rust)) found.add("255 255 255");
  if (/Color::CLEAR/.test(rust)) found.add("0 0 0");
  return found;
}

/** The block of `Look::of` for one style. */
function rustBlock(style: DockStyle): string {
  // `Look::of`, not `own` (which matches the styles first).
  const of = lookRs.split("impl Look")[1] ?? "";
  const body = of.split("match style {")[1]?.split("#[cfg(test)]")[0] ?? "";
  const start = body.indexOf(`DockStyle::${RUST_NAMES[style]} =>`);
  expect(start, `no block for ${style} in look.rs`).toBeGreaterThanOrEqual(0);
  const rest = body.slice(start + 1);
  const end = rest.search(/DockStyle::\w+ =>/);
  return end < 0 ? rest : rest.slice(0, end);
}

function paintColors(paint: StylePaint): Set<string> {
  const colors: Rgba[] = [];
  const fill = paint.fill;
  if (fill !== null) {
    if ("solid" in fill) colors.push(fill.solid);
    else {
      if (fill.base) colors.push(fill.base);
      colors.push(
        ...fill.stops.map(([, c]) => c),
        ...fill.over.map(([, c]) => c),
      );
    }
  }
  colors.push(...paint.glows.map((glow) => glow.color));
  if (paint.outline) colors.push(...paint.outline);
  if (paint.indicator) colors.push(paint.indicator);
  if (paint.hi === "none") colors.push([0, 0, 0, 0]);
  // The separator and the clock's ink are not drawn in a miniature, but
  // they are colors of the style in Rust.
  return new Set(colors.map(([r, g, b]) => `${r} ${g} ${b}`));
}

describe("STYLE_PAINTS", () => {
  it.each(DOCK_STYLES)("paints %s with the colors of look.rs", (style) => {
    const rust = rustColors(rustBlock(style));
    const ours = paintColors(STYLE_PAINTS[style]);
    for (const color of ours) {
      expect(rust, `${style}: ${color} is not in look.rs`).toContain(color);
    }
  });

  it("has the shelf of paint.rs", () => {
    const shelf = paintRs.split("fn shelf")[1]?.split("fn light_edge")[0] ?? "";
    const rust = rustColors(shelf);
    for (const [, [r, g, b]] of SHELF_STOPS) {
      expect(rust).toContain(`${r} ${g} ${b}`);
    }
  });

  it("marks the styles look.rs marks", () => {
    for (const style of DOCK_STYLES) {
      const block = rustBlock(style);
      const paint = STYLE_PAINTS[style];
      expect(paint.flat, style).toBe(/flat: true/.test(block));
      expect(paint.shelf, style).toBe(/shelf: true/.test(block));
      expect(paint.outline !== null, style).toBe(/outline:/.test(block));
      expect(paint.glows.length, style).toBe(
        (block.match(/Glow \{/g) ?? []).length,
      );
      expect(paint.hi === "none", style).toBe(
        /hi: Some\(Color::CLEAR\)/.test(block),
      );
    }
  });
});

describe("the CSS of a paint", () => {
  it("draws the theme's own bar for Nativo", () => {
    expect(fillCss(STYLE_PAINTS.native)).toBeNull();
  });

  it("puts the gradient over its base", () => {
    expect(fillCss(STYLE_PAINTS.liquidGlass)).toBe(
      "linear-gradient(180deg, rgb(255 255 255 / 0.24) 0%, rgb(255 255 255 / 0.07) 100%), " +
        "linear-gradient(180deg, rgb(32 38 56 / 0.5) 0%)",
    );
  });

  it("scales the glows and draws Neon's outline", () => {
    expect(glowCss(STYLE_PAINTS.rocket, 0.5)).toBe(
      "0 0 3.5px rgb(80 130 255 / 0.6)",
    );
    expect(glowCss(STYLE_PAINTS.paper, 0.5)).toBeNull();
    expect(outlineCss(STYLE_PAINTS.neon)).toBe(
      "linear-gradient(90deg, rgb(0 229 255 / 1) 0%, rgb(255 43 214 / 1) 100%)",
    );
  });
});
