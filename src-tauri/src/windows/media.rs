//! The Tocando agora (spec 0017 F4): the media session Windows shows in its
//! own controls (`GlobalSystemMediaTransportControlsSessionManager`), read
//! every second while a dock has the widget, and stopped when none has. Only
//! what plays now stays, in memory: title, artist, the program, and the
//! cover once per track.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::{DataReader, IInputStream, InputStreamOptions};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::Interface;

/// What plays now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub title: String,
    pub artist: String,
    /// The AppUserModelID of the program that plays it.
    pub app: String,
    pub playing: bool,
    pub can_toggle: bool,
    pub can_previous: bool,
    pub can_next: bool,
    /// The cover, as the program gave it, with its type (`image/png`).
    pub art: Option<Arc<(String, Vec<u8>)>>,
    /// The cover's color, its red, green and blue: the tile's ground, as a
    /// phone's media notification has it.
    pub tint: Option<(u8, u8, u8)>,
    /// The program's icon, a PNG: a small mark on the cover.
    pub icon: Option<Arc<Vec<u8>>>,
}

impl Snapshot {
    /// What tells one track from another: the cover is read again for a
    /// new one.
    pub fn track(&self) -> (String, String, String) {
        (self.app.clone(), self.title.clone(), self.artist.clone())
    }
}

static STATE: Mutex<Option<Snapshot>> = Mutex::new(None);
/// The docks (by window) that have a Tocando agora.
static WANTERS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
static WANTED: AtomicBool = AtomicBool::new(false);
static STARTED: Once = Once::new();

const EVERY: Duration = Duration::from_secs(1);
/// The largest cover kept: 2 MB.
const MAX_ART: u64 = 2 * 1024 * 1024;

/// What plays now, as last read; `None` when nothing does.
pub fn snapshot() -> Option<Snapshot> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Whether the dock `dock` has a Tocando agora: the reading runs while any
/// has one.
pub fn want(dock: isize, wanted: bool) {
    let on = {
        let mut wanters = WANTERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        wanters.retain(|each| *each != dock);
        if wanted {
            wanters.push(dock);
        }
        !wanters.is_empty()
    };
    WANTED.store(on, Ordering::Relaxed);
    if on {
        STARTED.call_once(|| {
            let _ = std::thread::Builder::new().name("media".into()).spawn(run);
        });
    }
}

/// The color of `image` (PNG, JPEG...): the mean of its pixels, read at
/// 16 × 16.
fn tint_of(image: &[u8]) -> Option<(u8, u8, u8)> {
    use windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, IWICImagingFactory,
        WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
        WICDecodeMetadataCacheOnLoad,
    };
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
    const SIDE: u32 = 16;
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(image).ok()?;
        let decoder = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
            .ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let scaler = wic.CreateBitmapScaler().ok()?;
        scaler
            .Initialize(&frame, SIDE, SIDE, WICBitmapInterpolationModeFant)
            .ok()?;
        let converter = wic.CreateFormatConverter().ok()?;
        converter
            .Initialize(
                &scaler,
                &GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .ok()?;
        let mut pixels = vec![0u8; (SIDE * SIDE * 4) as usize];
        converter
            .CopyPixels(std::ptr::null(), SIDE * 4, &mut pixels)
            .ok()?;
        let count = SIDE * SIDE;
        let sum = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .fold((0u32, 0u32, 0u32), |(r, g, b), px| {
                (
                    r + u32::from(px[2]),
                    g + u32::from(px[1]),
                    b + u32::from(px[0]),
                )
            });
        Some((
            (sum.0 / count) as u8,
            (sum.1 / count) as u8,
            (sum.2 / count) as u8,
        ))
    }
}

