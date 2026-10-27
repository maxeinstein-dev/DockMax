//! The arithmetic of the system widgets (spec 0017 F1): the processor's use
//! between two readings of its times, the memory in use, the network's rates
//! between two readings of its byte counters, and the history the graph
//! draws. Pure: `windows::system_activity` reads the numbers.

use std::collections::VecDeque;

use serde::Serialize;

/// The processor's times since the computer started, in 100 ns units, as
/// `GetSystemTimes` gives them: `kernel` includes `idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// How busy the processor was between two readings, 0–100; `None` when no
/// time passed (or the counters went back, as after a sleep).
pub fn cpu_percent(before: CpuTimes, after: CpuTimes) -> Option<f64> {
    let idle = after.idle.checked_sub(before.idle)?;
    let total = after
        .kernel
        .checked_sub(before.kernel)?
        .checked_add(after.user.checked_sub(before.user)?)?;
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle);
    Some((busy as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

/// The memory: what is in use, of the total, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Memory {
    pub total: u64,
    pub available: u64,
}

impl Memory {
    /// The share in use, 0–100; `None` with no total.
    pub fn percent(self) -> Option<f64> {
        (self.total > 0).then(|| {
            let used = self.total.saturating_sub(self.available);
            used as f64 / self.total as f64 * 100.0
        })
    }
}

/// The bytes the network adapters received and sent since they started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NetCounters {
    pub received: u64,
    pub sent: u64,
}

/// Bytes per second down and up between two readings `seconds` apart. A
/// counter that went back (an adapter came or went) counts as 0.
pub fn net_rates(before: NetCounters, after: NetCounters, seconds: f64) -> (f64, f64) {
    if seconds <= 0.0 {
        return (0.0, 0.0);
    }
    let rate = |a: u64, b: u64| b.checked_sub(a).map_or(0.0, |d| d as f64 / seconds);
    (
        rate(before.received, after.received),
        rate(before.sent, after.sent),
    )
}

/// A rate as people read it, the Brazilian way: `0 KB/s`, `340 KB/s`,
/// `1,2 MB/s`, `12 MB/s`, `1,1 GB/s`. Bytes, not bits, as the Task Manager's
/// "Desempenho" shows a file copy.
pub fn rate_text(bytes_per_second: f64) -> String {
    let kb = bytes_per_second.max(0.0) / 1024.0;
    if kb < 1000.0 {
        return format!("{} KB/s", kb.round() as u64);
    }
    let mb = kb / 1024.0;
    if mb < 1000.0 {
        return format!("{} MB/s", one_decimal(mb));
    }
    format!("{} GB/s", one_decimal(mb / 1024.0))
}

/// A number with one decimal below ten and none above, with a comma.
fn one_decimal(value: f64) -> String {
    if value < 10.0 {
        format!("{:.1}", value).replace('.', ",")
    } else {
        format!("{}", value.round() as u64)
    }
}

/// A rate in the few characters a one-icon tile has room for: `0K`,
/// `340K`, `1,2M`, `12M`, `1,1G`.
pub fn rate_short(bytes_per_second: f64) -> String {
    let text = rate_text(bytes_per_second);
    let (number, unit) = text.split_once(' ').unwrap_or((&text, ""));
    format!("{number}{}", unit.chars().next().unwrap_or('K'))
}

/// The last `capacity` values, oldest first: what a graph draws.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct History {
    values: VecDeque<f64>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> History {
        History {
            values: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.capacity == 0 {
            return;
        }
        if self.values.len() == self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        self.values.iter().copied()
    }
}

/// How many readings the graphs keep: a minute, at one every 2 s.
pub const HISTORY: usize = 30;

/// What the system widgets show now: the dock's tiles and the panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    /// The processor's use, 0–100; `None` until two readings were made.
    pub cpu: Option<f64>,
    pub memory: Option<Memory>,
    /// Bytes per second.
    pub down: f64,
    pub up: f64,
    /// The last [`HISTORY`] values, oldest first.
    pub cpu_history: Vec<f64>,
    pub down_history: Vec<f64>,
    pub up_history: Vec<f64>,
}

/// Turns the system's counters, read one after another, into a
/// [`Reading`]: the processor's use and the rates are the change since the
/// reading before.
#[derive(Debug, Clone)]
pub struct Sampler {
    cpu_times: Option<CpuTimes>,
    counters: Option<NetCounters>,
    cpu: History,
    down: History,
    up: History,
    reading: Reading,
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler {
            cpu_times: None,
            counters: None,
            cpu: History::new(HISTORY),
            down: History::new(HISTORY),
            up: History::new(HISTORY),
            reading: Reading {
                cpu: None,
                memory: None,
                down: 0.0,
                up: 0.0,
                cpu_history: Vec::new(),
                down_history: Vec::new(),
                up_history: Vec::new(),
            },
        }
    }
}

