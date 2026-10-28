import { POMODORO_CHOICES } from "../../core/widgets";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import SelectField from "../../panel/parts/SelectField";
import TextField from "../../panel/parts/TextField";
import ToggleField from "../../panel/parts/ToggleField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";
import "./pomodoro.css";

/** The "Ajustes" view of a Pomodoro panel: label, the minutes of focus, of
 * a pause and of the long pause, how many focuses before it, whether the
 * next stage starts by itself, the notice of Windows, and "Remover do
 * dock". Each change is saved at once. */
function PomodoroSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"pomodoro">) {
  const t = useT();
  const text = t.widgets.pomodoro;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  const minutes = (values: readonly number[]) =>
    values.map((value) => ({
      value: String(value),
      label: format(text.minutes, { n: value }),
    }));
  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.pomodoro}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <SelectField
        label={text.focus}
        value={String(config.focus)}
        options={minutes(POMODORO_CHOICES.focus)}
        onChange={(value) => settings.saveConfig({ focus: Number(value) })}
      />
      <SelectField
        label={text.short}
        value={String(config.short)}
        options={minutes(POMODORO_CHOICES.short)}
        onChange={(value) => settings.saveConfig({ short: Number(value) })}
      />
      <SelectField
        label={text.long}
        value={String(config.long)}
        options={minutes(POMODORO_CHOICES.long)}
        onChange={(value) => settings.saveConfig({ long: Number(value) })}
      />
      <SelectField
        label={text.every}
        value={String(config.every)}
        options={POMODORO_CHOICES.every.map((value) => ({
          value: String(value),
          label: format(text.focuses, { n: value }),
        }))}
        onChange={(value) => settings.saveConfig({ every: Number(value) })}
      />
      <ToggleField
        label={text.autoStart}
        hint={text.autoStartHint}
        checked={config.autoStart}
        onChange={(autoStart) => settings.saveConfig({ autoStart })}
      />
      <ToggleField
        label={text.windows}
        hint={text.windowsHint}
        checked={config.windows}
        onChange={(windows) => settings.saveConfig({ windows })}
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

export default PomodoroSettings;
