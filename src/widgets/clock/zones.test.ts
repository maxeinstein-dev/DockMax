import { describe, expect, it } from "vitest";
import {
  ianaTimeZones,
  utcOffset,
  utcOffsetMinutes,
  zoneCity,
  zonesByOffset,
} from "./zones";

// 2026-10-15 18:07:05 UTC.
const NOW = Date.UTC(2026, 9, 15, 18, 7, 5);
// 2026-07-15 12:00 UTC: summer in Lisbon.
const JULY = Date.UTC(2026, 6, 15, 12);

describe("zoneCity", () => {
  it("is the last part of the IANA name, with spaces", () => {
    expect(zoneCity("America/Sao_Paulo")).toBe("Sao Paulo");
    expect(zoneCity("America/Argentina/Buenos_Aires")).toBe("Buenos Aires");
    expect(zoneCity("Asia/Tokyo")).toBe("Tokyo");
    expect(zoneCity("UTC")).toBe("UTC");
  });
});

describe("utcOffset", () => {
  it("writes the offset as UTC with a true minus sign", () => {
    expect(utcOffset(NOW, "America/Sao_Paulo")).toBe("UTC−3");
    expect(utcOffset(NOW, "Asia/Tokyo")).toBe("UTC+9");
    expect(utcOffset(NOW, "Asia/Kolkata")).toBe("UTC+5:30");
  });

  it("is plain UTC at offset zero", () => {
    expect(utcOffset(NOW, "UTC")).toBe("UTC");
  });

  it("follows daylight saving time at the instant", () => {
    expect(utcOffset(JULY, "Europe/Lisbon")).toBe("UTC+1");
    expect(utcOffset(Date.UTC(2026, 0, 15, 12), "Europe/Lisbon")).toBe("UTC");
  });
});

describe("utcOffsetMinutes", () => {
  it("is the offset in minutes east of UTC, half and quarter hours too", () => {
    expect(utcOffsetMinutes(NOW, "America/Sao_Paulo")).toBe(-180);
    expect(utcOffsetMinutes(NOW, "UTC")).toBe(0);
    expect(utcOffsetMinutes(NOW, "Asia/Kolkata")).toBe(330);
    expect(utcOffsetMinutes(NOW, "Asia/Kathmandu")).toBe(345);
    expect(utcOffsetMinutes(NOW, "America/St_Johns")).toBe(-150);
  });
});

describe("zonesByOffset", () => {
  it("orders by offset, from the most negative to the most positive", () => {
    const order = zonesByOffset(NOW, [
      "Asia/Tokyo",
      "Asia/Kathmandu",
      "UTC",
      "Asia/Kolkata",
      "America/Sao_Paulo",
      "Pacific/Honolulu",
    ]).map((zone) => [zone.offset, zone.timeZone]);
    expect(order).toEqual([
      ["UTC−10", "Pacific/Honolulu"],
      ["UTC−3", "America/Sao_Paulo"],
      ["UTC", "UTC"],
      ["UTC+5:30", "Asia/Kolkata"],
      ["UTC+5:45", "Asia/Kathmandu"],
      ["UTC+9", "Asia/Tokyo"],
    ]);
  });

  it("orders by place name within the same offset, keeping the IANA name", () => {
    const order = zonesByOffset(NOW, [
      "America/Sao_Paulo",
      "America/Argentina/Buenos_Aires",
      "America/Fortaleza",
    ]).map((zone) => zone.timeZone);
    expect(order).toEqual([
      "America/Argentina/Buenos_Aires",
      "America/Fortaleza",
      "America/Sao_Paulo",
    ]);
  });

  it("uses the offset in force at the instant, daylight saving included", () => {
    const zones = ["Europe/Lisbon", "Africa/Abidjan", "Europe/Paris"];
    const january = Date.UTC(2026, 0, 15, 12);
    expect(zonesByOffset(JULY, zones).map((z) => z.timeZone)).toEqual([
      "Africa/Abidjan",
      "Europe/Lisbon",
      "Europe/Paris",
    ]);
    expect(zonesByOffset(JULY, zones).map((z) => z.offset)).toEqual([
      "UTC",
      "UTC+1",
      "UTC+2",
    ]);
    // In January Lisbon is at UTC with Abidjan: by name, Abidjan first.
    expect(zonesByOffset(january, zones).map((z) => z.offset)).toEqual([
      "UTC",
      "UTC",
      "UTC+1",
    ]);
  });
});

describe("ianaTimeZones", () => {
  it("is Intl's list, spelled exactly as Intl gives it", () => {
    const zones = ianaTimeZones();
    expect(zones).toContain("America/Sao_Paulo");
    expect(zones).toContain("Asia/Tokyo");
    expect(zones).toEqual(
      (Intl as unknown as { supportedValuesOf(key: string): string[] })
        .supportedValuesOf("timeZone")
        .filter((zone) => zone !== "Factory"),
    );
  });

  it("never offers Factory, which Rust refuses", () => {
    expect(ianaTimeZones()).not.toContain("Factory");
  });
});
