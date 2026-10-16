import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";

/** PROVISIONAL (W2 replaces it): the main view of a time-progress panel, with
 * only its header. */
function ProgressPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"time-progress">) {
  const t = useT();
  return (
    <PanelHeader
      title={widgetName(t, instance)}
      onAdjust={onAdjust}
      focusAction={returnedFromSettings}
    />
  );
}

export default ProgressPanel;
