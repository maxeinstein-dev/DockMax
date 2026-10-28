import { WATER_CHOICES } from "../../core/widgets";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import SelectField from "../../panel/parts/SelectField";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";
import "./water.css";

/** The "Ajustes" view of a Beber água panel: label, the goal in cups, the
 * size of a cup, the reminder, and "Remover do dock". Each change is saved
 * at once. */
function WaterSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"water">) {
  const t = useT();
  const text = t.widgets.water;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.water}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <SelectField
        label={text.goal}
        value={String(config.goal)}
        options={WATER_CHOICES.goal.map((value) => ({
          value: String(value),
          label: format(text.goalCups, { n: value }),
        }))}
        onChange={(value) => settings.saveConfig({ goal: Number(value) })}
      />
      <SelectField
        label={text.cup}
        value={String(config.cup)}
        options={WATER_CHOICES.cup.map((value) => ({
          value: String(value),
          label: format(text.cupMl, { n: value }),
        }))}
        onChange={(value) => settings.saveConfig({ cup: Number(value) })}
      />
      <SelectField
        label={text.remind}
        value={String(config.remind)}
        options={WATER_CHOICES.remind.map((value) => ({
          value: String(value),
          label:
            value === 0
              ? text.remindOff
              : format(text.remindEvery, { n: value }),
        }))}
        onChange={(value) => settings.saveConfig({ remind: Number(value) })}
      />
      <p className="sys-hint">{text.remindHint}</p>
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

export default WaterSettings;
