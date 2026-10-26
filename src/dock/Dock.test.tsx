import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import { dockSize } from "../core/appearance";
import { forgetIcons, type AppsSnapshot } from "../core/apps";
import type { WidgetInstance, WidgetsSnapshot } from "../core/widgets";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
import { ptBR } from "../i18n/pt-BR";
import Dock from "./Dock";
import { placeDock } from "./layout";
import { hoverConfigOf } from "./magnification";

type Handler = (event: { payload: unknown }) => void;

const { invoke, listen, handlers, unlisten } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  const unlisten = vi.fn();
  return {
    invoke: vi.fn(),
    unlisten,
    handlers,
    listen: vi.fn((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(unlisten);
    }),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const text = ptBR.dock;

const MEDIUM: Settings = {
  version: 1,
  theme: "system",
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
    hover: "magnify",
    // The taskbar stays: no clock at the end, so the numbers below are the
    // bar of the items alone.
    taskbar: "visible",
  },
};

const withDock = (dock: Partial<Settings["dock"]>): Settings => ({
  ...MEDIUM,
  dock: { ...MEDIUM.dock, ...dock },
});

/** Label pill widths the tests lay out (Configurações, Sobre). */
const LABEL_WIDTHS: Record<string, number> = {
  [text.items.settings]: 110,
  [text.items.about]: 60,
};

const rect = (x: number, y: number, width: number, height: number) => ({
  x,
  y,
  width,
  height,
});

// Numbers from layout.test.ts: medium dock, bottom edge, in a 189 × 123
// window (the size it asks for).
const REST_BAR = rect(36, 61, 117, 62);
const MEDIUM_WINDOW = { width: 189, height: 123 };
// With the taskbar hidden, which is the default: the tray is a third item
// (7 + 48 = 55 px) and the clock and the date follow a separator (7 + 6.6 +
// 7 + 76 = 96.6 px); the widest label reaches 158.3 px from the middle.
const STATUS_BAR = rect(-39.8, 61, 268.6, 62);
const STATUS_WINDOW = { width: 333, height: 102 };
const SETTINGS_AT_REST = rect(43, 68, 48, 48);
const ABOUT_AT_REST = rect(98, 68, 48, 48);

function expectClose(actual: unknown, expected: unknown, path = "") {
  if (typeof expected === "number") {
    expect(actual as number, path).toBeCloseTo(expected, 6);
    return;
  }
  if (Array.isArray(expected)) {
    expect(actual, path).toHaveLength(expected.length);
    expected.forEach((value, index) =>
      expectClose((actual as unknown[])[index], value, `${path}[${index}]`),
    );
    return;
  }
  const object = expected as Record<string, unknown>;
  expect(Object.keys(actual as object).sort(), path).toEqual(
    Object.keys(object).sort(),
  );
  for (const key of Object.keys(object)) {
    expectClose(
      (actual as Record<string, unknown>)[key],
      object[key],
      `${path}.${key}`,
    );
  }
}

function layoutCalls() {
  return invoke.mock.calls.filter(([command]) => command === "set_dock_layout");
}

function lastLayout() {
  const calls = layoutCalls();
  return (calls[calls.length - 1]?.[1] as { layout: unknown } | undefined)
    ?.layout;
}

function panelCalls() {
  return invoke.mock.calls.filter(([command]) => command === "toggle_panel");
}

function emit(event: string, payload: unknown) {
  const handler = handlers.get(event);
  expect(handler, event).toBeDefined();
  act(() => handler?.({ payload }));
}

/** Renders the dock and lets the initial settings read answer. */
async function renderDock() {
  const view = render(<Dock />);
  await act(async () => {});
  return view;
}

const settings = () =>
  screen.getByRole("button", { name: text.items.settings });
const about = () => screen.getByRole("button", { name: text.items.about });
const bar = () => screen.getByRole("navigation", { name: text.label });
const shownTip = () =>
  document.querySelector<HTMLElement>(".dock-tip[data-visible]");

/** Focuses `element` as the keyboard (or, with `visible` false, a click)
 * would: jsdom's `:focus-visible` does not follow the input modality. */
function focus(element: HTMLElement, visible = true) {
  const matches = element.matches.bind(element);
  vi.spyOn(element, "matches").mockImplementation((selector: string) =>
    selector === ":focus-visible" ? visible : matches(selector),
  );
  act(() => element.focus());
}

function stubReducedMotion(initial: boolean) {
  const listeners = new Set<() => void>();
  const media = {
    matches: initial,
    addEventListener: vi.fn((_: string, listener: () => void) =>
      listeners.add(listener),
    ),
    removeEventListener: vi.fn((_: string, listener: () => void) =>
      listeners.delete(listener),
    ),
  };
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) =>
      query === "(prefers-reduced-motion: reduce)" ? media : { matches: false },
    ),
  );
  return {
    media,
    set(matches: boolean) {
      media.matches = matches;
      act(() => listeners.forEach((listener) => listener()));
    },
  };
}

/** No widgets: the dock of Fase 1, Configurações and Sobre. */
const NO_WIDGETS: WidgetsSnapshot = { storage: { state: "ok" }, widgets: [] };

/** A clock that reads the same anywhere: 24 h in UTC, digital. */
const CLOCK: WidgetInstance<"clock"> = {
  ...WIDGET_FIXTURES["clock-default"],
  config: {
    timeZone: "UTC",
    hourCycle: "h23",
    showSeconds: false,
    iconStyle: "digital",
  },
};
const PROGRESS = WIDGET_FIXTURES["time-progress-sample"];
const NOTE = WIDGET_FIXTURES["note-default"];

const withWidgets = (
  widgets: WidgetInstance[],
  storage: WidgetsSnapshot["storage"] = { state: "ok" },
): WidgetsSnapshot => ({ storage, widgets });

/** No programs pinned. */
const NO_APPS: AppsSnapshot = { storage: { state: "ok" }, apps: [] };

const CALCULATOR = {
  id: "0199a4c0-0000-7000-8000-000000000001",
  appId: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
  name: "Calculadora",
};
const NOTEPAD = {
  id: "0199a4c0-0000-7000-8000-000000000002",
  appId: "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\notepad.exe",
  name: "Bloco de notas",
};
const ICON = "data:image/png;base64,iVBORw0KGgo=";

