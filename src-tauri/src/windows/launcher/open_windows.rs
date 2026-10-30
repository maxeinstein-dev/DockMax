//! The open windows the launcher lists (spec 0012 L7): the ones with a
//! taskbar button, read by the dock's own enumeration (`openwin::enumerate`,
//! the rule `domain::open_apps::has_button`), without the Dockyard's, each
//! with its program's name and icon as the dock shows them. Switching to one
//! and closing one go through the dock's own calls (`openwin::focus`,
//! `openwin::close`).
//!
//! Reading them is one `EnumWindows` and, for each window with a button, its
//! title, its process's file and its AppID: measured on the author's machine
//! (`reading_the_windows_is_cheap`) well under the 5 ms the launcher's
//! thread can spare, so it is done there, when the launcher opens and at
//! each key.

use crate::domain::launcher_windows::{Window, describe, is_ours};

/// What the launcher knows to describe a window's program: the dock's
/// pinned programs (its id, the program's id and name) and the Apps folder's
/// programs (id and name), as the launcher read them.
pub struct Programs<'a> {
    pub pinned: &'a [(String, String, String)],
    pub catalog: &'a [(String, String)],
}

/// The open windows with a taskbar button, the top of the stacking order
/// first (the one used last), without the Dockyard's own.
pub fn read(programs: &Programs<'_>) -> Vec<Window> {
    super::super::apps::init_com();
    let own_pid = std::process::id();
    let own_exe = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    let dock = super::super::open_apps::programs_of_windows(programs.pinned);
    super::super::openwin::enumerate()
        .into_iter()
        .filter_map(|window| {
            let exe = window
                .exe
                .as_deref()
                .map(|path| path.to_string_lossy().into_owned());
            if is_ours(window.pid, exe.as_deref(), own_pid, own_exe.as_deref()) {
                return None;
            }
            let catalog = window.aumid.as_deref().and_then(|aumid| {
                programs
                    .catalog
                    .iter()
                    .find(|(id, _)| id.eq_ignore_ascii_case(aumid))
                    .cloned()
            });
            let (icon, app) = describe(dock.get(&window.hwnd).cloned(), catalog, exe.as_deref());
            Some(Window {
                hwnd: window.hwnd,
                title: window.title,
                app,
                icon,
                exe,
            })
        })
        .collect()
}

/// Whether `hwnd` is still an open window of another program with a
/// taskbar button: what Enter and "Fechar janela" are checked against before
/// they act (the handle was read a moment ago, and Windows reuses them).
pub fn still_listed(hwnd: isize) -> bool {
    let own_pid = std::process::id();
    super::super::openwin::exists(hwnd)
        && super::super::openwin::enumerate()
            .iter()
            .any(|window| window.hwnd == hwnd && window.pid != own_pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a check: how long reading the windows takes on this machine.
    /// `cargo test reading_the_windows_is_cheap -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the windows of the machine"]
    fn reading_the_windows_is_cheap() {
        let programs = Programs {
            pinned: &[],
            catalog: &[],
        };
        let mut times = Vec::new();
        let mut count = 0;
        for _ in 0..20 {
            let start = std::time::Instant::now();
            count = read(&programs).len();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let first = times[0];
        times.sort_by(f64::total_cmp);
        println!(
            "{count} windows: first {first:.2} ms, median {:.2} ms, worst {:.2} ms",
            times[times.len() / 2],
            times[times.len() - 1]
        );
    }
}
