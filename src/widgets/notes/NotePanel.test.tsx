import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import { ptBR } from "../../i18n/pt-BR";
import { PANEL_TITLE_ID } from "../../panel/parts/focus";
import tokens from "../../themes/tokens.css?raw";
import NotePanel from "./NotePanel";
import { forgetNote } from "./memory";
import css from "./notes.css?raw";

const { invoke, log } = vi.hoisted(() => ({
  invoke: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../core/log")>()),
  log,
}));

const NOTE = { ...WIDGET_FIXTURES["note-default"], label: "Mercado" };
const text = ptBR.widgets.note;
const errors = ptBR.widgets.errors;

/** A note's text that must never reach the log. */
const SECRET = "senha do cofre: 1234";

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** The Rust side: what `get_note` gives and what `save_note` does. */
function backend({
  note = () => Promise.resolve(""),
  save = () => Promise.resolve(),
}: {
  note?: () => Promise<string>;
  save?: (body: string) => Promise<void>;
} = {}) {
  invoke.mockImplementation((command: string, args: { body?: string }) => {
    if (command === "get_note") return note();
    if (command === "save_note") return save(args.body ?? "");
    return Promise.reject(new Error(`unexpected ${command}`));
  });
}

/** The bodies sent to `save_note`, in order. */
function saves(): string[] {
  return invoke.mock.calls
    .filter(([command]) => command === "save_note")
    .map(([, args]) => (args as { body: string }).body);
}

/** Lets the pending promises and the timers due now run. */
async function settle(ms = 0) {
  await act(() => vi.advanceTimersByTimeAsync(ms));
}

function renderPanel(returnedFromSettings = false) {
  const onAdjust = vi.fn();
  const view = render(
    <NotePanel
      instance={NOTE}
      now={new Date(2026, 9, 16, 9, 41)}
      onAdjust={onAdjust}
      returnedFromSettings={returnedFromSettings}
    />,
  );
  return { ...view, onAdjust };
}

/** The panel with the note read, its editor shown. */
async function openNote() {
  const view = renderPanel();
  await settle();
  return { ...view, editor: screen.getByRole("textbox") };
}

function saveState() {
  return document.querySelector(".note-save");
}

