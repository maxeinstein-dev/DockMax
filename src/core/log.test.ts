import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { errorKind, log } from "./log";

const plugin = vi.hoisted(() => ({
  error: vi.fn(),
  warn: vi.fn(),
  info: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-log", () => plugin);

describe("log", () => {
  beforeEach(() => {
    for (const write of Object.values(plugin)) {
      write.mockReset();
      write.mockResolvedValue(undefined);
    }
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("writes each level through the plugin", () => {
    log.error("panel: render failed: TypeError");
    log.warn("settings: get_settings failed");
    log.info("dock: ready");

    expect(plugin.error).toHaveBeenCalledWith(
      "panel: render failed: TypeError",
    );
    expect(plugin.warn).toHaveBeenCalledWith("settings: get_settings failed");
    expect(plugin.info).toHaveBeenCalledWith("dock: ready");
  });

  it("falls back to the console when the plugin fails", async () => {
    plugin.error.mockRejectedValue(new Error("not allowed"));
    const fallback = vi.spyOn(console, "warn").mockImplementation(() => {});

    expect(() => log.error("panel: render failed: Error")).not.toThrow();
    await vi.waitFor(() =>
      expect(fallback).toHaveBeenCalledWith("panel: render failed: Error"),
    );
  });
});

describe("errorKind", () => {
  it("gives the name of an error, never its message", () => {
    expect(errorKind(new TypeError("C:\\Users\\someone\\secret"))).toBe(
      "TypeError",
    );
  });

  it("gives the name of a custom error class", () => {
    class SettingsLoadError extends Error {
      name = "SettingsLoadError";
    }

    expect(errorKind(new SettingsLoadError("x"))).toBe("SettingsLoadError");
  });

  it("hides a name that is not an identifier", () => {
    const error = new Error("x");
    error.name = "C:\\Users\\someone";

    expect(errorKind(error)).toBe("Error");
  });

  it("gives the type of other thrown values", () => {
    expect(errorKind("path C:\\Users\\someone")).toBe("string");
    expect(errorKind(undefined)).toBe("undefined");
    expect(errorKind({ token: "x" })).toBe("object");
  });
});
