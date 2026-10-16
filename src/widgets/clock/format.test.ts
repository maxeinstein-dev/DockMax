import { afterAll, beforeAll, describe, expect, it } from "vitest";
import type { ClockConfig } from "../../core/widgets";
import {
  formatClock,
  formatDockTime,
  msUntilNextMinute,
  resolveTimeZone,
  systemTimeZone,
} from "./format";

/* `TZ` changes the system zone, which Node re-reads when it changes. Deleting
 * it doesn't bring the original zone back, so it is restored by name. */
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

const clock = (config: Partial<ClockConfig> = {}): ClockConfig => ({
  timeZone: "America/Sao_Paulo",
  hourCycle: "h23",
  showSeconds: false,
  iconStyle: "digital",
  ...config,
});

// 2026-10-15 18:07:05.250 UTC = 15:07:05 in São Paulo (UTC−3, no DST).
const AFTERNOON = Date.UTC(2026, 9, 15, 18, 7, 5, 250);
// 2026-10-15 03:04:00 UTC = 00:04 in São Paulo.
const AFTER_MIDNIGHT = Date.UTC(2026, 9, 15, 3, 4);

describe("24 h", () => {
  it("shows HH:mm on the icon, padded, never seconds", () => {
    expect(formatDockTime(AFTERNOON, clock())).toBe("15:07");
    expect(formatDockTime(AFTER_MIDNIGHT, clock())).toBe("00:04");
    expect(formatDockTime(AFTERNOON, clock({ showSeconds: true }))).toBe(
      "15:07",
    );
  });

  it("adds seconds to the panel time only when asked, without AM/PM", () => {
    const reading = formatClock(AFTERNOON, clock({ showSeconds: true }));
    expect(reading.time).toBe("15:07:05");
    expect(reading.dayPeriod).toBeNull();
    expect(formatClock(AFTERNOON, clock()).time).toBe("15:07");
  });

  it("writes the date in full, in pt-BR by default", () => {
    expect(formatClock(AFTERNOON, clock()).date).toBe(
      "quinta-feira, 15 de outubro de 2026",
    );
  });

  it("accepts a Date as well as milliseconds", () => {
    expect(formatClock(new Date(AFTERNOON), clock())).toEqual(
      formatClock(AFTERNOON, clock()),
    );
  });
});

describe("12 h", () => {
  it("shows h:mm on the icon, without the period", () => {
    expect(formatDockTime(AFTERNOON, clock({ hourCycle: "h12" }))).toBe("3:07");
  });

  it("shows midnight as 12, not 0", () => {
    expect(formatDockTime(AFTER_MIDNIGHT, clock({ hourCycle: "h12" }))).toBe(
      "12:04",
    );
  });

  it("gives the period apart, for the panel", () => {
    const pm = formatClock(
      AFTERNOON,
      clock({ hourCycle: "h12", showSeconds: true }),
    );
    expect(pm.time).toBe("3:07:05");
    expect(pm.dayPeriod).toBe("PM");
    expect(
      formatClock(AFTER_MIDNIGHT, clock({ hourCycle: "h12" })).dayPeriod,
    ).toBe("AM");
  });
});

describe("time zones", () => {
  it("formats in the configured zone, date included", () => {
    // 03:07 the next day in Tokyo (UTC+9).
    const tokyo = formatClock(AFTERNOON, clock({ timeZone: "Asia/Tokyo" }));
    expect(tokyo.icon).toBe("03:07");
    expect(tokyo.date).toBe("sexta-feira, 16 de outubro de 2026");
    expect(tokyo).toMatchObject({
      timeZone: "Asia/Tokyo",
      system: false,
      timeZoneFallback: false,
    });
  });

  it("follows the system zone when none is set", () => {
    const reading = formatClock(AFTERNOON, clock({ timeZone: null }));
    expect(reading).toMatchObject({
      timeZone: systemTimeZone(),
      system: true,
      timeZoneFallback: false,
    });
    expect(reading.icon).toBe(
      formatDockTime(AFTERNOON, clock({ timeZone: systemTimeZone() })),
    );
  });

  it("falls back to the system zone, flagged, for a zone Intl doesn't know", () => {
    const reading = formatClock(AFTERNOON, clock({ timeZone: "Mars/Olympus" }));
    expect(reading).toMatchObject({
      timeZone: systemTimeZone(),
      system: true,
      timeZoneFallback: true,
    });
    // The system time, never an invented one.
    expect(reading.icon).toBe(
      formatDockTime(AFTERNOON, clock({ timeZone: null })),
    );
  });

  it("reports the canonical name Intl resolves", () => {
    expect(resolveTimeZone("asia/tokyo").timeZone).toBe("Asia/Tokyo");
  });
});

