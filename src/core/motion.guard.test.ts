import { describe, expect, it } from "vitest";
import { readSources } from "../test/sources";

/* Pins "Reduzir movimento" for every stylesheet: a selector that moves
 * (`animation`, `transition` or their `-duration`) is also in a
 * `@media (prefers-reduced-motion: reduce)` block of the same file that
 * sets the same family to `none` or to zero. `animation-name` alone doesn't
 * move: a selector that only swaps the keyframes follows its base rule. */

const sheets = readSources("src", /\.css$/);

type Family = "animation" | "transition";
const MOVING = /^(animation|transition)(-duration)?$/;
const STILL = /^(none|0m?s?)(\s*,\s*(none|0m?s?))*$/;

interface Rule {
  selectors: string[];
  declarations: [property: string, value: string][];
}

/** The rules of `css`, with those inside a reduced-motion block apart.
 * `@keyframes` are skipped; other at-rules count as plain rules. */
function parse(css: string): { rules: Rule[]; reduced: Rule[] } {
  const rules: Rule[] = [];
  const reduced: Rule[] = [];
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");

  function walk(from: number, to: number, into: Rule[]) {
    let i = from;
    while (i < to) {
      const open = text.indexOf("{", i);
      if (open < 0 || open >= to) return;
      const prelude = text.slice(i, open).trim();
      let depth = 1;
      let close = open + 1;
      for (; close < to && depth > 0; close++) {
        if (text[close] === "{") depth++;
        else if (text[close] === "}") depth--;
      }
      const end = close - 1;
      if (prelude.startsWith("@keyframes")) {
        // Frames, not selectors.
      } else if (prelude.startsWith("@")) {
        const target = /prefers-reduced-motion:\s*reduce/.test(prelude)
          ? reduced
          : into;
        walk(open + 1, end, target);
      } else {
        into.push({
          selectors: prelude
            .split(",")
            .map((s) => s.trim().replace(/\s+/g, " ")),
          declarations: text
            .slice(open + 1, end)
            .split(";")
            .map((d) => d.split(/:(.*)/s))
            .filter((parts) => parts.length > 1)
            .map(([p, v]) => [p!.trim(), (v ?? "").trim()]),
        });
      }
      i = close;
    }
  }

  walk(0, text.length, rules);
  return { rules, reduced };
}

/** What moves in `css` without a reduced-motion rule that stops it. */
function unguarded(css: string): string[] {
  const { rules, reduced } = parse(css);
  const stopped = new Set<string>();
  for (const rule of reduced) {
    for (const [property, value] of rule.declarations) {
      const family = MOVING.exec(property)?.[1] as Family | undefined;
      if (family && STILL.test(value)) {
        for (const selector of rule.selectors) {
          stopped.add(`${selector} ${family}`);
        }
      }
    }
  }
  const found: string[] = [];
  for (const rule of rules) {
    for (const [property, value] of rule.declarations) {
      const family = MOVING.exec(property)?.[1] as Family | undefined;
      if (!family || STILL.test(value)) continue;
      for (const selector of rule.selectors) {
        if (!stopped.has(`${selector} ${family}`)) {
          found.push(`${selector} { ${property} }`);
        }
      }
    }
  }
  return found;
}

describe("reduced motion guard", () => {
  it("reads every stylesheet", () => {
    expect(Object.keys(sheets)).toEqual(
      expect.arrayContaining(["/src/dock/Dock.css", "/src/panel/Panel.css"]),
    );
  });

  it("stops every animation and transition in src", () => {
    const offenders = Object.entries(sheets).flatMap(([path, css]) =>
      unguarded(css).map((rule) => `${path}: ${rule}`),
    );
    expect(offenders).toEqual([]);
  });

  it("finds what moves without being stopped", () => {
    expect(unguarded(".a { transition: opacity 0.2s; }")).toEqual([
      ".a { transition }",
    ]);
    expect(
      unguarded(`
        .a, .b { animation: pop 1s; }
        @media (prefers-reduced-motion: reduce) { .a { animation: none; } }
      `),
    ).toEqual([".b { animation }"]);
    // Stopping the other family doesn't count.
    expect(
      unguarded(`
        .a { transition: width 1s; }
        @media (prefers-reduced-motion: reduce) { .a { animation: none; } }
      `),
    ).toEqual([".a { transition }"]);
  });

  it("accepts a stopped rule, keyframes and a swap of keyframes", () => {
    expect(
      unguarded(`
        .a { transition: width 1s; animation: pop 1s; }
        .top .a { animation-name: pop-down; }
        @keyframes pop { from { opacity: 0; } }
        @media (prefers-reduced-motion: reduce) {
          .a { transition: none; animation-duration: 0s; }
        }
      `),
    ).toEqual([]);
  });
});
