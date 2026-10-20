import { describe, expect, it } from "vitest";
import parts from "../panel/parts/parts.css?raw";
import css from "./tokens.css?raw";

// WCAG 2.x contrast of the text and glyphs DockMax draws, computed from the
// tokens in tokens.css (DESIGN.md "Colors"), in both themes. Text needs AA
// (4.5:1); the tile glyph is a large icon stroke, a non-text graphic (3:1).
//
// Only the dock bar follows the dock opacity (Translúcido/Sólido), and it
// has no text of its own: the panel is always opaque (The Opaque Panel
// Rule), the label pill is 97% opaque and the tiles are opaque. The pill is
// therefore checked over a black and a white backdrop, which covers whatever
// shows through the bar and behind the dock.

type Rgb = readonly [number, number, number];
interface Rgba {
  rgb: Rgb;
  alpha: number;
}

/** Custom properties of the first rule whose selector is `selector`. */
function rule(text: string, selector: string): Map<string, string> {
  const start = text.indexOf(`${selector} {`);
  if (start < 0) throw new Error(`no rule for ${selector} in tokens.css`);
  const body = text.slice(
    text.indexOf("{", start) + 1,
    text.indexOf("}", start),
  );
  const props = new Map<string, string>();
  for (const match of body
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    if (match[1] && match[2]) props.set(match[1], match[2].trim());
  }
  return props;
}

const THEMES = {
  light: rule(css, ":root"),
  dark: rule(css, ':root[data-mode="dark"]'),
};

function token(theme: keyof typeof THEMES, name: string): string {
  const value = THEMES[theme].get(name) ?? THEMES.light.get(name);
  if (value === undefined) throw new Error(`tokens.css has no ${name}`);
  return value;
}

/** `#rrggbb`, `rgb(r g b / a)`, `hsl(h s% l% / a)` or a bare `r g b`
 * triplet. */
function parse(value: string): Rgba {
  const hslMatch =
    /^hsl\(\s*([\d.]+)\s+([\d.]+)%\s+([\d.]+)%\s*(?:\/\s*([\d.]+))?\s*\)$/.exec(
      value,
    );
  if (hslMatch) {
    return {
      rgb: hsl(
        Number(hslMatch[1]),
        Number(hslMatch[2]) / 100,
        Number(hslMatch[3]) / 100,
      ),
      alpha: hslMatch[4] === undefined ? 1 : Number(hslMatch[4]),
    };
  }
  const hex = /^#([0-9a-f]{6})$/i.exec(value);
  if (hex?.[1]) {
    const n = parseInt(hex[1], 16);
    return { rgb: [(n >> 16) & 255, (n >> 8) & 255, n & 255], alpha: 1 };
  }
  const rgb =
    /^(?:rgb\(\s*)?(\d+)\s+(\d+)\s+(\d+)\s*(?:\/\s*([\d.]+))?\s*\)?$/.exec(
      value,
    );
  if (rgb) {
    return {
      rgb: [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])],
      alpha: rgb[4] === undefined ? 1 : Number(rgb[4]),
    };
  }
  throw new Error(`unsupported color ${value}`);
}

function color(theme: keyof typeof THEMES, name: string): Rgba {
  return parse(token(theme, name));
}

/** The opacity of the text field's placeholder, as parts.css sets it. */
function placeholderOpacity(): number {
  const match = /\.field-input::placeholder\s*\{[^}]*?opacity:\s*([\d.]+)/.exec(
    parts,
  );
  if (!match) throw new Error("no placeholder opacity in parts.css");
  return Number(match[1]);
}

/** `top` painted over the opaque `bottom`. */
function over(top: Rgba, bottom: Rgb): Rgb {
  return top.rgb.map(
    (channel, i) => channel * top.alpha + (bottom[i] ?? 0) * (1 - top.alpha),
  ) as unknown as Rgb;
}

