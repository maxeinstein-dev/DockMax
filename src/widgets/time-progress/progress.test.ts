import { afterAll, beforeAll, describe, expect, it } from "vitest";
import type { Period } from "../../core/widgets";
import {
  periodBounds,
  progressOf,
  remainingParts,
  type ProgressConfig,
} from "./progress";

/* The periods use the system's local zone, so each block pins it through
 * `TZ`, which Node re-reads when it changes, on Windows too. Deleting `TZ`
 * doesn't bring the original zone back, so it is restored by name. */
// `src` doesn't load the Node types; only `TZ` is needed here.
declare const process: { env: Record<string, string | undefined> };

function inZone(timeZone: string): void {
  const before = process.env.TZ;
  const system = new Intl.DateTimeFormat().resolvedOptions().timeZone;
  beforeAll(() => {
    process.env.TZ = timeZone;
  });
  afterAll(() => {
    process.env.TZ = before ?? system;
  });
}

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

const config = (overrides: Partial<ProgressConfig> = {}): ProgressConfig => ({
  weekStart: "monday",
  workday: { start: "09:00", end: "18:00" },
  ...overrides,
});

/** A local instant. */
const at = (y: number, m: number, d: number, h = 0, min = 0, s = 0, ms = 0) =>
  new Date(y, m - 1, d, h, min, s, ms);

const running = (period: Period, now: Date, c = config()) => {
  const progress = progressOf(period, now, c);
  if (progress.state !== "running") throw new Error(`${period} is off`);
  return progress;
};

describe("calendar periods", () => {
  inZone("America/Sao_Paulo");

  it("runs in the local zone", () => {
    expect(at(2026, 10, 15).getTimezoneOffset()).toBe(180);
  });

  it("day: from local midnight to the next", () => {
    expect(periodBounds("day", at(2026, 10, 15, 13), config())).toEqual({
      start: at(2026, 10, 15),
      end: at(2026, 10, 16),
    });
    expect(running("day", at(2026, 10, 15, 18))).toEqual({
      state: "running",
      fraction: 0.75,
      remainingMs: 6 * HOUR,
    });
  });

  it("week: from Monday 00:00 by default", () => {
    // 2026-10-15 is a Thursday.
    expect(periodBounds("week", at(2026, 10, 15, 12), config())).toEqual({
      start: at(2026, 10, 12),
      end: at(2026, 10, 19),
    });
    // Sunday is the last day of a Monday week.
    expect(periodBounds("week", at(2026, 10, 18, 23), config())).toEqual({
      start: at(2026, 10, 12),
      end: at(2026, 10, 19),
    });
    expect(running("week", at(2026, 10, 15, 12)).fraction).toBe(3.5 / 7);
  });

  it("week: from Sunday 00:00 when set", () => {
    const sunday = config({ weekStart: "sunday" });
    expect(periodBounds("week", at(2026, 10, 15, 12), sunday)).toEqual({
      start: at(2026, 10, 11),
      end: at(2026, 10, 18),
    });
    expect(periodBounds("week", at(2026, 10, 18), sunday)).toEqual({
      start: at(2026, 10, 18),
      end: at(2026, 10, 25),
    });
    expect(running("week", at(2026, 10, 18), sunday).fraction).toBe(0);
  });

  it("week: across a month and a year", () => {
    // Thursday 2026-12-31: the week runs Monday 28 to Monday 4 January.
    expect(periodBounds("week", at(2026, 12, 31), config())).toEqual({
      start: at(2026, 12, 28),
      end: at(2027, 1, 4),
    });
  });

  it("month: by the calendar, whatever its length", () => {
    expect(periodBounds("month", at(2026, 10, 15), config())).toEqual({
      start: at(2026, 10, 1),
      end: at(2026, 11, 1),
    });
    expect(running("month", at(2026, 4, 16)).fraction).toBe(15 / 30);
    expect(running("month", at(2026, 2, 15)).fraction).toBe(14 / 28);
  });

  it("year: from 1 January to the next", () => {
    expect(periodBounds("year", at(2026, 10, 15), config())).toEqual({
      start: at(2026, 1, 1),
      end: at(2027, 1, 1),
    });
    expect(running("year", at(2026, 7, 2, 12)).fraction).toBe(182.5 / 365);
  });

  it("turns the month: last millisecond near 1, then 0", () => {
    const last = running("month", at(2026, 1, 31, 23, 59, 59, 999));
    expect(last.fraction).toBeCloseTo(1, 9);
    expect(last.remainingMs).toBe(1);
    expect(running("month", at(2026, 2, 1))).toEqual({
      state: "running",
      fraction: 0,
      remainingMs: 28 * DAY,
    });
  });

  it("turns the year: last millisecond near 1, then 0", () => {
    const last = running("year", at(2026, 12, 31, 23, 59, 59, 999));
    expect(last.fraction).toBeCloseTo(1, 9);
    expect(last.remainingMs).toBe(1);
    expect(running("year", at(2027, 1, 1))).toEqual({
      state: "running",
      fraction: 0,
      remainingMs: 365 * DAY,
    });
  });

  it("starts at exactly 0 on every boundary", () => {
    const monday = at(2026, 6, 1); // a Monday, the 1st of a month
    for (const period of ["day", "week", "month"] as const) {
      expect(running(period, monday).fraction).toBe(0);
    }
    expect(running("year", at(2026, 1, 1)).fraction).toBe(0);
  });

  it("leap year: 29 February is a day of a 29-day month and a 366-day year", () => {
    const leapDay = at(2028, 2, 29, 12);
    expect(periodBounds("day", leapDay, config())).toEqual({
      start: at(2028, 2, 29),
      end: at(2028, 3, 1),
    });
    expect(running("month", leapDay).fraction).toBe(28.5 / 29);
    expect(running("month", at(2028, 3, 1))).toMatchObject({ fraction: 0 });
    expect(running("year", at(2028, 12, 31, 12)).fraction).toBe(365.5 / 366);
    expect(running("year", at(2028, 1, 1)).remainingMs).toBe(366 * DAY);
  });

  it("accepts milliseconds as well as a Date", () => {
    const now = at(2026, 10, 15, 13);
    expect(progressOf("day", now.getTime(), config())).toEqual(
      progressOf("day", now, config()),
    );
  });
});

