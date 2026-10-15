import type { Period, TimeProgressConfig } from "../../core/widgets";

/* Time progress (spec 0003 §2.2, Progresso do tempo). Pure: the instant
 * comes in, so tests never wait for the real clock.
 *
 * Every boundary is in the system's local zone (the widget has no zone of
 * its own) and is built from local calendar fields with `new Date(y, m, d)`,
 * never by adding 24 h: a day with a daylight-saving change then lasts its
 * real 23 or 25 hours, and the week, month and year include that hour. */

/** An instant as a `Date` or as epoch milliseconds. */
export type Instant = Date | number;

/** What a period's ring and line show. */
export type Progress =
  | {
      state: "running";
      /** Elapsed share of the period, from 0 at its start to 1 at its end. */
      fraction: number;
      /** Milliseconds until the period ends. */
      remainingMs: number;
    }
  /** A workday period outside working hours or on a weekend: the panel
   * shows "fora do expediente", without a percentage. */
  | { state: "off" };

/** Start and end of the period around an instant. */
export interface PeriodBounds {
  start: Date;
  end: Date;
}

/** What the periods need from the instance's config. */
export type ProgressConfig = Pick<TimeProgressConfig, "weekStart" | "workday">;

/** Local midnight at the start of a calendar day. The month and day may
 * overflow (day 0, day 32, month 12): `Date` carries them over. Built with
 * `setFullYear` because `new Date(y, …)` maps years 0–99 to 1900–1999. */
function localDate(
  year: number,
  month: number,
  day: number,
  hours = 0,
  minutes = 0,
): Date {
  const date = new Date(2000, 0, 1, hours, minutes);
  date.setFullYear(year, month, day);
  // Set the time again: the day may have another offset than 2000-01-01.
  date.setHours(hours, minutes, 0, 0);
  return date;
}

/** `HH:mm` → minutes after midnight, or null when malformed. Rust validates
 * the workday; this only keeps a bad value from producing a wrong ring. */
function parseHhMm(value: string): number | null {
  const match = /^(\d{2}):(\d{2})$/.exec(value);
  if (!match) return null;
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 23 || minutes > 59) return null;
  return hours * 60 + minutes;
}

const SUNDAY = 0;
const SATURDAY = 6;

/** Bounds of a calendar period around `now`: `[start, end)`, so the end of
 * one period is the start of the next. The workday is `[start, end]` on a
 * weekday, so its last instant reads 100%; null outside it. */
export function periodBounds(
  period: Period,
  now: Instant,
  config: ProgressConfig,
): PeriodBounds | null {
  const at = new Date(now);
  const y = at.getFullYear();
  const m = at.getMonth();
  const d = at.getDate();
  switch (period) {
    case "day":
      return { start: localDate(y, m, d), end: localDate(y, m, d + 1) };
    case "week": {
      const first = config.weekStart === "monday" ? 1 : SUNDAY;
      const back = (at.getDay() - first + 7) % 7;
      return {
        start: localDate(y, m, d - back),
        end: localDate(y, m, d - back + 7),
      };
    }
    case "month":
      return { start: localDate(y, m, 1), end: localDate(y, m + 1, 1) };
    case "year":
      return { start: localDate(y, 0, 1), end: localDate(y + 1, 0, 1) };
    case "workday": {
      const weekday = at.getDay();
      if (weekday === SUNDAY || weekday === SATURDAY) return null;
      const from = parseHhMm(config.workday.start);
      const to = parseHhMm(config.workday.end);
      if (from === null || to === null || from >= to) return null;
      const start = localDate(y, m, d, Math.floor(from / 60), from % 60);
      const end = localDate(y, m, d, Math.floor(to / 60), to % 60);
      if (at < start || at > end) return null;
      return { start, end };
    }
  }
}

/** How far `now` is into `period`. */
export function progressOf(
  period: Period,
  now: Instant,
  config: ProgressConfig,
): Progress {
  const bounds = periodBounds(period, now, config);
  if (!bounds) return { state: "off" };
  const at = new Date(now).getTime();
  const start = bounds.start.getTime();
  const end = bounds.end.getTime();
  const fraction = Math.min(1, Math.max(0, (at - start) / (end - start)));
  return { state: "running", fraction, remainingMs: Math.max(0, end - at) };
}

/** A unit of the remaining-time text. */
export type RemainingUnit = "day" | "hour" | "minute";

/** One number of the remaining-time text, such as `{ value: 3, unit:
 * "hour" }`. The words ("3 h", "5 dias") come from the i18n with the panel
 * (PR 2), so this returns units and values, never phrases. */
export interface RemainingPart {
  value: number;
  unit: RemainingUnit;
}

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

/** Short remaining time: the two largest units, the second left out when
 * zero. A day or more reads in days and hours ("6 dias 23 h"); less reads in
 * hours and minutes ("3 h 12 min"), or minutes alone ("45 min").
 *
 * The last unit shown is rounded up, as countdowns are: 30 s left reads
 * "1 min", never "0 min" while time remains; only 0 reads "0 min". A "day"
 * here is 24 h of duration, so a 25-hour day starts at "1 dia 1 h". */
export function remainingParts(remainingMs: number): RemainingPart[] {
  const ms = Math.max(0, remainingMs);
  if (ms >= DAY_MS) {
    const hours = Math.ceil(ms / HOUR_MS);
    return compact([
      { value: Math.floor(hours / 24), unit: "day" },
      { value: hours % 24, unit: "hour" },
    ]);
  }
  const minutes = Math.ceil(ms / MINUTE_MS);
  // Rounding up 23:59:30 reaches a whole day.
  if (minutes === 24 * 60) return [{ value: 1, unit: "day" }];
  if (minutes < 60) return [{ value: minutes, unit: "minute" }];
  return compact([
    { value: Math.floor(minutes / 60), unit: "hour" },
    { value: minutes % 60, unit: "minute" },
  ]);
}

/** Drops a zero second unit: "5 dias", not "5 dias 0 h". */
function compact([first, second]: [
  RemainingPart,
  RemainingPart,
]): RemainingPart[] {
  return second.value === 0 ? [first] : [first, second];
}
