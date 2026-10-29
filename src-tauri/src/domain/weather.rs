//! The Clima (spec 0017 F5): the weather now and the next days at a city
//! the user typed, from Open-Meteo (free, no key nor account). Pure:
//! `windows/weather.rs` asks the service. Only the city's name (to find it)
//! and its coordinates leave the computer.

use serde::{Deserialize, Serialize};

/// How many days the forecast holds, today first.
pub const DAYS: usize = 6;

/// The weather's kind, from the WMO code Open-Meteo sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Condition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Storm,
}

impl Condition {
    /// The kind of the WMO weather code `code`; `None` for one it doesn't
    /// name.
    pub fn from_code(code: u16) -> Option<Condition> {
        Some(match code {
            0 => Condition::Clear,
            1 | 2 => Condition::PartlyCloudy,
            3 => Condition::Cloudy,
            45 | 48 => Condition::Fog,
            51..=57 => Condition::Drizzle,
            61..=67 | 80..=82 => Condition::Rain,
            71..=77 | 85 | 86 => Condition::Snow,
            95..=99 => Condition::Storm,
            _ => return None,
        })
    }
}

/// The weather now.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Current {
    /// °C.
    pub temperature: f64,
    /// How warm it feels, °C.
    pub feels_like: Option<f64>,
    /// 0–100.
    pub humidity: Option<f64>,
    /// km/h.
    pub wind: Option<f64>,
    pub condition: Condition,
    /// Daytime: night draws a moon for a clear sky.
    pub day: bool,
}

/// One day of the forecast.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// `YYYY-MM-DD`, the city's own date.
    pub date: String,
    pub condition: Condition,
    /// °C.
    pub max: f64,
    pub min: f64,
    /// The chance of rain, 0–100.
    pub rain: Option<f64>,
}

/// The weather now and the days ahead, today first.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub current: Current,
    pub days: Vec<Day>,
}

#[derive(Deserialize)]
struct RawCurrent {
    temperature_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    relative_humidity_2m: Option<f64>,
    wind_speed_10m: Option<f64>,
    weather_code: Option<u16>,
    is_day: Option<u8>,
}

#[derive(Deserialize)]
struct RawDaily {
    time: Vec<String>,
    weather_code: Vec<Option<u16>>,
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    #[serde(default)]
    precipitation_probability_max: Vec<Option<f64>>,
}

#[derive(Deserialize)]
struct RawForecast {
    current: RawCurrent,
    daily: Option<RawDaily>,
}

/// The forecast in Open-Meteo's answer; `None` when it is not one.
pub fn parse_forecast(json: &[u8]) -> Option<Forecast> {
    let raw: RawForecast = serde_json::from_slice(json).ok()?;
    let current = Current {
        temperature: raw.current.temperature_2m?,
        feels_like: raw.current.apparent_temperature,
        humidity: raw
            .current
            .relative_humidity_2m
            .map(|humidity| humidity.clamp(0.0, 100.0)),
        wind: raw.current.wind_speed_10m,
        condition: Condition::from_code(raw.current.weather_code?)?,
        day: raw.current.is_day != Some(0),
    };
    let days = raw.daily.map_or_else(Vec::new, |daily| {
        daily
            .time
            .into_iter()
            .enumerate()
            .filter_map(|(index, date)| {
                Some(Day {
                    date,
                    condition: Condition::from_code((*daily.weather_code.get(index)?)?)?,
                    max: (*daily.temperature_2m_max.get(index)?)?,
                    min: (*daily.temperature_2m_min.get(index)?)?,
                    rain: daily
                        .precipitation_probability_max
                        .get(index)
                        .copied()
                        .flatten(),
                })
            })
            .take(DAYS)
            .collect()
    });
    Some(Forecast { current, days })
}

/// The address of the forecast at a city.
pub fn forecast_url(latitude: f64, longitude: f64) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude:.4}&longitude={longitude:.4}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m,is_day\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max\
         &timezone=auto&forecast_days={DAYS}"
    )
}

/// A city the search found.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    /// The state and the country, for telling cities of one name apart.
    pub region: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
struct RawPlace {
    name: String,
    latitude: f64,
    longitude: f64,
    admin1: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize)]
struct RawPlaces {
    #[serde(default)]
    results: Vec<RawPlace>,
}

