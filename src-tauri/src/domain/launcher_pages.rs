//! The settings pages the launcher finds (spec 0012 §5, L2): a fixed list,
//! in the code, of the `ms-settings:` pages used most and of the sections of
//! the Dockyard's Configurações, each with its name in Portuguese and other
//! words that find it (`wifi`, `internet` and `rede` find "Wi-Fi"). Pure.

/// A section of the Configurações app; must match `Section` in
/// `src/settings/sections.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Workspaces,
    Look,
    Apps,
    Items,
    Accounts,
    About,
}

impl Section {
    /// Its name in the interface (`look`).
    pub fn as_str(self) -> &'static str {
        match self {
            Section::General => "general",
            Section::Workspaces => "workspaces",
            Section::Look => "look",
            Section::Apps => "apps",
            Section::Items => "items",
            Section::Accounts => "accounts",
            Section::About => "about",
        }
    }
}

/// Where a page is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A page of the Windows settings (`ms-settings:network-wifi`).
    Windows(&'static str),
    /// A section of the Dockyard's Configurações.
    Dockyard(Section),
}

/// One page the launcher finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// What its use is kept under (`setting:<id>`).
    pub id: &'static str,
    pub name: &'static str,
    /// Other words that find it.
    pub aliases: &'static [&'static str],
    pub target: Target,
}

const fn windows(
    id: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
    uri: &'static str,
) -> Page {
    Page {
        id,
        name,
        aliases,
        target: Target::Windows(uri),
    }
}

const fn dockyard(
    id: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
    section: Section,
) -> Page {
    Page {
        id,
        name,
        aliases,
        target: Target::Dockyard(section),
    }
}

/// Every page, Windows' first.
pub const PAGES: &[Page] = &[
    windows(
        "wifi",
        "Wi-Fi",
        &["wifi", "wireless", "internet", "rede sem fio"],
        "ms-settings:network-wifi",
    ),
    windows(
        "network",
        "Rede e Internet",
        &["rede", "internet", "ethernet", "network"],
        "ms-settings:network-status",
    ),
    windows("vpn", "VPN", &[], "ms-settings:network-vpn"),
    windows(
        "airplane",
        "Modo avião",
        &["aviao", "airplane"],
        "ms-settings:network-airplanemode",
    ),
    windows(
        "hotspot",
        "Hotspot móvel",
        &["roteador", "compartilhar internet"],
        "ms-settings:network-mobilehotspot",
    ),
    windows("proxy", "Proxy", &[], "ms-settings:network-proxy"),
    windows(
        "bluetooth",
        "Bluetooth e dispositivos",
        &["bluetooth", "fone", "dispositivos"],
        "ms-settings:bluetooth",
    ),
    windows(
        "printers",
        "Impressoras e scanners",
        &["impressora", "scanner", "imprimir"],
        "ms-settings:printers",
    ),
    windows(
        "mouse",
        "Mouse",
        &["touchpad", "ponteiro"],
        "ms-settings:mousetouchpad",
    ),
    windows(
        "sound",
        "Som",
        &["audio", "volume", "alto-falante", "microfone"],
        "ms-settings:sound",
    ),
    windows(
        "display",
        "Vídeo",
        &["tela", "monitor", "resolucao", "brilho", "escala"],
        "ms-settings:display",
    ),
    windows(
        "nightlight",
        "Luz noturna",
        &["luz azul"],
        "ms-settings:nightlight",
    ),
    windows(
        "power",
        "Energia e bateria",
        &["energia", "bateria", "suspensao", "desligar tela"],
        "ms-settings:powersleep",
    ),
    windows(
        "storage",
        "Armazenamento",
        &["disco", "espaco", "limpeza"],
        "ms-settings:storagesense",
    ),
    windows(
        "notifications",
        "Notificações",
        &["avisos", "nao incomodar"],
        "ms-settings:notifications",
    ),
    windows(
        "multitasking",
        "Multitarefa",
        &["janelas", "ajustar janelas", "snap"],
        "ms-settings:multitasking",
    ),
    windows(
        "clipboard",
        "Área de transferência",
        &["copiar", "colar", "historico"],
        "ms-settings:clipboard",
    ),
    windows(
        "background",
        "Plano de fundo",
        &["papel de parede", "wallpaper"],
        "ms-settings:personalization-background",
    ),
    windows(
        "colors",
        "Cores",
        &["modo escuro", "tema escuro", "destaque"],
        "ms-settings:colors",
    ),
    windows("themes", "Temas", &[], "ms-settings:themes"),
    windows(
        "lockscreen",
        "Tela de bloqueio",
        &["bloqueio"],
        "ms-settings:lockscreen",
    ),
    windows(
        "taskbar",
        "Barra de tarefas",
        &["taskbar"],
        "ms-settings:taskbar",
    ),
    windows("fonts", "Fontes", &[], "ms-settings:fonts"),
    windows(
        "installed",
        "Aplicativos instalados",
        &["apps", "desinstalar", "programas"],
        "ms-settings:appsfeatures",
    ),
    windows(
        "defaults",
        "Aplicativos padrão",
        &["apps padrao", "abrir com", "navegador padrao"],
        "ms-settings:defaultapps",
    ),
    windows(
        "startup",
        "Aplicativos de inicialização",
        &["inicializacao", "iniciar com o windows"],
        "ms-settings:startupapps",
    ),
    windows(
        "accounts",
        "Suas informações",
        &["conta", "usuario", "perfil"],
        "ms-settings:yourinfo",
    ),
    windows(
        "signin",
        "Opções de entrada",
        &["senha", "pin", "windows hello"],
        "ms-settings:signinoptions",
    ),
    windows(
        "datetime",
        "Data e hora",
        &["relogio", "fuso horario"],
        "ms-settings:dateandtime",
    ),
    windows(
        "language",
        "Idioma e região",
        &["idioma", "regiao", "teclado"],
        "ms-settings:regionlanguage",
    ),
    windows(
        "accessibility",
        "Acessibilidade",
        &["narrador", "lupa", "contraste", "tamanho do texto"],
        "ms-settings:easeofaccess",
    ),
    windows(
        "privacy",
        "Privacidade e segurança",
        &["privacidade", "permissoes", "seguranca"],
        "ms-settings:privacy",
    ),
    windows(
        "update",
        "Windows Update",
        &["atualizacoes", "atualizar"],
        "ms-settings:windowsupdate",
    ),
    windows(
        "about",
        "Sobre o computador",
        &["sistema", "nome do computador", "especificacoes"],
        "ms-settings:about",
    ),
    dockyard(
        "dockmax-general",
        "Configurações do Dockyard",
        &["dockyard", "dockmax", "dock", "lancador", "modo de cor"],
        Section::General,
    ),
    dockyard(
        "dockmax-workspaces",
        "Espaços de trabalho",
        &["espacos", "workspaces", "novo espaco"],
        Section::Workspaces,
    ),
    dockyard(
        "dockmax-look",
        "Aparência do dock",
        &[
            "estilo do dock",
            "estilo da barra",
            "tema do dock",
            "temas",
            "pacote de icones",
        ],
        Section::Look,
    ),
    dockyard(
        "dockmax-apps",
        "Apps do dock",
        &["fixar apps", "apps fixados"],
        Section::Apps,
    ),
    dockyard(
        "dockmax-items",
        "Widgets do dock",
        &[
            "itens do dock",
            "catalogo de widgets",
            "relogio",
            "nota",
            "atividade",
            "rede",
        ],
        Section::Items,
    ),
    dockyard(
        "dockmax-accounts",
        "Contas do Dockyard",
        &["contas google", "google agenda", "conectar conta", "agenda"],
        Section::Accounts,
    ),
    dockyard(
        "dockmax-about",
        "Sobre o Dockyard",
        &["versao do dockyard", "versao do dockmax"],
        Section::About,
    ),
];

