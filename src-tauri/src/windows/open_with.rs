//! Opens a program of the dock with files or a folder (spec 0018): the
//! files dropped on its tile, or the folder its tile was dropped on.
//!
//! The paths go to the program as when they are dropped on its shortcut: on
//! its command line, after the shortcut's own arguments. A Store app opens
//! them as "Abrir com" does. The terminals open *in* the folder instead, and
//! File Explorer opens the folder (or the file, with its program).

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, IDataObject};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    ASSOC_FILTER_NONE, ApplicationActivationManager, BHID_DataObject,
    IApplicationActivationManager, IAssocHandler, IObjectWithAppUserModelID, IShellItemArray,
    SHAssocEnumHandlers, SHCreateShellItemArrayFromIDLists, SHParseDisplayName, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, SW_SHOWNORMAL, TranslateMessage,
};
use windows::core::{HSTRING, Interface, PCWSTR, PWSTR};

/// A program that opens in a folder rather than with it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shell {
    /// Windows Terminal: `wt -d <folder>`.
    Terminal,
    /// A console (Prompt de Comando, PowerShell): its program, started in
    /// the folder.
    Console(PathBuf),
    /// File Explorer: it opens the folder, or the file with its program.
    Explorer,
}

/// Whether the program `id` of the Apps folder, running `target`, is a
/// terminal.
fn shell_of(id: &str, target: Option<&Path>) -> Option<Shell> {
    let name = target
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().to_ascii_lowercase());
    if id.starts_with("Microsoft.WindowsTerminal")
        || matches!(name.as_deref(), Some("wt.exe" | "windowsterminal.exe"))
    {
        return Some(Shell::Terminal);
    }
    if id == "Microsoft.Windows.Explorer" || name.as_deref() == Some("explorer.exe") {
        return Some(Shell::Explorer);
    }
    match (name.as_deref(), target) {
        (Some("cmd.exe" | "powershell.exe" | "pwsh.exe"), Some(target)) => {
            Some(Shell::Console(target.to_path_buf()))
        }
        _ => None,
    }
}

/// The folder a terminal opens in: the first path, or its folder when it is
/// a file.
fn folder_of(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?;
    if first.is_dir() {
        Some(first.clone())
    } else {
        first.parent().map(Path::to_path_buf)
    }
}

/// `"<path>"` for a command line: a folder that ends with `\` (a drive)
/// gets a `.`, or the `\` would escape the quote.
fn quoted(path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.ends_with('\\') {
        format!("\"{text}.\"")
    } else {
        format!("\"{text}\"")
    }
}

fn execute(file: &HSTRING, arguments: Option<&str>, folder: Option<&Path>) -> Result<(), String> {
    let arguments = arguments.map(HSTRING::from);
    let folder = folder.map(|folder| HSTRING::from(folder.as_os_str()));
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            file,
            arguments
                .as_ref()
                .map_or(PCWSTR::null(), |text| PCWSTR(text.as_ptr())),
            folder
                .as_ref()
                .map_or(PCWSTR::null(), |text| PCWSTR(text.as_ptr())),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute answers a number above 32 when it worked.
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute answered {}", result.0 as usize))
    }
}

/// The files of `paths` as the shell's items.
fn items_of(paths: &[PathBuf]) -> windows::core::Result<IShellItemArray> {
    let mut lists: Vec<*mut ITEMIDLIST> = Vec::new();
    let result = (|| {
        for path in paths {
            let mut list = std::ptr::null_mut();
            unsafe {
                SHParseDisplayName(&HSTRING::from(path.as_os_str()), None, &mut list, 0, None)?
            };
            lists.push(list);
        }
        let constant: Vec<*const ITEMIDLIST> = lists.iter().map(|list| list.cast_const()).collect();
        unsafe { SHCreateShellItemArrayFromIDLists(&constant) }
    })();
    for list in lists {
        unsafe { CoTaskMemFree(Some(list.cast_const().cast())) };
    }
    result
}

/// The extension the files of `paths` share, with its dot: none for a
/// folder, or for files of different kinds.
fn shared_extension(paths: &[PathBuf]) -> Option<String> {
    let mut extensions = paths.iter().map(|path| {
        path.is_file()
            .then(|| path.extension())
            .flatten()
            .map(|extension| format!(".{}", extension.to_string_lossy().to_lowercase()))
    });
    let first = extensions.next()??;
    extensions
        .all(|each| each.as_deref() == Some(&first))
        .then_some(first)
}

