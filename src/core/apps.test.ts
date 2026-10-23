import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  APP_ERROR_CODES,
  APPS_CHANGED_EVENT,
  appIcon,
  appMenu,
  forgetIcons,
  isAppCommandError,
  launchApp,
  listApps,
  moveApp,
  pinApp,
  revealApp,
  runAppEntry,
  unpinApp,
  useAppIcon,
  useApps,
  type AppMenu,
  type AppsSnapshot,
} from "./apps";
import { readSource } from "../test/sources";

type Handler = (event: { payload: unknown }) => void;

const { invoke, listen, handlers } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  return {
    invoke: vi.fn(),
    handlers,
    listen: vi.fn((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(() => {});
    }),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const SNAPSHOT: AppsSnapshot = {
  storage: { state: "ok" },
  apps: [
    {
      id: "0199a4c0-0000-7000-8000-000000000001",
      appId: "Some.App!App",
      name: "Some App",
    },
  ],
};

beforeEach(() => {
  handlers.clear();
  invoke.mockReset();
  forgetIcons();
});

afterEach(() => {
  vi.restoreAllMocks();
});

/* The Rust × TS contract of the apps (spec 0006): the JSON files are what
 * Rust serializes (a Rust test compares them with the serialization). */
describe("apps contract fixtures", () => {
  const read = (name: string): unknown =>
    JSON.parse(readSource(`src-tauri/tests/fixtures/apps/${name}`));

  it("has the error codes Rust sends, and only notReady more", () => {
    const sent = read("error-codes.json") as string[];

    expect([...APP_ERROR_CODES].sort()).toEqual([...sent, "notReady"].sort());
  });

  it("reads as the snapshot the interface expects", () => {
    const sample: AppsSnapshot = {
      storage: { state: "ok" },
      apps: [
        {
          id: "0199a4c0-0000-7000-8000-000000000001",
          appId: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
          name: "Calculadora",
        },
        {
          id: "0199a4c0-0000-7000-8000-000000000002",
          appId: "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\notepad.exe",
          name: "Bloco de notas",
        },
      ],
    };

    expect(read("snapshot-sample.json")).toEqual(sample);
  });
});

describe("the menu of a program", () => {
  it("matches what Rust sends", () => {
    const sample: AppMenu = {
      sections: [
        {
          kind: "category",
          title: "Recent chats",
          entries: [
            { key: 0, title: "First" },
            { key: 1, title: "Second" },
          ],
        },
        {
          kind: "recent",
          title: null,
          entries: [{ key: 2, title: "notes.txt" }],
        },
        {
          kind: "tasks",
          title: null,
          entries: [{ key: 3, title: "New window" }],
        },
      ],
      canReveal: true,
    };

    expect(
      JSON.parse(readSource("src-tauri/tests/fixtures/apps/menu-sample.json")),
    ).toEqual(sample);
  });

  it("is asked for, run by key and revealed by the pinned id", async () => {
    invoke.mockResolvedValue(undefined);

    await appMenu("one");
    await runAppEntry("one", 3);
    await revealApp("one");

    expect(invoke.mock.calls).toEqual([
      ["app_menu", { id: "one" }],
      ["run_app_entry", { id: "one", key: 3 }],
      ["reveal_app", { id: "one" }],
    ]);
  });

  it("passes a code through and hides any other failure", async () => {
    invoke.mockRejectedValueOnce({ code: "notFound" });
    await expect(appMenu("one")).rejects.toEqual({ code: "notFound" });

    invoke.mockRejectedValueOnce(new Error("C:\\secret"));
    await expect(runAppEntry("one", 0)).rejects.toEqual({ code: "notReady" });
  });
});

describe("app commands", () => {
  it("knows every code Rust sends, and notReady", () => {
    expect([...APP_ERROR_CODES].sort()).toEqual(
      [
        "alreadyPinned",
        "invalidApp",
        "launchFailed",
        "limitReached",
        "notFound",
        "notReady",
        "unavailable",
        "writeFailed",
      ].sort(),
    );
  });

  it("tells a code from anything else", () => {
    expect(isAppCommandError({ code: "alreadyPinned" })).toBe(true);
    expect(isAppCommandError({ code: "nope" })).toBe(false);
    expect(isAppCommandError("alreadyPinned")).toBe(false);
    expect(isAppCommandError(null)).toBe(false);
  });

  it("sends each command by its name and arguments", async () => {
    invoke.mockResolvedValue(undefined);

    await pinApp("Some.App!App");
    await unpinApp("one");
    await moveApp("one", "down");
    await launchApp("one");
    await listApps();

    expect(invoke.mock.calls).toEqual([
      ["pin_app", { appId: "Some.App!App" }],
      ["unpin_app", { id: "one" }],
      ["move_app", { id: "one", direction: "down" }],
      ["launch_app", { id: "one" }],
      ["list_apps", {}],
    ]);
  });

  it("passes a code from Rust through", async () => {
    invoke.mockRejectedValue({ code: "limitReached" });

    await expect(pinApp("x")).rejects.toEqual({ code: "limitReached" });
  });

  it("turns any other failure into notReady, without its message", async () => {
    invoke.mockRejectedValue(new Error("C:\\secret\\path"));

    await expect(launchApp("x")).rejects.toEqual({ code: "notReady" });
  });
});

describe("icons", () => {
  it("asks once for a program and a size", async () => {
    invoke.mockResolvedValue("data:image/png;base64,AA==");

    const first = await appIcon("Some.App!App", 96);
    const second = await appIcon("Some.App!App", 96);

    expect(first).toBe("data:image/png;base64,AA==");
    expect(second).toBe(first);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("app_icon", {
      appId: "Some.App!App",
      size: 96,
    });
  });

  it("asks again for another size", async () => {
    invoke.mockResolvedValue("data:image/png;base64,AA==");

    await appIcon("Some.App!App", 96);
    await appIcon("Some.App!App", 32);

    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("is null for a program with no icon, and for a failure", async () => {
    invoke.mockResolvedValueOnce(null).mockRejectedValueOnce({ code: "x" });

    expect(await appIcon("none")).toBeNull();
    expect(await appIcon("broken")).toBeNull();
  });

  it("reads a few at a time", async () => {
    let running = 0;
    let most = 0;
    invoke.mockImplementation(async () => {
      running += 1;
      most = Math.max(most, running);
      await new Promise((resolve) => setTimeout(resolve, 1));
      running -= 1;
      return "data:image/png;base64,AA==";
    });

    await Promise.all(
      Array.from({ length: 20 }, (_, n) => appIcon(`app${n}`, 32)),
    );

    expect(invoke).toHaveBeenCalledTimes(20);
    expect(most).toBeLessThanOrEqual(4);
  });

  it("gives the hook null until the icon is read", async () => {
    invoke.mockResolvedValue("data:image/png;base64,AA==");

    const { result } = renderHook(() => useAppIcon("Some.App!App"));
    expect(result.current).toBeNull();
    await act(async () => {});

    expect(result.current).toBe("data:image/png;base64,AA==");
  });
});

describe("useApps", () => {
  it("is null while loading, then the list", async () => {
    invoke.mockResolvedValue(SNAPSHOT);

    const { result } = renderHook(() => useApps());
    expect(result.current).toBeNull();
    await act(async () => {});

    expect(result.current).toEqual(SNAPSHOT);
  });

  it("follows apps-changed", async () => {
    invoke.mockResolvedValue(SNAPSHOT);
    const { result } = renderHook(() => useApps());
    await act(async () => {});

    act(() =>
      handlers.get(APPS_CHANGED_EVENT)?.({
        payload: { storage: { state: "ok" }, apps: [] },
      }),
    );

    expect(result.current?.apps).toEqual([]);
  });

  it("drops a read that a change overtook", async () => {
    let answer: (snapshot: AppsSnapshot) => void = () => {};
    invoke.mockReturnValue(new Promise((resolve) => (answer = resolve)));
    const { result } = renderHook(() => useApps());
    await act(async () => {});

    act(() =>
      handlers.get(APPS_CHANGED_EVENT)?.({
        payload: { storage: { state: "ok" }, apps: [] },
      }),
    );
    await act(async () => answer(SNAPSHOT));

    expect(result.current?.apps).toEqual([]);
  });

  it("stays null when the list can't be read", async () => {
    invoke.mockRejectedValue(new Error("x"));

    const { result } = renderHook(() => useApps());
    await act(async () => {});

    expect(result.current).toBeNull();
  });

  it("ignores an answer that is not a list", async () => {
    invoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useApps());
    await act(async () => {});

    expect(result.current).toBeNull();
  });
});
