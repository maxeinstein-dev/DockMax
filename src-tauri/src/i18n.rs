//! User-visible texts of the native side (the tray menu), the Rust
//! counterpart of `src/i18n/pt-BR.ts` (spec 0002 §2.3 item 7).
//!
//! `Messages` is the contract: a new locale is one more `Messages` value,
//! and the compiler rejects it if a text is missing. PT-BR and English
//! (spec 0019 G1); `set_language` picks one, from the settings.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::settings::Language;

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
    pub dockyard_page: &'static str,
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
    pub calendar: &'static str,
    pub cycle: &'static str,
    pub countdown: &'static str,
    /// The Contagem regressiva's tile: under the days, on the day, and
    /// once it is gone.
    pub countdown_days: &'static str,
    pub countdown_day: &'static str,
    pub countdown_today: &'static str,
    pub countdown_ago: &'static str,
    pub pomodoro: &'static str,
    /// The Pomodoro's notice of Windows, when it asks for one.
    pub pomodoro_focus_done: &'static str,
    pub pomodoro_pause_done: &'static str,
    pub todo: &'static str,
    /// The Lista de tarefas' tile: under the number open, one or many.
    pub todo_open: &'static str,
    pub todo_open_many: &'static str,
    /// The tile with none open.
    pub todo_none: &'static str,
    pub water: &'static str,
    pub media: &'static str,
    pub claude: &'static str,
    /// The Uso do Claude's meters: the five hours' and the week's.
    pub claude_five: &'static str,
    pub claude_five_short: &'static str,
    pub claude_week: &'static str,
    pub claude_week_short: &'static str,
    pub weather: &'static str,
    /// The media buttons of a window preview (F4).
    pub media_previous: &'static str,
    pub media_play: &'static str,
    pub media_pause: &'static str,
    pub media_next: &'static str,
    /// The progress tile without a period running.
    pub no_value: &'static str,
    /// The Calendário's tile (spec 0017 F2a): no account, no meeting left,
    /// and when the next one is (`{n}` minutes).
    pub calendar_connect: &'static str,
    pub calendar_free: &'static str,
    pub calendar_now: &'static str,
    pub calendar_in: &'static str,
    pub calendar_minutes: &'static str,
    pub calendar_tomorrow: &'static str,
    /// The Calendário's card (F2a-4): the days, a task late, the all-day
    /// events, the tasks' heading and an empty span. Weekdays, Sunday first.
    pub calendar_today: &'static str,
    pub calendar_tomorrow_title: &'static str,
    pub weekdays: [&'static str; 7],
    pub calendar_all_day: &'static str,
    pub calendar_tasks: &'static str,
    pub calendar_late: &'static str,
    pub calendar_nothing: &'static str,
    /// The Ciclo's Windows notice (F2b-2): neutral, the same for all.
    pub cycle_notice_title: &'static str,
    pub cycle_notice_line: &'static str,
    /// The notice of a new version (spec 0019 G3), once per version;
    /// `{version}` is replaced.
    pub update_notice_title: &'static str,
    pub update_notice_line: &'static str,
    /// A meeting with no title: from a free/busy calendar, or untitled.
    pub calendar_busy: &'static str,
    pub calendar_untitled: &'static str,
    /// Before the meeting after the next one: `Depois 15:00 Daily`.
    pub calendar_then: &'static str,
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
    /// The welcome tour's window (spec 0019 G2).
    pub tour_title: &'static str,
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
    /// Takes the widget clicked out of the dock (its menu).
    pub remove_widget: &'static str,
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
        remove_widget: "Remover do dock",
    },
    settings: SettingsMessages {
        window_title: "Configurações do Dockyard",
        tour_title: "Boas-vindas ao Dockyard",
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
        calendar: "Calendário",
        cycle: "Ciclo",
        countdown: "Contagem regressiva",
        countdown_days: "dias",
        countdown_day: "dia",
        countdown_today: "Hoje",
        countdown_ago: "atrás",
        pomodoro: "Pomodoro",
        pomodoro_focus_done: "Fim do foco: hora da pausa.",
        pomodoro_pause_done: "Fim da pausa: hora de focar.",
        todo: "Lista de tarefas",
        todo_open: "aberta",
        todo_open_many: "abertas",
        todo_none: "✓",
        water: "Beber água",
        media: "Tocando agora",
        claude: "Uso do Claude",
        claude_five: "5 horas",
        claude_five_short: "5h",
        claude_week: "Semana",
        claude_week_short: "Sem",
        weather: "Clima",
        media_previous: "Anterior",
        media_play: "Tocar",
        media_pause: "Pausar",
        media_next: "Próxima",
        no_value: "–",
        calendar_connect: "Conectar",
        calendar_free: "Livre",
        calendar_now: "agora",
        calendar_in: "em {n} min",
        calendar_minutes: "{n} min",
        calendar_tomorrow: "amanhã",
        calendar_today: "Hoje",
        calendar_tomorrow_title: "Amanhã",
        weekdays: ["dom.", "seg.", "ter.", "qua.", "qui.", "sex.", "sáb."],
        calendar_all_day: "Dia inteiro",
        calendar_tasks: "Tarefas",
        calendar_late: "atrasada",
        calendar_nothing: "Nada marcado",
        cycle_notice_title: "Lembrete do Ciclo",
        cycle_notice_line: "Abra o Ciclo no Dockyard para ver.",
        update_notice_title: "Dockyard {version} disponível",
        update_notice_line: "Para atualizar, abra Sobre no dock.",
        calendar_busy: "Ocupado",
        calendar_untitled: "(sem título)",
        calendar_then: "Depois",
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
        title: "Lançador do Dockyard",
        placeholder: "Buscar apps",
        app: "App",
        windows_page: "Configurações do Windows",
        dockyard_page: "Configurações do Dockyard",
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

const EN: Messages = Messages {
    tray: TrayMessages {
        task_manager: "Task Manager",
        search_apps: "Search apps…",
        auto_hide: "Automatically hide",
        open_settings: "Settings",
        quit: "Quit",
        new_workspace: "New workspace",
        workspaces: "Workspaces",
        size: "Size",
        one_icon: "1 icon",
        icons: "{n} icons",
        remove_widget: "Remove from dock",
    },
    settings: SettingsMessages {
        window_title: "Dockyard Settings",
        tour_title: "Welcome to Dockyard",
    },
    dock: DockMessages {
        settings: "Settings",
        clock: "Clock",
        time_progress: "Time progress",
        note: "Note",
        shortcut: "Shortcut",
        files: "Files",
        activity: "Activity",
        network: "Network activity",
        calendar: "Calendar",
        cycle: "Cycle",
        countdown: "Countdown",
        countdown_days: "days",
        countdown_day: "day",
        countdown_today: "Today",
        countdown_ago: "ago",
        pomodoro: "Pomodoro",
        pomodoro_focus_done: "Focus over: time for a break.",
        pomodoro_pause_done: "Break over: time to focus.",
        todo: "To-do list",
        todo_open: "open",
        todo_open_many: "open",
        todo_none: "✓",
        water: "Drink water",
        media: "Now playing",
        claude: "Claude usage",
        claude_five: "5 hours",
        claude_five_short: "5h",
        claude_week: "Week",
        claude_week_short: "Wk",
        weather: "Weather",
        media_previous: "Previous",
        media_play: "Play",
        media_pause: "Pause",
        media_next: "Next",
        no_value: "–",
        calendar_connect: "Connect",
        calendar_free: "Free",
        calendar_now: "now",
        calendar_in: "in {n} min",
        calendar_minutes: "{n} min",
        calendar_tomorrow: "tomorrow",
        calendar_today: "Today",
        calendar_tomorrow_title: "Tomorrow",
        weekdays: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
        calendar_all_day: "All day",
        calendar_tasks: "Tasks",
        calendar_late: "overdue",
        calendar_nothing: "Nothing scheduled",
        cycle_notice_title: "Cycle reminder",
        cycle_notice_line: "Open Cycle in Dockyard to see it.",
        update_notice_title: "Dockyard {version} available",
        update_notice_line: "To update, open About in the dock.",
        calendar_busy: "Busy",
        calendar_untitled: "(no title)",
        calendar_then: "Then",
        periods: ["Day", "Week", "Month", "Year", "Workday"],
        with_notice: "{name} (with notice)",
        workspace: "Workspace {name}",
        workspace_shown: "Workspace {name} (shown)",
        previews: "{name} windows",
        untitled: "Untitled window",
        close_window: "Close {name}",
        show_hidden_icons: "Show hidden icons",
        tray_icon: "Tray icon",
        wifi: "Wi-Fi",
        wired: "Wired network",
        offline: "No connection",
        volume: "Volume: {n}%",
        muted: "Muted",
        battery: "Battery: {n}%",
        battery_charging: "Battery: {n}% (charging)",
    },
    launcher: LauncherMessages {
        title: "Dockyard launcher",
        placeholder: "Search apps",
        app: "App",
        windows_page: "Windows Settings",
        dockyard_page: "Dockyard Settings",
        calculator: "Calculator · Enter copies the result",
        web: "Web search",
        search_for: "Search “{term}”",
        search_with: "{name} · web search",
        copied: "Result copied",
        recent: "Recent",
        reading: "Reading apps…",
        hint: "Type to search apps and settings or do a calculation",
        nothing_found: "No results",
        failed: "Couldn't open",
        results: "{n} results",
        one_result: "1 result",
        selected: "{name}, {kind}, {n} of {count}",
    },
};

/// Texts of the current locale.
/// Whether the texts are English; PT-BR until the settings are read.
static ENGLISH: AtomicBool = AtomicBool::new(false);

/// The texts of the active language.
pub fn messages() -> &'static Messages {
    if ENGLISH.load(Ordering::Relaxed) {
        &EN
    } else {
        &PT_BR
    }
}

