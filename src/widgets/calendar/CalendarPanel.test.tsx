import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CalendarEvent, CalendarView } from "../../core/calendar";
import { clockText, plainText } from "../../core/calendar";
import type { WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import CalendarPanel from "./CalendarPanel";

const text = ptBR.widgets.calendar;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const BRT = -3 * 3600;
// 2026-10-28 09:00 in Brasília.
const NOW = 1_793_188_800;

function event(partial: Partial<CalendarEvent>): CalendarEvent {
  return {
    id: "e",
    calendar: "max@example.com",
    calendarName: "Max",
    color: "#9fc6e7",
    title: "Daily",
    start: NOW + 30 * 60,
    end: NOW + 45 * 60,
    allDay: false,
    location: null,
    description: null,
    join: "https://meet.google.com/abc",
    link: "https://www.google.com/calendar/event?eid=x",
    ...partial,
  };
}

function instance(hidden: string[] = []): WidgetInstance<"calendar"> {
  return {
    id: "c",
    label: null,
    kind: "calendar",
    config: { width: 2, hidden },
  };
}

function answer(view: CalendarView) {
  invoke.mockImplementation((command: string) =>
    command === "calendar_events"
      ? Promise.resolve(view)
      : Promise.resolve(undefined),
  );
}

function view(events: CalendarEvent[], status = "ok"): CalendarView {
  return {
    snapshot: {
      status: status as "ok",
      events,
      sources: [],
      offset: BRT,
    },
    now: NOW,
  };
}

function show(config = instance()) {
  render(
    <CalendarPanel
      instance={config}
      now={new Date(NOW * 1000)}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

beforeEach(() => {
  invoke.mockReset();
});

describe("CalendarPanel", () => {
  it("lists today and tomorrow, with Entrar on a meeting", async () => {
    answer(
      view([
        event({ id: "a", title: "Daily" }),
        event({
          id: "b",
          title: "Dentista",
          start: NOW + 24 * 3600,
          end: NOW + 25 * 3600,
          join: null,
        }),
        event({
          id: "c",
          title: "Feriado",
          allDay: true,
          start: NOW - 9 * 3600,
          end: NOW + 15 * 3600,
          join: null,
        }),
      ]),
    );
    show();

    const today = await screen.findByRole("region", { name: text.today });
    expect(today).toHaveTextContent("Feriado");
    expect(today).toHaveTextContent("09:30");
    expect(
      screen.getByRole("region", { name: text.tomorrow }),
    ).toHaveTextContent("Dentista");
    expect(invoke).toHaveBeenCalledWith("calendar_events", { fresh: true });

    fireEvent.click(screen.getByRole("button", { name: "Entrar em Daily" }));
    expect(invoke).toHaveBeenCalledWith("open_calendar_link", {
      url: "https://meet.google.com/abc",
    });
  });

  it("leaves out the hidden calendars and past meetings", async () => {
    answer(
      view([
        event({ id: "a", title: "Plantão", calendar: "noiva" }),
        event({ id: "b", title: "Foi", start: NOW - 3600, end: NOW - 1800 }),
      ]),
    );
    show(instance(["noiva"]));

    const today = await screen.findByRole("region", { name: text.today });
    expect(today).toHaveTextContent(text.free);
    expect(screen.queryByText("Plantão")).toBeNull();
    expect(screen.queryByText("Foi")).toBeNull();
  });

  it("opens an event's details and comes back", async () => {
    answer(
      view([
        event({
          title: "Revisão",
          location: "Sala 2",
          description: "<p>Pauta</p><br>1. código",
        }),
      ]),
    );
    show();

    fireEvent.click(await screen.findByRole("button", { name: /^09:30/ }));
    expect(
      screen.getByRole("heading", { name: "Revisão" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Sala 2")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: text.openInGoogle }));
    expect(invoke).toHaveBeenCalledWith("open_calendar_link", {
      url: "https://www.google.com/calendar/event?eid=x",
    });
    fireEvent.click(screen.getByRole("button", { name: "Voltar" }));
    expect(
      await screen.findByRole("region", { name: text.today }),
    ).toBeInTheDocument();
  });

  it("says when there is no account", async () => {
    answer(view([], "noAccounts"));
    show();

    expect(await screen.findByText(text.status.noAccounts)).toBeInTheDocument();
  });
});

describe("calendar texts", () => {
  it("writes local times and plain descriptions", () => {
    expect(clockText(NOW, BRT)).toBe("09:00");
    expect(clockText(NOW + 15 * 3600, BRT)).toBe("00:00");
    expect(plainText("<p>Pauta</p><br>a &amp; b")).toBe("Pauta\n\na & b");
  });
});
