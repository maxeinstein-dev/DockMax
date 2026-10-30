import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";

/** The "Ajustes" view of a Lançador panel: label and "Remover do dock".
 * The hotkey is the launcher's, in Configurações › Geral. */
function LauncherSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"launcher">) {
  const t = useT();
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  return (
    <>
      <PanelHeader
        title={t.widgets.launcher.settingsTitle}
        onBack={onBack}
        focusAction
      />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.launcher}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <p className="sys-hint">{t.widgets.launcher.hint}</p>
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

export default LauncherSettings;
