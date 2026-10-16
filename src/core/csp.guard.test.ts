import { describe, expect, it } from "vitest";

/* Pins the strict CSP of the webviews (no `unsafe-inline`, no
 * `unsafe-eval`): no source file outside the tests writes HTML, inline
 * style or code from a string. A value per element goes through the CSSOM,
 * on a custom property: React's `style={{ "--x": … }}` does that
 * (`style.setProperty`), so it is the one `style` allowed, with `--*` keys
 * only. */

const sources = import.meta.glob<string>(
  ["/src/**/*.{ts,tsx}", "!/src/**/*.test.{ts,tsx}"],
  { query: "?raw", import: "default", eager: true },
);

const FORBIDDEN: [name: string, pattern: RegExp][] = [
  ["dangerouslySetInnerHTML", /dangerouslySetInnerHTML/],
  [".innerHTML", /\.innerHTML\b/],
  ["outerHTML", /\bouterHTML\b/],
  ["insertAdjacentHTML", /\binsertAdjacentHTML\b/],
  ['setAttribute("style")', /setAttribute\(\s*["'`]style["'`]/],
  ["<style", /<style[\s>/]/i],
  ["eval(", /\beval\s*\(/],
  ["new Function", /\bnew\s+Function\b/],
];

const OPEN = "([{";
const CLOSE = ")]}";

/** The text between the bracket at `start` and its match. */
function enclosed(text: string, start: number): string {
  let depth = 0;
  for (let i = start; i < text.length; i++) {
    const char = text[i] as string;
    if (OPEN.includes(char)) depth++;
    else if (CLOSE.includes(char) && --depth === 0) {
      return text.slice(start + 1, i);
    }
  }
  throw new Error(`unbalanced bracket at ${start}`);
}

/** `text` split on the commas outside any bracket. */
function topLevel(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let from = 0;
  for (let i = 0; i < text.length; i++) {
    const char = text[i] as string;
    if (OPEN.includes(char)) depth++;
    else if (CLOSE.includes(char)) depth--;
    else if (char === "," && depth === 0) {
      parts.push(text.slice(from, i));
      from = i + 1;
    }
  }
  parts.push(text.slice(from));
  return parts.map((part) => part.trim()).filter((part) => part !== "");
}

/** The object literals at the top of a `style={…}` expression. */
function objectLiterals(expression: string): string[] {
  const objects: string[] = [];
  for (let i = 0; i < expression.length; i++) {
    if (expression[i] === "{") {
      const body = enclosed(expression, i);
      objects.push(body);
      i += body.length + 1;
    }
  }
  return objects;
}

/** What breaks the strict CSP in `source`. */
function violations(source: string): string[] {
  const found = FORBIDDEN.filter(([, pattern]) => pattern.test(source)).map(
    ([name]) => name,
  );
  for (const match of source.matchAll(/\bstyle=\{/g)) {
    const expression = enclosed(
      source,
      (match.index ?? 0) + match[0].length - 1,
    );
    const objects = objectLiterals(expression);
    // A style object from elsewhere can't be checked: always a literal.
    if (objects.length === 0) found.push(`style={${expression.trim()}}`);
    for (const object of objects) {
      for (const entry of topLevel(object)) {
        const key = /^(["'])(--[\w-]+)\1\s*:/.exec(entry);
        if (!key) found.push(`style key ${entry.split(":")[0]?.trim()}`);
      }
    }
  }
  return found;
}

describe("strict CSP guard", () => {
  it("reads the sources, and not the tests", () => {
    const paths = Object.keys(sources);
    expect(paths).toContain("/src/dock/Dock.tsx");
    expect(paths).toContain("/src/panel/parts/TextField.tsx");
    expect(paths.filter((path) => /\.test\.tsx?$/.test(path))).toEqual([]);
  });

  it("finds nothing in src outside the tests", () => {
    const offenders = Object.entries(sources).flatMap(([path, text]) =>
      violations(text).map((violation) => `${path}: ${violation}`),
    );
    expect(offenders).toEqual([]);
  });

  it.each([
    "el.innerHTML = html;",
    "el.outerHTML = html;",
    'el.insertAdjacentHTML("beforeend", html);',
    'el.setAttribute("style", "color: red");',
    "el.setAttribute('style', css);",
    "<div dangerouslySetInnerHTML={{ __html: html }} />",
    "<style>{css}</style>",
    'eval("1");',
    'new Function("return 1");',
    '<div style={{ color: "red" }} />',
    '<div style={{ "--x": "1", color: "red" }} />',
    "<div style={{ ...base }} />",
    "<div style={styles} />",
  ])("rejects %s", (code) => {
    expect(violations(code)).not.toEqual([]);
  });

  it.each([
    '<div style={{ "--s": String(scale), "--lift": `${lift}px` } as CSSProperties} />',
    '<div style={at ? ({ "--tip-x": `${at.x}px` } as CSSProperties) : undefined} />',
    "el.style.setProperty('--x', '1px');",
    "const evaluate = (a: number) => a;",
  ])("accepts %s", (code) => {
    expect(violations(code)).toEqual([]);
  });
});
