import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CREATABLE_KINDS, type WidgetInstance } from "../core/widgets";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
import { ptBR } from "../i18n/pt-BR";
import DockItems from "../panel/DockItems";
import tokens from "../themes/tokens.css?raw";
import { widgetName } from "./names";
import { WIDGET_REGISTRY, definitionOf, isCreatableInstance } from "./registry";

const INSTANCES = {
  clock: WIDGET_FIXTURES["clock-sample"],
  "time-progress": WIDGET_FIXTURES["time-progress-default"],
  note: WIDGET_FIXTURES["note-default"],
} as const;

// 2026-10-15 18:07:05 UTC, a Thursday.
const NOW = new Date(Date.UTC(2026, 9, 15, 18, 7, 5));

describe("widget registry", () => {
  it("has exactly the creatable kinds", () => {
    expect(Object.keys(WIDGET_REGISTRY).sort()).toEqual(
      [...CREATABLE_KINDS].sort(),
    );
  });

  it.each(CREATABLE_KINDS)("ties %s to its name and glaze", (kind) => {
    const definition = WIDGET_REGISTRY[kind];
    expect(definition.kind).toBe(kind);
    expect(ptBR.widgets.kinds[definition.nameKey]).toBeTruthy();
    expect(ptBR.widgets.addNames[definition.nameKey]).toBeTruthy();
    // A static class of tokens.css, with the hue, saturation and lightness.
    expect(tokens).toMatch(
      new RegExp(`\\.${definition.glaze} \\{\\s*--h: \\d+;`),
    );
  });

  it("gives each kind its own glaze", () => {
    const glazes = Object.values(WIDGET_REGISTRY).map((d) => d.glaze);
    expect(new Set(glazes).size).toBe(glazes.length);
  });

  it("finds the definition of an instance's kind", () => {
    for (const instance of Object.values(INSTANCES)) {
      expect(definitionOf(instance).kind).toBe(instance.kind);
    }
  });

  it("tells the kinds the dock shows from the others", () => {
    expect(isCreatableInstance(INSTANCES.note)).toBe(true);
    expect(isCreatableInstance(WIDGET_FIXTURES["shortcut-url"])).toBe(false);
    expect(isCreatableInstance(WIDGET_FIXTURES["files-sample"])).toBe(false);
  });
});

describe("widgetName", () => {
  it("is the label, or the kind's name without one", () => {
    expect(widgetName(ptBR, INSTANCES.clock)).toBe("São Paulo");
    expect(widgetName(ptBR, INSTANCES.note)).toBe("Nota");
    expect(widgetName(ptBR, INSTANCES["time-progress"])).toBe(
      "Progresso do tempo",
    );
  });
});

/* The provisional components meet the contract the lanes implement: the
 * host can mount each of them with its props. */
describe.each(CREATABLE_KINDS)("the %s components", (kind) => {
  const instance: WidgetInstance = INSTANCES[kind];

  it("render a tile without a name of its own", () => {
    const { Tile } = definitionOf(instance);
    const { container } = render(<Tile instance={instance} now={NOW} />);

    expect(container.firstElementChild).toHaveAttribute("aria-hidden", "true");
  });

  it("render the main view, with Ajustes", () => {
    const { Panel } = definitionOf(instance);
    const onAdjust = vi.fn();
    render(
      <Panel
        instance={instance}
        now={NOW}
        onAdjust={onAdjust}
        returnedFromSettings
      />,
    );

    expect(
      screen.getByRole("heading", { name: widgetName(ptBR, instance) }),
    ).toBeInTheDocument();
    const adjust = screen.getByRole("button", { name: "Ajustes" });
    expect(adjust).toHaveFocus();
    fireEvent.click(adjust);
    expect(onAdjust).toHaveBeenCalledTimes(1);
  });

  it("render the settings view, focused on Voltar", () => {
    const { Settings } = definitionOf(instance);
    const onBack = vi.fn();
    render(
      <Settings
        instance={instance}
        onUpdate={() => Promise.resolve()}
        onRemove={() => Promise.resolve()}
        onBack={onBack}
      />,
    );

    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});

describe("DockItems (provisional)", () => {
  it("is the Itens do dock subview, focused on Voltar", () => {
    const onBack = vi.fn();
    render(
      <DockItems
        snapshot={WIDGET_FIXTURES["snapshot-sample"]}
        onBack={onBack}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Itens do dock" }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});
