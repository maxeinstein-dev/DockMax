import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";
import type { WeatherCity } from "./widgets";

/* The Clima's panel (spec 0017 F5): the forecast of a city, read in Rust
 * (`windows/weather.rs`) from Open-Meteo. Must match `Reading` there and
 * `domain/weather.rs`. */

export type WeatherCondition =
  | "clear"
  | "partlyCloudy"
  | "cloudy"
  | "fog"
  | "drizzle"
  | "rain"
  | "snow"
  | "storm";

export interface WeatherCurrent {
  /** °C. */
  temperature: number;
  feelsLike: number | null;
  /** 0–100. */
  humidity: number | null;
  /** km/h. */
  wind: number | null;
  condition: WeatherCondition;
  day: boolean;
}

export interface WeatherDay {
  /** `YYYY-MM-DD`, the city's own date. */
  date: string;
  condition: WeatherCondition;
  max: number;
  min: number;
  /** The chance of rain, 0–100. */
  rain: number | null;
}

export interface Forecast {
  current: WeatherCurrent;
  days: WeatherDay[];
}

export type WeatherReading =
  { state: "ready"; forecast: Forecast; at: number } | { state: "unavailable" };

/** A city the search found. */
export interface WeatherPlace {
  name: string;
  region: string | null;
  latitude: number;
  longitude: number;
}

/** How often an open panel asks for the last reading. */
export const WEATHER_EVERY_MS = 60_000;

/** The last reading at `city` (null before the first); `fresh` reads again
 * now if it is not recent. */
export function weatherReading(
  city: Pick<WeatherCity, "latitude" | "longitude">,
  fresh: boolean,
): Promise<WeatherReading | null> {
  return invoke<WeatherReading | null>("weather_reading", {
    latitude: city.latitude,
    longitude: city.longitude,
    fresh,
  });
}

/** The cities named like `query`. */
export function searchCities(query: string): Promise<WeatherPlace[]> {
  return invoke<WeatherPlace[]>("weather_search", { query });
}

/** The last reading at `city`, asked again while mounted; the first ask
 * reads anew if it is not recent. `undefined` while the first is on its
 * way, `null` when there is no city or no reading yet. */
export function useWeather(
  city: WeatherCity | null,
): WeatherReading | null | undefined {
  // The reading with the city it is of: another city's is never shown.
  const [held, setHeld] = useState<{
    key: string;
    reading: WeatherReading | null;
  }>();
  const latitude = city?.latitude;
  const longitude = city?.longitude;
  const key = city ? `${city.latitude},${city.longitude}` : null;
  useEffect(() => {
    if (key === null || latitude === undefined || longitude === undefined) {
      return;
    }
    let active = true;
    const ask = (fresh: boolean) => {
      weatherReading({ latitude, longitude }, fresh).then(
        (reading) => {
          if (active) setHeld({ key, reading });
        },
        (error: unknown) => {
          log.warn(`weather: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask(true);
    // A fresh reading lands a moment after: asked again soon, then every
    // minute.
    const soon = window.setTimeout(() => ask(false), 2_500);
    const every = window.setInterval(() => ask(false), WEATHER_EVERY_MS);
    return () => {
      active = false;
      window.clearTimeout(soon);
      window.clearInterval(every);
    };
  }, [key, latitude, longitude]);
  if (key === null) return null;
  return held?.key === key ? held.reading : undefined;
}

/** `24°`, rounded. */
export function degrees(value: number): string {
  // `|| 0`: no "-0°".
  return `${Math.round(value) || 0}°`;
}

/** The lines of each sky (24 px box, after Lucide's), as the dock draws
 * them (`native_dock/paint.rs`, `sky`). */
const CLOUD_TOP = "M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242";
const SKIES: Record<WeatherCondition | "night", readonly string[]> = {
  clear: [
    "M12 8a4 4 0 1 1 0 8a4 4 0 1 1 0 -8",
    "M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4 -1.4M17.3 6.7l1.4 -1.4",
  ],
  night: ["M12 3a6 6 0 0 0 9 9a9 9 0 1 1 -9 -9z"],
  partlyCloudy: [
    "M12 2v2M4.93 4.93l1.41 1.41M20 12h2M19.07 4.93l-1.41 1.41",
    "M15.947 12.65a4 4 0 0 0 -5.925 -4.128",
    "M13 22H7a5 5 0 1 1 4.9 -6H13a3 3 0 0 1 0 6z",
  ],
  cloudy: ["M17.5 19H9a7 7 0 1 1 6.71 -9h1.79a4.5 4.5 0 1 1 0 9z"],
  fog: [CLOUD_TOP, "M16 17H7M17 21H9"],
  drizzle: [CLOUD_TOP, "M8 19v1M8 14v1M16 19v1M16 14v1M12 21v1M12 16v1"],
  rain: [CLOUD_TOP, "M16 14v6M8 14v6M12 16v6"],
  snow: [
    CLOUD_TOP,
    "M8 15h0.01M8 19h0.01M12 17h0.01M12 21h0.01M16 15h0.01M16 19h0.01",
  ],
  storm: [
    "M6 16.326A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 0.5 8.973",
    "M13 12l-3 5h4l-3 5",
  ],
};

/** The lines of the sky of `condition`; a clear night has the moon. */
export function skyPaths(
  condition: WeatherCondition,
  day: boolean,
): readonly string[] {
  return condition === "clear" && !day ? SKIES.night : SKIES[condition];
}
