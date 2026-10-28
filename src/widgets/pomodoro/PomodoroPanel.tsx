import { clockOf, usePomodoro } from "../../core/pomodoro";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "./pomodoro.css";

/**
 * The main view of a Pomodoro panel (spec 0017 F2c-2): the stage, the time
 * left, Iniciar or Pausar, Pular and Reiniciar, and the focuses of the
 * round. The timer runs in Rust: closing the panel doesn't stop it.
 */
function PomodoroPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"pomodoro">) {
  const t = useT();
  const text = t.widgets.pomodoro;
  const [view, act] = usePomodoro(instance.id, instance.config);
  const started = view !== null && (view.running || view.left < view.total);
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      {view === null ? (
        <p className="pomodoro-note">{text.loading}</p>
      ) : (
        <>
          <p className="pomodoro-stage">{text.stages[view.stage]}</p>
          <p className="pomodoro-clock" role="timer">
            {clockOf(view.left)}
          </p>
          <div className="pomodoro-actions">
            <button
              type="button"
              className="secondary-button pomodoro-main"
              onClick={() => act(view.running ? "pause" : "start")}
            >
              {view.running ? text.pause : started ? text.resume : text.start}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => act("skip")}
            >
              {text.skip}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => act("reset")}
            >
              {text.reset}
            </button>
          </div>
          <p className="pomodoro-note">
            {format(text.round, { done: view.focuses, every: view.every })}
          </p>
        </>
      )}
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default PomodoroPanel;