describe("Dock", () => {
  let current: Settings;
  let snapshot: WidgetsSnapshot;
  let pinned: AppsSnapshot;
  let icon: string | null;

  beforeEach(() => {
    handlers.clear();
    unlisten.mockClear();
    listen.mockClear();
    current = MEDIUM;
    snapshot = NO_WIDGETS;
    pinned = NO_APPS;
    icon = ICON;
    forgetIcons();
    invoke.mockReset();
    invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "get_settings"
          ? current
          : command === "list_widgets"
            ? snapshot
            : command === "list_apps"
              ? pinned
              : command === "app_icon"
                ? icon
                : undefined,
      ),
    );
    vi.stubGlobal("innerWidth", MEDIUM_WINDOW.width);
    vi.stubGlobal("innerHeight", MEDIUM_WINDOW.height);
    const original = HTMLElement.prototype.getBoundingClientRect;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
      function (this: HTMLElement) {
        if (!this.classList.contains("dock-tip")) return original.call(this);
        const name = this.textContent ?? "";
        // Any other name: 8 px a character.
        const width = LABEL_WIDTHS[name] ?? name.length * 8;
        return { ...rect(0, 0, width, 24) } as DOMRect;
      },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
    const root = document.documentElement;
    for (const key of ["mode", "opacity", "size", "edge"]) {
      delete root.dataset[key];
    }
  });

  describe("items", () => {
    it("names each icon-only item in PT-BR", async () => {
      await renderDock();

      // Literal on purpose: catches a broken dictionary, not just a renamed
      // key.
      expect(
        screen.getByRole("navigation", { name: "Dock" }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "Configurações" }),
      ).toHaveAttribute("aria-label", "Configurações");
      expect(screen.getByRole("button", { name: "Sobre" })).toHaveAttribute(
        "aria-label",
        "Sobre",
      );
      expect(screen.getAllByRole("button")).toHaveLength(2);
    });

    it("glazes each tile with its static palette class and a line glyph", async () => {
      await renderDock();

      for (const [button, glaze] of [
        [settings(), "t-settings"],
        [about(), "t-about"],
      ] as const) {
        const tile = button.querySelector(".dock-tile");
        expect(tile).toHaveClass(glaze);
        expect(tile?.querySelector("svg.dock-glyph")).toHaveAttribute(
          "aria-hidden",
          "true",
        );
        expect(tile).not.toHaveAttribute("style");
      }
    });

    it("sets only CSS custom properties on the items", async () => {
      await renderDock();
      fireEvent.pointerOver(settings());

      for (const element of document.querySelectorAll("[style]")) {
        const style = (element as HTMLElement).style;
        const names = Array.from({ length: style.length }, (_, i) =>
          style.item(i),
        );
        expect(names.every((name) => name.startsWith("--"))).toBe(true);
      }
    });

    it("has no separator: Configurações and Sobre are one group", async () => {
      await renderDock();

      expect(document.querySelector(".dock-separator")).toBeNull();
    });
  });

  describe("layout", () => {
    it("reports the bar and the window it needs once on mount", async () => {
      // The settings the dock starts with, so nothing changes when they load.
      current = withDock({
        hover: DEFAULT_LOOK.hover,
        taskbar: DEFAULT_LOOK.taskbar,
      });

      await renderDock();

      expect(layoutCalls()).toHaveLength(1);
      expectClose(lastLayout(), {
        bar: STATUS_BAR,
        hold: [],
        window: STATUS_WINDOW,
      });
    });

    it("sizes the window for the hover effect: none, scale or magnify", async () => {
      const windowOf = async (hover: Settings["dock"]["hover"]) => {
        invoke.mockClear();
        current = withDock({ hover, hoverScale: 130 });
        const view = await renderDock();
        view.unmount();
        return (lastLayout() as { window: { width: number; height: number } })
          .window;
      };

      // Only the widest label sets the width at rest: 110 and 60 px labels
      // over 48 px items give 181. The height is the bar's padding (7), the
      // tile at its peak (48; 62.4 at 130 %), the gap to the label (10), the
      // label (24) and the room (8): 97 and 112 (spec 0005 §3.3).
      expect(await windowOf("none")).toEqual({ width: 181, height: 97 });
      expect(await windowOf("scale")).toEqual({ width: 181, height: 112 });
      expect(await windowOf("magnify")).toEqual(MEDIUM_WINDOW);
    });

    it("applies the size and edge to the page", async () => {
      current = withDock({ iconSize: 64, edge: "top" });

      await renderDock();

      expect(document.documentElement.dataset.size).toBe("l");
      expect(document.documentElement.dataset.edge).toBe("top");
    });

    it("reports again when the size preset changes", async () => {
      await renderDock();
      const reported = layoutCalls().length;

      emit("settings-changed", withDock({ iconSize: 64 }));

      expect(layoutCalls()).toHaveLength(reported + 1);
      // Large: 2 × 9 + 9 + 2 × 64 = 155 wide, 64 + 2 × 9 = 82 tall, in the
      // window as it still is.
      expectClose(lastLayout(), {
        bar: rect(17, 41, 155, 82),
        hold: [],
        window: { width: 209, height: 147 },
      });
      expect(document.documentElement.dataset.size).toBe("l");
    });

    it("anchors the bar to the top when the edge changes", async () => {
      await renderDock();

      emit("settings-changed", withDock({ edge: "top" }));

      expectClose(lastLayout(), {
        bar: rect(36, 0, 117, 62),
        hold: [],
        window: MEDIUM_WINDOW,
      });
    });

    it("reports again when the icon size changes within the same preset", async () => {
      // Rust closed the open panel for the move and reopens it only on the
      // next layout: one must come even though the dock draws the same.
      await renderDock();
      const reported = layoutCalls().length;

      emit("settings-changed", withDock({ iconSize: 50 }));

      expect(layoutCalls()).toHaveLength(reported + 1);
      expect(lastLayout()).toEqual(layoutCalls()[reported - 1]?.[1]?.layout);

      emit("settings-changed", withDock({ iconSize: 50, edge: "top" }));
      emit("settings-changed", withDock({ iconSize: 50, edge: "top" }));

      expect(layoutCalls()).toHaveLength(reported + 2);
    });

    it("does not report an unchanged layout again", async () => {
      await renderDock();
      const reported = layoutCalls().length;

      emit("settings-changed", { ...withDock({}), theme: "dark" });
      emit("settings-changed", withDock({ opacity: "solid" }));

      expect(layoutCalls()).toHaveLength(reported);
    });

    it("reports the defaults when the settings can't be read", async () => {
      invoke.mockImplementation((command: string) =>
        command === "get_settings" || command === "list_widgets"
          ? Promise.reject(new Error("no settings"))
          : Promise.resolve(undefined),
      );

      await renderDock();

      expect(layoutCalls()).toHaveLength(1);
      expectClose(lastLayout(), {
        bar: STATUS_BAR,
        hold: [],
        window: STATUS_WINDOW,
      });
    });

    it("places the bar again when the window is resized", async () => {
      let notify = () => {};
      const observed: Element[] = [];
      vi.stubGlobal(
        "ResizeObserver",
        class {
          constructor(callback: () => void) {
            notify = callback;
          }
          observe(element: Element) {
            observed.push(element);
          }
          disconnect() {}
        },
      );
      await renderDock();
      vi.stubGlobal("innerWidth", 240);
      vi.stubGlobal("innerHeight", 104);

      act(() => notify());

      expect(observed).toContain(document.documentElement);
      expectClose(lastLayout(), {
        bar: rect(61.5, 42, 117, 62),
        hold: [],
        window: MEDIUM_WINDOW,
      });
    });
  });

  describe("clock and date", () => {
    const status = () =>
      screen.getByRole("button", { name: text.status.calendar });

    afterEach(() => vi.useRealTimers());

    it("stands in for the taskbar's at the end of the bar, after a separator", async () => {
      current = withDock({ taskbar: "hidden" });

      await renderDock();

      const children = Array.from(bar().children);
      expect(children[children.length - 1]).toBe(status());
      expect(children[children.length - 2]).toHaveClass("dock-separator");
    });

    it("shows the time of the minute timer and the date", async () => {
      current = withDock({ taskbar: "autohide" });
      vi.useFakeTimers({ toFake: ["Date"] });
      vi.setSystemTime(new Date(2026, 9, 22, 18, 5));

      await renderDock();

      expect(status().querySelector(".dock-status-time")).toHaveTextContent(
        "18:05",
      );
      expect(
        status().querySelector(".dock-status-date"),
      ).not.toBeEmptyDOMElement();
    });

    it("is not drawn while the taskbar stays: it has its own clock", async () => {
      current = withDock({ taskbar: "visible" });

      await renderDock();

      expect(
        screen.queryByRole("button", { name: text.status.calendar }),
      ).not.toBeInTheDocument();
    });

    it("opens the Windows calendar on a click", async () => {
      current = withDock({ taskbar: "hidden" });
      await renderDock();

      fireEvent.click(status());

      expect(invoke).toHaveBeenCalledWith("open_flyout", {
        flyout: "calendar",
      });
    });

    it("is not an item: no magnification, no label, no arrow-key stop", async () => {
      current = withDock({ taskbar: "hidden" });
      await renderDock();

      fireEvent.pointerOver(status());

      expect(document.querySelector("[data-magnify]")).toBeNull();
      expect(shownTip()).toBeNull();
      expect(document.querySelectorAll(".dock-item")).toHaveLength(3);
      // The arrows move among the items only: the tray is the last of them.
      settings().focus();
      fireEvent.keyDown(settings(), { key: "End" });
      const tray = screen.getByRole("button", { name: text.items.tray });
      expect(tray).toHaveFocus();
      fireEvent.keyDown(tray, { key: "ArrowRight" });
      expect(tray).toHaveFocus();
    });

    it("makes the bar wider by a separator and its own width, and no item moves", async () => {
      current = withDock({ taskbar: "visible" });
      const without = await renderDock();
      const barWithout = (lastLayout() as { bar: { width: number } }).bar.width;
      without.unmount();
      invoke.mockClear();

      // Hiding on its own: the clock, and no tray.
      current = withDock({ taskbar: "autohide" });
      await renderDock();
      const barWith = (lastLayout() as { bar: { width: number } }).bar.width;

      // A separator (1 + 0.8 × 7) and the clock (76) between two gaps of 7.
      expect(barWith - barWithout).toBeCloseTo(7 + 6.6 + 7 + 76 - 0, 6);
    });
  });

  describe("tray item", () => {
    const tray = () => screen.queryByRole("button", { name: text.items.tray });

    it("is the last item while the taskbar is hidden altogether", async () => {
      current = withDock({ taskbar: "hidden" });

      await renderDock();

      const items = Array.from(document.querySelectorAll(".dock-item"));
      expect(items[items.length - 1]).toBe(tray());
    });

    it("is not an item while the taskbar hides on its own or stays: it has its own tray", async () => {
      for (const taskbar of ["autohide", "visible"] as const) {
        current = withDock({ taskbar });
        const view = await renderDock();

        expect(tray(), taskbar).toBeNull();
        view.unmount();
      }
    });

    it("opens the tray panel anchored where the item rests", async () => {
      current = withDock({ taskbar: "hidden" });
      await renderDock();

      fireEvent.click(tray() as HTMLElement);

      expect(invoke).toHaveBeenCalledWith(
        "toggle_panel",
        expect.objectContaining({ panelId: "tray" }),
      );
    });
  });

  describe("magnification", () => {
    it("magnifies the hovered item, pushes the neighbour and holds both", async () => {
      await renderDock();

      fireEvent.pointerOver(settings());

      expect(settings()).toHaveAttribute("data-magnify", "hovered");
      expect(about()).toHaveAttribute("data-magnify", "neighbor");
      expect(settings().style.getPropertyValue("--s")).toBe("1.4");
      expect(settings().style.getPropertyValue("--lift")).toBe("6px");
      expect(about().style.getPropertyValue("--s")).toBe("1.15");
      expectClose(lastLayout(), {
        bar: rect(22.8, 61, 143.4, 62),
        hold: [rect(29.8, 42.8, 67.2, 67.2), rect(104, 60.8, 55.2, 55.2)],
        window: MEDIUM_WINDOW,
      });
    });

    it("finds the hovered item from a pointer over its glyph", async () => {
      await renderDock();

      fireEvent.pointerOver(about().querySelector("svg") as Element);

      expect(about()).toHaveAttribute("data-magnify", "hovered");
    });

    it("does not report again while the pointer stays on the same item", async () => {
      await renderDock();
      fireEvent.pointerOver(settings());
      const calls = layoutCalls().length;

      fireEvent.pointerMove(settings());
      fireEvent.pointerOver(settings().querySelector(".dock-tile") as Element);

      expect(layoutCalls()).toHaveLength(calls);
    });

    it("rests over the bar's padding, on leave and on release", async () => {
      await renderDock();

      fireEvent.pointerOver(settings());
      fireEvent.pointerOver(bar());
      expect(settings()).not.toHaveAttribute("data-magnify");
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: MEDIUM_WINDOW,
      });

      fireEvent.pointerOver(about());
      fireEvent.pointerLeave(bar());
      expect(about()).not.toHaveAttribute("data-magnify");

      fireEvent.pointerOver(about());
      emit("dock-released", null);
      expect(about()).not.toHaveAttribute("data-magnify");
      expect(about().style.getPropertyValue("--s")).toBe("1");
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: MEDIUM_WINDOW,
      });
    });
  });

  describe("label pill", () => {
    it("shows the hovered item's name above its final tile", async () => {
      await renderDock();

      fireEvent.pointerOver(settings());

      const tip = shownTip();
      expect(tip).toHaveTextContent("Configurações");
      expect(tip).toHaveAttribute("aria-hidden", "true");
      expect(
        parseFloat(tip?.style.getPropertyValue("--tip-x") ?? ""),
      ).toBeCloseTo(63.4, 6);
      expect(
        parseFloat(tip?.style.getPropertyValue("--tip-y") ?? ""),
      ).toBeCloseTo(32.8, 6);
      expect(document.querySelectorAll(".dock-tip[data-visible]")).toHaveLength(
        1,
      );
    });

    it("shows it below the tile on the top edge", async () => {
      current = withDock({ edge: "top" });
      await renderDock();

      fireEvent.pointerOver(settings());

      expect(
        parseFloat(shownTip()?.style.getPropertyValue("--tip-y") ?? ""),
      ).toBeCloseTo(90.2, 6);
    });

    it("hides it when the pointer leaves", async () => {
      await renderDock();
      fireEvent.pointerOver(about());
      expect(shownTip()).toHaveTextContent("Sobre");

      fireEvent.pointerLeave(bar());

      expect(shownTip()).toBeNull();
    });
  });

  describe("open panel", () => {
    it("marks the open item as expanded", async () => {
      await renderDock();
      expect(settings()).toHaveAttribute("aria-expanded", "false");

      emit("panel-changed", { panelId: "settings" });

      expect(settings()).toHaveAttribute("aria-expanded", "true");
      expect(about()).toHaveAttribute("aria-expanded", "false");

      emit("panel-changed", { panelId: null });

      expect(settings()).toHaveAttribute("aria-expanded", "false");
    });

    it("never points to the panel with aria-controls", async () => {
      // The panel lives in another window: no IDREF here can reach it.
      await renderDock();

      for (const panelId of [null, "settings", "about"]) {
        emit("panel-changed", { panelId });
        expect(document.querySelector("[aria-controls]")).toBeNull();
      }
    });

    it("freezes the magnification while a panel is open", async () => {
      await renderDock();
      fireEvent.pointerOver(settings());

      emit("panel-changed", { panelId: "settings" });

      expect(settings()).not.toHaveAttribute("data-magnify");
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: MEDIUM_WINDOW,
      });

      fireEvent.pointerOver(about());
      expect(about()).not.toHaveAttribute("data-magnify");
      expect(shownTip()).toBeNull();

      emit("panel-changed", { panelId: null });
      fireEvent.pointerOver(about());
      expect(about()).toHaveAttribute("data-magnify", "hovered");
    });

    it("hides the label of the item whose panel is open", async () => {
      await renderDock();
      emit("panel-changed", { panelId: "settings" });

      focus(settings());
      expect(shownTip()).toBeNull();

      focus(about());
      expect(shownTip()).toHaveTextContent("Sobre");
      expect(about()).not.toHaveAttribute("data-magnify");
    });

    it("opens the clicked item's panel anchored where the item rests", async () => {
      await renderDock();
      fireEvent.pointerOver(settings());

      fireEvent.click(settings());
      fireEvent.click(about());

      expect(panelCalls()).toEqual([
        ["toggle_panel", { panelId: "settings", anchor: SETTINGS_AT_REST }],
        ["toggle_panel", { panelId: "about", anchor: ABOUT_AT_REST }],
      ]);
    });

    it("opens a panel asked for from outside, unless it is open", async () => {
      await renderDock();

      emit("dock-request-panel", { panelId: "settings" });
      expect(panelCalls()).toEqual([
        ["toggle_panel", { panelId: "settings", anchor: SETTINGS_AT_REST }],
      ]);

      emit("panel-changed", { panelId: "settings" });
      emit("dock-request-panel", { panelId: "settings" });
      expect(panelCalls()).toHaveLength(1);

      emit("dock-request-panel", { panelId: "about" });
      expect(panelCalls()[1]).toEqual([
        "toggle_panel",
        { panelId: "about", anchor: ABOUT_AT_REST },
      ]);
    });
  });

  describe("keyboard", () => {
    it("keeps a single tab stop that the arrows, Home and End move", async () => {
      await renderDock();
      expect(settings()).toHaveAttribute("tabindex", "0");
      expect(about()).toHaveAttribute("tabindex", "-1");
      act(() => settings().focus());

      fireEvent.keyDown(settings(), { key: "ArrowRight" });
      expect(about()).toHaveFocus();
      expect(about()).toHaveAttribute("tabindex", "0");
      expect(settings()).toHaveAttribute("tabindex", "-1");

      fireEvent.keyDown(about(), { key: "ArrowRight" });
      expect(about()).toHaveFocus();

      fireEvent.keyDown(about(), { key: "Home" });
      expect(settings()).toHaveFocus();

      fireEvent.keyDown(settings(), { key: "End" });
      expect(about()).toHaveFocus();

      fireEvent.keyDown(about(), { key: "ArrowLeft" });
      expect(settings()).toHaveFocus();

      fireEvent.keyDown(settings(), { key: "ArrowLeft" });
      expect(settings()).toHaveFocus();
    });

    it("opens the panel with Enter and Space, once per press", async () => {
      await renderDock();

      fireEvent.keyDown(settings(), { key: "Enter" });
      fireEvent.keyDown(about(), { key: " " });
      fireEvent.keyDown(about(), { key: " ", repeat: true });

      expect(panelCalls()).toEqual([
        ["toggle_panel", { panelId: "settings", anchor: SETTINGS_AT_REST }],
        ["toggle_panel", { panelId: "about", anchor: ABOUT_AT_REST }],
      ]);
    });

    it("magnifies and labels the item focused by keyboard", async () => {
      await renderDock();

      focus(about());

      expect(about()).toHaveAttribute("data-magnify", "hovered");
      expect(shownTip()).toHaveTextContent("Sobre");

      act(() => about().blur());

      expect(about()).not.toHaveAttribute("data-magnify");
      expect(shownTip()).toBeNull();
    });

    it("does not magnify an item focused by a click", async () => {
      await renderDock();

      focus(about(), false);

      expect(about()).not.toHaveAttribute("data-magnify");
      expect(shownTip()).toBeNull();
      expect(about()).not.toHaveAttribute("data-magnify");
      expect(about()).toHaveAttribute("tabindex", "0");
    });
  });

  describe("reduced motion", () => {
    it("never magnifies, holds only the bar and needs a smaller window", async () => {
      stubReducedMotion(true);
      current = withDock({ taskbar: DEFAULT_LOOK.taskbar });
      await renderDock();

      fireEvent.pointerOver(settings());

      expect(settings()).not.toHaveAttribute("data-magnify");
      expect(shownTip()).toHaveTextContent("Configurações");
      expect(layoutCalls()).toHaveLength(1);
      expectClose(lastLayout(), {
        bar: STATUS_BAR,
        hold: [],
        window: { width: 333, height: 97 },
      });
    });

    it("follows the Windows setting as it changes", async () => {
      const motion = stubReducedMotion(false);
      await renderDock();
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: MEDIUM_WINDOW,
      });

      motion.set(true);

      expectClose(lastLayout(), {
        bar: rect(36, 61, 117, 62),
        hold: [],
        window: { width: 181, height: 97 },
      });
    });
  });

  describe("widgets", () => {
    const names = () =>
      screen.getAllByRole("button").map((b) => b.getAttribute("aria-label"));

    it("puts the widgets first, in order, then a separator and the system items", async () => {
      snapshot = withWidgets([NOTE, CLOCK, PROGRESS]);
      await renderDock();

      expect(names()).toEqual([
        "Nota",
        "Relógio",
        "Trabalho",
        "Configurações",
        "Sobre",
      ]);
      const children = Array.from(bar().children);
      const separator = document.querySelector(".dock-separator");
      expect(separator).toHaveAttribute("aria-hidden", "true");
      expect(children.indexOf(separator as Element)).toBe(3);
      expect(document.querySelectorAll(".dock-separator")).toHaveLength(1);
    });

    it("shows only the system items while the storage is unavailable", async () => {
      snapshot = withWidgets([CLOCK], {
        state: "unavailable",
        reason: "newerVersion",
      });
      await renderDock();
      expect(names()).toEqual(["Configurações (com aviso)", "Sobre"]);
      expect(document.querySelector(".dock-separator")).toBeNull();
    });

    it("glazes each widget by its kind and draws its live face", async () => {
      vi.useFakeTimers({ toFake: ["Date"] });
      vi.setSystemTime(Date.UTC(2026, 9, 16, 9, 41, 30));
      try {
        snapshot = withWidgets([CLOCK, PROGRESS, NOTE]);
        await renderDock();
      } finally {
        vi.useRealTimers();
      }

      const tile = (name: string) =>
        screen.getByRole("button", { name }).querySelector(".dock-tile");
      expect(tile("Relógio")).toHaveClass("t-clock");
      expect(tile("Relógio")).toHaveTextContent("09:41");
      expect(tile("Trabalho")).toHaveClass("t-progress");
      expect(tile("Nota")).toHaveClass("t-notes");
      // The name, never the time.
      expect(screen.getByRole("button", { name: "Relógio" })).toHaveAttribute(
        "aria-label",
        "Relógio",
      );
      expect(
        screen.getByText("Relógio", { selector: ".dock-tip" }),
      ).toBeInTheDocument();
    });

    it("moves every live face at the minute turn, from one timer", async () => {
      vi.useFakeTimers({ toFake: ["Date", "setTimeout", "clearTimeout"] });
      try {
        vi.setSystemTime(Date.UTC(2026, 9, 16, 9, 41, 50));
        snapshot = withWidgets([
          CLOCK,
          { ...CLOCK, id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6099", label: "B" },
        ]);
        await renderDock();
        const faces = () =>
          Array.from(document.querySelectorAll(".clock-tile-time")).map(
            (face) => face.textContent,
          );
        expect(faces()).toEqual(["09:41", "09:41"]);
        expect(vi.getTimerCount()).toBe(1);

        await act(() => vi.advanceTimersByTimeAsync(10_000));

        expect(faces()).toEqual(["09:42", "09:42"]);
        expect(vi.getTimerCount()).toBe(1);
      } finally {
        vi.useRealTimers();
      }
    });

    it("badges Configurações while the storage was recovered or is unavailable", async () => {
      const cases: [WidgetsSnapshot["storage"], boolean][] = [
        [{ state: "ok" }, false],
        [{ state: "recovered" }, true],
        [{ state: "unavailable", reason: "openFailed" }, true],
      ];
      for (const [storage, badged] of cases) {
        snapshot = withWidgets([], storage);
        const view = await renderDock();
        const badges = document.querySelectorAll(".dock-badge");
        expect(badges, storage.state).toHaveLength(badged ? 1 : 0);
        // The badge is hidden; the item's name says it.
        const item = screen.getByRole("button", {
          name: badged ? "Configurações (com aviso)" : "Configurações",
        });
        if (badged) {
          expect(item.contains(badges[0] as Node)).toBe(true);
          expect(badges[0]).toHaveAttribute("aria-hidden", "true");
        }
        // The pill keeps the plain name.
        expect(
          [...document.querySelectorAll(".dock-tip")].map(
            (tip) => tip.textContent,
          ),
        ).toContain("Configurações");
        view.unmount();
      }
    });

    it("follows widgets-changed: items, names and the badge", async () => {
      snapshot = withWidgets([CLOCK]);
      await renderDock();

      emit(
        "widgets-changed",
        withWidgets([{ ...CLOCK, label: "Lisboa" }, NOTE], {
          state: "recovered",
        }),
      );

      expect(names()).toEqual([
        "Lisboa",
        "Nota",
        "Configurações (com aviso)",
        "Sobre",
      ]);
      expect(document.querySelector(".dock-badge")).not.toBeNull();
    });

    it("measures the pills again when a name changes, and sends the wider window", async () => {
      snapshot = withWidgets([CLOCK]);
      await renderDock();
      const before = layoutCalls().length;
      const widthOf = (layout: unknown) =>
        (layout as { window: { width: number } }).window.width;
      const narrow = widthOf(lastLayout());

      emit(
        "widgets-changed",
        withWidgets([{ ...CLOCK, label: "Um rótulo bem comprido aqui" }]),
      );
      await act(async () => {});

      const sent = layoutCalls()
        .slice(before)
        .map(([, args]) => widthOf((args as { layout: unknown }).layout));
      // One new layout, from the new widths: never the old widths with the
      // new names.
      expect(sent).toHaveLength(1);
      expect(sent[0]).toBeGreaterThan(narrow);
    });

    it("opens a widget's panel by its PanelId, anchored where it rests", async () => {
      snapshot = withWidgets([CLOCK]);
      await renderDock();
      const clock = screen.getByRole("button", { name: "Relógio" });

      fireEvent.click(clock);
      // Not on the dock: nothing to anchor to.
      emit("dock-request-panel", { panelId: { widget: NOTE.id } });

      const calls = panelCalls();
      expect(calls).toHaveLength(1);
      const args = calls[0]?.[1] as {
        panelId: unknown;
        anchor: { width: number; height: number };
      };
      expect(args.panelId).toEqual({ widget: CLOCK.id });
      expect(args.anchor).toMatchObject({ width: 48, height: 48 });

      emit("panel-changed", { panelId: { widget: CLOCK.id } });
      expect(clock).toHaveAttribute("aria-expanded", "true");
      expect(settings()).toHaveAttribute("aria-expanded", "false");
      // Open: asking for it again does nothing.
      emit("dock-request-panel", { panelId: { widget: CLOCK.id } });
      expect(panelCalls()).toHaveLength(1);
    });

    it("keeps the arrows, Home and End across every item", async () => {
      snapshot = withWidgets([CLOCK, NOTE]);
      await renderDock();
      const clock = screen.getByRole("button", { name: "Relógio" });
      const note = screen.getByRole("button", { name: "Nota" });
      expect(clock).toHaveAttribute("tabindex", "0");
      act(() => clock.focus());

      fireEvent.keyDown(clock, { key: "ArrowRight" });
      expect(note).toHaveFocus();
      fireEvent.keyDown(note, { key: "ArrowRight" });
      expect(settings()).toHaveFocus();
      fireEvent.keyDown(settings(), { key: "End" });
      expect(about()).toHaveFocus();
      fireEvent.keyDown(about(), { key: "Home" });
      expect(clock).toHaveFocus();
    });

    it("keeps the tab stop on its item when the list changes", async () => {
      snapshot = withWidgets([CLOCK, NOTE]);
      await renderDock();
      act(() => screen.getByRole("button", { name: "Nota" }).focus());

      emit("widgets-changed", withWidgets([PROGRESS, CLOCK, NOTE]));
      expect(screen.getByRole("button", { name: "Nota" })).toHaveAttribute(
        "tabindex",
        "0",
      );

      // Its item gone: the first item takes the stop.
      emit("widgets-changed", withWidgets([PROGRESS]));
      expect(screen.getByRole("button", { name: "Trabalho" })).toHaveAttribute(
        "tabindex",
        "0",
      );
    });

    it("drops a magnified index the list no longer has", async () => {
      snapshot = withWidgets([CLOCK, NOTE]);
      await renderDock();
      fireEvent.pointerOver(about());
      expect(shownTip()).toHaveTextContent("Sobre");

      emit("widgets-changed", NO_WIDGETS);

      // Index 3 is gone: nothing is magnified and no pill shows.
      expect(names()).toEqual(["Configurações", "Sobre"]);
      expect(shownTip()).toBeNull();
      expect(document.querySelector("[data-magnify]")).toBeNull();
    });
  });

  describe("pinned programs", () => {
    const withApps = (...apps: AppsSnapshot["apps"]): AppsSnapshot => ({
      storage: { state: "ok" },
      apps,
    });
    const program = (name: string) => screen.getByRole("button", { name });

    it("puts the programs first, in order, then a separator and the system items", async () => {
      pinned = withApps(CALCULATOR, NOTEPAD);
      await renderDock();

      const names = screen
        .getAllByRole("button")
        .map((button) => button.getAttribute("aria-label"));
      expect(names).toEqual([
        "Calculadora",
        "Bloco de notas",
        text.items.settings,
        text.items.about,
      ]);
      expect(document.querySelectorAll(".dock-separator")).toHaveLength(1);
    });

    it("separates the programs, the widgets and the system items", async () => {
      pinned = withApps(CALCULATOR);
      snapshot = withWidgets([CLOCK]);
      await renderDock();

      const bar = document.querySelector(".dock-bar");
      const kinds = Array.from(bar?.children ?? []).map((child) =>
        child.classList.contains("dock-separator") ? "|" : "item",
      );
      expect(kinds.join(" ")).toBe("item | item | item item");
    });

    it("draws the icon on the neutral plate", async () => {
      pinned = withApps(CALCULATOR);
      await renderDock();

      const tile = program("Calculadora").querySelector(".dock-tile");
      expect(tile).toHaveClass("t-app");
      expect(tile?.querySelector("img")).toHaveAttribute("src", ICON);
      expect(invoke).toHaveBeenCalledWith("app_icon", {
        appId: CALCULATOR.appId,
        size: 96,
      });
    });

    it("shows the first letter of the name while the icon is not there, and when Windows has none", async () => {
      pinned = withApps(CALCULATOR);
      icon = null;
      await renderDock();

      const tile = program("Calculadora").querySelector(".dock-tile");
      expect(tile?.querySelector("img")).toBeNull();
      expect(tile?.textContent).toBe("C");
    });

    it("opens the program on a click, by the id of the pinned item, and no panel", async () => {
      pinned = withApps(CALCULATOR, NOTEPAD);
      await renderDock();

      fireEvent.click(program("Bloco de notas"));

      expect(invoke).toHaveBeenCalledWith("launch_app", { id: NOTEPAD.id });
      expect(panelCalls()).toHaveLength(0);
    });

    it("animates the icon of a program that is opening, and only its own", async () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      pinned = withApps(CALCULATOR, NOTEPAD);
      await renderDock();
      expect(program("Calculadora")).not.toHaveAttribute("data-opening");

      fireEvent.click(program("Calculadora"));

      expect(program("Calculadora")).toHaveAttribute("data-opening");
      expect(program("Bloco de notas")).not.toHaveAttribute("data-opening");
      expect(settings()).not.toHaveAttribute("data-opening");
      await act(() => vi.advanceTimersByTimeAsync(1600));
      expect(program("Calculadora")).not.toHaveAttribute("data-opening");
    });

    it("stops the animation at once when the program could not be opened", async () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      pinned = withApps(CALCULATOR);
      await renderDock();
      invoke.mockImplementation((command: string) =>
        command === "launch_app"
          ? Promise.reject({ code: "launchFailed" })
          : Promise.resolve(undefined),
      );

      fireEvent.click(program("Calculadora"));
      await act(async () => {});

      expect(program("Calculadora")).not.toHaveAttribute("data-opening");
    });

    it("leaves no timer behind when the dock goes", async () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      pinned = withApps(CALCULATOR);
      const { unmount } = await renderDock();
      fireEvent.click(program("Calculadora"));

      unmount();

      expect(vi.getTimerCount()).toBe(0);
    });

    describe("dragging to reorder", () => {
      const THIRD = {
        id: "0199a4c0-0000-7000-8000-000000000003",
        appId: "Third!App",
        name: "Terceiro",
      };

      /** Where the programs rest, as the dock lays them out. */
      const rest = (count: number) =>
        placeDock({
          size: dockSize(MEDIUM.dock.iconSize),
          groups: [count, 2],
          hover: hoverConfigOf(MEDIUM.dock),
          status: false,
          edge: "bottom",
          magnified: null,
          viewport: {
            width: MEDIUM_WINDOW.width,
            height: MEDIUM_WINDOW.height,
          },
        }).items;
      const centerOf = (index: number, count = 3) => {
        const item = rest(count)[index];
        return (item?.x ?? 0) + (item?.width ?? 0) / 2;
      };
      const placeCalls = () =>
        invoke.mock.calls.filter(([command]) => command === "place_app");

      const press = (name: string, x: number) =>
        fireEvent.pointerDown(program(name), {
          pointerId: 1,
          button: 0,
          clientX: x,
        });
      const move = (name: string, x: number) =>
        fireEvent.pointerMove(program(name), { pointerId: 1, clientX: x });
      const release = (name: string, x: number) =>
        fireEvent.pointerUp(program(name), { pointerId: 1, clientX: x });

      beforeEach(() => {
        pinned = withApps(CALCULATOR, NOTEPAD, THIRD);
      });

      it("puts the program where it is let go, among the pinned ones", async () => {
        await renderDock();

        press("Calculadora", centerOf(0));
        move("Calculadora", centerOf(1));
        move("Calculadora", centerOf(2) + 3);
        release("Calculadora", centerOf(2) + 3);

        expect(placeCalls()).toEqual([
          ["place_app", { id: CALCULATOR.id, index: 2 }],
        ]);
      });

      it("moves a program to the start as well", async () => {
        await renderDock();

        press("Terceiro", centerOf(2));
        move("Terceiro", centerOf(0) - 5);
        release("Terceiro", centerOf(0) - 5);

        expect(placeCalls()).toEqual([
          ["place_app", { id: THIRD.id, index: 0 }],
        ]);
      });

      it("does nothing when it is let go where it was", async () => {
        await renderDock();

        press("Bloco de notas", centerOf(1));
        move("Bloco de notas", centerOf(1) + 8);
        release("Bloco de notas", centerOf(1) + 8);

        expect(placeCalls()).toHaveLength(0);
      });

      it("shows the item following the pointer and a mark where it would land", async () => {
        await renderDock();
        press("Calculadora", centerOf(0));

        move("Calculadora", centerOf(1) + 3);

        expect(program("Calculadora")).toHaveAttribute("data-dragging");
        expect(program("Calculadora").style.getPropertyValue("--drag-x")).toBe(
          `${centerOf(1) + 3 - centerOf(0)}px`,
        );
        // Over the second: it would go after it, before the third.
        expect(program("Terceiro")).toHaveAttribute("data-drop", "before");
        expect(program("Bloco de notas")).not.toHaveAttribute("data-drop");

        release("Calculadora", centerOf(1) + 3);
        expect(program("Calculadora")).not.toHaveAttribute("data-dragging");
        expect(program("Terceiro")).not.toHaveAttribute("data-drop");
      });

      it("marks the end of the bar after the last program", async () => {
        await renderDock();
        press("Calculadora", centerOf(0));

        move("Calculadora", centerOf(2) + 10);

        expect(program("Terceiro")).toHaveAttribute("data-drop", "after");
      });

      it("is a click while the pointer stays within a few pixels", async () => {
        await renderDock();

        press("Calculadora", centerOf(0));
        move("Calculadora", centerOf(0) + 3);
        release("Calculadora", centerOf(0) + 3);
        fireEvent.click(program("Calculadora"));

        expect(placeCalls()).toHaveLength(0);
        expect(invoke).toHaveBeenCalledWith("launch_app", {
          id: CALCULATOR.id,
        });
      });

      it("does not open the program the drag ended on", async () => {
        await renderDock();

        press("Calculadora", centerOf(0));
        move("Calculadora", centerOf(2));
        release("Calculadora", centerOf(2));
        fireEvent.click(program("Calculadora"));

        expect(invoke).not.toHaveBeenCalledWith(
          "launch_app",
          expect.anything(),
        );
      });

      it("is called off with Esc, and nothing moves", async () => {
        await renderDock();
        press("Calculadora", centerOf(0));
        move("Calculadora", centerOf(2));

        fireEvent.keyDown(program("Calculadora"), { key: "Escape" });
        release("Calculadora", centerOf(2));

        expect(placeCalls()).toHaveLength(0);
        expect(program("Calculadora")).not.toHaveAttribute("data-dragging");
      });

      it("is called off when the pointer leaves the dock for good", async () => {
        await renderDock();
        press("Calculadora", centerOf(0));
        move("Calculadora", centerOf(2));
        expect(program("Calculadora")).toHaveAttribute("data-dragging");

        emit("dock-released", null);
        release("Calculadora", centerOf(2));

        expect(program("Calculadora")).not.toHaveAttribute("data-dragging");
        expect(placeCalls()).toHaveLength(0);
      });

      it("is only for the programs: the other items do not drag", async () => {
        await renderDock();

        press(text.items.settings, centerOf(0));
        move(text.items.settings, centerOf(2));
        release(text.items.settings, centerOf(2));

        expect(settings()).not.toHaveAttribute("data-dragging");
        expect(placeCalls()).toHaveLength(0);
      });

      it("only starts with the main button", async () => {
        await renderDock();

        fireEvent.pointerDown(program("Calculadora"), {
          pointerId: 1,
          button: 2,
          clientX: centerOf(0),
        });
        move("Calculadora", centerOf(2));

        expect(program("Calculadora")).not.toHaveAttribute("data-dragging");
      });

      it("shows no label and no magnification while it drags", async () => {
        await renderDock();
        fireEvent.pointerOver(program("Bloco de notas"));
        expect(shownTip()).not.toBeNull();

        press("Calculadora", centerOf(0));
        move("Calculadora", centerOf(2));

        expect(shownTip()).toBeNull();
        expect(program("Bloco de notas")).not.toHaveAttribute("data-magnify");
      });
    });

    it("opens it with Enter and Space, once per press", async () => {
      pinned = withApps(CALCULATOR);
      await renderDock();
      const button = program("Calculadora");

      fireEvent.keyDown(button, { key: "Enter" });
      fireEvent.keyDown(button, { key: "Enter", repeat: true });
      fireEvent.keyDown(button, { key: " " });

      const launches = invoke.mock.calls.filter(
        ([command]) => command === "launch_app",
      );
      expect(launches).toHaveLength(2);
    });

    it("announces a menu and is expanded only while its menu is open", async () => {
      pinned = withApps(CALCULATOR);
      await renderDock();
      const button = program("Calculadora");
      expect(button).toHaveAttribute("aria-haspopup", "menu");
      expect(button).toHaveAttribute("aria-expanded", "false");

      emit("panel-changed", { panelId: { app: CALCULATOR.id } });

      expect(button).toHaveAttribute("aria-expanded", "true");
      expect(settings()).toHaveAttribute("aria-expanded", "false");
    });

    it("opens the menu on a right click, anchored where the item rests, and not the program", async () => {
      pinned = withApps(CALCULATOR, NOTEPAD);
      await renderDock();

      const proceeded = fireEvent.contextMenu(program("Bloco de notas"));

      // The page's own menu is not shown.
      expect(proceeded).toBe(false);
      expect(panelCalls()).toHaveLength(1);
      const [, args] = panelCalls()[0] as [
        string,
        { panelId: unknown; anchor: { x: number; width: number } },
      ];
      expect(args.panelId).toEqual({ app: NOTEPAD.id });
      expect(args.anchor.width).toBeGreaterThan(0);
      expect(invoke).not.toHaveBeenCalledWith("launch_app", expect.anything());
    });

    it("anchors the menu of each program at its own place", async () => {
      pinned = withApps(CALCULATOR, NOTEPAD);
      await renderDock();

      fireEvent.contextMenu(program("Calculadora"));
      fireEvent.contextMenu(program("Bloco de notas"));

      const anchors = panelCalls().map(
        ([, args]) => (args as { anchor: { x: number } }).anchor.x,
      );
      expect(anchors[1]).toBeGreaterThan(anchors[0] as number);
    });

    it("leaves the right click of the other items alone", async () => {
      pinned = withApps(CALCULATOR);
      await renderDock();

      const proceeded = fireEvent.contextMenu(settings());

      expect(proceeded).toBe(true);
      expect(panelCalls()).toHaveLength(0);
    });

    it("labels the hovered program, even while a panel is open", async () => {
      pinned = withApps(CALCULATOR);
      await renderDock();

      fireEvent.pointerOver(program("Calculadora"));

      expect(shownTip()?.textContent).toBe("Calculadora");
    });

    it("keeps a failure to open it to the log: the dock stays as it is", async () => {
      pinned = withApps(CALCULATOR);
      await renderDock();
      invoke.mockImplementation((command: string) =>
        command === "launch_app"
          ? Promise.reject({ code: "launchFailed" })
          : Promise.resolve(undefined),
      );

      fireEvent.click(program("Calculadora"));
      await act(async () => {});

      expect(screen.getAllByRole("button")).toHaveLength(3);
    });

    it("shows no program while the storage is unavailable", async () => {
      pinned = {
        storage: { state: "unavailable", reason: "openFailed" },
        apps: [CALCULATOR],
      };
      await renderDock();

      expect(screen.queryByRole("button", { name: "Calculadora" })).toBeNull();
      expect(screen.getAllByRole("button")).toHaveLength(2);
    });

    it("follows apps-changed: items and names", async () => {
      await renderDock();

      emit("apps-changed", withApps(CALCULATOR));
      expect(program("Calculadora")).toBeInTheDocument();

      emit("apps-changed", withApps(NOTEPAD));
      expect(screen.queryByRole("button", { name: "Calculadora" })).toBeNull();
      expect(program("Bloco de notas")).toBeInTheDocument();
    });

    it("keeps the arrows, Home and End across the programs and the system items", async () => {
      pinned = withApps(CALCULATOR, NOTEPAD);
      await renderDock();
      const first = program("Calculadora");
      focus(first);

      fireEvent.keyDown(first, { key: "End" });

      expect(about()).toHaveFocus();
      fireEvent.keyDown(about(), { key: "Home" });
      expect(first).toHaveFocus();
    });

    it("tells Rust the window it needs, with the programs in it", async () => {
      await renderDock();
      const before = layoutCalls().length;

      emit("apps-changed", withApps(CALCULATOR, NOTEPAD));
      await act(async () => {});

      expect(layoutCalls().length).toBeGreaterThan(before);
    });
  });

  describe("cleanup", () => {
    it("drops every listener and observer when unmounted", async () => {
      const disconnect = vi.fn();
      vi.stubGlobal(
        "ResizeObserver",
        class {
          observe() {}
          disconnect = disconnect;
        },
      );
      const motion = stubReducedMotion(false);
      const { unmount } = await renderDock();
      const events = listen.mock.calls.map(([event]) => event);
      expect(events).toEqual(
        expect.arrayContaining([
          "dock-released",
          "panel-changed",
          "dock-request-panel",
          "settings-changed",
          "widgets-changed",
          "apps-changed",
        ]),
      );

      unmount();
      await act(async () => {});

      expect(disconnect).toHaveBeenCalledOnce();
      expect(motion.media.removeEventListener).toHaveBeenCalledWith(
        "change",
        motion.media.addEventListener.mock.calls[0]?.[1],
      );
      expect(unlisten).toHaveBeenCalledTimes(listen.mock.calls.length);
    });
  });
});
