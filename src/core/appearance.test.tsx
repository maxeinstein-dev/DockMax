import {
  act,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Appearance from "../components/Appearance";
import { applyAppearance, useAppearance } from "./appearance";
import { SETTINGS_CHANGED_EVENT, type Settings } from "./settings";

type Handler = (event: { payload: Settings }) => void;

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

const DEFAULTS: Settings = {
  version: 1,
  theme: "system",
  dock: { edge: "bottom", iconSize: 48, opacity: "translucent" },
};

function settingsWith(
  theme: Settings["theme"],
  opacity: Settings["dock"]["opacity"],
) {
  return { ...DEFAULTS, theme, dock: { ...DEFAULTS.dock, opacity } };
}

describe("applyAppearance", () => {
  let root: HTMLElement;

  beforeEach(() => {
    root = document.createElement("html");
  });

  it.each(["light", "dark"] as const)("forces the %s theme", (theme) => {
    applyAppearance(root, settingsWith(theme, "translucent"), {
      window: "dock",
    });

    expect(root.dataset.mode).toBe(theme);
  });

  it("leaves the system theme to prefers-color-scheme", () => {
    applyAppearance(root, settingsWith("dark", "translucent"), {
      window: "panel",
    });

    applyAppearance(root, settingsWith("system", "translucent"), {
      window: "panel",
    });

    expect(root.hasAttribute("data-mode")).toBe(false);
  });

  it.each(["translucent", "solid"] as const)(
    "gives the dock the %s opacity",
    (opacity) => {
      applyAppearance(root, settingsWith("system", opacity), {
        window: "dock",
      });

      expect(root.dataset.opacity).toBe(opacity);
    },
  );

  it("never gives the panel an opacity: it is always opaque", () => {
    applyAppearance(root, settingsWith("dark", "translucent"), {
      window: "panel",
    });
    applyAppearance(root, settingsWith("dark", "solid"), { window: "panel" });

    expect(root.hasAttribute("data-opacity")).toBe(false);
    expect(root.dataset.mode).toBe("dark");
  });

  it("leaves size and edge alone (they are the dock layout's)", () => {
    applyAppearance(
      root,
      { ...DEFAULTS, dock: { edge: "top", iconSize: 64, opacity: "solid" } },
      { window: "dock" },
    );

    expect(Object.keys(root.dataset).sort()).toEqual(["opacity"]);
  });
});

describe("useAppearance", () => {
  const html = document.documentElement;

  beforeEach(() => {
    handlers.clear();
    invoke.mockReset();
    invoke.mockResolvedValue(settingsWith("light", "solid"));
  });

  afterEach(() => {
    delete html.dataset.mode;
    delete html.dataset.opacity;
  });

  it("applies the settings in use to <html>, then each change", async () => {
    const { result } = renderHook(() => useAppearance("dock"));
    expect(html.hasAttribute("data-mode")).toBe(false);

    await waitFor(() => expect(html.dataset.mode).toBe("light"));
    expect(html.dataset.opacity).toBe("solid");
    expect(result.current?.theme).toBe("light");

    const handler = handlers.get(SETTINGS_CHANGED_EVENT);
    act(() => handler?.({ payload: settingsWith("system", "translucent") }));

    expect(html.hasAttribute("data-mode")).toBe(false);
    expect(html.dataset.opacity).toBe("translucent");
  });

  it("applies only the theme in the panel", async () => {
    renderHook(() => useAppearance("panel"));

    await waitFor(() => expect(html.dataset.mode).toBe("light"));
    expect(html.hasAttribute("data-opacity")).toBe(false);
  });

  it("stops following changes when unmounted", async () => {
    const { unmount } = renderHook(() => useAppearance("dock"));
    await waitFor(() => expect(html.dataset.mode).toBe("light"));

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalled());
  });
});

describe("Appearance", () => {
  const html = document.documentElement;

  beforeEach(() => {
    handlers.clear();
    invoke.mockReset();
    invoke.mockResolvedValue(settingsWith("dark", "solid"));
  });

  afterEach(() => {
    delete html.dataset.mode;
    delete html.dataset.opacity;
  });

  it("renders its children and applies the settings of its window", async () => {
    render(
      <Appearance window="dock">
        <p>Dock</p>
      </Appearance>,
    );

    expect(screen.getByText("Dock")).toBeInTheDocument();
    await waitFor(() => expect(html.dataset.mode).toBe("dark"));
    expect(html.dataset.opacity).toBe("solid");
  });
});
