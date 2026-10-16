import { format, useT, type Messages } from "../../i18n";
import InlineNotice from "../../panel/parts/InlineNotice";
import PanelHeader from "../../panel/parts/PanelHeader";
import PanelFooter from "../../panel/PanelFooter";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./clock.css";
import { formatClock, type ClockReading, type Instant } from "./format";
import { utcOffset, zoneCity } from "./zones";

/** The footer's caption: the zone and its offset, `do sistema` when the
 * clock follows the system, or that the system zone wasn't identified
 * (then no name and no offset: nothing is invented). */
function zoneCaption(t: Messages, reading: ClockReading, now: Instant): string {
  const text = t.widgets.clock;
  if (reading.timeZone === null) return text.systemZoneUnknown;
  const city = zoneCity(reading.timeZone);
  return format(text.footer, {
    zone: reading.system ? format(text.systemZone, { zone: city }) : city,
    offset: utcOffset(now, reading.timeZone),
  });
}

/** The main view of a clock panel (DESIGN.md "Painel": Relógio): the time,
 * with the seconds when set and AM/PM in 12 h, the long date, and the zone
 * in the footer. A zone this WebView doesn't know shows the system time,
 * with a notice; formatting never throws. */
function ClockPanel({
  instance,
  now,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"clock">) {
  const t = useT();
  const reading = formatClock(now, instance.config);
  const seconds = instance.config.showSeconds
    ? reading.time.slice(reading.icon.length)
    : null;
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {reading.timeZoneFallback ? (
        <InlineNotice title={t.widgets.clock.zoneFallback} />
      ) : null}
      <div className="clock-reading">
        <p className="clock-time">
          <span>{reading.icon}</span>
          {seconds ? <span className="clock-seconds">{seconds}</span> : null}
          {reading.dayPeriod ? (
            <span className="clock-period">{reading.dayPeriod}</span>
          ) : null}
        </p>
        <p className="clock-date">{reading.date}</p>
      </div>
      <PanelFooter>{zoneCaption(t, reading, now)}</PanelFooter>
    </>
  );
}

export default ClockPanel;