/** CSS hsl() to sRGB, s and l in 0–1. */
function hsl(h: number, s: number, l: number): Rgb {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) =>
    255 * (l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1)));
  return [f(0), f(8), f(4)];
}

function luminance(rgb: Rgb): number {
  const [r, g, b] = rgb.map((channel) => {
    const c = channel / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [
    number,
    number,
  ];
  return (hi + 0.05) / (lo + 0.05);
}

const BLACK: Rgb = [0, 0, 0];
const WHITE: Rgb = [255, 255, 255];
const TEXT = 4.5;
const GRAPHIC = 3;

interface Pair {
  name: string;
  ratio: number;
  min: number;
}

function textPairs(theme: keyof typeof THEMES): Pair[] {
  const surface = color(theme, "--surface-rgb").rgb;
  const ink = over(color(theme, "--ink"), surface);
  const ink2 = over(color(theme, "--ink-2"), surface);
  const accentInk = color(theme, "--accent-ink").rgb;
  const soft = color(theme, "--accent-soft");
  const track = over(color(theme, "--track"), surface);
  const tipInk = color(theme, "--tip-ink").rgb;
  const tipBg = color(theme, "--tip-bg");
  const danger = over(color(theme, "--danger"), surface);
  const field = over(color(theme, "--field"), surface);
  const inkColor = color(theme, "--ink");
  // `color-mix(in srgb, var(--ink) 9%, transparent)`: the ink at 9 % of
  // its alpha.
  const navHover = over({ ...inkColor, alpha: inkColor.alpha * 0.09 }, surface);
  const dangerSoft = over(color(theme, "--danger-soft"), surface);
  return [
    { name: "--ink on the panel", ratio: contrast(ink, surface), min: TEXT },
    { name: "--ink-2 on the panel", ratio: contrast(ink2, surface), min: TEXT },
    {
      name: "--accent-ink on --accent-soft (panel)",
      ratio: contrast(accentInk, over(soft, surface)),
      min: TEXT,
    },
    {
      // The selected segment sits inside the track.
      name: "--accent-ink on --accent-soft (track)",
      ratio: contrast(accentInk, over(soft, track)),
      min: TEXT,
    },
    {
      name: "--ink on a hovered segment (--track)",
      ratio: contrast(ink, track),
      min: TEXT,
    },
    {
      name: "--ink on the key hint (--kbd)",
      ratio: contrast(ink, over(color(theme, "--kbd"), surface)),
      min: TEXT,
    },
    {
      // "Remover nota", inside the confirmation.
      name: "--danger-ink on --danger",
      ratio: contrast(color(theme, "--danger-ink").rgb, danger),
      min: TEXT,
    },
    {
      // "Remover do dock" and the note's save error.
      name: "--danger on the panel",
      ratio: contrast(danger, surface),
      min: TEXT,
    },
    {
      // The hovered "Remover do dock".
      name: "--danger on --danger-soft",
      ratio: contrast(danger, dangerSoft),
      min: TEXT,
    },
    {
      // The text of the inline confirmation.
      name: "--ink on --danger-soft",
      ratio: contrast(ink, dangerSoft),
      min: TEXT,
    },
    {
      name: "--ink in a text field (--field)",
      ratio: contrast(ink, field),
      min: TEXT,
    },
    {
      // The kind's name in an empty "Rótulo".
      name: "the placeholder (--ink-2) in a text field (--field)",
      ratio: contrast(
        over(
          { ...color(theme, "--ink-2"), alpha: placeholderOpacity() },
          field,
        ),
        field,
      ),
      min: TEXT,
    },
    {
      // A quiet or icon button's text, and the value of a navigation row.
      name: "--ink-2 on --track",
      ratio: contrast(ink2, track),
      min: TEXT,
    },
    {
      name: "--ink-2 on a hovered navigation row",
      ratio: contrast(ink2, navHover),
      min: TEXT,
    },
    {
      name: "--tip-ink on --tip-bg over black",
      ratio: contrast(tipInk, over(tipBg, BLACK)),
      min: TEXT,
    },
    {
      name: "--tip-ink on --tip-bg over white",
      ratio: contrast(tipInk, over(tipBg, WHITE)),
      min: TEXT,
    },
  ];
}

/** Non-text marks the user must see (WCAG 1.4.11). */
function graphicPairs(theme: keyof typeof THEMES): Pair[] {
  const surface = color(theme, "--surface-rgb").rgb;
  const track = over(color(theme, "--pg-track"), surface);
  return [
    {
      name: "--pg-fill on --pg-track",
      ratio: contrast(over(color(theme, "--pg-fill"), track), track),
      min: GRAPHIC,
    },
    {
      // The bottom line is what marks a text field as one.
      name: "--field-line under a text field",
      ratio: contrast(
        over(color(theme, "--field-line"), surface),
        over(color(theme, "--field"), surface),
      ),
      min: GRAPHIC,
    },
  ];
}

/** The ratio, rounded down to two decimals, for the test name. */
function shown(pair: Pair): Pair & { shown: string } {
  return { ...pair, shown: (Math.floor(pair.ratio * 100) / 100).toFixed(2) };
}

/** A tile glaze's middle colour: halfway between the top (+7 %) and the
 * bottom (-5 %), under the sheen, which is transparent by then. */
function glazeMiddle(name: string): Rgb {
  const props = rule(css, `.${name}`);
  const number = (prop: string) => parseFloat(props.get(prop) ?? "NaN");
  return hsl(number("--h"), number("--sat") / 100, (number("--l") + 1) / 100);
}

/** Each tile glaze's top colour (DESIGN.md "Dock item / ladrilho": the
 * hue's lightness + 7% at the top, where the glaze is lightest). */
function glazeTops(): { name: string; rgb: Rgb }[] {
  const tiles = [...css.matchAll(/\.(t-[\w-]+)\s*\{/g)];
  if (tiles.length === 0) throw new Error("no tile glaze in tokens.css");
  return tiles.map((match) => {
    const name = match[1] as string;
    const props = rule(css, `.${name}`);
    const number = (prop: string) => parseFloat(props.get(prop) ?? "NaN");
    return {
      name,
      rgb: hsl(number("--h"), number("--sat") / 100, (number("--l") + 7) / 100),
    };
  });
}

describe("contrast (WCAG AA)", () => {
  for (const theme of ["light", "dark"] as const) {
    describe(`${theme} theme`, () => {
      it.each([...textPairs(theme), ...graphicPairs(theme)].map(shown))(
        "$name: $shown:1, at least $min:1",
        ({ ratio, min }) => {
          expect(ratio).toBeGreaterThanOrEqual(min);
        },
      );
    });
  }

  const glyph = parse(token("light", "--glyph")).rgb;
  it.each(
    glazeTops().map(({ name, rgb }) =>
      shown({ name, ratio: contrast(glyph, rgb), min: GRAPHIC }),
    ),
  )(
    "white glyph on the top of $name: $shown:1, at least $min:1",
    ({ ratio, min }) => {
      expect(ratio).toBeGreaterThanOrEqual(min);
    },
  );

  // The live icons write text on the glaze: the clock's time, the
  // progress's percentage.
  it.each(
    ["t-clock", "t-progress"].map((name) =>
      shown({ name, ratio: contrast(glyph, glazeMiddle(name)), min: TEXT }),
    ),
  )(
    "white text in the middle of $name: $shown:1, at least $min:1",
    ({ ratio, min }) => {
      expect(ratio).toBeGreaterThanOrEqual(min);
    },
  );

  it("parses what it checks", () => {
    expect(contrast(BLACK, WHITE)).toBeCloseTo(21, 5);
    expect(hsl(0, 1, 0.5).map(Math.round)).toEqual([255, 0, 0]);
    expect(glazeTops().map((tile) => tile.name)).toContain("t-settings");
    expect(textPairs("dark")[0]?.ratio).not.toBe(textPairs("light")[0]?.ratio);
  });
});
