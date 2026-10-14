import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Dock from "./Dock";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("Dock", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
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
});
