/// <reference types="node" />
import { randomUUID } from "node:crypto";
import { existsSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { ROOT, listFiles, readSource, readSources } from "./sources";

/* The guard of the guards: the reader every file-wide guard uses sees a
 * file created now, not a list frozen when the session started. The probe
 * is harmless to the other guards (no HTML, no motion, no color-scheme)
 * because they may read src/ while it exists. */

describe("the source reader", () => {
  it("starts at the repository root", () => {
    expect(existsSync(resolve(ROOT, "package.json"))).toBe(true);
    expect(existsSync(resolve(ROOT, "src-tauri/Cargo.toml"))).toBe(true);
  });

  it("lists the sources with paths from the root, sorted", () => {
    const files = listFiles("src");
    expect(files).toContain("/src/dock/Dock.tsx");
    expect(files).toContain("/src/panel/parts/parts.css");
    expect(files).toEqual([...files].sort());
    expect(files.some((path) => path.includes("node_modules"))).toBe(false);
  });

  it("reads a file by its key or by its path from the root", () => {
    const byKey = readSource("/src/themes/tokens.css");
    expect(byKey).toContain(":root");
    expect(readSource("src/themes/tokens.css")).toBe(byKey);
  });

  it("leaves the tests out unless asked", () => {
    const sources = readSources("src", /\.tsx?$/);
    expect(Object.keys(sources)).toContain("/src/dock/Dock.tsx");
    expect(Object.keys(sources).filter((p) => /\.test\.tsx?$/.test(p))).toEqual(
      [],
    );
    const all = readSources("src", /\.tsx?$/, { tests: true });
    expect(Object.keys(all)).toContain("/src/test/sources.test.ts");
  });

  it("sees a file created after the session started", () => {
    const name = `probe-${randomUUID()}.generated.ts`;
    const key = `/src/test/${name}`;
    const path = resolve(ROOT, `src/test/${name}`);
    const text = `export const probe = ${JSON.stringify(name)};\n`;
    expect(listFiles("src")).not.toContain(key);
    writeFileSync(path, text, "utf8");
    try {
      expect(listFiles("src")).toContain(key);
      expect(readSources("src", /\.tsx?$/)[key]).toBe(text);
    } finally {
      rmSync(path, { force: true });
    }
    expect(listFiles("src")).not.toContain(key);
  });
});
