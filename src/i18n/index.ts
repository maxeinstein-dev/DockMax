import { ptBR, type Messages } from "./pt-BR";

export type { Messages };

/** Locales the interface ships. Adding one is a new entry here and in
 * `MESSAGES`. */
type Locale = "pt-BR";

const MESSAGES: Record<Locale, Messages> = {
  "pt-BR": ptBR,
};

/** Only PT-BR exists for now; the locale will come from the settings. */
const ACTIVE_LOCALE: Locale = "pt-BR";

/** The messages of the active locale. */
export function useT(): Messages {
  return MESSAGES[ACTIVE_LOCALE];
}