beforeEach(() => {
  vi.useFakeTimers();
  forgetNote(NOTE.id);
  invoke.mockReset();
  log.error.mockReset();
  log.warn.mockReset();
  log.info.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

function expectNothingLogged(secret: string) {
  for (const spy of [log.error, log.warn, log.info]) {
    for (const call of spy.mock.calls) {
      expect(JSON.stringify(call)).not.toContain(secret);
    }
  }
}

describe("reading the note", () => {
  it("reads the text with get_note and shows it in the editor", async () => {
    backend({ note: () => Promise.resolve("Pão\nLeite") });
    renderPanel();
    await settle();

    expect(invoke).toHaveBeenCalledWith("get_note", { id: NOTE.id });
    const editor = screen.getByRole("textbox", { name: "Mercado" });
    expect(editor).toHaveValue("Pão\nLeite");
    expect(saveState()).toHaveTextContent(text.saveStates.idle);
  });

  it("shows no editor until the text is read", async () => {
    const read = deferred<string>();
    backend({ note: () => read.promise });
    renderPanel();

    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(saveState()).toBeNull();
    read.resolve("oi");
    await settle();
    expect(screen.getByRole("textbox")).toHaveValue("oi");
  });

  it("is the Windows 11 field, named by the panel title", async () => {
    backend();
    const { editor } = await openNote();

    expect(editor.tagName).toBe("TEXTAREA");
    expect(editor).toHaveClass("field-input", "note-editor");
    expect(editor).toHaveAttribute("aria-labelledby", PANEL_TITLE_ID);
    expect(editor).toHaveAttribute("placeholder", text.placeholder);
    expect(editor).not.toHaveAttribute("readonly");
  });

  it("has the header with Ajustes, focused when back from Ajustes", async () => {
    backend();
    const { onAdjust, unmount } = renderPanel(true);
    await settle();

    const adjust = screen.getByRole("button", { name: "Ajustes" });
    expect(adjust).toHaveFocus();
    fireEvent.click(adjust);
    expect(onAdjust).toHaveBeenCalledTimes(1);
    unmount();

    renderPanel(false);
    await settle();
    expect(screen.getByRole("button", { name: "Ajustes" })).not.toHaveFocus();
  });
});

describe("saving", () => {
  it("saves 500 ms after the last keystroke, saying so in the footer", async () => {
    const save = deferred<void>();
    backend({ save: () => save.promise });
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "o" } });
    await settle(300);
    fireEvent.change(editor, { target: { value: "oi" } });
    expect(saveState()).toHaveTextContent(text.saveStates.pending);
    await settle(499);
    expect(saves()).toEqual([]);
    await settle(1);
    expect(saves()).toEqual(["oi"]);
    expect(saveState()).toHaveTextContent(text.saveStates.saving);
    expect(saveState()).toHaveAttribute("data-state", "saving");

    save.resolve();
    await settle();
    expect(saveState()).toHaveTextContent(text.saveStates.idle);
    expect(saveState()).toHaveAttribute("data-state", "idle");
  });

  it("sends the instance's id and the whole text", async () => {
    backend();
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "Café" } });
    await settle(500);
    expect(invoke).toHaveBeenCalledWith("save_note", {
      id: NOTE.id,
      body: "Café",
    });
  });

  it("saves at once when the editor loses the focus", async () => {
    backend();
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "oi" } });
    fireEvent.blur(editor);
    await settle();
    expect(saves()).toEqual(["oi"]);
    await settle(1000);
    expect(saves()).toEqual(["oi"]);
  });

  it("saves at once when the window loses the focus or hides", async () => {
    backend();
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "a" } });
    fireEvent.blur(window);
    await settle();
    expect(saves()).toEqual(["a"]);

    fireEvent.change(editor, { target: { value: "ab" } });
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    fireEvent(document, new Event("visibilitychange"));
    await settle();
    expect(saves()).toEqual(["a", "ab"]);
  });

  it("saves the pending text when it unmounts (another panel, closing)", async () => {
    backend();
    const { editor, unmount } = await openNote();

    fireEvent.change(editor, { target: { value: "até já" } });
    unmount();
    expect(saves()).toEqual(["até já"]);
    await settle(1000);
    expect(saves()).toEqual(["até já"]);
  });

  it("on unmount, waits for the running save and then saves the newer text", async () => {
    const first = deferred<void>();
    let calls = 0;
    backend({
      save: () => (++calls === 1 ? first.promise : Promise.resolve()),
    });
    const { editor, unmount } = await openNote();

    fireEvent.change(editor, { target: { value: "a" } });
    await settle(500);
    fireEvent.change(editor, { target: { value: "ab" } });
    unmount();
    expect(saves()).toEqual(["a"]);

    first.resolve();
    await settle();
    expect(saves()).toEqual(["a", "ab"]);
  });

  it("saves nothing when nothing changed", async () => {
    backend({ note: () => Promise.resolve("igual") });
    const { editor, unmount } = await openNote();

    fireEvent.change(editor, { target: { value: "igual!" } });
    fireEvent.change(editor, { target: { value: "igual" } });
    fireEvent.blur(editor);
    unmount();
    await settle(1000);
    expect(saves()).toEqual([]);
  });
});

describe("a failed save", () => {
  it("shows the 64 KiB limit and keeps the text on screen", async () => {
    backend({ save: () => Promise.reject({ code: "noteTooLarge" }) });
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: SECRET } });
    await settle(500);
    expect(saveState()).toHaveTextContent(errors.noteTooLarge);
    expect(saveState()).toHaveAttribute("data-state", "error");
    expect(editor).toHaveValue(SECRET);
    // Not retried on its own.
    await settle(5000);
    expect(saves()).toHaveLength(1);
    expectNothingLogged(SECRET);
  });

  it("keeps the message while typing, and clears it once saved", async () => {
    let refuse = true;
    backend({
      save: () =>
        refuse ? Promise.reject({ code: "noteTooLarge" }) : Promise.resolve(),
    });
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "longo" } });
    await settle(500);
    refuse = false;
    fireEvent.change(editor, { target: { value: "curto" } });
    expect(saveState()).toHaveTextContent(errors.noteTooLarge);
    await settle(500);
    expect(saveState()).toHaveTextContent(text.saveStates.idle);
  });

  it("shows the message of the error's code", async () => {
    backend({ save: () => Promise.reject({ code: "unavailable" }) });
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "a" } });
    await settle(500);
    expect(saveState()).toHaveTextContent(errors.unavailable);
  });

  it("gives up on a save after 10 s, and the next change retries", async () => {
    let calls = 0;
    backend({
      save: () => (++calls === 1 ? new Promise(() => {}) : Promise.resolve()),
    });
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: "a" } });
    await settle(500);
    await settle(9999);
    expect(saveState()).toHaveTextContent(text.saveStates.saving);
    await settle(1);
    expect(saveState()).toHaveTextContent(errors.writeFailed);

    fireEvent.change(editor, { target: { value: "ab" } });
    await settle(500);
    expect(saves()).toEqual(["a", "ab"]);
    expect(saveState()).toHaveTextContent(text.saveStates.idle);
  });

  it("never logs the text, even when the save fails unexpectedly", async () => {
    backend({ save: () => Promise.reject(new Error(SECRET)) });
    const { editor } = await openNote();

    fireEvent.change(editor, { target: { value: SECRET } });
    await settle(500);
    expect(saveState()).toHaveTextContent(errors.notReady);
    expectNothingLogged(SECRET);
  });
});

