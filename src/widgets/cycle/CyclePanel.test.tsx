import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CycleStatus, CycleView } from "../../core/cycle";
import { dayText } from "../../core/cycle";
import type { WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import CyclePanel from "./CyclePanel";

const text = ptBR.widgets.cycle;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const TODAY = 20_389; // 2025-10-28

const instance: WidgetInstance<"cycle"> = {
  id: "c",
  label: null,
  kind: "cycle",
  config: {
    symbol: "🌸",
    calendar: "noiva@example.com",
    sensitive: 5,
    notices: { sensitive: true, eve: true, day: true },
    reminders: [],
  },
};

function status(phase: CycleStatus["phase"], cycles = 6): CycleStatus {
  return {
    phase,
    averages: { cycle: 28.4, period: 5, cycles },
    day: 12,
    last: TODAY - 11,
    next: TODAY + 17,
    marks: 7,
  };
}

function answer(view: CycleView) {
  invoke.mockImplementation(() => Promise.resolve(view));
}

function show() {
  render(
    <CyclePanel
      instance={instance}
      now={new Date()}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

beforeEach(() => {
  invoke.mockReset();
});

describe("CyclePanel", () => {
  it("says where the cycle is, the next start and the averages", async () => {
    answer({
      status: status({ phase: "between", in: 17 }),
      today: TODAY,
      accounts: true,
      notices: [],
    });
    show();

    expect(
      await screen.findByText("Próxima menstruação em 17 dias"),
    ).toBeInTheDocument();
    expect(screen.getByText(dayText(TODAY + 17))).toBeInTheDocument();
    expect(screen.getByText("28 dias")).toBeInTheDocument();
    expect(
      screen.getByText("Com base nos últimos 6 ciclos."),
    ).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("cycle_status", {
      config: instance.config,
    });
    expect(screen.queryByText(text.today)).not.toBeInTheDocument();
  });

  it("lists today's notices, with the reminders' text", async () => {
    answer({
      status: status({ phase: "sensitive", in: 5 }),
      today: TODAY,
      accounts: true,
      notices: [
        { kind: "sensitive" },
        { kind: "reminder", text: "Comprar chocolate" },
      ],
    });
    show();
    const today = await screen.findByRole("region", { name: text.today });
    expect(today).toHaveTextContent(text.notices.sensitive);
    expect(today).toHaveTextContent("Comprar chocolate");
  });

  it("names the period's day and the sensitive window", async () => {
    answer({
      status: status({ phase: "period", day: 2 }),
      today: TODAY,
      accounts: true,
      notices: [],
    });
    show();
    expect(await screen.findByText("Menstruação · dia 2")).toBeInTheDocument();
  });

  it("says the window's days, one or many", async () => {
    answer({
      status: status({ phase: "sensitive", in: 1 }),
      today: TODAY,
      accounts: true,
      notices: [],
    });
    show();
    expect(
      await screen.findByText("TPM · a menstruação deve vir em 1 dia"),
    ).toBeInTheDocument();
  });

  it("explains the defaults without marks enough, and asks for an account", async () => {
    answer({
      status: status({ phase: "unknown" }, 0),
      today: TODAY,
      accounts: true,
      notices: [],
    });
    show();
    expect(await screen.findByText(text.unknown)).toBeInTheDocument();
    expect(screen.getByText(/usa 28 dias de ciclo/)).toBeInTheDocument();
  });

  it("asks for a Google account when there is none", async () => {
    answer({ status: null, today: TODAY, accounts: false, notices: [] });
    show();
    expect(await screen.findByText(text.noAccounts)).toBeInTheDocument();
  });
});

describe("cycle dates", () => {
  it("writes a local day number as day/month", () => {
    expect(dayText(TODAY)).toBe("28/10");
  });
});
