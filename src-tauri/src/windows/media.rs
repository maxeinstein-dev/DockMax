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
use windows::Storage::Streams::{DataReader, IInputStream};
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
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_ART {
        return None;
    }
    let kind = stream
        .ContentType()
        .map(|kind| kind.to_string())
        .unwrap_or_default();
    let reader = DataReader::CreateDataReader(&stream.cast::<IInputStream>().ok()?).ok()?;
    let loaded = reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0u8; loaded as usize];
    reader.ReadBytes(&mut bytes).ok()?;
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
    };
    if snapshot.title.is_empty() && snapshot.artist.is_empty() {
        return None;
    }
    snapshot.art = match previous {
        Some(previous) if previous.track() == snapshot.track() && previous.art.is_some() => {
            previous.art.clone()
        }
        _ => art_of(&properties).map(Arc::new),
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
