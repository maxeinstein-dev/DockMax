import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { clockOf, type PomodoroView } from "../../core/pomodoro";
import type { PomodoroConfig, WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import PomodoroPanel from "./PomodoroPanel";
import PomodoroSettings from "./PomodoroSettings";

const text = ptBR.widgets.pomodoro;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const CONFIG: PomodoroConfig = {
  focus: 25,
  short: 5,
  long: 15,
  every: 4,
  autoStart: false,
  windows: false,
};

const pomodoro: WidgetInstance<"pomodoro"> = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6090",
  label: null,
  kind: "pomodoro",
  config: CONFIG,
};

function view(change: Partial<PomodoroView> = {}): PomodoroView {
  return {
    stage: "focus",
    left: 25 * 60_000,
    total: 25 * 60_000,
    running: false,
    focuses: 0,
    every: 4,
    ...change,
  };
}

function panel() {
  render(
    <PomodoroPanel
      instance={pomodoro}
      now={new Date()}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

beforeEach(() => {
  invoke.mockReset();
});

describe("clockOf", () => {
  it("writes minutes and seconds, rounded up", () => {
    expect(clockOf(25 * 60_000)).toBe("25:00");
    expect(clockOf(61_001)).toBe("01:02");
    expect(clockOf(0)).toBe("00:00");
  });
});

describe("PomodoroPanel", () => {
  it("shows a focus waiting to start, and starts it", async () => {
    invoke.mockImplementation((command: string) =>
      Promise.resolve(
        command === "pomodoro_act" ? view({ running: true }) : view(),
      ),
    );
    panel();
    expect(await screen.findByRole("timer")).toHaveTextContent("25:00");
    expect(screen.getByText(text.stages.focus)).toBeInTheDocument();
    expect(
      screen.getByText("0 de 4 focos até a pausa longa"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: text.start }));
    expect(invoke).toHaveBeenCalledWith("pomodoro_act", {
      widget: pomodoro.id,
      config: CONFIG,
      action: "start",
    });
    expect(
      await screen.findByRole("button", { name: text.pause }),
    ).toBeInTheDocument();
  });

  it("offers Continuar for a stage paused halfway", async () => {
    invoke.mockImplementation(() =>
      Promise.resolve(view({ stage: "short", left: 90_000, total: 300_000 })),
    );
    panel();
    expect(
      await screen.findByRole("button", { name: text.resume }),
    ).toBeInTheDocument();
    expect(screen.getByRole("timer")).toHaveTextContent("01:30");
    expect(screen.getByText(text.stages.short)).toBeInTheDocument();
  });

  it("skips and starts over", async () => {
    invoke.mockImplementation(() => Promise.resolve(view()));
    panel();
    fireEvent.click(await screen.findByRole("button", { name: text.skip }));
    fireEvent.click(screen.getByRole("button", { name: text.reset }));
    const actions = invoke.mock.calls
      .filter(([command]) => command === "pomodoro_act")
      .map(([, args]) => (args as { action: string }).action);
    expect(actions).toEqual(["skip", "reset"]);
  });
});

type OnUpdate = (update: WidgetUpdate<"pomodoro">) => Promise<void>;

describe("PomodoroSettings", () => {
  it("saves the times and the switches", () => {
    const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
    render(
      <PomodoroSettings
        instance={pomodoro}
        onUpdate={onUpdate}
        onRemove={() => Promise.resolve()}
        onBack={() => undefined}
      />,
    );
    fireEvent.change(screen.getByLabelText(text.focus), {
      target: { value: "50" },
    });
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({ focus: 50 }) as PomodoroConfig,
      }),
    );
    fireEvent.click(screen.getByRole("switch", { name: text.autoStart }));
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({
          focus: 50,
          autoStart: true,
        }) as PomodoroConfig,
      }),
    );
  });
});
