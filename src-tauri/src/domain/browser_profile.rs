//! The profiles of Google Chrome (spec 0009): which profile a window belongs
//! to, and the name and photo it has. Pure rules: the files are read in
//! `windows/profiles.rs`.
//!
//! Chrome gives every window an AppID that carries its profile: `Chrome` for
//! the default profile and `Chrome.UserData.<folder>` for the others, the
//! folder's name without its spaces (`Profile 1` is `Profile1`). The Windows
//! taskbar uses it to keep the profiles apart, and so does the dock.

use serde_json::Value;

/// One profile of Chrome, from the `Local State` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfile {
    /// The profile's folder under `User Data` (`Default`, `Profile 1`).
    pub dir: String,
    /// The name the user gave it.
    pub name: String,
    /// The file of the account's photo inside the folder, if there is one.
    pub photo: Option<String>,
}

/// Whether `name` can be the name of a file inside a profile folder: no
/// folder, no drive, no stream, a `.png`.
pub fn is_photo_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.contains(['/', '\\', ':'])
        && !name.contains("..")
        && !name.chars().any(char::is_control)
        && name.to_ascii_lowercase().ends_with(".png")
}

/// The profiles `Local State` lists (`profile.info_cache`), in the order of
/// their folders' names. Anything that is not what Chrome writes is left out;
/// text that is not JSON gives none.
pub fn parse_local_state(text: &str) -> Vec<ChromeProfile> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(cache) = value
        .get("profile")
        .and_then(|profile| profile.get("info_cache"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    let mut profiles: Vec<ChromeProfile> = cache
        .iter()
        .filter(|(dir, _)| is_profile_dir(dir))
        .map(|(dir, entry)| ChromeProfile {
            dir: dir.clone(),
            name: entry
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty() && name.chars().count() <= 100)
                .map_or_else(|| dir.clone(), str::to_owned),
            photo: entry
                .get("gaia_picture_file_name")
                .and_then(Value::as_str)
                .filter(|file| is_photo_file_name(file))
                .map(str::to_owned),
        })
        .collect();
    profiles.sort_by(|a, b| a.dir.cmp(&b.dir));
    profiles
}

/// Whether `dir` is a profile folder's name: one name, not a path.
fn is_profile_dir(dir: &str) -> bool {
    !dir.is_empty()
        && dir.len() <= 100
        && !dir.contains(['/', '\\', ':'])
        && !dir.contains("..")
        && !dir.chars().any(char::is_control)
}

/// The profile a window with the AppID `aumid` belongs to, if the AppID is one
/// Chrome makes and the profile is in `profiles`.
pub fn profile_for_aumid<'a>(
    aumid: &str,
    profiles: &'a [ChromeProfile],
) -> Option<&'a ChromeProfile> {
    if aumid.eq_ignore_ascii_case("Chrome") {
        return profiles.iter().find(|profile| profile.dir == "Default");
    }
    let folder = aumid
        .get(..16)
        .filter(|start| start.eq_ignore_ascii_case("Chrome.UserData."))
        .map(|_| &aumid[16..])?;
    profiles
        .iter()
        .find(|profile| profile.dir.replace(' ', "").eq_ignore_ascii_case(folder))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL_STATE: &str = r#"{
      "browser": {"enabled_labs_experiments": ["x"]},
      "profile": {"info_cache": {
        "Profile 1": {"name": "Trabalho", "gaia_picture_file_name": "Google Profile Picture.png", "user_name": "a@b.c"},
        "Default": {"name": "Pessoal", "gaia_picture_file_name": "Google Profile Picture.png"},
        "Profile 3": {"name": "  ", "gaia_picture_file_name": "../../secret.png"},
        "../Evil": {"name": "Evil"},
        "Guest Profile": {"name": "Convidado"}
      }}
    }"#;

    #[test]
    fn the_profiles_come_from_the_info_cache_by_folder() {
        let profiles = parse_local_state(LOCAL_STATE);

        let dirs: Vec<&str> = profiles.iter().map(|p| p.dir.as_str()).collect();
        assert_eq!(dirs, ["Default", "Guest Profile", "Profile 1", "Profile 3"]);
        assert_eq!(profiles[0].name, "Pessoal");
        assert_eq!(
            profiles[0].photo.as_deref(),
            Some("Google Profile Picture.png")
        );
        // A blank name falls back to the folder, and a photo that is a path
        // is not a photo.
        assert_eq!(profiles[3].name, "Profile 3");
        assert_eq!(profiles[3].photo, None);
        assert_eq!(profiles[1].photo, None);
    }

    #[test]
    fn what_is_not_chromes_local_state_has_no_profiles() {
        for text in [
            "",
            "not json",
            "[]",
            r#"{"profile":{}}"#,
            r#"{"profile":{"info_cache":[1]}}"#,
        ] {
            assert!(parse_local_state(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn photo_files_are_png_files_of_the_folder() {
        assert!(is_photo_file_name("Google Profile Picture.png"));
        assert!(is_photo_file_name("a.PNG"));
        for bad in [
            "",
            "a.jpg",
            "a",
            "dir/a.png",
            "dir\\a.png",
            "C:a.png",
            "a.png:s.png",
            "..png",
            "a\u{0}.png",
        ] {
            assert!(!is_photo_file_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_appid_of_a_window_names_its_profile() {
        let profiles = parse_local_state(LOCAL_STATE);

        let found = |aumid: &str| profile_for_aumid(aumid, &profiles).map(|p| p.name.as_str());
        assert_eq!(found("Chrome"), Some("Pessoal"));
        assert_eq!(found("chrome"), Some("Pessoal"));
        assert_eq!(found("Chrome.UserData.Profile1"), Some("Trabalho"));
        assert_eq!(found("Chrome.UserData.profile1"), Some("Trabalho"));
        assert_eq!(found("Chrome.UserData.Default"), Some("Pessoal"));
        assert_eq!(found("Chrome.UserData.Profile9"), None);
        assert_eq!(found("Chrome.UserData."), None);
        assert_eq!(found("Chromium"), None);
        assert_eq!(found("Microsoft.Edge"), None);
        assert_eq!(found(""), None);
        // Not slicing in the middle of a character.
        assert_eq!(found("Chrome.UserData.Perfil\u{e7}"), None);
        assert_eq!(
            found("\u{e7}\u{e7}\u{e7}\u{e7}\u{e7}\u{e7}\u{e7}\u{e7}\u{e7}"),
            None
        );
    }
}
