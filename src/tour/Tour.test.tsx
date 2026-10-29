import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ptBR } from "../i18n/pt-BR";
import TourApp from "./TourApp";

const text = ptBR.tour;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation((command: string) =>
    Promise.resolve(
      command === "get_settings" ? { launcherHotkey: "Ctrl+Space" } : null,
    ),
  );
});

describe("TourApp", () => {
  it("walks the steps forward and back, naming the launcher's hotkey", async () => {
    render(<TourApp />);
    await act(() => Promise.resolve());

    expect(
      screen.getByRole("heading", { name: text.steps.welcome.title }),
    ).toHaveFocus();
    expect(screen.getByText("1 de 6")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: text.back })).toBeNull();

    for (let step = 0; step < 4; step++) {
      fireEvent.click(screen.getByRole("button", { name: text.next }));
    }
    expect(
      screen.getByRole("heading", { name: "Ache tudo com Ctrl+Space" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: text.back }));
    expect(
      screen.getByRole("heading", { name: text.steps.widgets.title }),
    ).toBeInTheDocument();
  });

  it("ends by closing, or by opening Configurações", async () => {
    render(<TourApp />);
    await act(() => Promise.resolve());
    for (let step = 0; step < 5; step++) {
      fireEvent.click(screen.getByRole("button", { name: text.next }));
    }
    expect(screen.queryByRole("button", { name: text.next })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: text.openSettings }));
    expect(invoke).toHaveBeenCalledWith("finish_tour", { settings: true });
  });

  it("skips with the button or Esc", async () => {
    render(<TourApp />);
    await act(() => Promise.resolve());
    fireEvent.click(screen.getByRole("button", { name: text.skip }));
    expect(invoke).toHaveBeenCalledWith("finish_tour", { settings: false });
    invoke.mockClear();
    fireEvent.keyDown(screen.getByRole("main"), { key: "Escape" });
    expect(invoke).toHaveBeenCalledWith("finish_tour", { settings: false });
  });
});
