import type {
  DockEdge,
  DockOpacity,
  DockStyle,
  HoverEffect,
  Indicator,
  IndicatorColor,
  SettingsErrorCode,
  SizePreset,
  TaskbarMode,
  Theme,
} from "../core/settings";
import type {
  ClockIconStyle,
  CreatableKind,
  HourCycle,
  Period,
  WeekStart,
  WidgetErrorCode,
  WidgetKind,
} from "../core/widgets";
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
      edge: "Posição",
      size: "Tamanho",
      opacity: "Opacidade do dock",
      taskbar: "Barra de tarefas do Windows",
    },
    theme: {
      system: "Sistema",
      light: "Claro",
      dark: "Escuro",
    } satisfies Record<Theme, string>,
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
        canary: "Canarinho",
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
      iconSizeOutOfRange:
        "Esse tamanho não é aceito. Escolha Pequeno, Médio ou Grande.",
      unsupportedVersion:
        "Estas configurações são de outra versão. Reinicie o DockMax e tente de novo.",
      writeFailed: "Não foi possível salvar a mudança. Tente de novo.",
      notReady:
        "As configurações não estão disponíveis agora. Reinicie o DockMax e tente de novo.",
    } satisfies Record<SettingsErrorCode, string>,
  },
  /** The notification area in its panel (ADR 0009, Fase E). */
  tray: {
    /** Shown while no program has put an icon there. */
    empty: "Nenhum programa colocou um ícone aqui ainda.",
    /** Under the empty text: why a running program may be missing. */
    emptyHint:
      "Alguns só aparecem depois de abertos de novo, depois que o DockMax assumiu a bandeja.",
    /** The icon of a program without a tooltip. */
    unnamed: "Programa sem nome",
    /** Footer: the buttons do what the taskbar's did. */
    footer: "Botão direito abre o menu do programa",
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
    repository: "github.com/maxeinstein-dev/DockMax",
    footer: "Projeto independente",
  },
  /** Texts of the widgets and of Configurações › Itens do dock (spec 0003
   * §2.2, §2.3; the approved mock `docs/design/widgets-mock.html`). A text
   * with `{name}` placeholders goes through `format`; a `{ one, other }`
   * pair through `plural`. */
  widgets: {
    /** Name of each kind: the dock item's name and the panel title when the
     * instance has no label, and the label field's placeholder. */
    kinds: {
      clock: "Relógio",
      "time-progress": "Progresso do tempo",
      note: "Nota",
      shortcut: "Atalho",
      files: "Arquivos",
    } satisfies Record<WidgetKind, string>,
    /** Short name of each kind on the add buttons of Itens do dock. */
    addNames: {
      clock: "Relógio",
      "time-progress": "Progresso",
      note: "Nota",
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
      /** In place of the count at the limit; the add buttons turn off. */
      limit: "Limite de {max} widgets",
      footer: "A ordem é a do dock",
      footerUnavailable: "Configurações e Sobre continuam funcionando",
      /** Notice of a database set aside and recreated, until dismissed. */
      recovered: {
        title: "O banco de dados estava danificado e foi recriado.",
        detail:
          "As notas antigas ficaram guardadas na pasta de dados do DockMax.",
        dismiss: "Entendi",
      },
      /** Notice of a database that can't be used in this run. */
      unavailableNotice: {
        title: "Os widgets estão indisponíveis.",
        detail:
          "O banco de dados não pôde ser aberto e não foi alterado: suas notas continuam no arquivo. O motivo está no log do DockMax.",
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
      noteTooLarge: "Passou de 64 KB e não foi salvo. Encurte o texto.",
      invalidConfig:
        "Não foi possível aplicar este ajuste. Feche o painel e tente de novo.",
      unavailable:
        "Os widgets estão indisponíveis agora. O motivo está no log do DockMax.",
      writeFailed: "Não foi possível salvar a mudança. Tente de novo.",
      notReady:
        "Os widgets não estão disponíveis agora. Reinicie o DockMax e tente de novo.",
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
