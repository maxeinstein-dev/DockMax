import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "../system/system.css";

/** The main view of a Lançador panel (spec 0012). A click on the dock opens
 * the launcher itself, never this panel: it only says how the item works,
 * should it be opened some other way. */
function LauncherPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"launcher">) {
  const t = useT();
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <p className="sys-hint">{t.widgets.launcher.hint}</p>
    </>
  );
}

export default LauncherPanel;
