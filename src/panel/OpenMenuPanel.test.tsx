import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { OpenOther } from "../core/openApps";
import { ptBR } from "../i18n/pt-BR";
import OpenMenuPanel from "./OpenMenuPanel";

const text = ptBR.apps.menu;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const OTHER: OpenOther = {
  key: "00000000000000aa",
  appId: "C:\\Apps\\code.exe",
  name: "code",
  count: 1,
  active: false,
  badge: false,
};

const flush = () => act(() => Promise.resolve());
const item = (name: string) => screen.getByRole("menuitem", { name });
const sent = (name: string) =>
  invoke.mock.calls
    .filter(([command]) => command === name)
    .map(([, args]) => args as unknown);

describe("OpenMenuPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  it("names the program and offers a new instance, closing and pinning", () => {
    render(<OpenMenuPanel other={OTHER} onDone={() => {}} />);

    expect(screen.getByRole("heading", { name: "code" })).toBeInTheDocument();
    expect(
      screen.getByRole("menu", { name: "Menu de code" }),
    ).toBeInTheDocument();
    expect(
      screen.getAllByRole("menuitem").map((entry) => entry.textContent),
    ).toEqual([text.newInstance, text.closeWindow, text.pin]);
    expect(item(text.newInstance)).toHaveFocus();
  });

  it("says all the windows when the program has more than one", () => {
    render(<OpenMenuPanel other={{ ...OTHER, count: 3 }} onDone={() => {}} />);

    expect(item(text.closeWindows)).toBeInTheDocument();
    expect(
      screen.queryByRole("menuitem", { name: text.closeWindow }),
    ).toBeNull();
  });

  it.each([
    [text.newInstance, "launch_open"],
    [text.closeWindow, "close_open"],
    [text.pin, "pin_open"],
  ])("runs %s by the key, and closes the panel", async (name, command) => {
    const onDone = vi.fn();
    render(<OpenMenuPanel other={OTHER} onDone={onDone} />);

    fireEvent.click(item(name));
    await flush();

    expect(sent(command)).toEqual([{ key: OTHER.key }]);
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("shows why it failed and stays open", async () => {
    invoke.mockRejectedValue({ code: "limitReached" });
    const onDone = vi.fn();
    render(<OpenMenuPanel other={OTHER} onDone={onDone} />);

    fireEvent.click(item(text.pin));
    await flush();

    expect(onDone).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(
      ptBR.apps.errors.limitReached,
    );
  });

  it("moves between the items with the arrows, Home and End", () => {
    render(<OpenMenuPanel other={OTHER} onDone={() => {}} />);
    const menu = screen.getByRole("menu");

    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(item(text.closeWindow)).toHaveFocus();
    fireEvent.keyDown(menu, { key: "End" });
    expect(item(text.pin)).toHaveFocus();
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(item(text.pin)).toHaveFocus();
    fireEvent.keyDown(menu, { key: "Home" });
    expect(item(text.newInstance)).toHaveFocus();
  });
});
