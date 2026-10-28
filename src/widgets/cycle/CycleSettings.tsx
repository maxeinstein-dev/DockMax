import { useState } from "react";
import { useCalendar } from "../../core/calendar";
import { cycleTestNotice } from "../../core/cycle";
import { errorKind, log } from "../../core/log";
import {
  CYCLE_WINDOWS,
  MAX_REMINDER_CHARS,
  MAX_REMINDERS,
  REMINDER_ANCHORS,
  REMINDER_DAYS,
  type CycleNotices,
  type CycleReminder,
  type ReminderAnchor,
} from "../../core/widgets";
import { format, plural, useT, type Messages } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import IconButton from "../../panel/parts/IconButton";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import SelectField from "../../panel/parts/SelectField";
import TextField from "../../panel/parts/TextField";
import ToggleField from "../../panel/parts/ToggleField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";
import "./cycle.css";

/** `1 dia antes`, `No dia`, `3 dias depois`. */
function daysText(t: Messages, days: number): string {
  const text = t.widgets.cycle.reminderDays;
  if (days === 0) return text.same;
  return days < 0 ? plural(text.before, -days) : plural(text.after, days);
}

/** `1 dia antes da TPM`: when a reminder falls. */
function whenText(t: Messages, reminder: CycleReminder): string {
  return format(t.widgets.cycle.reminderWhen, {
    days: daysText(t, reminder.days),
    anchor: t.widgets.cycle.anchors[reminder.anchor],
  });
}

const DAY_OPTIONS = Array.from(
  { length: REMINDER_DAYS * 2 + 1 },
  (_, index) => index - REMINDER_DAYS,
);

/** The "Ajustes" view of a Ciclo panel: label, the symbol of the marks,
 * the calendar they are in, how many days before the sensitive window
 * opens, the Windows notices, her reminders and "Remover do dock". Each
 * change is saved at once. */
function CycleSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"cycle">) {
  const t = useT();
  const text = t.widgets.cycle;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  const [symbol, setSymbol] = useState(config.symbol);
  const sources = useCalendar()?.snapshot?.sources ?? [];
  const [reminderText, setReminderText] = useState("");
  const [reminderAnchor, setReminderAnchor] =
    useState<ReminderAnchor>("sensitive");
  const [reminderDays, setReminderDays] = useState(-1);
  const [testSent, setTestSent] = useState<"sent" | "failed" | null>(null);

  // Characters as Rust counts them.
  const tooLong = [...reminderText.trim()].length > MAX_REMINDER_CHARS;

  function saveNotices(change: Partial<CycleNotices>) {
    settings.saveConfig({ notices: { ...config.notices, ...change } });
  }

  function addReminder() {
    const text = reminderText.trim();
    if (text === "" || tooLong || config.reminders.length >= MAX_REMINDERS)
      return;
    settings.saveConfig({
      reminders: [
        ...config.reminders,
        { text, anchor: reminderAnchor, days: reminderDays },
      ],
    });
    setReminderText("");
  }

  function removeReminder(index: number) {
    settings.saveConfig({
      reminders: config.reminders.filter((_, at) => at !== index),
    });
  }

  function sendTest() {
    cycleTestNotice().then(
      () => setTestSent("sent"),
      (error: unknown) => {
        log.warn(`cycle: test notice failed: ${errorKind(error)}`);
        setTestSent("failed");
      },
    );
  }

  function commitSymbol() {
    const trimmed = symbol.trim();
    if (trimmed === "" || trimmed === config.symbol) {
      setSymbol(config.symbol);
      return;
    }
    settings.saveConfig({ symbol: trimmed });
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.cycle}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <TextField
        label={text.symbol}
        value={symbol}
        invalid={settings.error === "invalidCycle"}
        onChange={setSymbol}
        onCommit={commitSymbol}
      />
      <label className="cycle-select">
        <span className="field-label">{text.calendar}</span>
        <select
          className="field-input"
          value={config.calendar ?? ""}
          onChange={(event) =>
            settings.saveConfig({ calendar: event.target.value || null })
          }
        >
          <option value="">{text.allCalendars}</option>
          {sources.map((source) => (
            <option key={source.id} value={source.id}>
              {source.name}
            </option>
          ))}
        </select>
      </label>
      <OptionGroup
        label={text.window}
        options={CYCLE_WINDOWS.map((value) => ({
          value: String(value),
          label: String(value),
        }))}
        selected={String(config.sensitive)}
        onSelect={(value) => settings.saveConfig({ sensitive: Number(value) })}
      />
      <p className="sys-hint">{text.windowHint}</p>
      <fieldset className="cycle-group">
        <legend>{text.noticesTitle}</legend>
        <ToggleField
          label={text.noticeToggles.sensitive}
          checked={config.notices.sensitive}
          onChange={(sensitive) => saveNotices({ sensitive })}
        />
        <ToggleField
          label={text.noticeToggles.eve}
          checked={config.notices.eve}
          onChange={(eve) => saveNotices({ eve })}
        />
        <ToggleField
          label={text.noticeToggles.day}
          checked={config.notices.day}
          onChange={(day) => saveNotices({ day })}
        />
        <p className="sys-hint">{text.noticesHint}</p>
        <div className="cycle-test">
          <button type="button" className="secondary-button" onClick={sendTest}>
            {text.testNotice}
          </button>
          {testSent ? (
            <span role="status" className="sys-hint">
              {testSent === "sent" ? text.testSent : text.testFailed}
            </span>
          ) : null}
        </div>
      </fieldset>
      <fieldset className="cycle-group">
        <legend>{text.remindersTitle}</legend>
        {config.reminders.length === 0 ? (
          <p className="sys-hint">{text.noReminders}</p>
        ) : (
          <ul className="cycle-reminders">
            {config.reminders.map((reminder, index) => (
              <li key={`${index}|${reminder.text}`}>
                <span className="cycle-reminder-text">{reminder.text}</span>
                <span className="cycle-reminder-when">
                  {whenText(t, reminder)}
                </span>
                <IconButton
                  icon="trash"
                  tone="danger"
                  label={format(text.removeReminder, { text: reminder.text })}
                  onClick={() => removeReminder(index)}
                />
              </li>
            ))}
          </ul>
        )}
        {config.reminders.length < MAX_REMINDERS ? (
          <>
            <TextField
              label={text.reminderText}
              placeholder={text.reminderPlaceholder}
              value={reminderText}
              invalid={tooLong}
              onChange={setReminderText}
            />
            <SelectField
              label={text.reminderDaysLabel}
              value={String(reminderDays)}
              options={DAY_OPTIONS.map((days) => ({
                value: String(days),
                label: daysText(t, days),
              }))}
              onChange={(value) => setReminderDays(Number(value))}
            />
            <SelectField
              label={text.reminderAnchorLabel}
              value={reminderAnchor}
              options={REMINDER_ANCHORS.map((anchor) => ({
                value: anchor,
                label: text.anchors[anchor],
              }))}
              onChange={(value) => setReminderAnchor(value as ReminderAnchor)}
            />
            <button
              type="button"
              className="secondary-button"
              disabled={reminderText.trim() === "" || tooLong}
              onClick={addReminder}
            >
              {text.addReminder}
            </button>
          </>
        ) : null}
        <p className="sys-hint">{text.remindersHint}</p>
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

export default CycleSettings;
