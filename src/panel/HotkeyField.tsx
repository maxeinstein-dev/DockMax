import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import {
  DEFAULT_HOTKEY,
  captureLauncherHotkey,
  hotkeyFromEvent,
  hotkeyLabel,
  setLauncherHotkey,
  type HotkeyErrorCode,
} from "../core/launcher";
import { format, useT } from "../i18n";
import "./parts/parts.css";

type HotkeyFieldProps = {
  /** The hotkey in use (`Settings.launcherHotkey`). */
  hotkey: string;
  disabled?: boolean;
};

type State =
  | { kind: "idle" }
  | { kind: "capturing" }
  | { kind: "saving" }
  | { kind: "saved"; hotkey: string }
  | { kind: "refused"; code: HotkeyErrorCode };

/**
 * The launcher's hotkey in Configurações › Geral (spec 0012 L3): a button
 * that shows it and, pressed, captures the next combination. While it
 * captures, Rust lets go of the current hotkey (or pressing it would open
 * the launcher); Esc or leaving the button cancels. Rust tries the new one
 * for real before saving it, and says why when it can't.
 */
function HotkeyField({ hotkey, disabled = false }: HotkeyFieldProps) {
  const t = useT();
  const text = t.settings.launcher.hotkey;
  const hintId = useId();
  const statusId = useId();
  const [state, setState] = useState<State>({ kind: "idle" });
  const capturing = useRef(false);
  const label = (value: string) => hotkeyLabel(value, text.space);

  function stopCapture() {
    if (!capturing.current) return;
    capturing.current = false;
    captureLauncherHotkey(false);
  }

  // Leaving Configurações in the middle gives the hotkey back.
  useEffect(
    () => () => {
      if (capturing.current) captureLauncherHotkey(false);
    },
    [],
  );

  function start() {
    capturing.current = true;
    captureLauncherHotkey(true);
    setState({ kind: "capturing" });
  }

  function cancel() {
    stopCapture();
    setState({ kind: "idle" });
  }

  function save(wanted: string) {
    setState({ kind: "saving" });
    void setLauncherHotkey(wanted).then((code) => {
      // Rust ended the capture: the new hotkey, or the one before it, is
      // held again.
      capturing.current = false;
      setState(
        code === null
          ? { kind: "saved", hotkey: wanted }
          : { kind: "refused", code },
      );
    });
  }

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (state.kind !== "capturing") return;
    const plain =
      !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (plain && event.key === "Tab") {
      cancel();
      return;
    }
    // Every other key is the capture's: not the page's, not Windows' (the
    // window's menu on Alt+Space), not Esc closing Configurações.
    event.preventDefault();
    event.stopPropagation();
    if (plain && event.key === "Escape") {
      cancel();
      return;
    }
    const pressed = hotkeyFromEvent(event);
    if (pressed === "modifier") return;
    if (pressed === "unsupported") {
      stopCapture();
      setState({ kind: "refused", code: "unsupportedKey" });
      return;
    }
    save(pressed);
  }

  const status =
    state.kind === "saving"
      ? text.saving
      : state.kind === "saved"
        ? format(text.saved, { hotkey: label(state.hotkey) })
        : null;

  return (
    <div className="hotkey-field">
      <div className="toggle-row">
        <div className="toggle-text">
          <span className="toggle-label">{text.label}</span>
          <span id={hintId} className="toggle-hint">
            {text.hint}
          </span>
        </div>
        <button
          type="button"
          className="secondary-button hotkey-button"
          aria-label={
            state.kind === "capturing"
              ? text.press
              : format(text.change, { hotkey: label(hotkey) })
          }
          aria-describedby={hintId}
          aria-pressed={state.kind === "capturing"}
          disabled={disabled || state.kind === "saving"}
          onClick={() => {
            if (state.kind !== "capturing") start();
          }}
          onKeyDown={onKeyDown}
          onBlur={() => {
            if (state.kind === "capturing") cancel();
          }}
        >
          {state.kind === "capturing" ? text.press : label(hotkey)}
        </button>
      </div>
      <p id={statusId} className="toggle-hint" role="status">
        {status}
      </p>
      {state.kind === "refused" ? (
        <p className="field-error" role="alert">
          {text.errors[state.code]}
        </p>
      ) : null}
      {hotkey !== DEFAULT_HOTKEY && state.kind !== "capturing" ? (
        <button
          type="button"
          className="quiet-button"
          disabled={disabled || state.kind === "saving"}
          onClick={() => save(DEFAULT_HOTKEY)}
        >
          {text.restore}
        </button>
      ) : null}
    </div>
  );
}

export default HotkeyField;
