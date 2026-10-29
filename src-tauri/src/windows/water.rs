//! The Beber água's reminder (spec 0017 F3-2): the dock looks every minute;
//! when one is due, the tile bounces and keeps a badge until its panel
//! opens, and again after as many minutes if still no cup comes.

use std::sync::Mutex;

use crate::domain::widgets::WaterConfig;

/// When each widget was last reminded, in Unix seconds.
static REMINDED: Mutex<Vec<(String, i64)>> = Mutex::new(Vec::new());

/// The widgets (by ID) reminded and not seen: a badge on their tile.
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The widgets of `waters` whose reminder is due at `now` and not given in
/// the last `remind` minutes: they get the badge and the bounce.
pub fn check(waters: &[(String, WaterConfig)], now: i64, offset: i64) {
    let due: Vec<String> = {
        let mut reminded = REMINDED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        waters
            .iter()
            .filter(|(_, config)| config.due(now, offset))
            .filter_map(|(id, config)| {
                let wait = i64::from(config.remind) * 60;
                let last = reminded.iter_mut().find(|(each, _)| each == id);
                match last {
                    Some((_, at)) if now - *at < wait => None,
                    Some((_, at)) => {
                        *at = now;
                        Some(id.clone())
                    }
                    None => {
                        reminded.push((id.clone(), now));
                        Some(id.clone())
                    }
                }
            })
            .collect()
    };
    if due.is_empty() {
        return;
    }
    {
        let mut pending = PENDING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for id in &due {
            if !pending.contains(id) {
                pending.push(id.clone());
            }
        }
    }
    super::native_dock::nudged(due);
}

/// Whether the widget `widget` was reminded and not seen: its badge.
pub fn pending(widget: &str) -> bool {
    PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .any(|each| each == widget)
}

/// Its panel opened: the reminder was seen. Whether it had a badge.
pub fn seen(widget: &str) -> bool {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = pending.len();
    pending.retain(|each| each != widget);
    pending.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_due_reminder_leaves_a_badge_once_per_interval() {
        // 2026-10-28 at 19:37 in São Paulo (UTC-3), no cup today.
        let offset = -3 * 3600;
        let now = 20_389 * 24 * 3600 + 19 * 3600 + 37 * 60 - offset;
        let config = WaterConfig {
            remind: 45,
            ..WaterConfig::default()
        };
        let id = "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f6099".to_string();
        check(&[(id.clone(), config.clone())], now, offset);
        assert!(pending(&id));
        assert!(seen(&id));
        // Seen, and not again before another interval.
        check(&[(id.clone(), config.clone())], now + 60, offset);
        assert!(!pending(&id));
        check(&[(id.clone(), config)], now + 45 * 60, offset);
        assert!(pending(&id));
    }
}
