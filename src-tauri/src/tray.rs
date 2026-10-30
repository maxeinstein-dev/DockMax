use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::AppHandle;
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

/// The product name: the same in every locale.
const TOOLTIP: &str = "Dockyard";

/// The tray icon's id, to find it again when the taskbar's theme changes.
const ID: &str = "dockyard";

/// The glyph's ink: the D + Salto alone, with no square, as the system's own
/// tray icons (network, volume) are (docs/marca/0001-proposta.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// For the light taskbar.
    Black,
    /// For the dark taskbar.
    White,
}

/// The sizes drawn, one per scale: 100%, 125%, 150% and 200%.
const SIZES: [u32; 4] = [16, 20, 24, 32];

/// The ink for the taskbar, from `SystemUsesLightTheme` (the taskbar's, not
/// the apps' `AppsUseLightTheme`): 1 is a light taskbar; 0 or no value, a
/// dark one, as Windows 11 comes.
pub fn ink_for(system_uses_light_theme: Option<u32>) -> Ink {
    match system_uses_light_theme {
        Some(value) if value != 0 => Ink::Black,
        _ => Ink::White,
    }
}

/// The size for the primary monitor's DPI: the smallest drawn that is at
/// least the icon's size at that scale (16 px at 96 DPI), so Windows only
/// ever shrinks it; 32 above 200%.
pub fn size_for(dpi: u32) -> u32 {
    let wanted = (16 * dpi.max(1)).div_ceil(96);
    SIZES
        .into_iter()
        .find(|&size| size >= wanted)
        .unwrap_or(SIZES[SIZES.len() - 1])
}

/// The PNG of `ink` at `size`, compiled into the binary.
fn png(ink: Ink, size: u32) -> &'static [u8] {
    match (ink, size) {
        (Ink::Black, 16) => include_bytes!("../icons/tray/preto-16.png"),
        (Ink::Black, 20) => include_bytes!("../icons/tray/preto-20.png"),
        (Ink::Black, 24) => include_bytes!("../icons/tray/preto-24.png"),
        (Ink::Black, _) => include_bytes!("../icons/tray/preto-32.png"),
        (Ink::White, 16) => include_bytes!("../icons/tray/branco-16.png"),
        (Ink::White, 20) => include_bytes!("../icons/tray/branco-20.png"),
        (Ink::White, 24) => include_bytes!("../icons/tray/branco-24.png"),
        (Ink::White, _) => include_bytes!("../icons/tray/branco-32.png"),
    }
}

/// The PNG as the RGBA image the tray takes; `None` if it can't be read.
fn decode(bytes: &[u8]) -> Option<Image<'static>> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buffer).ok()?;
    if frame.color_type != png::ColorType::Rgba || frame.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buffer.truncate(frame.buffer_size());
    Some(Image::new_owned(buffer, frame.width, frame.height))
}

/// `SystemUsesLightTheme` as the registry has it now.
#[cfg(windows)]
fn system_uses_light_theme() -> Option<u32> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(std::ptr::from_mut(&mut value).cast()),
            Some(&mut size),
        )
    };
    read.is_ok().then_some(value)
}

#[cfg(not(windows))]
fn system_uses_light_theme() -> Option<u32> {
    None
}

/// The primary monitor's DPI; 96 when it can't be told.
#[cfg(windows)]
fn primary_dpi() -> u32 {
    crate::windows::monitors::all()
        .into_iter()
        .find(|monitor| monitor.primary)
        .map_or(96, |monitor| (monitor.scale * 96.0).round() as u32)
}

#[cfg(not(windows))]
fn primary_dpi() -> u32 {
    96
}

/// The ink and size the icon has now, so a change of an unrelated setting
/// doesn't set the same icon again.
static SHOWN: Mutex<Option<(Ink, u32)>> = Mutex::new(None);

fn shown() -> MutexGuard<'static, Option<(Ink, u32)>> {
    SHOWN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The icon for the taskbar's theme and the primary monitor's scale now.
fn current() -> (Ink, u32) {
    (ink_for(system_uses_light_theme()), size_for(primary_dpi()))
}

/// Adds the Dockyard icon to the notification area. Its menu, the Dockyard's
/// own (spec 0014 R3), opens on right-click with what the dock's own menu
/// has.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id(ID)
        .tooltip(TOOLTIP)
        .on_tray_icon_event(|icon, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // A late check, in case a theme change went unseen.
                refresh(icon.app_handle());
                #[cfg(windows)]
                crate::windows::native_dock::tray_menu(icon.app_handle());
            }
        });
    let (ink, size) = current();
    if let Some(image) = decode(png(ink, size)) {
        tray = tray.icon(image);
        *shown() = Some((ink, size));
    } else if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Sets the icon again if the taskbar's theme or the primary monitor's scale
/// changed. Cheap when nothing did: a registry read and a monitor list.
pub fn refresh(app: &AppHandle) {
    let now = current();
    let mut shown = shown();
    if *shown == Some(now) {
        return;
    }
    let Some(tray) = app.tray_by_id(ID) else {
        return;
    };
    let Some(image) = decode(png(now.0, now.1)) else {
        return;
    };
    match tray.set_icon(Some(image)) {
        Ok(()) => *shown = Some(now),
        Err(error) => log::warn!("tray: failed to change the icon: {error}"),
    }
}

/// `refresh` on the main thread, for a window procedure on another thread:
/// posted, never waited for.
pub fn refresh_soon(app: &AppHandle) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || refresh(&handle)) {
        log::warn!("tray: failed to queue the icon check: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_taskbar_theme_picks_the_ink() {
        assert_eq!(ink_for(Some(1)), Ink::Black);
        assert_eq!(ink_for(Some(0)), Ink::White);
        assert_eq!(ink_for(None), Ink::White);
    }

    #[test]
    fn the_primary_scale_picks_the_size() {
        assert_eq!(size_for(96), 16);
        assert_eq!(size_for(120), 20);
        assert_eq!(size_for(144), 24);
        assert_eq!(size_for(192), 32);
        // In between: the next size up, which Windows only shrinks.
        assert_eq!(size_for(108), 20);
        assert_eq!(size_for(168), 32);
        // Above 200% and nonsense values stay in range.
        assert_eq!(size_for(288), 32);
        assert_eq!(size_for(0), 16);
    }

    #[test]
    fn every_embedded_icon_decodes_at_its_size() {
        for ink in [Ink::Black, Ink::White] {
            for size in SIZES {
                let image = decode(png(ink, size)).expect("a readable PNG");
                assert_eq!((image.width(), image.height()), (size, size));
            }
        }
    }
}