/// The icon of the program `app` that plays: its item of the Apps folder,
/// or, for a bare `Spotify.exe`, the file of the open window that runs it.
fn icon_of(app: &str) -> Option<Vec<u8>> {
    if let Some(png) = super::apps::icon_png(app, 64) {
        return Some(png);
    }
    let name = app.rsplit(['\\', '/']).next()?.to_lowercase();
    let file = super::openwin::enumerate().into_iter().find_map(|window| {
        let exe = window.exe?;
        let same = exe
            .file_name()
            .is_some_and(|file| file.to_string_lossy().to_lowercase() == name);
        same.then_some(exe)
    })?;
    super::apps::icon_png(&file.to_string_lossy(), 64)
}

fn manager() -> windows::core::Result<Manager> {
    Manager::RequestAsync()?.join()
}

/// The cover of `properties`: its type and bytes.
fn art_of(properties: &Properties) -> Option<(String, Vec<u8>)> {
    let stream = properties
        .Thumbnail()
        .ok()?
        .OpenReadAsync()
        .ok()?
        .join()
        .ok()?;
    let kind = stream
        .ContentType()
        .map(|kind| kind.to_string())
        .unwrap_or_default();
    // The size a program gives may be 0 while the cover is still on its
    // way: read in parts until the stream ends, whatever it says.
    let reader = DataReader::CreateDataReader(&stream.cast::<IInputStream>().ok()?).ok()?;
    reader
        .SetInputStreamOptions(InputStreamOptions::Partial)
        .ok()?;
    let mut bytes = Vec::new();
    loop {
        let loaded = reader.LoadAsync(64 * 1024).ok()?.join().ok()?;
        if loaded == 0 {
            break;
        }
        let start = bytes.len();
        bytes.resize(start + loaded as usize, 0);
        reader.ReadBytes(&mut bytes[start..]).ok()?;
        if bytes.len() as u64 > MAX_ART {
            return None;
        }
    }
    if bytes.is_empty() {
        return None;
    }
    let kind = if kind.starts_with("image/") {
        kind
    } else {
        "image/png".into()
    };
    Some((kind, bytes))
}

/// What plays now, read from Windows; `previous` gives the cover again when
/// the track is the same.
fn read(manager: &Manager, previous: Option<&Snapshot>) -> Option<Snapshot> {
    let session = manager.GetCurrentSession().ok()?;
    let properties = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let info = session.GetPlaybackInfo().ok()?;
    let controls = info.Controls().ok();
    let flag =
        |read: Option<windows::core::Result<bool>>| read.and_then(Result::ok).unwrap_or(false);
    let mut snapshot = Snapshot {
        title: properties
            .Title()
            .map(|t| t.to_string())
            .unwrap_or_default(),
        artist: properties
            .Artist()
            .map(|t| t.to_string())
            .unwrap_or_default(),
        app: session
            .SourceAppUserModelId()
            .map(|t| t.to_string())
            .unwrap_or_default(),
        playing: info.PlaybackStatus().ok() == Some(Status::Playing),
        can_toggle: flag(controls.as_ref().map(|c| c.IsPlayPauseToggleEnabled())),
        can_previous: flag(controls.as_ref().map(|c| c.IsPreviousEnabled())),
        can_next: flag(controls.as_ref().map(|c| c.IsNextEnabled())),
        art: None,
        tint: None,
        icon: None,
    };
    if snapshot.title.is_empty() && snapshot.artist.is_empty() {
        return None;
    }
    match previous {
        Some(previous) if previous.track() == snapshot.track() && previous.art.is_some() => {
            snapshot.art = previous.art.clone();
            snapshot.tint = previous.tint;
        }
        _ => {
            snapshot.art = art_of(&properties).map(Arc::new);
            snapshot.tint = snapshot.art.as_ref().and_then(|art| tint_of(&art.1));
        }
    }
    snapshot.icon = match previous {
        Some(previous) if previous.app == snapshot.app => previous.icon.clone(),
        _ => icon_of(&snapshot.app).map(Arc::new),
    };
    Some(snapshot)
}

