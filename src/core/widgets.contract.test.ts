import { describe, expect, expectTypeOf, it } from "vitest";
import {
  CREATABLE_KINDS,
  WIDGET_ERROR_CODES,
  WIDGET_KINDS,
  type WidgetCommand,
  type WidgetCommandSignatures,
} from "./widgets";
import { WIDGET_FIXTURES } from "./widgets.fixtures";

/* The Rust × TS contract (spec 0003 §2.6). The JSON files are what Rust
 * serializes (a Rust test compares them with the serialization); the TS
 * fixtures are typed against `widgets.ts`. Changing one side without the
 * other breaks this test or the Rust one. */

const DIR = "/src-tauri/tests/fixtures/widgets/";

const files = import.meta.glob<string>(
  "/src-tauri/tests/fixtures/widgets/*.json",
  { query: "?raw", import: "default", eager: true },
);

/** File name without folder or `.json`, as the keys of `WIDGET_FIXTURES`. */
function nameOf(path: string): string {
  return path.slice(DIR.length, -".json".length);
}

const parsed = new Map(
  Object.entries(files).map(([path, text]) => [
    nameOf(path),
    JSON.parse(text) as unknown,
  ]),
);

describe("widget contract fixtures", () => {
  it("finds the JSON fixtures", () => {
    expect(parsed.size).toBeGreaterThan(0);
  });

  it("has one TS fixture per JSON file and no other", () => {
    expect([...parsed.keys()].sort()).toEqual(
      Object.keys(WIDGET_FIXTURES).sort(),
    );
  });

  it.each(Object.entries(WIDGET_FIXTURES))(
    "%s matches its JSON file",
    (name, fixture) => {
      expect(parsed.get(name)).toEqual(fixture);
    },
  );

  it("lists every code Rust sends, which are the TS codes but notReady", () => {
    const rust = parsed.get("error-codes");
    expect(new Set(rust as string[]).size).toBe((rust as string[]).length);
    expect(new Set(rust as string[])).toEqual(
      new Set(WIDGET_ERROR_CODES.filter((code) => code !== "notReady")),
    );
    expect(WIDGET_ERROR_CODES).toContain("notReady");
  });
});

describe("widget contract constants", () => {
  it("only clock, time progress and note can be created", () => {
    expect(CREATABLE_KINDS).toEqual(["clock", "time-progress", "note"]);
    expect(CREATABLE_KINDS.every((kind) => WIDGET_KINDS.includes(kind))).toBe(
      true,
    );
  });

  it("describes every command and nothing else", () => {
    expectTypeOf<
      keyof WidgetCommandSignatures
    >().toEqualTypeOf<WidgetCommand>();
  });
});
