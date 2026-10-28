import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { todayOf } from "../../core/countdown";
import type { WaterConfig, WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import WaterPanel from "./WaterPanel";
import WaterSettings from "./WaterSettings";

const text = ptBR.widgets.water;

type OnUpdate = (update: WidgetUpdate<"water">) => Promise<void>;

/** 28/10/2026, 15h. */
const NOW = new Date(2026, 9, 28, 15);
const TODAY = todayOf(NOW);

function water(config: Partial<WaterConfig> = {}): WidgetInstance<"water"> {
  return {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6094",
    label: null,
    kind: "water",
    config: {
      goal: 8,
      cup: 250,
      remind: 0,
      count: 0,
      day: 0,
      last: null,
      ...config,
    },
  };
}

function panel(instance: WidgetInstance<"water">) {
  const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
  render(
    <WaterPanel
      instance={instance}
      now={NOW}
      onAdjust={() => undefined}
      returnedFromSettings={false}
      onUpdate={onUpdate}
    />,
  );
  return onUpdate;
}

describe("WaterPanel", () => {
  it("counts today's cups and adds one, with its time", () => {
    const onUpdate = panel(water({ count: 3, day: TODAY }));
    expect(screen.getByText("3 de 8 copos")).toBeInTheDocument();
    expect(screen.getByText("750 de 2.000 ml hoje")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "+ 1 copo (250 ml)" }));
    expect(onUpdate).toHaveBeenLastCalledWith({
      label: null,
      config: expect.objectContaining({
        count: 4,
        day: TODAY,
        last: Math.floor(NOW.getTime() / 1000),
      }) as WaterConfig,
    });
  });

  it("starts a new day at zero", () => {
    const onUpdate = panel(water({ count: 6, day: TODAY - 1 }));
    expect(screen.getByText("0 de 8 copos")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: text.undo })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "+ 1 copo (250 ml)" }));
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({
          count: 1,
          day: TODAY,
        }) as WaterConfig,
      }),
    );
  });

  it("undoes a cup without moving the last one", () => {
    const onUpdate = panel(water({ count: 2, day: TODAY, last: 123 }));
    fireEvent.click(screen.getByRole("button", { name: text.undo }));
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({ count: 1, last: 123 }) as WaterConfig,
      }),
    );
  });

  it("says when the goal is reached, and the reminder", () => {
    panel(water({ count: 8, day: TODAY, remind: 60 }));
    expect(screen.getByText(text.reached)).toBeInTheDocument();
    expect(
      screen.getByText("Lembrete depois de 60 min sem um copo, das 8h às 22h"),
    ).toBeInTheDocument();
  });
});

describe("WaterSettings", () => {
  it("turns the reminder on", () => {
    const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
    render(
      <WaterSettings
        instance={water()}
        onUpdate={onUpdate}
        onRemove={() => Promise.resolve()}
        onBack={() => undefined}
      />,
    );
    fireEvent.change(screen.getByLabelText(text.remind), {
      target: { value: "90" },
    });
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({ remind: 90 }) as WaterConfig,
      }),
    );
  });
});
