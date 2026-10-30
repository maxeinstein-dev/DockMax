import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CalendarEvent,
  CalendarTask,
  CalendarView,
} from "../../core/calendar";
import { clockText, plainText, sortTasks } from "../../core/calendar";
import type { WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import CalendarPanel from "./CalendarPanel";
import css from "./calendar.css?raw";
import { expectSaidIn, findAlert, liveRegions } from "../../test/alerts";

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
    busy: false,
    ...partial,
  };
}

function instance(hidden: string[] = []): WidgetInstance<"calendar"> {
  return {
    id: "c",
    label: null,
    kind: "calendar",
    config: {
      width: 2,
      hidden,
      tasks: true,
      taskOrder: [],
      cardPeriod: "auto",
      cardTasks: true,
    },
  };
}

function answer(view: CalendarView) {
  invoke.mockImplementation((command: string) =>
    command === "calendar_events"
      ? Promise.resolve(view)
      : Promise.resolve(undefined),
  );
}

function view(
  events: CalendarEvent[],
  status = "ok",
  tasks: CalendarTask[] = [],
): CalendarView {
  return {
    snapshot: {
      status: status as "ok",
      events,
      sources: [],
      tasks,
      offset: BRT,
    },
    now: NOW,
    open: null,
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

  it("ticks off a late task and drops it from the list", async () => {
    const task: CalendarTask = {
      account: "max@example.com",
      list: "L1",
      listName: "Minhas tarefas",
      id: "t1",
      title: "Pagar boleto",
      notes: null,
      due: "2026-10-27",
      late: true,
      link: null,
    };
    invoke.mockImplementation((command: string) => {
      if (command === "calendar_events")
        return Promise.resolve(view([], "ok", [task]));
      if (command === "complete_calendar_task") return Promise.resolve();
      return Promise.resolve();
    });
    show();

    const box = await screen.findByRole("checkbox", {
      name: "Concluir Pagar boleto",
    });
    expect(screen.getByText(text.late)).toBeInTheDocument();
    fireEvent.click(box);
    expect(invoke).toHaveBeenCalledWith("complete_calendar_task", {
      account: "max@example.com",
      list: "L1",
      id: "t1",
    });
    expect(await screen.findByText(text.noTasks)).toBeInTheDocument();
  });

  it("brings a task back with a message when Google refuses", async () => {
    const task: CalendarTask = {
      account: "a",
      list: "L",
      id: "t",
      listName: "",
      title: "Ligar",
      notes: null,
      due: "2026-10-28",
      late: false,
      link: null,
    };
    invoke.mockImplementation((command: string) =>
      command === "calendar_events"
        ? Promise.resolve(view([], "ok", [task]))
        : command === "complete_calendar_task"
          ? Promise.reject("no")
          : Promise.resolve(),
    );
    show();

    const done = await screen.findByRole("checkbox", {
      name: "Concluir Ligar",
    });
    const before = liveRegions();
    fireEvent.click(done);
    expect(await findAlert()).toHaveTextContent(text.completeFailed);
    expectSaidIn(before, await findAlert());
    expect(
      screen.getByRole("checkbox", { name: "Concluir Ligar" }),
    ).toBeInTheDocument();
  });

  it("hides the tasks when the widget says so", async () => {
    answer(view([], "ok"));
    render(
      <CalendarPanel
        instance={{
          ...instance(),
          config: {
            width: 2,
            hidden: [],
            tasks: false,
            taskOrder: [],
            cardPeriod: "auto",
            cardTasks: true,
          },
        }}
        now={new Date(NOW * 1000)}
        onAdjust={() => undefined}
        returnedFromSettings={false}
      />,
    );
    await screen.findByRole("region", { name: text.today });
    expect(screen.queryByRole("region", { name: text.tasks })).toBeNull();
  });

  it("opens on the event clicked on the card over the dock", async () => {
    invoke.mockImplementation((command: string) =>
      command === "calendar_events"
        ? Promise.resolve({
            ...view([event({ id: "x", title: "Sprint review" })]),
            open: "x",
          })
        : Promise.resolve(),
    );
    show();

    expect(
      await screen.findByRole("heading", { name: "Sprint review" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Voltar" })).toBeInTheDocument();
  });

  it("names an event with no title: busy or untitled", async () => {
    answer(
      view([
        event({ id: "a", title: "", busy: true, join: null }),
        event({ id: "b", title: "", start: NOW + 3600, end: NOW + 7200 }),
      ]),
    );
    show();

    expect(await screen.findByText(text.busy)).toBeInTheDocument();
    expect(screen.getByText(text.noTitle)).toBeInTheDocument();
  });

  it("never grows wider than its card for a long title", () => {
    // jsdom has no layout: the rules that keep a long title inside are
    // checked as text. Without them the list widened the panel, cutting
    // the title and pushing Ajustes out of the card.
    expect(css).toMatch(
      /\.cal-list \{[^}]*grid-template-columns: minmax\(0, 1fr\)/,
    );
    expect(css).toMatch(/\.cal-list li \{[^}]*min-width: 0/);
    expect(css).toMatch(/\.cal-title \{[^}]*-webkit-line-clamp: 2/);
    // The Ajustes view: a fieldset is as wide as its longest e-mail unless
    // told otherwise.
    expect(css).toMatch(/\.cal-sources \{[^}]*min-width: 0/);
    expect(css).toMatch(/\.cal-source-account \{[^}]*text-overflow: ellipsis/);
  });

  it("says when there is no account", async () => {
    answer(view([], "noAccounts"));
    show();

    expect(await screen.findByText(text.status.noAccounts)).toBeInTheDocument();
  });
});

describe("task order", () => {
  const task = (title: string, id = title): CalendarTask => ({
    account: "a",
    list: "L",
    listName: "",
    id,
    title,
    notes: null,
    due: "2026-10-28",
    late: false,
    link: null,
  });

  it("puts the titles ordered in the panel first, then Google's", () => {
    const google = ["Regar", "Ferro", "Própolis", "Escovar"].map((t) =>
      task(t),
    );
    expect(
      sortTasks(google, ["Própolis", "Ferro"]).map((each) => each.title),
    ).toEqual(["Própolis", "Ferro", "Regar", "Escovar"]);
    expect(sortTasks(google, []).map((each) => each.title)).toEqual([
      "Regar",
      "Ferro",
      "Própolis",
      "Escovar",
    ]);
  });

  it("moves a task down and keeps the whole order shown", async () => {
    const onUpdate = vi.fn(() => Promise.resolve());
    invoke.mockImplementation((command: string) =>
      command === "calendar_events"
        ? Promise.resolve(
            view([], "ok", [task("Regar"), task("Ferro"), task("Própolis")]),
          )
        : Promise.resolve(),
    );
    render(
      <CalendarPanel
        instance={instance()}
        now={new Date(NOW * 1000)}
        onAdjust={() => undefined}
        returnedFromSettings={false}
        onUpdate={onUpdate}
      />,
    );

    fireEvent.click(
      await screen.findByRole("button", { name: "Descer Regar" }),
    );
    expect(onUpdate).toHaveBeenCalledWith({
      label: null,
      config: expect.objectContaining({
        taskOrder: ["Ferro", "Regar", "Própolis"],
      }),
    });
    // Until the saved order comes back, the first one can't go up.
    expect(screen.getByRole("button", { name: "Subir Regar" })).toBeDisabled();
  });
});

describe("calendar texts", () => {
  it("writes local times and plain descriptions", () => {
    expect(clockText(NOW, BRT)).toBe("09:00");
    expect(clockText(NOW + 15 * 3600, BRT)).toBe("00:00");
    expect(plainText("<p>Pauta</p><br>a &amp; b")).toBe("Pauta\n\na & b");
  });
});
