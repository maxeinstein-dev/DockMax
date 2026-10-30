//! Starting with Windows (spec 0008): the Dockyard entry under the user's `Run`
//! key (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`), which is what
//! the "Startup apps" page of Windows lists and lets the user turn off. Only
//! the user's own keys: nothing needs rights. Besides the `Run` key, only the
//! entry's own line of `Explorer\StartupApproved\Run` (where that page keeps
//! which entries are off), when the old entry is taken over.

use std::path::Path;

/// The name of the entry in the `Run` key.
pub const ENTRY: &str = "Dockyard";

/// The name of the entry before the Dockyard was renamed (it was the
/// DockMax). It points at the old program; a run of this one takes its place
/// (`migrate_legacy`).
pub const LEGACY_ENTRY: &str = "DockMax";

/// The command Windows runs at sign-in: the program, in quotes (the path may
/// have spaces).
pub fn command_for(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// Whether the "Startup apps" page of Windows leaves an entry on, from what
/// it keeps of it under `Explorer\StartupApproved\Run`: a binary value whose
/// first byte is even when on and odd when the user turned it off. No value
/// (never touched there): on.
pub fn approved(value: Option<&[u8]>) -> bool {
    value
        .and_then(|bytes| bytes.first())
        .is_none_or(|first| first % 2 == 0)
}

/// Whether the old entry started the program at sign-in: it is in the `Run`
/// key and the "Startup apps" page leaves it on.
pub fn legacy_was_on(present: bool, approval: Option<&[u8]>) -> bool {
    present && approved(approval)
}

#[cfg(windows)]
mod registry {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_BINARY, REG_SAM_FLAGS, REG_SZ,
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows::core::HSTRING;

    pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// Where the "Startup apps" page of Windows keeps which entries are off.
    pub const APPROVED_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn open(path: &str, access: REG_SAM_FLAGS) -> Result<HKEY, String> {
        let mut key = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(path),
                None,
                access,
                &mut key,
            )
        };
        status
            .ok()
            .map(|()| key)
            .map_err(|error| format!("the key {path} could not be opened: {error}"))
    }

    /// The bytes of the value `name` under `path`; `None` when the key or the
    /// value is not there (or can't be read).
    pub fn read(path: &str, name: &str) -> Option<Vec<u8>> {
        let key = open(path, KEY_QUERY_VALUE).ok()?;
        let name = HSTRING::from(name);
        let mut size = 0u32;
        let mut value = None;
        let sized =
            unsafe { RegQueryValueExW(key, &name, None, None, None, Some(&mut size)) }.is_ok();
        if sized {
            let mut bytes = vec![0u8; size as usize];
            let read = unsafe {
                RegQueryValueExW(
                    key,
                    &name,
                    None,
                    None,
                    Some(bytes.as_mut_ptr()),
                    Some(&mut size),
                )
            }
            .is_ok();
            if read {
                bytes.truncate(size as usize);
                value = Some(bytes);
            }
        }
        let _ = unsafe { RegCloseKey(key) };
        value
    }

    /// What a value becomes.
    pub enum Value<'a> {
        Text(&'a str),
        Binary(&'a [u8]),
        Gone,
    }

    /// Writes the value `name` under `path`, or removes it (one that is not
    /// there is no error).
    pub fn write(path: &str, name: &str, value: Value<'_>) -> Result<(), String> {
        let key = open(path, KEY_SET_VALUE)?;
        let name = HSTRING::from(name);
        let result = match value {
            Value::Text(text) => {
                // REG_SZ is UTF-16, with its terminating null, as bytes.
                let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                let bytes: Vec<u8> = wide.iter().flat_map(|unit| unit.to_le_bytes()).collect();
                unsafe { RegSetValueExW(key, &name, None, REG_SZ, Some(&bytes)) }
                    .ok()
                    .map_err(|error| format!("the entry could not be written: {error}"))
            }
            Value::Binary(bytes) => {
                unsafe { RegSetValueExW(key, &name, None, REG_BINARY, Some(bytes)) }
                    .ok()
                    .map_err(|error| format!("the value could not be written: {error}"))
            }
            Value::Gone => {
                let status = unsafe { RegDeleteValueW(key, &name) };
                if status == ERROR_FILE_NOT_FOUND {
                    Ok(())
                } else {
                    status
                        .ok()
                        .map_err(|error| format!("the entry could not be removed: {error}"))
                }
            }
        };
        let _ = unsafe { RegCloseKey(key) };
        result
    }
}

