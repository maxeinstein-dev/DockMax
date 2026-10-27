import { describe, expect, it } from "vitest";
import css from "./SettingsSections.css?raw";

// The finish of the sections (spec 0014 R4) is the Configurações app's
// only: the panel shares the views and keeps its own look. Every selector
// of the file starts inside the app's card.

function selectors(text: string): string[] {
  const plain = text.replace(/\/\*[\s\S]*?\*\//g, "");
  return [...plain.matchAll(/([^{}]+)\{[^{}]*\}/g)]
    .flatMap((match) => (match[1] ?? "").split(","))
    .map((selector) => selector.trim())
    .filter((selector) => selector.length > 0);
}

describe("SettingsSections.css", () => {
  it("styles only inside the Configurações app's card", () => {
    const found = selectors(css);
    expect(found.length).toBeGreaterThan(20);
    expect(found.filter((s) => !s.startsWith(".settings-card "))).toEqual([]);
  });
});
