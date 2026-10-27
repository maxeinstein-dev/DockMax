import { describe, expect, it } from "vitest";
import { memoryPercent, rateText, sizeText } from "./systemActivity";

describe("system activity texts", () => {
  it("writes rates as the dock does (rate_text in Rust)", () => {
    expect(rateText(0)).toBe("0 KB/s");
    expect(rateText(340 * 1024)).toBe("340 KB/s");
    expect(rateText(1.2 * 1024 * 1024)).toBe("1,2 MB/s");
    expect(rateText(12.4 * 1024 * 1024)).toBe("12 MB/s");
    expect(rateText(1.1 * 1024 ** 3)).toBe("1,1 GB/s");
    expect(rateText(-5)).toBe("0 KB/s");
  });

  it("writes sizes of memory the Brazilian way", () => {
    expect(sizeText(8 * 1024 ** 3)).toBe("8,0 GB");
    expect(sizeText(512 * 1024 ** 2)).toBe("512 MB");
  });

  it("gives the share of the memory in use", () => {
    expect(memoryPercent({ total: 16, available: 4 })).toBe(75);
    expect(memoryPercent({ total: 0, available: 0 })).toBeNull();
    expect(memoryPercent(null)).toBeNull();
  });
});