describe("workday", () => {
  inZone("America/Sao_Paulo");

  // Thursday 2026-10-15.
  const thursday = (h: number, min = 0, s = 0) => at(2026, 10, 15, h, min, s);

  it("is off before the start", () => {
    expect(progressOf("workday", thursday(8, 59, 59), config())).toEqual({
      state: "off",
    });
  });

  it("runs from the start, at exactly 0", () => {
    expect(running("workday", thursday(9))).toEqual({
      state: "running",
      fraction: 0,
      remainingMs: 9 * HOUR,
    });
    expect(running("workday", thursday(13, 30)).fraction).toBe(0.5);
  });

  it("reads exactly 1 at the end, then is off", () => {
    expect(running("workday", thursday(18))).toEqual({
      state: "running",
      fraction: 1,
      remainingMs: 0,
    });
    expect(progressOf("workday", thursday(18, 0, 1), config())).toEqual({
      state: "off",
    });
  });

  it("is off on the weekend, even within the hours", () => {
    expect(progressOf("workday", at(2026, 10, 17, 10), config())).toEqual({
      state: "off",
    });
    expect(progressOf("workday", at(2026, 10, 18, 10), config())).toEqual({
      state: "off",
    });
    expect(running("workday", at(2026, 10, 16, 10)).fraction).toBe(1 / 9);
  });

  it("uses the configured hours", () => {
    const early = config({ workday: { start: "07:30", end: "16:30" } });
    expect(running("workday", thursday(7, 30), early).fraction).toBe(0);
    expect(running("workday", thursday(12), early).fraction).toBe(0.5);
    expect(progressOf("workday", thursday(17), early).state).toBe("off");
  });

  it("is off with hours Rust would refuse, instead of a wrong ring", () => {
    for (const workday of [
      { start: "18:00", end: "09:00" },
      { start: "09:00", end: "09:00" },
      { start: "9:00", end: "18:00" },
      { start: "09:00", end: "24:00" },
    ]) {
      expect(progressOf("workday", thursday(12), config({ workday }))).toEqual({
        state: "off",
      });
    }
  });
});

