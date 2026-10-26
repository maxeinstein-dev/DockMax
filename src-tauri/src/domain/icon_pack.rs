//! Icon packs (spec 0007): the `pack.json` a pack comes with, what is valid in
//! it, and how a program finds its icon in one. Pure rules: the folder is read
//! in `persistence/icon_packs.rs`.
//!
//! A pack comes from a folder the user fills, maybe from a download, so
//! everything here treats it as untrusted input: sizes are bounded and paths
//! are checked as text before any file is opened.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The largest `pack.json` read, in bytes.
pub const MAX_MANIFEST_BYTES: u64 = 256 * 1024;

/// The most programs a pack names.
pub const MAX_ENTRIES: usize = 2000;

/// The longest name of a program (a key of `icons`), in characters.
pub const MAX_KEY_CHARS: usize = 100;

/// The longest name, author and version of a pack, in characters.
pub const MAX_NAME_CHARS: usize = 60;

/// The longest description of a pack, in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 300;

/// The largest image read, in bytes.
pub const MAX_IMAGE_BYTES: u64 = 512 * 1024;

/// An image is at least this many pixels on each side, and at most
/// [`MAX_IMAGE_SIDE`].
pub const MIN_IMAGE_SIDE: u32 = 16;
pub const MAX_IMAGE_SIDE: u32 = 1024;

/// Whether `id` can be the id of a pack (the name of its folder): lowercase
/// letters, digits and `-`, from 1 to 48 characters.
pub fn is_valid_pack_id(id: &str) -> bool {
    (1..=48).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// What a program is looked up by: lowercase, with only the letters and
/// digits (so `Bloco de notas`, `bloco-de-notas` and `BLOCO_DE_NOTAS` are one
/// name), without accents on the letters Portuguese uses, and without a `.exe`
/// or `.lnk` at the end. Empty when nothing is left.
pub fn normalize_key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let bare = ["exe", "lnk"]
        .iter()
        .find_map(|extension| {
            lower
                .strip_suffix(extension)
                .and_then(|rest| rest.strip_suffix('.'))
        })
        .unwrap_or(&lower);
    bare.chars()
        .filter_map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => Some('a'),
            'é' | 'è' | 'ê' | 'ë' => Some('e'),
            'í' | 'ì' | 'î' | 'ï' => Some('i'),
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => Some('o'),
            'ú' | 'ù' | 'û' | 'ü' => Some('u'),
            'ç' => Some('c'),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// Whether `path` can name an image inside a pack: relative, going only
/// downwards, with no drive and no NTFS stream (`:`), ending in `.png`.
pub fn is_valid_image_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    !path.is_empty()
        && path.len() <= 260
        && !path.contains(':')
        && !path.chars().any(char::is_control)
        && parts
            .iter()
            .all(|part| !part.is_empty() && *part != "." && *part != ".." && part.trim() == *part)
        && path.to_ascii_lowercase().ends_with(".png")
}

/// `#RRGGBB`.
fn is_hex_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The size, in pixels, an image declares in its PNG header; `None` when it
/// is not a PNG. Reads nothing past the header.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

/// Whether the declared size of an image is one the dock takes.
pub fn is_valid_image_size((width, height): (u32, u32)) -> bool {
    (MIN_IMAGE_SIDE..=MAX_IMAGE_SIDE).contains(&width)
        && (MIN_IMAGE_SIDE..=MAX_IMAGE_SIDE).contains(&height)
}

/// The `pack.json` of a pack, as written. Fields it does not know are left
/// out; the optional ones may be missing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub accent: Option<String>,
    /// The name of a program → the image, relative to the pack's folder.
    pub icons: BTreeMap<String, String>,
    /// The image of a program no name covers.
    pub fallback: Option<String>,
}

/// Why a pack is refused. The text is shown to the user and carries no path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PackError {
    #[error("pack.json is missing")]
    ManifestMissing,
    #[error("pack.json is too big")]
    ManifestTooBig,
    #[error("pack.json is not valid")]
    ManifestInvalid,
    #[error("the pack has no name")]
    NoName,
    #[error("a text of the pack is too long")]
    TextTooLong,
    #[error("the accent is not a #RRGGBB color")]
    BadAccent,
    #[error("the pack names too many programs")]
    TooManyEntries,
    #[error("a name of a program is empty or too long")]
    BadKey,
    #[error("an image path is not allowed")]
    BadPath,
}

