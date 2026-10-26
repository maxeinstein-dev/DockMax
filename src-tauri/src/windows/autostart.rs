//! Starting with Windows (spec 0008): the DockMax entry under the user's `Run`
//! key (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`), which is what
//! the "Startup apps" page of Windows lists and lets the user turn off. Only
//! the user's own key: nothing needs rights, and nothing else is touched.

use std::path::Path;

/// The name of the entry in the `Run` key.
pub const ENTRY: &str = "DockMax";

/// The command Windows runs at sign-in: the program, in quotes (the path may
/// have spaces).
pub fn command_for(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

#[cfg(windows)]
mod registry {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
        RegOpenKeyExW, RegSetValueExW,
    };
    use windows::core::HSTRING;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn open() -> Result<HKEY, String> {
        let mut key = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(RUN_KEY),
                None,
                KEY_SET_VALUE,
                &mut key,
            )
        };
        status
            .ok()
            .map(|()| key)
            .map_err(|error| format!("the Run key could not be opened: {error}"))
    }

    /// Writes (or, with `None`, removes) the entry.
    pub fn write(command: Option<&str>) -> Result<(), String> {
        let key = open()?;
        let name = HSTRING::from(super::ENTRY);
        let result = match command {
            Some(command) => {
                // REG_SZ is UTF-16, with its terminating null, as bytes.
                let wide: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
                let bytes: Vec<u8> = wide.iter().flat_map(|unit| unit.to_le_bytes()).collect();
                unsafe { RegSetValueExW(key, &name, None, REG_SZ, Some(&bytes)) }
                    .ok()
                    .map_err(|error| format!("the entry could not be written: {error}"))
            }
            None => {
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
    if !enabled {
        return registry::write(None);
    }
    let exe =
        std::env::current_exe().map_err(|error| format!("no path of the program: {error}"))?;
    registry::write(Some(&command_for(&exe)))
}

#[cfg(not(windows))]
pub fn apply(_enabled: bool) -> Result<(), String> {
    Err("not Windows".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_is_the_path_in_quotes() {
        assert_eq!(
            command_for(Path::new(r"C:\Program Files\DockMax\dockmax.exe")),
            r#""C:\Program Files\DockMax\dockmax.exe""#
        );
    }
}
