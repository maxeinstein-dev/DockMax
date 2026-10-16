import { useLayoutEffect, useRef } from "react";
import type { Period, TimeProgressConfig } from "../../core/widgets";
import { format, plural, useT, type Messages } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import { progressOf, remainingParts } from "./progress";
import "./time-progress.css";

/** `faltam 2 dias 3 h`, from the units of `remainingParts`. */
function remainingText(t: Messages, remainingMs: number): string {
  const text = t.widgets.progress;
  const time = remainingParts(remainingMs)
    .map((part) => plural(text.units[part.unit], part.value))
    .join(" ");
  return format(text.remaining, { time });
}

/** The bar of a period. Its share goes through the CSSOM on `--f`, never a
 * `style` attribute in the markup (strict CSP). Decorative: the line's
 * percentage says the same. */
function Bar({ fraction }: { fraction: number }) {
  const fill = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    fill.current?.style.setProperty("--f", fraction.toFixed(4));
  }, [fraction]);
  return (
    <span className="pg-bar" aria-hidden="true">
      <span ref={fill} className="pg-fill" />
    </span>
  );
}

/** One period's line: name, bar, percentage and time left; or, for the
 * workday off, "Fora do expediente" in place of the bar. */
function PeriodLine({
  period,
  now,
  config,
}: {
  period: Period;
  now: Date;
  config: TimeProgressConfig;
}) {
  const t = useT();
  const text = t.widgets.progress;
  const progress = progressOf(period, now, config);
  if (progress.state === "off") {
    return (
      <li className="off">
        <span className="pg-name">{text.periods[period]}</span>
        <span className="pg-left">{text.offWorkday}</span>
      </li>
    );
  }
  return (
    <li>
      <span className="pg-name">{text.periods[period]}</span>
      <Bar fraction={progress.fraction} />
      <span className="pg-pct">{Math.floor(progress.fraction * 100)}%</span>
      <span className="pg-left">{remainingText(t, progress.remainingMs)}</span>
    </li>
  );
}

/** The main view of a time-progress panel (DESIGN.md "Painel": Progresso do
 * tempo): a line per period, in the config's order, and the week's first
 * day in the footer. */
function ProgressPanel({
  instance,
  now,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"time-progress">) {
  const t = useT();
  const { config } = instance;
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <ul className="pg-list">
        {config.periods.map((period) => (
          <PeriodLine key={period} period={period} now={now} config={config} />
        ))}
      </ul>
      <PanelFooter>{t.widgets.progress.footer[config.weekStart]}</PanelFooter>
    </>
  );
}

export default ProgressPanel;
