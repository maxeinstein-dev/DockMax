import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  MAX_APPS,
  forgetIcons,
  type AppsSnapshot,
  type CatalogEntry,
  type PinnedApp,
} from "../core/apps";
import { ptBR } from "../i18n/pt-BR";
import AppsPanel from "./AppsPanel";

const text = ptBR.apps;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const app = (n: number, name: string): PinnedApp => ({
  id: `0199a4c0-0000-7000-8000-00000000000${n}`,
  appId: `app${n}!App`,
  name,
});
const CALCULATOR = app(1, "Calculadora");
const NOTEPAD = app(2, "Bloco de notas");

const CATALOG: CatalogEntry[] = [
  { appId: "app1!App", name: "Calculadora" },
  { appId: "app2!App", name: "Bloco de notas" },
  { appId: "app3!App", name: "Configurações" },
  { appId: "app4!App", name: "Câmera" },
  { appId: "app5!App", name: "Zoom" },
];

function snapshot(
  apps: PinnedApp[],
  storage: AppsSnapshot["storage"] = { state: "ok" },
): AppsSnapshot {
  return { storage, apps };
}

/** Answers the commands; `overrides` by command name. */
function answer(
  overrides: Record<string, (args: unknown) => Promise<unknown>> = {},
) {
  invoke.mockImplementation((command: string, args: unknown) => {
    const custom = overrides[command];
    if (custom) return custom(args);
    switch (command) {
      case "list_catalog":
        return Promise.resolve(CATALOG);
      case "app_icon":
        return Promise.resolve("data:image/png;base64,AA==");
      default:
        return Promise.resolve(undefined);
    }
  });
}

const flush = () => act(() => Promise.resolve());

async function renderPanel(
  apps: AppsSnapshot | null,
  onBack: () => void = () => {},
) {
  const view = render(<AppsPanel snapshot={apps} onBack={onBack} />);
  await flush();
  return view;
}

const button = (name: string) => screen.getByRole("button", { name });
const pinnedList = () => screen.getByRole("list", { name: text.listLabel });
const catalogList = () => screen.getByRole("list", { name: text.catalogLabel });
const sent = (name: string) =>
  invoke.mock.calls
    .filter(([command]) => command === name)
    .map(([, args]) => args as unknown);

