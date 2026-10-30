import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import { ptBR } from "../i18n/pt-BR";
import LookPanel from "./LookPanel";

const text = ptBR.settings.look;

const SETTINGS: Settings = {
  version: 1,
  theme: "dark",
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  launcherHotkey: "Alt+Space",
  language: "system",
  tourShown: true,
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

const flush = () => act(() => Promise.resolve());

function withDock(change: Partial<Settings["dock"]>): Settings {
  return { ...SETTINGS, dock: { ...SETTINGS.dock, ...change } };
}

function sentSettings(): Settings {
  const [command, args] = invoke.mock.calls[invoke.mock.calls.length - 1] as [
    string,
    { settings: Settings },
  ];
  expect(command).toBe("update_settings");
  return args.settings;
}

/** The saves sent: the view also asks for the icon packs when it opens. */
const saves = () =>
  invoke.mock.calls.filter(([command]) => command === "update_settings");

const group = (name: string) => screen.getByRole("group", { name });

function chosen(name: string): string[] {
  return within(group(name))
    .getAllByRole("button")
    .filter((button) => button.getAttribute("aria-pressed") === "true")
    .map((button) => button.getAttribute("aria-label") ?? button.textContent)
    .filter((label): label is string => label !== null);
}

/** A slider moved and let go, as the user does. */
function drag(name: string, value: number) {
  const slider = screen.getByRole("slider", { name });
  fireEvent.change(slider, { target: { value: String(value) } });
  return slider;
}

describe("LookPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockReturnValue(new Promise(() => {}));
    log.warn.mockReset();
  });

  it("names the view and goes back with Voltar, focused on arrival", () => {
    const onBack = vi.fn();
    render(<LookPanel settings={SETTINGS} onBack={onBack} />);

    expect(
      screen.getByRole("heading", { level: 1, name: text.title }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();

    fireEvent.click(back);

    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("chooses an icon pack and saves it with the other settings", async () => {
    invoke.mockImplementation((command: string) =>
      command === "list_icon_packs"
        ? Promise.resolve([
            {
              id: "cores",
              name: "Cores lisas",
              author: "",
              version: "",
              description: "",
              accent: null,
              iconCount: 4,
              covers: 0,
              total: 0,
              error: null,
            },
          ])
        : new Promise(() => {}),
    );
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);
    await flush();

    fireEvent.click(
      within(group(text.iconPack)).getByRole("button", { name: "Cores lisas" }),
    );

    expect(sentSettings().dock.iconPack).toBe("cores");
  });

  it("lists the styles in order, with the one in use chosen", () => {
    render(
      <LookPanel settings={withDock({ style: "neon" })} onBack={vi.fn()} />,
    );

    const cards = within(group(text.style)).getAllByRole("button");
    expect(cards.map((card) => card.textContent)).toEqual([
      "Nativo",
      "Liquid Glass",
      "Meia-noite",
      "Rocket",
      "Prateleira Aqua",
      "Neon",
      "Papel",
      "Flutuante",
      "Retrô",
      "Clássico 98",
      "Fósforo",
      "Tático",
    ]);
    expect(chosen(text.style)).toEqual(["Neon"]);
  });

  it("leaves the border and the corners out for a skin, its image has them", () => {
    render(
      <LookPanel
        settings={withDock({ style: "classic98" })}
        onBack={vi.fn()}
      />,
    );

    expect(screen.queryByRole("switch", { name: text.border })).toBeNull();
    expect(screen.queryByRole("slider", { name: text.corners })).toBeNull();
    expect(
      screen.getByRole("switch", { name: text.reflections }),
    ).toBeInTheDocument();
  });

  it("saves the full settings on choosing a style", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    fireEvent.click(
      within(group(text.style)).getByRole("button", {
        name: "Prateleira Aqua",
      }),
    );

    expect(saves()).toHaveLength(1);
    expect(sentSettings()).toEqual(withDock({ style: "aquaShelf" }));
  });

  it("shows the look the style has, and the changes over it", () => {
    const { rerender } = render(
      <LookPanel settings={withDock({ style: "floating" })} onBack={vi.fn()} />,
    );

    // Flutuante: no outline, 40 % corners, with reflections.
    expect(screen.getByRole("switch", { name: text.border })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(
      screen.getByRole("switch", { name: text.reflections }),
    ).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("slider", { name: text.corners })).toHaveValue(
      "40",
    );

    rerender(
      <LookPanel
        settings={withDock({
          style: "floating",
          tweaks: {
            floating: { border: true, corners: 90, reflections: false },
          },
        })}
        onBack={vi.fn()}
      />,
    );

    expect(screen.getByRole("switch", { name: text.border })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(
      screen.getByRole("switch", { name: text.reflections }),
    ).toHaveAttribute("aria-checked", "false");
    expect(screen.getByRole("slider", { name: text.corners })).toHaveValue(
      "90",
    );
  });

  it("saves a switch as a change of the chosen style only", () => {
    render(
      <LookPanel
        settings={withDock({
          style: "rocket",
          tweaks: { neon: { corners: 10 } },
        })}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("switch", { name: text.border }));

    expect(sentSettings().dock.tweaks).toEqual({
      neon: { corners: 10 },
      rocket: { border: false },
    });
  });

  it("keeps the earlier changes of the style when it changes another", () => {
    render(
      <LookPanel
        settings={withDock({
          tweaks: { native: { corners: 30 } },
        })}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("switch", { name: text.reflections }));

    expect(sentSettings().dock.tweaks).toEqual({
      native: { corners: 30, reflections: true },
    });
  });

  it("saves a slider once it is let go, not at every step of the drag", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    const slider = drag(text.corners, 20);
    drag(text.corners, 30);

    // The value follows the pointer on the screen, nothing is saved yet.
    expect(slider).toHaveValue("30");
    expect(screen.getByText("30%")).toBeInTheDocument();
    expect(saves()).toHaveLength(0);

    fireEvent.pointerUp(slider);

    expect(saves()).toHaveLength(1);
    expect(sentSettings().dock.tweaks).toEqual({ native: { corners: 30 } });
  });

  it("saves a slider moved by keyboard when the key is released", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    const slider = drag(text.corners, 70);
    fireEvent.keyUp(slider, { key: "ArrowRight" });

    expect(sentSettings().dock.tweaks).toEqual({ native: { corners: 70 } });
  });

  it("does not save a slider let go where it was", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    fireEvent.pointerUp(screen.getByRole("slider", { name: text.corners }));
    fireEvent.keyUp(screen.getByRole("slider", { name: text.corners }), {
      key: "Tab",
    });

    expect(saves()).toHaveLength(0);
  });

  it("restores the style: drops its changes and no other", () => {
    render(
      <LookPanel
        settings={withDock({
          style: "neon",
          tweaks: { neon: { corners: 10 }, native: { border: false } },
        })}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: text.restore }));

    expect(sentSettings().dock.tweaks).toEqual({ native: { border: false } });
  });

  it("cannot restore a style that has no changes", () => {
    render(
      <LookPanel
        settings={withDock({ tweaks: { neon: { corners: 10 } } })}
        onBack={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: text.restore })).toBeDisabled();
  });

  it("offers the three hover effects and saves the one chosen", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    expect(
      within(group(text.hover))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Nenhum", "Escala", "Ampliação"]);
    expect(chosen(text.hover)).toEqual(["Escala"]);

    fireEvent.click(
      within(group(text.hover)).getByRole("button", { name: "Ampliação" }),
    );

    expect(sentSettings().dock.hover).toBe("magnify");
  });

  it("asks for the scale only with the Escala effect", () => {
    const { rerender } = render(
      <LookPanel settings={SETTINGS} onBack={vi.fn()} />,
    );

    expect(
      screen.getByRole("slider", { name: text.hoverScale }),
    ).toHaveAttribute("aria-valuetext", "1,10×");

    for (const hover of ["none", "magnify"] as const) {
      rerender(<LookPanel settings={withDock({ hover })} onBack={vi.fn()} />);
      expect(
        screen.queryByRole("slider", { name: text.hoverScale }),
      ).not.toBeInTheDocument();
    }
  });

  it("saves the scale in percent", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    const slider = drag(text.hoverScale, 125);
    fireEvent.pointerUp(slider);

    expect(sentSettings().dock.hoverScale).toBe(125);
    expect(screen.getByRole("slider", { name: text.hoverScale })).toHaveValue(
      "125",
    );
  });

  it("offers the two indicators and saves the one chosen", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    expect(chosen(text.indicator)).toEqual(["Ponto"]);

    fireEvent.click(
      within(group(text.indicator)).getByRole("button", { name: "Traço" }),
    );

    expect(sentSettings().dock.indicator).toBe("dash");
  });

  it("offers the three icon shapes and saves the one chosen", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    expect(chosen(text.iconShape)).toEqual(["Placas arredondadas"]);
    expect(
      within(group(text.iconShape))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Placas arredondadas", "Mesmo tamanho", "Como vieram"]);

    fireEvent.click(
      within(group(text.iconShape)).getByRole("button", {
        name: "Mesmo tamanho",
      }),
    );

    expect(sentSettings().dock.iconShape).toBe("uniform");
  });

  it("offers the three opening animations and saves the one chosen", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    expect(chosen(text.openAnimation)).toEqual(["Pulo"]);
    expect(
      within(group(text.openAnimation))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Pulo", "Pulso", "Tremor"]);

    fireEvent.click(
      within(group(text.openAnimation)).getByRole("button", {
        name: "Tremor",
      }),
    );

    expect(sentSettings().dock.openAnimation).toBe("shake");
  });

  it("offers the style's own color and the palette, and saves the color", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    const swatches = within(group(text.indicatorColor)).getAllByRole("button");
    expect(swatches).toHaveLength(19);
    expect(swatches[0]).toHaveTextContent(text.indicatorDefault);
    expect(chosen(text.indicatorColor)).toEqual([text.indicatorDefault]);

    fireEvent.click(
      within(group(text.indicatorColor)).getByRole("button", { name: "Azul" }),
    );

    expect(sentSettings().dock.indicatorColor).toBe("#0A84FF");
  });

  it("goes back to the style's own color", () => {
    render(
      <LookPanel
        settings={withDock({ indicatorColor: "#ff453a" })}
        onBack={vi.fn()}
      />,
    );

    // A color written by hand in lower case matches its swatch.
    expect(chosen(text.indicatorColor)).toEqual(["Vermelho"]);

    fireEvent.click(
      within(group(text.indicatorColor)).getByRole("button", {
        name: text.indicatorDefault,
      }),
    );

    expect(sentSettings().dock.indicatorColor).toBeNull();
  });

  it("moves between the swatches with the arrows, one Tab stop", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    const swatches = within(group(text.indicatorColor)).getAllByRole("button");
    expect(swatches.filter((s) => s.tabIndex === 0)).toEqual([swatches[0]]);

    swatches[0]?.focus();
    fireEvent.keyDown(swatches[0] as HTMLElement, { key: "ArrowRight" });
    expect(swatches[1]).toHaveFocus();
    fireEvent.keyDown(swatches[1] as HTMLElement, { key: "End" });
    expect(swatches[swatches.length - 1]).toHaveFocus();
    fireEvent.keyDown(swatches[swatches.length - 1] as HTMLElement, {
      key: "ArrowRight",
    });
    expect(swatches[0]).toHaveFocus();
    // Choosing never happens by moving.
    expect(saves()).toHaveLength(0);
  });

  it("shows the translated reason when Rust refuses a value, and goes back", async () => {
    invoke.mockReset();
    invoke.mockRejectedValue({ code: "lookOutOfRange" });
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    fireEvent.click(
      within(group(text.indicator)).getByRole("button", { name: "Traço" }),
    );
    await flush();

    expect(screen.getByRole("alert")).toHaveTextContent(
      ptBR.settings.errors.lookOutOfRange,
    );
    expect(chosen(text.indicator)).toEqual(["Ponto"]);
  });

  it("cannot change anything while the settings can't be read", () => {
    render(<LookPanel settings={null} onBack={vi.fn()} />);

    for (const card of within(group(text.style)).getAllByRole("button")) {
      expect(card).toBeDisabled();
    }
    expect(screen.getByRole("switch", { name: text.border })).toBeDisabled();
    expect(screen.getByRole("slider", { name: text.corners })).toBeDisabled();
    for (const swatch of within(group(text.indicatorColor)).getAllByRole(
      "button",
    )) {
      expect(swatch).toBeDisabled();
    }
  });

  it("paints each swatch through a custom property, never an inline style", () => {
    render(<LookPanel settings={SETTINGS} onBack={vi.fn()} />);

    const blue = within(group(text.indicatorColor)).getByRole("button", {
      name: "Azul",
    });

    expect(blue.style.getPropertyValue("--swatch")).toBe("#0A84FF");
    expect(blue.style.length).toBe(1);
  });
});
