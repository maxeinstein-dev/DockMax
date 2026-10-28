import { describe, expect, it } from "vitest";
import { daysLeft, longDay, parseDay, targetDay, todayOf } from "./countdown";
import type { CountdownConfig } from "./widgets";

function day(text: string): number {
  const parsed = parseDay(text);
  if (parsed === null) throw new Error(text);
  return parsed;
}

function config(date: string | null, yearly = false): CountdownConfig {
  return { date, yearly };
}

describe("countdown", () => {
  const today = day("2026-10-28");

  it("reads only real days in the settings form", () => {
    expect(parseDay("2026-12-20")).not.toBeNull();
    for (const bad of ["2026-02-30", "20/12/2026", "2026-1-5", ""]) {
      expect(parseDay(bad)).toBeNull();
    }
  });

  it("counts the days left and gone, as Rust does", () => {
    expect(daysLeft(config("2026-12-20"), today)).toBe(53);
    expect(daysLeft(config("2026-10-28"), today)).toBe(0);
    expect(daysLeft(config("2026-10-25"), today)).toBe(-3);
    expect(daysLeft(config(null), today)).toBeNull();
    expect(daysLeft(config("2200-01-01"), today)).toBeNull();
  });

  it("counts a yearly date to its next return", () => {
    expect(daysLeft(config("1990-03-10", true), today)).toBe(133);
    expect(daysLeft(config("1990-11-02", true), today)).toBe(5);
    expect(daysLeft(config("1990-10-28", true), today)).toBe(0);
    expect(targetDay(config("2024-02-29", true), today)).toBe(
      day("2027-02-28"),
    );
    expect(targetDay(config("2024-02-29", true), day("2028-01-10"))).toBe(
      day("2028-02-29"),
    );
  });

  it("takes today from the local clock", () => {
    expect(todayOf(new Date(2026, 9, 28, 23, 59))).toBe(today);
    expect(todayOf(new Date(2026, 9, 28, 0, 1))).toBe(today);
  });

  it("says the day in words", () => {
    expect(longDay(day("2026-12-20"))).toBe("domingo, 20 de dezembro de 2026");
  });
});