/// Makes the entry as `enabled` says: it runs this program at sign-in, or it
/// is gone. Turning it on again rewrites the path, so a program that moved
/// (an update) is found again.
#[cfg(windows)]
pub fn apply(enabled: bool) -> Result<(), String> {
    use registry::{RUN_KEY, Value};
    if !enabled {
        return registry::write(RUN_KEY, ENTRY, Value::Gone);
    }
    let exe =
        std::env::current_exe().map_err(|error| format!("no path of the program: {error}"))?;
    registry::write(RUN_KEY, ENTRY, Value::Text(&command_for(&exe)))
}

#[cfg(not(windows))]
pub fn apply(_enabled: bool) -> Result<(), String> {
    Err("not Windows".to_owned())
}

/// Takes the place of the entry of the DockMax, the name before the rename:
/// it started the old program. When it is there, it goes away; if it was on,
/// the Dockyard's is written with this program, so the user keeps starting
/// with Windows. What the "Startup apps" page keeps of it (on or off) moves to
/// the new name, so an entry the user turned off there stays off. Returns
/// whether the old entry was on. Nothing to do (the usual case): `Ok(false)`.
#[cfg(windows)]
pub fn migrate_legacy() -> Result<bool, String> {
    use registry::{APPROVED_KEY, RUN_KEY, Value};
    if registry::read(RUN_KEY, LEGACY_ENTRY).is_none() {
        return Ok(false);
    }
    let approval = registry::read(APPROVED_KEY, LEGACY_ENTRY);
    let on = legacy_was_on(true, approval.as_deref());
    // The new one first: a failure halfway leaves the old entry, and the next
    // run tries again.
    if let Some(bytes) = &approval {
        registry::write(APPROVED_KEY, ENTRY, Value::Binary(bytes))?;
    }
    if on {
        apply(true)?;
    }
    registry::write(RUN_KEY, LEGACY_ENTRY, Value::Gone)?;
    if approval.is_some() {
        registry::write(APPROVED_KEY, LEGACY_ENTRY, Value::Gone)?;
    }
    Ok(on)
}

#[cfg(not(windows))]
pub fn migrate_legacy() -> Result<bool, String> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_is_the_path_in_quotes() {
        assert_eq!(
            command_for(Path::new(r"C:\Program Files\Dockyard\dockyard.exe")),
            r#""C:\Program Files\Dockyard\dockyard.exe""#
        );
    }

    #[test]
    fn the_entry_has_the_new_name_and_the_old_one_is_the_dockmax() {
        assert_eq!(ENTRY, "Dockyard");
        assert_eq!(LEGACY_ENTRY, "DockMax");
    }

    #[test]
    fn an_entry_the_startup_apps_page_never_touched_is_on() {
        assert!(approved(None));
        assert!(approved(Some(&[])));
    }

    #[test]
    fn an_even_first_byte_is_on_and_an_odd_one_is_off() {
        // As Windows writes them: 02 on, 03 off (and 06/07 for some entries).
        assert!(approved(Some(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])));
        assert!(approved(Some(&[0x06, 0, 0, 0])));
        assert!(!approved(Some(&[
            0x03, 0x1a, 0x2b, 0, 0, 0, 0, 0, 0, 0, 0, 0
        ])));
        assert!(!approved(Some(&[0x07])));
    }

    #[test]
    fn the_old_entry_was_on_only_when_present_and_not_turned_off() {
        assert!(legacy_was_on(true, None));
        assert!(legacy_was_on(true, Some(&[0x02])));
        assert!(!legacy_was_on(true, Some(&[0x03])));
        assert!(!legacy_was_on(false, None));
        assert!(!legacy_was_on(false, Some(&[0x02])));
    }
}
