import { rateText, useSystemActivity } from "../../core/systemActivity";
import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import { Sparkline } from "../system/parts";

/** A minute of readings, one every 2 s. */
const SLOTS = 30;

/** The graph's floor, so a quiet network draws flat (as the dock). */
const FLOOR = 64 * 1024;

/** The main view of an Atividade de rede panel (spec 0017 F1): the rates
 * down and up now and their last minute, on one scale, read every 2 s
 * while it is open. */
function NetworkPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"network">) {
  const t = useT();
  const text = t.widgets.network;
  const reading = useSystemActivity();
  const downs = reading?.downHistory ?? [];
  const ups = reading?.upHistory ?? [];
  const peak = Math.max(FLOOR, ...downs, ...ups);
  const rows = [
    { name: text.down, rate: reading?.down, history: downs },
    { name: text.up, rate: reading?.up, history: ups },
  ];
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <ul className="sys-list">
        {rows.map((row) => (
          <li key={row.name}>
            <span className="sys-name">{row.name}</span>
            <span className="sys-figure">
              {row.rate === undefined
                ? t.widgets.system.reading
                : rateText(row.rate)}
            </span>
            <Sparkline values={row.history} peak={peak} slots={SLOTS} />
          </li>
        ))}
      </ul>
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default NetworkPanel;
