import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetSettingsProps } from "../contract";

/** PROVISIONAL (W2 replaces it): the "Ajustes" view of a time-progress panel,
 * with only its header. No "Remover do dock" yet. */
function ProgressSettings({ onBack }: WidgetSettingsProps<"time-progress">) {
  const t = useT();
  return (
    <PanelHeader
      title={t.widgets.progress.settingsTitle}
      onBack={onBack}
      focusAction
    />
  );
}

export default ProgressSettings;
