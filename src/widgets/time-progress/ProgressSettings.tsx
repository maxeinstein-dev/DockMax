import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import {
  isWidgetCommandError,
  PERIODS,
  WEEK_STARTS,
  type MoveDirection,
  type Period,
  type WidgetErrorCode,
  type WidgetInstance,
  type Workday,
} from "../../core/widgets";
import { format, useT } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import IconButton from "../../panel/parts/IconButton";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import { labelFromDraft } from "../../panel/parts/label";
import type { WidgetSettingsProps, WidgetUpdate } from "../contract";
import { WidthGroup } from "../system/parts";
import "./time-progress.css";

type Update = WidgetUpdate<"time-progress">;

/** The label and config of an instance, as `update_widget` takes them. */
function updateOf(instance: WidgetInstance<"time-progress">): Update {
  return { label: instance.label, config: instance.config };
}

/** The error code of a refused command; anything else counts as a failed
 * write (the contract rejects only with a `WidgetCommandError`). */
function errorCode(error: unknown): WidgetErrorCode {
  return isWidgetCommandError(error) ? error.code : "writeFailed";
}

const OPPOSITE: Record<MoveDirection, MoveDirection> = {
  up: "down",
  down: "up",
};

/** The "Ajustes" view of a time-progress panel (DESIGN.md "Painel":
 * Ajustes do progresso): label, the periods (checkboxes in the panel's
 * order, with up and down, and the workday hours on their own line while
 * it is checked), the week's first day and "Remover do dock".
 *
 * Every change is saved at once with `onUpdate`, label and config in
 * full. The view shows it right away; a refusal shows the error's message
 * in the footer and puts every field back to `instance`, the last accepted
 * one. Text fields (label, hours) save when they are done: on blur or
 * Enter. */
function ProgressSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"time-progress">) {
  const t = useT();
  const text = t.widgets.progress;
  const periodsLabelId = useId();

  /** What the view shows: the last change sent, or the instance. */
  const [shown, setShown] = useState<Update>(() => updateOf(instance));
  const [labelDraft, setLabelDraft] = useState(instance.label ?? "");
  const [workdayDraft, setWorkdayDraft] = useState<Workday>(
    instance.config.workday,
  );
  const [error, setError] = useState<WidgetErrorCode | null>(null);
  const [removing, setRemoving] = useState(false);

  // A new accepted instance (this view's change, or another window's)
  // replaces what is shown. Compared by value: the host may hand over an
  // equal copy.
  const acceptedKey = JSON.stringify(updateOf(instance));
  const [syncedKey, setSyncedKey] = useState(acceptedKey);
  if (syncedKey !== acceptedKey) {
    setSyncedKey(acceptedKey);
    showInstance(instance);
  }

  // The last accepted instance, for a refusal that settles after it changed.
  const latest = useRef(instance);
  useEffect(() => {
    latest.current = instance;
  });

  function showInstance(accepted: WidgetInstance<"time-progress">) {
    setShown(updateOf(accepted));
    setLabelDraft(accepted.label ?? "");
    setWorkdayDraft(accepted.config.workday);
  }

  async function save(next: Update) {
    setError(null);
    setShown(next);
    try {
      await onUpdate(next);
    } catch (refusal) {
      showInstance(latest.current);
      setError(errorCode(refusal));
    }
  }

  function commitLabel() {
    const label = labelFromDraft(labelDraft);
    if (label === "tooLong") {
      setLabelDraft(shown.label ?? "");
      setError("invalidLabel");
      return;
    }
    setLabelDraft(label ?? "");
    if (label !== shown.label) void save({ ...shown, label });
  }

  function commitHour(field: keyof Workday) {
    const saved = shown.config.workday;
    const value = workdayDraft[field];
    if (value === "") {
      // A cleared time field: nothing to save, the hour comes back.
      setWorkdayDraft({ ...workdayDraft, [field]: saved[field] });
      return;
    }
    if (value === saved[field]) return;
    void save({
      ...shown,
      config: { ...shown.config, workday: { ...saved, [field]: value } },
    });
  }

  const periods = shown.config.periods;
  /** Checked periods in the panel's order, then the others. */
  const rows = [...periods, ...PERIODS.filter((p) => !periods.includes(p))];

  function setPeriods(next: Period[]) {
    void save({ ...shown, config: { ...shown.config, periods: next } });
  }

  function toggle(period: Period, checked: boolean) {
    const next = checked
      ? [...periods, period]
      : periods.filter((p) => p !== period);
    // At least one period; the last checkbox is also disabled.
    if (next.length > 0) setPeriods(next);
  }

  // The up/down buttons, for the focus after a move: a period that reaches
  // an end turns its button off, so the focus moves to the other one.
  const moveButtons = useRef(new Map<string, HTMLButtonElement>());
  const focusAfterMove = useRef<{
    period: Period;
    direction: MoveDirection;
  } | null>(null);
  useLayoutEffect(() => {
    const target = focusAfterMove.current;
    if (!target) return;
    focusAfterMove.current = null;
    const buttons = moveButtons.current;
    const same = buttons.get(`${target.period}:${target.direction}`);
    const other = buttons.get(`${target.period}:${OPPOSITE[target.direction]}`);
    (same && !same.disabled ? same : other)?.focus({ preventScroll: true });
  });

  function move(period: Period, direction: MoveDirection) {
    const from = periods.indexOf(period);
    const to = direction === "up" ? from - 1 : from + 1;
    if (from < 0 || to < 0 || to >= periods.length) return;
    const next = [...periods];
    next.splice(to, 0, ...next.splice(from, 1));
    focusAfterMove.current = { period, direction };
    setPeriods(next);
  }

  function moveButton(period: Period, direction: MoveDirection) {
    const index = periods.indexOf(period);
    const atEnd = direction === "up" ? index <= 0 : index >= periods.length - 1;
    const key = `${period}:${direction}`;
    return (
      <IconButton
        ref={(button) => {
          if (button) moveButtons.current.set(key, button);
          else moveButtons.current.delete(key);
        }}
        icon={direction}
        label={format(
          direction === "up"
            ? t.widgets.common.moveUp
            : t.widgets.common.moveDown,
          { name: text.periods[period] },
        )}
        disabled={index < 0 || atEnd}
        onClick={() => move(period, direction)}
      />
    );
  }

  async function remove() {
    setError(null);
    setRemoving(true);
    try {
      await onRemove();
    } catch (refusal) {
      setRemoving(false);
      setError(errorCode(refusal));
    }
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds["time-progress"]}
        value={labelDraft}
        invalid={error === "invalidLabel"}
        onChange={setLabelDraft}
        onCommit={commitLabel}
      />
      <div className="period-field">
        <span id={periodsLabelId} className="period-field-label">
          {text.periodsLabel}
        </span>
        <ul className="periods" aria-labelledby={periodsLabelId}>
          {rows.map((period) => {
            const checked = periods.includes(period);
            return [
              <li key={period}>
                <label>
                  <input
                    type="checkbox"
                    checked={checked}
                    disabled={checked && periods.length === 1}
                    onChange={(event) => toggle(period, event.target.checked)}
                  />
                  <span className="period-name">{text.periods[period]}</span>
                  {period === "workday" ? (
                    <>
                      {" "}
                      <span className="period-hint">{text.workdayDays}</span>
                    </>
                  ) : null}
                </label>
                {moveButton(period, "up")}
                {moveButton(period, "down")}
              </li>,
              period === "workday" && checked ? (
                <li key="workday-hours" className="period-hours">
                  <span>{text.workdayFrom}</span>
                  <TextField
                    label={text.workdayStart}
                    hideLabel
                    type="time"
                    value={workdayDraft.start}
                    invalid={error === "invalidWorkday"}
                    onChange={(start) =>
                      setWorkdayDraft({ ...workdayDraft, start })
                    }
                    onCommit={() => commitHour("start")}
                  />
                  <span>{text.workdayTo}</span>
                  <TextField
                    label={text.workdayEnd}
                    hideLabel
                    type="time"
                    value={workdayDraft.end}
                    invalid={error === "invalidWorkday"}
                    onChange={(end) =>
                      setWorkdayDraft({ ...workdayDraft, end })
                    }
                    onCommit={() => commitHour("end")}
                  />
                </li>
              ) : null,
            ];
          })}
        </ul>
        <p className="progress-hint">{text.ringHint}</p>
      </div>
      <OptionGroup
        label={text.weekStart}
        options={WEEK_STARTS.map((value) => ({
          value,
          label: text.weekStarts[value],
        }))}
        selected={shown.config.weekStart}
        onSelect={(weekStart) =>
          void save({ ...shown, config: { ...shown.config, weekStart } })
        }
      />
      <WidthGroup
        width={shown.config.width}
        onSelect={(width) =>
          void save({ ...shown, config: { ...shown.config, width } })
        }
      />
      <p className="progress-hint">{text.widthHint}</p>
      <RemoveButton onClick={() => void remove()} disabled={removing} />
      <PanelFooter alert={error ? t.widgets.errors[error] : null}>
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default ProgressSettings;
