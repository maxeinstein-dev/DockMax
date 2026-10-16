//! Clock widget config (spec 0003 §2.2, Clock): which time zone, 24 h or
//! 12 h, whether the panel shows seconds and how the dock icon draws the
//! time.

use jiff::tz::TimeZoneDatabase;
use serde::{Deserialize, Serialize};

use super::WidgetError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClockConfig {
    /// IANA time zone name, such as `America/Sao_Paulo`; `None` follows the
    /// system time zone. Serialized as `null`, never left out, so the
    /// interface always gets the field.
    pub time_zone: Option<String>,
    pub hour_cycle: HourCycle,
    /// Seconds in the panel only; the dock icon never shows them.
    pub show_seconds: bool,
    /// How the dock icon shows the time. Added after the first release of
    /// this config (spec 0003 §2.11): a stored config without it reads as
    /// [`ClockIconStyle::Digital`], through `#[serde(default)]`.
    pub icon_style: ClockIconStyle,
}

impl ClockConfig {
    /// Checks the time zone name against the IANA database bundled in the
    /// binary (never the system's, so the answer is the same on every
    /// machine). `hour_cycle`, `show_seconds` and `icon_style` are closed by
    /// their types.
    pub fn validate(&self) -> Result<(), WidgetError> {
        match &self.time_zone {
            None => Ok(()),
            Some(name) if is_iana_time_zone(name) => Ok(()),
            Some(_) => Err(WidgetError::InvalidTimeZone),
        }
    }
}

/// Whether `name` is an IANA time zone written exactly as the database
/// writes it. The lookup ignores ASCII case, so the name it gives back is
/// compared too: `america/sao_paulo` is rejected rather than stored in a
/// spelling `Intl` may not take. `Etc/Unknown` has no IANA name and is
/// rejected as well, and so is `Factory`: it is in the database, but it
/// means "time zone not set", not a place.
fn is_iana_time_zone(name: &str) -> bool {
    name != "Factory"
        && TimeZoneDatabase::bundled()
            .get(name)
            .is_ok_and(|tz| tz.iana_name() == Some(name))
}

/// How hours are counted, named as in `Intl.DateTimeFormat`'s `hourCycle`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HourCycle {
    /// 0–23.
    #[default]
    H23,
    /// 1–12 with AM/PM.
    H12,
}

closed_set!(HourCycle { H23, H12 });

/// How the dock icon of a clock shows the time (spec 0003 §2.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClockIconStyle {
    /// `HH:mm` (or `h:mm` in 12 h, without AM/PM).
    #[default]
    Digital,
    /// White hands on the glaze, without numbers.
    Analog,
}

closed_set!(ClockIconStyle { Digital, Analog });

#[cfg(test)]
mod tests {
    use super::*;

    fn with_time_zone(name: &str) -> ClockConfig {
        ClockConfig {
            time_zone: Some(name.to_owned()),
            ..ClockConfig::default()
        }
    }

    #[test]
    fn default_follows_the_system_time_zone_and_is_valid() {
        let config = ClockConfig::default();
        assert_eq!(config.time_zone, None);
        assert_eq!(config.hour_cycle, HourCycle::H23);
        assert!(!config.show_seconds);
        assert_eq!(config.icon_style, ClockIconStyle::Digital);
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn icon_styles_use_lowercase_names_in_json() {
        assert_eq!(
            serde_json::to_string(&ClockIconStyle::ALL).unwrap(),
            r#"["digital","analog"]"#
        );
        assert!(serde_json::from_str::<ClockIconStyle>(r#""Analog""#).is_err());
    }

    #[test]
    fn a_stored_config_without_the_icon_style_reads_as_digital() {
        let config: ClockConfig =
            serde_json::from_str(r#"{"timeZone":null,"hourCycle":"h12","showSeconds":true}"#)
                .unwrap();
        assert_eq!(config.icon_style, ClockIconStyle::Digital);
        assert_eq!(config.hour_cycle, HourCycle::H12);
        assert!(config.show_seconds);
    }

    #[test]
    fn iana_names_in_their_exact_spelling_are_accepted() {
        let accepted = [
            "America/Sao_Paulo",
            "Europe/Lisbon",
            "Asia/Kolkata",
            "America/Argentina/Buenos_Aires",
            "UTC",
            // Links (old names) keep their own name in the database, so
            // they are accepted as written.
            "Asia/Calcutta",
            "Etc/UTC",
            "Brazil/East",
        ];
        for name in accepted {
            assert_eq!(with_time_zone(name).validate(), Ok(()), "{name}");
        }
    }

    #[test]
    fn unknown_or_misspelled_time_zones_are_rejected() {
        let rejected = [
            "",
            " ",
            "Mars/Olympus_Mons",
            "America/SaoPaulo",
            "america/sao_paulo",
            "AMERICA/SAO_PAULO",
            "utc",
            "Etc/Unknown",
            // In the database, but not a place: it means "time zone not set".
            "Factory",
            " America/Sao_Paulo",
            "America/Sao_Paulo ",
            "-03:00",
            "+0300",
            "EST5EDT,M3.2.0,M11.1.0",
            "/etc/localtime",
            "../../etc/passwd",
            "America/Sao_Paulo\0",
        ];
        for name in rejected {
            assert_eq!(
                with_time_zone(name).validate(),
                Err(WidgetError::InvalidTimeZone),
                "{name:?}"
            );
        }
    }

    #[test]
    fn the_error_message_never_names_the_time_zone() {
        let error = with_time_zone("Mars/Olympus_Mons").validate().unwrap_err();
        let message = error.to_string();
        assert!(
            !message.contains("Mars") && !message.contains('/'),
            "{message}"
        );
    }
}
