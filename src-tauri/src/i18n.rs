//! User-visible texts of the native side (the tray menu), the Rust
//! counterpart of `src/i18n/pt-BR.ts` (spec 0002 §2.3 item 7).
//!
//! `Messages` is the contract: a new locale is one more `Messages` value,
//! and the compiler rejects it if a text is missing. Only PT-BR exists for
//! now, so `messages()` returns it.

/// Every text of the native interface, grouped by area.
#[derive(Debug)]
pub struct Messages {
    pub tray: TrayMessages,
    pub settings: SettingsMessages,
}

/// The Configurações app (spec 0011 H1).
#[derive(Debug)]
pub struct SettingsMessages {
    /// The window's title, in its title bar and in Alt+Tab.
    pub window_title: &'static str,
}

/// Items of the tray icon menu.
#[derive(Debug)]
pub struct TrayMessages {
    /// Shows the dock when hidden, hides it otherwise.
    pub toggle_dock: &'static str,
    /// Opens the Configurações app.
    pub open_settings: &'static str,
    /// Ends the app.
    pub quit: &'static str,
}

const PT_BR: Messages = Messages {
    tray: TrayMessages {
        toggle_dock: "Mostrar/ocultar dock",
        open_settings: "Configurações",
        quit: "Sair",
    },
    settings: SettingsMessages {
        window_title: "Configurações do DockMax",
    },
};

/// Texts of the current locale.
pub fn messages() -> &'static Messages {
    &PT_BR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pt_br_has_every_tray_text() {
        let tray = &messages().tray;

        assert_eq!(tray.toggle_dock, "Mostrar/ocultar dock");
        assert_eq!(tray.open_settings, "Configurações");
        assert_eq!(tray.quit, "Sair");
    }
}
