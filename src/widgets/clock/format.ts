import type { ClockConfig } from "../../core/widgets";
import { activeLocale } from "../../i18n";

/* Clock formatting (spec 0003 §2.2, Relógio). Pure: the instant comes in,
 * so tests never wait for the real clock. Everything goes through
 * `Intl.DateTimeFormat`, which knows the IANA zones and the locale's
 * names; no date library. */

/** An instant as a `Date` or as epoch milliseconds. */
export type Instant = Date | number;

/** The time zone a clock actually uses. */
export interface ResolvedTimeZone {
  /** IANA name used to format, such as `America/Sao_Paulo`. Null when the
   * clock follows the system zone and `Intl` can't name it (it reports
   * `Etc/Unknown`, which it then refuses as a `timeZone`): the clock is
   * formatted without a `timeZone`, in `Intl`'s own default, and the panel
   * says the system zone wasn't identified instead of naming one. */
  timeZone: string | null;
  /** True when the clock follows the system zone: either the config asks for
   * it (null) or the configured zone is unknown here. The panel names it
   * "do sistema" through the i18n. */
  system: boolean;
  /** True when the config names a zone this WebView's `Intl` doesn't know.
   * Rust accepted the name against its own IANA base, so the two bases
   * differ; the clock shows the system time and the panel says so, instead
   * of inventing an offset. */
  timeZoneFallback: boolean;
}

/** What the clock panel shows; the dock icon shows `icon`. */
export interface ClockReading extends ResolvedTimeZone {
  /** Dock icon text: hours and minutes, never seconds. `HH:mm` in 24 h;
   * `h:mm` in 12 h, without AM/PM: the 40 px icon has room for four or five
   * digits only, and the period is in the panel (`dayPeriod`). */
  icon: string;
  /** Panel time: `HH:mm` or `h:mm`, plus `:ss` with `showSeconds`. */
  time: string;
  /** The locale's AM/PM marker in 12 h, such as `PM`; null in 24 h. */
  dayPeriod: string | null;
  /** Long date in the clock's zone, such as
   * `quinta-feira, 15 de outubro de 2026`. */
  date: string;
}

/** The canonical name `Intl` gives `timeZone`, or null when it doesn't know
 * it: `Intl.DateTimeFormat` throws a `RangeError` for such a zone. */
function knownTimeZone(timeZone: string): string | null {
  try {
    return new Intl.DateTimeFormat("en-US", { timeZone }).resolvedOptions()
      .timeZone;
  } catch (error) {
    if (!(error instanceof RangeError)) throw error;
    return null;
  }
}

/** The zone `Intl` uses when no `timeZone` is given, or null when `Intl`
 * can't name it: with an unknown system zone it reports `Etc/Unknown`, a
 * name it then refuses as a `timeZone`. */
export function systemTimeZone(): string | null {
  const name: unknown = new Intl.DateTimeFormat().resolvedOptions().timeZone;
  return typeof name === "string" ? knownTimeZone(name) : null;
}

/** The zone to format with. A zone `Intl` doesn't know falls back to the
 * system zone, flagged; never to an invented offset. */
export function resolveTimeZone(timeZone: string | null): ResolvedTimeZone {
  const resolved = timeZone === null ? null : knownTimeZone(timeZone);
  if (resolved !== null) {
    return { timeZone: resolved, system: false, timeZoneFallback: false };
  }
  return {
    timeZone: systemTimeZone(),
    system: true,
    timeZoneFallback: timeZone !== null,
  };
}

/** The `timeZone` option: left out for a system zone `Intl` can't name, so
 * it formats in its own default instead of throwing. */
function zoneOption(timeZone: string | null): { timeZone?: string } {
  return timeZone === null ? {} : { timeZone };
}

interface TimeParts {
  hour: string;
  minute: string;
  second: string;
  dayPeriod: string | null;
}

/** Hour, minute, second and AM/PM as the locale writes them. The text is
 * assembled from the parts so the icon can never carry seconds or the
 * period, whatever pattern the locale has. */
function timeParts(
  instant: Instant,
  timeZone: string | null,
  hourCycle: ClockConfig["hourCycle"],
  locale: string,
): TimeParts {
  const parts = new Intl.DateTimeFormat(locale, {
    ...zoneOption(timeZone),
    // Explicit `hourCycle`, never `hour12`: `hour12: false` can pick h24
    // and show midnight as 24:00.
    hourCycle,
    // 24 h pads the hour (09:05), 12 h doesn't (9:05), as clocks are read.
    hour: hourCycle === "h23" ? "2-digit" : "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).formatToParts(instant);
  const part = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find((p) => p.type === type)?.value;
  return {
    hour: part("hour") ?? "",
    minute: part("minute") ?? "",
    second: part("second") ?? "",
    dayPeriod: hourCycle === "h12" ? (part("dayPeriod") ?? null) : null,
  };
}

/** Everything the clock shows at `instant`, in its configured zone. */
export function formatClock(
  instant: Instant,
  config: ClockConfig,
  locale: string = activeLocale(),
): ClockReading {
  const zone = resolveTimeZone(config.timeZone);
  const t = timeParts(instant, zone.timeZone, config.hourCycle, locale);
  const icon = `${t.hour}:${t.minute}`;
  const date = new Intl.DateTimeFormat(locale, {
    ...zoneOption(zone.timeZone),
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  }).format(instant);
  return {
    ...zone,
    icon,
    time: config.showSeconds ? `${icon}:${t.second}` : icon,
    dayPeriod: t.dayPeriod,
    date,
  };
}

/** Just the dock icon text (see `ClockReading.icon`). */
export function formatDockTime(
  instant: Instant,
  config: ClockConfig,
  locale: string = activeLocale(),
): string {
  return formatClock(instant, config, locale).icon;
}

const MINUTE_MS = 60_000;

/** Milliseconds from `now` to the next minute boundary, for the dock's
 * single timer: one wake-up per minute, landing as the minute turns, instead
 * of polling every second. A whole minute when `now` is exactly on a
 * boundary, since that minute is already shown. Minute boundaries are the
 * same in every zone in use today (all offsets are whole minutes), so one
 * timer serves every clock. */
export function msUntilNextMinute(now: Instant): number {
  const ms = typeof now === "number" ? now : now.getTime();
  // Double modulo keeps instants before 1970 positive.
  return MINUTE_MS - (((ms % MINUTE_MS) + MINUTE_MS) % MINUTE_MS);
}
