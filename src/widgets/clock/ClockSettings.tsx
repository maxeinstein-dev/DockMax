import { useId, useMemo, useRef, useState } from "react";
import {
  CLOCK_ICON_STYLES,
  HOUR_CYCLES,
  MAX_LABEL_CHARS,
  isWidgetCommandError,
  labelLength,
  type ClockConfig,
  type ClockIconStyle,
  type HourCycle,
} from "../../core/widgets";
import { format, useT, type Messages } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import SelectField, { type SelectOption } from "../../panel/parts/SelectField";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps, WidgetUpdate } from "../contract";
import "./clock.css";
import { systemTimeZone } from "./format";
import { ianaTimeZones, zoneCity } from "./zones";

/** The zone list's value for "follow the system" (`timeZone: null`): no
 * IANA name is empty. */
const SYSTEM_ZONE = "";

type SecondsOption = "hide" | "show";

/** The message of a failed command; a rejection without a code (never
 * expected) reads as a failed save. */
function errorMessage(t: Messages, error: unknown): string {
  return isWidgetCommandError(error)
    ? t.widgets.errors[error.code]
    : t.widgets.errors.writeFailed;
}

/** The zone list: the system zone first, then `Intl`'s IANA names as
 * written, plus a configured zone `Intl` doesn't list (Rust accepted it),
 * so the list always shows the clock's own value. */
function zoneOptions(t: Messages, configured: string | null): SelectOption[] {
  const text = t.widgets.clock;
  const system = systemTimeZone();
  const options: SelectOption[] = [
    {
      value: SYSTEM_ZONE,
      label:
        system === null
          ? text.systemZoneOptionUnknown
          : format(text.systemZoneOption, { zone: zoneCity(system) }),
    },
  ];
  const zones = ianaTimeZones();
  if (configured !== null && !zones.includes(configured)) {
    options.push({
      value: configured,
      label: format(text.zoneOption, {
        city: zoneCity(configured),
        zone: configured,
      }),
    });
  }
  for (const zone of zones) {
    options.push({
      value: zone,
      label: format(text.zoneOption, { city: zoneCity(zone), zone }),
    });
  }
  return options;
}

/** The "Ajustes" view of a clock panel (DESIGN.md "Painel": Ajustes do
 * relógio). Every change is sent at once, label and config in full,
 * built on the changes still on their way; a refused one brings every
 * field back to `instance` and shows its message in the footer. */
function ClockSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"clock">) {
  const t = useT();
  const text = t.widgets.clock;
  const labelErrorId = useId();

  /** The last change sent and not settled yet: shown in place of
   * `instance` until Rust answers. */
  const [pending, setPending] = useState<WidgetUpdate<"clock"> | null>(null);
  /** The label as typed, until it is committed; null shows the label of
   * `pending` or `instance`. */
  const [labelDraft, setLabelDraft] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [removing, setRemoving] = useState(false);
  /** Number of the last change sent: only its outcome counts. */
  const sent = useRef(0);
  const current: WidgetUpdate<"clock"> = pending ?? {
    label: instance.label,
    config: instance.config,
  };
  const config = current.config;
  const zones = useMemo(
    () => zoneOptions(t, config.timeZone),
    [t, config.timeZone],
  );
  const labelText = labelDraft ?? current.label ?? "";
  const labelTooLong = labelLength(labelText) > MAX_LABEL_CHARS;

  function send(update: WidgetUpdate<"clock">) {
    const id = ++sent.current;
    setPending(update);
    setError(null);
    onUpdate(update).then(
      () => {
        if (sent.current === id) setPending(null);
      },
      (reason: unknown) => {
        if (sent.current !== id) return;
        // Every field shows `instance` again, the label too.
        setPending(null);
        setError(errorMessage(t, reason));
      },
    );
  }

  function changeConfig(change: Partial<ClockConfig>) {
    send({ label: current.label, config: { ...config, ...change } });
  }

  function commitLabel() {
    if (labelDraft === null || labelTooLong) return;
    const label = labelDraft.trim() === "" ? null : labelDraft;
    setLabelDraft(null);
    if (label !== current.label) send({ label, config });
  }

  function remove() {
    setRemoving(true);
    setError(null);
    onRemove().catch((reason: unknown) => {
      setRemoving(false);
      setError(errorMessage(t, reason));
    });
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        value={labelText}
        placeholder={t.widgets.kinds.clock}
        invalid={labelTooLong}
        describedBy={labelTooLong ? labelErrorId : undefined}
        onChange={setLabelDraft}
        onCommit={commitLabel}
      />
      {labelTooLong ? (
        <p id={labelErrorId} className="clock-field-error">
          {t.widgets.errors.invalidLabel}
        </p>
      ) : null}
      <SelectField
        label={text.timeZone}
        value={config.timeZone ?? SYSTEM_ZONE}
        options={zones}
        onChange={(value) =>
          changeConfig({ timeZone: value === SYSTEM_ZONE ? null : value })
        }
      />
      <OptionGroup<HourCycle>
        label={text.hourCycle}
        options={HOUR_CYCLES.map((value) => ({
          value,
          label: text.hourCycles[value],
        }))}
        selected={config.hourCycle}
        onSelect={(hourCycle) => changeConfig({ hourCycle })}
      />
      <OptionGroup<SecondsOption>
        label={text.seconds}
        options={(["hide", "show"] as const).map((value) => ({
          value,
          label: text.secondsOptions[value],
        }))}
        selected={config.showSeconds ? "show" : "hide"}
        onSelect={(value) => changeConfig({ showSeconds: value === "show" })}
      />
      <OptionGroup<ClockIconStyle>
        label={text.iconStyle}
        options={CLOCK_ICON_STYLES.map((value) => ({
          value,
          label: text.iconStyles[value],
        }))}
        selected={config.iconStyle}
        onSelect={(iconStyle) => changeConfig({ iconStyle })}
      />
      <RemoveButton onClick={remove} disabled={removing} />
      <PanelFooter alert={error}>{t.widgets.common.footer}</PanelFooter>
    </>
  );
}

export default ClockSettings;
