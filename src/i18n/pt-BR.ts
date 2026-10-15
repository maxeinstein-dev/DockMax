import type {
  DockEdge,
  DockOpacity,
  SettingsErrorCode,
  SizePreset,
  Theme,
} from "../core/settings";

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
    },
  },
  panel: {
    /** Heading of each panel. */
    titles: {
      settings: "Configurações",
      about: "Sobre",
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
    /** Footer text: there is no Save button. */
    footer: "Salvo automaticamente",
    /** Shown when a change is refused, by error code; the option goes back
     * to its previous value. A new code without a message does not
     * compile. */
    errors: {
      iconSizeOutOfRange:
        "Esse tamanho não é aceito. Escolha Pequeno, Médio ou Grande.",
      unsupportedVersion:
        "Estas configurações são de outra versão. Reinicie o DockMax e tente de novo.",
      writeFailed: "Não foi possível salvar a mudança. Tente de novo.",
      notReady:
        "As configurações não estão disponíveis agora. Reinicie o DockMax e tente de novo.",
    } satisfies Record<SettingsErrorCode, string>,
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
    /** Text of the link to the source code repository. */
    repository: "github.com/maxeinstein-dev/DockMax",
    footer: "Projeto independente",
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
