//! The icon packs on disk (spec 0007): the folders under `IconPacks`, their
//! `pack.json` and their images. The folder is the user's and may hold what a
//! download put there, so nothing is opened without the checks of §3.2: sizes,
//! the real path staying inside the pack, and the PNG header.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::domain::icon_pack::{
    MAX_IMAGE_BYTES, MAX_MANIFEST_BYTES, Manifest, PackError, is_valid_image_path,
    is_valid_image_size, is_valid_pack_id, png_size,
};

/// The folder the packs live in, under the app's data folder.
pub const FOLDER: &str = "IconPacks";

/// The most pack folders looked at; the rest are left out.
const MAX_PACKS: usize = 100;

/// A pack that can be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub id: String,
    pub dir: PathBuf,
    pub manifest: Manifest,
}

/// A folder of the packs folder: a pack, or the reason it is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    pub pack: Result<Pack, PackError>,
}

/// The packs folder inside `data_dir`.
pub fn root(data_dir: &Path) -> PathBuf {
    data_dir.join(FOLDER)
}

/// Reads at most `most` bytes of `path`; `None` if it is bigger or can't be
/// read.
fn read_bounded(path: &Path, most: u64) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(most + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= most).then_some(bytes)
}

/// The real path of `relative` inside `dir` (itself a real path), or `None`
/// if it does not exist or leads outside it (a link or a junction).
fn inside(dir: &Path, relative: &str) -> Option<PathBuf> {
    let real = dir.join(relative).canonicalize().ok()?;
    real.starts_with(dir).then_some(real)
}

fn load(id: &str, dir: &Path) -> Result<Pack, PackError> {
    let dir = dir.canonicalize().map_err(|_| PackError::ManifestMissing)?;
    let path = inside(&dir, "pack.json").ok_or(PackError::ManifestMissing)?;
    let too_big = fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_MANIFEST_BYTES);
    if too_big {
        return Err(PackError::ManifestTooBig);
    }
    let bytes = read_bounded(&path, MAX_MANIFEST_BYTES).ok_or(PackError::ManifestMissing)?;
    let text = String::from_utf8(bytes).map_err(|_| PackError::ManifestInvalid)?;
    // A BOM is what some editors put in front of a JSON file.
    let manifest = Manifest::parse(text.trim_start_matches('\u{feff}'))?;
    Ok(Pack {
        id: id.to_owned(),
        dir,
        manifest,
    })
}

/// Every pack folder under `root`, by id. A missing folder has no packs. Names
/// that are not a valid id, and what is not a folder, are left out.
pub fn scan(root: &Path) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<Found> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let id = entry.file_name().into_string().ok()?;
            is_valid_pack_id(&id).then(|| Found {
                pack: load(&id, &entry.path()),
                id,
            })
        })
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found.truncate(MAX_PACKS);
    found
}

