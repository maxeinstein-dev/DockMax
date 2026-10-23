//! The Windows panels the dock opens on a click, in place of the taskbar's
//! own (ADR 0009): the calendar of the clock, and the quick settings of
//! battery, network and volume.
//!
//! Windows opens both from the keyboard (Win+N and Win+A), and that is all
//! it offers: the panels belong to the shell and have no API. The keys are
//! sent as input, the way a hand would press them. They open where Windows
//! puts them, at the corner of the screen.

use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_A, VK_LWIN, VK_N,
};
use serde::Deserialize;

/// A Windows panel the dock can open. Must match `Flyout` in
/// `src/core/system.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Flyout {
    /// Notifications and the calendar (Win+N).
    Calendar,
    /// Wi-Fi, Bluetooth, brightness, volume (Win+A).
    QuickSettings,
}

impl Flyout {
    /// The letter pressed with the Windows key.
    fn key(self) -> VIRTUAL_KEY {
        match self {
            Flyout::Calendar => VK_N,
            Flyout::QuickSettings => VK_A,
        }
    }
}

fn press(key: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// The keys of a flyout, in the order they are pressed and released.
fn keys(flyout: Flyout) -> [INPUT; 4] {
    [
        press(VK_LWIN, false),
        press(flyout.key(), false),
        press(flyout.key(), true),
        press(VK_LWIN, true),
    ]
}

/// Opens the flyout. Fails when Windows took none of the keys (another
/// program blocks the input).
pub fn open(flyout: Flyout) -> Result<(), String> {
    let inputs = keys(flyout);
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(format!("only {sent} of {} keys were sent", inputs.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn describe(input: &INPUT) -> (u16, bool) {
        let key = unsafe { input.Anonymous.ki };
        (key.wVk.0, key.dwFlags == KEYEVENTF_KEYUP)
    }

    #[test]
    fn each_flyout_presses_the_windows_key_and_its_letter() {
        for (flyout, letter) in [(Flyout::Calendar, VK_N), (Flyout::QuickSettings, VK_A)] {
            let keys = keys(flyout);

            // Windows down, letter down, letter up, Windows up.
            let expected = [
                (VK_LWIN.0, false),
                (letter.0, false),
                (letter.0, true),
                (VK_LWIN.0, true),
            ];
            let sent: Vec<_> = keys.iter().map(describe).collect();
            assert_eq!(sent, expected, "{flyout:?}");
        }
    }

    #[test]
    fn the_interface_names_the_flyouts_in_camel_case() {
        assert_eq!(
            serde_json::from_str::<Flyout>("\"calendar\"").unwrap(),
            Flyout::Calendar
        );
        assert_eq!(
            serde_json::from_str::<Flyout>("\"quickSettings\"").unwrap(),
            Flyout::QuickSettings
        );
        assert!(serde_json::from_str::<Flyout>("\"quick-settings\"").is_err());
        assert!(serde_json::from_str::<Flyout>("\"run\"").is_err());
    }
}
