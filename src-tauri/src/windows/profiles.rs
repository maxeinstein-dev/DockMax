//! The profile of a Chrome window (spec 0009): its name, and its photo shrunk
//! to a badge for the dock's icon, as the Windows taskbar badges each Chrome
//! button. Reads two things of the user's Chrome, nothing else and never
//! writing: the profile list in `Local State` and the account photo file of a
//! profile folder. Neither the names nor the photo go to a log.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use crate::domain::browser_profile::{ChromeProfile, parse_local_state, profile_for_aumid};
use crate::domain::icon_pack::{MAX_IMAGE_SIDE, png_size};
use crate::domain::thumbnail::shrink;

/// The most of `Local State` that is read.
const MAX_LOCAL_STATE_BYTES: u64 = 8 * 1024 * 1024;

/// The most of a photo that is read.
const MAX_PHOTO_BYTES: u64 = 4 * 1024 * 1024;

/// The side of the badge, in pixels: the dock draws it at a few tens.
const BADGE_SIDE: u32 = 64;

/// What was read of `Local State`, and when the file had last changed.
struct Known {
    modified: Option<SystemTime>,
    profiles: Vec<ChromeProfile>,
}

static PROFILES: Mutex<Option<Known>> = Mutex::new(None);

/// The badges made, by AppID: kept until `Local State` changes.
static BADGES: Mutex<Option<HashMap<String, Option<Vec<u8>>>>> = Mutex::new(None);

fn user_data_dir() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local)
            .join("Google")
            .join("Chrome")
            .join("User Data"),
    )
}

fn read_bounded(path: &std::path::Path, most: u64) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(most + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= most).then_some(bytes)
}

/// The profiles of Chrome; read again only when `Local State` changed.
fn profiles() -> Vec<ChromeProfile> {
    let Some(path) = user_data_dir().map(|dir| dir.join("Local State")) else {
        return Vec::new();
    };
    let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok();
    let mut known = PROFILES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(known) = known.as_ref()
        && known.modified == modified
    {
        return known.profiles.clone();
    }
    let profiles = read_bounded(&path, MAX_LOCAL_STATE_BYTES)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| parse_local_state(&text))
        .unwrap_or_default();
    *known = Some(Known {
        modified,
        profiles: profiles.clone(),
    });
    BADGES.lock().unwrap_or_else(PoisonError::into_inner).take();
    profiles
}

/// The profile of a window with the AppID `aumid`, if it is a Chrome one.
pub fn profile_of(aumid: &str) -> Option<ChromeProfile> {
    profile_for_aumid(aumid, &profiles()).cloned()
}

/// Whether the profile has a photo file to badge with.
pub fn has_badge(aumid: &str) -> bool {
    badge_source(aumid).is_some()
}

fn badge_source(aumid: &str) -> Option<PathBuf> {
    let profile = profile_of(aumid)?;
    let path = user_data_dir()?.join(&profile.dir).join(profile.photo?);
    path.is_file().then_some(path)
}

fn make_badge(aumid: &str) -> Option<Vec<u8>> {
    let path = badge_source(aumid)?;
    let bytes = read_bounded(&path, MAX_PHOTO_BYTES)?;
    // A PNG, of a size worth decoding, before anything is decoded.
    let (width, height) = png_size(&bytes)?;
    if width == 0 || height == 0 || width > 4 * MAX_IMAGE_SIDE || height > 4 * MAX_IMAGE_SIDE {
        return None;
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let rgba = to_rgba(frame.color_type, &buffer[..frame.buffer_size()])?;
    let (width, height, pixels) = shrink(frame.width, frame.height, &rgba, BADGE_SIDE)?;
    super::apps::encode_png(width, height, &pixels)
}

/// The pixels of a decoded frame as RGBA, 8 bits.
fn to_rgba(color: png::ColorType, data: &[u8]) -> Option<Vec<u8>> {
    use png::ColorType::{Grayscale, GrayscaleAlpha, Indexed, Rgb, Rgba};
    Some(match color {
        Rgba => data.to_vec(),
        Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        Indexed => return None,
    })
}

/// The badge of the profile of a window with the AppID `aumid`, as a small
/// PNG; `None` when it is not a Chrome profile with a photo. Reads and decodes
/// a file: off the event loop.
pub fn badge_png(aumid: &str) -> Option<Vec<u8>> {
    // Looking the profile up also drops the badges if `Local State` changed.
    let _ = profiles();
    if let Some(known) = BADGES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|badges| badges.get(aumid))
    {
        return known.clone();
    }
    let made = make_badge(aumid);
    BADGES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_with(HashMap::new)
        .insert(aumid.to_owned(), made.clone());
    made
}
