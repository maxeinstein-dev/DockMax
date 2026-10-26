import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppMenu, PinnedApp } from "../core/apps";
import { ptBR } from "../i18n/pt-BR";
import AppMenuPanel from "./AppMenuPanel";

const text = ptBR.apps.menu;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const APP: PinnedApp = {
  id: "0199a4c0-0000-7000-8000-000000000001",
  appId: "Some.App!App",
  name: "Algum App",
};

const MENU: AppMenu = {
  canReveal: true,
  sections: [
    {
      kind: "category",
      title: "Pastas recentes",
      entries: [
        { key: 0, title: "projeto-a" },
        { key: 1, title: "projeto-b" },
      ],
    },
    { kind: "recent", title: null, entries: [{ key: 2, title: "nota.txt" }] },
    { kind: "tasks", title: null, entries: [{ key: 3, title: "Nova janela" }] },
  ],
};

function answer(
  menu: AppMenu | Error | Promise<AppMenu> = MENU,
  others: Record<string, () => Promise<unknown>> = {},
) {
  invoke.mockImplementation((command: string) => {
    if (command === "app_menu") {
      return menu instanceof Error
        ? Promise.reject(menu)
        : Promise.resolve(menu);
    }
    return others[command]?.() ?? Promise.resolve(undefined);
  });
}

const flush = () => act(() => Promise.resolve());

async function renderMenu(onDone: () => void = () => {}, openCount?: number) {
  const view = render(
    <AppMenuPanel app={APP} openCount={openCount} onDone={onDone} />,
  );
  await flush();
  return view;
}

const item = (name: string) => screen.getByRole("menuitem", { name });
const sent = (name: string) =>
  invoke.mock.calls
    .filter(([command]) => command === name)
    .map(([, args]) => args as unknown);

describe("AppMenuPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    answer();
  });

  it("is titled with the name of the program and asks for its menu once", async () => {
    await renderMenu();

    expect(
      screen.getByRole("heading", { name: "Algum App", level: 1 }),
    ).toBeInTheDocument();
    expect(sent("app_menu")).toEqual([{ id: APP.id }]);
    expect(
      screen.getByRole("menu", { name: "Menu de Algum App" }),
    ).toBeInTheDocument();
  });

  it("shows the sections as the taskbar does, each under its heading", async () => {
    await renderMenu();

    const groups = screen.getAllByRole("group");
    const headings = groups.map(
      (group) => within(group).queryByRole("heading")?.textContent ?? null,
    );
    // The program's own category by its name, then recent, tasks, actions.
    expect(headings).toEqual([
      "Pastas recentes",
      text.sections.recent,
      text.sections.tasks,
      null,
    ]);
    expect(within(groups[0]!).getAllByRole("menuitem")).toHaveLength(2);
  });

  it("names a category without a title as recent ones", async () => {
    answer({
      canReveal: false,
      sections: [
        { kind: "category", title: null, entries: MENU.sections[0]!.entries },
      ],
    });
    await renderMenu();

    expect(
      screen.getByRole("heading", { name: text.sections.category }),
    ).toBeInTheDocument();
  });

  it("runs an entry by its key and closes", async () => {
    const onDone = vi.fn();
    await renderMenu(onDone);

    fireEvent.click(item("projeto-b"));
    await flush();

    expect(sent("run_app_entry")).toEqual([{ id: APP.id, key: 1 }]);
    expect(onDone).toHaveBeenCalledOnce();
  });

  it("opens the program, shows its file and removes it, by the pinned id", async () => {
    const onDone = vi.fn();
    await renderMenu(onDone);

    fireEvent.click(item(text.open));
    fireEvent.click(item(text.reveal));
    fireEvent.click(item(text.remove));
    await flush();

    expect(sent("launch_app")).toEqual([{ id: APP.id }]);
    expect(sent("reveal_app")).toEqual([{ id: APP.id }]);
    expect(sent("unpin_app")).toEqual([{ id: APP.id }]);
    expect(onDone).toHaveBeenCalledTimes(3);
  });

  it("offers to show the file only for a program that has one", async () => {
    answer({ ...MENU, canReveal: false });
    await renderMenu();

    expect(screen.queryByRole("menuitem", { name: text.reveal })).toBeNull();
    expect(item(text.open)).toBeInTheDocument();
  });

  it("keeps the actions while the menu is being read", async () => {
    answer(new Promise<AppMenu>(() => {}));
    await renderMenu();

    expect(screen.getByRole("status")).toHaveTextContent(text.loading);
    expect(item(text.open)).toBeInTheDocument();
    expect(item(text.remove)).toBeInTheDocument();
  });

  it("says it could not read the menu and keeps the actions", async () => {
    answer(new Error("x"));
    await renderMenu();

    expect(screen.getByRole("alert")).toHaveTextContent(text.failed);
    expect(item(text.open)).toBeInTheDocument();
  });

  it("shows why an entry did not run and stays open", async () => {
    const onDone = vi.fn();
    answer(MENU, {
      run_app_entry: () => Promise.reject({ code: "launchFailed" }),
    });
    await renderMenu(onDone);

    fireEvent.click(item("nota.txt"));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(
      ptBR.apps.errors.launchFailed,
    );
    expect(onDone).not.toHaveBeenCalled();
  });

  it("takes the focus on the first item once the menu is there", async () => {
    await renderMenu();

    expect(item("projeto-a")).toHaveFocus();
  });

  it("moves between the items with the arrows, Home and End", async () => {
    await renderMenu();
    const first = item("projeto-a");

    fireEvent.keyDown(first, { key: "ArrowDown" });
    expect(item("projeto-b")).toHaveFocus();
    fireEvent.keyDown(item("projeto-b"), { key: "End" });
    expect(item(text.remove)).toHaveFocus();
    fireEvent.keyDown(item(text.remove), { key: "ArrowDown" });
    expect(item(text.remove)).toHaveFocus();
    fireEvent.keyDown(item(text.remove), { key: "ArrowUp" });
    expect(item(text.reveal)).toHaveFocus();
    fireEvent.keyDown(item(text.reveal), { key: "Home" });
    expect(first).toHaveFocus();
    fireEvent.keyDown(first, { key: "ArrowUp" });
    expect(first).toHaveFocus();
  });

  describe("closing the windows of an open program (spec 0009 D3)", () => {
    it("offers nothing to close while the program has no window", async () => {
      await renderMenu();

      expect(
        screen.queryByRole("menuitem", { name: text.closeWindow }),
      ).toBeNull();
      expect(
        screen.queryByRole("menuitem", { name: text.closeWindows }),
      ).toBeNull();
    });

    it("offers to close the window, or all of them, and asks Rust by the pinned id", async () => {
      const onDone = vi.fn();
      await renderMenu(onDone, 1);
      expect(item(text.closeWindow)).toBeInTheDocument();

      fireEvent.click(item(text.closeWindow));
      await flush();

      expect(sent("close_app")).toEqual([{ id: APP.id }]);
      expect(onDone).toHaveBeenCalledTimes(1);
    });

    it("says all the windows when there are several", async () => {
      await renderMenu(() => {}, 2);

      expect(item(text.closeWindows)).toBeInTheDocument();
    });
  });
});
