import { describe, expect, it } from "vitest";
import { clockText } from "./status";

describe("clockText", () => {
  it("shows the time in 24 hours and the date as day, month and year, in PT-BR", () => {
    expect(clockText(new Date(2026, 9, 22, 18, 5), "pt-BR")).toEqual({
      time: "18:05",
      date: "22/10/2026",
    });
  });

  it("pads the time and the date to two digits", () => {
    expect(clockText(new Date(2026, 0, 3, 7, 4), "pt-BR")).toEqual({
      time: "07:04",
      date: "03/01/2026",
    });
  });

  it("never shows 24:00 at midnight", () => {
    expect(clockText(new Date(2026, 9, 23, 0, 0), "pt-BR").time).toBe("00:00");
  });

  it("follows the language it is given", () => {
    expect(clockText(new Date(2026, 9, 22, 18, 5), "en-GB").date).toBe(
      "22/10/2026",
    );
    expect(clockText(new Date(2026, 9, 22, 18, 5), "en-US").date).toBe(
      "10/22/2026",
    );
  });
});
