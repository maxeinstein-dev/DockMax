import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { LauncherStatus } from "../core/launcher";
import { ptBR } from "../i18n/pt-BR";
import LauncherField from "./LauncherField";

const text = ptBR.settings.launcher;

type Listener = (event: { payload: LauncherStatus }) => void;

const { invoke, listen, listeners } = vi.hoisted(() => {
  const listeners: Listener[] = [];
  return {
    listeners,
    invoke: vi.fn(),
    listen: vi.fn((_event: string, handler: Listener) => {
      listeners.push(handler);
      return Promise.resolve(() => {});
    }),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

async function renderField(
  on: boolean | null,
  status: LauncherStatus,
  onChange = vi.fn(),
) {
  invoke.mockImplementation((command: string) =>
    Promise.resolve(command === "launcher_status" ? status : undefined),
  );
  render(
    <LauncherField
      on={on}
      onChange={onChange}
      hotkey={on === null ? null : "Alt+Space"}
    />,
  );
  await act(async () => {});
  return onChange;
}

const toggle = () =>
  screen.getByRole("switch", { name: "Lançador (Alt+Espaço)" });
const taken = "Alt+Espaço está em uso por outro programa.";

describe("LauncherField", () => {
  beforeEach(() => {
    invoke.mockReset();
    listeners.length = 0;
  });

  it("switches Alt+Space on and off", async () => {
    const onChange = await renderField(true, { hotkeyTaken: false });

    expect(toggle()).toHaveAttribute("aria-checked", "true");
    fireEvent.click(toggle());
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("waits for the settings before it can be switched", async () => {
    await renderField(null, { hotkeyTaken: false });

    expect(toggle()).toBeDisabled();
  });

  it("says when another program holds Alt+Space", async () => {
    await renderField(true, { hotkeyTaken: true });

    expect(screen.getByText(taken)).toBeInTheDocument();
  });

  it("says nothing of the hotkey while it is off", async () => {
    await renderField(false, { hotkeyTaken: true });

    expect(screen.queryByText(taken)).not.toBeInTheDocument();
  });

  it("follows the status Rust sends", async () => {
    await renderField(true, { hotkeyTaken: false });
    expect(screen.queryByText(taken)).not.toBeInTheDocument();

    act(() => {
      for (const listener of listeners) {
        listener({ payload: { hotkeyTaken: true } });
      }
    });
    expect(screen.getByText(taken)).toBeInTheDocument();
  });

  it("names the hotkey in use", async () => {
    invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "launcher_status" ? { hotkeyTaken: true } : undefined,
      ),
    );
    render(<LauncherField on onChange={vi.fn()} hotkey="Ctrl+Shift+Space" />);
    await act(async () => {});

    expect(
      screen.getByRole("switch", { name: "Lançador (Ctrl+Shift+Espaço)" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Ctrl+Shift+Espaço está em uso por outro programa."),
    ).toBeInTheDocument();
  });

  it("clears the history once, and says so", async () => {
    await renderField(true, { hotkeyTaken: false });

    fireEvent.click(screen.getByRole("button", { name: text.clear }));
    await act(async () => {});

    expect(invoke).toHaveBeenCalledWith("clear_launcher_history");
    expect(screen.getByRole("button", { name: text.cleared })).toBeDisabled();
  });

  it("can be cleared again when clearing failed", async () => {
    await renderField(true, { hotkeyTaken: false });
    invoke.mockImplementation(() => Promise.reject(new Error("locked")));

    fireEvent.click(screen.getByRole("button", { name: text.clear }));
    await act(async () => {});

    expect(screen.getByRole("button", { name: text.clear })).toBeEnabled();
  });
});
