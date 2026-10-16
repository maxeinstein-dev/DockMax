import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { WidgetInstance, WidgetsSnapshot } from "../core/widgets";
import { ptBR } from "../i18n/pt-BR";
import tokens from "../themes/tokens.css?raw";
import { draftOf, forgetNote, keepDraft } from "../widgets/notes/memory";
import DockItems from "./DockItems";
import css from "./DockItems.css?raw";

const text = ptBR.widgets;

const { invoke, log } = vi.hoisted(() => ({
  invoke: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log,
}));

const CLOCK: WidgetInstance = {
  id: "c1",
  label: null,
  kind: "clock",
  config: {
    timeZone: null,
    hourCycle: "h23",
    showSeconds: false,
    iconStyle: "digital",
  },
};

const LISBON: WidgetInstance = {
  id: "c2",
  label: "Casa",
  kind: "clock",
  config: {
    timeZone: "America/Sao_Paulo",
    hourCycle: "h23",
    showSeconds: false,
    iconStyle: "analog",
  },
};

const PROGRESS: WidgetInstance = {
  id: "p1",
  label: null,
  kind: "time-progress",
  config: {
    periods: ["day", "week"],
    weekStart: "monday",
    workday: { start: "09:00", end: "18:00" },
  },
};

const NOTE: WidgetInstance = {
  id: "n1",
  label: null,
  kind: "note",
  config: {},
};

function snapshot(
  widgets: WidgetInstance[],
  storage: WidgetsSnapshot["storage"] = { state: "ok" },
): WidgetsSnapshot {
  return { storage, widgets };
}

const THREE = snapshot([CLOCK, PROGRESS, NOTE]);

/** Twelve clocks: the limit. */
const TWELVE = snapshot(
  Array.from({ length: 12 }, (_, i) => ({ ...CLOCK, id: `c${i}` })),
);

/** A promise settled by the test. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Answers each command; `get_note` with `notes[id]`. */
function answer(notes: Record<string, string> = {}) {
  invoke.mockImplementation((command: string, args: { id?: string }) => {
    switch (command) {
      case "get_note":
        return Promise.resolve(notes[args.id ?? ""] ?? "");
      case "add_widget":
        return Promise.resolve({ ...NOTE, id: "new" });
      default:
        return Promise.resolve(undefined);
    }
  });
}

/** Lets settled promises run their callbacks. */
const flush = () => act(() => Promise.resolve());

function list() {
  return screen.getByRole("list", { name: text.items.listLabel });
}

function rows() {
  return within(list()).getAllByRole("listitem");
}

function button(name: string) {
  return screen.getByRole("button", { name });
}

function moveUp(name: string) {
  return button(`Subir ${name}`);
}

function moveDown(name: string) {
  return button(`Descer ${name}`);
}

function removeButton(name: string) {
  return button(`Remover ${name}`);
}

/** The commands sent with `name`, by their arguments. */
function sent(name: string) {
  return invoke.mock.calls
    .filter(([command]) => command === name)
    .map(([, args]) => args as unknown);
}

