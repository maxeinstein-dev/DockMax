//! The Clima (spec 0017 F5): the weather at a city the user typed. The
//! config holds the width and the city picked from the search; the weather
//! is read in `windows::weather`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// One icon: the sky and the temperature; two: the day's highest and
/// lowest beside them.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 1..=2;

/// The longest name a city keeps.
pub const MAX_NAME: usize = 120;

/// A city, as the search found it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherCity {
    pub name: String,
    /// The state and the country, if the search gave them.
    #[serde(default)]
    pub region: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

// `validate` refuses a coordinate that is not a number.
impl Eq for WeatherCity {}

impl WeatherCity {
    fn valid(&self) -> bool {
        let text = |text: &str| !text.trim().is_empty() && text.chars().count() <= MAX_NAME;
        text(&self.name)
            && self.region.as_deref().is_none_or(text)
            && (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WeatherConfig {
    pub width: u8,
    /// `None` until the user picks one.
    pub city: Option<WeatherCity>,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        WeatherConfig {
            width: 1,
            city: None,
        }
    }
}

impl WeatherConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if !WIDTHS.contains(&self.width) {
            return Err(WidgetError::InvalidWidth);
        }
        if self.city.as_ref().is_some_and(|city| !city.valid()) {
            return Err(WidgetError::InvalidConfig);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn city(latitude: f64, name: &str) -> WeatherConfig {
        WeatherConfig {
            width: 1,
            city: Some(WeatherCity {
                name: name.into(),
                region: Some("Minas Gerais, Brasil".into()),
                latitude,
                longitude: -43.94,
            }),
        }
    }

    #[test]
    fn a_city_needs_a_name_and_real_coordinates() {
        assert_eq!(WeatherConfig::default().validate(), Ok(()));
        assert_eq!(city(-19.92, "Belo Horizonte").validate(), Ok(()));
        for bad in [
            city(f64::NAN, "Belo Horizonte"),
            city(91.0, "Belo Horizonte"),
            city(-19.92, "  "),
            city(-19.92, &"a".repeat(MAX_NAME + 1)),
        ] {
            assert_eq!(bad.validate(), Err(WidgetError::InvalidConfig));
        }
        let wide = WeatherConfig {
            width: 3,
            city: None,
        };
        assert_eq!(wide.validate(), Err(WidgetError::InvalidWidth));
    }
}
