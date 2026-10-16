import { describe, expect, it } from "vitest";
import { ianaTimeZones, utcOffset, zoneCity } from "./zones";

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