/// The bytes of an image of `pack`, if the file is what the manifest says: a
/// PNG of an accepted size, no bigger than the limit, really inside the pack.
pub fn read_image(pack: &Pack, relative: &str) -> Option<Vec<u8>> {
    if !is_valid_image_path(relative) {
        return None;
    }
    let path = inside(&pack.dir, relative)?;
    let bytes = read_bounded(&path, MAX_IMAGE_BYTES)?;
    png_size(&bytes).filter(|size| is_valid_image_size(*size))?;
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A PNG header declaring `width` × `height`, and nothing else: all the
    /// reader looks at.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[0; 16]);
        bytes
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn packs() -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let root = root(temp.path());
        fs::create_dir_all(&root).unwrap();
        (temp, root)
    }

    fn good(root: &Path, id: &str) {
        let dir = root.join(id);
        write(
            &dir,
            "pack.json",
            br#"{"name":"Cores","icons":{"Calculadora":"apps/calc.png"},"fallback":"other.png"}"#,
        );
        write(&dir, "apps/calc.png", &png(96, 96));
        write(&dir, "other.png", &png(64, 64));
    }

    #[test]
    fn a_missing_folder_has_no_packs() {
        let temp = TempDir::new().unwrap();
        assert!(scan(&root(temp.path())).is_empty());
    }

    #[test]
    fn the_folders_with_a_valid_id_are_the_packs_by_id() {
        let (_temp, root) = packs();
        good(&root, "zeta");
        good(&root, "alfa");
        good(&root, "Not Valid");
        fs::write(root.join("loose-file"), b"x").unwrap();

        let found = scan(&root);

        let ids: Vec<&str> = found.iter().map(|found| found.id.as_str()).collect();
        assert_eq!(ids, ["alfa", "zeta"]);
        let pack = found[0].pack.as_ref().unwrap();
        assert_eq!(pack.manifest.name, "Cores");
    }

    #[test]
    fn a_folder_with_a_bad_manifest_is_listed_with_the_reason() {
        let (_temp, root) = packs();
        good(&root, "good");
        fs::create_dir_all(root.join("empty")).unwrap();
        write(&root.join("broken"), "pack.json", b"{ nope");
        write(&root.join("nameless"), "pack.json", b"{}");
        write(
            &root.join("traversal"),
            "pack.json",
            br#"{"name":"A","icons":{"x":"../x.png"}}"#,
        );
        write(
            &root.join("huge"),
            "pack.json",
            &vec![b' '; MAX_MANIFEST_BYTES as usize + 1],
        );

        let found = scan(&root);

        let reason = |id: &str| {
            found
                .iter()
                .find(|found| found.id == id)
                .unwrap()
                .pack
                .as_ref()
                .err()
                .copied()
        };
        assert_eq!(reason("good"), None);
        assert_eq!(reason("empty"), Some(PackError::ManifestMissing));
        assert_eq!(reason("broken"), Some(PackError::ManifestInvalid));
        assert_eq!(reason("nameless"), Some(PackError::NoName));
        assert_eq!(reason("traversal"), Some(PackError::BadPath));
        assert_eq!(reason("huge"), Some(PackError::ManifestTooBig));
    }

    #[test]
    fn a_manifest_with_a_byte_order_mark_is_read() {
        let (_temp, root) = packs();
        write(
            &root.join("bom"),
            "pack.json",
            "\u{feff}{\"name\":\"A\"}".as_bytes(),
        );

        assert!(scan(&root)[0].pack.is_ok());
    }

    #[test]
    fn an_image_is_read_when_it_is_a_png_of_an_accepted_size() {
        let (_temp, root) = packs();
        good(&root, "cores");
        let pack = scan(&root).remove(0).pack.unwrap();

        assert_eq!(read_image(&pack, "apps/calc.png"), Some(png(96, 96)));
        assert_eq!(read_image(&pack, "apps\\calc.png"), Some(png(96, 96)));
        assert_eq!(read_image(&pack, "missing.png"), None);
        assert_eq!(read_image(&pack, "../cores/other.png"), None);
    }

    #[test]
    fn an_image_that_is_not_what_it_says_is_refused() {
        let (_temp, root) = packs();
        let dir = root.join("odd");
        write(&dir, "pack.json", br#"{"name":"Odd"}"#);
        write(
            &dir,
            "text.png",
            b"this is not a png, though it is long enough",
        );
        write(&dir, "tiny.png", &png(8, 8));
        write(&dir, "wide.png", &png(4000, 64));
        let mut big = png(64, 64);
        big.resize(MAX_IMAGE_BYTES as usize + 1, 0);
        write(&dir, "big.png", &big);
        let pack = scan(&root).remove(0).pack.unwrap();

        for name in ["text.png", "tiny.png", "wide.png", "big.png"] {
            assert_eq!(read_image(&pack, name), None, "{name}");
        }
    }

    #[test]
    fn a_link_that_leaves_the_pack_is_refused() {
        let (temp, root) = packs();
        good(&root, "linked");
        let outside = temp.path().join("secret.png");
        fs::write(&outside, png(64, 64)).unwrap();
        let link = root.join("linked").join("leak.png");
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&outside, &link);
        #[cfg(not(windows))]
        let made = std::os::unix::fs::symlink(&outside, &link);
        if made.is_err() {
            // No right to make links here: nothing to check.
            return;
        }
        let pack = scan(&root).remove(0).pack.unwrap();

        assert_eq!(read_image(&pack, "leak.png"), None);
        assert!(read_image(&pack, "other.png").is_some());
    }
}