describe("DockItems", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
    answer({ n1: "Comprar café\nLigar\n" });
  });

  it("mounts with the focus on Voltar and goes back with it", async () => {
    const onBack = vi.fn();
    render(<DockItems snapshot={THREE} onBack={onBack} />);
    await flush();

    expect(
      screen.getByRole("heading", { name: text.items.title }),
    ).toBeInTheDocument();
    expect(button(text.common.back)).toHaveFocus();
    fireEvent.click(button(text.common.back));
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("lists the widgets in dock order with name and detail", async () => {
    render(
      <DockItems
        snapshot={snapshot([LISBON, CLOCK, PROGRESS, NOTE])}
        onBack={vi.fn()}
      />,
    );
    await flush();

    expect(rows().map((row) => row.textContent)).toEqual([
      "CasaRelógio · Sao Paulo",
      "RelógioFuso do sistema",
      "Progresso do tempoDia, Semana",
      "Nota2 linhas",
    ]);
    expect(invoke).toHaveBeenCalledWith("get_note", { id: "n1" });
    expect(screen.getByText(text.items.footer)).toBeInTheDocument();
  });

  it("says an empty note is empty", async () => {
    answer({ n1: "\n" });
    render(<DockItems snapshot={snapshot([NOTE])} onBack={vi.fn()} />);
    await flush();

    expect(rows()[0]).toHaveTextContent(`Nota${text.note.empty}`);
  });

  it("shows only the kind of a labelled note it can't read", async () => {
    invoke.mockImplementation((command: string) =>
      command === "get_note"
        ? Promise.reject({ code: "invalidConfig" })
        : Promise.resolve(undefined),
    );
    render(
      <DockItems
        snapshot={snapshot([{ ...NOTE, label: "Lista" }])}
        onBack={vi.fn()}
      />,
    );
    await flush();

    expect(rows()[0]?.textContent).toBe("ListaNota");
  });

  it("turns off subir on the first and descer on the last", async () => {
    render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
    await flush();

    expect(moveUp("Relógio")).toBeDisabled();
    expect(moveDown("Relógio")).toBeEnabled();
    expect(moveUp("Progresso do tempo")).toBeEnabled();
    expect(moveDown("Progresso do tempo")).toBeEnabled();
    expect(moveUp("Nota")).toBeEnabled();
    expect(moveDown("Nota")).toBeDisabled();
  });

  it("moves with move_widget and keeps the focus on the moved item", async () => {
    const { rerender } = render(
      <DockItems snapshot={THREE} onBack={vi.fn()} />,
    );
    await flush();

    act(() => moveUp("Progresso do tempo").focus());
    fireEvent.click(moveUp("Progresso do tempo"));
    expect(invoke).toHaveBeenCalledWith("move_widget", {
      id: "p1",
      direction: "up",
    });
    await flush();
    rerender(
      <DockItems
        snapshot={snapshot([PROGRESS, CLOCK, NOTE])}
        onBack={vi.fn()}
      />,
    );

    // At the top its subir turns off: the focus goes to its descer.
    expect(moveUp("Progresso do tempo")).toBeDisabled();
    expect(moveDown("Progresso do tempo")).toHaveFocus();
  });

  it("keeps the focus on the same button while it can still move", async () => {
    const { rerender } = render(
      <DockItems snapshot={THREE} onBack={vi.fn()} />,
    );
    await flush();

    act(() => moveDown("Relógio").focus());
    fireEvent.click(moveDown("Relógio"));
    await flush();
    rerender(
      <DockItems
        snapshot={snapshot([PROGRESS, CLOCK, NOTE])}
        onBack={vi.fn()}
      />,
    );

    expect(moveDown("Relógio")).toHaveFocus();
  });

  it("offers the three kinds with the count and adds with add_widget", async () => {
    render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
    await flush();

    expect(screen.getByText(text.items.add)).toBeInTheDocument();
    expect(screen.getByText("3 de 12")).toBeInTheDocument();
    for (const [kind, name] of [
      ["clock", "Relógio"],
      ["time-progress", "Progresso"],
      ["note", "Nota"],
    ] as const) {
      fireEvent.click(button(name));
      expect(invoke).toHaveBeenCalledWith("add_widget", { kind });
    }
  });

  it("turns the add buttons off at the limit", async () => {
    render(<DockItems snapshot={TWELVE} onBack={vi.fn()} />);
    await flush();

    expect(screen.getByText("Limite de 12 widgets")).toBeInTheDocument();
    expect(screen.queryByText("12 de 12")).toBeNull();
    for (const name of ["Relógio", "Progresso", "Nota"]) {
      expect(button(name)).toBeDisabled();
    }
  });

  it("moves the focus to the new item when adding reaches the limit", async () => {
    const eleven = snapshot(TWELVE.widgets.slice(0, 11));
    const { rerender } = render(
      <DockItems snapshot={eleven} onBack={vi.fn()} />,
    );
    await flush();

    act(() => button("Nota").focus());
    fireEvent.click(button("Nota"));
    await flush();
    rerender(
      <DockItems
        snapshot={snapshot([...eleven.widgets, { ...NOTE, id: "new" }])}
        onBack={vi.fn()}
      />,
    );
    await flush();

    expect(button("Nota")).toBeDisabled();
    expect(moveUp("Nota")).toHaveFocus();
  });

  it("says so when the dock has no widgets", async () => {
    render(<DockItems snapshot={snapshot([])} onBack={vi.fn()} />);
    await flush();

    expect(screen.getByText(text.items.empty)).toBeInTheDocument();
    expect(screen.queryByRole("list")).toBeNull();
    expect(screen.getByText("0 de 12")).toBeInTheDocument();
  });

  it("shows the recovered notice before the list and dismisses it", async () => {
    const pending = deferred<void>();
    invoke.mockImplementation((command: string) =>
      command === "dismiss_storage_notice"
        ? pending.promise
        : Promise.resolve(""),
    );
    const { rerender } = render(
      <DockItems
        snapshot={snapshot([CLOCK], { state: "recovered" })}
        onBack={vi.fn()}
      />,
    );
    await flush();

    const notice = screen.getByRole("status");
    expect(notice).toHaveTextContent(text.items.recovered.title);
    expect(notice).toHaveTextContent(text.items.recovered.detail);
    expect(
      notice.compareDocumentPosition(list()) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    act(() => button(text.items.recovered.dismiss).focus());
    fireEvent.click(button(text.items.recovered.dismiss));
    expect(invoke).toHaveBeenCalledWith("dismiss_storage_notice", {});
    expect(button(text.items.recovered.dismiss)).toBeDisabled();

    pending.resolve();
    await flush();
    rerender(<DockItems snapshot={snapshot([CLOCK])} onBack={vi.fn()} />);

    expect(screen.queryByRole("status")).toBeNull();
    // The notice went with the focus: it goes to Voltar.
    expect(button(text.common.back)).toHaveFocus();
  });

  it("shows the reason when dismissing fails and lets it try again", async () => {
    invoke.mockImplementation((command: string) =>
      command === "dismiss_storage_notice"
        ? Promise.reject({ code: "writeFailed" })
        : Promise.resolve(""),
    );
    render(
      <DockItems
        snapshot={snapshot([], { state: "recovered" })}
        onBack={vi.fn()}
      />,
    );
    await flush();

    fireEvent.click(button(text.items.recovered.dismiss));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.writeFailed,
    );
    expect(button(text.items.recovered.dismiss)).toBeEnabled();
  });

  it("shows only the unavailable notice when the storage can't be used", async () => {
    render(
      <DockItems
        snapshot={snapshot([], {
          state: "unavailable",
          reason: "newerVersion",
        })}
        onBack={vi.fn()}
      />,
    );
    await flush();

    const notice = screen.getByRole("status");
    expect(notice).toHaveTextContent(text.items.unavailableNotice.title);
    expect(notice).toHaveTextContent(text.items.unavailableNotice.detail);
    expect(screen.queryByRole("list")).toBeNull();
    expect(screen.queryByText(text.items.add)).toBeNull();
    expect(screen.queryByText(text.items.empty)).toBeNull();
    expect(screen.getByText(text.items.footerUnavailable)).toBeInTheDocument();
  });

  it("shows neither list nor add while the widgets can't be read", () => {
    render(<DockItems snapshot={null} onBack={vi.fn()} />);

    expect(button(text.common.back)).toBeInTheDocument();
    expect(screen.queryByRole("list")).toBeNull();
    expect(screen.queryByText(text.items.add)).toBeNull();
    expect(screen.getByText(text.items.footerUnavailable)).toBeInTheDocument();
  });

  it("shows the message of a refused change and clears it on the next", async () => {
    invoke.mockImplementation((command: string) =>
      command === "move_widget"
        ? Promise.reject({ code: "notFound" })
        : command === "add_widget"
          ? Promise.reject({ code: "limitReached" })
          : Promise.resolve(""),
    );
    render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
    await flush();

    fireEvent.click(moveDown("Relógio"));
    await flush();
    expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notFound);

    fireEvent.click(button("Relógio"));
    expect(screen.queryByRole("alert")).toBeNull();
    await flush();
    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.limitReached,
    );
  });

  it("shows notReady when a command fails without a code", async () => {
    invoke.mockImplementation((command: string) =>
      command === "move_widget"
        ? Promise.reject(new Error("ipc"))
        : Promise.resolve(""),
    );
    render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
    await flush();

    fireEvent.click(moveDown("Relógio"));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notReady);
  });

  describe("removing", () => {
    beforeEach(() => forgetNote("n1"));

    it("removes a clock at once and moves the focus to the next item", async () => {
      const { rerender } = render(
        <DockItems snapshot={THREE} onBack={vi.fn()} />,
      );
      await flush();

      act(() => removeButton("Relógio").focus());
      fireEvent.click(removeButton("Relógio"));
      await flush();
      expect(sent("remove_widget")).toEqual([{ id: "c1" }]);
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();

      rerender(
        <DockItems snapshot={snapshot([PROGRESS, NOTE])} onBack={vi.fn()} />,
      );
      expect(removeButton("Progresso do tempo")).toHaveFocus();
    });

    it("removes the last item and moves the focus to the one before", async () => {
      answer({ n1: "" });
      const { rerender } = render(
        <DockItems snapshot={THREE} onBack={vi.fn()} />,
      );
      await flush();

      act(() => removeButton("Nota").focus());
      fireEvent.click(removeButton("Nota"));
      await flush();
      // An empty note goes without asking.
      expect(sent("remove_widget")).toEqual([{ id: "n1" }]);

      rerender(
        <DockItems snapshot={snapshot([CLOCK, PROGRESS])} onBack={vi.fn()} />,
      );
      expect(removeButton("Progresso do tempo")).toHaveFocus();
    });

    it("moves the focus to the first add button when the list empties", async () => {
      const { rerender } = render(
        <DockItems snapshot={snapshot([PROGRESS])} onBack={vi.fn()} />,
      );
      await flush();

      act(() => removeButton("Progresso do tempo").focus());
      fireEvent.click(removeButton("Progresso do tempo"));
      await flush();
      rerender(<DockItems snapshot={snapshot([])} onBack={vi.fn()} />);

      expect(button(text.addNames.clock)).toHaveFocus();
    });

    it("asks in the row before removing a note with text", async () => {
      const { rerender } = render(
        <DockItems snapshot={THREE} onBack={vi.fn()} />,
      );
      await flush();

      fireEvent.click(removeButton("Nota"));
      await flush();
      const row = rows()[2]!;
      const confirm = within(row).getByRole("alert");
      expect(confirm).toHaveTextContent(text.note.confirmRemove.title);
      expect(sent("remove_widget")).toEqual([]);
      expect(
        within(row).getByRole("button", {
          name: text.note.confirmRemove.cancel,
        }),
      ).toHaveFocus();

      // Cancelar: the remove button comes back with the focus.
      fireEvent.click(
        within(row).getByRole("button", {
          name: text.note.confirmRemove.cancel,
        }),
      );
      expect(within(row).queryByRole("alert")).not.toBeInTheDocument();
      expect(removeButton("Nota")).toHaveFocus();

      fireEvent.click(removeButton("Nota"));
      await flush();
      fireEvent.click(
        within(rows()[2]!).getByRole("button", {
          name: text.note.confirmRemove.confirm,
        }),
      );
      await flush();
      expect(sent("remove_widget")).toEqual([{ id: "n1" }]);

      rerender(
        <DockItems snapshot={snapshot([CLOCK, PROGRESS])} onBack={vi.fn()} />,
      );
      expect(removeButton("Progresso do tempo")).toHaveFocus();
    });

    it("asks for a note whose text isn't saved, and forgets it once removed", async () => {
      answer({ n1: "" });
      keepDraft("n1", { text: "Pão", error: "writeFailed" });
      render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
      await flush();

      fireEvent.click(removeButton("Nota"));
      await flush();
      fireEvent.click(
        within(rows()[2]!).getByRole("button", {
          name: text.note.confirmRemove.confirm,
        }),
      );
      await flush();

      expect(sent("remove_widget")).toEqual([{ id: "n1" }]);
      expect(draftOf("n1")).toBeNull();
    });

    it("asks for a note it can't read", async () => {
      invoke.mockImplementation((command: string) =>
        command === "get_note"
          ? Promise.reject({ code: "invalidConfig" })
          : Promise.resolve(undefined),
      );
      render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
      await flush();

      fireEvent.click(removeButton("Nota"));
      await flush();
      expect(within(rows()[2]!).getByRole("alert")).toHaveTextContent(
        text.note.confirmRemove.title,
      );
    });

    it("keeps the item and the focus when the removal is refused", async () => {
      invoke.mockImplementation((command: string) =>
        command === "remove_widget"
          ? Promise.reject({ code: "writeFailed" })
          : Promise.resolve(""),
      );
      render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
      await flush();

      act(() => removeButton("Relógio").focus());
      fireEvent.click(removeButton("Relógio"));
      await flush();

      expect(screen.getByRole("alert")).toHaveTextContent(
        text.errors.writeFailed,
      );
      expect(removeButton("Relógio")).toHaveFocus();
      expect(removeButton("Relógio")).toBeEnabled();
    });
  });

  it("works from the keyboard: every control is a button in Tab order", async () => {
    render(<DockItems snapshot={THREE} onBack={vi.fn()} />);
    await flush();

    for (const control of screen.getAllByRole("button")) {
      expect(control.tagName).toBe("BUTTON");
      expect(control).not.toHaveAttribute("tabindex", "-1");
    }
  });
});

describe("DockItems.css", () => {
  it("scrolls the list above 268 px", () => {
    expect(css).toMatch(/\.dock-items-list\s*\{[^}]*max-height:\s*268px/);
    expect(css).toMatch(/\.dock-items-list\s*\{[^}]*overflow-y:\s*auto/);
  });

  it("uses only tokens that tokens.css defines", () => {
    const defined = new Set(
      [...tokens.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const own = new Set(
      [...css.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const used = [...css.matchAll(/var\((--[\w-]+)/g)].map((match) => match[1]);
    expect(used.length).toBeGreaterThan(0);
    expect(
      used.filter((token) => !defined.has(token) && !own.has(token)),
    ).toEqual([]);
  });
});
