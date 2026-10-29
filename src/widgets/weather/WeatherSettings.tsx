import { useState } from "react";
import { errorKind, log } from "../../core/log";
import { searchCities, type WeatherPlace } from "../../core/weather";
import { WEATHER_WIDTHS } from "../../core/widgets";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { WidthGroup } from "../system/parts";
import { useSystemSettings } from "../system/useSettings";
import "./weather.css";

/** The search's state: nothing asked yet, on its way, the cities found,
 * or no answer. */
type Search =
  | { state: "idle" }
  | { state: "searching" }
  | { state: "found"; places: WeatherPlace[] }
  | { state: "failed" };

/** The "Ajustes" view of a Clima panel: label, the city (searched by its
 * name and picked from what was found), width and "Remover do dock". Each
 * change is saved at once. */
function WeatherSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"weather">) {
  const t = useT();
  const text = t.widgets.weather;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState<Search>({ state: "idle" });

  function find() {
    const name = query.trim();
    if (!name || search.state === "searching") return;
    setSearch({ state: "searching" });
    searchCities(name).then(
      (places) => setSearch({ state: "found", places }),
      (error: unknown) => {
        log.warn(`weather: search failed: ${errorKind(error)}`);
        setSearch({ state: "failed" });
      },
    );
  }

  function pick(place: WeatherPlace) {
    settings.saveConfig({
      city: {
        name: place.name,
        region: place.region,
        latitude: place.latitude,
        longitude: place.longitude,
      },
    });
    setSearch({ state: "idle" });
    setQuery("");
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.weather}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <p className="weather-city" aria-live="polite">
        {config.city
          ? format(text.cityNow, {
              city: [config.city.name, config.city.region]
                .filter(Boolean)
                .join(", "),
            })
          : text.noCity}
      </p>
      <div className="weather-search">
        <TextField
          label={text.search}
          placeholder={text.searchPlaceholder}
          value={query}
          onChange={setQuery}
          onCommit={find}
        />
        <button
          type="button"
          className="secondary-button"
          onClick={find}
          disabled={!query.trim() || search.state === "searching"}
        >
          {text.searchButton}
        </button>
      </div>
      {search.state === "searching" ? (
        <p className="sys-hint" role="status">
          {text.searching}
        </p>
      ) : search.state === "failed" ? (
        <p className="sys-hint" role="status">
          {text.searchFailed}
        </p>
      ) : search.state === "found" && search.places.length === 0 ? (
        <p className="sys-hint" role="status">
          {text.nothingFound}
        </p>
      ) : search.state === "found" ? (
        <ul className="weather-places" aria-label={text.found}>
          {search.places.map((place) => (
            <li key={`${place.latitude},${place.longitude}`}>
              <button
                type="button"
                className="weather-place"
                onClick={() => pick(place)}
              >
                <span className="weather-place-name">{place.name}</span>
                {place.region ? (
                  <span className="weather-place-region">{place.region}</span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
      <WidthGroup
        width={config.width}
        widths={WEATHER_WIDTHS}
        onSelect={(width) => settings.saveConfig({ width })}
      />
      <p className="sys-hint">{text.widthHint}</p>
      <p className="sys-hint">{text.privacy}</p>
      <RemoveButton
        onClick={() => void settings.remove()}
        disabled={settings.removing}
      />
      <PanelFooter
        alert={settings.error ? t.widgets.errors[settings.error] : null}
      >
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default WeatherSettings;
