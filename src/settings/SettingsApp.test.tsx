import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import { ptBR } from "../i18n/pt-BR";
import SettingsApp from "./SettingsApp";

const text = ptBR.settings.app;

type Listener = (event: { payload: unknown }) => void;

const { invoke, close, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
  close: vi.fn(() => Promise.resolve()),
  listeners: new Map<string, Listener>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: Listener) => {
    listeners.set(event, handler);
    return Promise.resolve(() => {});
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close }),
}));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

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

async function renderApp() {
  const view = render(<SettingsApp />);
  await act(async () => {});
  return view;
}

const nav = () => screen.getByRole("navigation", { name: text.nav });
const sectionButton = (name: string) =>
  within(nav()).getByRole("button", { name });

describe("SettingsApp", () => {
  beforeEach(() => {
    delete window.__DOCKMAX_SECTION__;
    listeners.clear();
    close.mockClear();
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "get_settings") return Promise.resolve(SETTINGS);
      if (command === "list_widgets") {
        return Promise.resolve({ storage: { state: "ok" }, widgets: [] });
      }
      if (command === "list_apps") {
        return Promise.resolve({ storage: { state: "ok" }, apps: [] });
      }
      return new Promise(() => {});
    });
  });

  it("lists its sections in the side bar, Geral first and chosen", async () => {
    await renderApp();

    expect(
      within(nav())
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual([
      "Geral",
      "Aparência do dock",
      "Apps do dock",
      "Itens do dock",
      "Sobre",
    ]);
    expect(sectionButton("Geral")).toHaveAttribute("aria-current", "page");
    expect(
      screen.getByRole("group", { name: ptBR.settings.groups.theme }),
    ).toBeInTheDocument();
  });

  it("has no rows that open subviews: the sections are in the side bar", async () => {
    await renderApp();

    expect(
      screen.queryByRole("button", { name: /Aparência do dock.*Nativo/ }),
    ).toBeNull();
  });

  it("shows the look of the dock with no Voltar, at a width that fits it", async () => {
    await renderApp();

    fireEvent.click(sectionButton("Aparência do dock"));

    expect(sectionButton("Aparência do dock")).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(sectionButton("Geral")).not.toHaveAttribute("aria-current");
    expect(
      screen.getByRole("heading", { level: 1, name: ptBR.settings.look.title }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Voltar" })).toBeNull();
    expect(document.querySelector(".settings-card")).toHaveAttribute(
      "data-section",
      "look",
    );
  });

  it.each([
    ["Apps do dock", ptBR.apps.title],
    ["Itens do dock", ptBR.widgets.items.title],
    ["Sobre", ptBR.panel.titles.about],
  ])("shows %s under its own title", async (section, title) => {
    await renderApp();

    fireEvent.click(sectionButton(section));

    expect(
      screen.getByRole("heading", { level: 1, name: title }),
    ).toBeInTheDocument();
  });

  it("starts on the section the launcher asked for", async () => {
    window.__DOCKMAX_SECTION__ = "items";
    await renderApp();

    expect(sectionButton("Itens do dock")).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("starts on Geral when asked for a section it does not have", async () => {
    window.__DOCKMAX_SECTION__ = "developer";
    await renderApp();

    expect(sectionButton("Geral")).toHaveAttribute("aria-current", "page");
  });

  it("shows the section the launcher found while open", async () => {
    await renderApp();

    act(() => listeners.get("settings-section")?.({ payload: "about" }));
    expect(sectionButton("Sobre")).toHaveAttribute("aria-current", "page");
    act(() => listeners.get("settings-section")?.({ payload: "nope" }));
    expect(sectionButton("Sobre")).toHaveAttribute("aria-current", "page");
  });

  it("closes its window with Esc", async () => {
    await renderApp();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(close).toHaveBeenCalledTimes(1);
  });
});
