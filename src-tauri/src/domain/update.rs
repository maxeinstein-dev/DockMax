//! The automatic update (spec 0019 G3), its rules without the network: what
//! Sobre shows, when to look again and which new version deserves a Windows
//! notice. `windows::update` does the asking, downloading and installing.

use std::time::Duration;

use serde::Serialize;

/// The first look, a moment after the app opens: the dock comes first.
pub const FIRST_CHECK: Duration = Duration::from_secs(15);
/// Then every 6 hours (the author's decision, 2026-10-30).
pub const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// What Sobre shows about updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UpdateState {
    /// Nothing new, or not looked yet.
    None,
    /// A newer version, with the notes of its release.
    #[serde(rename_all = "camelCase")]
    Available {
        version: String,
        notes: Option<String>,
    },
    /// Downloading `version`: bytes so far and, when the server says, the
    /// total.
    #[serde(rename_all = "camelCase")]
    Downloading {
        version: String,
        downloaded: u64,
        total: Option<u64>,
    },
    /// Downloaded: the app is closing to run the installer.
    #[serde(rename_all = "camelCase")]
    Installing { version: String },
}

impl UpdateState {
    /// The state after a look that found `found` (`None`: nothing newer).
    /// A download or an install under way is kept: the look of the 6 hours
    /// doesn't take Sobre's progress away.
    pub fn after_check(self, found: Option<(String, Option<String>)>) -> Self {
        match self {
            busy @ (Self::Downloading { .. } | Self::Installing { .. }) => busy,
            _ => match found {
                Some((version, notes)) => Self::Available { version, notes },
                None => Self::None,
            },
        }
    }

    /// Whether a download or an install is under way.
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Downloading { .. } | Self::Installing { .. })
    }
}

/// Remembers the versions already announced: the notice comes once per new
/// version, not at every look.
#[derive(Debug, Default)]
pub struct Notices {
    last: Option<String>,
}

impl Notices {
    /// None announced yet.
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// Whether `version` should be announced now; it counts as announced
    /// from here on.
    pub fn first_time(&mut self, version: &str) -> bool {
        if self.last.as_deref() == Some(version) {
            return false;
        }
        self.last = Some(version.to_owned());
        true
    }
}

/// How long to wait before the next look, `since_last` after the last one
/// (`None`: no look yet).
pub fn wait(since_last: Option<Duration>) -> Duration {
    match since_last {
        None => FIRST_CHECK,
        Some(elapsed) => EVERY.saturating_sub(elapsed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available(version: &str) -> UpdateState {
        UpdateState::Available {
            version: version.into(),
            notes: None,
        }
    }

    #[test]
    fn a_look_sets_what_sobre_shows() {
        let found = Some(("0.5.0".to_string(), Some("Notas".to_string())));
        assert_eq!(
            UpdateState::None.after_check(found.clone()),
            UpdateState::Available {
                version: "0.5.0".into(),
                notes: Some("Notas".into())
            }
        );
        assert_eq!(available("0.5.0").after_check(None), UpdateState::None);
        assert_eq!(
            available("0.5.0").after_check(Some(("0.6.0".into(), None))),
            available("0.6.0")
        );
    }

    #[test]
    fn a_look_keeps_a_download_or_an_install() {
        let downloading = UpdateState::Downloading {
            version: "0.5.0".into(),
            downloaded: 10,
            total: Some(100),
        };
        assert_eq!(downloading.clone().after_check(None), downloading);
        let installing = UpdateState::Installing {
            version: "0.5.0".into(),
        };
        assert_eq!(
            installing.clone().after_check(Some(("0.6.0".into(), None))),
            installing
        );
        assert!(downloading.is_busy() && installing.is_busy());
        assert!(!available("0.5.0").is_busy() && !UpdateState::None.is_busy());
    }

    #[test]
    fn the_state_reaches_the_page_tagged() {
        assert_eq!(
            serde_json::to_value(UpdateState::None).unwrap(),
            serde_json::json!({ "state": "none" })
        );
        assert_eq!(
            serde_json::to_value(available("0.5.0")).unwrap(),
            serde_json::json!({ "state": "available", "version": "0.5.0", "notes": null })
        );
        assert_eq!(
            serde_json::to_value(UpdateState::Downloading {
                version: "0.5.0".into(),
                downloaded: 1,
                total: None
            })
            .unwrap(),
            serde_json::json!({ "state": "downloading", "version": "0.5.0", "downloaded": 1, "total": null })
        );
    }

    #[test]
    fn each_version_is_announced_once() {
        let mut notices = Notices::default();
        assert!(notices.first_time("0.5.0"));
        assert!(!notices.first_time("0.5.0"));
        assert!(!notices.first_time("0.5.0"));
        assert!(notices.first_time("0.6.0"));
        assert!(!notices.first_time("0.6.0"));
    }

    #[test]
    fn the_first_look_comes_soon_then_every_six_hours() {
        assert_eq!(wait(None), FIRST_CHECK);
        assert!(FIRST_CHECK <= Duration::from_secs(60));
        assert_eq!(wait(Some(Duration::ZERO)), Duration::from_secs(6 * 3600));
        assert_eq!(
            wait(Some(Duration::from_secs(3600))),
            Duration::from_secs(5 * 3600)
        );
        assert_eq!(wait(Some(Duration::from_secs(7 * 3600))), Duration::ZERO);
    }
}
