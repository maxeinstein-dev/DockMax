import { dayText, useCycle, type CycleStatus } from "../../core/cycle";
import { format, plural, useT, type Messages } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./cycle.css";

/** The sentence of where the cycle is. */
function phaseText(t: Messages, status: CycleStatus): string {
  const text = t.widgets.cycle;
  const phase = status.phase;
  switch (phase.phase) {
    case "unknown":
      return text.unknown;
    case "period":
      return format(text.period, { day: phase.day });
    case "sensitive":
      return plural(text.sensitive, phase.in);
    case "between":
      return phase.in <= 0 ? text.due : plural(text.between, phase.in);
  }
}

/**
 * The main view of a Ciclo panel (spec 0017 F2b): where the cycle is in
 * words, the next start expected and the averages of the last cycles, read
 * from the marks in her calendar. The tile says none of it.
 */
function CyclePanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"cycle">) {
  const t = useT();
  const text = t.widgets.cycle;
  const view = useCycle(instance.config);
  const status = view?.status ?? null;
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {view && !view.accounts ? (
        <p className="cycle-note">{text.noAccounts}</p>
      ) : status === null ? (
        <p className="cycle-note">{text.loading}</p>
      ) : (
        <>
          <p className="cycle-phase">{phaseText(t, status)}</p>
          {status.next !== null ? (
            <dl className="cycle-facts">
              <dt>{text.next}</dt>
              <dd>{dayText(status.next)}</dd>
              <dt>{text.last}</dt>
              <dd>{status.last === null ? "–" : dayText(status.last)}</dd>
              <dt>{text.cycleLength}</dt>
              <dd>
                {format(text.days, {
                  n: Math.round(status.averages.cycle),
                })}
              </dd>
              <dt>{text.periodLength}</dt>
              <dd>
                {format(text.days, {
                  n: Math.round(status.averages.period),
                })}
              </dd>
            </dl>
          ) : null}
          <p className="cycle-note">
            {status.averages.cycles === 0
              ? format(text.defaults, { symbol: instance.config.symbol })
              : plural(text.basedOn, status.averages.cycles)}
          </p>
        </>
      )}
      <PanelFooter>
        {format(text.footer, { symbol: instance.config.symbol })}
      </PanelFooter>
    </>
  );
}

export default CyclePanel;