impl Manifest {
    /// Checks what serde can't: the sizes and the paths.
    pub fn validate(&self) -> Result<(), PackError> {
        if self.name.trim().is_empty() {
            return Err(PackError::NoName);
        }
        let short = |text: &str, most: usize| text.chars().count() <= most;
        if !short(&self.name, MAX_NAME_CHARS)
            || !short(&self.author, MAX_NAME_CHARS)
            || !short(&self.version, MAX_NAME_CHARS)
            || !short(&self.description, MAX_DESCRIPTION_CHARS)
            || [&self.name, &self.author, &self.version, &self.description]
                .iter()
                .any(|text| text.chars().any(char::is_control))
        {
            return Err(PackError::TextTooLong);
        }
        if self
            .accent
            .as_deref()
            .is_some_and(|accent| !is_hex_color(accent))
        {
            return Err(PackError::BadAccent);
        }
        if self.icons.len() > MAX_ENTRIES {
            return Err(PackError::TooManyEntries);
        }
        if self
            .icons
            .keys()
            .any(|key| normalize_key(key).is_empty() || key.chars().count() > MAX_KEY_CHARS)
        {
            return Err(PackError::BadKey);
        }
        if self
            .icons
            .values()
            .chain(self.fallback.iter())
            .any(|path| !is_valid_image_path(path))
        {
            return Err(PackError::BadPath);
        }
        Ok(())
    }