/// Opens the files of `paths` with the program `id` (running `target`) as
/// "Abrir com" does: its handler for their kind, found by its AppID or its
/// program. Whether it was one of them.
fn open_with_handler(
    id: &str,
    target: Option<&Path>,
    paths: &[PathBuf],
) -> windows::core::Result<bool> {
    let Some(extension) = shared_extension(paths) else {
        return Ok(false);
    };
    let handlers = unsafe { SHAssocEnumHandlers(&HSTRING::from(extension), ASSOC_FILTER_NONE)? };
    loop {
        let mut found: [Option<IAssocHandler>; 1] = [None];
        let mut got = 0;
        if unsafe { handlers.Next(&mut found, Some(&mut got)) }.is_err() || got == 0 {
            return Ok(false);
        }
        let Some(handler) = found[0].take() else {
            return Ok(false);
        };
        let app = handler
            .cast::<IObjectWithAppUserModelID>()
            .ok()
            .and_then(|object| unsafe { object.GetAppID() }.ok())
            .and_then(taken);
        let program = unsafe { handler.GetName() }.ok().and_then(taken);
        let same_program = target
            .zip(program.as_deref())
            .is_some_and(|(target, program)| {
                target
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(program)
            });
        if app.as_deref() == Some(id) || same_program {
            let data: IDataObject =
                unsafe { items_of(paths)?.BindToHandler(None, &BHID_DataObject)? };
            unsafe { handler.Invoke(&data)? };
            return Ok(true);
        }
    }
}

/// The text of a string the shell gave, freed.
fn taken(text: PWSTR) -> Option<String> {
    let value = unsafe { text.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(text.0.cast_const().cast())) };
    value
}

/// Opens the Store app `id` with `paths`, as "Abrir com" does.
fn activate_for_files(id: &str, paths: &[PathBuf]) -> windows::core::Result<()> {
    let manager: IApplicationActivationManager =
        unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_ALL)? };
    let items = items_of(paths)?;
    unsafe { manager.ActivateForFile(&HSTRING::from(id), &items, &HSTRING::from("open"))? };
    Ok(())
}

/// Opens the program `id` with `paths`: a terminal in their folder, File
/// Explorer on the first, a Store app as "Abrir com" does, any other with
/// them on its command line.
pub fn open(id: &str, paths: &[PathBuf]) -> Result<(), String> {
    super::apps::init_com();
    let target = super::apps::target_of(id);
    match shell_of(id, target.as_deref()) {
        Some(Shell::Terminal) => {
            let folder = folder_of(paths).ok_or("nothing to open")?;
            return execute(
                &HSTRING::from("wt.exe"),
                Some(&format!("-d {}", quoted(&folder))),
                None,
            );
        }
        Some(Shell::Console(program)) => {
            let folder = folder_of(paths).ok_or("nothing to open")?;
            return execute(&HSTRING::from(program.as_os_str()), None, Some(&folder));
        }
        Some(Shell::Explorer) => {
            let first = paths.first().ok_or("nothing to open")?;
            return execute(&HSTRING::from(first.as_os_str()), None, None);
        }
        None => {}
    }
    // Files open with the program's handler for their kind, as "Abrir com"
    // does: a Store app's too.
    match open_with_handler(id, target.as_deref(), paths) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(error) => log::info!("open with: the handler did not open them: {error}"),
    }
    if id.contains('!') {
        match activate_for_files(id, paths) {
            Ok(()) => return Ok(()),
            Err(error) => log::info!("open with: the Store app took no files: {error}"),
        }
    }
    let program = target.ok_or("the program takes no files")?;
    if !program.is_file() {
        return Err("the program has no file to run".into());
    }
    // The shortcut's own arguments first (a Chrome profile, say).
    let arguments = super::apps::arguments_of(id)
        .into_iter()
        .chain(paths.iter().map(|path| quoted(path)))
        .collect::<Vec<_>>()
        .join(" ");
    execute(
        &HSTRING::from(program.as_os_str()),
        Some(&arguments),
        folder_of(paths).as_deref(),
    )
}