describe("daylight saving: the day's real length", () => {
  describe("America/New_York", () => {
    inZone("America/New_York");

    it("runs in New York time", () => {
      expect(at(2026, 1, 15).getTimezoneOffset()).toBe(300);
    });

    it("8 March 2026 lasts 23 h", () => {
      const { start, end } = periodBounds("day", at(2026, 3, 8, 12), config())!;
      expect(end.getTime() - start.getTime()).toBe(23 * HOUR);
      // 12:00 is 11 real hours in (the 02:00 hour was skipped), 12 left.
      expect(running("day", at(2026, 3, 8, 12))).toEqual({
        state: "running",
        fraction: 11 / 23,
        remainingMs: 12 * HOUR,
      });
    });

    it("1 November 2026 lasts 25 h", () => {
      const { start, end } = periodBounds(
        "day",
        at(2026, 11, 1, 12),
        config(),
      )!;
      expect(end.getTime() - start.getTime()).toBe(25 * HOUR);
      expect(running("day", at(2026, 11, 1, 12)).fraction).toBe(13 / 25);
    });

    it("the week holding the change keeps its local midnights", () => {
      const { start, end } = periodBounds("week", at(2026, 3, 8), config())!;
      expect(start).toEqual(at(2026, 3, 2));
      expect(end).toEqual(at(2026, 3, 9));
      expect(end.getTime() - start.getTime()).toBe(7 * DAY - HOUR);
    });

    it("the workday on a changed day still spans its local hours", () => {
      // Monday 9 March, the first workday after the change.
      expect(running("workday", at(2026, 3, 9, 9)).fraction).toBe(0);
      expect(running("workday", at(2026, 3, 9, 18)).remainingMs).toBe(0);
    });
  });

  describe("Europe/Lisbon", () => {
    inZone("Europe/Lisbon");

    it("29 March 2026 lasts 23 h", () => {
      const { start, end } = periodBounds("day", at(2026, 3, 29, 9), config())!;
      expect(end.getTime() - start.getTime()).toBe(23 * HOUR);
    });

    it("25 October 2026 lasts 25 h", () => {
      const { start, end } = periodBounds(
        "day",
        at(2026, 10, 25, 9),
        config(),
      )!;
      expect(end.getTime() - start.getTime()).toBe(25 * HOUR);
    });

    it("October 2026 holds the extra hour", () => {
      const { start, end } = periodBounds("month", at(2026, 10, 1), config())!;
      expect(end.getTime() - start.getTime()).toBe(31 * DAY + HOUR);
    });
  });
});

describe("daylight saving at midnight: America/Santiago", () => {
  // Chile changes the clock at local midnight. On 6 September 2026 00:00
  // doesn't exist (the day starts at 01:00 and lasts 23 h); on 4 April 2026
  // the clock goes from 24:00 back to 23:00, so that day lasts 25 h.
  inZone("America/Santiago");

  it("runs in Santiago time", () => {
    expect(at(2026, 7, 15).getTimezoneOffset()).toBe(240);
    expect(at(2026, 1, 15).getTimezoneOffset()).toBe(180);
  });

  describe("6 September 2026, without a midnight", () => {
    // The first instant of the day: 00:00 −04 is 01:00 −03.
    const dayStart = Date.UTC(2026, 8, 6, 4);

    it("day: starts at 01:00 and lasts 23 h", () => {
      const { start, end } = periodBounds("day", at(2026, 9, 6, 12), config())!;
      expect(start.getTime()).toBe(dayStart);
      expect(start.getHours()).toBe(1);
      expect(end).toEqual(at(2026, 9, 7));
      expect(end.getTime() - start.getTime()).toBe(23 * HOUR);
      // 12:00 is 11 real hours in, 12 left.
      expect(running("day", at(2026, 9, 6, 12))).toEqual({
        state: "running",
        fraction: 11 / 23,
        remainingMs: 12 * HOUR,
      });
    });

    it("day: the day before ends where this one starts", () => {
      expect(running("day", new Date(dayStart - 1))).toMatchObject({
        remainingMs: 1,
      });
      expect(running("day", new Date(dayStart))).toEqual({
        state: "running",
        fraction: 0,
        remainingMs: 23 * HOUR,
      });
    });

    it("week: a Monday week holds 7 days less an hour", () => {
      const { start, end } = periodBounds(
        "week",
        at(2026, 9, 6, 12),
        config(),
      )!;
      expect(start).toEqual(at(2026, 8, 31));
      expect(end).toEqual(at(2026, 9, 7));
      expect(end.getTime() - start.getTime()).toBe(7 * DAY - HOUR);
    });

    it("week: a Sunday week starts at 01:00, at exactly 0", () => {
      const sunday = config({ weekStart: "sunday" });
      const { start, end } = periodBounds("week", at(2026, 9, 6, 12), sunday)!;
      expect(start.getTime()).toBe(dayStart);
      expect(end).toEqual(at(2026, 9, 13));
      expect(running("week", new Date(dayStart), sunday).fraction).toBe(0);
    });

    it("month: September holds 30 days less an hour", () => {
      const { start, end } = periodBounds("month", at(2026, 9, 6), config())!;
      expect(start).toEqual(at(2026, 9, 1));
      expect(end).toEqual(at(2026, 10, 1));
      expect(end.getTime() - start.getTime()).toBe(30 * DAY - HOUR);
    });
  });

  describe("4 April 2026, with a repeated hour before midnight", () => {
    // The second 23:30 of the day, at −04.
    const secondLateHour = new Date(Date.UTC(2026, 3, 5, 3, 30));

    it("day: lasts 25 h", () => {
      const { start, end } = periodBounds("day", at(2026, 4, 4, 12), config())!;
      expect(start).toEqual(at(2026, 4, 4));
      expect(end).toEqual(at(2026, 4, 5));
      expect(end.getTime() - start.getTime()).toBe(25 * HOUR);
      expect(running("day", at(2026, 4, 4, 12))).toEqual({
        state: "running",
        fraction: 12 / 25,
        remainingMs: 13 * HOUR,
      });
    });

    it("day: the repeated hour still belongs to 4 April", () => {
      expect(secondLateHour.getDate()).toBe(4);
      expect(running("day", secondLateHour)).toEqual({
        state: "running",
        fraction: 24.5 / 25,
        remainingMs: HOUR / 2,
      });
    });

    it("week: holds the extra hour", () => {
      // 4 April 2026 is a Saturday.
      const monday = periodBounds("week", secondLateHour, config())!;
      expect(monday.start).toEqual(at(2026, 3, 30));
      expect(monday.end).toEqual(at(2026, 4, 6));
      expect(monday.end.getTime() - monday.start.getTime()).toBe(
        7 * DAY + HOUR,
      );
      const sunday = periodBounds(
        "week",
        secondLateHour,
        config({ weekStart: "sunday" }),
      )!;
      expect(sunday.start).toEqual(at(2026, 3, 29));
      expect(sunday.end).toEqual(at(2026, 4, 5));
      expect(sunday.end.getTime() - sunday.start.getTime()).toBe(
        7 * DAY + HOUR,
      );
    });

    it("month: April holds 30 days and an hour", () => {
      const { start, end } = periodBounds("month", secondLateHour, config())!;
      expect(start).toEqual(at(2026, 4, 1));
      expect(end).toEqual(at(2026, 5, 1));
      expect(end.getTime() - start.getTime()).toBe(30 * DAY + HOUR);
    });
  });
});