    /// Parses and validates the text of a `pack.json`.
    pub fn parse(text: &str) -> Result<Manifest, PackError> {
        let manifest: Manifest =
            serde_json::from_str(text).map_err(|_| PackError::ManifestInvalid)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// The image for a program known by any of `names` (its shown name, the
    /// name of its file…), in that order, else the pack's own fallback. The
    /// path is the one written in the manifest.
    pub fn image_for<'a>(&'a self, names: &[&str]) -> Option<&'a str> {
        let keyed: BTreeMap<String, &str> = self
            .icons
            .iter()
            .map(|(key, path)| (normalize_key(key), path.as_str()))
            .collect();
        names
            .iter()
            .map(|name| normalize_key(name))
            .filter(|key| !key.is_empty())
            .find_map(|key| keyed.get(&key).copied())
            .or(self.fallback.as_deref())
    }

    /// Whether some name of the program has its own image in the pack (the
    /// fallback does not count): the "covers" of a pack.
    pub fn covers(&self, names: &[&str]) -> bool {
        names
            .iter()
            .map(|name| normalize_key(name))
            .filter(|key| !key.is_empty())
            .any(|key| self.icons.keys().any(|own| normalize_key(own) == key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> Result<Manifest, PackError> {
        Manifest::parse(json)
    }

    #[test]
    fn pack_ids_are_lowercase_names() {
        assert!(is_valid_pack_id("pixel-system"));
        assert!(is_valid_pack_id("a1"));
        assert!(!is_valid_pack_id(""));
        assert!(!is_valid_pack_id("Pixel"));
        assert!(!is_valid_pack_id("a b"));
        assert!(!is_valid_pack_id("a_b"));
        assert!(!is_valid_pack_id("../x"));
        assert!(!is_valid_pack_id(&"a".repeat(49)));
        assert!(is_valid_pack_id(&"a".repeat(48)));
    }

    #[test]
    fn keys_ignore_case_accents_spaces_punctuation_and_the_extension() {
        assert_eq!(normalize_key("Bloco de notas"), "blocodenotas");
        assert_eq!(normalize_key("bloco-de-notas"), "blocodenotas");
        assert_eq!(normalize_key("BLOCO_DE_NOTAS"), "blocodenotas");
        assert_eq!(normalize_key("Relógio"), "relogio");
        assert_eq!(normalize_key("Configurações"), "configuracoes");
        assert_eq!(normalize_key("Whoami.exe"), "whoami");
        assert_eq!(normalize_key("Atalho de teste.LNK"), "atalhodeteste");
        assert_eq!(normalize_key("  "), "");
        assert_eq!(normalize_key("..."), "");
        // The extension goes only at the end, and only the two.
        assert_eq!(normalize_key("exe"), "exe");
        assert_eq!(normalize_key("a.txt"), "atxt");
    }

    #[test]
    fn image_paths_go_only_downwards_inside_the_pack() {
        for good in ["a.png", "apps/a.png", "apps\\a.png", "A.PNG", "x/y/z.png"] {
            assert!(is_valid_image_path(good), "{good}");
        }
        for bad in [
            "",
            "a.jpg",
            "a",
            "../a.png",
            "apps/../a.png",
            "/a.png",
            "\\a.png",
            "C:\\a.png",
            "c:a.png",
            "a.png:stream.png",
            "a//b.png",
            "./a.png",
            " a.png",
            "apps/ .png",
            "a\u{0}.png",
        ] {
            assert!(!is_valid_image_path(bad), "{bad:?}");
        }
        assert!(!is_valid_image_path(&format!("{}.png", "a".repeat(260))));
    }

    #[test]
    fn a_png_declares_its_size_in_the_header() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&96u32.to_be_bytes());
        png.extend_from_slice(&64u32.to_be_bytes());
        assert_eq!(png_size(&png), Some((96, 64)));

        assert_eq!(png_size(b"GIF89a not a png at all, but long enough"), None);
        assert_eq!(png_size(&png[..20]), None);
        let mut other = png.clone();
        other[12..16].copy_from_slice(b"IDAT");
        assert_eq!(png_size(&other), None);
    }

    #[test]
    fn image_sizes_are_bounded() {
        assert!(is_valid_image_size((16, 16)));
        assert!(is_valid_image_size((1024, 512)));
        assert!(!is_valid_image_size((15, 64)));
        assert!(!is_valid_image_size((64, 1025)));
        assert!(!is_valid_image_size((0, 0)));
    }

    #[test]
    fn a_manifest_needs_a_name_and_may_leave_the_rest_out() {
        let found = manifest(r#"{"name":"Cores"}"#).unwrap();
        assert_eq!(found.name, "Cores");
        assert!(found.icons.is_empty());
        assert_eq!(found.fallback, None);
        assert_eq!(manifest(r#"{}"#), Err(PackError::NoName));
        assert_eq!(manifest(r#"{"name":"  "}"#), Err(PackError::NoName));
        assert_eq!(manifest("not json"), Err(PackError::ManifestInvalid));
        assert_eq!(manifest(r#"[1]"#), Err(PackError::ManifestInvalid));
        // What the pack says that this app doesn't know is left out.
        assert!(manifest(r#"{"name":"A","padding":0.1,"widget":{}}"#).is_ok());
    }

    #[test]
    fn a_manifest_refuses_what_is_too_long_or_unsafe() {
        let long = "x".repeat(MAX_NAME_CHARS + 1);
        assert_eq!(
            manifest(&format!(r#"{{"name":"{long}"}}"#)),
            Err(PackError::TextTooLong)
        );
        assert_eq!(
            manifest(r#"{"name":"A","author":"a\nb"}"#),
            Err(PackError::TextTooLong)
        );
        assert_eq!(
            manifest(r#"{"name":"A","accent":"blue"}"#),
            Err(PackError::BadAccent)
        );
        assert!(manifest(r##"{"name":"A","accent":"#3A63D6"}"##).is_ok());
        assert_eq!(
            manifest(r#"{"name":"A","icons":{"...":"a.png"}}"#),
            Err(PackError::BadKey)
        );
        assert_eq!(
            manifest(r#"{"name":"A","icons":{"x":"../a.png"}}"#),
            Err(PackError::BadPath)
        );
        assert_eq!(
            manifest(r#"{"name":"A","fallback":"C:\\a.png"}"#),
            Err(PackError::BadPath)
        );
        let too_long_key = "k".repeat(MAX_KEY_CHARS + 1);
        assert_eq!(
            manifest(&format!(
                r#"{{"name":"A","icons":{{"{too_long_key}":"a.png"}}}}"#
            )),
            Err(PackError::BadKey)
        );
    }

    #[test]
    fn a_manifest_names_a_bounded_number_of_programs() {
        let many: Vec<String> = (0..=MAX_ENTRIES)
            .map(|n| format!(r#""app{n}":"a.png""#))
            .collect();
        let json = format!(r#"{{"name":"A","icons":{{{}}}}}"#, many.join(","));
        assert_eq!(manifest(&json), Err(PackError::TooManyEntries));
    }

    fn colors() -> Manifest {
        manifest(
            r#"{"name":"Cores","icons":{"Calculadora":"calc.png","Bloco de notas":"notes.png"},"fallback":"other.png"}"#,
        )
        .unwrap()
    }

    #[test]
    fn a_program_is_found_by_the_first_name_the_pack_knows() {
        let pack = colors();
        assert_eq!(pack.image_for(&["Calculadora"]), Some("calc.png"));
        assert_eq!(pack.image_for(&["calculadora.exe"]), Some("calc.png"));
        assert_eq!(
            pack.image_for(&["Bloco-de-Notas", "Calculadora"]),
            Some("notes.png")
        );
        // The shown name first, then the file's.
        assert_eq!(
            pack.image_for(&["Desconhecido", "calculadora"]),
            Some("calc.png")
        );
    }

    #[test]
    fn a_program_the_pack_does_not_name_gets_the_fallback_if_there_is_one() {
        let pack = colors();
        assert_eq!(pack.image_for(&["Desconhecido"]), Some("other.png"));
        assert_eq!(pack.image_for(&[]), Some("other.png"));
        let plain = manifest(r#"{"name":"A","icons":{"x":"x.png"}}"#).unwrap();
        assert_eq!(plain.image_for(&["y"]), None);
    }

    #[test]
    fn covering_counts_only_the_names_of_the_pack() {
        let pack = colors();
        assert!(pack.covers(&["Calculadora"]));
        assert!(pack.covers(&["?", "bloco de notas"]));
        assert!(!pack.covers(&["Desconhecido"]));
        assert!(!pack.covers(&["..."]));
    }
}
