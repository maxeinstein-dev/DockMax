/// <reference types="node" />
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { ROOT } from "../test/sources";
import { DOCK_STYLES } from "./settings";
import { THEMES, choicesOf, themeOf } from "./themes";

const PACKS = resolve(ROOT, "src-tauri/icon-packs");

describe("themes", () => {
  it("are made of styles and bundled packs that exist", () => {
    for (const theme of THEMES) {
      expect(DOCK_STYLES).toContain(theme.style);
      if (theme.iconPack !== null) {
        expect(
          existsSync(resolve(PACKS, theme.iconPack, "pack.json")),
          theme.iconPack,
        ).toBe(true);
      }
    }
  });

  it("each makes its own choices, so the one in use is never in doubt", () => {
    expect(new Set(THEMES.map((theme) => theme.id)).size).toBe(THEMES.length);
    for (const theme of THEMES) {
      expect(themeOf(choicesOf(theme))).toBe(theme.id);
    }
  });

  it("starts at the standard one: the original style and icons", () => {
    expect(
      themeOf({ style: "native", iconPack: null, indicatorColor: null }),
    ).toBe("standard");
  });

  it("is none when one of the three choices is the user's own", () => {
    const neon = choicesOf(THEMES.find((theme) => theme.id === "neon")!);
    expect(themeOf({ ...neon, iconPack: "argila" })).toBeNull();
    expect(themeOf({ ...neon, style: "paper" })).toBeNull();
    expect(themeOf({ ...neon, indicatorColor: "#FF8A1F" })).toBeNull();
  });
});
