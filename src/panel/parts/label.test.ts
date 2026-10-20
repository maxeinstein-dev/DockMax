import { describe, expect, it } from "vitest";
import { labelFromDraft } from "./label";

describe("labelFromDraft", () => {
  it("trims, and an empty label is the kind's name (null)", () => {
    expect(labelFromDraft("  Lisboa ")).toBe("Lisboa");
    expect(labelFromDraft("   ")).toBeNull();
    expect(labelFromDraft("")).toBeNull();
  });

  it("refuses more than 40 code points, counted after trimming", () => {
    expect(labelFromDraft(` ${"a".repeat(40)} `)).toBe("a".repeat(40));
    expect(labelFromDraft("a".repeat(41))).toBe("tooLong");
    expect(labelFromDraft("😀".repeat(40))).toBe("😀".repeat(40));
    expect(labelFromDraft("😀".repeat(41))).toBe("tooLong");
  });
});
