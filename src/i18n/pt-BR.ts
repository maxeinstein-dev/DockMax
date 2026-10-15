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
    underConstruction: "Em construção.",
    /** Shown in place of a panel that failed to render. */
    failed: "Não foi possível mostrar este painel.",
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
