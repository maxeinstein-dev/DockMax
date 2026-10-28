import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CycleConfig, WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import CycleSettings from "./CycleSettings";

const text = ptBR.widgets.cycle;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => undefined),
}));

const CONFIG: CycleConfig = {
  symbol: "🌸",
  calendar: null,
  sensitive: 5,
  notices: { sensitive: true, eve: true, day: true, windows: true },
  reminders: [{ text: "Paciência", anchor: "period", days: 1 }],
};

const cycle: WidgetInstance<"cycle"> = {
  id: "c",
  label: null,
  kind: "cycle",
  config: CONFIG,
};

type OnUpdate = (update: WidgetUpdate<"cycle">) => Promise<void>;

function show(instance = cycle) {
  const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
  render(
    <CycleSettings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={() => Promise.resolve()}
      onBack={() => undefined}
    />,
  );
  return onUpdate;
}

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(() => Promise.resolve(null));
});

describe("CycleSettings", () => {
  it("turns the Windows notice on", () => {
    const onUpdate = show({
      ...cycle,
      config: { ...CONFIG, notices: { ...CONFIG.notices, windows: false } },
    });
    fireEvent.click(
      screen.getByRole("switch", { name: text.noticeToggles.windows }),
    );
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({
          notices: { sensitive: true, eve: true, day: true, windows: true },
        }) as CycleConfig,
      }),
    );
  });

  it("says when Windows has the notices off", async () => {
    invoke.mockImplementation(() => Promise.reject("notificationsOff"));
    show();
    fireEvent.click(screen.getByRole("button", { name: text.testNotice }));
    expect(await screen.findByRole("status")).toHaveTextContent(
      text.testResult.off,
    );
  });

  it("turns a Windows notice off", () => {
    const onUpdate = show();
    fireEvent.click(
      screen.getByRole("switch", { name: text.noticeToggles.eve }),
    );
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({
          notices: { sensitive: true, eve: false, day: true, windows: true },
        }) as CycleConfig,
      }),
    );
  });

  it("lists, adds and removes reminders", () => {
    const onUpdate = show();
    expect(screen.getByText("Paciência")).toBeInTheDocument();
    expect(screen.getByText("1 dia depois da menstruação")).toBeInTheDocument();

    const add = screen.getByRole("button", { name: text.addReminder });
    expect(add).toBeDisabled();
    fireEvent.change(screen.getByLabelText(text.reminderText), {
      target: { value: " Comprar chocolate " },
    });
    fireEvent.change(screen.getByLabelText(text.reminderDaysLabel), {
      target: { value: "-2" },
    });
    fireEvent.click(add);
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: expect.objectContaining({
          reminders: [
            CONFIG.reminders[0],
            { text: "Comprar chocolate", anchor: "sensitive", days: -2 },
          ],
        }) as CycleConfig,
      }),
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Remover o lembrete Paciência" }),
    );
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        // The one just added stays.
        config: expect.objectContaining({
          reminders: [
            { text: "Comprar chocolate", anchor: "sensitive", days: -2 },
          ],
        }) as CycleConfig,
      }),
    );
  });

  it("refuses a reminder longer than Rust takes", () => {
    show();
    fireEvent.change(screen.getByLabelText(text.reminderText), {
      target: { value: "x".repeat(121) },
    });
    expect(
      screen.getByRole("button", { name: text.addReminder }),
    ).toBeDisabled();
  });

  it("sends a test notice and says so", async () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: text.testNotice }));
    expect(invoke).toHaveBeenCalledWith("cycle_test_notice", {
      widget: "c",
      windows: true,
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      text.testResult.sent,
    );
  });
});