impl Sampler {
    /// Takes the counters read now, `seconds` after the ones before (any
    /// value the first time). A counter that couldn't be read leaves its
    /// value out.
    pub fn feed(
        &mut self,
        seconds: f64,
        cpu: Option<CpuTimes>,
        memory: Option<Memory>,
        counters: Option<NetCounters>,
    ) -> &Reading {
        let percent = match (self.cpu_times, cpu) {
            (Some(before), Some(after)) => cpu_percent(before, after),
            _ => None,
        };
        self.cpu_times = cpu;
        let (down, up) = match (self.counters, counters) {
            (Some(before), Some(after)) => net_rates(before, after, seconds),
            _ => (0.0, 0.0),
        };
        let first = self.counters.is_none();
        self.counters = counters;
        if let Some(percent) = percent {
            self.cpu.push(percent);
        }
        if !first && counters.is_some() {
            self.down.push(down);
            self.up.push(up);
        }
        self.reading = Reading {
            cpu: percent,
            memory,
            down,
            up,
            cpu_history: self.cpu.values().collect(),
            down_history: self.down.values().collect(),
            up_history: self.up.values().collect(),
        };
        &self.reading
    }

    pub fn reading(&self) -> &Reading {
        &self.reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_processor_is_as_busy_as_the_time_it_was_not_idle() {
        let before = CpuTimes {
            idle: 1_000,
            kernel: 3_000,
            user: 1_000,
        };
        // 1000 units passed: 750 idle, so a quarter busy.
        let after = CpuTimes {
            idle: 1_750,
            kernel: 3_800,
            user: 1_200,
        };
        assert_eq!(cpu_percent(before, after), Some(25.0));
        assert_eq!(cpu_percent(before, before), None);
        // The counters went back: no reading.
        assert_eq!(cpu_percent(after, before), None);
    }

    #[test]
    fn the_memory_in_use_is_what_is_not_available() {
        let memory = Memory {
            total: 16,
            available: 4,
        };
        assert_eq!(memory.percent(), Some(75.0));
        assert_eq!(Memory::default().percent(), None);
    }

    #[test]
    fn the_rates_are_bytes_per_second_and_never_negative() {
        let before = NetCounters {
            received: 1_000,
            sent: 500,
        };
        let after = NetCounters {
            received: 5_000,
            sent: 700,
        };
        assert_eq!(net_rates(before, after, 2.0), (2_000.0, 100.0));
        assert_eq!(net_rates(after, before, 2.0), (0.0, 0.0));
        assert_eq!(net_rates(before, after, 0.0), (0.0, 0.0));
    }

    #[test]
    fn the_rates_read_the_brazilian_way() {
        assert_eq!(rate_text(0.0), "0 KB/s");
        assert_eq!(rate_text(340.0 * 1024.0), "340 KB/s");
        assert_eq!(rate_text(1.2 * 1024.0 * 1024.0), "1,2 MB/s");
        assert_eq!(rate_text(12.4 * 1024.0 * 1024.0), "12 MB/s");
        assert_eq!(rate_text(1.1 * 1024.0 * 1024.0 * 1024.0), "1,1 GB/s");
        assert_eq!(rate_short(0.0), "0K");
        assert_eq!(rate_short(340.0 * 1024.0), "340K");
        assert_eq!(rate_short(1.2 * 1024.0 * 1024.0), "1,2M");
    }

    #[test]
    fn the_history_keeps_the_last_values() {
        let mut history = History::new(3);
        for value in [1.0, 2.0, 3.0, 4.0] {
            history.push(value);
        }
        assert_eq!(history.values().collect::<Vec<_>>(), [2.0, 3.0, 4.0]);
    }

    #[test]
    fn the_sampler_measures_the_change_since_the_reading_before() {
        let mut sampler = Sampler::default();
        let times = |idle, kernel| CpuTimes {
            idle,
            kernel,
            user: 0,
        };
        let counters = |received| NetCounters { received, sent: 0 };

        let first = sampler.feed(0.0, Some(times(0, 0)), None, Some(counters(0)));
        assert_eq!(first.cpu, None);
        assert_eq!(first.down, 0.0);
        assert!(first.down_history.is_empty());

        let second = sampler
            .feed(2.0, Some(times(50, 100)), None, Some(counters(4_000)))
            .clone();
        assert_eq!(second.cpu, Some(50.0));
        assert_eq!(second.down, 2_000.0);
        assert_eq!(second.cpu_history, [50.0]);
        assert_eq!(second.down_history, [2_000.0]);

        // The network couldn't be read: no rate, and the next one starts
        // over.
        let third = sampler.feed(2.0, Some(times(150, 200)), None, None).clone();
        assert_eq!(third.cpu, Some(0.0));
        assert_eq!(third.down, 0.0);
        assert_eq!(third.down_history, [2_000.0]);
        assert_eq!(sampler.reading(), &third);
    }
}