describe("a system zone Intl doesn't identify", () => {
  // An unknown `TZ` makes `Intl` report `Etc/Unknown`, a name it refuses
  // itself as a `timeZone`.
  inZone("Mars/Base");

  /** What `Intl` shows with its own default zone, without a `timeZone`. */
  const intlDefault = (instant: number) =>
    new Intl.DateTimeFormat("pt-BR", {
      hourCycle: "h23",
      hour: "2-digit",
      minute: "2-digit",
    }).format(instant);

  it("is reported as unidentified", () => {
    expect(systemTimeZone()).toBeNull();
    expect(resolveTimeZone(null)).toEqual({
      timeZone: null,
      system: true,
      timeZoneFallback: false,
    });
  });

  it("formats with Intl's default zone when the clock follows the system", () => {
    const reading = formatClock(AFTERNOON, clock({ timeZone: null }));
    expect(reading).toMatchObject({
      timeZone: null,
      system: true,
      timeZoneFallback: false,
    });
    expect(reading.icon).toBe(intlDefault(AFTERNOON));
    expect(reading.date).toBe(
      new Intl.DateTimeFormat("pt-BR", { dateStyle: "full" }).format(AFTERNOON),
    );
  });

  it("formats with Intl's default zone when the configured zone is unknown too", () => {
    const reading = formatClock(
      AFTERNOON,
      clock({ timeZone: "Mars/Olympus", hourCycle: "h12" }),
    );
    expect(reading).toMatchObject({
      timeZone: null,
      system: true,
      timeZoneFallback: true,
    });
    expect(formatDockTime(AFTERNOON, clock({ timeZone: "Mars/Olympus" }))).toBe(
      intlDefault(AFTERNOON),
    );
  });

  it("still formats a zone Intl knows", () => {
    expect(formatClock(AFTERNOON, clock())).toMatchObject({
      icon: "15:07",
      timeZone: "America/Sao_Paulo",
      system: false,
    });
  });
});

describe("msUntilNextMinute", () => {
  it("lands on the next minute boundary", () => {
    expect(msUntilNextMinute(AFTERNOON)).toBe(60_000 - 5_250);
    expect((AFTERNOON + msUntilNextMinute(AFTERNOON)) % 60_000).toBe(0);
  });

  it("waits a whole minute when already on a boundary", () => {
    expect(msUntilNextMinute(Date.UTC(2026, 9, 15, 18, 8))).toBe(60_000);
  });

  it("is 1 ms on the last millisecond of a minute, and the icon turns", () => {
    const last = Date.UTC(2026, 9, 15, 18, 7, 59, 999);
    expect(msUntilNextMinute(new Date(last))).toBe(1);
    expect(formatDockTime(last, clock())).toBe("15:07");
    expect(formatDockTime(last + 1, clock())).toBe("15:08");
  });

  it("turns the hour and the day at the boundary too", () => {
    // 23:59:59.999 → 00:00 in São Paulo, and the date changes with it.
    const last = Date.UTC(2026, 9, 16, 2, 59, 59, 999);
    expect(formatDockTime(last, clock())).toBe("23:59");
    expect(formatDockTime(last + 1, clock())).toBe("00:00");
    expect(formatClock(last + 1, clock()).date).toBe(
      "sexta-feira, 16 de outubro de 2026",
    );
  });

  it("stays positive before 1970", () => {
    expect(msUntilNextMinute(-1)).toBe(1);
    expect(msUntilNextMinute(-60_000)).toBe(60_000);
  });
});
