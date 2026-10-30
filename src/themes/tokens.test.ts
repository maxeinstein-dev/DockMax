import { describe, expect, it } from "vitest";
import design from "../../DESIGN.md?raw";
import designJsonText from "../../.impeccable/design.json?raw";
import { readSources } from "../test/sources";
import css from "./tokens.css?raw";
import { PANEL_INSET } from "../panel/geometry";

// tokens.css must carry DESIGN.md exactly. The expected values are read from
// DESIGN.md (frontmatter) and .impeccable/design.json, never restated here,
// so changing the design system means changing those files.

const designJson = JSON.parse(designJsonText) as {
  extensions: {
    colorMeta: Record<string, { role: string; canonical: string }>;
    shadows: { name: string; value: string }[];
    motion: { name: string; value: string }[];
  };
};

/** One two-level section of the DESIGN.md frontmatter, as flat strings. */
function frontmatter(section: string): Record<string, string> {
  const yaml = design.split(/^---$/m)[1] ?? "";
  const body = yaml.split(new RegExp(`^${section}:$`, "m"))[1] ?? "";
  const entries: Record<string, string> = {};
  for (const line of body.split("\n").slice(1)) {
    if (/^\S/.test(line)) break;
    const match = /^ {2}([\w-]+): (.+)$/.exec(line);
    if (match?.[1] && match[2]) entries[match[1]] = match[2].replace(/"/g, "");
  }
  return entries;
}

/** Same value whatever the spelling: Prettier writes `0.13` for `.13`. */
function normalize(value: string): string {
  return value
    .toLowerCase()
    .replace(/"/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/#[0-9a-f]+|\d*\.\d+|\d+/g, (token) =>
      token.startsWith("#") ? token : String(Number(token)),
    );
}

/** Body of the block opened right after `start` (balanced braces). */
function blockAfter(text: string, start: number): string {
  const open = text.indexOf("{", start);
  let depth = 0;
  for (let i = open; i < text.length; i++) {
    if (text[i] === "{") depth++;
    if (text[i] === "}" && --depth === 0) return text.slice(open + 1, i);
  }
  throw new Error("unbalanced braces in tokens.css");
}

/** Custom properties of the rule whose selector is exactly `selector`. */
function declarations(text: string, selector: string): Map<string, string> {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = new RegExp(`(^|\\n)\\s*${escaped}\\s*\\{`).exec(text);
  if (!match) throw new Error(`no rule for ${selector} in tokens.css`);
  const props = new Map<string, string>();
  const body = blockAfter(text, match.index).replace(/\/\*[\s\S]*?\*\//g, "");
  for (const declaration of body.split(";")) {
    const prop = /^\s*(--[\w-]+)\s*:\s*([\s\S]+?)\s*$/.exec(declaration);
    if (prop?.[1] && prop[2]) props.set(prop[1], normalize(prop[2]));
  }
  return props;
}

/** The `color-scheme` of the rule whose selector is exactly `selector`. */
function colorScheme(text: string, selector: string): string | undefined {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = new RegExp(`(^|\\n)\\s*${escaped}\\s*\\{`).exec(text);
  if (!match) return undefined;
  const body = blockAfter(text, match.index).replace(/\/\*[\s\S]*?\*\//g, "");
  return /(?:^|;)\s*color-scheme\s*:\s*([^;]+?)\s*(?:;|$)/.exec(body)?.[1];
}

const mediaStart = css.indexOf("@media (prefers-color-scheme: dark)");
const blocks = {
  light: declarations(css, ":root"),
  systemDark: declarations(
    mediaStart < 0 ? "" : blockAfter(css, mediaStart),
    ':root:not([data-mode="light"])',
  ),
  forcedDark: declarations(css, ':root[data-mode="dark"]'),
};

const colors = frontmatter("colors");
const rounded = frontmatter("rounded");
const spacing = frontmatter("spacing");

/** CSS token → DESIGN.md color name (without the -light / -dark suffix). */
const MODE_COLORS: Record<string, string> = {
  "--accent": "cobalto",
  "--accent-ink": "cobalto-ink",
  "--accent-soft": "cobalto-soft",
  "--on-accent": "cobalto-on",
  "--ink": "ink",
  "--ink-2": "ink-2",
  "--hair": "hair",
  "--hi": "hi",
  "--tip-bg": "tip-bg",
  "--tip-ink": "tip-ink",
  "--track": "track",
  "--kbd": "kbd",
  "--danger": "perigo",
  "--danger-ink": "perigo-ink",
  "--danger-soft": "perigo-soft",
  "--field": "field",
  "--field-line": "field-line",
  "--pg-fill": "pg-fill",
  "--pg-track": "pg-track",
};

const SURFACES = { light: "porcelana-surface", dark: "grafite-surface" };

function rgbTriplet(hex: string): string {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return `${r} ${g} ${b}`;
}

function shadow(name: string): string {
  const found = designJson.extensions.shadows.find((s) => s.name === name);
  if (!found) throw new Error(`design.json has no shadow ${name}`);
  return normalize(found.value);
}

const tertiary = Object.entries(designJson.extensions.colorMeta).filter(
  ([, meta]) => meta.role === "tertiary",
);

describe("tokens.css", () => {
  it("maps every DESIGN.md color to a token", () => {
    const mapped = new Set([
      ...Object.values(MODE_COLORS).flatMap((n) => [`${n}-light`, `${n}-dark`]),
      ...Object.values(SURFACES),
      "glifo",
      ...tertiary.map(([name]) => name),
    ]);
    expect(Object.keys(colors).sort()).toEqual([...mapped].sort());
  });

  for (const [mode, block] of Object.entries(blocks)) {
    const suffix = mode === "light" ? "light" : "dark";

    it(`has every theme color in the ${mode} block`, () => {
      for (const [token, name] of Object.entries(MODE_COLORS)) {
        expect(block.get(token), token).toBe(
          normalize(colors[`${name}-${suffix}`] ?? "missing in DESIGN.md"),
        );
      }
      expect(block.get("--surface-rgb")).toBe(
        rgbTriplet(colors[SURFACES[suffix]] ?? ""),
      );
      expect(block.get("--shadow")).toBe(shadow(`surface-${suffix}`));
      expect(block.get("--shadow-panel")).toBe(shadow(`panel-${suffix}`));
    });
  }

  it("keeps the panel shadow within the panel window's inset", () => {
    // Each layer reaches its offset plus its blur plus its spread beyond
    // the card; past PANEL_INSET it would be cut by the window edge.
    for (const [mode, block] of Object.entries(blocks)) {
      const value = block.get("--shadow-panel") ?? "";
      const layers = value.split(/,(?![^(]*\))/);
      expect(layers.length, mode).toBeGreaterThan(0);
      for (const layer of layers) {
        const [x = 0, y = 0, blur = 0, spread = 0] = (
          layer.trim().match(/^(?:-?[\d.]+(?:px)?\s*){2,4}/)?.[0] ?? ""
        )
          .trim()
          .split(/\s+/)
          .map((length) => parseFloat(length));
        const reach = Math.max(Math.abs(x), Math.abs(y)) + blur + spread;
        expect(reach, `${mode}: ${layer}`).toBeLessThanOrEqual(PANEL_INSET);
      }
    }
  });

  it("keeps the dark copies inside the system media query and equal", () => {
    expect(mediaStart).toBeGreaterThanOrEqual(0);
    expect(blocks.systemDark).toEqual(blocks.forcedDark);
  });

  it("tells the native controls the theme in every block", () => {
    // The list of a <select>, the time field, the checkbox and the scroll
    // bars are drawn by the WebView in its own light or dark look: without
    // `color-scheme` they stay light in the dark theme, with the theme's
    // light --ink on them (2026-10-16 manual test: the zone list unreadable).
    expect(colorScheme(css, ":root")).toBe("light");
    expect(
      colorScheme(
        mediaStart < 0 ? "" : blockAfter(css, mediaStart),
        ':root:not([data-mode="light"])',
      ),
    ).toBe("dark");
    expect(colorScheme(css, ':root[data-mode="dark"]')).toBe("dark");
  });

  it("sets color-scheme only in tokens.css", () => {
    const offenders = Object.entries(sources)
      .filter(([path]) => path !== "/src/themes/tokens.css")
      .filter(([path]) => !/\.test\.tsx?$/.test(path))
      .filter(([, text]) => /color-scheme\s*:|colorScheme/.test(text))
      .map(([path]) => path);
    expect(offenders).toEqual([]);
  });

  it("has the glyph color", () => {
    expect(blocks.light.get("--glyph")).toBe(normalize(colors.glifo ?? ""));
  });

  it("has the three size presets, Médio by default", () => {
    for (const size of ["s", "m", "l"]) {
      const preset = declarations(css, `:root[data-size="${size}"]`);
      expect(Object.fromEntries(preset)).toEqual({
        "--icon": spacing[`icon-${size}`],
        "--pad": spacing[`pad-${size}`],
        "--gap": spacing[`gap-${size}`],
        "--r-bar": rounded[`bar-${size}`],
      });
    }
    for (const token of ["--icon", "--pad", "--gap", "--r-bar"]) {
      expect(blocks.light.get(token)).toBe(
        declarations(css, ':root[data-size="m"]').get(token),
      );
    }
  });

  it("has the dock opacities, translucent by default", () => {
    expect(blocks.light.get("--surface-a")).toBe("0.8");
    expect(
      declarations(css, ':root[data-opacity="translucent"]').get("--surface-a"),
    ).toBe("0.8");
    expect(
      declarations(css, ':root[data-opacity="solid"]').get("--surface-a"),
    ).toBe("1");
  });

  it("has the radii", () => {
    for (const name of ["panel", "track", "segment", "kbd", "tile", "pill"]) {
      expect(blocks.light.get(`--r-${name}`), name).toBe(rounded[name]);
    }
    expect(blocks.light.get("--r-dot")).toBe(rounded.dot);
  });

  it("has the system font stacks", () => {
    for (const [token, face] of [
      ["--font", "Text"],
      ["--font-display", "Display"],
    ] as const) {
      const stack = new RegExp(
        `fontFamily: "(Segoe UI Variable ${face}[^"]+)"`,
      );
      expect(blocks.light.get(token), token).toBe(
        normalize(stack.exec(design)?.[1] ?? "missing in DESIGN.md"),
      );
    }
  });

  it("has the motion tokens", () => {
    for (const name of ["ease", "t-magnify", "t-pop"]) {
      const motion = designJson.extensions.motion.find((m) => m.name === name);
      expect(blocks.light.get(`--${name}`), name).toBe(
        normalize(motion?.value ?? "missing in design.json"),
      );
    }
  });

  it("has a static class per tile glaze", () => {
    expect(tertiary).toHaveLength(8);
    for (const [name, meta] of tertiary) {
      const match =
        /^(\.t-[\w-]+) \{ --h: (\d+); --sat: (\d+%); --l: (\d+%); \}$/.exec(
          meta.canonical,
        );
      expect(match, name).not.toBeNull();
      const [, selector = "", h, sat, l] = match ?? [];
      expect(Object.fromEntries(declarations(css, selector))).toEqual({
        "--h": h,
        "--sat": sat,
        "--l": l,
      });
      expect(normalize(colors[name] ?? "")).toBe(`hsl(${h} ${sat} ${l})`);
    }
    expect(blocks.light.get("--sat-k")).toBe("1");
    expect(declarations(css, ".illustrative").get("--sat-k")).toBe("0.55");
  });
});

// Every source file as text, read from the disk (src/test/sources.ts), the
// tests included: the glass guard looks at them too.
const sources = readSources("src", /\.(css|tsx?)$/, { tests: true });

describe("the provisional PR 1 glass", () => {
  it("reads the CSS sources as text", () => {
    expect(sources["/src/themes/tokens.css"]).toBe(css);
    expect(sources["/src/panel/Panel.css"]).toContain(".panel");
  });

  it("is gone from src/", () => {
    const needle = "--" + "glass-";
    const offenders = Object.entries(sources)
      .filter(([, text]) => text.includes(needle))
      .map(([path]) => path);
    expect(offenders).toEqual([]);
  });
});
