import { fireEvent, render, screen } from "@testing-library/react";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import type { ClockConfig, WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import ClockPanel from "./ClockPanel";

// `src` doesn't load the Node types; only `TZ` is needed here.
declare const process: { env: Record<string, string | undefined> };

/** Runs the tests of the enclosing `describe` with `timeZone` as the
 * system zone (Node re-reads `TZ` when it changes). */
function inZone(timeZone: string): void {
  const before = process.env.TZ;
  const system = new Intl.DateTimeFormat().resolvedOptions().timeZone;
  beforeAll(() => {
    process.env.TZ = timeZone;
  });
  afterAll(() => {
    process.env.TZ = before ?? system;
  });
}

// 2026-10-15 18:07:05 UTC = 15:07:05 in São Paulo, a Thursday.
const NOW = new Date(Date.UTC(2026, 9, 15, 18, 7, 5));

const clock = (
  config: Partial<ClockConfig> = {},
  label: string | null = null,
): WidgetInstance<"clock"> => ({
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6071",
  label,
  kind: "clock",
  config: {
    timeZone: "America/Sao_Paulo",
    hourCycle: "h23",
    showSeconds: false,
    iconStyle: "digital",
    ...config,
  },
});

function renderPanel(
  instance: WidgetInstance<"clock">,
  { now = NOW, returnedFromSettings = false, onAdjust = () => {} } = {},
) {
  return render(
    <ClockPanel
      instance={instance}
      now={now}
      onAdjust={onAdjust}
      returnedFromSettings={returnedFromSettings}
    />,
  );
}

const time = (container: HTMLElement) =>
  container.querySelector(".clock-time")?.textContent;

describe("ClockPanel", () => {
  it("is titled by the label, or the kind's name", () => {
    const { unmount } = renderPanel(clock({}, "Escritório"));
    expect(
      screen.getByRole("heading", { name: "Escritório" }),
    ).toBeInTheDocument();
    unmount();

    renderPanel(clock());
    expect(
      screen.getByRole("heading", { name: "Relógio" }),
    ).toBeInTheDocument();
  });

  it("opens the settings from Ajustes, focused when coming back", () => {
    const onAdjust = vi.fn();
    renderPanel(clock(), { onAdjust, returnedFromSettings: true });

    const adjust = screen.getByRole("button", { name: "Ajustes" });
    expect(adjust).toHaveFocus();
    fireEvent.click(adjust);
    expect(onAdjust).toHaveBeenCalledTimes(1);
  });

  it("doesn't take the focus when opened", () => {
    renderPanel(clock());
    expect(screen.getByRole("button", { name: "Ajustes" })).not.toHaveFocus();
  });

  it("shows the time in 24 h, without seconds by default", () => {
    const { container } = renderPanel(clock());
    expect(time(container)).toBe("15:07");
    expect(container.querySelector(".clock-seconds")).toBeNull();
    expect(container.querySelector(".clock-period")).toBeNull();
  });

  it("shows the seconds when asked", () => {
    const { container } = renderPanel(clock({ showSeconds: true }));
    expect(time(container)).toBe("15:07:05");
    expect(container.querySelector(".clock-seconds")).toHaveTextContent(
      /^:05$/,
    );
  });

  it("shows AM/PM in 12 h", () => {
    const { container } = renderPanel(
      clock({ hourCycle: "h12", showSeconds: true }),
    );
    expect(time(container)).toBe("3:07:05PM");
    expect(container.querySelector(".clock-period")).toHaveTextContent(/^PM$/);
  });

  it("shows the long date in the clock's zone", () => {
    // 02:30 UTC on the 16th: still the 15th in São Paulo, the 16th in Tokyo.
    const late = new Date(Date.UTC(2026, 9, 16, 2, 30));
    const { unmount } = renderPanel(clock(), { now: late });
    expect(
      screen.getByText("quinta-feira, 15 de outubro de 2026"),
    ).toBeInTheDocument();
    unmount();

    renderPanel(clock({ timeZone: "Asia/Tokyo" }), { now: late });
    expect(
      screen.getByText("sexta-feira, 16 de outubro de 2026"),
    ).toBeInTheDocument();
  });

  it("names the zone and its offset in the footer", () => {
    renderPanel(clock({ timeZone: "Asia/Tokyo" }));
    expect(screen.getByText("Tokyo · UTC+9")).toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows the next second when `now` changes", () => {
    const instance = clock({ showSeconds: true });
    const { container, rerender } = renderPanel(instance);
    rerender(
      <ClockPanel
        instance={instance}
        now={new Date(NOW.getTime() + 1000)}
        onAdjust={() => {}}
        returnedFromSettings={false}
      />,
    );
    expect(time(container)).toBe("15:07:06");
  });
});

describe("ClockPanel, following the system zone", () => {
  inZone("America/Sao_Paulo");

  it("says the zone is the system's", () => {
    const { container } = renderPanel(clock({ timeZone: null }));
    expect(time(container)).toBe("15:07");
    expect(
      screen.getByText("Fuso do sistema: Sao Paulo · UTC−3"),
    ).toBeInTheDocument();
  });

  it("warns, with the system time, when Intl doesn't know the zone", () => {
    const { container } = renderPanel(clock({ timeZone: "Mars/Olympus" }));

    expect(screen.getByRole("status")).toHaveTextContent(
      ptBR.widgets.clock.zoneFallback,
    );
    expect(time(container)).toBe("15:07");
    expect(
      screen.getByText("Fuso do sistema: Sao Paulo · UTC−3"),
    ).toBeInTheDocument();
  });
});

describe("ClockPanel, with a system zone Intl doesn't identify", () => {
  inZone("Mars/Base");

  /** What `Intl` shows with its own default zone. */
  const intlDefault = () =>
    new Intl.DateTimeFormat("pt-BR", {
      hourCycle: "h23",
      hour: "2-digit",
      minute: "2-digit",
    }).format(NOW);

  it("says so in the footer, never naming a zone", () => {
    const { container } = renderPanel(clock({ timeZone: null }));

    expect(time(container)).toBe(intlDefault());
    expect(
      screen.getByText(ptBR.widgets.clock.systemZoneUnknown),
    ).toBeInTheDocument();
    expect(screen.queryByText(/UTC/)).not.toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("warns about the configured zone too, without throwing", () => {
    const { container } = renderPanel(clock({ timeZone: "Mars/Olympus" }));

    expect(time(container)).toBe(intlDefault());
    expect(screen.getByRole("status")).toHaveTextContent(
      ptBR.widgets.clock.zoneFallback,
    );
    expect(
      screen.getByText(ptBR.widgets.clock.systemZoneUnknown),
    ).toBeInTheDocument();
  });
});
