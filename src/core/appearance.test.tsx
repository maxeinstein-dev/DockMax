import {
  act,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Appearance from "../components/Appearance";
import {
  DEFAULT_DOCK_LOOK,
  applyAppearance,
  dockSize,
  useAppearance,
} from "./appearance";
import {
  DEFAULT_LOOK,
  SETTINGS_CHANGED_EVENT,
  type Settings,
} from "./settings";

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
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  launcherHotkey: "Alt+Space",
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
  },
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

  it("gives the dock its size preset and edge", () => {
    applyAppearance(
      root,
      {
        ...DEFAULTS,
        dock: { edge: "top", iconSize: 64, opacity: "solid", ...DEFAULT_LOOK },
      },
      { window: "dock" },
    );

    expect(root.dataset.size).toBe("l");
    expect(root.dataset.edge).toBe("top");

    applyAppearance(root, DEFAULTS, { window: "dock" });

    expect(root.dataset.size).toBe("m");
    expect(root.dataset.edge).toBe("bottom");
  });

  it("gives the dock its style, outline, reflections and indicator", () => {
    applyAppearance(
      root,
      {
        ...DEFAULTS,
        dock: {
          ...DEFAULTS.dock,
          style: "floating",
          indicator: "dash",
          indicatorColor: "#0A84FF",
        },
      },
      { window: "dock" },
    );

    expect(root.dataset.style).toBe("floating");
    expect(root.dataset.border).toBe("off");
    expect(root.dataset.reflect).toBe("on");
    expect(root.dataset.indicator).toBe("dash");
    expect(root.style.getPropertyValue("--corners")).toBe("0.4");
    expect(root.style.getPropertyValue("--indicator-color")).toBe("#0A84FF");
  });

  it("gives the dock the shape of the icons and the opening animation", () => {
    applyAppearance(
      root,
      {
        ...DEFAULTS,
        dock: {
          ...DEFAULTS.dock,
          iconShape: "original",
          openAnimation: "shake",
        },
      },
      { window: "dock" },
    );

    expect(root.dataset.iconShape).toBe("original");
    expect(root.dataset.openAnimation).toBe("shake");
  });

  it("starts from plates and the bounce", () => {
    applyAppearance(root, DEFAULTS, { window: "dock" });

    expect(root.dataset.iconShape).toBe("plate");
    expect(root.dataset.openAnimation).toBe("bounce");
  });

  it("puts the changes of the chosen style over its own look", () => {
    applyAppearance(
      root,
      {
        ...DEFAULTS,
        dock: {
          ...DEFAULTS.dock,
          style: "floating",
          tweaks: {
            floating: { border: true, corners: 100, reflections: false },
          },
        },
      },
      { window: "dock" },
    );

    expect(root.dataset.border).toBe("on");
    expect(root.dataset.reflect).toBe("off");
    expect(root.style.getPropertyValue("--corners")).toBe("1");
  });

  it("drops the indicator color when it goes back to the own one of the style", () => {
    applyAppearance(
      root,
      { ...DEFAULTS, dock: { ...DEFAULTS.dock, indicatorColor: "#FF0000" } },
      { window: "dock" },
    );
    expect(root.style.getPropertyValue("--indicator-color")).toBe("#FF0000");

    applyAppearance(root, DEFAULTS, { window: "dock" });

    expect(root.style.getPropertyValue("--indicator-color")).toBe("");
  });

  it("never gives the panel a size or edge", () => {
    applyAppearance(
      root,
      {
        ...DEFAULTS,
        dock: { edge: "top", iconSize: 40, opacity: "solid", ...DEFAULT_LOOK },
      },
      { window: "panel" },
    );

    expect(Object.keys(root.dataset)).toEqual([]);
  });
});

describe("dockSize", () => {
  it.each([
    [40, "s"],
    [48, "m"],
    [64, "l"],
  ] as const)("maps the preset %i to %s", (iconSize, size) => {
    expect(dockSize(iconSize)).toBe(size);
  });

  it.each([
    [32, "s"],
    [44, "s"],
    [45, "m"],
    [56, "m"],
    [57, "l"],
    [96, "l"],
  ] as const)(
    "draws any other size %i as the closest preset, %s",
    (iconSize, size) => {
      expect(dockSize(iconSize)).toBe(size);
    },
  );

  it("is Médio at the bottom by default", () => {
    expect(DEFAULT_DOCK_LOOK).toEqual({ size: "m", edge: "bottom" });
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
    for (const key of ["mode", "opacity", "size", "edge"]) {
      delete html.dataset[key];
    }
  });

  it("applies the settings in use to <html>, then each change", async () => {
    const { result } = renderHook(() => useAppearance("dock"));
    expect(html.hasAttribute("data-mode")).toBe(false);

    await waitFor(() => expect(html.dataset.mode).toBe("light"));
    expect(html.dataset.opacity).toBe("solid");
    expect(html.dataset.size).toBe("m");
    expect(html.dataset.edge).toBe("bottom");
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
    expect(html.hasAttribute("data-size")).toBe(false);
    expect(html.hasAttribute("data-edge")).toBe(false);
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
    for (const key of ["mode", "opacity", "size", "edge"]) {
      delete html.dataset[key];
    }
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
