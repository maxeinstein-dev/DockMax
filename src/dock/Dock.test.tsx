import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Dock from "./Dock";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Dock", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("renders the dock placeholder buttons", () => {
    render(<Dock />);

    expect(
      screen.getByRole("button", { name: "Configurações" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sobre" })).toBeInTheDocument();
  });

  it("toggles the settings panel anchored to the clicked item", () => {
    render(<Dock />);
    const button = screen.getByRole("button", { name: "Configurações" });
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

    fireEvent.click(screen.getByRole("button", { name: "Sobre" }));

    expect(invoke).toHaveBeenCalledWith("toggle_panel", {
      panelId: "about",
      anchor: { x: 0, y: 0, width: 0, height: 0 },
    });
  });

  it("reports the visible bar rect on mount", () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      x: 20,
      y: 40,
      width: 200,
      height: 64,
    } as DOMRect);

    render(<Dock />);

    expect(invoke).toHaveBeenCalledWith("set_dock_hit_rect", {
      rect: { x: 20, y: 40, width: 200, height: 64 },
    });
  });

  it("reports the bar rect again when its layout changes", () => {
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
    const bar = screen.getByRole("navigation", { name: "Dock" });
    vi.spyOn(bar, "getBoundingClientRect").mockReturnValue({
      x: 10,
      y: 40,
      width: 220,
      height: 64,
    } as DOMRect);
    invoke.mockClear();

    notify();

    expect(observed).toContain(bar);
    expect(invoke).toHaveBeenCalledWith("set_dock_hit_rect", {
      rect: { x: 10, y: 40, width: 220, height: 64 },
    });
  });
});
