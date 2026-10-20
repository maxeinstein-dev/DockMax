import type { Instant } from "./format";

/* Time zone names for the clock's panel and settings (spec 0003 §2.2).
 * Pure, through `Intl` only. */

/** A short place name for an IANA zone: its last part, with spaces, as
 * `America/Sao_Paulo` → `Sao Paulo`. The base's own name, never a
 * translation that could drift from it. */
export function zoneCity(timeZone: string): string {
  return timeZone.slice(timeZone.lastIndexOf("/") + 1).replace(/_/g, " ");
}

// `shortOffset` is ES2021 Intl; the project's lib is ES2020.
const SHORT_OFFSET =
  "shortOffset" as Intl.DateTimeFormatOptions["timeZoneName"];

/** The offset of `timeZone` at `instant`, as `UTC−3`, `UTC+5:30` or `UTC`,
 * with a true minus sign. `timeZone` must be one `Intl` knows (a resolved
 * zone). */
export function utcOffset(instant: Instant, timeZone: string): string {
  const name =
    new Intl.DateTimeFormat("en-US", { timeZone, timeZoneName: SHORT_OFFSET })
      .formatToParts(instant)
      .find((part) => part.type === "timeZoneName")?.value ?? "GMT";
  // Offset zero reads `GMT` in some ICU builds and `GMT+0` in others.
  if (/^GMT(?:[+-]0(?::00)?)?$/.test(name)) return "UTC";
  return name.replace(/^GMT/, "UTC").replace("-", "−");
}

/** The offset of `timeZone` at `instant` in minutes east of UTC (São Paulo
 * −180, Kolkata 330), read from the same text as `utcOffset`, so the order
 * and the text can't disagree. */
export function utcOffsetMinutes(instant: Instant, timeZone: string): number {
  const match = /^UTC(?:([+−])(\d{1,2})(?::(\d{2}))?)?$/.exec(
    utcOffset(instant, timeZone),
  );
  if (!match?.[1]) return 0;
  const minutes = Number(match[2]) * 60 + Number(match[3] ?? 0);
  return match[1] === "−" ? -minutes : minutes;
}

/** A zone with its offset at one instant. */
export interface ZoneAtInstant {
  /** The IANA name, exactly as given. */
  timeZone: string;
  /** The offset as `utcOffset` writes it, as `UTC−3`. */
  offset: string;
  /** The same offset in minutes east of UTC. */
  minutes: number;
}

/** `zones` ordered by their offset at `instant`, from the most negative to
 * the most positive, and by place name (`zoneCity`), then by IANA name,
 * within the same offset. The offset is the one in force at `instant`,
 * daylight saving time included: Lisbon is `UTC+1` in July and `UTC` in
 * January, so the order of a list read at another instant may differ. */
export function zonesByOffset(
  instant: Instant,
  zones: readonly string[],
): ZoneAtInstant[] {
  return zones
    .map((timeZone) => ({
      timeZone,
      offset: utcOffset(instant, timeZone),
      minutes: utcOffsetMinutes(instant, timeZone),
      city: zoneCity(timeZone),
    }))
    .sort(
      (a, b) =>
        a.minutes - b.minutes ||
        compareText(a.city, b.city) ||
        compareText(a.timeZone, b.timeZone),
    )
    .map(({ timeZone, offset, minutes }) => ({ timeZone, offset, minutes }));
}

/** Plain code-unit order: the same on every machine, whatever its locale. */
function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

// `Intl.supportedValuesOf` is ES2022; the project's lib is ES2020.
type IntlWithValues = typeof Intl & {
  supportedValuesOf?: (key: "timeZone") => string[];
};

let zones: readonly string[] | null = null;

/** The IANA zones this WebView's `Intl` knows, spelled exactly as it gives
 * them, without `Factory` (Rust refuses it). Read once. */
export function ianaTimeZones(): readonly string[] {
  if (zones === null) {
    const intl = Intl as IntlWithValues;
    zones = (intl.supportedValuesOf?.("timeZone") ?? []).filter(
      (zone) => zone !== "Factory",
    );
  }
  return zones;
}
