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
}

/// Items of the tray icon menu.
#[derive(Debug)]
pub struct TrayMessages {
    /// Shows the dock when hidden, hides it otherwise.
    pub toggle_dock: &'static str,
    /// Opens the Settings panel, showing the dock first if it is hidden.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the tray in R1"))]
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