/// The cities in Open-Meteo's search answer; `None` when it is not one.
pub fn parse_places(json: &[u8]) -> Option<Vec<Place>> {
    let raw: RawPlaces = serde_json::from_slice(json).ok()?;
    Some(
        raw.results
            .into_iter()
            .map(|place| {
                let region: Vec<String> = [place.admin1, place.country]
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.is_empty() && *part != place.name)
                    .collect();
                Place {
                    name: place.name,
                    region: (!region.is_empty()).then(|| region.join(", ")),
                    latitude: place.latitude,
                    longitude: place.longitude,
                }
            })
            .collect(),
    )
}

/// The longest name searched.
pub const MAX_QUERY: usize = 80;

/// The address of the search for `query`, in Portuguese; `None` for an
/// empty or too long one.
pub fn search_url(query: &str) -> Option<String> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > MAX_QUERY {
        return None;
    }
    let mut url = url::Url::parse("https://geocoding-api.open-meteo.com/v1/search").ok()?;
    url.query_pairs_mut()
        .append_pair("name", query)
        .append_pair("count", "6")
        .append_pair("language", "pt")
        .append_pair("format", "json");
    Some(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &[u8] = br#"{
        "latitude": -19.92, "longitude": -43.94,
        "current": {"time": "2026-10-29T12:30", "interval": 900, "temperature_2m": 24.3,
            "relative_humidity_2m": 61, "apparent_temperature": 25.1, "is_day": 1,
            "weather_code": 2, "wind_speed_10m": 10.4},
        "daily": {"time": ["2026-10-29", "2026-10-30", "2026-10-31"],
            "weather_code": [2, 63, null],
            "temperature_2m_max": [28.1, 25.0, 27.0],
            "temperature_2m_min": [18.2, 17.5, 16.0],
            "precipitation_probability_max": [10, 80, 5]}
    }"#;

    #[test]
    fn it_reads_now_and_the_days() {
        let forecast = parse_forecast(ANSWER).unwrap();
        assert_eq!(forecast.current.temperature, 24.3);
        assert_eq!(forecast.current.condition, Condition::PartlyCloudy);
        assert_eq!(forecast.current.humidity, Some(61.0));
        assert!(forecast.current.day);
        // A day without a code is left out.
        assert_eq!(forecast.days.len(), 2);
        assert_eq!(forecast.days[1].condition, Condition::Rain);
        assert_eq!(forecast.days[1].rain, Some(80.0));
        assert_eq!(forecast.days[0].date, "2026-10-29");
    }

    #[test]
    fn an_odd_answer_is_none() {
        assert!(parse_forecast(br#"{"error":true,"reason":"bad"}"#).is_none());
        assert!(parse_forecast(b"not json").is_none());
        assert!(
            parse_forecast(br#"{"current":{"temperature_2m":20,"weather_code":1000}}"#).is_none()
        );
    }

    #[test]
    fn the_codes_fall_in_kinds() {
        assert_eq!(Condition::from_code(0), Some(Condition::Clear));
        assert_eq!(Condition::from_code(45), Some(Condition::Fog));
        assert_eq!(Condition::from_code(55), Some(Condition::Drizzle));
        assert_eq!(Condition::from_code(81), Some(Condition::Rain));
        assert_eq!(Condition::from_code(86), Some(Condition::Snow));
        assert_eq!(Condition::from_code(96), Some(Condition::Storm));
        assert_eq!(Condition::from_code(7), None);
    }

    #[test]
    fn it_reads_the_cities_found() {
        let json = br#"{"results":[
            {"id":1,"name":"Belo Horizonte","latitude":-19.92,"longitude":-43.94,"admin1":"Minas Gerais","country":"Brasil"},
            {"id":2,"name":"Lisboa","latitude":38.72,"longitude":-9.14,"admin1":"Lisboa","country":"Portugal"}
        ]}"#;
        let places = parse_places(json).unwrap();
        assert_eq!(places[0].region.as_deref(), Some("Minas Gerais, Brasil"));
        assert_eq!(places[1].region.as_deref(), Some("Portugal"));
        assert_eq!(parse_places(br#"{"generationtime_ms":0.5}"#), Some(vec![]));
    }

    #[test]
    fn the_search_address_is_encoded() {
        let url = search_url("  São Paulo ").unwrap();
        assert!(url.contains("name=S%C3%A3o+Paulo"), "{url}");
        assert!(url.starts_with("https://geocoding-api.open-meteo.com/"));
        assert!(search_url("   ").is_none());
        assert!(search_url(&"a".repeat(MAX_QUERY + 1)).is_none());
        assert!(forecast_url(-19.92, -43.94).contains("latitude=-19.9200&longitude=-43.9400"));
    }
}
