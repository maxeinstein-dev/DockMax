import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppsSnapshot } from "../core/apps";
import type { PanelReadyArgs, PanelShowPayload } from "../core/panels";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import type {
  CreatableKind,
  WidgetInstance,
  WidgetsSnapshot,
} from "../core/widgets";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
import { ptBR } from "../i18n/pt-BR";
import type {
  WidgetPanelProps,
  WidgetSettingsProps,
} from "../widgets/contract";
import Panel from "./Panel";

/* The panel host routing to each view (W5): a widget's main view and its
 * Ajustes, Configurações and Itens do dock, Sobre. The kinds' components
 * are stand-ins that show the props they get. */

type Handler = (event: { payload: unknown }) => void;

const { invoke, listen, handlers, props } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  return {
    handlers,
    invoke: vi.fn(),
    listen: vi.fn((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(() => {});
    }),
    /** The last props each stand-in got. */
    props: {
      panel: null as WidgetPanelProps<CreatableKind> | null,
      settings: null as WidgetSettingsProps<CreatableKind> | null,
    },
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

vi.mock("../widgets/registry", async (importOriginal) => {
  const real = await importOriginal<typeof import("../widgets/registry")>();
  function StandInPanel(p: WidgetPanelProps<CreatableKind>) {
    props.panel = p;
    return (
      <>
        <h1 id="panel-title">{p.instance.label ?? p.instance.kind}</h1>
        <button type="button" onClick={p.onAdjust}>
          Ajustes
        </button>
        <output data-testid="now">{p.now.toISOString()}</output>
      </>
    );
  }
  function StandInSettings(p: WidgetSettingsProps<CreatableKind>) {
    props.settings = p;
    return (
      <>
        <h1 id="panel-title">Ajustes</h1>
        <button type="button" onClick={p.onBack}>
          Voltar
        </button>
      </>
    );
  }
  return {
    ...real,
    definitionOf: (instance: WidgetInstance<CreatableKind>) => ({
      ...real.definitionOf(instance),
      Panel: StandInPanel,
      Settings: StandInSettings,
    }),
  };
});

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

const CLOCK = WIDGET_FIXTURES["clock-sample"];
const NOTE = { ...WIDGET_FIXTURES["note-default"], label: "Mercado" };
const LIST: WidgetsSnapshot = {
  storage: { state: "ok" },
  widgets: [CLOCK, NOTE],
};

let widgets: WidgetsSnapshot | Error;
let apps: AppsSnapshot;
let openSnapshot: unknown;
let waits = 0;
/** The frame's fake height: grows to tell one measure from another. */
let frameHeight = 200;

function show(panelId: PanelShowPayload["panelId"]) {
  const handler = handlers.get("panel-show");
  if (!handler) throw new Error("panel-show listener not registered");
  act(() => handler({ payload: { panelId, wait: ++waits } }));
}

function emit(event: string, payload: unknown) {
  const handler = handlers.get(event);
  if (!handler) throw new Error(`${event} listener not registered`);
  act(() => handler({ payload }));
}

function readyCalls(): PanelReadyArgs[] {
  return invoke.mock.calls
    .filter(([command]) => command === "panel_ready")
    .map(([, args]) => args as PanelReadyArgs);
}

const hides = () =>
  invoke.mock.calls.filter(([command]) => command === "hide_panel").length;

const card = () => document.querySelector("section.panel");

/** Renders the panel and lets the settings and the widgets be read. */
async function renderPanel() {
  const view = render(<Panel />);
  await act(async () => {});
  return view;
}

describe("Panel routing", () => {
  beforeEach(() => {
    handlers.clear();
    waits = 0;
    frameHeight = 200;
    widgets = LIST;
    apps = { storage: { state: "ok" }, apps: [] };
    openSnapshot = { apps: [], others: [] };
    props.panel = null;
    props.settings = null;
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "get_settings") return Promise.resolve(SETTINGS);
      if (command === "list_apps") return Promise.resolve(apps);
      if (command === "list_catalog") return Promise.resolve([]);
      if (command === "list_open_apps") return Promise.resolve(openSnapshot);
      if (command === "app_menu") {
        return Promise.resolve({
          canReveal: false,
          sections: [
            {
              kind: "tasks",
              title: null,
              entries: [{ key: 0, title: "Nova janela" }],
            },
          ],
        });
      }
      if (command === "list_widgets") {
        return widgets instanceof Error
          ? Promise.reject(widgets)
          : Promise.resolve(widgets);
      }
      return Promise.resolve(undefined);
    });
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
      function (this: HTMLElement) {
        const height = this.classList.contains("panel-frame") ? frameHeight : 0;
        return { x: 0, y: 0, width: 316, height } as DOMRect;
      },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  describe("a widget panel", () => {
    it("shows the kind's main view, with its own title, on a card of its kind", async () => {
      await renderPanel();

      show({ widget: NOTE.id });

      expect(props.panel?.instance).toEqual(NOTE);
      expect(props.panel?.returnedFromSettings).toBe(false);
      expect(card()).toHaveAttribute("data-panel", "widget");
      expect(card()).toHaveAttribute("data-widget", "note");
      // One heading: the view's, which names the card.
      expect(screen.getAllByRole("heading")).toHaveLength(1);
      expect(screen.getByRole("region", { name: "Mercado" })).toBe(card());
      expect(readyCalls()).toEqual([
        {
          panelId: { widget: NOTE.id },
          wait: 1,
          size: { width: 316, height: 200 },
          inset: 8,
        },
      ]);

      show({ widget: CLOCK.id });
      expect(card()).toHaveAttribute("data-widget", "clock");
      expect(props.panel?.instance).toEqual(CLOCK);
    });

    it("swaps to Ajustes and back, answering panel_ready for each view", async () => {
      await renderPanel();
      show({ widget: CLOCK.id });

      frameHeight = 320;
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));

      expect(props.settings?.instance).toEqual(CLOCK);
      expect(
        screen.queryByRole("button", { name: "Ajustes" }),
      ).not.toBeInTheDocument();

      frameHeight = 180;
      fireEvent.click(screen.getByRole("button", { name: "Voltar" }));

      expect(props.panel?.returnedFromSettings).toBe(true);
      expect(readyCalls().map(({ wait, size }) => [wait, size.height])).toEqual(
        [
          [1, 200],
          [1, 320],
          [1, 180],
        ],
      );
    });

    it("starts each show on the main view", async () => {
      await renderPanel();
      show({ widget: CLOCK.id });
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));

      show({ widget: CLOCK.id });

      expect(screen.getByRole("button", { name: "Ajustes" })).toBeVisible();
      expect(props.panel?.returnedFromSettings).toBe(false);
    });

    it("saves through update_widget, the config tagged with the kind", async () => {
      await renderPanel();
      show({ widget: CLOCK.id });
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));

      await act(() =>
        props.settings!.onUpdate({ label: "Lisboa", config: CLOCK.config }),
      );

      expect(invoke).toHaveBeenCalledWith("update_widget", {
        id: CLOCK.id,
        label: "Lisboa",
        config: { kind: "clock", config: CLOCK.config },
      });
    });

    it("passes a refused update on to the view", async () => {
      await renderPanel();
      show({ widget: CLOCK.id });
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
      invoke.mockImplementation((command: string) =>
        command === "update_widget"
          ? Promise.reject({ code: "invalidLabel" })
          : Promise.resolve(undefined),
      );

      await expect(
        props.settings!.onUpdate({ label: " ", config: CLOCK.config }),
      ).rejects.toEqual({ code: "invalidLabel" });
    });

    it("removes through remove_widget and closes the panel once", async () => {
      await renderPanel();
      show({ widget: NOTE.id });
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));

      await act(() => props.settings!.onRemove());
      emit("widgets-changed", { storage: { state: "ok" }, widgets: [CLOCK] });

      expect(invoke).toHaveBeenCalledWith("remove_widget", { id: NOTE.id });
      expect(hides()).toBe(1);
    });

    it("keeps the panel open when the removal fails", async () => {
      await renderPanel();
      show({ widget: NOTE.id });
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
      invoke.mockImplementation((command: string) =>
        command === "remove_widget"
          ? Promise.reject({ code: "writeFailed" })
          : Promise.resolve(undefined),
      );

      await act(() =>
        expect(props.settings!.onRemove()).rejects.toEqual({
          code: "writeFailed",
        }),
      );

      expect(hides()).toBe(0);
    });

    it("closes when its instance is removed elsewhere", async () => {
      await renderPanel();
      show({ widget: NOTE.id });
      expect(hides()).toBe(0);

      emit("widgets-changed", { storage: { state: "ok" }, widgets: [CLOCK] });

      expect(hides()).toBe(1);
      // Nothing of the gone instance is shown.
      expect(screen.queryByRole("heading")).not.toBeInTheDocument();
      // A later change does not close it again.
      emit("widgets-changed", { storage: { state: "ok" }, widgets: [] });
      expect(hides()).toBe(1);
    });

    it("does not close while the list can't be read", async () => {
      widgets = new Error("no list");
      await renderPanel();

      show({ widget: NOTE.id });

      expect(hides()).toBe(0);
    });

    it("moves its now every second while the main view shows", async () => {
      vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
      vi.setSystemTime(Date.UTC(2026, 9, 16, 9, 41, 7, 400));
      await renderPanel();
      show({ widget: CLOCK.id });
      const now = () => screen.getByTestId("now").textContent;
      expect(now()).toBe("2026-10-16T09:41:07.400Z");

      await act(() => vi.advanceTimersByTimeAsync(600));
      expect(now()).toBe("2026-10-16T09:41:08.000Z");
      await act(() => vi.advanceTimersByTimeAsync(1000));
      expect(now()).toBe("2026-10-16T09:41:09.000Z");

      // No timer while Ajustes shows.
      fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
      expect(vi.getTimerCount()).toBe(0);
    });

    it("sends the size again when the content grows within a view", async () => {
      let resized = () => {};
      vi.stubGlobal(
        "ResizeObserver",
        class {
          constructor(callback: () => void) {
            resized = callback;
          }
          observe() {}
          disconnect() {}
        },
      );
      await renderPanel();
      show({ widget: NOTE.id });

      act(() => resized());
      expect(readyCalls()).toHaveLength(1);

      frameHeight = 260;
      act(() => resized());

      expect(readyCalls()).toHaveLength(2);
      expect(readyCalls()[1]).toMatchObject({
        wait: 1,
        size: { height: 260 },
      });
    });
  });

  describe("Configurações", () => {
    it("opens Itens do dock in place and comes back to its row", async () => {
      await renderPanel();
      show("settings");
      const row = () =>
        screen.getByRole("button", {
          name: new RegExp(ptBR.widgets.items.title),
        });
      expect(row()).toHaveTextContent("2 de 12");

      fireEvent.click(row());

      // Itens do dock brings its own title, in place of Configurações.
      expect(screen.getAllByRole("heading")).toHaveLength(1);
      expect(
        screen.getByRole("heading", { name: ptBR.widgets.items.title }),
      ).toBeInTheDocument();
      expect(readyCalls()).toHaveLength(2);

      fireEvent.click(
        screen.getByRole("button", { name: ptBR.widgets.common.back }),
      );

      expect(
        screen.getByRole("heading", { name: ptBR.panel.titles.settings }),
      ).toBeInTheDocument();
      expect(row()).toHaveFocus();
      expect(readyCalls()).toHaveLength(3);
    });

    it("opens Aparência do dock in place and comes back to its row", async () => {
      await renderPanel();
      show("settings");
      const row = () =>
        screen.getByRole("button", {
          name: new RegExp(ptBR.settings.look.title),
        });
      expect(row()).toHaveTextContent(ptBR.settings.look.styles.native);

      fireEvent.click(row());

      // It brings its own title, in place of Configurações.
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("heading", { name: ptBR.settings.look.title }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("radiogroup", { name: ptBR.settings.look.style }),
      ).toBeInTheDocument();
      expect(readyCalls()).toHaveLength(2);

      fireEvent.click(
        screen.getByRole("button", { name: ptBR.widgets.common.back }),
      );

      expect(
        screen.getByRole("heading", { name: ptBR.panel.titles.settings }),
      ).toBeInTheDocument();
      // The focus goes back to the row that opened it, not to Itens do dock.
      expect(row()).toHaveFocus();
      expect(readyCalls()).toHaveLength(3);
    });

    it("opens Apps do dock in place and comes back to its row", async () => {
      await renderPanel();
      show("settings");
      const row = () =>
        screen.getByRole("button", {
          name: new RegExp(ptBR.apps.title),
        });

      fireEvent.click(row());

      // It brings its own title, in place of Configurações.
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("heading", { name: ptBR.apps.title }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("textbox", { name: ptBR.apps.searchLabel }),
      ).toBeInTheDocument();
      expect(readyCalls()).toHaveLength(2);

      fireEvent.click(
        screen.getByRole("button", { name: ptBR.widgets.common.back }),
      );

      expect(
        screen.getByRole("heading", { name: ptBR.panel.titles.settings }),
      ).toBeInTheDocument();
      // The focus goes back to the row that opened it.
      expect(row()).toHaveFocus();
      expect(readyCalls()).toHaveLength(3);
    });

    it("keeps Sobre as it was", async () => {
      await renderPanel();
      show("about");
      expect(
        screen.getByRole("heading", { name: ptBR.panel.titles.about }),
      ).toBeInTheDocument();
      expect(card()).not.toHaveAttribute("data-widget");
    });
  });

  describe("the menu of a pinned program", () => {
    const PROGRAM = {
      id: "0199a4c0-0000-7000-8000-000000000001",
      appId: "Some.App!App",
      name: "Algum App",
    };
    const SECOND = { ...PROGRAM, id: "0199a4c0-0000-7000-8000-000000000002" };

    it("shows the menu of the program under its own title", async () => {
      apps = { storage: { state: "ok" }, apps: [PROGRAM] };
      await renderPanel();

      show({ app: PROGRAM.id });
      await act(async () => {});

      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("heading", { name: "Algum App" }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("menuitem", { name: "Nova janela" }),
      ).toBeInTheDocument();
      expect(card()).toHaveAttribute("data-panel", "app");
      expect(readyCalls().length).toBeGreaterThan(0);
    });

    it("closes when the program is unpinned, once", async () => {
      apps = { storage: { state: "ok" }, apps: [PROGRAM, SECOND] };
      await renderPanel();
      show({ app: PROGRAM.id });
      expect(hides()).toBe(0);

      emit("apps-changed", { storage: { state: "ok" }, apps: [SECOND] });

      expect(hides()).toBe(1);
      expect(screen.queryByRole("heading")).not.toBeInTheDocument();
      emit("apps-changed", { storage: { state: "ok" }, apps: [] });
      expect(hides()).toBe(1);
    });

    it("does not close while the list is not read", async () => {
      apps = undefined as unknown as AppsSnapshot;
      await renderPanel();

      show({ app: PROGRAM.id });

      expect(hides()).toBe(0);
    });

    it("shows nothing for a program the list does not have", async () => {
      await renderPanel();

      show({ app: PROGRAM.id });

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(hides()).toBe(1);
    });
  });

  describe("the menu of an open program that is not pinned", () => {
    const OTHER = {
      key: "00000000000000aa",
      appId: "C:\\Apps\\code.exe",
      name: "code",
      count: 1,
      active: false,
      badge: false,
    };

    it("shows the menu of the program under its own title", async () => {
      openSnapshot = { apps: [], others: [OTHER] };
      await renderPanel();

      show({ open: OTHER.key });
      await act(async () => {});

      expect(screen.getByRole("heading", { name: "code" })).toBeInTheDocument();
      expect(
        screen.getByRole("menuitem", { name: "Fixar no dock" }),
      ).toBeInTheDocument();
      expect(card()).toHaveAttribute("data-panel", "open");
      expect(readyCalls().length).toBeGreaterThan(0);
    });

    it("closes when the program is not open any more, once", async () => {
      openSnapshot = { apps: [], others: [OTHER] };
      await renderPanel();
      show({ open: OTHER.key });
      expect(hides()).toBe(0);

      emit("open-windows-changed", { apps: [], others: [] });

      expect(hides()).toBe(1);
      emit("open-windows-changed", { apps: [], others: [] });
      expect(hides()).toBe(1);
    });

    it("does not close while the open programs are not read", async () => {
      openSnapshot = undefined;
      await renderPanel();

      show({ open: OTHER.key });

      expect(hides()).toBe(0);
    });

    it("shows nothing for a program that is not open", async () => {
      await renderPanel();

      show({ open: OTHER.key });

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(hides()).toBe(1);
    });
  });
});
