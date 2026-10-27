import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { WidthGroup } from "../system/parts";
import { useSystemSettings } from "../system/useSettings";

/** The "Ajustes" view of an Atividade de rede panel: label, width (two
 * icons add the graph) and "Remover do dock". Each change is saved at
 * once. */
function NetworkSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"network">) {
  const t = useT();
  const text = t.widgets.network;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.network}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <WidthGroup
        width={settings.shown.config.width}
        onSelect={(width) => settings.saveConfig({ width })}
      />
      <p className="sys-hint">{text.widthHint}</p>
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

export default NetworkSettings;
