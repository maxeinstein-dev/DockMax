import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import type { WidgetInstance, WidgetsSnapshot } from "../core/widgets";
import type { AppsSnapshot } from "../core/apps";
import { ptBR } from "../i18n/pt-BR";
import SettingsPanel from "./SettingsPanel";

const text = ptBR.settings;

const SETTINGS: Settings = {
  version: 1,
  theme: "dark",
  launchAtLogin: false,
  launcher: true,
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
  },
};

const { invoke, log } = vi.hoisted(() => ({
  invoke: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log,
}));

/** A promise settled by the test. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function group(name: string) {
  return screen.getByRole("group", { name });
}

function option(groupName: string, name: string) {
  return within(group(groupName)).getByRole("button", { name });
}

/** The labels of the chosen options of a group. */
function chosen(groupName: string): string[] {
  return within(group(groupName))
    .getAllByRole("button")
    .filter((button) => button.getAttribute("aria-pressed") === "true")
    .map((button) => button.textContent);
}

/** Lets settled promises run their callbacks. */
const flush = () => act(() => Promise.resolve());

describe("SettingsPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it("shows the four groups with the settings in use", () => {
    render(<SettingsPanel settings={SETTINGS} />);

    expect(chosen(text.groups.theme)).toEqual([text.theme.dark]);
    expect(chosen(text.groups.edge)).toEqual([text.edge.bottom]);
    expect(chosen(text.groups.size)).toEqual([text.size.medium]);
    expect(chosen(text.groups.opacity)).toEqual([text.opacity.translucent]);
    // The dock takes the taskbar's place until the user says otherwise.
    expect(chosen(text.groups.taskbar)).toEqual([text.taskbar.hidden]);
    expect(screen.getByText(text.footer)).toBeInTheDocument();
    expect(screen.getByText("Esc").tagName).toBe("KBD");
  });

  it("offers the options in PT-BR", () => {
    render(<SettingsPanel settings={SETTINGS} />);

    // Literal on purpose: catches a broken dictionary.
    expect(
      within(group("Tema"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Sistema", "Claro", "Escuro"]);
    expect(
      within(group("Posição"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Embaixo", "Em cima"]);
    expect(
      within(group("Tamanho"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Pequeno", "Médio", "Grande"]);
    expect(
      within(group("Opacidade do dock"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Translúcido", "Sólido"]);
  });

  it.each([
    [text.groups.theme, text.theme.light, { ...SETTINGS, theme: "light" }],
    [
      text.groups.edge,
      text.edge.top,
      { ...SETTINGS, dock: { ...SETTINGS.dock, edge: "top" } },
    ],
    [
      text.groups.size,
      text.size.large,
      { ...SETTINGS, dock: { ...SETTINGS.dock, iconSize: 64 } },
    ],
    [
      text.groups.size,
      text.size.small,
      { ...SETTINGS, dock: { ...SETTINGS.dock, iconSize: 40 } },
    ],
    [
      text.groups.taskbar,
      text.taskbar.visible,
      { ...SETTINGS, dock: { ...SETTINGS.dock, taskbar: "visible" } },
    ],
    [
      text.groups.taskbar,
      text.taskbar.autohide,
      { ...SETTINGS, dock: { ...SETTINGS.dock, taskbar: "autohide" } },
    ],
    [
      text.groups.opacity,
      text.opacity.solid,
      {
        ...SETTINGS,
        dock: { ...SETTINGS.dock, opacity: "solid" },
      },
    ],
  ])("saves the full settings on choosing %s: %s", (groupName, name, sent) => {
    invoke.mockReturnValue(new Promise(() => {}));
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(groupName, name));

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("update_settings", { settings: sent });
  });

  it("chooses what the middle button does", () => {
    invoke.mockReturnValue(new Promise(() => {}));
    render(<SettingsPanel settings={SETTINGS} />);
    expect(chosen(text.groups.middleClick)).toEqual([
      text.middleClick.newInstance,
    ]);

    fireEvent.click(option(text.groups.middleClick, text.middleClick.none));

    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: {
        ...SETTINGS,
        dock: { ...SETTINGS.dock, middleClick: "none" },
      },
    });
  });

  it("turns showing the open apps on and off, in the dock settings", () => {
    invoke.mockReturnValue(new Promise(() => {}));
    render(<SettingsPanel settings={SETTINGS} />);
    const toggle = screen.getByRole("switch", { name: text.showOpenApps });
    expect(toggle).toHaveAttribute("aria-checked", "true");

    fireEvent.click(toggle);

    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: {
        ...SETTINGS,
        dock: { ...SETTINGS.dock, showOpenApps: false },
      },
    });
  });

  it("turns hiding on its own on, then shows its two times to change", () => {
    invoke.mockReturnValue(new Promise(() => {}));
    const { rerender } = render(<SettingsPanel settings={SETTINGS} />);
    const toggle = screen.getByRole("switch", { name: text.autoHide });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    // The times only matter while it hides.
    expect(
      screen.queryByRole("slider", { name: text.autoHideReveal }),
    ).not.toBeInTheDocument();

    fireEvent.click(toggle);

    const on = { ...SETTINGS, dock: { ...SETTINGS.dock, autoHide: true } };
    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: on,
    });

    rerender(<SettingsPanel settings={on} />);
    const reveal = screen.getByRole("slider", { name: text.autoHideReveal });
    expect(reveal).toHaveAttribute("aria-valuetext", "0,3 s");
    expect(
      screen.getByRole("slider", { name: text.autoHideDelay }),
    ).toHaveAttribute("aria-valuetext", "0,7 s");

    fireEvent.change(reveal, { target: { value: "600" } });
    fireEvent.pointerUp(reveal);

    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: { ...on, dock: { ...on.dock, autoHideReveal: 600 } },
    });
  });

  it("turns starting with Windows on and off, saving the full settings", () => {
    invoke.mockReturnValue(new Promise(() => {}));
    const { rerender } = render(<SettingsPanel settings={SETTINGS} />);
    const toggle = () =>
      screen.getByRole("switch", { name: text.launchAtLogin });
    expect(toggle()).toHaveAttribute("aria-checked", "false");

    fireEvent.click(toggle());

    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: { ...SETTINGS, launchAtLogin: true },
    });
    expect(toggle()).toHaveAttribute("aria-checked", "true");

    rerender(<SettingsPanel settings={{ ...SETTINGS, launchAtLogin: true }} />);
    fireEvent.click(toggle());

    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: { ...SETTINGS, launchAtLogin: false },
    });
  });

  it("does nothing on choosing the option already chosen", () => {
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.theme, text.theme.dark));

    expect(invoke).not.toHaveBeenCalled();
  });

  it("shows the choice while it saves, then follows settings-changed", async () => {
    const save = deferred<Settings>();
    invoke.mockReturnValue(save.promise);
    const top: Settings = {
      ...SETTINGS,
      dock: { ...SETTINGS.dock, edge: "top" },
    };
    const { rerender } = render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.edge, text.edge.top));
    expect(chosen(text.groups.edge)).toEqual([text.edge.top]);

    // Saved, settings-changed not here yet: still the choice.
    save.resolve(top);
    await flush();
    expect(chosen(text.groups.edge)).toEqual([text.edge.top]);

    rerender(<SettingsPanel settings={top} />);
    expect(chosen(text.groups.edge)).toEqual([text.edge.top]);

    // From here on the confirmed settings rule.
    rerender(<SettingsPanel settings={SETTINGS} />);
    expect(chosen(text.groups.edge)).toEqual([text.edge.bottom]);
  });

  it("follows settings-changed arriving before the command answers", async () => {
    const save = deferred<Settings>();
    invoke.mockReturnValue(save.promise);
    const solid: Settings = {
      ...SETTINGS,
      dock: { ...SETTINGS.dock, opacity: "solid" },
    };
    const { rerender } = render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.opacity, text.opacity.solid));
    rerender(<SettingsPanel settings={solid} />);
    save.resolve(solid);
    await flush();

    expect(chosen(text.groups.opacity)).toEqual([text.opacity.solid]);
    rerender(<SettingsPanel settings={SETTINGS} />);
    expect(chosen(text.groups.opacity)).toEqual([text.opacity.translucent]);
  });

  it("shows the translated reason and goes back when Rust refuses", async () => {
    invoke.mockRejectedValue({ code: "writeFailed" });
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.size, text.size.large));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Não foi possível salvar a mudança. Tente de novo.",
    );
    expect(chosen(text.groups.size)).toEqual([text.size.medium]);
  });

  it.each([
    ["iconSizeOutOfRange", text.errors.iconSizeOutOfRange],
    ["unsupportedVersion", text.errors.unsupportedVersion],
    ["notReady", text.errors.notReady],
  ])("shows the message of %s", async (code, message) => {
    invoke.mockRejectedValue({ code });
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.theme, text.theme.light));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(message);
  });

  it("shows notReady when the command fails without a code", async () => {
    invoke.mockRejectedValue(new Error("C:\\Users\\someone"));
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.theme, text.theme.light));
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notReady);
    expect(chosen(text.groups.theme)).toEqual([text.theme.dark]);
  });

  it("clears the message on the next choice", async () => {
    invoke.mockRejectedValueOnce({ code: "writeFailed" });
    invoke.mockReturnValue(new Promise(() => {}));
    render(<SettingsPanel settings={SETTINGS} />);
    fireEvent.click(option(text.groups.theme, text.theme.light));
    await flush();
    expect(screen.getByRole("alert")).toBeInTheDocument();

    fireEvent.click(option(text.groups.theme, text.theme.system));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByText(text.footer)).toBeInTheDocument();
  });

  it("builds a quick second choice on the first one", () => {
    invoke.mockReturnValue(new Promise(() => {}));
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.theme, text.theme.light));
    fireEvent.click(option(text.groups.edge, text.edge.top));

    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: {
        ...SETTINGS,
        theme: "light",
        dock: { ...SETTINGS.dock, edge: "top" },
      },
    });
  });

  it("ignores an older change answering after a newer one", async () => {
    const first = deferred<Settings>();
    const second = deferred<Settings>();
    invoke
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    render(<SettingsPanel settings={SETTINGS} />);

    fireEvent.click(option(text.groups.theme, text.theme.light));
    fireEvent.click(option(text.groups.theme, text.theme.system));
    first.reject({ code: "writeFailed" });
    await flush();

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(chosen(text.groups.theme)).toEqual([text.theme.system]);
  });

  it("chooses no size for an icon size that matches no preset", () => {
    render(
      <SettingsPanel
        settings={{ ...SETTINGS, dock: { ...SETTINGS.dock, iconSize: 56 } }}
      />,
    );

    expect(chosen(text.groups.size)).toEqual([]);
    expect(chosen(text.groups.theme)).toEqual([text.theme.dark]);
    // The group is still reachable: its first option takes the Tab stop.
    expect(option(text.groups.size, text.size.small)).toHaveAttribute(
      "tabindex",
      "0",
    );
  });

  it("chooses nothing and saves nothing while the settings can't be read", () => {
    render(<SettingsPanel settings={null} />);

    for (const name of Object.values(text.groups)) {
      expect(chosen(name)).toEqual([]);
    }

    fireEvent.click(option(text.groups.theme, text.theme.light));

    expect(invoke).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(text.errors.notReady);
  });

  describe("keyboard", () => {
    it("has one Tab stop per group, on the chosen option", () => {
      render(<SettingsPanel settings={SETTINGS} />);

      const expected: [string, string][] = [
        [text.groups.theme, text.theme.dark],
        [text.groups.edge, text.edge.bottom],
        [text.groups.size, text.size.medium],
        [text.groups.opacity, text.opacity.translucent],
      ];
      for (const [name, label] of expected) {
        const stops = within(group(name))
          .getAllByRole("button")
          .filter((button) => button.tabIndex === 0);
        expect(stops.map((button) => button.textContent)).toEqual([label]);
      }
    });

    it("moves the focus with the arrows, wrapping, without choosing", () => {
      render(<SettingsPanel settings={SETTINGS} />);
      const dark = option(text.groups.theme, text.theme.dark);
      dark.focus();

      fireEvent.keyDown(dark, { key: "ArrowRight" });
      const system = option(text.groups.theme, text.theme.system);
      expect(system).toHaveFocus();
      expect(system).toHaveAttribute("tabindex", "0");
      expect(dark).toHaveAttribute("tabindex", "-1");

      fireEvent.keyDown(system, { key: "ArrowLeft" });
      expect(dark).toHaveFocus();

      fireEvent.keyDown(dark, { key: "ArrowUp" });
      expect(option(text.groups.theme, text.theme.light)).toHaveFocus();

      fireEvent.keyDown(document.activeElement as Element, {
        key: "ArrowDown",
      });
      expect(dark).toHaveFocus();

      fireEvent.keyDown(dark, { key: "Home" });
      expect(system).toHaveFocus();
      fireEvent.keyDown(system, { key: "End" });
      expect(dark).toHaveFocus();

      expect(invoke).not.toHaveBeenCalled();
      expect(chosen(text.groups.theme)).toEqual([text.theme.dark]);
    });

    it("keeps the arrows inside their group", () => {
      render(<SettingsPanel settings={SETTINGS} />);
      const top = option(text.groups.edge, text.edge.top);
      top.focus();

      fireEvent.keyDown(top, { key: "ArrowRight" });

      expect(option(text.groups.edge, text.edge.bottom)).toHaveFocus();
    });

    it("returns the Tab stop to the chosen option when the focus leaves", () => {
      render(<SettingsPanel settings={SETTINGS} />);
      const dark = option(text.groups.theme, text.theme.dark);
      dark.focus();
      fireEvent.keyDown(dark, { key: "ArrowRight" });

      act(() => option(text.groups.edge, text.edge.bottom).focus());

      expect(dark).toHaveAttribute("tabindex", "0");
      expect(option(text.groups.theme, text.theme.system)).toHaveAttribute(
        "tabindex",
        "-1",
      );
    });
  });
});

