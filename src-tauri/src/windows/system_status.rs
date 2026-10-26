//! What the dock shows at its end while the Windows taskbar is hidden, as the
//! taskbar does (and the PrimoDock): the battery, the network and the
//! volume. Read when asked; cheap enough to ask every few seconds.

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
    GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses, IF_TYPE_IEEE80211, IF_TYPE_SOFTWARE_LOOPBACK,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::Com::{CLSCTX_ALL, CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// The battery: how full, and whether it is charging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// How the computer is online.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Wifi,
    Wired,
    Offline,
}

/// The volume of the default output: 0–100, and whether it is muted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume {
    pub percent: u8,
    pub muted: bool,
}

/// Everything at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// `None` on a computer with no battery: nothing is shown for it.
    pub battery: Option<Battery>,
    pub network: Network,
    /// `None` with no output device.
    pub volume: Option<Volume>,
}

pub fn read() -> Status {
    Status {
        battery: battery(),
        network: network(),
        volume: volume(),
    }
}

fn battery() -> Option<Battery> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    battery_of(&status)
}

/// The battery `status` describes; `None` with none (flag 128) or an unknown
/// charge (255).
fn battery_of(status: &SYSTEM_POWER_STATUS) -> Option<Battery> {
    const NO_BATTERY: u8 = 128;
    const UNKNOWN: u8 = 255;
    if status.BatteryFlag & NO_BATTERY != 0
        || status.BatteryFlag == UNKNOWN
        || status.BatteryLifePercent == UNKNOWN
    {
        return None;
    }
    Some(Battery {
        percent: status.BatteryLifePercent.min(100),
        // 8: charging; or on the charger.
        charging: status.BatteryFlag & 8 != 0 || status.ACLineStatus == 1,
    })
}

/// Online through a Wi-Fi adapter, through another one, or not at all: an
/// adapter counts when it is up and has a gateway.
fn network() -> Network {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 16 * 1024;
    for _ in 0..3 {
        // u64 keeps the buffer aligned for the structure.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let result = unsafe { GetAdaptersAddresses(0, flags, None, Some(first), &mut size) };
        const BUFFER_OVERFLOW: u32 = 111;
        if result == BUFFER_OVERFLOW {
            continue;
        }
        if result != 0 {
            return Network::Offline;
        }
        let mut found = Network::Offline;
        let mut at = first.cast_const();
        while !at.is_null() {
            let adapter = unsafe { &*at };
            let online = adapter.OperStatus == IfOperStatusUp
                && adapter.IfType != IF_TYPE_SOFTWARE_LOOPBACK
                && !adapter.FirstGatewayAddress.is_null();
            if online {
                if adapter.IfType == IF_TYPE_IEEE80211 {
                    return Network::Wifi;
                }
                found = Network::Wired;
            }
            at = adapter.Next;
        }
        return found;
    }
    Network::Offline
}

/// The default output's volume. Needs COM on the calling thread (the dock's).
fn volume() -> Option<Volume> {
    unsafe {
        let devices: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER).ok()?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let control: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let level = control.GetMasterVolumeLevelScalar().ok()?;
        let muted = control.GetMute().ok()?.as_bool();
        Some(Volume {
            percent: (level.clamp(0.0, 1.0) * 100.0).round() as u8,
            muted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn power(flag: u8, percent: u8, line: u8) -> SYSTEM_POWER_STATUS {
        SYSTEM_POWER_STATUS {
            ACLineStatus: line,
            BatteryFlag: flag,
            BatteryLifePercent: percent,
            ..Default::default()
        }
    }

    #[test]
    fn a_desktop_has_no_battery_to_show() {
        assert_eq!(battery_of(&power(128, 255, 1)), None);
        assert_eq!(battery_of(&power(255, 255, 255)), None);
    }

    #[test]
    fn a_laptop_shows_its_charge_and_whether_it_charges() {
        assert_eq!(
            battery_of(&power(1, 80, 0)),
            Some(Battery {
                percent: 80,
                charging: false
            })
        );
        assert_eq!(
            battery_of(&power(8, 40, 1)),
            Some(Battery {
                percent: 40,
                charging: true
            })
        );
    }
}
