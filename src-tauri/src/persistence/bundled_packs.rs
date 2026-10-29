//! The icon packs that come with the app (spec 0019, G4): copied into the
//! packs folder the first time the app opens, as if the user had put them
//! there, so they show up in Configurações like any other pack and are never
//! turned on by themselves.
//!
//! A pack is copied again only when the app brings a new version of it: one
//! the user deleted stays deleted until then. What was copied is noted in
//! [`MARKER`], in the packs folder.

use std::fs;
use std::io;
use std::path::Path;

use crate::domain::icon_pack::Manifest;

/// The file, in the packs folder, with a line `<id> <version>` per pack
/// copied. Not a folder, so the scan leaves it out.
pub const MARKER: &str = ".embutidos";

/// A pack in the binary: its id and its files, by path inside the pack.
pub struct Bundled {
    pub id: &'static str,
    pub files: &'static [(&'static str, &'static [u8])],
}

macro_rules! pack_files {
    ($id:literal: $($path:literal),* $(,)?) => {
        &[$(($path, include_bytes!(concat!("../../icon-packs/", $id, "/", $path)))),*]
    };
}

/// Retrô 8-bit: pixel art made from `icon-packs/fonte/retro-8bit.txt` by
/// `icon-packs/fonte/gerar.py`. A test checks this list against the pack.
pub const RETRO_8BIT: Bundled = Bundled {
    id: "retro-8bit",
    files: pack_files!("retro-8bit":
        "pack.json",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/cartucho.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/edge.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/lixeira.png",
        "apps/notion.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Every pack in the binary.
pub const ALL: &[Bundled] = &[RETRO_8BIT];

impl Bundled {
    /// The `pack.json` of the pack, as in the binary.
    pub fn manifest(&self) -> Manifest {
        let text = self
            .files
            .iter()
            .find(|(path, _)| *path == "pack.json")
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .unwrap_or_default();
        Manifest::parse(text).unwrap_or_default()
    }

    fn line(&self) -> String {
        format!("{} {}", self.id, self.manifest().version)
    }
}

/// Copies into `root` (the packs folder) each pack of `packs` not copied yet
/// in its version. Returns whether something was copied. A pack folder that
/// is a link is left alone.
pub fn install(root: &Path, packs: &[Bundled]) -> io::Result<bool> {
    let marker = root.join(MARKER);
    let mut noted = fs::read_to_string(&marker).unwrap_or_default();
    let mut copied = false;
    for pack in packs {
        let line = pack.line();
        if noted.lines().any(|noted| noted.trim() == line) {
            continue;
        }
        let dir = root.join(pack.id);
        if fs::symlink_metadata(&dir).is_ok_and(|meta| meta.file_type().is_symlink()) {
            continue;
        }
        for (path, bytes) in pack.files {
            let file = dir.join(path);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file, bytes)?;
        }
        if !noted.is_empty() && !noted.ends_with('\n') {
            noted.push('\n');
        }
        noted.push_str(&line);
        noted.push('\n');
        fs::write(&marker, &noted)?;
        copied = true;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::icon_pack::normalize_key;
    use crate::persistence::icon_packs;
    use std::collections::{BTreeMap, BTreeSet};
    use tempfile::TempDir;

    const SOURCE: &str = include_str!("../../icon-packs/fonte/retro-8bit.txt");

    #[test]
    fn the_list_of_files_is_the_pack() {
        let manifest = RETRO_8BIT.manifest();
        assert_eq!(manifest.name, "Retrô 8-bit");
        let named: BTreeSet<&str> = manifest
            .icons
            .values()
            .chain(manifest.fallback.iter())
            .map(String::as_str)
            .collect();
        let listed: BTreeSet<&str> = RETRO_8BIT
            .files
            .iter()
            .map(|(path, _)| *path)
            .filter(|path| *path != "pack.json")
            .collect();
        assert_eq!(named, listed);
        let on_disk: BTreeSet<String> =
            fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("icon-packs/retro-8bit/apps"))
                .unwrap()
                .map(|entry| format!("apps/{}", entry.unwrap().file_name().to_string_lossy()))
                .collect();
        assert_eq!(
            on_disk,
            listed.iter().map(|path| path.to_string()).collect()
        );
    }

    #[test]
    fn the_installed_pack_is_one_the_app_takes() {
        let root = TempDir::new().unwrap();
        assert!(install(root.path(), ALL).unwrap());
        let found = icon_packs::scan(root.path());
        assert_eq!(found.len(), 1);
        let pack = found[0].pack.clone().expect("the pack is valid");
        assert_eq!(pack.id, "retro-8bit");
        for path in pack
            .manifest
            .icons
            .values()
            .chain(pack.manifest.fallback.iter())
        {
            assert!(icon_packs::read_image(&pack, path).is_some(), "{path}");
        }
        // The author's dock, by the names Windows shows.
        for app in [
            "Explorador de Arquivos",
            "FileZilla",
            "Google Chrome",
            "WhatsApp",
            "Terminal",
            "Visual Studio Code",
            "Bloco de notas",
        ] {
            assert!(pack.manifest.covers(&[app]), "{app}");
        }
        assert_eq!(
            pack.manifest.image_for(&["Um programa qualquer"]),
            Some("apps/cartucho.png")
        );
    }

    #[test]
    fn no_two_names_of_the_pack_are_one_key() {
        let manifest = RETRO_8BIT.manifest();
        let mut seen = BTreeSet::new();
        for name in manifest.icons.keys() {
            assert!(seen.insert(normalize_key(name)), "{name}");
        }
    }

    #[test]
    fn a_pack_is_copied_once_per_version() {
        let root = TempDir::new().unwrap();
        assert!(install(root.path(), ALL).unwrap());
        assert!(!install(root.path(), ALL).unwrap());
        // Deleted by the user: it stays deleted.
        fs::remove_dir_all(root.path().join("retro-8bit")).unwrap();
        assert!(!install(root.path(), ALL).unwrap());
        assert!(!root.path().join("retro-8bit").exists());
        // A new version comes back.
        fs::write(root.path().join(MARKER), "retro-8bit 0.9.0\n").unwrap();
        assert!(install(root.path(), ALL).unwrap());
        assert!(root.path().join("retro-8bit/pack.json").is_file());
        let marker = fs::read_to_string(root.path().join(MARKER)).unwrap();
        assert_eq!(marker, format!("retro-8bit 0.9.0\n{}\n", RETRO_8BIT.line()));
    }

    /// What `retro-8bit.txt` says about the images.
    struct Source {
        /// A character of a drawing → its color.
        palette: BTreeMap<char, [u8; 3]>,
        /// The drawings by file: 16 lines of 16 characters each.
        drawings: BTreeMap<String, Vec<String>>,
        /// How many pixels of the PNG a character is, on each side.
        scale: usize,
    }

    fn source() -> Source {
        let mut palette = BTreeMap::new();
        let mut drawings: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut scale = 0;
        let mut current = String::new();
        for line in SOURCE.lines().map(str::trim_end) {
            if line.is_empty() || line.starts_with('#') || line.starts_with("nomes:") {
                continue;
            }
            if let Some(value) = line.strip_prefix("pack.scale:") {
                scale = value.trim().parse().unwrap();
                continue;
            }
            if line.starts_with("pack.") {
                continue;
            }
            if let Some(color) = line.strip_prefix("cor.") {
                let (key, hex) = color.split_once(':').unwrap();
                let hex = hex.trim();
                let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap();
                palette.insert(
                    key.trim().chars().next().unwrap(),
                    [byte(1), byte(3), byte(5)],
                );
            } else if let Some(id) = line.strip_prefix('@') {
                current = format!("apps/{id}.png");
                drawings.insert(current.clone(), Vec::new());
            } else {
                drawings.get_mut(&current).unwrap().push(line.to_owned());
            }
        }
        Source {
            palette,
            drawings,
            scale,
        }
    }

    #[test]
    fn the_images_are_the_drawings_of_the_source() {
        let Source {
            palette,
            drawings,
            scale,
        } = source();
        let images: BTreeMap<&str, &[u8]> = RETRO_8BIT
            .files
            .iter()
            .filter(|(path, _)| *path != "pack.json")
            .copied()
            .collect();
        assert_eq!(
            drawings.keys().map(String::as_str).collect::<Vec<_>>(),
            images.keys().copied().collect::<Vec<_>>()
        );
        for (path, rows) in &drawings {
            let mut reader = png::Decoder::new(images[path.as_str()])
                .read_info()
                .unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            let frame = reader.next_frame(&mut pixels).unwrap();
            assert_eq!(frame.color_type, png::ColorType::Rgba, "{path}");
            assert_eq!(
                (frame.width, frame.height),
                (16 * scale as u32, 16 * scale as u32)
            );
            for (y, row) in rows.iter().enumerate() {
                for (x, cell) in row.chars().enumerate() {
                    let expected = match cell {
                        '.' => [0, 0, 0, 0],
                        _ => {
                            let [r, g, b] = palette[&cell];
                            [r, g, b, 255]
                        }
                    };
                    // Every pixel of the 8×8 block is the cell's color.
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let at = ((y * scale + dy) * 16 * scale + x * scale + dx) * 4;
                            assert_eq!(pixels[at..at + 4], expected, "{path} ({x}, {y})");
                        }
                    }
                }
            }
        }
    }
}
