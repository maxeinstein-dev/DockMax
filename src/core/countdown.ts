import { activeLocale } from "../i18n";
import type { CountdownConfig } from "./widgets";

/* The Contagem regressiva (spec 0017 F2c): the days to a date. Mirrors
 * `domain/widgets/countdown.rs`, which draws the dock's tile; the panel
 * counts here, so it follows `now` without a command. */

const DAY_MS = 24 * 3600 * 1000;

/** The longest a date may be from today, either way: 100 years. */
const MAX_DAYS = 100 * 366;

/** Today's local day number (days since 1970-01-01). */
export function todayOf(now: Date): number {
  return Math.floor(
    (now.getTime() - now.getTimezoneOffset() * 60_000) / DAY_MS,
  );
}

/** `2026-12-20` as a day number; null for anything else. */
export function parseDay(text: string): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]) - 1;
  const day = Number(match[3]);
  const date = new Date(Date.UTC(year, month, day));
  if (date.getUTCMonth() !== month || date.getUTCDate() !== day) return null;
  return Math.round(date.getTime() / DAY_MS);
}

/** The day of `month` (0–11) and `day` in `year`: the 29th of February is
 * the 28th in the years without one. */
function inYear(year: number, month: number, day: number): number {
  const date = new Date(Date.UTC(year, month, day));
  const fits = date.getUTCMonth() === month;
  const kept = fits ? date : new Date(Date.UTC(year, month, day - 1));
  return Math.round(kept.getTime() / DAY_MS);
}

/** The day the count goes to, from `today`: the date, or its next return
 * when yearly (today included). null without a date. */
export function targetDay(
  config: CountdownConfig,
  today: number,
): number | null {
  const day = config.date === null ? null : parseDay(config.date);
  if (day === null || !config.yearly) return day;
  const date = new Date(day * DAY_MS);
  const year = new Date(today * DAY_MS).getUTCFullYear();
  const month = date.getUTCMonth();
  const dayOfMonth = date.getUTCDate();
  const thisYear = inYear(year, month, dayOfMonth);
  return thisYear < today ? inYear(year + 1, month, dayOfMonth) : thisYear;
}

/** Days from `today` to the date: 0 on the day, negative once it is gone;
 * null without a date, or one too far to count. */
export function daysLeft(
  config: CountdownConfig,
  today: number,
): number | null {
  const target = targetDay(config, today);
  if (target === null) return null;
  const left = target - today;
  return Math.abs(left) <= MAX_DAYS ? left : null;
}

/** `domingo, 20 de dezembro de 2026`: a day number in words. */
export function longDay(day: number): string {
  return new Intl.DateTimeFormat(activeLocale(), {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(day * DAY_MS));
}