describe("AppsPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    forgetIcons();
    answer();
  });

  describe("the pinned programs", () => {
    it("lists them in dock order, each with subir, descer and remover", async () => {
      await renderPanel(snapshot([CALCULATOR, NOTEPAD]));

      const rows = within(pinnedList()).getAllByRole("listitem");
      expect(rows.map((row) => row.textContent)).toEqual([
        "Calculadora",
        "Bloco de notas",
      ]);
      for (const name of ["Calculadora", "Bloco de notas"]) {
        expect(button(`Subir ${name}`)).toBeInTheDocument();
        expect(button(`Descer ${name}`)).toBeInTheDocument();
        expect(button(`Remover ${name} do dock`)).toBeInTheDocument();
      }
    });

    it("turns off subir on the first and descer on the last", async () => {
      await renderPanel(snapshot([CALCULATOR, NOTEPAD]));

      expect(button("Subir Calculadora")).toBeDisabled();
      expect(button("Descer Calculadora")).toBeEnabled();
      expect(button("Descer Bloco de notas")).toBeDisabled();
    });

    it("says there are none, and the count", async () => {
      await renderPanel(snapshot([]));

      expect(screen.getByText(text.empty)).toBeInTheDocument();
      expect(screen.getByText(`0 de ${MAX_APPS}`)).toBeInTheDocument();
    });

    it("moves a program by the id of the pinned item", async () => {
      await renderPanel(snapshot([CALCULATOR, NOTEPAD]));

      fireEvent.click(button("Descer Calculadora"));

      expect(sent("move_app")).toEqual([
        { id: CALCULATOR.id, direction: "down" },
      ]);
    });

    it("unpins a program by the id of the pinned item", async () => {
      await renderPanel(snapshot([CALCULATOR, NOTEPAD]));

      fireEvent.click(button("Remover Bloco de notas do dock"));

      expect(sent("unpin_app")).toEqual([{ id: NOTEPAD.id }]);
    });

    it("shows the reason in the footer when a change is refused", async () => {
      answer({ move_app: () => Promise.reject({ code: "notFound" }) });
      await renderPanel(snapshot([CALCULATOR, NOTEPAD]));

      fireEvent.click(button("Descer Calculadora"));
      await flush();

      expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notFound);
    });

    it("says notReady for a failure that has no code", async () => {
      answer({ unpin_app: () => Promise.reject(new Error("x")) });
      await renderPanel(snapshot([CALCULATOR]));

      fireEvent.click(button("Remover Calculadora do dock"));
      await flush();

      expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notReady);
    });

    it("clears the reason at the next action", async () => {
      answer({ move_app: () => Promise.reject({ code: "notFound" }) });
      await renderPanel(snapshot([CALCULATOR, NOTEPAD]));
      fireEvent.click(button("Descer Calculadora"));
      await flush();

      answer();
      fireEvent.click(button("Descer Calculadora"));
      await flush();

      expect(screen.queryByRole("alert")).toBeNull();
    });
  });

  describe("the catalog", () => {
    it("is asked for once the apps are available, and lists the programs", async () => {
      await renderPanel(snapshot([]));

      expect(sent("list_catalog")).toHaveLength(1);
      const names = within(catalogList())
        .getAllByRole("listitem")
        .map((row) => row.textContent);
      expect(names).toEqual(CATALOG.map((entry) => entry.name));
    });

    it("says it is reading while the list has not come", async () => {
      answer({ list_catalog: () => new Promise(() => {}) });
      await renderPanel(snapshot([]));

      expect(screen.getByRole("status")).toHaveTextContent(text.loading);
    });

    it("searches by name without regard to case or accents", async () => {
      await renderPanel(snapshot([]));

      fireEvent.change(
        screen.getByRole("textbox", { name: text.searchLabel }),
        {
          target: { value: "CAMERA" },
        },
      );

      const names = within(catalogList())
        .getAllByRole("listitem")
        .map((row) => row.textContent);
      expect(names).toEqual(["Câmera"]);
      fireEvent.change(
        screen.getByRole("textbox", { name: text.searchLabel }),
        {
          target: { value: "configuracoes" },
        },
      );
      expect(
        within(catalogList())
          .getAllByRole("listitem")
          .map((row) => row.textContent),
      ).toEqual(["Configurações"]);
    });

    it("says when nothing matches", async () => {
      await renderPanel(snapshot([]));

      fireEvent.change(
        screen.getByRole("textbox", { name: text.searchLabel }),
        {
          target: { value: "xyz" },
        },
      );

      expect(screen.getByText(text.noMatch)).toBeInTheDocument();
      expect(
        screen.queryByRole("list", { name: text.catalogLabel }),
      ).toBeNull();
    });

    it("pins a program on a click, by its id in the Apps folder", async () => {
      await renderPanel(snapshot([]));

      fireEvent.click(button("Fixar Zoom"));

      expect(sent("pin_app")).toEqual([{ appId: "app5!App" }]);
    });

    it("marks the programs already pinned and does not pin them again", async () => {
      await renderPanel(snapshot([CALCULATOR]));

      const pinned = button("Calculadora (fixado)");

      expect(pinned).toBeDisabled();
      expect(button("Fixar Zoom")).toBeEnabled();
    });

    it("puts the focus back on the search field after pinning", async () => {
      await renderPanel(snapshot([]));

      fireEvent.click(button("Fixar Zoom"));
      await flush();

      expect(
        screen.getByRole("textbox", { name: text.searchLabel }),
      ).toHaveFocus();
    });

    it("shows why a pin was refused", async () => {
      answer({ pin_app: () => Promise.reject({ code: "alreadyPinned" }) });
      await renderPanel(snapshot([]));

      fireEvent.click(button("Fixar Zoom"));
      await flush();

      expect(screen.getByRole("alert")).toHaveTextContent(
        text.errors.alreadyPinned,
      );
    });

    it("turns pinning off at the limit and says so", async () => {
      const many = Array.from({ length: MAX_APPS }, (_, n) => ({
        id: `0199a4c0-0000-7000-8000-${String(n).padStart(12, "0")}`,
        appId: `many${n}!App`,
        name: `App ${n}`,
      }));
      await renderPanel(snapshot(many));

      expect(button("Fixar Zoom")).toBeDisabled();
      expect(
        screen.getByText(`Limite de ${MAX_APPS} apps`),
      ).toBeInTheDocument();
    });

    it("says it could not read the programs, and tries again", async () => {
      answer({ list_catalog: () => Promise.reject({ code: "unavailable" }) });
      await renderPanel(snapshot([]));
      expect(screen.getByRole("alert")).toHaveTextContent(text.catalogFailed);

      answer();
      fireEvent.click(button(text.retry));
      await flush();

      expect(sent("list_catalog")).toHaveLength(2);
      expect(catalogList()).toBeInTheDocument();
    });

    it("reads the icons of the list at the size of the list", async () => {
      await renderPanel(snapshot([CALCULATOR]));

      expect(invoke).toHaveBeenCalledWith("app_icon", {
        appId: "app1!App",
        size: 32,
      });
    });
  });

  describe("without a database", () => {
    it("says the apps are unavailable and offers nothing to change", async () => {
      await renderPanel(
        snapshot([], { state: "unavailable", reason: "openFailed" }),
      );

      expect(
        screen.getByText(text.unavailableNotice.title),
      ).toBeInTheDocument();
      expect(screen.queryByRole("list", { name: text.listLabel })).toBeNull();
      expect(sent("list_catalog")).toHaveLength(0);
      expect(screen.getByText(text.footerUnavailable)).toBeInTheDocument();
    });

    it("says the same while the list can't be read", async () => {
      await renderPanel(null);

      expect(
        screen.getByText(text.unavailableNotice.title),
      ).toBeInTheDocument();
    });
  });

  it("goes back from Voltar", async () => {
    const onBack = vi.fn();
    await renderPanel(snapshot([]), onBack);

    fireEvent.click(button("Voltar"));

    expect(onBack).toHaveBeenCalledOnce();
  });
});
