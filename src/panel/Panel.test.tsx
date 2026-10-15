import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PanelReadyArgs, PanelShowPayload } from "../core/panels";
import type { Settings } from "../core/settings";
import { ptBR } from "../i18n/pt-BR";
import Panel from "./Panel";

const text = ptBR.panel;

const SETTINGS: Settings = {
  version: 1,
  theme: "system",
  dock: { edge: "bottom", iconSize: 48, opacity: "translucent" },
};

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

const { log, failing } = vi.hoisted(() => ({
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
  /** The panel whose content throws, if any. */
  failing: { panelId: null as PanelShowPayload["panelId"] | null },
}));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log,
}));
vi.mock("./PanelContent", async (importOriginal) => {
  const { default: Real } =
    await importOriginal<typeof import("./PanelContent")>();
  return {
    default: (props: Parameters<typeof Real>[0]) => {
      if (props.panelId === failing.panelId) {
        // The message carries a path that must never reach the log.
        throw new TypeError("C:\\Users\\someone\\settings.json");
      }
      return <Real {...props} />;
    },
  };
});

/** Frame sizes the fake layout gives each content, so a size sent for the
 * wrong panel shows. */
const SIZES = {
  settings: { width: 336, height: 345.4 },
  about: { width: 316, height: 235.2 },
  failed: { width: 336, height: 104 },
};

/** What the frame held each time it was measured. */
const measured: string[] = [];

function fakeLayout(this: HTMLElement): DOMRect {
  const rect = (size: { width: number; height: number }) =>
    ({ x: 0, y: 0, top: 0, left: 0, ...size }) as DOMRect;
  if (!this.classList.contains("panel-frame")) return rect(SIZES.failed);
  measured.push(this.textContent);
  if (this.querySelector(".segmented")) return rect(SIZES.settings);
  if (this.querySelector(".about-head")) return rect(SIZES.about);
  return rect(SIZES.failed);
}

function show(panelId: PanelShowPayload["panelId"]) {
  const handler = handlers.get("panel-show");
  if (!handler) throw new Error("panel-show listener not registered");
  act(() => handler({ payload: { panelId } }));
}

/** The `panel_ready` calls so far, in order. */
function readyCalls(): PanelReadyArgs[] {
  return invoke.mock.calls
    .filter(([command]) => command === "panel_ready")
    .map(([, args]) => args as PanelReadyArgs);
}

