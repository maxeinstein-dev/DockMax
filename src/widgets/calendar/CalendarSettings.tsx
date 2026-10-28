import { useCalendar } from "../../core/calendar";
import { CALENDAR_WIDTHS } from "../../core/widgets";
import { useT } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { WidthGroup } from "../system/parts";
import { useSystemSettings } from "../system/useSettings";
import "./calendar.css";

/** The "Ajustes" view of a Calendário panel: label, width, the calendars
 * it shows (every one of the accounts, unchecked to hide) and "Remover do
 * dock". Each change is saved at once. */
function CalendarSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"calendar">) {
  const t = useT();
  const text = t.widgets.calendar;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  const sources = useCalendar()?.snapshot?.sources ?? [];

  function toggle(id: string, shown: boolean) {
    const hidden = shown
      ? config.hidden.filter((each) => each !== id)
      : [...config.hidden, id];
    settings.saveConfig({ hidden });
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.calendar}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <WidthGroup
        width={config.width}
        widths={CALENDAR_WIDTHS}
        onSelect={(width) => settings.saveConfig({ width })}
      />
      <OptionGroup
        label={text.showTasks}
        options={[
          { value: "show", label: text.showTasksOptions.show },
          { value: "hide", label: text.showTasksOptions.hide },
        ]}
        selected={config.tasks ? "show" : "hide"}
        onSelect={(value) => settings.saveConfig({ tasks: value === "show" })}
      />
      <fieldset className="cal-sources">
        <legend>{text.calendars}</legend>
        {sources.length === 0 ? (
          <p className="sys-hint">{text.noCalendars}</p>
        ) : (
          <>
            {sources.map((source) => (
              <label key={source.id} className="cal-source">
                <input
                  type="checkbox"
                  checked={!config.hidden.includes(source.id)}
                  onChange={(event) => toggle(source.id, event.target.checked)}
                />
                <span className="cal-source-name">{source.name}</span>
                <span className="cal-source-account">{source.account}</span>
              </label>
            ))}
            <p className="sys-hint">{text.calendarsHint}</p>
          </>
        )}
      </fieldset>
      <RemoveButton
        onClick={() => void settings.remove()}
        disabled={settings.removing}
      />
      <PanelFooter
        alert={settings.error ? t.widgets.errors[settings.error] : null}
      >
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default CalendarSettings;
