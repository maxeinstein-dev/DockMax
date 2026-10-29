import {
  degrees,
  skyPaths,
  useWeather,
  type WeatherCondition,
} from "../../core/weather";
import { activeLocale, format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./weather.css";

/** A sky, drawn as the dock draws it. Decorative. */
export function Sky({
  condition,
  day = true,
  className,
}: {
  condition: WeatherCondition;
  day?: boolean;
  className: string;
}) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      {skyPaths(condition, day).map((path) => (
        <path key={path} d={path} />
      ))}
    </svg>
  );
}

/** The weekday of `date` (`YYYY-MM-DD`), short: `qui.`. */
function weekday(date: string): string {
  // Midday local: the same day in any zone.
  return new Intl.DateTimeFormat(activeLocale(), { weekday: "short" }).format(
    new Date(`${date}T12:00:00`),
  );
}

/**
 * The main view of a Clima panel (spec 0017 F5): the weather now at the
 * city, how it feels, the humidity and the wind, and the days ahead with
 * their highest, lowest and chance of rain.
 */
function WeatherPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"weather">) {
  const t = useT();
  const text = t.widgets.weather;
  const { city } = instance.config;
  const reading = useWeather(city);

  const body = () => {
    if (!city) return <p className="weather-note">{text.noCity}</p>;
    if (reading === undefined || reading === null) {
      return <p className="weather-note">{text.loading}</p>;
    }
    if (reading.state === "unavailable") {
      return <p className="weather-note">{text.unavailable}</p>;
    }
    const { current, days } = reading.forecast;
    const details = [
      current.feelsLike === null
        ? null
        : format(text.feelsLike, { value: degrees(current.feelsLike) }),
      current.humidity === null
        ? null
        : format(text.humidity, { value: Math.round(current.humidity) }),
      current.wind === null
        ? null
        : format(text.wind, { value: Math.round(current.wind) }),
    ].filter(Boolean);
    return (
      <>
        <section className="weather-now" aria-label={text.now}>
          <Sky
            className="weather-now-sky"
            condition={current.condition}
            day={current.day}
          />
          <div>
            <p className="weather-temperature">
              {degrees(current.temperature)}
            </p>
            <p className="weather-condition">
              {text.conditions[current.condition]}
            </p>
          </div>
        </section>
        {details.length > 0 ? (
          <p className="weather-details">{details.join(" · ")}</p>
        ) : null}
        <ul className="weather-days" aria-label={text.days}>
          {days.map((day, index) => (
            <li key={day.date} className="weather-day">
              <span className="weather-day-name">
                {index === 0 ? text.today : weekday(day.date)}
              </span>
              <Sky className="weather-day-sky" condition={day.condition} />
              <span className="weather-day-rain">
                {day.rain !== null && day.rain >= 10
                  ? `${Math.round(day.rain)}%`
                  : ""}
              </span>
              <span className="weather-day-range">
                <span className="visually-hidden">
                  {text.conditions[day.condition]},{" "}
                </span>
                {format(text.range, {
                  max: degrees(day.max),
                  min: degrees(day.min),
                })}
              </span>
            </li>
          ))}
        </ul>
      </>
    );
  };

  return (
    <>
      <PanelHeader
        title={instance.label ?? city?.name ?? widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {body()}
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default WeatherPanel;
