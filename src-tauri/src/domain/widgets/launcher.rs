//! The Lançador (spec 0012, "o botão Iniciar" do PrimoDock): an item of the
//! dock that opens and closes the launcher. It has nothing to keep: the
//! hotkey it shows in its tip is the launcher's own (`launcherHotkey` in the
//! settings), so the config is empty and stays `{}`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {}

impl LauncherConfig {
    /// Always valid: there is nothing in it.
    pub fn validate(&self) -> Result<(), WidgetError> {
        Ok(())
    }
}

/// The item's name with the launcher's hotkey, as the tip shows it:
/// "Lançador (Alt+Espaço)". `hotkey` is as the settings keep it
/// (`Alt+Space`), and its Space key is named `space` in the interface's
/// language; without a hotkey (turned off in Configurações), only the name.
pub fn tip(name: &str, hotkey: Option<&str>, space: &str) -> String {
    let Some(hotkey) = hotkey else {
        return name.to_owned();
    };
    let keys: Vec<&str> = hotkey
        .split('+')
        .map(|key| if key == "Space" { space } else { key })
        .collect();
    format!("{name} ({})", keys.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_is_empty_and_reads_from_anything_older() {
        assert_eq!(
            serde_json::to_string(&LauncherConfig::default()).unwrap(),
            "{}"
        );
        let read: LauncherConfig = serde_json::from_str(r#"{"width": 3}"#).unwrap();
        assert_eq!(read, LauncherConfig::default());
        assert_eq!(read.validate(), Ok(()));
    }

    #[test]
    fn the_tip_has_the_hotkey_when_there_is_one() {
        assert_eq!(
            tip("Lançador", Some("Alt+Space"), "Espaço"),
            "Lançador (Alt+Espaço)"
        );
        assert_eq!(
            tip("Launcher", Some("Alt+Space"), "Space"),
            "Launcher (Alt+Space)"
        );
        assert_eq!(
            tip("Lançador", Some("Ctrl+Shift+K"), "Espaço"),
            "Lançador (Ctrl+Shift+K)"
        );
        assert_eq!(
            tip("Busca", Some("Win+Shift+Space"), "Espaço"),
            "Busca (Win+Shift+Espaço)"
        );
        assert_eq!(tip("Lançador", None, "Espaço"), "Lançador");
    }
}
