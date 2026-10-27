import {
  memoryPercent,
  sizeText,
  useSystemActivity,
} from "../../core/systemActivity";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import { MeterBar, Sparkline } from "../system/parts";

/** A minute of readings, one every 2 s. */
const SLOTS = 30;

/** The main view of an Atividade panel (spec 0017 F1): the processor, with
 * its last minute, and the memory in use, read every 2 s while it is open.
 * Both show whatever the tile shows. */
function ActivityPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"activity">) {
  const t = useT();
  const text = t.widgets.activity;
  const reading = useSystemActivity();
  const percent = (value: number | null) =>
    value === null
      ? t.widgets.system.reading
      : format(t.widgets.system.percent, { n: Math.round(value) });
  const cpu = reading?.cpu ?? null;
  const memory = reading?.memory ?? null;
  const memoryShare = memoryPercent(memory);
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <ul className="sys-list">
        <li>
          <span className="sys-name">{text.cpu}</span>
          <span className="sys-figure">{percent(cpu)}</span>
          <MeterBar fraction={cpu === null ? null : cpu / 100} />
          <Sparkline
            values={reading?.cpuHistory ?? []}
            peak={100}
            slots={SLOTS}
          />
        </li>
        <li>
          <span className="sys-name">{text.memory}</span>
          <span className="sys-figure">{percent(memoryShare)}</span>
          <MeterBar
            fraction={memoryShare === null ? null : memoryShare / 100}
          />
          {memory ? (
            <span className="sys-detail">
              {format(text.memoryUse, {
                used: sizeText(memory.total - memory.available),
                total: sizeText(memory.total),
              })}
            </span>
          ) : null}
        </li>
      </ul>
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default ActivityPanel;
