import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CountdownConfig, WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import CountdownPanel from "./CountdownPanel";
import CountdownSettings from "./CountdownSettings";

const text = ptBR.widgets.countdown;

function countdown(
  config: Partial<CountdownConfig> = {},
  label: string | null = "Casamento",
): WidgetInstance<"countdown"> {
  return {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6089",
    label,
    kind: "countdown",
    config: { date: "2026-12-20", yearly: false, ...config },
  };
}

/** 28/10/2026, at noon. */
const NOW = new Date(2026, 9, 28, 12);

function panel(instance: WidgetInstance<"countdown">) {
  render(
    <CountdownPanel
      instance={instance}
      now={NOW}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

describe("CountdownPanel", () => {
  it("says the days left and the day, under its name", () => {
    panel(countdown());
    expect(screen.getByText("Casamento")).toBeInTheDocument();
    expect(screen.getByText("Faltam 53 dias")).toBeInTheDocument();
    expect(
      screen.getByText("domingo, 20 de dezembro de 2026"),
    ).toBeInTheDocument();
  });

  it("says the day itself, and the days gone", () => {
    panel(countdown({ date: "2026-10-28" }));
    expect(screen.getByText(text.today)).toBeInTheDocument();
  });

  it("counts the days gone once it passed", () => {
    panel(countdown({ date: "2026-10-27" }));
    expect(screen.getByText("Foi há 1 dia")).toBeInTheDocument();
  });

  it("goes to the next one when yearly", () => {
    panel(countdown({ date: "1990-03-10", yearly: true }));
    expect(screen.getByText("Faltam 133 dias")).toBeInTheDocument();
    expect(screen.getByText(text.everyYear)).toBeInTheDocument();
  });

  it("asks for a date without one", () => {
    panel(countdown({ date: null }, null));
    expect(screen.getByText(text.noDate)).toBeInTheDocument();
    expect(screen.getByText("Contagem regressiva")).toBeInTheDocument();
  });
});

type OnUpdate = (update: WidgetUpdate<"countdown">) => Promise<void>;

function settings(instance: WidgetInstance<"countdown">) {
  const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
  render(
    <CountdownSettings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={() => Promise.resolve()}
      onBack={() => undefined}
    />,
  );
  return onUpdate;
}

describe("CountdownSettings", () => {
  it("saves a new date once the field is done", () => {
    const onUpdate = settings(countdown());
    const field = screen.getByLabelText(text.date);
    fireEvent.change(field, { target: { value: "2027-05-22" } });
    fireEvent.blur(field);
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: { date: "2027-05-22", yearly: false },
      }),
    );
  });

  it("clears the date when the field is emptied", () => {
    const onUpdate = settings(countdown());
    const field = screen.getByLabelText(text.date);
    fireEvent.change(field, { target: { value: "" } });
    fireEvent.blur(field);
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({ config: { date: null, yearly: false } }),
    );
  });

  it("turns the yearly count on", () => {
    const onUpdate = settings(countdown());
    fireEvent.click(screen.getByRole("switch", { name: text.yearly }));
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: { date: "2026-12-20", yearly: true },
      }),
    );
  });
});
