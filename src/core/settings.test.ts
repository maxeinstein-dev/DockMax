import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  SETTINGS_CHANGED_EVENT,
  getSettings,
  updateSettings,
  useSettings,
  type Settings,
  type SettingsErrorCode,
  DEFAULT_LOOK,
} from "./settings";

type Handler = (event: { payload: Settings }) => void;

const { invoke, listen, handlers, unlisten, log } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  return {
    invoke: vi.fn(),
    unlisten: vi.fn(),
    handlers,
    listen: vi.fn(),
    log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("./log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./log")>()),
  log,
}));

const DEFAULTS: Settings = {
  version: 1,
  theme: "system",
  launchAtLogin: false,
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
  },
};

const DARK: Settings = { ...DEFAULTS, theme: "dark" };

function change(settings: Settings) {
  const handler = handlers.get(SETTINGS_CHANGED_EVENT);
  if (!handler) throw new Error("settings-changed listener not registered");
  act(() => handler({ payload: settings }));
}

/** A promise settled by the test, to order the read and the event. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("settings commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it("reads the settings with get_settings", async () => {
    invoke.mockResolvedValue(DEFAULTS);

    await expect(getSettings()).resolves.toEqual(DEFAULTS);
    expect(invoke).toHaveBeenCalledWith("get_settings");
  });

  it("sends the full settings to update_settings", async () => {
    invoke.mockResolvedValue(DARK);

    await expect(updateSettings(DARK)).resolves.toEqual(DARK);
    expect(invoke).toHaveBeenCalledWith("update_settings", {
      settings: DARK,
    });
  });

  it.each([
    "iconSizeOutOfRange",
    "unsupportedVersion",
    "writeFailed",
    "notReady",
  ] satisfies SettingsErrorCode[])(
    "rejects with the code of a refused change: %s",
    async (code) => {
      invoke.mockRejectedValue({ code });

      await expect(updateSettings(DARK)).rejects.toEqual({ code });
      expect(log.warn).not.toHaveBeenCalled();
    },
  );

  it.each([
    ["a message from Tauri", "invalid args `settings` for command", "string"],
    ["an unknown code", { code: "diskFull" }, "object"],
    ["an Error", new TypeError("C:\\Users\\someone"), "TypeError"],
    ["nothing", undefined, "undefined"],
  ])(
    "rejects with notReady and logs only the kind for %s",
    async (_, error, kind) => {
      invoke.mockRejectedValue(error);

      await expect(updateSettings(DARK)).rejects.toEqual({ code: "notReady" });
      expect(log.warn).toHaveBeenCalledWith(`settings: change failed: ${kind}`);
    },
  );
});

describe("useSettings", () => {
  beforeEach(() => {
    handlers.clear();
    invoke.mockReset();
    unlisten.mockReset();
    listen.mockReset();
    listen.mockImplementation((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(unlisten);
    });
    log.warn.mockReset();
  });

  it("is null while loading, then gives the settings in use", async () => {
    const read = deferred<Settings>();
    invoke.mockReturnValue(read.promise);

    const { result } = renderHook(() => useSettings());
    expect(result.current).toBeNull();
    expect(invoke).toHaveBeenCalledWith("get_settings");

    await act(async () => read.resolve(DEFAULTS));
    expect(result.current).toEqual(DEFAULTS);
  });

  it("follows settings-changed", async () => {
    invoke.mockResolvedValue(DEFAULTS);
    const { result } = renderHook(() => useSettings());
    await waitFor(() => expect(result.current).toEqual(DEFAULTS));

    change(DARK);

    expect(result.current).toEqual(DARK);
  });

  it("drops an initial read that answers after a change", async () => {
    const read = deferred<Settings>();
    invoke.mockReturnValue(read.promise);
    const { result } = renderHook(() => useSettings());

    change(DARK);
    await act(async () => read.resolve(DEFAULTS));

    expect(result.current).toEqual(DARK);
  });

  it("stays null and logs only the kind of error when the read fails", async () => {
    invoke.mockRejectedValue(new TypeError("C:\\Users\\someone"));

    const { result } = renderHook(() => useSettings());

    await waitFor(() =>
      expect(log.warn).toHaveBeenCalledWith("settings: read failed: TypeError"),
    );
    expect(result.current).toBeNull();
  });

  it("stops listening when unmounted", async () => {
    invoke.mockResolvedValue(DEFAULTS);
    const { unmount } = renderHook(() => useSettings());
    expect(listen).toHaveBeenCalledWith(
      SETTINGS_CHANGED_EVENT,
      expect.any(Function),
    );

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });

  it("ignores a read that answers after unmounting", async () => {
    const read = deferred<Settings>();
    invoke.mockReturnValue(read.promise);
    const { result, unmount } = renderHook(() => useSettings());

    unmount();
    await act(async () => read.reject(new Error("late")));

    expect(result.current).toBeNull();
    expect(log.warn).not.toHaveBeenCalled();
  });
});
