import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Settings } from "../core/settings";
import type { WidgetInstance, WidgetsSnapshot } from "../core/widgets";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
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

describe("Dock", () => {
  let current: Settings;
  let snapshot: WidgetsSnapshot;

  beforeEach(() => {
    handlers.clear();
    unlisten.mockClear();
    listen.mockClear();
    current = MEDIUM;
    snapshot = NO_WIDGETS;
    invoke.mockReset();
    invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "get_settings"
          ? current
          : command === "list_widgets"
            ? snapshot
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
        command === "get_settings" || command === "list_widgets"
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
