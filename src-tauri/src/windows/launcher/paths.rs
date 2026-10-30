//! Reads the folder of a path typed in the launcher (spec 0012 L5), off its
//! thread: `std::fs::read_dir` only, which costs little and loads neither
//! the shell nor COM in the app's process. The icons of the rows come from
//! the helper, as the files' (L4).

use std::os::windows::fs::MetadataExt;
use std::time::{Duration, Instant};

use crate::domain::launcher_paths::{Entry, MAX_ENTRIES, Problem, TIMEOUT_MS};

/// `FILE_ATTRIBUTE_HIDDEN`, `FILE_ATTRIBUTE_SYSTEM` and
/// `FILE_ATTRIBUTE_DIRECTORY`.
const HIDDEN: u32 = 0x2;
const SYSTEM: u32 = 0x4;
const DIRECTORY: u32 = 0x10;

/// Reads `folder` on a thread of its own and hands `done` what it found.
/// A folder that hangs (a share that does not answer) keeps only that
/// thread; the launcher stops waiting at `TIMEOUT_MS` and drops the answer.
pub fn read(folder: String, done: impl FnOnce(Result<Vec<Entry>, Problem>) + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("launcher-folder".into())
        .spawn(move || done(entries(&folder)));
    if let Err(error) = spawned {
        log::warn!("launcher: a folder could not be read: {error}");
    }
}

/// The entries of `folder`, at most `MAX_ENTRIES`, read until `TIMEOUT_MS`
/// passes: a huge folder gives what was read by then.
fn entries(folder: &str) -> Result<Vec<Entry>, Problem> {
    let deadline = Instant::now() + Duration::from_millis(u64::from(TIMEOUT_MS));
    let listing = std::fs::read_dir(folder).map_err(|error| problem(&error))?;
    let mut entries = Vec::new();
    for entry in listing {
        if entries.len() >= MAX_ENTRIES || Instant::now() >= deadline {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        // The attributes come with the listing: no other call to the disk.
        let attributes = entry
            .metadata()
            .map(|metadata| metadata.file_attributes())
            .unwrap_or(0);
        entries.push(Entry::new(
            &name,
            attributes & DIRECTORY != 0,
            attributes & (HIDDEN | SYSTEM) != 0,
        ));
    }
    Ok(entries)
}

fn problem(error: &std::io::Error) -> Problem {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => Problem::Denied,
        _ => Problem::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_read_with_its_kinds_and_its_problems() {
        let root = std::env::temp_dir().join(format!("dockyard-l5-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Pasta")).unwrap();
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let folder = format!("{}\\", root.display());
        let mut read = entries(&folder).unwrap();
        read.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            read,
            [
                Entry::new("Pasta", true, false),
                Entry::new("a.txt", false, false)
            ]
        );
        assert_eq!(
            entries(&format!("{folder}nao existe\\")),
            Err(Problem::NotFound)
        );
        // A file is not a folder.
        assert_eq!(entries(&format!("{folder}a.txt\\")), Err(Problem::NotFound));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
