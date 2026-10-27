import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { hotkeyFromEvent, hotkeyLabel } from "../core/launcher";
import { ptBR } from "../i18n/pt-BR";
import HotkeyField from "./HotkeyField";

const text = ptBR.settings.launcher.hotkey;

type Listener = (event: { payload: string }) => void;

const { invoke, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, Listener>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: Listener) => {
    listeners.set(event, handler);
    return Promise.resolve(() => listeners.delete(event));
  }),
}));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const button = () => screen.getByRole("button", { pressed: false });
const capturing = () => screen.getByRole("button", { name: text.press });

/** A key press as Chromium reports it on Windows: the virtual key in
 * `keyCode`, the text in `key`. */
function press(
  keyCode: number,
  modifiers: Partial<
    Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>
  > = {},
  key = "",
) {
  fireEvent.keyDown(capturing(), { keyCode, key, ...modifiers });
}

describe("HotkeyField", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  it("shows the hotkey in use, readable", () => {
    render(<HotkeyField hotkey="Alt+Space" />);

    expect(button()).toHaveTextContent("Alt+Espaço");
    expect(button()).toHaveAccessibleName("Alterar o atalho, hoje Alt+Espaço");
    expect(
      screen.queryByRole("button", { name: text.restore }),
    ).not.toBeInTheDocument();
  });

  it("lets go of the hotkey while it captures, and saves the new one", async () => {
    render(<HotkeyField hotkey="Alt+Space" />);

    fireEvent.click(button());
    expect(invoke).toHaveBeenCalledWith("capture_launcher_hotkey", {
      on: true,
    });
    expect(capturing()).toHaveAttribute("aria-pressed", "true");

    // Modifiers alone wait for the key.
    press(17, { ctrlKey: true }, "Control");
    expect(invoke).not.toHaveBeenCalledWith(
      "set_launcher_hotkey",
      expect.anything(),
    );
    press(75, { ctrlKey: true, shiftKey: true }, "K");
    await act(async () => {});

    expect(invoke).toHaveBeenCalledWith("set_launcher_hotkey", {
      hotkey: "Ctrl+Shift+K",
    });
    expect(screen.getByRole("status")).toHaveTextContent(
      "Atalho salvo: Ctrl+Shift+K",
    );
  });

  it("says why Rust refused the combination", async () => {
    invoke.mockImplementation((command: string) =>
      command === "set_launcher_hotkey"
        ? Promise.reject({ code: "taken" })
        : Promise.resolve(undefined),
    );
    render(<HotkeyField hotkey="Alt+Space" />);

    fireEvent.click(button());
    press(32, { altKey: true, shiftKey: true }, " ");
    await act(async () => {});

    expect(screen.getByRole("alert")).toHaveTextContent(text.errors.taken);
  });

  it("refuses a key a hotkey can't have, without asking Rust", () => {
    render(<HotkeyField hotkey="Alt+Space" />);

    fireEvent.click(button());
    press(13, { altKey: true }, "Enter");

    expect(screen.getByRole("alert")).toHaveTextContent(
      text.errors.unsupportedKey,
    );
    expect(invoke).toHaveBeenCalledWith("capture_launcher_hotkey", {
      on: false,
    });
  });

  it("cancels with Esc, without closing Configurações", () => {
    const onWindowKey = vi.fn();
    window.addEventListener("keydown", onWindowKey);
    render(<HotkeyField hotkey="Alt+Space" />);

    fireEvent.click(button());
    press(27, {}, "Escape");

    expect(onWindowKey).not.toHaveBeenCalled();
    expect(button()).toHaveTextContent("Alt+Espaço");
    expect(invoke).toHaveBeenLastCalledWith("capture_launcher_hotkey", {
      on: false,
    });
    window.removeEventListener("keydown", onWindowKey);
  });

  it("gives the hotkey back when it loses the focus or goes away", () => {
    const { unmount } = render(<HotkeyField hotkey="Alt+Space" />);

    fireEvent.click(button());
    fireEvent.blur(capturing());
    expect(invoke).toHaveBeenLastCalledWith("capture_launcher_hotkey", {
      on: false,
    });

    fireEvent.click(button());
    invoke.mockClear();
    unmount();
    expect(invoke).toHaveBeenCalledWith("capture_launcher_hotkey", {
      on: false,
    });
  });

  it("takes Alt+Space from Rust, where Windows would open the menu", async () => {
    render(<HotkeyField hotkey="Ctrl+Shift+K" />);

    expect(listeners.has("launcher-hotkey-pressed")).toBe(false);
    fireEvent.click(button());
    await act(async () => {});
    act(() =>
      listeners.get("launcher-hotkey-pressed")?.({
        payload: "Alt+Shift+Space",
      }),
    );
    await act(async () => {});

    expect(invoke).toHaveBeenCalledWith("set_launcher_hotkey", {
      hotkey: "Alt+Shift+Space",
    });
    expect(listeners.has("launcher-hotkey-pressed")).toBe(false);
  });

  it("goes back to Alt+Space", async () => {
    render(<HotkeyField hotkey="Ctrl+Shift+K" />);

    fireEvent.click(screen.getByRole("button", { name: text.restore }));
    await act(async () => {});

    expect(invoke).toHaveBeenCalledWith("set_launcher_hotkey", {
      hotkey: "Alt+Space",
    });
  });
});

describe("hotkeyFromEvent", () => {
  const none = {
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
  };

  it("writes the combination as Rust keeps it", () => {
    expect(
      hotkeyFromEvent({ ...none, keyCode: 75, ctrlKey: true, shiftKey: true }),
    ).toBe("Ctrl+Shift+K");
    expect(
      hotkeyFromEvent({
        keyCode: 32,
        ctrlKey: true,
        altKey: true,
        shiftKey: true,
        metaKey: true,
      }),
    ).toBe("Ctrl+Alt+Shift+Win+Space");
    expect(hotkeyFromEvent({ ...none, keyCode: 55, altKey: true })).toBe(
      "Alt+7",
    );
    expect(hotkeyFromEvent({ ...none, keyCode: 123, altKey: true })).toBe(
      "Alt+F12",
    );
    expect(hotkeyFromEvent({ ...none, keyCode: 135, altKey: true })).toBe(
      "Alt+F24",
    );
    expect(hotkeyFromEvent({ ...none, keyCode: 81 })).toBe("Q");
  });

  it("tells modifiers and other keys apart", () => {
    for (const keyCode of [16, 17, 18, 91, 92]) {
      expect(hotkeyFromEvent({ ...none, keyCode, altKey: true })).toBe(
        "modifier",
      );
    }
    // Enter, Tab, Up, F25 (none), the keypad's 1, the semicolon, 0 (unknown).
    for (const keyCode of [13, 9, 38, 136, 97, 186, 0]) {
      expect(hotkeyFromEvent({ ...none, keyCode, altKey: true })).toBe(
        "unsupported",
      );
    }
  });

  it("reads Space in the user's language", () => {
    expect(hotkeyLabel("Alt+Space", "Espaço")).toBe("Alt+Espaço");
    expect(hotkeyLabel("Ctrl+Shift+K", "Espaço")).toBe("Ctrl+Shift+K");
  });
});
