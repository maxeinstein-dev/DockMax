import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ptBR } from "../i18n/pt-BR";
import Dock from "./Dock";
import { MAGNIFICATION, magnifiedRect } from "./magnification";

const { invoke, listen, handlers, unlisten } = vi.hoisted(() => {
  const handlers = new Map<string, () => void>();
  const unlisten = vi.fn();
  return {
    invoke: vi.fn(),
    unlisten,
    handlers,
    listen: vi.fn((event: string, handler: () => void) => {
      handlers.set(event, handler);
      return Promise.resolve(unlisten);
    }),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const text = ptBR.dock;

const BAR = { x: 20, y: 40, width: 200, height: 64 };
const SETTINGS = { x: 28, y: 48, width: 110, height: 48 };
const ABOUT = { x: 146, y: 48, width: 66, height: 48 };

/** Lays out the bar and its items at fixed rects. */
function layOut() {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    function (this: HTMLElement) {
      const rect =
        this.tagName === "NAV"
          ? BAR
          : this.textContent === text.items.settings
            ? SETTINGS
            : this.textContent === text.items.about
              ? ABOUT
              : { x: 0, y: 0, width: 0, height: 0 };
      return rect as DOMRect;
    },
  );
}

function regionCalls() {
  return invoke.mock.calls.filter(
    ([command]) => command === "set_dock_hit_region",
  );
}

function lastRegion() {
  const calls = regionCalls();
  return calls[calls.length - 1]?.[1];
}

const settings = () =>
  screen.getByRole("button", { name: text.items.settings });
const about = () => screen.getByRole("button", { name: text.items.about });
const bar = () => screen.getByRole("navigation", { name: text.label });

describe("Dock", () => {
  beforeEach(() => {
    handlers.clear();
    unlisten.mockClear();
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("renders the dock placeholder buttons", () => {
    render(<Dock />);

    expect(settings()).toBeInTheDocument();
    expect(about()).toBeInTheDocument();
  });

  it("shows its texts in PT-BR", () => {
    render(<Dock />);

    // Literal on purpose: catches a broken dictionary, not just a renamed key.
    expect(
      screen.getByRole("navigation", { name: "Dock" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Configurações" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sobre" })).toBeInTheDocument();
  });

  it("toggles the settings panel anchored to the clicked item", () => {
    render(<Dock />);
    const button = settings();
    vi.spyOn(button, "getBoundingClientRect").mockReturnValue({
      x: 16,
      y: 12,
      width: 104,
      height: 40,
    } as DOMRect);

    fireEvent.click(button);

    expect(invoke).toHaveBeenCalledWith("toggle_panel", {
      panelId: "settings",
      anchor: { x: 16, y: 12, width: 104, height: 40 },
    });
  });

  it("toggles the about panel", () => {
    render(<Dock />);

    fireEvent.click(about());

    expect(invoke).toHaveBeenCalledWith("toggle_panel", {
      panelId: "about",
      anchor: { x: 0, y: 0, width: 0, height: 0 },
    });
  });

  it("reports the visible bar and nothing to hold on mount", () => {
    layOut();

    render(<Dock />);

    expect(invoke).toHaveBeenCalledWith("set_dock_hit_region", {
      region: { bar: BAR, hold: [] },
    });
  });

  it("gives Dock.css the shared magnification values", () => {
    render(<Dock />);

    expect(bar().style.getPropertyValue("--dock-hovered-scale")).toBe("1.4");
    expect(bar().style.getPropertyValue("--dock-hovered-lift")).toBe("6");
    expect(bar().style.getPropertyValue("--dock-neighbor-scale")).toBe("1.15");
  });

  it("magnifies the hovered item and its neighbour", () => {
    render(<Dock />);

    fireEvent.pointerOver(settings());

    expect(settings()).toHaveAttribute("data-magnify", "hovered");
    expect(about()).toHaveAttribute("data-magnify", "neighbor");
  });

  it("holds the final rects of the magnified items while hovering", () => {
    layOut();
    render(<Dock />);
    invoke.mockClear();

    fireEvent.pointerOver(settings());

    expect(regionCalls()).toHaveLength(1);
    expect(lastRegion()).toEqual({
      region: {
        bar: BAR,
        hold: [
          magnifiedRect(SETTINGS, MAGNIFICATION.hovered),
          magnifiedRect(ABOUT, MAGNIFICATION.neighbor),
        ],
      },
    });
  });

  it("finds the hovered item from a pointer over its label", () => {
    layOut();
    render(<Dock />);

    fireEvent.pointerOver(screen.getByText(text.items.about));

    expect(about()).toHaveAttribute("data-magnify", "hovered");
    expect(lastRegion()).toEqual({
      region: {
        bar: BAR,
        hold: [
          magnifiedRect(SETTINGS, MAGNIFICATION.neighbor),
          magnifiedRect(ABOUT, MAGNIFICATION.hovered),
        ],
      },
    });
  });

  it("does not report again while the pointer moves within the same item", () => {
    layOut();
    render(<Dock />);
    fireEvent.pointerOver(settings());
    invoke.mockClear();

    fireEvent.pointerMove(settings());
    fireEvent.pointerOver(screen.getByText(text.items.settings));

    expect(regionCalls()).toHaveLength(0);
  });

  it("moves the magnification when the pointer moves to another item", () => {
    layOut();
    render(<Dock />);
    fireEvent.pointerOver(settings());

    fireEvent.pointerMove(about());

    expect(about()).toHaveAttribute("data-magnify", "hovered");
    expect(settings()).toHaveAttribute("data-magnify", "neighbor");
    expect(lastRegion()).toEqual({
      region: {
        bar: BAR,
        hold: [
          magnifiedRect(SETTINGS, MAGNIFICATION.neighbor),
          magnifiedRect(ABOUT, MAGNIFICATION.hovered),
        ],
      },
    });
  });

  it("clears the magnification over the bar's padding", () => {
    layOut();
    render(<Dock />);
    fireEvent.pointerOver(settings());

    fireEvent.pointerOver(bar());

    expect(settings()).not.toHaveAttribute("data-magnify");
    expect(about()).not.toHaveAttribute("data-magnify");
    expect(lastRegion()).toEqual({ region: { bar: BAR, hold: [] } });
  });

  it("clears the magnification when the pointer leaves the bar", () => {
    layOut();
    render(<Dock />);
    fireEvent.pointerOver(settings());

    fireEvent.pointerLeave(bar());

    expect(settings()).not.toHaveAttribute("data-magnify");
    expect(lastRegion()).toEqual({ region: { bar: BAR, hold: [] } });
  });

  it("clears the magnification when Rust releases the mouse", () => {
    layOut();
    render(<Dock />);
    fireEvent.pointerOver(settings());
    const released = handlers.get("dock-released");
    expect(released).toBeDefined();

    act(() => released?.());

    expect(settings()).not.toHaveAttribute("data-magnify");
    expect(about()).not.toHaveAttribute("data-magnify");
    expect(lastRegion()).toEqual({ region: { bar: BAR, hold: [] } });
  });

  it("holds only the bar when reduced motion turns magnification off", () => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn((query: string) => ({
        matches: query === "(prefers-reduced-motion: reduce)",
      })),
    );
    layOut();
    render(<Dock />);
    expect(lastRegion()).toEqual({ region: { bar: BAR, hold: [] } });
    invoke.mockClear();

    fireEvent.pointerOver(settings());

    // Same region as before the hover: nothing new to report.
    expect(regionCalls()).toEqual([]);
  });

  it("reports the region again when the layout changes", () => {
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
    render(<Dock />);
    vi.spyOn(bar(), "getBoundingClientRect").mockReturnValue({
      x: 10,
      y: 40,
      width: 220,
      height: 64,
    } as DOMRect);
    invoke.mockClear();

    notify();

    expect(observed).toContain(bar());
    expect(invoke).toHaveBeenCalledWith("set_dock_hit_region", {
      region: { bar: { x: 10, y: 40, width: 220, height: 64 }, hold: [] },
    });
  });

  it("does not report an unchanged layout twice", () => {
    let notify = () => {};
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: () => void) {
          notify = callback;
        }
        observe() {}
        disconnect() {}
      },
    );
    layOut();
    render(<Dock />);
    invoke.mockClear();

    notify();

    expect(regionCalls()).toHaveLength(0);
  });

  it("disconnects the resize observer when unmounted", () => {
    const disconnect = vi.fn();
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        disconnect = disconnect;
      },
    );
    const { unmount } = render(<Dock />);
    expect(disconnect).not.toHaveBeenCalled();

    unmount();

    expect(disconnect).toHaveBeenCalledOnce();
  });

  it("stops listening for the release when unmounted", async () => {
    const { unmount } = render(<Dock />);
    expect(listen).toHaveBeenCalledWith("dock-released", expect.any(Function));

    unmount();
    await Promise.resolve();

    expect(unlisten).toHaveBeenCalled();
  });
});
