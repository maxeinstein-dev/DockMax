//! The Tocando agora (spec 0017 F4): what Windows plays now, from its media
//! session. The config holds only the width; the session is read in
//! `windows::media`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// One icon: the cover; two or three: the cover, the title and the artist.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 1..=3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MediaConfig {
    pub width: u8,
}

impl Default for MediaConfig {
    fn default() -> Self {
        MediaConfig { width: 2 }
    }
}

impl MediaConfig {
    pub fn validate(&self) -> Result<(), WidgetError> {
        if WIDTHS.contains(&self.width) {
            Ok(())
        } else {
            Err(WidgetError::InvalidWidth)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_takes_one_to_three_icons() {
        assert_eq!(MediaConfig::default().validate(), Ok(()));
        assert_eq!(MediaConfig { width: 3 }.validate(), Ok(()));
        for width in [0, 4] {
            assert_eq!(
                MediaConfig { width }.validate(),
                Err(WidgetError::InvalidWidth)
            );
        }
    }
}
