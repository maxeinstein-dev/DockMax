import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ProgressPanel from "./ProgressPanel";
import { FRIDAY_NOON, SATURDAY_NOON, progressInstance } from "./testing";

function panel(
  instance = progressInstance(),
  now = FRIDAY_NOON,
  returnedFromSettings = false,
) {
  const onAdjust = vi.fn();
  const view = render(
    <ProgressPanel
      instance={instance}
      now={now}
      onAdjust={onAdjust}
      returnedFromSettings={returnedFromSettings}
    />,
  );
  return { ...view, onAdjust };
}

function rows() {
  return within(screen.getByRole("list")).getAllByRole("listitem");
}

describe("ProgressPanel", () => {
  it("has the instance's name as title and the Ajustes button", () => {
    const { onAdjust } = panel(progressInstance({}, "Trabalho"));

    expect(
      screen.getByRole("heading", { name: "Trabalho" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ajustes" }));
    expect(onAdjust).toHaveBeenCalledTimes(1);
  });

  it("names itself by kind without a label", () => {
    panel();
    expect(
      screen.getByRole("heading", { name: "Progresso do tempo" }),
    ).toBeInTheDocument();
  });

  it("focuses Ajustes only when back from the settings", () => {
    const { unmount } = panel();
    expect(screen.getByRole("button", { name: "Ajustes" })).not.toHaveFocus();
    unmount();

    panel(progressInstance(), FRIDAY_NOON, true);
    expect(screen.getByRole("button", { name: "Ajustes" })).toHaveFocus();
  });

  it("shows one line per period, in the config's order", () => {
    panel(progressInstance({ periods: ["month", "day", "workday"] }));

    const lines = rows();
    expect(lines).toHaveLength(3);
    expect(lines[0]).toHaveTextContent(/^Mês/);
    expect(lines[1]).toHaveTextContent(/^Dia/);
    expect(lines[2]).toHaveTextContent(/^Expediente/);
  });

  it("gives each running period its percentage and time left", () => {
    panel(progressInstance({ periods: ["day", "week", "workday"] }));

    const [day, week, workday] = rows();
    expect(day).toHaveTextContent("Dia50%faltam 12 h");
    // Monday 00:00 to Monday 00:00; Friday noon is 4.5 of 7 days.
    expect(week).toHaveTextContent("Semana64%faltam 2 dias 12 h");
    expect(workday).toHaveTextContent("Expediente33%faltam 6 h");
  });

  it("fills each bar through the CSSOM, never a style attribute in the markup", () => {
    const { container } = panel(progressInstance({ periods: ["day"] }));

    const fill = container.querySelector<HTMLElement>(".pg-fill");
    expect(fill).not.toBeNull();
    expect(Number(fill?.style.getPropertyValue("--f"))).toBeCloseTo(0.5, 4);
    expect(fill?.closest(".pg-bar")).toHaveAttribute("aria-hidden", "true");
  });

  it("updates the bars with the instant", () => {
    const instance = progressInstance({ periods: ["day"] });
    const { container, rerender, onAdjust } = panel(instance);
    rerender(
      <ProgressPanel
        instance={instance}
        now={new Date(2026, 9, 16, 18, 0)}
        onAdjust={onAdjust}
        returnedFromSettings={false}
      />,
    );

    const fill = container.querySelector<HTMLElement>(".pg-fill");
    expect(Number(fill?.style.getPropertyValue("--f"))).toBeCloseTo(0.75, 4);
    expect(rows()[0]).toHaveTextContent("Dia75%faltam 6 h");
  });

  it("says Fora do expediente, without bar or percentage, off the workday", () => {
    const { container } = panel(
      progressInstance({ periods: ["workday", "day"] }),
      SATURDAY_NOON,
    );

    const [workday, day] = rows();
    expect(workday).toHaveTextContent(/^ExpedienteFora do expediente$/);
    expect(workday?.querySelector(".pg-bar")).toBeNull();
    expect(day).toHaveTextContent("Dia50%");
    expect(container.querySelectorAll(".pg-bar")).toHaveLength(1);
  });

  it("says when the week starts, in the footer", () => {
    const { unmount } = panel();
    expect(screen.getByRole("contentinfo")).toHaveTextContent(
      "Semana começa na segunda",
    );
    unmount();

    panel(progressInstance({ weekStart: "sunday" }));
    expect(screen.getByRole("contentinfo")).toHaveTextContent(
      "Semana começa no domingo",
    );
  });
});
