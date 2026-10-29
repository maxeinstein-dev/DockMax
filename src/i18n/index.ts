import { useSyncExternalStore } from "react";
import type { Language } from "../core/settings";
import { en } from "./en";
import { ptBR, type Messages } from "./pt-BR";

export type { Messages };

/** Locales the interface ships (spec 0019 G1). Adding one is a new entry
 * here, in `MESSAGES` and in Rust's `i18n.rs`. */
export type Locale = "pt-BR" | "en";

const MESSAGES: Record<Locale, Messages> = {
  "pt-BR": ptBR,
  en,
};

/** The locale of `language`; "Sistema" is the Windows display language as
 * the WebView has it: Portuguese if it is one, English otherwise (as Rust
 * decides in `i18n::set_language`). */
export function resolveLocale(
  language: Language,
  system: string = typeof navigator === "undefined"
    ? "pt-BR"
    : navigator.language,
): Locale {
  if (language !== "system") return language;
  return system.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

/** Until the settings arrive, the system's. */
let active: Locale = resolveLocale("system");
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The locale in use, for `Intl` and `toLocaleString`. */
export function activeLocale(): Locale {
  return active;
}

/** Switches the interface to `language`: every `useT` renders again. */
export function setLanguage(language: Language): void {
  const next = resolveLocale(language);
  if (typeof document !== "undefined") document.documentElement.lang = next;
  if (next === active) return;
  active = next;
  for (const listener of listeners) listener();
}

/** The messages of the active locale. */
export function useT(): Messages {
  return MESSAGES[useSyncExternalStore(subscribe, activeLocale)];
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
