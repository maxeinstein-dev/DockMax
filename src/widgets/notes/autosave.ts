import type { WidgetCommandError, WidgetErrorCode } from "../../core/widgets";

/* The note's automatic save (spec 0003 §2.2, Nota): about 500 ms after the
 * last keystroke, and right away when the panel asks (losing focus, closing).
 * Pure scheduling: `save` and the timer are injected, so tests drive it with
 * fake timers and a fake save, without React or `invoke`. */

/** Where the note's text stands, for the panel. */
export type AutosaveState =
  /** Everything on screen is saved. */
  | "idle"
  /** A change waits for the pause in typing. */
  | "pending"
  /** A save is running. */
  | "saving"
  /** The last save failed and nothing newer is waiting; the text stays on
   * screen, unsaved. */
  | "error";

export interface AutosaveStatus {
  state: AutosaveState;
  /** Code of the last failed save, kept while the text on screen is still
   * unsaved, so the message doesn't blink away while the user types. Only
   * the code: the note's text never leaves this module in an error. */
  error: WidgetErrorCode | null;
}

/** The timer functions, injected (`setTimeout`/`clearTimeout`). */
export interface Schedule {
  setTimeout(callback: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
}

export interface AutosaveOptions {
  /** Saves the whole text (`save_note`); rejects with a
   * `WidgetCommandError`. */
  save: (text: string) => Promise<void>;
  /** Pause in typing before saving. */
  delayMs?: number;
  /** Defaults to the global timers, looked up at each call so fake timers
   * apply. */
  schedule?: Schedule;
  /** The text already stored (from `get_note`): typing it back saves
   * nothing. */
  savedText?: string;
}

export interface Autosave {
  /** The text on screen changed: (re)starts the pause. */
  change(text: string): void;
  /** Saves the pending text now, without waiting for the pause, and
   * resolves when no save is left running. Resolves on failure too: the
   * outcome is in `getStatus()`. */
  flush(): Promise<void>;
  /** Stops the timer and every later save and notification. Pending text is
   * dropped: call `flush()` first to keep it. */
  dispose(): void;
  /** The current status; the same object until it changes, as
   * `useSyncExternalStore` needs. */
  getStatus(): AutosaveStatus;
  /** Calls `listener` after each status change; returns the unsubscribe. */
  subscribe(listener: () => void): () => void;
}

export const AUTOSAVE_DELAY_MS = 500;

const globalSchedule: Schedule = {
  setTimeout: (callback, ms) => globalThis.setTimeout(callback, ms),
  clearTimeout: (handle) =>
    globalThis.clearTimeout(handle as Parameters<typeof clearTimeout>[0]),
};

/** The code of a failed save. Anything that isn't a command error (a bug, a
 * refused payload) reads as `writeFailed`: the text wasn't saved. */
function errorCodeOf(error: unknown): WidgetErrorCode {
  const code = (error as Partial<WidgetCommandError> | null)?.code;
  return typeof code === "string" ? code : "writeFailed";
}

export function createAutosave({
  save,
  delayMs = AUTOSAVE_DELAY_MS,
  schedule = globalSchedule,
  savedText = "",
}: AutosaveOptions): Autosave {
  /** Text on screen. */
  let latest = savedText;
  /** Text the store holds. */
  let saved = savedText;
  /** Text whose last save failed: not retried on its own, or a refused text
   * (too large) would loop; a new change or a `flush()` retries it. */
  let failed: string | null = null;
  let timer: { handle: unknown } | null = null;
  /** At most one save at a time, so two saves can never race. */
  let inFlight: Promise<void> | null = null;
  let disposed = false;
  let status: AutosaveStatus = { state: "idle", error: null };
  const listeners = new Set<() => void>();

  function setStatus(state: AutosaveState, error: WidgetErrorCode | null) {
    if (disposed || (status.state === state && status.error === error)) return;
    status = { state, error };
    for (const listener of listeners) listener();
  }

  function cancelTimer() {
    if (timer) schedule.clearTimeout(timer.handle);
    timer = null;
  }

  /** Starts a save of the text on screen, unless one is running or there is
   * nothing new to save. */
  function pump() {
    if (disposed || inFlight) return;
    if (latest === saved) {
      setStatus("idle", null);
      return;
    }
    const text = latest;
    setStatus("saving", status.error);
    let request: Promise<void>;
    try {
      request = Promise.resolve(save(text));
    } catch (error) {
      request = Promise.reject(error);
    }
    inFlight = request
      .then(
        () => {
          saved = text;
          failed = null;
          if (status.error !== null) setStatus(status.state, null);
        },
        (error: unknown) => {
          failed = text;
          setStatus(status.state, errorCodeOf(error));
        },
      )
      .finally(() => {
        inFlight = null;
        settle();
      });
  }

  /** After a save: wait for a running pause, save what arrived meanwhile,
   * or rest. */
  function settle() {
    if (disposed) return;
    if (timer) setStatus("pending", status.error);
    else if (latest === saved) setStatus("idle", null);
    else if (latest === failed) setStatus("error", status.error);
    else pump();
  }

  /** The pause in typing ended. While a save runs, its `settle` saves this
   * text when it ends. */
  function onPause() {
    timer = null;
    pump();
  }

  return {
    change(text) {
      if (disposed) return;
      latest = text;
      cancelTimer();
      if (inFlight) {
        // The running save finishes first; `settle` picks this text up.
        timer = { handle: schedule.setTimeout(onPause, delayMs) };
        return;
      }
      if (latest === saved) {
        setStatus("idle", null);
        return;
      }
      timer = { handle: schedule.setTimeout(onPause, delayMs) };
      setStatus("pending", status.error);
    },

    async flush() {
      if (disposed) return;
      cancelTimer();
      // An explicit request retries a text that failed before.
      failed = null;
      pump();
      // `settle` may chain one more save for text typed during the first.
      while (inFlight) await inFlight;
    },

    dispose() {
      cancelTimer();
      disposed = true;
      listeners.clear();
    },

    getStatus: () => status,

    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
