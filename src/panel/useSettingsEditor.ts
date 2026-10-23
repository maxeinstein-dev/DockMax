import { useRef, useState } from "react";
import {
  isSettingsCommandError,
  updateSettings,
  type Settings,
  type SettingsErrorCode,
} from "../core/settings";

/** A change sent to Rust and not yet confirmed by `settings-changed`. */
interface Pending {
  /** Tells the latest change from older ones still answering. */
  id: number;
  /** The full settings sent. */
  value: Settings;
  /** Rust accepted it. */
  saved: boolean;
  /** New confirmed settings arrived (`settings-changed`) since it was
   * sent. */
  confirmed: boolean;
}

export interface SettingsEditor {
  /** What the controls show: the change in flight, until it is both saved
   * and confirmed, else the settings in use; null while they can't be read. */
  shown: Settings | null;
  /** Applies `next` to what is shown and saves the result. */
  change: (next: (current: Settings) => Settings) => void;
  /** Why the last change was refused, until the next one. */
  error: SettingsErrorCode | null;
}

/**
 * The editing of the settings by a panel view: a choice shows at once while
 * Rust saves it; if Rust refuses, the reason is `error` and the controls go
 * back to the confirmed settings (spec 0002 §2.3 item 4, §2.4 item 3).
 * `settings` is the settings in use (`useSettings`), the only source of the
 * confirmed values.
 */
export function useSettingsEditor(settings: Settings | null): SettingsEditor {
  const [pending, setPending] = useState<Pending | null>(null);
  const [error, setError] = useState<SettingsErrorCode | null>(null);
  const lastId = useRef(0);
  const [lastSettings, setLastSettings] = useState(settings);

  // New confirmed settings: the pending change has its answer from
  // `settings-changed`, or will once saved (the event and the command's
  // answer may come in either order). A saved change is done then.
  if (lastSettings !== settings) {
    setLastSettings(settings);
    if (pending) {
      setPending(pending.saved ? null : { ...pending, confirmed: true });
    }
  }

  // Optimistic until both saved and confirmed.
  const shown =
    pending && !(pending.saved && pending.confirmed) ? pending.value : settings;

  function change(next: (current: Settings) => Settings) {
    setError(null);
    if (shown === null) {
      // Nothing to build the full object from.
      setError("notReady");
      return;
    }
    const value = next(shown);
    const id = ++lastId.current;
    setPending({ id, value, saved: false, confirmed: false });
    updateSettings(value).then(
      () => {
        setPending((current) => {
          if (current?.id !== id) return current;
          return current.confirmed ? null : { ...current, saved: true };
        });
      },
      (reason: unknown) => {
        // An older change answering late says nothing about the latest.
        if (id !== lastId.current) return;
        setPending(null);
        setError(isSettingsCommandError(reason) ? reason.code : "notReady");
      },
    );
  }

  return { shown, change, error };
}
