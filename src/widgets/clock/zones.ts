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
