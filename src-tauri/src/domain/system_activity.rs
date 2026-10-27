//! The arithmetic of the system widgets (spec 0017 F1): the processor's use
//! between two readings of its times, the memory in use, the network's rates
//! between two readings of its byte counters, and the history the graph
//! draws. Pure: `windows::system_activity` reads the numbers.

use std::collections::VecDeque;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

    pub fn used(self) -> u64 {
        self.total.saturating_sub(self.available)
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

/// A size of memory, the Brazilian way: `7,8 GB`, `512 MB`.
pub fn size_text(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 {
        format!("{} GB", one_decimal(gb))
    } else {
        format!("{} MB", (bytes as f64 / (1024.0 * 1024.0)).round() as u64)
    }
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

    /// The highest value kept, for the graph's scale; at least `floor`, so a
    /// quiet network doesn't fill the graph with noise.
    pub fn peak(&self, floor: f64) -> f64 {
        self.values.iter().copied().fold(floor, f64::max)
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
        assert_eq!(memory.used(), 12);
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
        assert_eq!(size_text(8 * 1024 * 1024 * 1024), "8,0 GB");
        assert_eq!(size_text(512 * 1024 * 1024), "512 MB");
    }

    #[test]
    fn the_history_keeps_the_last_values() {
        let mut history = History::new(3);
        for value in [1.0, 2.0, 3.0, 4.0] {
            history.push(value);
        }
        assert_eq!(history.values().collect::<Vec<_>>(), [2.0, 3.0, 4.0]);
        assert_eq!(history.peak(10.0), 10.0);
        assert_eq!(history.peak(1.0), 4.0);
    }
}