/// [`open`] on a thread of its own, which keeps answering the shell a
/// moment after: a program's drop may still read the files then.
pub fn open_off_thread(id: String, paths: Vec<PathBuf>) {
    off_thread(move || {
        if let Err(error) = open(&id, &paths) {
            log::warn!("open with: the program could not open them: {error}");
        }
    });
}

/// A program's tile let go off the dock at the screen point `(x, y)`: it
/// opens with the folder or the file there, if any (spec 0018).
pub fn open_at(id: String, x: i32, y: i32) {
    off_thread(move || match super::shell_target::at(x, y) {
        Some(path) => {
            if let Err(error) = open(&id, &[path]) {
                log::warn!("open with: the program could not open it: {error}");
            }
        }
        None => log::info!("open with: no folder there"),
    });
}

fn off_thread(work: impl FnOnce() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("dock-open-with".into())
        .spawn(move || {
            work();
            let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut message = MSG::default();
            while std::time::Instant::now() < until {
                while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                    unsafe {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
    if let Err(error) = spawned {
        log::warn!("open with: no thread: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_terminals_open_in_the_folder() {
        assert_eq!(
            shell_of("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App", None),
            Some(Shell::Terminal)
        );
        let cmd = PathBuf::from(r"C:\Windows\System32\cmd.exe");
        assert_eq!(
            shell_of(
                "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\cmd.exe",
                Some(&cmd)
            ),
            Some(Shell::Console(cmd.clone()))
        );
        let pwsh = PathBuf::from(r"C:\Program Files\PowerShell\7\PWSH.EXE");
        assert_eq!(
            shell_of("Microsoft.PowerShell", Some(&pwsh)),
            Some(Shell::Console(pwsh.clone()))
        );
        let code = PathBuf::from(r"C:\Users\x\AppData\Local\Programs\Microsoft VS Code\Code.exe");
        assert_eq!(shell_of("Microsoft.VisualStudioCode", Some(&code)), None);
        assert_eq!(shell_of("MSEdge", None), None);
        assert_eq!(
            shell_of("Microsoft.Windows.Explorer", None),
            Some(Shell::Explorer)
        );
    }

    #[test]
    fn a_drive_keeps_its_quote() {
        assert_eq!(quoted(Path::new(r"C:\")), r#""C:\.""#);
        assert_eq!(
            quoted(Path::new(r"C:\Projetos\Dockyard")),
            r#""C:\Projetos\Dockyard""#
        );
    }

    #[test]
    fn files_of_one_kind_share_their_extension() {
        let here = std::env::temp_dir();
        let a = here.join("dockyard-ext-a.TXT");
        let b = here.join("dockyard-ext-b.txt");
        let c = here.join("dockyard-ext-c.md");
        for file in [&a, &b, &c] {
            std::fs::write(file, "").unwrap();
        }
        assert_eq!(
            shared_extension(&[a.clone(), b.clone()]).as_deref(),
            Some(".txt")
        );
        assert_eq!(shared_extension(&[a.clone(), c.clone()]), None);
        assert_eq!(shared_extension(std::slice::from_ref(&here)), None);
        for file in [a, b, c] {
            let _ = std::fs::remove_file(file);
        }
    }

    #[test]
    fn a_file_opens_the_terminal_in_its_folder() {
        let here = std::env::temp_dir();
        assert_eq!(folder_of(std::slice::from_ref(&here)), Some(here.clone()));
        let file = here.join("dockyard-not-a-folder.txt");
        assert_eq!(folder_of(&[file]), Some(here));
        assert_eq!(folder_of(&[]), None);
    }
}

#[cfg(test)]
mod machine {
    /// Opens a temporary text file with the Bloco de notas of this machine,
    /// a Store app, as a drop on its tile does.
    /// `cargo test -- --ignored notepad_opens_a_file`.
    #[test]
    #[ignore = "opens the Bloco de notas of this machine"]
    fn notepad_opens_a_file() {
        let file = std::env::temp_dir().join("teste-dockyard.txt");
        std::fs::write(&file, "teste do Dockyard").unwrap();
        let id = "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App";
        assert_eq!(super::open(id, &[file]), Ok(()));
    }
}
