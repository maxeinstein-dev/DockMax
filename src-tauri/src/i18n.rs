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
    pub launcher: LauncherMessages,
}

/// The launcher (spec 0012).
#[derive(Debug)]
pub struct LauncherMessages {
    /// The window's name for assistive technology.
    pub title: &'static str,
    /// What the empty search box shows, and its name.
    pub placeholder: &'static str,
    /// What a program is, under its name.
    pub app: &'static str,
    /// What the other results are, under their names (spec 0012 L2).
    pub windows_page: &'static str,
    pub dockmax_page: &'static str,
    pub calculator: &'static str,
    pub web: &'static str,
    /// A web search's name and what is under it; `{term}` and `{name}` are
    /// replaced.
    pub search_for: &'static str,
    pub search_with: &'static str,
    /// Said once a sum's result is in the clipboard.
    pub copied: &'static str,
    /// The title above the most used, with nothing typed.
    pub recent: &'static str,
    pub reading: &'static str,
    pub hint: &'static str,
    pub nothing_found: &'static str,
    /// Under a program that could not be opened.
    pub failed: &'static str,
    /// How many results there are; `{n}` is replaced.
    pub results: &'static str,
    pub one_result: &'static str,
    /// The selected result, read aloud; `{name}`, `{kind}`, `{n}` and
    /// `{count}` are replaced.
    pub selected: &'static str,
}

/// The dock's own texts (spec 0011 H3): the names of its items, in their
/// labels, and what a live tile shows without a value.
#[derive(Debug)]
pub struct DockMessages {
    pub settings: &'static str,
    /// The names of the widget kinds, for an instance without a label.
    pub clock: &'static str,
    pub time_progress: &'static str,
    pub note: &'static str,
    pub shortcut: &'static str,
    pub files: &'static str,
    pub activity: &'static str,
    pub network: &'static str,
    /// The progress tile without a period running.
    pub no_value: &'static str,
    /// The periods' names on a wide progress tile, in the order of
    /// `Period::ALL`.
    pub periods: [&'static str; 5],
    /// An item's name while it carries the attention badge; `{name}` is
    /// replaced.
    pub with_notice: &'static str,
    /// A workspace dot, for assistive technology (spec 0013 C3), and the one
    /// shown; `{name}` is replaced.
    pub workspace: &'static str,
    pub workspace_shown: &'static str,
    /// The preview card (spec 0015), for assistive technology: the card,
    /// a window without a title, and a close button; `{name}` is replaced.
    pub previews: &'static str,
    pub untitled: &'static str,
    pub close_window: &'static str,
    /// The labels of the small icons at the end; `{n}` is a percentage.
    pub show_hidden_icons: &'static str,
    pub tray_icon: &'static str,
    pub wifi: &'static str,
    pub wired: &'static str,
    pub offline: &'static str,
    pub volume: &'static str,
    pub muted: &'static str,
    pub battery: &'static str,
    pub battery_charging: &'static str,
}

/// The Configurações app (spec 0011 H1).
#[derive(Debug)]
pub struct SettingsMessages {
    /// The window's title, in its title bar and in Alt+Tab.
    pub window_title: &'static str,
}

/// Items of the tray icon menu and of the dock's own menu.
#[derive(Debug)]
pub struct TrayMessages {
    /// Opens the Windows Task Manager, as the taskbar's menu does.
    pub task_manager: &'static str,
    /// Opens the launcher (spec 0012).
    pub search_apps: &'static str,
    /// Turns the dock's own hiding on and off (checked when on).
    pub auto_hide: &'static str,
    /// Opens the Configurações app.
    pub open_settings: &'static str,
    /// Ends the app.
    pub quit: &'static str,
    /// Makes a workspace and shows it (the dock's menu, spec 0013).
    pub new_workspace: &'static str,
    /// The title over the workspaces in the dock's menu (spec 0014 R3).
    pub workspaces: &'static str,
    /// The title over a widget's widths in the dock's menu (spec 0017
    /// F1b), and each width: `{n}` is the number of icons.
    pub size: &'static str,
    pub one_icon: &'static str,
    pub icons: &'static str,
}

const PT_BR: Messages = Messages {
    tray: TrayMessages {
        task_manager: "Gerenciador de Tarefas",
        search_apps: "Buscar apps…",
        auto_hide: "Ocultar automaticamente",
        open_settings: "Configurações",
        quit: "Sair",
        new_workspace: "Novo espaço",
        workspaces: "Espaços de trabalho",
        size: "Tamanho",
        one_icon: "1 ícone",
        icons: "{n} ícones",
    },
    settings: SettingsMessages {
        window_title: "Configurações do DockMax",
    },
    dock: DockMessages {
        settings: "Configurações",
        clock: "Relógio",
        time_progress: "Progresso do tempo",
        note: "Nota",
        shortcut: "Atalho",
        files: "Arquivos",
        activity: "Atividade",
        network: "Atividade de rede",
        no_value: "–",
        periods: ["Dia", "Semana", "Mês", "Ano", "Expediente"],
        with_notice: "{name} (com aviso)",
        workspace: "Espaço {name}",
        workspace_shown: "Espaço {name} (mostrado)",
        previews: "Janelas de {name}",
        untitled: "Janela sem título",
        close_window: "Fechar {name}",
        show_hidden_icons: "Mostrar ícones ocultos",
        tray_icon: "Ícone da bandeja",
        wifi: "Wi-Fi",
        wired: "Rede com fio",
        offline: "Sem conexão",
        volume: "Volume: {n}%",
        muted: "Sem som",
        battery: "Bateria: {n}%",
        battery_charging: "Bateria: {n}% (carregando)",
    },
    launcher: LauncherMessages {
        title: "Lançador do DockMax",
        placeholder: "Buscar apps",
        app: "App",
        windows_page: "Configurações do Windows",
        dockmax_page: "Configurações do DockMax",
        calculator: "Calculadora · Enter copia o resultado",
        web: "Busca na web",
        search_for: "Buscar “{term}”",
        search_with: "{name} · busca na web",
        copied: "Resultado copiado",
        recent: "Recentes",
        reading: "Lendo os apps…",
        hint: "Digite para buscar apps e configurações ou fazer uma conta",
        nothing_found: "Nenhum resultado",
        failed: "Não foi possível abrir",
        results: "{n} resultados",
        one_result: "1 resultado",
        selected: "{name}, {kind}, {n} de {count}",
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

        assert_eq!(tray.task_manager, "Gerenciador de Tarefas");
        assert_eq!(tray.auto_hide, "Ocultar automaticamente");
        assert_eq!(tray.open_settings, "Configurações");
        assert_eq!(tray.quit, "Sair");
    }
}