/// The page `id` of the list.
pub fn page(id: &str) -> Option<&'static Page> {
    PAGES.iter().find(|page| page.id == id)
}

/// Whether `uri` is one of the Windows pages of the list.
pub fn is_listed_uri(uri: &str) -> bool {
    PAGES
        .iter()
        .any(|page| matches!(page.target, Target::Windows(listed) if listed == uri))
}

/// The key under which the use of the page `id` is kept.
pub fn page_key(id: &str) -> String {
    format!("setting:{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_its_own_id_and_a_windows_settings_address() {
        let mut ids: Vec<&str> = PAGES.iter().map(|page| page.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PAGES.len(), "an id twice");
        for page in PAGES {
            assert!(
                page.id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-'),
                "{}",
                page.id
            );
            if let Target::Windows(uri) = page.target {
                assert!(uri.starts_with("ms-settings:"), "{uri}");
                assert!(!uri.contains(char::is_whitespace), "{uri}");
            }
        }
    }

    #[test]
    fn the_aliases_are_written_as_the_search_reads_them() {
        for page in PAGES {
            for alias in page.aliases {
                assert_eq!(
                    super::super::launcher::normalize(alias),
                    *alias,
                    "{}",
                    page.id
                );
            }
        }
    }

    #[test]
    fn every_section_of_configuracoes_is_listed() {
        for section in [
            Section::General,
            Section::Workspaces,
            Section::Look,
            Section::Apps,
            Section::Items,
            Section::Accounts,
            Section::About,
        ] {
            assert!(
                PAGES
                    .iter()
                    .any(|page| page.target == Target::Dockyard(section)),
                "{}",
                section.as_str()
            );
        }
    }

    #[test]
    fn only_listed_pages_open() {
        assert!(is_listed_uri("ms-settings:network-wifi"));
        assert!(!is_listed_uri("ms-settings:developers"));
        assert!(!is_listed_uri("https://example.com"));
        assert_eq!(page("wifi").map(|page| page.name), Some("Wi-Fi"));
        assert_eq!(page("nope"), None);
        assert_eq!(page_key("wifi"), "setting:wifi");
    }
}
