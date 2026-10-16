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

/** Values for the `{name}` placeholders of a message. */
export type MessageValues = Readonly<Record<string, string | number>>;

/** `template` with each `{name}` replaced by `values[name]`. A placeholder
 * without a value is left as written, so a missing value shows up in the
 * interface (and in tests) instead of vanishing. Values are inserted as
 * text: React escapes them, nothing here is HTML. */
export function format(template: string, values: MessageValues): string {
  return template.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    Object.prototype.hasOwnProperty.call(values, name)
      ? String(values[name])
      : placeholder,
  );
}

/** A message that depends on a count: `one` for 1, `other` otherwise (the
 * PT-BR rule; another locale may need `Intl.PluralRules`). */
export interface Plural {
  readonly one: string;
  readonly other: string;
}

/** The form of `message` for `count`, with `{n}` replaced by it. */
export function plural(message: Plural, count: number): string {
  return format(count === 1 ? message.one : message.other, { n: count });
}
