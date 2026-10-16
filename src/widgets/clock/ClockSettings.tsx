import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetSettingsProps } from "../contract";

/** PROVISIONAL (W1 replaces it): the "Ajustes" view of a clock panel,
 * with only its header. No "Remover do dock" yet. */
function ClockSettings({ onBack }: WidgetSettingsProps<"clock">) {
  const t = useT();
  return (
    <PanelHeader
      title={t.widgets.clock.settingsTitle}
      onBack={onBack}
      focusAction
    />
  );
}

export default ClockSettings;
