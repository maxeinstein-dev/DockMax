//! The Pomodoro's timer (spec 0017 F2c-2), without the clock: every step
//! takes the time, in milliseconds. Focus, pause, focus, pause... and a long
//! pause after every `every` focuses.

use serde::Serialize;

use crate::domain::widgets::PomodoroConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Focus,
    Short,
    Long,
}

const MINUTE: i64 = 60_000;

/// How long `stage` lasts, in milliseconds.
pub fn length(stage: Stage, config: &PomodoroConfig) -> i64 {
    i64::from(match stage {
        Stage::Focus => config.focus,
        Stage::Short => config.short,
        Stage::Long => config.long,
    }) * MINUTE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    pub stage: Stage,
    /// When the stage ends, while it runs.
    pub ends: Option<i64>,
    /// What is left of the stage while it waits.
    pub left: i64,
    /// Focuses done since the last long pause.
    pub focuses: u8,
}

impl Timer {
    /// A focus, waiting to start.
    pub fn new(config: &PomodoroConfig) -> Timer {
        Timer {
            stage: Stage::Focus,
            ends: None,
            left: length(Stage::Focus, config),
            focuses: 0,
        }
    }

    pub fn running(&self) -> bool {
        self.ends.is_some()
    }

    /// What is left of the stage at `now`.
    pub fn left_at(&self, now: i64) -> i64 {
        self.ends.map_or(self.left, |ends| (ends - now).max(0))
    }

    pub fn start(&mut self, now: i64) {
        if self.ends.is_none() {
            self.ends = Some(now + self.left);
        }
    }

    pub fn pause(&mut self, now: i64) {
        if let Some(ends) = self.ends.take() {
            self.left = (ends - now).max(0);
        }
    }

    /// To the next stage, waiting, or running from `at` when the config
    /// starts it by itself.
    fn advance(&mut self, config: &PomodoroConfig, at: Option<i64>) {
        let next = match self.stage {
            Stage::Focus => {
                self.focuses = self.focuses.saturating_add(1);
                if self.focuses >= config.every {
                    Stage::Long
                } else {
                    Stage::Short
                }
            }
            Stage::Long => {
                self.focuses = 0;
                Stage::Focus
            }
            Stage::Short => Stage::Focus,
        };
        self.stage = next;
        self.left = length(next, config);
        self.ends = at.filter(|_| config.auto_start).map(|at| at + self.left);
    }

    /// The stage ends now, as if its time ran out; the next one waits.
    pub fn skip(&mut self, config: &PomodoroConfig) {
        self.advance(config, None);
    }

    /// The stage that ended by `now`, if one did: the timer is then on the
    /// next one.
    pub fn tick(&mut self, config: &PomodoroConfig, now: i64) -> Option<Stage> {
        let ends = self.ends?;
        if now < ends {
            return None;
        }
        let done = self.stage;
        self.advance(config, Some(ends));
        Some(done)
    }

    /// The share of the stage gone at `now`, 0–1.
    pub fn fraction(&self, config: &PomodoroConfig, now: i64) -> f32 {
        let total = length(self.stage, config).max(1);
        (1.0 - self.left_at(now) as f64 / total as f64).clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(auto_start: bool) -> PomodoroConfig {
        PomodoroConfig {
            auto_start,
            ..PomodoroConfig::default()
        }
    }

    #[test]
    fn it_counts_down_and_pauses() {
        let config = config(false);
        let mut timer = Timer::new(&config);
        assert_eq!(timer.left_at(0), 25 * MINUTE);
        timer.start(1_000);
        assert_eq!(timer.left_at(1_000 + 10 * MINUTE), 15 * MINUTE);
        timer.pause(1_000 + 10 * MINUTE);
        assert!(!timer.running());
        // Paused, the time stands still.
        assert_eq!(timer.left_at(99 * MINUTE), 15 * MINUTE);
        timer.start(100 * MINUTE);
        assert_eq!(timer.tick(&config, 114 * MINUTE), None);
        assert_eq!(timer.tick(&config, 115 * MINUTE), Some(Stage::Focus));
        assert_eq!(timer.stage, Stage::Short);
        assert!(!timer.running());
        assert_eq!(timer.left_at(0), 5 * MINUTE);
    }

    #[test]
    fn a_long_pause_comes_after_every_fourth_focus() {
        let config = config(false);
        let mut timer = Timer::new(&config);
        let mut stages = Vec::new();
        for _ in 0..9 {
            timer.skip(&config);
            stages.push(timer.stage);
        }
        use Stage::*;
        assert_eq!(
            stages,
            [Short, Focus, Short, Focus, Short, Focus, Long, Focus, Short]
        );
    }

    #[test]
    fn the_next_stage_runs_by_itself_when_asked() {
        let config = config(true);
        let mut timer = Timer::new(&config);
        timer.start(0);
        // Seen late: the pause started when the focus ended.
        assert_eq!(timer.tick(&config, 26 * MINUTE), Some(Stage::Focus));
        assert!(timer.running());
        assert_eq!(timer.left_at(26 * MINUTE), 4 * MINUTE);
        assert_eq!(timer.fraction(&config, 26 * MINUTE), 0.2);
    }
}