describe("Panel", () => {
  beforeEach(() => {
    handlers.clear();
    measured.length = 0;
    invoke.mockReset();
    invoke.mockImplementation((command: string) =>
      Promise.resolve(command === "get_settings" ? SETTINGS : undefined),
    );
    unlisten.mockReset();
    log.error.mockReset();
    failing.panelId = null;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
      fakeLayout,
    );
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
    expect(
      screen.getByRole("group", { name: ptBR.settings.groups.theme }),
    ).toBeInTheDocument();

    show("about");
    expect(
      screen.getByRole("heading", { name: text.titles.about }),
    ).toBeInTheDocument();
    expect(screen.getByText(ptBR.about.footer)).toBeInTheDocument();
    expect(screen.queryByRole("group")).not.toBeInTheDocument();
  });

  it("shows its texts in PT-BR", () => {
    render(<Panel />);

    // Literal on purpose: catches a broken dictionary, not just a renamed key.
    show("settings");
    expect(
      screen.getByRole("heading", { name: "Configurações" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Salvo automaticamente")).toBeInTheDocument();

    show("about");
    expect(screen.getByRole("heading", { name: "Sobre" })).toBeInTheDocument();
    expect(screen.getByText("Projeto independente")).toBeInTheDocument();
  });

  it("shows the settings in use", async () => {
    render(<Panel />);
    await act(() => Promise.resolve());

    show("settings");

    expect(screen.getByRole("button", { name: "Sistema" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Médio" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  describe("panel_ready", () => {
    it("is not sent before a panel is shown", () => {
      render(<Panel />);

      expect(readyCalls()).toEqual([]);
    });

    it("is sent once per panel-show, with the frame size and inset", () => {
      render(<Panel />);

      show("settings");

      expect(readyCalls()).toEqual([
        {
          panelId: "settings",
          size: { width: 336, height: 346 },
          inset: 8,
        },
      ]);
    });

    it("is sent again on switching, measured on the new content only", () => {
      render(<Panel />);
      show("settings");

      show("about");

      expect(readyCalls()).toEqual([
        { panelId: "settings", size: { width: 336, height: 346 }, inset: 8 },
        { panelId: "about", size: { width: 316, height: 236 }, inset: 8 },
      ]);
      // The about measurement never saw the settings content.
      expect(measured).toHaveLength(2);
      expect(measured[1]).not.toContain(ptBR.settings.groups.theme);
      expect(measured[1]).toContain(ptBR.about.footer);
    });

    it("is sent again when the same panel is shown again", () => {
      render(<Panel />);
      show("about");

      show("about");

      expect(readyCalls().map((args) => args.panelId)).toEqual([
        "about",
        "about",
      ]);
    });

    it("is sent for a panel whose content failed, with the failure's size", () => {
      failing.panelId = "settings";
      vi.spyOn(console, "error").mockImplementation(() => {});
      render(<Panel />);

      show("settings");

      expect(readyCalls()).toEqual([
        { panelId: "settings", size: { width: 336, height: 104 }, inset: 8 },
      ]);
    });

    it("logs only the kind of error when it fails", async () => {
      invoke.mockImplementation((command: string) =>
        command === "panel_ready"
          ? Promise.reject(new Error("C:\\Users\\someone"))
          : Promise.resolve(command === "get_settings" ? SETTINGS : undefined),
      );
      render(<Panel />);

      show("about");
      await act(() => Promise.resolve());

      expect(log.error).toHaveBeenCalledWith("panel: ready failed: Error");
    });
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

    expect(invoke).not.toHaveBeenCalledWith("hide_panel");
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
    expect(invoke).not.toHaveBeenCalledWith("hide_panel");
  });

  it("stops listening to panel-show and settings-changed when unmounted", async () => {
    const { unmount } = render(<Panel />);
    expect([...handlers.keys()].sort()).toEqual([
      "panel-show",
      "settings-changed",
    ]);

    unmount();
    await act(() => Promise.resolve());

    expect(unlisten).toHaveBeenCalledTimes(2);
  });

  describe("when a panel's content fails", () => {
    beforeEach(() => {
      failing.panelId = "settings";
      // React reports caught render errors on console.error; expected here.
      vi.spyOn(console, "error").mockImplementation(() => {});
    });

    it("shows the failure in place of the content, keeping the title", () => {
      render(<Panel />);

      show("settings");

      expect(screen.getByRole("alert")).toHaveTextContent(
        "Não foi possível mostrar este painel.",
      );
      expect(
        screen.getByRole("heading", { name: text.titles.settings }),
      ).toBeInTheDocument();
      expect(screen.queryByText(ptBR.settings.footer)).not.toBeInTheDocument();
    });

    it("still hides on Escape", () => {
      render(<Panel />);
      show("settings");

      fireEvent.keyDown(window, { key: "Escape" });

      expect(invoke).toHaveBeenCalledWith("hide_panel");
    });

    it("logs only the kind of error", () => {
      render(<Panel />);

      show("settings");

      expect(log.error).toHaveBeenCalledTimes(1);
      expect(log.error).toHaveBeenCalledWith("panel: render failed: TypeError");
    });

    it("recovers when another panel is shown", () => {
      render(<Panel />);
      show("settings");

      show("about");

      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(
        screen.getByRole("heading", { name: text.titles.about }),
      ).toBeInTheDocument();
      expect(screen.getByText(ptBR.about.footer)).toBeInTheDocument();
    });
  });
});
