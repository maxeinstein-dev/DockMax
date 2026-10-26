import { useState } from "react";
import { clearLauncherHistory, useLauncherStatus } from "../core/launcher";
import { useT } from "../i18n";
import InlineNotice from "./parts/InlineNotice";
import ToggleField from "./parts/ToggleField";
import "./parts/parts.css";

type LauncherFieldProps = {
  /** Whether Alt+Space opens the launcher (`Settings.launcher`); null while
   * the settings load. */
  on: boolean | null;
  onChange: (on: boolean) => void;
};

/**
 * The launcher's place in Configurações › Geral (spec 0012 L1): the switch
 * of Alt+Space, a notice when another program holds it (the launcher then
 * opens from the dock's menu), and forgetting what it opened.
 */
function LauncherField({ on, onChange }: LauncherFieldProps) {
  const t = useT();
  const text = t.settings.launcher;
  const status = useLauncherStatus();
  const [cleared, setCleared] = useState(false);
  return (
    <>
      <ToggleField
        label={text.label}
        hint={text.hint}
        checked={on ?? true}
        disabled={on === null}
        onChange={onChange}
      />
      {on && status?.hotkeyTaken ? (
        <InlineNotice title={text.taken} detail={text.takenDetail} />
      ) : null}
      <div className="toggle-row">
        <div className="toggle-text">
          <span className="toggle-label">{text.history}</span>
          <span className="toggle-hint">{text.historyHint}</span>
        </div>
        <button
          type="button"
          className="secondary-button"
          disabled={cleared}
          onClick={() => {
            void clearLauncherHistory().then(setCleared);
          }}
        >
          {cleared ? text.cleared : text.clear}
        </button>
      </div>
    </>
  );
}

export default LauncherField;
