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
  launcherHotkey: "Alt+Space",
  language: "system",
  tourShown: true,
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
    delete window.__DOCKYARD_SECTION__;
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
      "Espaços de trabalho",
      "Aparência do dock",
      "Apps do dock",
      "Widgets",
      "Contas",
      "Sobre",
    ]);
    expect(sectionButton("Geral")).toHaveAttribute("aria-current", "page");
    expect(
      screen.getByRole("group", { name: ptBR.settings.groups.theme }),
    ).toBeInTheDocument();
  });

  it("heads each section with its plate, title and line", async () => {
    await renderApp();

    for (const button of within(nav()).getAllByRole("button")) {
      // The plate is decorative: the button keeps the section's name.
      expect(button.querySelector(".section-plate")).toHaveAttribute(
        "aria-hidden",
        "true",
      );
    }
    expect(document.querySelector(".section-header")).toHaveTextContent(
      text.leads.general,
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      screen.getByRole("heading", { level: 1, name: "Geral" }),
    ).toBeInTheDocument();
  });

  it("shows the position and the taskbar as cards with a picture", async () => {
    await renderApp();

    const edges = screen.getByRole("group", {
      name: ptBR.settings.groups.edge,
    });
    expect(
      within(edges).getByRole("button", { name: "Embaixo" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      within(edges)
        .getByRole("button", { name: "Em cima" })
        .querySelector(".thumb-screen"),
    ).toHaveAttribute("data-edge", "top");

    const taskbar = within(
      screen.getByRole("group", { name: ptBR.settings.groups.taskbar }),
    ).getByRole("button", { name: "Visível" });
    expect(taskbar).toHaveAccessibleDescription(
      ptBR.settings.taskbarHints.visible,
    );
    expect(taskbar).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(taskbar);
    await act(async () => {});
    const saved = invoke.mock.calls.find(
      ([command]) => command === "update_settings",
    )?.[1] as { settings: Settings } | undefined;
    expect(saved?.settings.dock.taskbar).toBe("visible");
  });

  it("shows the look's choices as cards, the chosen one moving", async () => {
    await renderApp();
    fireEvent.click(sectionButton("Aparência do dock"));
    await act(async () => {});

    const look = ptBR.settings.look;
    for (const [group, chosen] of [
      [look.hover, look.hoverOptions.scale],
      [look.indicator, look.indicatorOptions.dot],
      [look.iconShape, look.iconShapeOptions.plate],
      [look.openAnimation, look.openAnimationOptions.bounce],
    ] as const) {
      const card = within(screen.getByRole("group", { name: group })).getByRole(
        "button",
        { name: chosen },
      );
      expect(card, group).toHaveAttribute("aria-pressed", "true");
      expect(card.querySelector(".thumb"), group).not.toBeNull();
    }
    const bounce = within(
      screen.getByRole("group", { name: look.openAnimation }),
    ).getByRole("button", { name: look.openAnimationOptions.bounce });
    expect(bounce.querySelector(".thumb")).toHaveAttribute("data-chosen");
    const shake = within(
      screen.getByRole("group", { name: look.openAnimation }),
    ).getByRole("button", { name: look.openAnimationOptions.shake });
    expect(shake.querySelector(".thumb")).not.toHaveAttribute("data-chosen");
  });

  it("puts Geral in titled groups, the launcher among them", async () => {
    await renderApp();

    expect(
      screen
        .getAllByRole("heading", { level: 2 })
        .map((heading) => heading.textContent),
    ).toEqual([
      text.groups.dock,
      text.groups.taskbar,
      text.groups.behavior,
      text.groups.launcher,
      ptBR.settings.groups.language,
      text.groups.startup,
    ]);
    expect(
      within(
        screen.getByRole("region", { name: text.groups.startup }),
      ).getByRole("switch", { name: ptBR.settings.launchAtLogin }),
    ).toBeInTheDocument();
  });

  it("shows the styles with the programs of the workspace in use", async () => {
    invoke.mockImplementation((command: string, args?: { appId?: string }) => {
      if (command === "get_settings") return Promise.resolve(SETTINGS);
      if (command === "list_apps") {
        return Promise.resolve({
          storage: { state: "ok" },
          apps: [
            { id: "a1", appId: "Edge", name: "Edge" },
            { id: "a2", appId: "Code", name: "Code" },
          ],
        });
      }
      if (command === "list_workspaces") {
        return Promise.resolve({
          workspaces: [
            { id: "w1", name: "Principal", color: "#0A84FF", items: [] },
            { id: "w2", name: "Trabalho", color: "#FF9F0A", items: ["app:a2"] },
          ],
          inUse: "w2",
        });
      }
      if (command === "app_icon") {
        return Promise.resolve(`data:image/png;base64,${args?.appId ?? ""}`);
      }
      return new Promise(() => {});
    });
    await renderApp();

    fireEvent.click(sectionButton("Aparência do dock"));
    await act(async () => {});

    const card = within(
      screen.getByRole("group", { name: ptBR.settings.look.style }),
    ).getByRole("button", { name: "Neon" });
    // Only the program of Trabalho, the workspace in use.
    expect(
      [...card.querySelectorAll("img")].map((img) => img.getAttribute("src")),
    ).toEqual(["data:image/png;base64,Code"]);
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
    ["Widgets", ptBR.settings.app.sections.items],
    ["Sobre", ptBR.panel.titles.about],
  ])("shows %s under its own title", async (section, title) => {
    await renderApp();

    fireEvent.click(sectionButton(section));

    expect(
      screen.getByRole("heading", { level: 1, name: title }),
    ).toBeInTheDocument();
  });

  it("starts on the section the launcher asked for", async () => {
    window.__DOCKYARD_SECTION__ = "items";
    await renderApp();

    expect(sectionButton("Widgets")).toHaveAttribute("aria-current", "page");
  });

  it("starts on Geral when asked for a section it does not have", async () => {
    window.__DOCKYARD_SECTION__ = "developer";
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
