import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { degrees, type WeatherReading } from "../../core/weather";
import type {
  WeatherCity,
  WeatherConfig,
  WidgetInstance,
} from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import WeatherPanel from "./WeatherPanel";
import WeatherSettings from "./WeatherSettings";

const text = ptBR.widgets.weather;

type OnUpdate = (update: WidgetUpdate<"weather">) => Promise<void>;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const BH: WeatherCity = {
  name: "Belo Horizonte",
  region: "Minas Gerais, Brasil",
  latitude: -19.92,
  longitude: -43.94,
};

function weather(city: WeatherCity | null): WidgetInstance<"weather"> {
  return {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f60a0",
    label: null,
    kind: "weather",
    config: { width: 1, city },
  };
}

function panel(city: WeatherCity | null, reading: WeatherReading | null) {
  invoke.mockImplementation(() => Promise.resolve(reading));
  render(
    <WeatherPanel
      instance={weather(city)}
      now={new Date(2026, 9, 29, 12)}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
}

beforeEach(() => {
  invoke.mockReset();
});

describe("WeatherPanel", () => {
  it("shows the weather now and the days ahead", async () => {
    panel(BH, {
      state: "ready",
      at: 0,
      forecast: {
        current: {
          temperature: 24.3,
          feelsLike: 25.1,
          humidity: 61,
          wind: 10.4,
          condition: "partlyCloudy",
          day: true,
        },
        days: [
          {
            date: "2026-10-29",
            condition: "partlyCloudy",
            max: 28.1,
            min: 18.2,
            rain: 5,
          },
          {
            date: "2026-10-30",
            condition: "rain",
            max: 25,
            min: 17.5,
            rain: 80,
          },
        ],
      },
    });
    expect(await screen.findByText("24°")).toBeInTheDocument();
    expect(screen.getByText(text.conditions.partlyCloudy)).toBeInTheDocument();
    expect(
      screen.getByText("Sensação 25° · Umidade 61% · Vento 10 km/h"),
    ).toBeInTheDocument();
    expect(screen.getByText(text.today)).toBeInTheDocument();
    expect(screen.getByText("80%")).toBeInTheDocument();
    expect(screen.getByText("28° / 18°")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: BH.name })).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("weather_reading", {
      latitude: BH.latitude,
      longitude: BH.longitude,
      fresh: true,
    });
  });

  it("asks for a city first, and says when there is no answer", async () => {
    panel(null, null);
    expect(screen.getByText(text.noCity)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
    panel(BH, { state: "unavailable" });
    expect(await screen.findByText(text.unavailable)).toBeInTheDocument();
  });
});

describe("WeatherSettings", () => {
  it("finds a city by its name and keeps the one picked", async () => {
    invoke.mockImplementation(() =>
      Promise.resolve([
        { ...BH },
        {
          name: "Belo Horizonte",
          region: "Pará, Brasil",
          latitude: -1,
          longitude: -48,
        },
      ]),
    );
    const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
    render(
      <WeatherSettings
        instance={weather(null)}
        onUpdate={onUpdate}
        onRemove={() => Promise.resolve()}
        onBack={() => undefined}
      />,
    );
    fireEvent.change(screen.getByLabelText(text.search), {
      target: { value: "belo horizonte" },
    });
    fireEvent.click(screen.getByRole("button", { name: text.searchButton }));
    expect(invoke).toHaveBeenCalledWith("weather_search", {
      query: "belo horizonte",
    });
    fireEvent.click(
      await screen.findByRole("button", { name: /Minas Gerais, Brasil/ }),
    );
    expect(onUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({
        config: { width: 1, city: BH } satisfies WeatherConfig,
      }),
    );
  });
});

describe("degrees", () => {
  it("rounds, with no minus zero", () => {
    expect(degrees(24.5)).toBe("25°");
    expect(degrees(-0.4)).toBe("0°");
    expect(degrees(-3.6)).toBe("-4°");
  });
});
