import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PanelShowPayload } from "../core/panels";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import type { WidgetsSnapshot } from "../core/widgets";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
import { ptBR } from "../i18n/pt-BR";
import { forgetNote } from "../widgets/notes/memory";
import Panel from "./Panel";

/* A note in the real panel host, with the real registry (NotePanel and
 * NoteSettings): what reaches the stored note across the views and the
 * shows, when a save fails or is still running (spec 0003 §2.2, Nota). */

type Handler = (event: { payload: unknown }) => void;

const { invoke, listen, handlers } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  return {
    handlers,
    invoke: vi.fn(),
    listen: vi.fn((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(() => {});
    }),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const SETTINGS: Settings = {
  version: 1,
  theme: "system",
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
  },
};

const NOTE = { ...WIDGET_FIXTURES["note-default"], label: "Mercado" };
const LIST: WidgetsSnapshot = { storage: { state: "ok" }, widgets: [NOTE] };
const errors = ptBR.widgets.errors;
const common = ptBR.widgets.common;

interface Deferred {
  promise: Promise<void>;
  resolve: () => void;
}

function deferred(): Deferred {
  let resolve!: () => void;
  const promise = new Promise<void>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

/** The Rust side of the note: `stored` is what `get_note` reads; `save`
 * answers each `save_note` (and may write `stored` itself). */
let stored = "";
let save: (body: string) => Promise<void> = (body) => {
  stored = body;
  return Promise.resolve();
};
let waits = 0;

function show(panelId: PanelShowPayload["panelId"]) {
  const handler = handlers.get("panel-show");
  if (!handler) throw new Error("panel-show listener not registered");
  act(() => handler({ payload: { panelId, wait: ++waits } }));
}

const calls = (name: string) =>
  invoke.mock.calls.filter(([command]) => command === name);

/** Lets the pending promises and the timers due now run. */
async function settle(ms = 0) {
  await act(() => vi.advanceTimersByTimeAsync(ms));
}

/** The panel with the note shown and read. */
async function openNote() {
  render(<Panel />);
  await settle();
  show({ widget: NOTE.id });
  await settle();
  return screen.getByRole("textbox", { name: "Mercado" });
}

function saveState() {
  return document.querySelector(".note-save");
}

beforeEach(() => {
  vi.useFakeTimers();
  forgetNote(NOTE.id);
  handlers.clear();
  waits = 0;
  stored = "";
  save = (body) => {
    stored = body;
    return Promise.resolve();
  };
  invoke.mockReset();
  invoke.mockImplementation((command: string, args?: { body?: string }) => {
    switch (command) {
      case "get_settings":
        return Promise.resolve(SETTINGS);
      case "list_widgets":
        return Promise.resolve(LIST);
      case "get_note":
        return Promise.resolve(stored);
      case "save_note":
        return save(args?.body ?? "");
      default:
        return Promise.resolve(undefined);
    }
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("a note whose save fails", () => {
  it("stays on its main view with the text when Ajustes is clicked, and never removes without asking", async () => {
    save = () => Promise.reject({ code: "noteTooLarge" });
    const editor = await openNote();
    fireEvent.change(editor, { target: { value: "Pão" } });

    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    await settle();

    // Ajustes would read the stored note (empty) and remove it at once.
    const remove = screen.queryByRole("button", { name: common.remove });
    if (remove) fireEvent.click(remove);
    await settle();
    expect(calls("remove_widget")).toEqual([]);
    expect(screen.getByRole("textbox", { name: "Mercado" })).toHaveValue("Pão");
    expect(saveState()).toHaveTextContent(errors.noteTooLarge);

    // The next click tries again, and opens Ajustes once saved.
    save = (body) => {
      stored = body;
      return Promise.resolve();
    };
    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    await settle();
    expect(stored).toBe("Pão");
    expect(screen.getByRole("button", { name: common.remove })).toBeVisible();
  });
});

describe("showing the note again while a save runs", () => {
  it("reads the note only after the saves of the last show, so the editor has the newest text", async () => {
    // Writes on the call and answers later: the first save hangs.
    const first = deferred();
    let count = 0;
    save = (body) => {
      stored = body;
      return ++count === 1 ? first.promise : Promise.resolve();
    };
    const editor = await openNote();
    fireEvent.change(editor, { target: { value: "a" } });
    await settle(500);
    fireEvent.change(editor, { target: { value: "ab" } });

    // A new show remounts the content: the old editor saves "ab" once "a"
    // answers, and the new one reads the note only then. Until that, no
    // editor: one shown now would start from "a" (the draft hides it, but
    // its next save, "abc", would be overwritten by the old editor's "ab").
    show({ widget: NOTE.id });
    await settle(500);
    expect(stored).toBe("a");
    expect(screen.queryByRole("textbox", { name: "Mercado" })).toBeNull();
    expect(document.querySelector(".note-placeholder")).not.toBeNull();

    first.resolve();
    await settle();
    expect(stored).toBe("ab");
    expect(screen.getByRole("textbox", { name: "Mercado" })).toHaveValue("ab");
    // Typing then saves over the newest text, and nothing saves after it.
    fireEvent.change(screen.getByRole("textbox", { name: "Mercado" }), {
      target: { value: "abc" },
    });
    await settle(500);
    expect(stored).toBe("abc");
    await settle(1000);
    expect(stored).toBe("abc");
  });
});

describe("closing and showing again a note whose save failed", () => {
  it("brings back the unsaved text with its error, until a save confirms it", async () => {
    let refuse = true;
    save = (body) => {
      if (refuse) return Promise.reject({ code: "noteTooLarge" });
      stored = body;
      return Promise.resolve();
    };
    const editor = await openNote();
    fireEvent.change(editor, { target: { value: "Pão" } });
    await settle(500);
    expect(saveState()).toHaveTextContent(errors.noteTooLarge);

    show("about");
    await settle();
    show({ widget: NOTE.id });
    await settle();

    expect(stored).toBe("");
    expect(screen.getByRole("textbox", { name: "Mercado" })).toHaveValue("Pão");
    expect(saveState()).toHaveTextContent(errors.noteTooLarge);
    // Only in memory: never in the browser's storage.
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);

    refuse = false;
    fireEvent.change(screen.getByRole("textbox", { name: "Mercado" }), {
      target: { value: "Pão e leite" },
    });
    await settle(500);
    expect(stored).toBe("Pão e leite");

    // Saved: the next show starts from the stored note, with nothing to
    // bring back.
    stored = "outro";
    show({ widget: NOTE.id });
    await settle();
    expect(screen.getByRole("textbox", { name: "Mercado" })).toHaveValue(
      "outro",
    );
    expect(saveState()).toHaveTextContent(ptBR.widgets.note.saveStates.idle);
  });
});