describe("a note that can't be read (spec §2.10, N7)", () => {
  it("locks the editor with a notice and never saves over the text", async () => {
    backend({ note: () => Promise.reject({ code: "writeFailed" }) });
    const { unmount } = renderPanel();
    await settle();

    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(text.unreadable);
    expect(saveState()).toBeNull();
    fireEvent.blur(window);
    unmount();
    await settle(1000);
    expect(saves()).toEqual([]);
  });

  it("locks it for any failure of the read", async () => {
    backend({ note: () => Promise.reject(new Error(SECRET)) });
    renderPanel();
    await settle();

    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(text.unreadable);
    expectNothingLogged(SECRET);
  });
});

describe("notes.css", () => {
  it("uses only tokens that tokens.css defines", () => {
    const defined = new Set(
      [...tokens.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const used = [...css.matchAll(/var\((--[\w-]+)/g)].map((match) => match[1]);
    expect(used.length).toBeGreaterThan(0);
    expect(used.filter((token) => !defined.has(token))).toEqual([]);
  });
});

describe("Ajustes", () => {
  it("saves the pending text before opening Ajustes, which reads the stored note", async () => {
    const save = deferred<void>();
    backend({ save: () => save.promise });
    const { editor, onAdjust } = await openNote();
    fireEvent.change(editor, { target: { value: "Pão" } });

    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    await settle();

    // Saved at once, not after the pause; Ajustes waits for it.
    expect(saves()).toEqual(["Pão"]);
    expect(onAdjust).not.toHaveBeenCalled();
    // A second click while saving asks once.
    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));

    save.resolve();
    await settle();
    expect(onAdjust).toHaveBeenCalledTimes(1);
  });

  it("waits for a running save and the newer text typed during it", async () => {
    const first = deferred<void>();
    const second = deferred<void>();
    const pending = [first, second];
    backend({ save: () => pending.shift()!.promise });
    const { editor, onAdjust } = await openNote();
    fireEvent.change(editor, { target: { value: "" } });
    fireEvent.change(editor, { target: { value: "a" } });
    await settle(500);
    fireEvent.change(editor, { target: { value: "ab" } });

    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    first.resolve();
    await settle();
    expect(onAdjust).not.toHaveBeenCalled();
    expect(saves()).toEqual(["a", "ab"]);

    second.resolve();
    await settle();
    expect(onAdjust).toHaveBeenCalledTimes(1);
  });

  // With the real panel host: src/panel/NoteRouting.test.tsx.
  it("does not open Ajustes when the save fails: the text and the error stay", async () => {
    backend({ save: () => Promise.reject({ code: "writeFailed" }) });
    const { editor, onAdjust } = await openNote();
    fireEvent.change(editor, { target: { value: "Pão" } });

    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    await settle();

    expect(onAdjust).not.toHaveBeenCalled();
    expect(editor).toHaveValue("Pão");
    expect(saveState()).toHaveTextContent(errors.writeFailed);
  });

  it("does not open Ajustes after the panel went away", async () => {
    const save = deferred<void>();
    backend({ save: () => save.promise });
    const { editor, onAdjust, unmount } = await openNote();
    fireEvent.change(editor, { target: { value: "Pão" } });
    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));

    unmount();
    save.resolve();
    await settle();

    expect(onAdjust).not.toHaveBeenCalled();
  });
});
