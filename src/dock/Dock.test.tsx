import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Settings } from "../core/settings";
import { ptBR } from "../i18n/pt-BR";
import Dock from "./Dock";

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
  dock: { edge: "bottom", iconSize: 48, opacity: "translucent" },
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

describe("Dock", () => {
  let current: Settings;

  beforeEach(() => {
    handlers.clear();
    unlisten.mockClear();
    listen.mockClear();
    current = MEDIUM;
    invoke.mockReset();
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "get_settings" ? current : undefined),
    );
    vi.stubGlobal("innerWidth", MEDIUM_WINDOW.width);
    vi.stubGlobal("innerHeight", MEDIUM_WINDOW.height);
    const original = HTMLElement.prototype.getBoundingClientRect;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
      function (this: HTMLElement) {
        if (!this.classList.contains("dock-tip")) return original.call(this);
        const width = LABEL_WIDTHS[this.textContent ?? ""] ?? 0;
        return { ...rect(0, 0, width, 24) } as DOMRect;
      },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
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
      await renderDock();

      expect(layoutCalls()).toHaveLength(1);
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: MEDIUM_WINDOW,
      });
    });

    it("applies the size and edge to the page", async () => {
      current = withDock({ iconSize: 64, edge: "top" });

      await renderDock();

      expect(document.documentElement.dataset.size).toBe("l");
      expect(document.documentElement.dataset.edge).toBe("top");
    });

    it("reports again when the size preset changes", async () => {
      await renderDock();

      emit("settings-changed", withDock({ iconSize: 64 }));

      expect(layoutCalls()).toHaveLength(2);
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

      emit("settings-changed", withDock({ iconSize: 50 }));

      expect(layoutCalls()).toHaveLength(2);
      expect(lastLayout()).toEqual(layoutCalls()[0]?.[1]?.layout);

      emit("settings-changed", withDock({ iconSize: 50, edge: "top" }));
      emit("settings-changed", withDock({ iconSize: 50, edge: "top" }));

      expect(layoutCalls()).toHaveLength(3);
    });

    it("does not report an unchanged layout again", async () => {
      await renderDock();

      emit("settings-changed", { ...withDock({}), theme: "dark" });
      emit("settings-changed", withDock({ opacity: "solid" }));

      expect(layoutCalls()).toHaveLength(1);
    });

    it("reports the defaults when the settings can't be read", async () => {
      invoke.mockImplementation((command: string) =>
        command === "get_settings"
          ? Promise.reject(new Error("no settings"))
          : Promise.resolve(undefined),
      );

      await renderDock();

      expect(layoutCalls()).toHaveLength(1);
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: MEDIUM_WINDOW,
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
    it("marks the open item and points it to the panel", async () => {
      await renderDock();
      expect(settings()).toHaveAttribute("aria-expanded", "false");
      expect(settings()).not.toHaveAttribute("aria-controls");

      emit("panel-changed", { panelId: "settings" });

      expect(settings()).toHaveAttribute("aria-expanded", "true");
      expect(settings()).toHaveAttribute("aria-controls", "panel");
      expect(about()).toHaveAttribute("aria-expanded", "false");

      emit("panel-changed", { panelId: null });

      expect(settings()).toHaveAttribute("aria-expanded", "false");
      expect(settings()).not.toHaveAttribute("aria-controls");
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
      await renderDock();

      fireEvent.pointerOver(settings());

      expect(settings()).not.toHaveAttribute("data-magnify");
      expect(shownTip()).toHaveTextContent("Configurações");
      expect(layoutCalls()).toHaveLength(1);
      expectClose(lastLayout(), {
        bar: REST_BAR,
        hold: [],
        window: { width: 181, height: 97 },
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
