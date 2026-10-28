//! The DockMax's secrets in the Windows vault (ADR 0012): generic
//! credentials of the user who is signed in, encrypted by Windows. The
//! names start with `DockMax/`; the values are UTF-8 text.

use windows::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
use windows::Win32::Security::Credentials::{
    CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW,
    CredEnumerateW, CredFree, CredReadW, CredWriteW,
};
use windows::core::{HSTRING, PWSTR};

/// The secret named `name`; `None` when there is none (or it can't be read
/// as text).
pub fn read(name: &str) -> Option<String> {
    let mut found: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe { CredReadW(&HSTRING::from(name), CRED_TYPE_GENERIC, None, &mut found) }.ok()?;
    let text = unsafe {
        let credential = &*found;
        let bytes = std::slice::from_raw_parts(
            credential.CredentialBlob,
            credential.CredentialBlobSize as usize,
        );
        String::from_utf8(bytes.to_vec()).ok()
    };
    unsafe { CredFree(found.cast()) };
    text
}

/// Keeps `secret` under `name`, replacing what was there.
pub fn write(name: &str, secret: &str) -> windows::core::Result<()> {
    let name = HSTRING::from(name);
    let mut blob = secret.as_bytes().to_vec();
    let credential = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(name.as_ptr().cast_mut()),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        LastWritten: FILETIME::default(),
        ..Default::default()
    };
    let written = unsafe { CredWriteW(&credential, 0) };
    // The copy of the secret goes as soon as Windows has it.
    blob.fill(0);
    written
}

/// Removes the secret `name`; nothing to remove is not an error.
pub fn delete(name: &str) -> windows::core::Result<()> {
    match unsafe { CredDeleteW(&HSTRING::from(name), CRED_TYPE_GENERIC, None) } {
        Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        other => other,
    }
}

/// The names of the secrets that start with `prefix`, sorted.
pub fn names(prefix: &str) -> Vec<String> {
    let filter = HSTRING::from(format!("{prefix}*"));
    let mut count = 0u32;
    let mut list: *mut *mut CREDENTIALW = std::ptr::null_mut();
    if unsafe { CredEnumerateW(&filter, None, &mut count, &mut list) }.is_err() {
        return Vec::new();
    }
    let mut names: Vec<String> = unsafe {
        std::slice::from_raw_parts(list, count as usize)
            .iter()
            .filter_map(|credential| (**credential).TargetName.to_string().ok())
            .collect()
    };
    unsafe { CredFree(list.cast()) };
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_is_kept_listed_replaced_and_removed() {
        // A name of its own, so it never meets a real one; removed at the end.
        let prefix = format!("DockMax/test-{}/", std::process::id());
        let name = format!("{prefix}one");
        write(&name, "first").unwrap();
        write(&name, "segundo ç").unwrap();
        let read_back = read(&name);
        let listed = names(&prefix);
        delete(&name).unwrap();

        assert_eq!(read_back.as_deref(), Some("segundo ç"));
        assert_eq!(listed, std::slice::from_ref(&name));
        assert_eq!(read(&name), None);
        // Removing what isn't there is fine.
        assert_eq!(delete(&name), Ok(()));
    }
}
