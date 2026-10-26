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
    pub dock: DockMessages,
}

/// The dock's own texts (spec 0011 H3): the names of its items, in their
/// labels, and what a live tile shows without a value.
#[derive(Debug)]
pub struct DockMessages {
    pub settings: &'static str,
    pub about: &'static str,
    pub tray: &'static str,
    /// The names of the widget kinds, for an instance without a label.
    pub clock: &'static str,
    pub time_progress: &'static str,
    pub note: &'static str,
    pub shortcut: &'static str,
    pub files: &'static str,
    /// The progress tile without a period running.
    pub no_value: &'static str,
    /// An item's name while it carries the attention badge; `{name}` is
    /// replaced.
    pub with_notice: &'static str,
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
    dock: DockMessages {
        settings: "Configurações",
        about: "Sobre",
        tray: "Bandeja do sistema",
        clock: "Relógio",
        time_progress: "Progresso do tempo",
        note: "Nota",
        shortcut: "Atalho",
        files: "Arquivos",
        no_value: "–",
        with_notice: "{name} (com aviso)",
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
