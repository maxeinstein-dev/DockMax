import { daysLeft, longDay, targetDay, todayOf } from "../../core/countdown";
import { plural, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./countdown.css";

/**
 * The main view of a Contagem regressiva panel (spec 0017 F2c): the days
 * left in words, and the day itself. Its name is the widget's label.
 */
function CountdownPanel({
  instance,
  now,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"countdown">) {
  const t = useT();
  const text = t.widgets.countdown;
  const { config } = instance;
  const today = todayOf(now);
  const left = daysLeft(config, today);
  const target = targetDay(config, today);
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {left === null || target === null ? (
        <p className="countdown-note">{text.noDate}</p>
      ) : (
        <>
          <p className="countdown-days">
            {left === 0
              ? text.today
              : left > 0
                ? plural(text.left, left)
                : plural(text.ago, -left)}
          </p>
          <p className="countdown-date">{longDay(target)}</p>
          {config.yearly ? (
            <p className="countdown-note">{text.everyYear}</p>
          ) : null}
        </>
      )}
      <PanelFooter>{t.widgets.common.footer}</PanelFooter>
    </>
  );
}

export default CountdownPanel;
