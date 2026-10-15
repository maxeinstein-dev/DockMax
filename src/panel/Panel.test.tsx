import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PanelShowPayload } from "../core/panels";
import { ptBR } from "../i18n/pt-BR";
import Panel from "./Panel";

const text = ptBR.panel;

type Handler = (event: { payload: PanelShowPayload }) => void;

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

function show(panelId: PanelShowPayload["panelId"]) {
  const handler = handlers.get("panel-show");
  if (!handler) throw new Error("panel-show listener not registered");
  act(() => handler({ payload: { panelId } }));
}

describe("Panel", () => {
  beforeEach(() => {
    handlers.clear();
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders nothing until a panel is shown", () => {
    const { container } = render(<Panel />);

    expect(container).toBeEmptyDOMElement();
  });

  it("renders the panel named by the panel-show event", () => {
    render(<Panel />);

    show("settings");
    expect(
      screen.getByRole("heading", { name: text.titles.settings }),
    ).toBeInTheDocument();
    expect(screen.getByText(text.underConstruction)).toBeInTheDocument();

    show("about");
    expect(
      screen.getByRole("heading", { name: text.titles.about }),
    ).toBeInTheDocument();
  });

  it("shows its texts in PT-BR", () => {
    render(<Panel />);

    // Literal on purpose: catches a broken dictionary, not just a renamed key.
    show("settings");
    expect(
      screen.getByRole("heading", { name: "Configurações" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Em construção.")).toBeInTheDocument();

    show("about");
    expect(screen.getByRole("heading", { name: "Sobre" })).toBeInTheDocument();
  });

  it("asks to hide the panel on Escape", () => {
    render(<Panel />);
    show("about");

    fireEvent.keyDown(window, { key: "Escape" });

    expect(invoke).toHaveBeenCalledWith("hide_panel");
  });

  it("ignores other keys", () => {
    render(<Panel />);

    fireEvent.keyDown(window, { key: "Enter" });

    expect(invoke).not.toHaveBeenCalled();
  });

  it("removes its keydown listener when unmounted", () => {
    const add = vi.spyOn(window, "addEventListener");
    const remove = vi.spyOn(window, "removeEventListener");
    const { unmount } = render(<Panel />);
    const added = add.mock.calls.find(([type]) => type === "keydown");
    expect(added).toBeDefined();

    unmount();

    expect(remove).toHaveBeenCalledWith("keydown", added?.[1]);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("stops listening when unmounted", async () => {
    const { unmount } = render(<Panel />);

    unmount();
    await Promise.resolve();

    expect(unlisten).toHaveBeenCalled();
  });
});
