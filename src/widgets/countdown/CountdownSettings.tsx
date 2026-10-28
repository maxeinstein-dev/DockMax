import { useState } from "react";
import { parseDay } from "../../core/countdown";
import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import ToggleField from "../../panel/parts/ToggleField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";
import "./countdown.css";

/** The "Ajustes" view of a Contagem regressiva panel: its name (the
 * label), the day, whether it comes back every year, and "Remover do
 * dock". Each change is saved at once. */
function CountdownSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"countdown">) {
  const t = useT();
  const text = t.widgets.countdown;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  const [date, setDate] = useState(config.date ?? "");
  const invalid = date !== "" && parseDay(date) === null;

  function commitDate() {
    if (invalid) return;
    const next = date === "" ? null : date;
    if (next !== config.date) settings.saveConfig({ date: next });
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={text.name}
        placeholder={text.namePlaceholder}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <TextField
        label={text.date}
        type="date"
        value={date}
        invalid={invalid || settings.error === "invalidCountdown"}
        onChange={setDate}
        onCommit={commitDate}
      />
      <ToggleField
        label={text.yearly}
        hint={text.yearlyHint}
        checked={config.yearly}
        onChange={(yearly) => settings.saveConfig({ yearly })}
      />
      <RemoveButton
        onClick={() => void settings.remove()}
        disabled={settings.removing}
      />
      <PanelFooter
        alert={settings.error ? t.widgets.errors[settings.error] : null}
      >
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default CountdownSettings;
