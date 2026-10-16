import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";

/** PROVISIONAL (W3 replaces it): the main view of a note panel, with
 * only its header. */
function NotePanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"note">) {
  const t = useT();
  return (
    <PanelHeader
      title={widgetName(t, instance)}
      onAdjust={onAdjust}
      focusAction={returnedFromSettings}
    />
  );
}

export default NotePanel;
