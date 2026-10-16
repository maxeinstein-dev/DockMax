import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  WIDGETS_CHANGED_EVENT,
  WIDGET_ERROR_CODES,
  addWidget,
  dismissStorageNotice,
  getNote,
  isWidgetCommandError,
  listWidgets,
  moveWidget,
  removeWidget,
  saveNote,
  updateWidget,
  useWidgets,
  type WidgetsSnapshot,
} from "./widgets";
import { WIDGET_FIXTURES } from "./widgets.fixtures";

type Handler = (event: { payload: WidgetsSnapshot }) => void;

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

const CLOCK = WIDGET_FIXTURES["clock-default"];
const NOTE = WIDGET_FIXTURES["note-default"];
const SNAPSHOT: WidgetsSnapshot = WIDGET_FIXTURES["snapshot-sample"];
const EMPTY: WidgetsSnapshot = { storage: { state: "ok" }, widgets: [] };
const RECOVERED: WidgetsSnapshot = {
  storage: { state: "recovered" },
  widgets: [CLOCK],
};

/** A text that must never reach the log: an ID, a label, a path. */
const SECRET = "C:\\Users\\someone\\Mercado";

/** Each wrapper, the command it runs, its arguments and what it gives. */
const CALLS = [
  {
    name: "listWidgets",
    call: () => listWidgets(),
    command: "list_widgets",
    args: {},
    result: SNAPSHOT,
    logged: "widgets: list failed",
  },
  {
    name: "addWidget",
    call: () => addWidget({ kind: "clock" }),
    command: "add_widget",
    args: { kind: "clock" },
    result: CLOCK,
    logged: "widgets: add failed",
  },
  {
    name: "removeWidget",
    call: () => removeWidget({ id: NOTE.id }),
    command: "remove_widget",
    args: { id: NOTE.id },
    result: undefined,
    logged: "widgets: remove failed",
  },
  {
    name: "moveWidget",
    call: () => moveWidget({ id: CLOCK.id, direction: "down" }),
    command: "move_widget",
    args: { id: CLOCK.id, direction: "down" },
    result: undefined,
    logged: "widgets: move failed",
  },
  {
    name: "updateWidget",
    call: () =>
      updateWidget({
        id: CLOCK.id,
        label: "São Paulo",
        config: { kind: "clock", config: CLOCK.config },
      }),
    command: "update_widget",
    args: {
      id: CLOCK.id,
      label: "São Paulo",
      config: { kind: "clock", config: CLOCK.config },
    },
    result: { ...CLOCK, label: "São Paulo" },
    logged: "widgets: update failed",
  },
  {
    name: "getNote",
    call: () => getNote({ id: NOTE.id }),
    command: "get_note",
    args: { id: NOTE.id },
    result: "Comprar café",
    logged: "widgets: note read failed",
  },
  {
    name: "saveNote",
    call: () => saveNote({ id: NOTE.id, body: "Comprar café" }),
    command: "save_note",
    args: { id: NOTE.id, body: "Comprar café" },
    result: undefined,
    logged: "widgets: note save failed",
  },
  {
    name: "dismissStorageNotice",
    call: () => dismissStorageNotice(),
    command: "dismiss_storage_notice",
    args: {},
    result: undefined,
    logged: "widgets: notice dismiss failed",
  },
];

function change(snapshot: WidgetsSnapshot) {
  const handler = handlers.get(WIDGETS_CHANGED_EVENT);
  if (!handler) throw new Error("widgets-changed listener not registered");
  act(() => handler({ payload: snapshot }));
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

describe("isWidgetCommandError", () => {
  it.each(WIDGET_ERROR_CODES)("accepts the code %s", (code) => {
    expect(isWidgetCommandError({ code })).toBe(true);
  });

  it.each([
    ["an unknown code", { code: "diskFull" }],
    ["a code that is not a string", { code: 1 }],
    ["an object without a code", {}],
    ["a string", "limitReached"],
    ["null", null],
    ["an Error", new Error("limitReached")],
  ])("refuses %s", (_, error) => {
    expect(isWidgetCommandError(error)).toBe(false);
  });
});

describe("widget commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it.each(CALLS)(
    "$name runs $command with its arguments",
    async ({ call, command, args, result }) => {
      invoke.mockResolvedValue(result);

      await expect(call()).resolves.toEqual(result);
      expect(invoke).toHaveBeenCalledExactlyOnceWith(command, args);
    },
  );

  it.each(
    CALLS.flatMap((entry) =>
      WIDGET_ERROR_CODES.map((code) => ({ ...entry, code })),
    ),
  )("$name rejects with the code Rust sent: $code", async ({ call, code }) => {
    invoke.mockRejectedValue({ code });

    await expect(call()).rejects.toEqual({ code });
    expect(log.warn).not.toHaveBeenCalled();
  });

  it.each(
    CALLS.flatMap((entry) =>
      (
        [
          ["a message from Tauri", `invalid args \`id\`: ${SECRET}`, "string"],
          ["an unknown code", { code: "diskFull", path: SECRET }, "object"],
          ["an Error", new TypeError(SECRET), "TypeError"],
          ["nothing", undefined, "undefined"],
        ] as const
      ).map(([what, error, kind]) => ({ ...entry, what, error, kind })),
    ),
  )(
    "$name rejects with notReady and logs only the kind for $what",
    async ({ call, error, kind, logged }) => {
      invoke.mockRejectedValue(error);

      await expect(call()).rejects.toEqual({ code: "notReady" });
      expect(log.warn).toHaveBeenCalledExactlyOnceWith(`${logged}: ${kind}`);
    },
  );
});

