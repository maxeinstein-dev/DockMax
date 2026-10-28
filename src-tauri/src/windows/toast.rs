//! Windows notices (spec 0017 F2b-2): a toast of Windows with the DockMax's
//! name. The app is not packaged, so its name for the notices
//! (AppUserModelID) is registered under the user's classes the first time,
//! as Windows asks of desktop apps.
//!
//! The Ciclo's notice is neutral: "Lembrete do Ciclo", nothing else. What
//! the reminder says shows only in the Ciclo's panel.

use std::sync::OnceLock;

use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{
    NotificationSetting, ToastNotification, ToastNotificationManager,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegSetValueExW,
};
use windows::core::{HSTRING, PCWSTR};

/// The app's identifier (`tauri.conf.json`), set when it starts.
static IDENTIFIER: OnceLock<String> = OnceLock::new();

/// Keeps the app's identifier, the name of its notices.
pub fn set_identifier(identifier: &str) {
    let _ = IDENTIFIER.set(identifier.to_owned());
}

fn identifier() -> &'static str {
    IDENTIFIER
        .get()
        .map(String::as_str)
        .unwrap_or("dev.maxsueleinstein.dockmax")
}

/// `text` safe inside XML.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The toast's XML: a title and a line.
fn toast_xml(title: &str, line: &str) -> String {
    format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escaped(title),
        escaped(line)
    )
}

/// Registers the app's name for its notices: `HKCU\Software\Classes\
/// AppUserModelId\<identifier>`, with the name Windows shows over them.
fn register() -> windows::core::Result<()> {
    let path = HSTRING::from(format!(
        "Software\\Classes\\AppUserModelId\\{}",
        identifier()
    ));
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &path,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let name: Vec<u16> = "DockMax\0".encode_utf16().collect();
        let bytes = std::slice::from_raw_parts(name.as_ptr().cast::<u8>(), name.len() * 2);
        let set = RegSetValueExW(
            key,
            &HSTRING::from("DisplayName"),
            None,
            REG_SZ,
            Some(bytes),
        );
        let _ = RegCloseKey(key);
        set.ok()
    }
}

/// Shows a toast with `title` and `line`: `false` when Windows has the
/// notices off (for every app or for the DockMax), where it would take the
/// toast and show nothing.
pub fn show(title: &str, line: &str) -> windows::core::Result<bool> {
    // WinRT on a thread of its own (the calendar's): COM first. A thread
    // already set up another way answers an error that changes nothing.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    register()?;
    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(toast_xml(title, line)))?;
    let toast = ToastNotification::CreateToastNotification(&xml)?;
    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(identifier()))?;
    if notifier.Setting()? != NotificationSetting::Enabled {
        return Ok(false);
    }
    notifier.Show(&toast)?;
    Ok(true)
}

/// The Ciclo's notice: neutral, the same for every notice and reminder.
pub fn cycle_notice() -> windows::core::Result<bool> {
    let text = &crate::i18n::messages().dock;
    show(text.cycle_notice_title, text.cycle_notice_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toast_says_only_its_neutral_text() {
        let xml = toast_xml("Lembrete do Ciclo", "Abra o <Ciclo> & veja");
        assert!(xml.contains("<text>Lembrete do Ciclo</text>"));
        assert!(xml.contains("<text>Abra o &lt;Ciclo&gt; &amp; veja</text>"));
        assert!(
            XmlDocument::new()
                .and_then(|doc| doc.LoadXml(&HSTRING::from(xml.as_str())))
                .is_ok()
        );
    }
}
