import type { AppErrorCode } from "../core/apps";
import type { HotkeyErrorCode } from "../core/launcher";
import type { PackErrorCode } from "../core/iconPacks";
import type {
  DockEdge,
  DockOpacity,
  DockStyle,
  HoverEffect,
  IconShape,
  Indicator,
  MiddleClick,
  IndicatorColor,
  KeywordProblem,
  Language,
  OpenAnimation,
  SettingsErrorCode,
  SizePreset,
  TaskbarMode,
  Theme,
} from "../core/settings";
import type {
  ActivityShow,
  ActivityStyle,
  CardPeriod,
  ClockIconStyle,
  CreatableKind,
  HourCycle,
  Period,
  WeekStart,
  WidgetErrorCode,
  WidgetKind,
} from "../core/widgets";
import type { GoogleErrorCode } from "../core/google";
import type { AutosaveState } from "../widgets/notes/autosave";
import type { RemainingUnit } from "../widgets/time-progress/progress";

/** Every user-visible text of the interface, grouped by area. Its shape is
 * the contract each locale must meet (see `Messages`). */
const messages = {
  dock: {
    /** Accessible name of the dock bar. */
    label: "Dock",
    /** Name of each dock item, by panel. */
    items: {
      settings: "Configurações",
      about: "Sobre",
      tray: "Bandeja do sistema",
    },
    /** The clock and the date at the end of the dock. */
    status: {
      /** Accessible name of the button: it opens the Windows calendar. */
      calendar: "Relógio e calendário",
    },
  },
  panel: {
    /** Heading of each panel. */
    titles: {
      settings: "Configurações",
      about: "Sobre",
      tray: "Bandeja do sistema",
    },
    /** Shown in place of a panel that failed to render. */
    failed: "Não foi possível mostrar este painel.",
    /** Key hint at the right of every panel footer: `<kbd>key</kbd> action`. */
    closeHint: { key: "Esc", action: "fecha" },
  },
  settings: {
    /** Label of each option group, by setting. */
    groups: {
      theme: "Tema",
      language: "Idioma",
      edge: "Posição",
      size: "Tamanho",
      opacity: "Opacidade do dock",
      taskbar: "Barra de tarefas do Windows",
      middleClick: "Botão do meio",
    },
    /** What the middle button does on a program of the dock. */
    middleClick: {
      newInstance: "Nova instância",
      none: "Nada",
    } satisfies Record<MiddleClick, string>,
    /** The Configurações app (spec 0011 H1): its sections, in the side
     * bar. */
    app: {
      nav: "Seções das configurações",
      sections: {
        general: "Geral",
        workspaces: "Espaços de trabalho",
        look: "Aparência do dock",
        apps: "Apps do dock",
        items: "Widgets",
        accounts: "Contas",
        about: "Sobre",
      },
      /** The top of the side bar (spec 0014 R1). */
      brand: "Configurações",
      /** The line under each section's title. */
      leads: {
        general: "Como o Dockyard se encaixa no Windows.",
        workspaces: "Um dock para cada coisa que você faz.",
        look: "Deixe o dock com a sua cara.",
        apps: "Os programas fixados no dock de cada espaço.",
        items: "Escolha no catálogo e arrume os do dock de cada espaço.",
        accounts: "As agendas e as tarefas que os widgets mostram.",
        about: "Versão, licença e onde ficam os seus dados.",
      },
      /** The titles of the groups of Geral and Aparência. */
      groups: {
        dock: "Dock",
        taskbar: "Windows",
        behavior: "Comportamento",
        launcher: "Lançador",
        startup: "Inicialização",
        style: "Tema",
        tweaks: "Personalizar este tema",
        hover: "Ao passar o mouse",
        indicator: "Indicador",
        icons: "Ícones",
        openAnimation: "Animação de abertura",
      },
    },
    /** The switch that opens the Dockyard when the user signs in. */
    showOpenApps: "Mostrar apps abertos",
    showOpenAppsHint: "Os que não estão fixados, depois dos widgets",
    showBadges: "Mostrar selos de notificação",
    showBadgesHint: "O número de não lidas que o app põe na barra do Windows",
    autoHide: "Ocultar automaticamente",
    autoHideHint:
      "O dock desce quando o ponteiro sai e sobe ao encostar na borda da tela",
    autoHideReveal: "Atraso para mostrar",
    autoHideRevealHint:
      "Quanto tempo o ponteiro fica na borda antes de o dock subir",
    autoHideDelay: "Atraso para ocultar",
    autoHideDelayHint:
      "Quanto tempo o ponteiro fica fora antes de o dock descer",
    everyMonitor: "Dock em todos os monitores",
    everyMonitorHint:
      "Cada monitor com o seu dock, no lugar da barra de tarefas",
    switchEveryMonitor: "Trocar de espaço em todos os monitores",
    switchEveryMonitorHint: "Trocar num dock troca os outros monitores junto",
    indicatorFollowsWorkspace: "Indicador na cor do espaço",
    indicatorFollowsWorkspaceHint:
      "A marca dos apps abertos usa a cor do espaço mostrado",
    previews: "Prévia ao passar o mouse",
    previewsHint: "Pare o ponteiro sobre um app aberto para ver as janelas",
    previewDelay: "Atraso da prévia",
    previewDelayHint: "Quanto tempo parado antes de a prévia aparecer",
    previewsOnlySeveral: "Só com 2 ou mais janelas",
    previewsOnlySeveralHint: "Um app com uma janela só não mostra a prévia",
    launchAtLogin: "Iniciar com o Windows",
    launchAtLoginHint: "Abre o Dockyard ao entrar no Windows",
    /** The launcher (spec 0012): its switch, the notice when another
     * program holds its hotkey, its history and the hotkey; `{hotkey}` is
     * replaced. */
    launcher: {
      label: "Lançador ({hotkey})",
      hint: "Busca e abre apps de qualquer lugar; também em Buscar apps…, no menu do dock",
      taken: "{hotkey} está em uso por outro programa.",
      takenDetail: "Abra o lançador por Buscar apps…, no menu do dock.",
      history: "Histórico do lançador",
      historyHint: "Os apps que você mais abre por ele aparecem primeiro",
      clear: "Limpar o histórico",
      cleared: "Histórico limpo",
      hotkey: {
        label: "Atalho do lançador",
        hint: "Clique no botão e pressione a nova combinação",
        change: "Alterar o atalho, hoje {hotkey}",
        press: "Pressione a combinação… (Esc cancela)",
        saving: "Salvando…",
        saved: "Atalho salvo: {hotkey}",
        restore: "Voltar para Alt+Espaço",
        space: "Espaço",
        errors: {
          malformed: "Essa combinação não pode ser usada.",
          noModifier: "Use Ctrl, Alt, Shift ou Win junto com a tecla.",
          takesTyping:
            "Essa combinação tiraria a digitação ou um atalho de edição de todos os programas. Use Alt ou Win, ou Ctrl com Shift.",
          reserved: "Essa combinação é do Windows ou do Dockyard.",
          taken: "Essa combinação está em uso por outro programa.",
          notSaved: "Não foi possível salvar o atalho. Tente de novo.",
          unsupportedKey: "Use uma letra, um número, F1 a F24 ou Espaço.",
        } satisfies Record<HotkeyErrorCode, string>,
      },
      keywords: {
        label: "Busca na web",
        hint: "No lançador, digite a palavra-chave, um espaço e o que buscar: g dockyard. A primeira da lista é a busca padrão, usada quando nada mais é encontrado.",
        list: "Palavras-chave da busca na web",
        add: "Adicionar palavra-chave",
        edit: "Editar",
        remove: "Remover",
        makeDefault: "Tornar padrão",
        isDefault: "Padrão",
        save: "Salvar",
        cancel: "Cancelar",
        keyword: "Palavra-chave",
        name: "Nome",
        template: "Endereço (com %s no lugar do termo)",
        templatePlaceholder: "https://exemplo.com/busca?q=%s",
        full: "A lista já tem {max} palavras-chave. Remova uma para adicionar outra.",
        empty: "Nenhuma palavra-chave. O lançador não busca na web.",
        problems: {
          keywordEmpty: "Digite a palavra-chave.",
          keywordSpaces: "A palavra-chave não pode ter espaços.",
          keywordTooLong: "Use até 16 caracteres.",
          keywordRepeated: "Essa palavra-chave já está na lista.",
          nameEmpty: "Digite o nome da busca.",
          nameTooLong: "Use até 40 caracteres.",
          templateNotHttps:
            "O endereço precisa começar com https:// e ter um site.",
          templateNoTerm: "Coloque %s onde o termo buscado entra.",
          templateSpaces: "O endereço não pode ter espaços.",
          templateTooLong: "Use até 500 caracteres.",
        } satisfies Record<KeywordProblem, string>,
      },
    },
    theme: {
      system: "Sistema",
      light: "Claro",
      dark: "Escuro",
    } satisfies Record<Theme, string>,
    /** Each language by its own name, so anyone finds theirs. */
    language: {
      system: "Sistema",
      "pt-BR": "Português",
      en: "English",
    } satisfies Record<Language, string>,
    edge: {
      bottom: "Embaixo",
      top: "Em cima",
    } satisfies Record<DockEdge, string>,
    size: {
      small: "Pequeno",
      medium: "Médio",
      large: "Grande",
    } satisfies Record<SizePreset, string>,
    opacity: {
      translucent: "Translúcido",
      solid: "Sólido",
    } satisfies Record<DockOpacity, string>,
    /** What the dock does with the Windows taskbar (ADR 0009): leaves it,
     * takes the edge and lets it hide, or takes the edge and hides it. */
    taskbar: {
      visible: "Visível",
      autohide: "Automática",
      hidden: "Oculta",
    } satisfies Record<TaskbarMode, string>,
    /** Under each taskbar card of the Configurações app (spec 0014 R2). */
    taskbarHints: {
      visible: "O dock fica logo acima da barra de tarefas.",
      autohide: "Ela aparece ao encostar na borda da tela.",
      hidden: "Só o dock; o relógio e a bandeja vêm para ele.",
    } satisfies Record<TaskbarMode, string>,
    /** Configurações › Aparência do dock (spec 0005 §3.5). */
    look: {
      /** The row that opens it, and its title. */
      title: "Aparência do dock",
      style: "Estilo",
      styles: {
        native: "Nativo",
        liquidGlass: "Liquid Glass",
        midnight: "Meia-noite",
        rocket: "Rocket",
        aquaShelf: "Prateleira Aqua",
        neon: "Neon",
        paper: "Papel",
        floating: "Flutuante",
      } satisfies Record<DockStyle, string>,
      /** Group of the changes that belong to the chosen style. */
      tweaks: "Personalizar este estilo",
      border: "Borda",
      corners: "Cantos",
      cornersHint: "O quanto a barra é arredondada",
      reflections: "Reflexos",
      reflectionsHint: "Uma imagem espelhada embaixo de cada ícone",
      restore: "Restaurar",
      restoreHint: "Descarta as mudanças deste estilo",
      hover: "Efeito ao passar o mouse",
      hoverOptions: {
        none: "Nenhum",
        scale: "Escala",
        magnify: "Ampliação",
      } satisfies Record<HoverEffect, string>,
      hoverScale: "Escala ao passar o mouse",
      indicator: "Indicador",
      indicatorOptions: {
        dot: "Ponto",
        dash: "Traço",
      } satisfies Record<Indicator, string>,
      iconShape: "Formato dos ícones",
      iconShapeOptions: {
        plate: "Placas arredondadas",
        uniform: "Mesmo tamanho",
        original: "Como vieram",
      } satisfies Record<IconShape, string>,
      /** The icon packs (spec 0007). `{name}`, `{covers}`, `{total}`,
       * `{count}` and `{id}` are filled in by the panel. */
      iconPack: "Ícones dos apps",
      iconPackHint:
        "Troque os ícones de todos os apps por um pacote da pasta de pacotes",
      iconPackOriginal: "Original",
      iconPackCovers: "{name} · veste {covers} de {total} apps",
      iconPackCount: "{name} · {count} ícones",
      iconPackMissing: "{id} (indisponível)",
      iconPackLoading: "Lendo os pacotes…",
      iconPackFailed: "Não foi possível ler os pacotes",
      openPacksFolder: "Abrir pasta de pacotes",
      reloadPacks: "Recarregar pacotes",
      packProblem: "Pacote {id}: {reason}",
      packErrors: {
        manifestMissing: "falta o pack.json",
        manifestTooBig: "o pack.json é grande demais",
        manifestInvalid: "o pack.json não é válido",
        noName: "o pacote não tem nome",
        textTooLong: "um texto do pacote é longo demais",
        badAccent: "a cor de destaque não é um #RRGGBB",
        tooManyEntries: "o pacote cita programas demais",
        badKey: "o nome de um programa está vazio ou é longo demais",
        badPath: "o caminho de uma imagem não é permitido",
      } satisfies Record<PackErrorCode, string>,
      openAnimation: "Animação de abertura",
      openAnimationOptions: {
        bounce: "Pulo",
        pulse: "Pulso",
        shake: "Tremor",
      } satisfies Record<OpenAnimation, string>,
      indicatorColor: "Cor do indicador",
      /** The style's own color, before any is chosen. */
      indicatorDefault: "Padrão",
      /** Names of the colors offered, by `#RRGGBB`. */
      colors: {
        "#FFFFFF": "Branco",
        "#E8E8EC": "Cinza muito claro",
        "#C7C7CC": "Cinza claro",
        "#8E8E93": "Cinza",
        "#2C3A5A": "Grafite azulado",
        "#0B1020": "Azul-noite",
        "#000000": "Preto",
        "#0A84FF": "Azul",
        "#64D2FF": "Azul-claro",
        "#00E5FF": "Ciano",
        "#30D158": "Verde",
        "#1E8A3C": "Verde-escuro",
        "#FFD60A": "Amarelo",
        "#FF9F0A": "Laranja",
        "#FF453A": "Vermelho",
        "#FF2D95": "Rosa",
        "#BF5AF2": "Roxo",
        "#9DB4FF": "Lilás",
      } satisfies Record<IndicatorColor, string>,
    },
    /** Footer text: there is no Save button. */
    footer: "Salvo automaticamente",
    /** Shown when a change is refused, by error code; the option goes back
     * to its previous value. A new code without a message does not
     * compile. */
    errors: {
      lookOutOfRange: "Esse ajuste do visual não é aceito.",
      launcherKeywordsInvalid:
        "Essa palavra-chave da busca na web não é aceita.",
      launcherHotkeyInvalid: "Esse atalho do lançador não é aceito.",
      iconSizeOutOfRange:
        "Esse tamanho não é aceito. Escolha Pequeno, Médio ou Grande.",
      unsupportedVersion:
        "Estas configurações são de outra versão. Reinicie o Dockyard e tente de novo.",
      writeFailed: "Não foi possível salvar a mudança. Tente de novo.",
      notReady:
        "As configurações não estão disponíveis agora. Reinicie o Dockyard e tente de novo.",
    } satisfies Record<SettingsErrorCode, string>,
  },
  /** The notification area in its panel (ADR 0009, Fase E). */
  tray: {
    /** Shown while no program has put an icon there. */
    empty: "Nenhum programa colocou um ícone aqui ainda.",
    /** Under the empty text: why a running program may be missing. */
    emptyHint:
      "Alguns só aparecem depois de abertos de novo, depois que o Dockyard assumiu a bandeja.",
    /** The icon of a program without a tooltip. */
    unnamed: "Programa sem nome",
    /** Footer: the buttons do what the taskbar's did. */
    footer: "Botão direito abre o menu do programa",
    /** Configurações › Geral: which tray icons the dock shows itself, at its
     * end beside the battery, the network and the volume (as the
     * PrimoDock). */
    inDock: {
      label: "Ícones da bandeja no dock",
      hint: "Ao lado de bateria, rede e volume; os outros ficam na seta. Até {max}.",
      panel: "Painel",
      dock: "Dock",
      /** While the dock holds no icon to choose. */
      none: "Nenhum ícone na bandeja agora.",
    },
  },
  about: {
    /** One line under the app name. */
    description: "Dock para Windows 11, feito com Tauri.",
    /** Labels of the key/value list; the values come from the app. */
    labels: {
      version: "Versão",
      license: "Licença",
      code: "Código",
    },
    /** The license, by name: the text is in the LICENSE file. */
    licenseValue: "Uso pessoal",
    /** The source code repository, shown as selectable text (a link from Fase 3). */
    repository: "github.com/maxeinstein-dev/Dockyard",
    footer: "Projeto independente",
    /** Opens the welcome tour again (spec 0019 G2). */
    tour: "Ver o tour",
  },
  /** The welcome tour (spec 0019 G2). `{hotkey}` is the launcher's. */
  tour: {
    count: "{n} de {total}",
    skip: "Pular",
    back: "Voltar",
    next: "Próximo",
    start: "Começar",
    openSettings: "Abrir Configurações",
    steps: {
      welcome: {
        title: "Boas-vindas ao Dockyard",
        body: "Um dock para o seu Windows: os seus apps, espaços de trabalho e widgets vivos num só lugar. Sem conta e sem rastreamento.",
      },
      apps: {
        title: "Seus apps no dock",
        body: "Clique para abrir um app ou trazer a janela dele; passe o mouse para ver as janelas abertas.\nArraste para mudar a ordem e use o clique direito para fixar, fechar e mais.",
      },
      workspaces: {
        title: "Espaços de trabalho",
        body: "As bolinhas no começo do dock são os seus espaços. Cada um tem os seus apps, widgets e janelas.\nTroque com um clique, com a roda do mouse sobre o dock ou com Ctrl+Alt+número.",
      },
      widgets: {
        title: "Widgets vivos",
        body: "Relógio, clima, agenda, Pomodoro e outros mostram o que importa no próprio ícone. Clique para abrir o painel; os ajustes ficam lá dentro.\nAdicione mais em Configurações › Widgets.",
      },
      launcher: {
        title: "Ache tudo com {hotkey}",
        body: "O lançador acha apps e páginas das Configurações, faz contas e busca na web por palavra-chave, como “g dockyard”.",
      },
      done: {
        title: "Tudo pronto",
        body: "Abra as Configurações pelo clique direito no dock para mudar tema, posição, tamanho e muito mais.\nPara rever este tour, vá em Configurações › Sobre.",
      },
    },
  },
  /** Texts of the widgets and of Configurações › Itens do dock (spec 0003
   * §2.2, §2.3; the approved mock `docs/design/widgets-mock.html`). A text
   * with `{name}` placeholders goes through `format`; a `{ one, other }`
   * pair through `plural`. */
  /** Configurações › Apps do dock (spec 0006). */
  workspaces: {
    title: "Espaços de trabalho",
    lead: "Cada espaço tem o seu dock e guarda as suas janelas. Troque pelas bolinhas no dock, pela roda do mouse sobre ele ou com Ctrl+Alt+número. Editar aqui não muda o que o dock mostra.",
    loading: "Lendo os espaços…",
    listLabel: "Espaços",
    editing: "Espaço em edição",
    shortcut: "Ctrl+Alt+{n}",
    create: "+ Novo espaço",
    name: "Nome",
    color: "Cor",
    colors: {
      "#0A84FF": "Azul",
      "#FF9F0A": "Laranja",
      "#30D158": "Verde",
      "#BF5AF2": "Roxo",
      "#FF375F": "Rosa",
      "#FF453A": "Vermelho",
      "#FFD60A": "Amarelo",
      "#8E8E93": "Cinza",
    },
    counts: "{apps} apps e {widgets} widgets",
    editApps: "Editar apps",
    editItems: "Editar widgets",
    moveUp: "Subir {name}",
    moveDown: "Descer {name}",
    remove: "Excluir {name}",
    confirmTitle: "Excluir {name}?",
    confirmDetail:
      "Os apps e widgets que só estão nele saem do dock, e o texto das notas vai junto.",
    cancel: "Cancelar",
    confirm: "Excluir espaço",
    errors: {
      notFound: "Esse espaço não existe mais.",
      limitReached: "O limite é de 9 espaços.",
      invalidName: "Use de 1 a 32 caracteres, sem espaços nas pontas.",
      invalidColor: "Escolha uma das cores.",
      lastWorkspace: "O último espaço não pode ser excluído.",
      unavailable: "Os dados do Dockyard não estão disponíveis agora.",
      writeFailed: "Não deu para salvar. Tente de novo.",
      notReady: "Não deu para concluir. Tente de novo.",
    },
  },
  apps: {
    /** The navigation row in Configurações, and the subview's title. */
    title: "Apps do dock",
    /** The row's value and the count: `3 de 40`. */
    count: "{count} de {max}",
    /** The row's value while the apps can't be used. */
    unavailable: "Indisponível",
    /** Accessible name of the list of pinned apps. */
    listLabel: "Apps na ordem do dock",
    empty: "Nenhum app no dock. Procure um abaixo.",
    add: "Adicionar um app",
    /** In place of the count at the limit; pinning turns off. */
    limit: "Limite de {max} apps",
    searchLabel: "Buscar app instalado",
    searchPlaceholder: "Buscar por nome",
    catalogLabel: "Apps instalados",
    loading: "Lendo os apps instalados…",
    noMatch: "Nenhum app com esse nome.",
    catalogFailed: "Não foi possível ler os apps instalados.",
    retry: "Tentar de novo",
    /** Accessible names of the buttons of each program, with its name. */
    pin: "Fixar {name}",
    pinned: "{name} (fixado)",
    moveUp: "Subir {name}",
    moveDown: "Descer {name}",
    remove: "Remover {name} do dock",
    footer: "A ordem é a do dock",
    footerUnavailable: "Configurações e Sobre continuam funcionando",
    /** The menu of a pinned program (right click on its icon). */
    menu: {
      /** Accessible name of the menu, with the program's name. */
      label: "Menu de {name}",
      /** Heading of each kind of section; a category has its own name. */
      sections: {
        tasks: "Tarefas",
        category: "Recentes",
        recent: "Recentes",
        frequent: "Frequentes",
      },
      loading: "Lendo o menu…",
      failed: "Não foi possível ler o menu deste app.",
      open: "Abrir",
      reveal: "Mostrar no Explorador de Arquivos",
      remove: "Remover do dock",
      /** Spec 0009 D3. */
      newInstance: "Nova instância",
      closeWindow: "Fechar janela",
      closeWindows: "Fechar todas as janelas",
      pin: "Fixar no dock",
      /** Spec 0013 C3. */
      moveTo: "Mover para {name}",
      footer: "O menu da barra de tarefas",
      openFooter: "Um app aberto, que não está fixado",
    },
    /** Notice of a database that can't be used in this run. */
    unavailableNotice: {
      title: "Os apps do dock estão indisponíveis.",
      detail:
        "O banco de dados não pôde ser aberto e não foi alterado. O motivo está no log do Dockyard.",
    },
    /** Shown when an apps change fails, by error code. A new code without
     * a message does not compile. */
    errors: {
      limitReached: "O dock já tem 40 apps. Remova um para fixar outro.",
      notFound: "Este app não está mais no dock.",
      alreadyPinned: "Este app já está no dock.",
      invalidApp: "Este app não está mais instalado ou não pode ser fixado.",
      launchFailed: "Não foi possível abrir o app.",
      unavailable:
        "Os apps estão indisponíveis agora. O motivo está no log do Dockyard.",
      writeFailed: "Não foi possível salvar a mudança. Tente de novo.",
      notReady:
        "Os apps não estão disponíveis agora. Reinicie o Dockyard e tente de novo.",
    } satisfies Record<AppErrorCode, string>,
  },
  /** Configurações › Contas (spec 0017 F2a, ADR 0012). */
  accounts: {
    client: {
      title: "App do Google",
      /** Before a client is set: what to do, in short. */
      intro:
        'Para ler as suas agendas, o Dockyard usa um app que você cria no Google Cloud, só para você. Crie um cliente do tipo "App para computador" e cole aqui o ID e o segredo.',
      id: "ID do cliente",
      secret: "Segredo do cliente",
      save: "Salvar",
      /** With a client set: its id. */
      current: "Cliente: {id}",
      change: "Trocar",
      cancel: "Cancelar",
      /** The secret stays in the Windows vault. */
      hint: "O segredo fica no cofre de credenciais do Windows e não aparece de novo.",
    },
    list: {
      title: "Contas Google",
      empty: "Nenhuma conta conectada.",
      connect: "Conectar conta Google",
      /** While the browser is open on Google's page. */
      waiting: "Termine o login no navegador…",
      /** Ends the wait, for a browser closed halfway. */
      cancel: "Cancelar login",
      /** Before a client is set. */
      needsClient: "Salve o app do Google acima para conectar uma conta.",
      calendars: "Agendas",
      showCalendars: "Ver agendas de {email}",
      hideCalendars: "Ocultar agendas de {email}",
      loading: "Lendo as agendas…",
      noCalendars: "Nenhuma agenda nesta conta.",
      primary: "principal",
      disconnect: "Desconectar",
      disconnectNamed: "Desconectar {email}",
      confirmDisconnect: {
        title: "Desconectar esta conta?",
        detail: "As agendas dela somem dos widgets. Dá para conectar de novo.",
        cancel: "Cancelar",
        confirm: "Desconectar",
      },
      /** Said once an account was connected. */
      connected: "{email} foi conectada.",
    },
    errors: {
      noClient: "Salve o app do Google antes de conectar uma conta.",
      invalidClientId:
        "Esse ID não parece de um cliente Google. Ele termina em .apps.googleusercontent.com.",
      invalidClientSecret: "Cole o segredo do cliente, numa linha só.",
      refused: "O login foi cancelado no Google.",
      timedOut: "O login demorou mais de 5 minutos. Tente de novo.",
      cancelled: "O login foi cancelado.",
      offline: "Sem conexão com o Google agora.",
      signInAgain:
        "O Google não aceita mais esta conta. Desconecte e conecte de novo.",
      notConnected: "Esta conta não está mais conectada.",
      unexpected: "O Google respondeu algo inesperado. O motivo está no log.",
      vault: "Não foi possível guardar no cofre de credenciais do Windows.",
      notAllowed: "Esta janela não pode mexer nas contas.",
      notReady: "As contas não estão disponíveis agora. Reinicie o Dockyard.",
    } satisfies Record<GoogleErrorCode, string>,
  },
  widgets: {
    /** Name of each kind: the dock item's name and the panel title when the
     * instance has no label, and the label field's placeholder. */
    kinds: {
      clock: "Relógio",
      "time-progress": "Progresso do tempo",
      note: "Nota",
      shortcut: "Atalho",
      files: "Arquivos",
      activity: "Atividade",
      network: "Atividade de rede",
      calendar: "Calendário",
      cycle: "Ciclo",
      countdown: "Contagem regressiva",
      pomodoro: "Pomodoro",
      todo: "Lista de tarefas",
      water: "Beber água",
      media: "Tocando agora",
      claude: "Uso do Claude",
      weather: "Clima",
    } satisfies Record<WidgetKind, string>,
    /** Short name of each kind on the add buttons of Itens do dock. */
    addNames: {
      clock: "Relógio",
      "time-progress": "Progresso",
      note: "Nota",
      activity: "Atividade",
      network: "Rede",
      calendar: "Calendário",
      cycle: "Ciclo",
      countdown: "Contagem regressiva",
      pomodoro: "Pomodoro",
      todo: "Lista de tarefas",
      water: "Beber água",
      media: "Tocando agora",
      claude: "Uso do Claude",
      weather: "Clima",
    } satisfies Record<CreatableKind, string>,
    /** Parts every widget panel shares. */
    common: {
      /** Quiet button of the main view: opens the instance's settings. */
      adjust: "Ajustes",
      /** Accessible name of the icon button that goes back from a subview. */
      back: "Voltar",
      label: "Rótulo",
      /** Removes the instance; at the end of every widget's settings. */
      remove: "Remover do dock",
      /** Footer of the settings views: there is no Save button. */
      footer: "Salvo automaticamente",
      /** Accessible names of the move buttons, with the item's name. */
      moveUp: "Subir {name}",
      moveDown: "Descer {name}",
      /** Accessible name of the remove button of each item in Itens do
       * dock, with the item's name. */
      removeNamed: "Remover {name}",
      /** Subtitle in Itens do dock of an instance with a label:
       * `{kind} · {detail}`, as `Relógio · Lisboa`. */
      labelledSubtitle: "{kind} · {detail}",
    },
    clock: {
      settingsTitle: "Ajustes do relógio",
      timeZone: "Fuso horário",
      /** The first option of the zone list: follow the system zone. */
      systemZoneOption: "Do sistema ({zone})",
      /** The same option when `Intl` can't name the system zone. */
      systemZoneOptionUnknown: "Do sistema",
      /** Any other option: the zone's offset now, a city name and the IANA
       * name, as `UTC−3 · Sao Paulo · America/Sao_Paulo`. */
      zoneOption: "{offset} · {city} · {zone}",
      /** A configured zone this WebView doesn't know: no offset to show. */
      zoneOptionUnknown: "{city} · {zone}",
      hourCycle: "Formato",
      hourCycles: {
        h23: "24 horas",
        h12: "12 horas",
      } satisfies Record<HourCycle, string>,
      seconds: "Segundos no painel",
      /** Under the seconds group: the dock icon never shows seconds (spec
       * 0003 Q2), which the manual test of 2026-10-20 showed is not obvious. */
      secondsHint: "O ícone do dock mostra só horas e minutos.",
      secondsOptions: {
        hide: "Ocultar",
        show: "Mostrar",
      },
      iconStyle: "Ícone no dock",
      iconStyles: {
        digital: "Digital",
        analog: "Analógico",
      } satisfies Record<ClockIconStyle, string>,
      /** Footer of the main view: the zone and its offset, as
       * `São Paulo · UTC−3`. */
      footer: "{zone} · {offset}",
      /** The zone part of the footer for a clock that follows the system. */
      systemZone: "Fuso do sistema: {zone}",
      /** The footer when not even the system zone was identified. */
      systemZoneUnknown: "Fuso do sistema não identificado",
      /** Notice when the WebView doesn't know the configured zone: the
       * system time is shown instead, never an invented offset. */
      zoneFallback:
        "Este fuso não é conhecido aqui; mostrando a hora do sistema.",
      /** Subtitle in Itens do dock of a clock that follows the system. */
      systemZoneSubtitle: "Fuso do sistema",
    },
    progress: {
      settingsTitle: "Ajustes do progresso",
      periods: {
        day: "Dia",
        week: "Semana",
        month: "Mês",
        year: "Ano",
        workday: "Expediente",
      } satisfies Record<Period, string>,
      periodsLabel: "Períodos, na ordem do painel",
      /** Next to the workday period: which days it counts. */
      workdayDays: "seg. a sex.",
      /** The workday hours on the period's line: `das [09:00] às [18:00]`. */
      workdayFrom: "das",
      workdayTo: "às",
      workdayStart: "Início do expediente",
      workdayEnd: "Fim do expediente",
      ringHint: "O primeiro período é o do anel no ícone.",
      /** Under the width: what each width shows (spec 0017 F1b). */
      widthHint:
        "Com 2 ícones, os dois primeiros períodos em anéis; com 3, os três primeiros em barras.",
      weekStart: "Semana começa",
      weekStarts: {
        monday: "Segunda",
        sunday: "Domingo",
      } satisfies Record<WeekStart, string>,
      /** Footer of the main view. */
      footer: {
        monday: "Semana começa na segunda",
        sunday: "Semana começa no domingo",
      } satisfies Record<WeekStart, string>,
      /** A workday period outside working hours, in place of its bar. */
      offWorkday: "Fora do expediente",
      /** Time left in a period: `faltam {time}`, the time from `units`
       * joined by a space, as `faltam 2 dias 3 h`. */
      remaining: "faltam {time}",
      units: {
        day: { one: "{n} dia", other: "{n} dias" },
        hour: { one: "{n} h", other: "{n} h" },
        minute: { one: "{n} min", other: "{n} min" },
      } satisfies Record<RemainingUnit, { one: string; other: string }>,
      /** The icon's center without a running period. */
      noValue: "–",
    },
    note: {
      settingsTitle: "Ajustes da nota",
      placeholder: "Escreva aqui. Salva sozinho.",
      /** Footer of the main view, by save state; a failed save shows the
       * message of its error code instead of `error`. */
      saveStates: {
        idle: "Salvo",
        pending: "Salvando…",
        saving: "Salvando…",
        error: "Não foi salvo.",
      } satisfies Record<AutosaveState, string>,
      /** The note's text can't be read (not valid UTF-8): the editor is
       * locked so a save never replaces the stored text (spec §2.10). */
      unreadable:
        "Não foi possível ler esta nota. Para não perder o texto guardado, ela não pode ser editada.",
      /** Inline confirmation of removing a note with text. */
      confirmRemove: {
        title: "Remover a nota apaga o texto.",
        detail: "Não dá para desfazer.",
        cancel: "Cancelar",
        confirm: "Remover nota",
      },
      /** Subtitle in Itens do dock: how many lines, or empty. */
      lines: { one: "{n} linha", other: "{n} linhas" },
      empty: "vazia",
    },
    /** The system widgets (spec 0017 F1): what they share. */
    system: {
      /** The tile's width in icons. */
      width: "Largura no dock",
      widths: { one: "1 ícone", two: "2 ícones", three: "3 ícones" },
      /** While the first reading is not in. */
      reading: "Lendo…",
      /** A share: `42%`. */
      percent: "{n}%",
    },
    activity: {
      settingsTitle: "Ajustes da atividade",
      cpu: "Processador",
      memory: "Memória",
      /** The memory in use: `9,1 de 16 GB`. */
      memoryUse: "{used} de {total}",
      /** The last minute of the processor, under its bar. */
      cpuHistory: "Processador no último minuto",
      show: "Mostrar",
      shows: {
        both: "Os dois",
        cpu: "Processador",
        memory: "Memória",
      } satisfies Record<ActivityShow, string>,
      style: "Estilo no dock",
      styles: {
        bars: "Barras",
        rings: "Anéis",
        numbers: "Números",
      } satisfies Record<ActivityStyle, string>,
      widthHint:
        "Com 3 ícones, o dock mostra também o processador no último minuto.",
      footer: "Atualiza a cada 2 segundos",
    },
    network: {
      settingsTitle: "Ajustes da rede",
      down: "Download",
      up: "Upload",
      /** The graph of the last minute. */
      history: "Rede no último minuto",
      widthHint:
        "Com 2 ícones, o dock mostra o gráfico do último minuto; com 3, maior.",
      footer: "Placas de rede ligadas · a cada 2 segundos",
    },
    /** Configurações › Widgets (spec 0017): the catalog of every kind. */
    catalog: {
      title: "Catálogo",
      /** The list of the widgets in the dock, above the catalog. */
      inUse: "Em uso",
      search: "Buscar widgets",
      /** No kind matches the search. */
      none: "Nenhum widget com esse nome.",
      categories: {
        system: "Sistema",
        time: "Tempo",
        productivity: "Produtividade",
        media: "Mídia",
        personal: "Pessoal",
      },
      descriptions: {
        clock: "A hora no ícone, digital ou de ponteiros, em qualquer fuso.",
        "time-progress": "Quanto já passou do dia, da semana, do mês e do ano.",
        note: "Um bloco de notas a um clique, que salva sozinho.",
        activity: "Processador e memória em barras, anéis ou números.",
        network: "Download e upload agora, com o gráfico do último minuto.",
        calendar:
          "As próximas reuniões das suas agendas Google, com Entrar a um clique.",
        cycle: "Acompanha um ciclo marcado na agenda, com discrição no dock.",
        todo: "Tarefas com marca de feita; o dock mostra quantas faltam.",
        water: "Os copos do dia contra uma meta, com lembrete opcional.",
        claude:
          "Os limites do seu plano do Claude: as 5 horas e a semana, com quando reiniciam.",
        weather:
          "O tempo agora na sua cidade e os próximos dias, com máxima, mínima e chuva.",
        media:
          "O que toca no Windows, com capa e os botões de tocar, voltar e avançar.",
        pomodoro:
          "Foco e pausa com o tempo no dock; continua contando com o painel fechado.",
        countdown:
          "Os dias que faltam para uma data, como um casamento ou as férias.",
      } satisfies Record<CreatableKind, string>,
      /** The add button of a card, and its accessible name. */
      add: "Adicionar",
      addNamed: "Adicionar {name}",
      /** Said once a widget went to the dock. */
      added: "{name} foi para o dock.",
    },
    /** The Calendário (spec 0017 F2a). */
    calendar: {
      settingsTitle: "Ajustes do calendário",
      today: "Hoje",
      tomorrow: "Amanhã",
      allDay: "Dia inteiro",
      free: "Nada marcado.",
      /** While the first reading is not in. */
      loading: "Lendo as agendas…",
      /** A meeting under way. */
      now: "agora",
      /** `09:00–09:30`. */
      range: "{start}–{end}",
      join: "Entrar",
      joinNamed: "Entrar em {title}",
      openInGoogle: "Abrir no Google Agenda",
      /** The event's subview. */
      when: "Quando",
      where: "Local",
      calendar: "Agenda",
      noTitle: "(sem título)",
      /** An event of a calendar shared as free/busy only. */
      busy: "Ocupado",
      status: {
        noAccounts:
          "Nenhuma conta Google conectada. Conecte em Configurações › Contas.",
        offline: "Sem conexão: mostrando a última leitura.",
        signInAgain:
          "Uma conta precisa ser conectada de novo em Configurações › Contas.",
      },
      calendars: "Agendas mostradas",
      calendarsHint:
        "Uma agenda nova aparece sozinha; desmarque para esconder.",
      noCalendars: "As agendas aparecem aqui depois da primeira leitura.",
      footer: "Atualiza a cada minuto",
      /** Google Tasks (F2a-3). */
      tasks: "Tarefas",
      late: "atrasada",
      noTasks: "Nenhuma tarefa para hoje.",
      completeNamed: "Concluir {title}",
      moveUp: "Subir {title}",
      moveDown: "Descer {title}",
      completeFailed: "Não foi possível concluir a tarefa no Google.",
      /** The card over the dock (F2a-4). */
      cardContent: "Ao passar o mouse",
      cardContents: { both: "Reuniões e tarefas", meetings: "Só reuniões" },
      cardPeriod: "Período do cartão",
      cardPeriods: {
        auto: "Auto",
        day: "Dia",
        twoDays: "2 dias",
        threeDays: "3 dias",
        week: "Semana",
      } satisfies Record<CardPeriod, string>,
      cardHint:
        "Auto mostra o dia; depois das 18h, as de amanhã até o meio-dia. O cartão rola com a roda do mouse.",
      showTasks: "Tarefas do Google",
      showTasksOptions: { show: "Mostrar", hide: "Ocultar" },
    },
    /** The Ciclo (spec 0017 F2b): the panel says what the tile doesn't. */
    /** The Clima (spec 0017 F5). */
    weather: {
      settingsTitle: "Ajustes do Clima",
      loading: "Lendo o tempo…",
      unavailable:
        "Não foi possível ler o tempo agora. Tentando de novo em alguns minutos.",
      noCity: "Escolha a cidade nos Ajustes.",
      now: "Agora",
      days: "Próximos dias",
      today: "Hoje",
      feelsLike: "Sensação {value}",
      humidity: "Umidade {value}%",
      wind: "Vento {value} km/h",
      range: "{max} / {min}",
      conditions: {
        clear: "Céu limpo",
        partlyCloudy: "Parcialmente nublado",
        cloudy: "Nublado",
        fog: "Neblina",
        drizzle: "Garoa",
        rain: "Chuva",
        snow: "Neve",
        storm: "Tempestade",
      },
      footer: "Open-Meteo · lido a cada 30 min",
      cityNow: "Cidade: {city}",
      search: "Buscar cidade",
      searchPlaceholder: "Ex.: Belo Horizonte",
      searchButton: "Buscar",
      searching: "Buscando…",
      searchFailed: "Não foi possível buscar agora. Tente de novo.",
      nothingFound: "Nenhuma cidade com esse nome.",
      found: "Cidades encontradas",
      widthHint:
        "Com 1 ícone, o céu e a temperatura; com 2, a máxima e a mínima do dia ao lado.",
      privacy:
        "Só o nome buscado e as coordenadas da cidade vão para o Open-Meteo, sem conta nem localização automática.",
    },
    /** The Uso do Claude (spec 0017 F5). */
    claude: {
      settingsTitle: "Ajustes do Uso do Claude",
      loading: "Lendo o uso…",
      fiveHour: "Sessão (5 horas)",
      week: "Semana",
      used: "{n}% usado",
      resets: "Reinicia {when}",
      resetWords: {
        within: "em {time}",
        hours: "{n} h",
        minutes: "{n} min",
        soon: "em instantes",
      },
      states: {
        signedOut: "O Claude Code não está conectado neste computador.",
        expired: "O acesso do Claude Code expirou e não deu para renovar.",
        unavailable:
          "Não foi possível ler o uso agora. Tentando de novo em alguns minutos.",
      },
      footer: "Como o /usage do Claude Code · lido a cada 5 min",
      signIn: "Entrar no Claude Code",
      signInOpened:
        "Conclua o login na janela do terminal. O uso aparece aqui em seguida.",
      signInFailed: "Não foi possível abrir o terminal do Claude Code.",
      widthHint:
        "Com 1 ícone, o anel das 5 horas; com 2, as 5 horas e a semana.",
      privacy:
        "O Dockyard usa o acesso que o Claude Code já guarda neste computador, só para perguntar o uso à Anthropic. Não copia nem guarda nada.",
    },
    /** The Tocando agora (spec 0017 F4). */
    media: {
      settingsTitle: "Ajustes do Tocando agora",
      loading: "Procurando o que toca…",
      nothing: "Nada tocando agora.",
      untitled: "Sem título",
      previous: "Anterior",
      play: "Tocar",
      pause: "Pausar",
      next: "Próxima",
      from: "Tocando em",
      footer: "O que o Windows mostra nos controles de mídia",
      widthHint:
        "Com 1 ícone, só a capa; com 2, a capa com o título e o artista; com 3, também os botões de voltar, tocar ou pausar e avançar. O clique do meio toca ou pausa.",
    },
    /** The Beber água (spec 0017 F3-2). */
    water: {
      settingsTitle: "Ajustes da água",
      cups: "{n} de {goal} copos",
      ml: "{n} de {goal} ml hoje",
      progress: "Copos do dia",
      add: "+ 1 copo ({cup} ml)",
      undo: "Desfazer",
      reached: "Meta do dia batida.",
      footer: "Zera à meia-noite · salvo no Dockyard",
      reminding: "Lembrete depois de {n} min sem um copo, das 8h às 22h",
      goal: "Meta do dia",
      goalCups: "{n} copos",
      cup: "Tamanho do copo",
      cupMl: "{n} ml",
      remind: "Lembrete",
      remindOff: "Desligado",
      remindEvery: "Depois de {n} min sem um copo",
      remindHint:
        "Das 8h às 22h, o Beber água balança no dock e ganha um selo; o selo some ao abrir o painel.",
    },
    /** The Lista de tarefas (spec 0017 F3). */
    todo: {
      settingsTitle: "Ajustes da lista",
      new: "Nova tarefa",
      placeholder: "Nova tarefa",
      full: "A lista chegou a 200 tarefas",
      add: "Adicionar",
      empty: "Nenhuma tarefa ainda.",
      allDone: "Tudo feito.",
      openTitle: "Abertas",
      doneTitle: { one: "{n} feita", other: "{n} feitas" },
      remove: "Remover a tarefa {text}",
      clearDone: "Limpar as feitas",
      footer: {
        one: "{n} aberta · salvo no Dockyard",
        other: "{n} abertas · salvo no Dockyard",
      },
    },
    /** The Pomodoro (spec 0017 F2c-2). */
    pomodoro: {
      settingsTitle: "Ajustes do Pomodoro",
      loading: "Carregando…",
      stages: { focus: "Foco", short: "Pausa", long: "Pausa longa" },
      start: "Iniciar",
      pause: "Pausar",
      resume: "Continuar",
      skip: "Pular",
      reset: "Reiniciar",
      round: "{done} de {every} focos até a pausa longa",
      footer: "Continua contando com o painel fechado",
      focus: "Foco",
      short: "Pausa",
      long: "Pausa longa",
      every: "Pausa longa a cada",
      minutes: "{n} min",
      focuses: "{n} focos",
      autoStart: "Começar a próxima etapa sozinho",
      autoStartHint: "Sem isso, a etapa seguinte espera o Iniciar.",
      windows: "Também no Windows",
      windowsHint:
        "No fim de cada etapa o Pomodoro balança no dock e ganha um selo; isto soma uma notificação do Windows, que precisa estar ligada.",
    },
    /** The Contagem regressiva (spec 0017 F2c). */
    countdown: {
      settingsTitle: "Ajustes da contagem",
      name: "Nome",
      namePlaceholder: "Casamento",
      date: "Data",
      yearly: "Repetir todo ano",
      yearlyHint: "Para aniversários: conta sempre até o próximo.",
      noDate: "Escolha a data nos Ajustes.",
      today: "É hoje!",
      left: { one: "Falta {n} dia", other: "Faltam {n} dias" },
      ago: { one: "Foi há {n} dia", other: "Foi há {n} dias" },
      everyYear: "Todo ano",
    },
    cycle: {
      settingsTitle: "Ajustes do ciclo",
      unknown: "Nenhuma marca encontrada ainda.",
      period: "Menstruação · dia {day}",
      sensitive: {
        one: "TPM · a menstruação deve vir em {n} dia",
        other: "TPM · a menstruação deve vir em {n} dias",
      },
      between: {
        one: "Próxima menstruação em {n} dia",
        other: "Próxima menstruação em {n} dias",
      },
      due: "A próxima está prevista para estes dias.",
      next: "Próxima prevista",
      last: "Última",
      cycleLength: "Ciclo médio",
      periodLength: "Duração média",
      days: "{n} dias",
      defaults:
        "Com menos de duas marcas, a previsão usa 28 dias de ciclo e 5 de duração. Cada marca é um evento “{symbol}” de dia inteiro no primeiro dia.",
      basedOn: {
        one: "Com base no último ciclo.",
        other: "Com base nos últimos {n} ciclos.",
      },
      footer: "Lido dos eventos “{symbol}”, sem guardar",
      noAccounts: "Conecte a conta Google em Configurações › Contas.",
      loading: "Lendo as marcas…",
      symbol: "Símbolo das marcas",
      calendar: "Agenda das marcas",
      allCalendars: "Todas as agendas",
      window: "TPM: dias antes",
      windowHint: "Quantos dias antes da menstruação prevista a TPM começa.",
      today: "Hoje",
      notices: {
        sensitive: "A TPM deve começar hoje.",
        eve: "A menstruação deve vir amanhã.",
        day: "A menstruação deve vir hoje.",
      },
      noticesTitle: "Avisos",
      noticeToggles: {
        sensitive: "Quando a TPM começar",
        eve: "Na véspera da menstruação",
        day: "No dia da menstruação",
        windows: "Também no Windows",
      },
      noticesHint:
        "A partir das 9h o Ciclo ganha um selo no dock e dá um balanço. O que é aparece só no painel, e o selo some ao abri-lo.",
      windowsHint:
        "Uma notificação discreta “Lembrete do Ciclo”, sem detalhes. Precisa das notificações do Windows ligadas.",
      testNotice: "Enviar aviso de teste",
      testResult: {
        sent: "Pronto: o selo fica no Ciclo até você abrir o painel de novo.",
        off: "As notificações do Windows estão desligadas. Ative em Configurações › Sistema › Notificações.",
        failed: "O Windows não mostrou o aviso.",
      },
      remindersTitle: "Lembretes",
      noReminders: "Nenhum lembrete ainda.",
      remindersHint:
        "Caem no dia escolhido junto com os avisos, e o texto aparece só no painel.",
      reminderText: "Novo lembrete",
      reminderPlaceholder: "Comprar chocolate",
      reminderDaysLabel: "Quando",
      reminderAnchorLabel: "Contando de",
      anchors: { sensitive: "da TPM", period: "da menstruação" },
      reminderDays: {
        same: "No dia",
        before: { one: "{n} dia antes", other: "{n} dias antes" },
        after: { one: "{n} dia depois", other: "{n} dias depois" },
      },
      reminderWhen: "{days} {anchor}",
      addReminder: "Adicionar lembrete",
      removeReminder: "Remover o lembrete {text}",
    },
    items: {
      /** The navigation row in Configurações, and the subview's title. */
      title: "Itens do dock",
      /** The row's value and the add count: `3 de 12`. */
      count: "{count} de {max}",
      /** The row's value while the widgets can't be used. */
      unavailable: "Indisponível",
      /** Accessible name of the list. */
      listLabel: "Widgets na ordem do dock",
      empty: "Nenhum widget no dock. Adicione um abaixo.",
      add: "Adicionar",
      /** The widgets of the other workspaces, to show in this one too
       * (spec 0013 C3): the same widget, a note with the same text. */
      fromOthers: "De outros espaços",
      /** In place of the count at the limit; the add buttons turn off. */
      limit: "Limite de {max} widgets",
      footer: "A ordem é a do dock",
      footerUnavailable: "Configurações e Sobre continuam funcionando",
      /** Notice of a database set aside and recreated, until dismissed. */
      recovered: {
        title: "O banco de dados estava danificado e foi recriado.",
        detail:
          "As notas antigas ficaram guardadas na pasta de dados do Dockyard.",
        dismiss: "Entendi",
      },
      /** Notice of a database that can't be used in this run. */
      unavailableNotice: {
        title: "Os widgets estão indisponíveis.",
        detail:
          "O banco de dados não pôde ser aberto e não foi alterado: suas notas continuam no arquivo. O motivo está no log do Dockyard.",
      },
    },
    /** Accessible name of the Settings item while it has the warning
     * badge: `Configurações (com aviso)`. */
    withNotice: "{name} (com aviso)",
    /** Said by screen readers after a row with the warning dot. */
    notice: "com aviso",
    /** Shown when a widget change fails, by error code; the value goes
     * back to the previous one. A new code without a message does not
     * compile. */
    errors: {
      limitReached: "O dock já tem 12 widgets. Remova um para adicionar outro.",
      notCreatable: "Este tipo de widget ainda não pode ser adicionado.",
      notFound: "Este widget não está mais no dock.",
      kindMismatch:
        "Este ajuste não vale para este widget. Feche o painel e tente de novo.",
      invalidLabel: "Use de 1 a 40 caracteres, com ao menos um visível.",
      invalidTimeZone: "Esse fuso horário não é conhecido. Escolha outro.",
      invalidPeriods: "Escolha ao menos um período, sem repetir.",
      invalidWorkday:
        "O expediente precisa terminar depois de começar, no mesmo dia.",
      invalidTarget:
        "Esse destino não é aceito. Use um caminho do Windows ou um endereço http(s).",
      invalidFolder:
        "Essa pasta não é aceita. Use o caminho completo de uma pasta do Windows.",
      invalidWidth: "Essa largura não vale para este widget.",
      invalidCalendars: "Não foi possível guardar os ajustes do calendário.",
      invalidCountdown: "Escolha uma data válida.",
      invalidPomodoro: "Não foi possível guardar os tempos do Pomodoro.",
      invalidTasks: "Não foi possível guardar a lista.",
      invalidWater: "Não foi possível guardar os copos.",
      invalidCycle:
        "Use um símbolo de até 16 caracteres, uma janela de 1 a 10 dias e até 10 lembretes de 120 caracteres.",
      noteTooLarge: "Passou de 64 KB e não foi salvo. Encurte o texto.",
      invalidConfig:
        "Não foi possível aplicar este ajuste. Feche o painel e tente de novo.",
      unavailable:
        "Os widgets estão indisponíveis agora. O motivo está no log do Dockyard.",
      writeFailed: "Não foi possível salvar a mudança. Tente de novo.",
      notReady:
        "Os widgets não estão disponíveis agora. Reinicie o Dockyard e tente de novo.",
    } satisfies Record<WidgetErrorCode, string>,
  },
} as const;

/** Same keys as the PT-BR messages, any text. */
type Widen<T> = {
  readonly [K in keyof T]: T[K] extends string ? string : Widen<T[K]>;
};

/** The contract of a locale: a future `en.ts` typed as `Messages` must
 * provide every key (and no other) to compile. */
export type Messages = Widen<typeof messages>;

export const ptBR: Messages = messages;
