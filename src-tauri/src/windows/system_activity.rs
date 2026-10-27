//! Reads what the system widgets show (spec 0017 F1): the processor's
//! times, the memory and the network's byte counters. Each read is cheap
//! (a few system calls) and is made only while a widget that shows it is in
//! a dock; the arithmetic is `domain::system_activity`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::FILETIME;
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, IF_TYPE_SOFTWARE_LOOPBACK, MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

use crate::domain::system_activity::{CpuTimes, Memory, NetCounters, Reading, Sampler};

fn ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

/// The processor's times now; `None` if Windows doesn't give them.
pub fn cpu() -> Option<CpuTimes> {
    let (mut idle, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    Some(CpuTimes {
        idle: ticks(idle),
        kernel: ticks(kernel),
        user: ticks(user),
    })
}

/// The memory now.
pub fn memory() -> Option<Memory> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some(Memory {
        total: status.ullTotalPhys,
        available: status.ullAvailPhys,
    })
}

/// The bytes received and sent by the adapters that are up and are real
/// hardware (no loopback, no virtual filter adapters, which would count the
/// same bytes twice).
pub fn network() -> Option<NetCounters> {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    unsafe { GetIfTable2(&mut table) }.ok().ok()?;
    if table.is_null() {
        return None;
    }
    let mut counters = NetCounters::default();
    unsafe {
        let count = (*table).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), count);
        for row in rows {
            // The first bit of the flags: HardwareInterface.
            let hardware = row.InterfaceAndOperStatusFlags._bitfield & 1 == 1;
            if hardware && row.Type != IF_TYPE_SOFTWARE_LOOPBACK && row.OperStatus == IfOperStatusUp
            {
                counters.received = counters.received.saturating_add(row.InOctets);
                counters.sent = counters.sent.saturating_add(row.OutOctets);
            }
        }
        FreeMibTable(table.cast());
    }
    Some(counters)
}

/// The one sampler of the app, shared by the docks of every monitor and the
/// panel, with when it last read.
static SAMPLER: Mutex<Option<(Instant, Sampler)>> = Mutex::new(None);

/// Readings closer than this reuse the last one: two docks, or a dock and
/// the panel, asking in the same moment read the system once.
const FRESH: Duration = Duration::from_millis(1_500);

/// What the system widgets show now: read again unless the last reading is
/// fresh.
pub fn sample() -> Reading {
    let mut sampler = SAMPLER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    match sampler.as_mut() {
        Some((at, sampler)) if now.duration_since(*at) < FRESH => sampler.reading().clone(),
        Some((at, sampler)) => {
            let seconds = now.duration_since(*at).as_secs_f64();
            *at = now;
            sampler.feed(seconds, cpu(), memory(), network()).clone()
        }
        None => {
            let mut fresh = Sampler::default();
            let reading = fresh.feed(0.0, cpu(), memory(), network()).clone();
            *sampler = Some((now, fresh));
            reading
        }
    }
}
