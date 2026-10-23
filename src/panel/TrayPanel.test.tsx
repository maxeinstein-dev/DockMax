import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TrayIcon } from "../core/tray";
import { ptBR } from "../i18n/pt-BR";
import TrayPanel from "./TrayPanel";

const text = ptBR.tray;

type Handler = (event: { payload: TrayIcon[] }) => void;

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

const icon = (key: string, tip: string, extra: Partial<TrayIcon> = {}) => ({
  key,
  tip,
  image: "data:image/png;base64,AAAA",
  hidden: false,
  clickable: true,
  ...extra,
});

const FORTI = icon("1:129", "FortiClient 7.4.3");
const DOCKER = icon("2:1", "Docker Desktop");

/** Renders the panel with `icons` as the list Rust answers with. */
async function renderTray(icons: TrayIcon[]) {
  invoke.mockImplementation((command: string) =>
    Promise.resolve(command === "list_tray" ? icons : undefined),
  );
  const view = render(<TrayPanel />);
  await act(async () => {});
  return view;
}

const calls = (command: string) =>
  invoke.mock.calls.filter(([name]) => name === command);

describe("TrayPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    handlers.clear();
    unlisten.mockClear();
  });

  it("lists each icon as a button named by its tooltip, with its image", async () => {
    await renderTray([FORTI, DOCKER]);

    const buttons = screen.getAllByRole("button");
    expect(buttons.map((button) => button.getAttribute("aria-label"))).toEqual([
      "FortiClient 7.4.3",
      "Docker Desktop",
    ]);
    expect(buttons[0]?.querySelector("img")).toHaveAttribute(
      "src",
      "data:image/png;base64,AAAA",
    );
    expect(buttons[0]).toHaveAttribute("title", "FortiClient 7.4.3");
  });

  it("leaves out the icons a program asked to hide", async () => {
    await renderTray([FORTI, icon("3:0", "WhatsApp", { hidden: true })]);

    expect(screen.queryByRole("button", { name: "WhatsApp" })).toBeNull();
    expect(screen.getAllByRole("button")).toHaveLength(1);
  });

  it("names an icon without a tooltip, and draws a mark when the image is missing", async () => {
    await renderTray([icon("4:1", "", { image: null })]);

    const button = screen.getByRole("button", { name: text.unnamed });
    expect(button.querySelector("img")).toBeNull();
    expect(button.querySelector(".tray-icon-blank")).not.toBeNull();
  });

  it("clicks with the left button and then closes the panel", async () => {
    await renderTray([FORTI]);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: FORTI.tip }));
    });

    expect(calls("tray_click")).toEqual([
      ["tray_click", { key: "1:129", button: "left" }],
    ]);
    expect(calls("hide_panel")).toHaveLength(1);
  });

  it("opens the program's own menu with the right button and keeps the panel open, as the taskbar's flyout did", async () => {
    await renderTray([FORTI]);

    const event = await act(async () => {
      const button = screen.getByRole("button", { name: FORTI.tip });
      return fireEvent.contextMenu(button);
    });

    // The panel's own context menu never shows over the program's.
    expect(event).toBe(false);
    expect(calls("tray_click")).toEqual([
      ["tray_click", { key: "1:129", button: "right" }],
    ]);
    // The program's menu is open: the panel stays for the next icon.
    expect(calls("hide_panel")).toHaveLength(0);
  });

  it("closes the panel even when the click fails", async () => {
    invoke.mockImplementation((command: string) =>
      command === "list_tray"
        ? Promise.resolve([FORTI])
        : command === "tray_click"
          ? Promise.reject("refused")
          : Promise.resolve(undefined),
    );
    render(<TrayPanel />);
    await act(async () => {});

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: FORTI.tip }));
    });

    expect(calls("hide_panel")).toHaveLength(1);
  });

  it("cannot click an icon that takes no clicks", async () => {
    await renderTray([icon("5:1", "Volume", { clickable: false })]);

    expect(screen.getByRole("button", { name: "Volume" })).toBeDisabled();
  });

  it("explains the empty list instead of showing nothing", async () => {
    await renderTray([]);

    expect(screen.getByText(text.empty)).toBeInTheDocument();
    expect(screen.getByText(text.emptyHint)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("says nothing while the list is loading", () => {
    invoke.mockImplementation(() => new Promise(() => {}));

    render(<TrayPanel />);

    expect(screen.queryByText(text.empty)).toBeNull();
  });

  it("follows tray-changed while it is open", async () => {
    await renderTray([FORTI]);

    act(() => handlers.get("tray-changed")?.({ payload: [FORTI, DOCKER] }));

    expect(screen.getAllByRole("button")).toHaveLength(2);
    act(() => handlers.get("tray-changed")?.({ payload: [] }));
    expect(screen.getByText(text.empty)).toBeInTheDocument();
  });

  it("ignores a list that was read before a change arrived", async () => {
    let answer!: (icons: TrayIcon[]) => void;
    invoke.mockImplementation((command: string) =>
      command === "list_tray"
        ? new Promise((resolve) => (answer = resolve))
        : Promise.resolve(undefined),
    );
    render(<TrayPanel />);

    act(() => handlers.get("tray-changed")?.({ payload: [DOCKER] }));
    await act(async () => answer([FORTI]));

    expect(screen.getAllByRole("button")).toHaveLength(1);
    expect(
      screen.getByRole("button", { name: DOCKER.tip }),
    ).toBeInTheDocument();
  });

  it("stops listening when it closes", async () => {
    const view = await renderTray([FORTI]);

    view.unmount();
    await act(async () => {});

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("shows the footer hint about the right button", async () => {
    await renderTray([FORTI]);

    expect(screen.getByText(text.footer)).toBeInTheDocument();
  });
});
