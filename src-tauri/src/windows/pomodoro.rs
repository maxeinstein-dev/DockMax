//! The Pomodoros' timers (spec 0017 F2c-2), one per widget: in memory, so
//! they keep counting with the panel closed; a restart of the app starts
//! them over. While one runs, a thread watches the end of its stage: the
//! tile then bounces and keeps a badge until its panel opens, and a notice
//! of Windows comes too when the widget asks for one.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;

use crate::domain::pomodoro::{Stage, Timer, length};
use crate::domain::widgets::PomodoroConfig;

/// Each widget's timer, with the config it runs by.
static TIMERS: Mutex<Vec<(String, Timer, PomodoroConfig)>> = Mutex::new(Vec::new());

/// The Pomodoros (by widget ID) with a stage ended and not seen: a badge on
/// their tile until their panel opens.
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The watching thread is on.
static WATCHING: AtomicBool = AtomicBool::new(false);

/// How often the thread looks.
const EVERY: Duration = Duration::from_millis(500);

fn now() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

/// What the panel shows of a timer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub stage: Stage,
    /// Milliseconds left of the stage, and its whole length.
    pub left: i64,
    pub total: i64,
    pub running: bool,
    /// Focuses done since the last long pause, of `every`.
    pub focuses: u8,
    pub every: u8,
}

/// What the panel asks of a timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Pause,
    Skip,
    Reset,
}

impl Action {
    pub fn parse(text: &str) -> Option<Action> {
        match text {
            "start" => Some(Action::Start),
            "pause" => Some(Action::Pause),
            "skip" => Some(Action::Skip),
            "reset" => Some(Action::Reset),
            _ => None,
        }
    }
}

/// `work` on the timer of `widget` (a new one the first time), with
/// `config` as its config from now on.
fn with_timer<R>(
    widget: &str,
    config: &PomodoroConfig,
    work: impl FnOnce(&mut Timer, &PomodoroConfig, i64) -> R,
) -> R {
    let mut timers = TIMERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let index = match timers.iter().position(|(id, _, _)| id == widget) {
        Some(index) => index,
        None => {
            timers.push((widget.to_owned(), Timer::new(config), config.clone()));
            timers.len() - 1
        }
    };
    let (_, timer, kept) = &mut timers[index];
    if kept != config {
        // New times count from the next stage; a timer waiting at the start
        // of its stage takes the new length now.
        if !timer.running() && timer.left == length(timer.stage, kept) {
            timer.left = length(timer.stage, config);
        }
        *kept = config.clone();
    }
    work(timer, config, now())
}

fn view_of(timer: &Timer, config: &PomodoroConfig, now: i64) -> View {
    View {
        stage: timer.stage,
        left: timer.left_at(now),
        total: length(timer.stage, config),
        running: timer.running(),
        focuses: timer.focuses,
        every: config.every,
    }
}

/// The timer of `widget` as the panel shows it.
pub fn view(widget: &str, config: &PomodoroConfig) -> View {
    with_timer(widget, config, |timer, config, now| {
        view_of(timer, config, now)
    })
}

/// Starts, pauses, skips or starts over the timer of `widget`.
pub fn act(widget: &str, config: &PomodoroConfig, action: Action) -> View {
    let view = with_timer(widget, config, |timer, config, now| {
        match action {
            Action::Start => timer.start(now),
            Action::Pause => timer.pause(now),
            Action::Skip => timer.skip(config),
            Action::Reset => *timer = Timer::new(config),
        }
        view_of(timer, config, now)
    });
    if view.running {
        watch();
    }
    super::native_dock::calendar_changed();
    view
}

/// The tile of `widget`: the share of the stage gone, the minutes left
/// (rounded up), and whether it runs.
pub fn face(widget: &str, config: &PomodoroConfig) -> (f32, i64, bool) {
    with_timer(widget, config, |timer, config, now| {
        let left = timer.left_at(now);
        (
            timer.fraction(config, now),
            (left + 59_999) / 60_000,
            timer.running(),
        )
    })
}

/// Whether the Pomodoro `widget` has a stage ended and not seen: its badge.
pub fn pending(widget: &str) -> bool {
    PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .any(|each| each == widget)
}

/// Its panel opened: the end was seen. Whether it had a badge.
pub fn seen(widget: &str) -> bool {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = pending.len();
    pending.retain(|each| each != widget);
    pending.len() != before
}

/// Starts the thread that watches the running timers, if it is not on.
fn watch() {
    if WATCHING.swap(true, Ordering::AcqRel) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("pomodoro".into())
        .spawn(|| {
            // The minutes each running timer showed, to paint the tile
            // again when they change.
            let mut shown: Vec<(String, i64)> = Vec::new();
            loop {
                std::thread::sleep(EVERY);
                let now = now();
                let (ended, running, minutes) = {
                    let mut timers = TIMERS
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let mut ended = Vec::new();
                    let mut minutes = Vec::new();
                    for (id, timer, config) in timers.iter_mut() {
                        if let Some(stage) = timer.tick(config, now) {
                            ended.push((id.clone(), stage, config.windows));
                        }
                        if timer.running() {
                            minutes.push((id.clone(), (timer.left_at(now) + 59_999) / 60_000));
                        }
                    }
                    let running = timers.iter().any(|(_, timer, _)| timer.running());
                    (ended, running, minutes)
                };
                if !ended.is_empty() {
                    ended_stages(&ended);
                } else if minutes != shown {
                    super::native_dock::calendar_changed();
                }
                shown = minutes;
                if !running {
                    WATCHING.store(false, Ordering::Release);
                    // A timer started meanwhile found the flag still on.
                    let again = TIMERS
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .iter()
                        .any(|(_, timer, _)| timer.running());
                    if !again || WATCHING.swap(true, Ordering::AcqRel) {
                        return;
                    }
                }
            }
        });
    if let Err(error) = spawned {
        WATCHING.store(false, Ordering::Release);
        log::warn!("pomodoro: no thread: {error}");
    }
}

/// Stages ended: the badge and the bounce of their tiles, and a notice of
/// Windows for those that ask.
fn ended_stages(ended: &[(String, Stage, bool)]) {
    let widgets: Vec<String> = ended.iter().map(|(id, _, _)| id.clone()).collect();
    {
        let mut pending = PENDING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for widget in &widgets {
            if !pending.contains(widget) {
                pending.push(widget.clone());
            }
        }
    }
    super::native_dock::nudged(widgets);
    if let Some((_, stage, _)) = ended.iter().find(|(_, _, windows)| *windows) {
        let text = &crate::i18n::messages().dock;
        let line = match stage {
            Stage::Focus => text.pomodoro_focus_done,
            Stage::Short | Stage::Long => text.pomodoro_pause_done,
        };
        match super::toast::show(text.pomodoro, line) {
            Ok(true) => {}
            Ok(false) => log::info!("pomodoro: Windows has the notices off"),
            Err(error) => log::warn!("pomodoro: the notice was not shown: {error}"),
        }
    }
}