describe("useWidgets", () => {
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

  it("is null while loading, then gives the instances", async () => {
    const read = deferred<WidgetsSnapshot>();
    invoke.mockReturnValue(read.promise);

    const { result } = renderHook(() => useWidgets());
    expect(result.current).toBeNull();
    expect(invoke).toHaveBeenCalledWith("list_widgets", {});

    await act(async () => read.resolve(SNAPSHOT));
    expect(result.current).toEqual(SNAPSHOT);
  });

  it("follows widgets-changed", async () => {
    invoke.mockResolvedValue(SNAPSHOT);
    const { result } = renderHook(() => useWidgets());
    await waitFor(() => expect(result.current).toEqual(SNAPSHOT));

    change(RECOVERED);
    expect(result.current).toEqual(RECOVERED);

    change(EMPTY);
    expect(result.current).toEqual(EMPTY);
  });

  it("drops an initial read that answers after a change", async () => {
    const read = deferred<WidgetsSnapshot>();
    invoke.mockReturnValue(read.promise);
    const { result } = renderHook(() => useWidgets());

    change(RECOVERED);
    await act(async () => read.resolve(SNAPSHOT));

    expect(result.current).toEqual(RECOVERED);
  });

  it("keeps a change when the initial read fails after it", async () => {
    const read = deferred<WidgetsSnapshot>();
    invoke.mockReturnValue(read.promise);
    const { result } = renderHook(() => useWidgets());

    change(RECOVERED);
    await act(async () => read.reject({ code: "unavailable" }));

    expect(result.current).toEqual(RECOVERED);
  });

  it("stays null and logs no data when the read is refused", async () => {
    invoke.mockRejectedValue({ code: "unavailable" });

    const { result } = renderHook(() => useWidgets());

    await waitFor(() =>
      expect(log.warn).toHaveBeenCalledExactlyOnceWith("widgets: read failed"),
    );
    expect(result.current).toBeNull();
  });

  it("stays null and logs only the kind when the read breaks", async () => {
    invoke.mockRejectedValue(new TypeError(SECRET));

    const { result } = renderHook(() => useWidgets());

    await waitFor(() =>
      expect(log.warn).toHaveBeenCalledWith("widgets: read failed"),
    );
    expect(log.warn.mock.calls).toEqual([
      ["widgets: list failed: TypeError"],
      ["widgets: read failed"],
    ]);
    expect(result.current).toBeNull();
  });

  it("stops listening when unmounted", async () => {
    invoke.mockResolvedValue(SNAPSHOT);
    const { unmount } = renderHook(() => useWidgets());
    expect(listen).toHaveBeenCalledWith(
      WIDGETS_CHANGED_EVENT,
      expect.any(Function),
    );

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });

  it("ignores a read that answers after unmounting", async () => {
    const read = deferred<WidgetsSnapshot>();
    invoke.mockReturnValue(read.promise);
    const { result, unmount } = renderHook(() => useWidgets());

    unmount();
    await act(async () => read.reject({ code: "unavailable" }));

    expect(result.current).toBeNull();
    expect(log.warn).not.toHaveBeenCalled();
  });

  it("ignores a change that arrives after unmounting", async () => {
    invoke.mockResolvedValue(SNAPSHOT);
    const { result, unmount } = renderHook(() => useWidgets());
    await waitFor(() => expect(result.current).toEqual(SNAPSHOT));

    unmount();
    change(EMPTY);

    expect(result.current).toEqual(SNAPSHOT);
  });
});