describe("SettingsPanel › Aparência do dock row", () => {
  const look = ptBR.settings.look;

  it("names the style in use and opens the subview", () => {
    const onOpenLook = vi.fn();
    render(
      <SettingsPanel
        settings={{ ...SETTINGS, dock: { ...SETTINGS.dock, style: "neon" } }}
        onOpenLook={onOpenLook}
      />,
    );
    const row = screen.getByRole("button", { name: new RegExp(look.title) });

    expect(row).toHaveTextContent(look.styles.neon);

    fireEvent.click(row);

    expect(onOpenLook).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("has no row without a way to open it", () => {
    render(<SettingsPanel settings={SETTINGS} />);

    expect(
      screen.queryByRole("button", { name: new RegExp(look.title) }),
    ).not.toBeInTheDocument();
  });

  it("takes the focus when it is the row the user comes back to", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        onOpenLook={vi.fn()}
        onOpenItems={vi.fn()}
        focusLookRow
      />,
    );

    expect(
      screen.getByRole("button", { name: new RegExp(look.title) }),
    ).toHaveFocus();
  });
});

describe("SettingsPanel › Apps do dock row", () => {
  const apps = ptBR.apps;
  const pinned = (count: number): AppsSnapshot => ({
    storage: { state: "ok" },
    apps: Array.from({ length: count }, (_, n) => ({
      id: `0199a4c0-0000-7000-8000-${String(n).padStart(12, "0")}`,
      appId: `app${n}!App`,
      name: `App ${n}`,
    })),
  });
  const row = () =>
    screen.getByRole("button", { name: new RegExp(apps.title) });

  it("counts the pinned programs and opens the subview", () => {
    const onOpenApps = vi.fn();
    render(
      <SettingsPanel
        settings={SETTINGS}
        apps={pinned(3)}
        onOpenApps={onOpenApps}
      />,
    );

    expect(row()).toHaveTextContent("3 de 40");

    fireEvent.click(row());

    expect(onOpenApps).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("says Indisponível while the apps are loading, unreadable or unusable", () => {
    const { rerender } = render(
      <SettingsPanel settings={SETTINGS} onOpenApps={vi.fn()} />,
    );
    expect(row()).toHaveTextContent(apps.unavailable);

    rerender(
      <SettingsPanel
        settings={SETTINGS}
        apps={{
          storage: { state: "unavailable", reason: "openFailed" },
          apps: [],
        }}
        onOpenApps={vi.fn()}
      />,
    );
    expect(row()).toHaveTextContent(apps.unavailable);
  });

  it("has no row without a way to open it", () => {
    render(<SettingsPanel settings={SETTINGS} apps={pinned(1)} />);

    expect(
      screen.queryByRole("button", { name: new RegExp(apps.title) }),
    ).not.toBeInTheDocument();
  });

  it("takes the focus when it is the row the user comes back to", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        apps={pinned(1)}
        onOpenApps={vi.fn()}
        onOpenItems={vi.fn()}
        focusAppsRow
      />,
    );

    expect(row()).toHaveFocus();
  });
});

