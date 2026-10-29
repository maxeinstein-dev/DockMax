import { resetText, useClaudeUsage, type ClaudeLimit } from "../../core/claude";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./claude.css";

/**
 * The main view of a Uso do Claude panel (spec 0017 F5): the five hours'
 * limit and the week's, each with the share used and when it starts over,
 * as the Claude apps show them.
 */
function ClaudePanel({
  instance,
  now,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"claude">) {
  const t = useT();
  const text = t.widgets.claude;
  const reading = useClaudeUsage();
  const seconds = Math.floor(now.getTime() / 1000);

  const limit = (name: string, value: ClaudeLimit | null) => (
    <section className="claude-limit" aria-label={name}>
      <div className="claude-limit-head">
        <span className="claude-limit-name">{name}</span>
        <span className="claude-limit-percent">
          {value ? format(text.used, { n: Math.round(value.percent) }) : "–"}
        </span>
      </div>
      <progress
        className="claude-bar"
        max={100}
        value={value?.percent ?? 0}
        aria-label={name}
      />
      {value?.resetsAt != null ? (
        <p className="claude-reset">
          {format(text.resets, {
            when: resetText(value.resetsAt, seconds, text.resetWords),
          })}
        </p>
      ) : null}
    </section>
  );

  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {reading === undefined || reading === null ? (
        <p className="claude-note">{text.loading}</p>
      ) : reading.state === "ready" ? (
        <>
          {limit(text.fiveHour, reading.usage.fiveHour)}
          {limit(text.week, reading.usage.sevenDay)}
        </>
      ) : (
        <p className="claude-note">{text.states[reading.state]}</p>
      )}
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default ClaudePanel;
