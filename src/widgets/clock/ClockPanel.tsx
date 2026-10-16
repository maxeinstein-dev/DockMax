import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";

/** PROVISIONAL (W1 replaces it): the main view of a clock panel, with
 * only its header. */
function ClockPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"clock">) {
  const t = useT();
  return (
    <PanelHeader
      title={widgetName(t, instance)}
      onAdjust={onAdjust}
      focusAction={returnedFromSettings}
    />
  );
}

export default ClockPanel;