describe("SettingsPanel › Itens do dock row", () => {
  const items = ptBR.widgets.items;
  const CLOCK: WidgetInstance = {
    id: "c1",
    label: null,
    kind: "clock",
    config: {
      timeZone: null,
      hourCycle: "h23",
      showSeconds: false,
      iconStyle: "digital",
    },
  };

  function widgets(
    count: number,
    storage: WidgetsSnapshot["storage"] = { state: "ok" },
  ): WidgetsSnapshot {
    return {
      storage,
      widgets: Array.from({ length: count }, (_, i) => ({
        ...CLOCK,
        id: `c${i}`,
      })),
    };
  }

  function row() {
    return screen.getByRole("button", { name: new RegExp(items.title) });
  }

  it("comes after the four groups, with the count, and opens the subview", () => {
    const onOpenItems = vi.fn();
    render(
      <SettingsPanel
        settings={SETTINGS}
        widgets={widgets(3)}
        onOpenItems={onOpenItems}
      />,
    );

    expect(row()).toHaveTextContent("3 de 12");
    expect(row()).not.toHaveTextContent(ptBR.widgets.notice);
    expect(
      group(text.groups.opacity).compareDocumentPosition(row()) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    fireEvent.click(row());
    expect(onOpenItems).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("marks the row while the storage was recovered", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        widgets={widgets(2, { state: "recovered" })}
        onOpenItems={vi.fn()}
      />,
    );

    expect(row()).toHaveTextContent("2 de 12");
    expect(row()).toHaveTextContent(ptBR.widgets.notice);
    expect(row().querySelector(".warning-dot")).not.toBeNull();
  });

  it("says Indisponível, marked, while the storage can't be used", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        widgets={widgets(0, { state: "unavailable", reason: "openFailed" })}
        onOpenItems={vi.fn()}
      />,
    );

    expect(row()).toHaveTextContent(items.unavailable);
    expect(row()).not.toHaveTextContent("de 12");
    expect(row()).toHaveTextContent(ptBR.widgets.notice);
  });

  it("says Indisponível, unmarked, while the widgets can't be read", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        widgets={null}
        onOpenItems={vi.fn()}
      />,
    );

    expect(row()).toHaveTextContent(items.unavailable);
    expect(row()).not.toHaveTextContent(ptBR.widgets.notice);
  });

  it("takes the focus coming back from the subview", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        widgets={widgets(1)}
        onOpenItems={vi.fn()}
        focusItemsRow
      />,
    );

    expect(row()).toHaveFocus();
  });

  it("leaves the focus alone otherwise", () => {
    render(
      <SettingsPanel
        settings={SETTINGS}
        widgets={widgets(1)}
        onOpenItems={vi.fn()}
      />,
    );

    expect(row()).not.toHaveFocus();
  });

  it("has no row without a way to open the subview", () => {
    render(<SettingsPanel settings={SETTINGS} widgets={widgets(1)} />);

    expect(
      screen.queryByRole("button", { name: new RegExp(items.title) }),
    ).toBeNull();
  });
});
