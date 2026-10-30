//! The name of a program read from its file: the `FileDescription` of its
//! version resource, or its file name (`domain::program_name`). For the tray
//! icons that come without a tooltip. Kept per file: the tray is listed on
//! every change, and a file's version resource does not change while it runs.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use ::windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use ::windows::core::{HSTRING, PCWSTR};

use crate::domain::program_name;

/// The most files remembered; past it the memory starts over (programs with
/// a tray icon are a handful).
const MAX_REMEMBERED: usize = 256;

static NAMES: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

/// The name of the program in `path`, read once per file.
pub fn of_file(path: &Path) -> Option<String> {
    let key = path.to_string_lossy().to_lowercase();
    if let Some(known) = NAMES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|names| names.get(&key))
    {
        return known.clone();
    }
    let name = program_name::from_file(file_description(path).as_deref(), path);
    let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    let names = names.get_or_insert_with(HashMap::new);
    if names.len() >= MAX_REMEMBERED {
        names.clear();
    }
    names.insert(key, name.clone());
    name
}

/// The `FileDescription` of the file's version resource, in the first
/// language it lists (or US English).
fn file_description(path: &Path) -> Option<String> {
    let file = HSTRING::from(path.as_os_str());
    let size = unsafe { GetFileVersionInfoSizeW(&file, None) };
    if size == 0 {
        return None;
    }
    let mut block = vec![0u8; size as usize];
    unsafe { GetFileVersionInfoW(&file, None, size, block.as_mut_ptr().cast()) }.ok()?;
    let mut languages = Vec::new();
    if let Some(pairs) = query(&block, "\\VarFileInfo\\Translation", false) {
        for pair in pairs.as_chunks::<4>().0 {
            let language = u16::from_le_bytes([pair[0], pair[1]]);
            let code_page = u16::from_le_bytes([pair[2], pair[3]]);
            languages.push(format!("{language:04x}{code_page:04x}"));
        }
    }
    languages.extend(["040904b0".to_string(), "040904e4".to_string()]);
    languages.iter().find_map(|language| {
        let bytes = query(
            &block,
            &format!("\\StringFileInfo\\{language}\\FileDescription"),
            true,
        )?;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .take_while(|&unit| unit != 0)
            .collect();
        let text = String::from_utf16_lossy(&units);
        (!text.trim().is_empty()).then_some(text)
    })
}

/// The bytes of the value `name` in the version block, copied out of it.
/// `text`: the value is a string, whose length Windows gives in UTF-16 units.
fn query(block: &[u8], name: &str, text: bool) -> Option<Vec<u8>> {
    let name = HSTRING::from(name);
    let mut value: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut length = 0u32;
    let found = unsafe {
        VerQueryValueW(
            block.as_ptr().cast(),
            PCWSTR(name.as_ptr()),
            &mut value,
            &mut length,
        )
    };
    if !found.as_bool() || value.is_null() || length == 0 {
        return None;
    }
    // The value points into `block`: read no further than the block holds.
    let start = (value as usize).checked_sub(block.as_ptr() as usize)?;
    let wanted = if text {
        length as usize * 2
    } else {
        length as usize
    };
    let end = start.checked_add(wanted)?.min(block.len());
    block.get(start..end).map(<[u8]>::to_vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_of_windows_has_its_description_and_a_missing_file_its_name() {
        let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        let notepad = Path::new(&windir).join("System32").join("notepad.exe");
        if notepad.exists() {
            let name = of_file(&notepad).unwrap();
            assert!(!name.is_empty());
            assert!(!name.to_lowercase().ends_with(".exe"), "{name}");
            // Read once, then remembered.
            assert_eq!(of_file(&notepad), Some(name));
        }

        assert_eq!(
            of_file(Path::new(r"C:\no\such\dir\Programa.exe")).as_deref(),
            Some("Programa")
        );
    }
}
