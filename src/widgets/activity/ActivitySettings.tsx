import { ACTIVITY_SHOWS, ACTIVITY_STYLES } from "../../core/widgets";
import { useT } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { WidthGroup } from "../system/parts";
import { useSystemSettings } from "../system/useSettings";

/** The "Ajustes" view of an Atividade panel: label, what it shows, how the
 * dock draws it, its width and "Remover do dock". Each change is saved at
 * once. */
function ActivitySettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"activity">) {
  const t = useT();
  const text = t.widgets.activity;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.activity}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <OptionGroup
        label={text.show}
        options={ACTIVITY_SHOWS.map((value) => ({
          value,
          label: text.shows[value],
        }))}
        selected={config.show}
        onSelect={(show) => settings.saveConfig({ show })}
      />
      <OptionGroup
        label={text.style}
        options={ACTIVITY_STYLES.map((value) => ({
          value,
          label: text.styles[value],
        }))}
        selected={config.style}
        onSelect={(style) => settings.saveConfig({ style })}
      />
      <WidthGroup
        width={config.width}
        onSelect={(width) => settings.saveConfig({ width })}
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

export default ActivitySettings;