fn run() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let mut found: Option<Manager> = None;
    loop {
        std::thread::sleep(EVERY);
        if !WANTED.load(Ordering::Relaxed) {
            continue;
        }
        if found.is_none() {
            found = manager()
                .inspect_err(|error| log::warn!("media: no manager: {error}"))
                .ok();
        }
        let Some(manager) = found.as_ref() else {
            continue;
        };
        let previous = snapshot();
        let next = read(manager, previous.as_ref());
        // The cover, compared by pointer: the same track keeps its Arc.
        let same = match (&previous, &next) {
            (Some(a), Some(b)) => {
                a.track() == b.track()
                    && a.playing == b.playing
                    && a.can_toggle == b.can_toggle
                    && a.can_previous == b.can_previous
                    && a.can_next == b.can_next
                    && a.art.as_ref().map(Arc::as_ptr) == b.art.as_ref().map(Arc::as_ptr)
                    && a.icon.as_ref().map(Arc::as_ptr) == b.icon.as_ref().map(Arc::as_ptr)
            }
            (None, None) => true,
            _ => false,
        };
        if !same {
            *STATE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = next;
            super::native_dock::media_changed();
        }
    }
}

/// Whether the program `app` of the media session (an AppUserModelID, or a
/// bare `Spotify.exe`) runs the window with the AppID `aumid` and the file
/// `exe`.
fn same_program(app: &str, aumid: Option<&str>, exe: Option<&std::path::Path>) -> bool {
    let app = app.to_lowercase();
    if app.is_empty() {
        return false;
    }
    let by_id = aumid
        .map(str::to_lowercase)
        .is_some_and(|aumid| aumid == app || aumid.starts_with(&format!("{app}.")));
    let by_file = exe
        .and_then(std::path::Path::file_name)
        .map(|file| file.to_string_lossy().to_lowercase())
        .is_some_and(|file| file == app || app.ends_with(&format!("\\{file}")));
    by_id || by_file
}

/// Whether what plays now comes from one of the windows `windows`: whether
/// it plays, when it does. Read from Windows on the calling thread, which
/// must not be a window's (it waits).
pub fn session_in(windows: &[isize]) -> Option<bool> {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let session = manager().ok()?.GetCurrentSession().ok()?;
    let app = session.SourceAppUserModelId().ok()?.to_string();
    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .is_ok_and(|status| status == Status::Playing);
    let ours = super::openwin::enumerate().into_iter().any(|window| {
        windows.contains(&window.hwnd)
            && same_program(&app, window.aumid.as_deref(), window.exe.as_deref())
    });
    ours.then_some(playing)
}

/// What the panel or the dock asks of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Toggle,
    Previous,
    Next,
}

impl Action {
    pub fn parse(text: &str) -> Option<Action> {
        match text {
            "toggle" => Some(Action::Toggle),
            "previous" => Some(Action::Previous),
            "next" => Some(Action::Next),
            _ => None,
        }
    }
}

/// `action` on what plays now, on a thread of its own.
pub fn control(action: Action) {
    let spawned = std::thread::Builder::new()
        .name("media-control".into())
        .spawn(move || {
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let done = (|| -> windows::core::Result<bool> {
                let session = manager()?.GetCurrentSession()?;
                match action {
                    Action::Toggle => session.TryTogglePlayPauseAsync()?.join(),
                    Action::Previous => session.TrySkipPreviousAsync()?.join(),
                    Action::Next => session.TrySkipNextAsync()?.join(),
                }
            })();
            if let Err(error) = done {
                log::warn!("media: {action:?} failed: {error}");
            }
        });
    if let Err(error) = spawned {
        log::warn!("media: no thread: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::same_program;

    #[test]
    fn the_session_is_matched_to_its_program() {
        let spotify = Path::new(r"C:\Users\x\AppData\Roaming\Spotify\Spotify.exe");
        assert!(same_program("Spotify.exe", None, Some(spotify)));
        assert!(same_program(
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify",
            Some("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"),
            None
        ));
        // Chrome's session and a window of one of its profiles.
        assert!(same_program("Chrome", Some("Chrome.ABCDEF"), None));
        assert!(!same_program("Spotify.exe", Some("Chrome"), None));
        assert!(!same_program("", Some(""), None));
    }
}