/// Uses the texts of `language` from now on: the menus are built again at
/// each opening, and the dock's names at its next paint.
pub fn set_language(language: Language) {
    ENGLISH.store(
        is_english(language, system_is_portuguese),
        Ordering::Relaxed,
    );
}

/// Whether `language` means English; "Sistema" asks `portuguese` whether
/// Windows is shown in Portuguese.
fn is_english(language: Language, portuguese: impl FnOnce() -> bool) -> bool {
    match language {
        Language::En => true,
        Language::PtBr => false,
        Language::System => !portuguese(),
    }
}

/// Whether the Windows display language is a Portuguese one.
#[cfg(windows)]
fn system_is_portuguese() -> bool {
    /// `LANG_PORTUGUESE`: the primary language of every Portuguese.
    const PORTUGUESE: u16 = 0x16;
    let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    id & 0x3ff == PORTUGUESE
}

#[cfg(not(windows))]
fn system_is_portuguese() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_language_decides_only_for_sistema() {
        assert!(is_english(Language::En, || true));
        assert!(!is_english(Language::PtBr, || false));
        assert!(!is_english(Language::System, || true));
        assert!(is_english(Language::System, || false));
    }

    #[test]
    fn english_keeps_the_placeholders() {
        assert!(EN.launcher.search_for.contains("{term}"));
        assert!(EN.dock.update_notice_title.contains("{version}"));
        assert!(PT_BR.dock.update_notice_title.contains("{version}"));
        assert_eq!(EN.tray.open_settings, "Settings");
    }

    #[test]
    fn pt_br_has_every_tray_text() {
        let tray = &messages().tray;

        assert_eq!(tray.task_manager, "Gerenciador de Tarefas");
        assert_eq!(tray.auto_hide, "Ocultar automaticamente");
        assert_eq!(tray.open_settings, "Configurações");
        assert_eq!(tray.quit, "Sair");
    }
}
