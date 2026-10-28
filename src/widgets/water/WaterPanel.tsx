import { useState } from "react";
import { todayOf } from "../../core/countdown";
import { errorKind, log } from "../../core/log";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./water.css";

/**
 * The main view of a Beber água panel (spec 0017 F3-2): the day's cups
 * against the goal, in cups and milliliters, and a cup more or less. The
 * count is saved in the widget's config and starts over at midnight.
 */
function WaterPanel({
  instance,
  now,
  onAdjust,
  returnedFromSettings,
  onUpdate,
}: WidgetPanelProps<"water">) {
  const t = useT();
  const text = t.widgets.water;
  const { config } = instance;
  const today = todayOf(now);
  const count = config.day === today ? config.count : 0;
  const [failed, setFailed] = useState(false);

  function save(next: number, drank: boolean) {
    setFailed(false);
    onUpdate?.({
      label: instance.label,
      config: {
        ...config,
        count: next,
        day: today,
        last: drank ? Math.floor(now.getTime() / 1000) : config.last,
      },
    }).catch((error: unknown) => {
      log.warn(`water: save failed: ${errorKind(error)}`);
      setFailed(true);
    });
  }

  const reached = count >= config.goal;
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <p className="water-count">
        {format(text.cups, { n: count, goal: config.goal })}
      </p>
      <p className="water-ml">
        {format(text.ml, {
          n: (count * config.cup).toLocaleString("pt-BR"),
          goal: (config.goal * config.cup).toLocaleString("pt-BR"),
        })}
      </p>
      <progress
        className="water-bar"
        max={config.goal}
        value={Math.min(count, config.goal)}
        aria-label={text.progress}
      />
      <div className="water-actions">
        <button
          type="button"
          className="secondary-button water-main"
          disabled={count >= 99}
          onClick={() => save(count + 1, true)}
        >
          {format(text.add, { cup: config.cup })}
        </button>
        <button
          type="button"
          className="secondary-button"
          disabled={count === 0}
          onClick={() => save(count - 1, false)}
        >
          {text.undo}
        </button>
      </div>
      {reached ? <p className="water-note">{text.reached}</p> : null}
      <PanelFooter alert={failed ? t.widgets.errors.invalidWater : null}>
        {config.remind > 0
          ? format(text.reminding, { n: config.remind })
          : text.footer}
      </PanelFooter>
    </>
  );
}

export default WaterPanel;