describe("remainingParts", () => {
  const MIN = 60_000;

  it("reads hours and minutes under a day", () => {
    expect(remainingParts(3 * HOUR + 12 * MIN)).toEqual([
      { value: 3, unit: "hour" },
      { value: 12, unit: "minute" },
    ]);
    expect(remainingParts(45 * MIN)).toEqual([{ value: 45, unit: "minute" }]);
    expect(remainingParts(2 * HOUR)).toEqual([{ value: 2, unit: "hour" }]);
  });

  it("reads days and hours from a day up", () => {
    expect(remainingParts(5 * DAY)).toEqual([{ value: 5, unit: "day" }]);
    expect(remainingParts(DAY + 3 * HOUR)).toEqual([
      { value: 1, unit: "day" },
      { value: 3, unit: "hour" },
    ]);
    // A 25-hour day, just started.
    expect(remainingParts(25 * HOUR)).toEqual([
      { value: 1, unit: "day" },
      { value: 1, unit: "hour" },
    ]);
  });

  it("rounds the last unit up, carrying over", () => {
    expect(remainingParts(3 * HOUR + 11 * MIN + 30_000)).toEqual([
      { value: 3, unit: "hour" },
      { value: 12, unit: "minute" },
    ]);
    expect(remainingParts(30_000)).toEqual([{ value: 1, unit: "minute" }]);
    expect(remainingParts(59 * MIN + 1)).toEqual([{ value: 1, unit: "hour" }]);
    expect(remainingParts(DAY - 30_000)).toEqual([{ value: 1, unit: "day" }]);
    expect(remainingParts(7 * DAY - MIN)).toEqual([{ value: 7, unit: "day" }]);
  });

  it("reads 0 min only when nothing is left", () => {
    expect(remainingParts(0)).toEqual([{ value: 0, unit: "minute" }]);
    expect(remainingParts(-5)).toEqual([{ value: 0, unit: "minute" }]);
    expect(remainingParts(1)).toEqual([{ value: 1, unit: "minute" }]);
  });
});
