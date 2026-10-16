import { useEffect, useMemo, useRef, useState } from "react";
import {
  CLOCK_ICON_STYLES,
  HOUR_CYCLES,
  isWidgetCommandError,
  type ClockConfig,
  type ClockIconStyle,
  type HourCycle,
  type WidgetErrorCode,
  type WidgetInstance,
} from "../../core/widgets";
import { format, useT, type Messages } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import SelectField, { type SelectOption } from "../../panel/parts/SelectField";
import TextField from "../../panel/parts/TextField";
import { labelFromDraft } from "../../panel/parts/label";
import type { WidgetSettingsProps, WidgetUpdate } from "../contract";
import "./clock.css";
import { systemTimeZone } from "./format";
import { ianaTimeZones, zoneCity } from "./zones";

/** The zone list's value for "follow the system" (`timeZone: null`): no
 * IANA name is empty. */
const SYSTEM_ZONE = "";

type SecondsOption = "hide" | "show";

type Update = WidgetUpdate<"clock">;

/** An instance's label and config as one comparable value: the host may
 * hand over an equal copy. */
function keyOf(update: Update): string {
  return JSON.stringify({ label: update.label, config: update.config });
}

const updateOf = (instance: WidgetInstance<"clock">): Update => ({
  label: instance.label,
  config: instance.config,
});

/** The error code of a failed command; a rejection without a code (never
 * expected) reads as a failed save. */
function errorCode(error: unknown): WidgetErrorCode {
  return isWidgetCommandError(error) ? error.code : "writeFailed";
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

  /** The last change sent, shown in place of `instance` until `instance`
   * equals it (`widgets-changed` and the command's answer come in no set
   * order) or it is refused. `settled`: the command answered; the next new
   * instance replaces it even if it differs (another window won). */
  const [pending, setPending] = useState<{
    update: Update;
    settled: boolean;
  } | null>(null);
  /** The label as typed, until it is committed; null shows the label of
   * `pending` or `instance`. */
  const [labelDraft, setLabelDraft] = useState<string | null>(null);
  const [error, setError] = useState<WidgetErrorCode | null>(null);
  const [removing, setRemoving] = useState(false);
  /** Number of the last change sent: only its outcome counts. */
  const sent = useRef(0);

  const acceptedKey = keyOf(updateOf(instance));
  const [syncedKey, setSyncedKey] = useState(acceptedKey);
  if (syncedKey !== acceptedKey) {
    setSyncedKey(acceptedKey);
    if (
      pending !== null &&
      (pending.settled || keyOf(pending.update) === acceptedKey)
    ) {
      setPending(null);
    }
  }
  // The last accepted instance, for an answer that settles after it came.
  const latestKey = useRef(acceptedKey);
  useEffect(() => {
    latestKey.current = acceptedKey;
  });

  const current: Update = pending?.update ?? updateOf(instance);
  const config = current.config;
  const zones = useMemo(
    () => zoneOptions(t, config.timeZone),
    [t, config.timeZone],
  );
  const labelText = labelDraft ?? current.label ?? "";

  function send(update: Update) {
    const id = ++sent.current;
    setPending({ update, settled: false });
    setError(null);
    onUpdate(update).then(
      () => {
        if (sent.current !== id) return;
        // Already shown by `instance`, or waiting for it.
        setPending((shown) =>
          shown === null || keyOf(shown.update) === latestKey.current
            ? null
            : { ...shown, settled: true },
        );
      },
      (reason: unknown) => {
        if (sent.current !== id) return;
        // Every field shows `instance` again, the label too.
        setPending(null);
        setError(errorCode(reason));
      },
    );
  }

  function changeConfig(change: Partial<ClockConfig>) {
    send({ label: current.label, config: { ...config, ...change } });
  }

  function commitLabel() {
    if (labelDraft === null) return;
    const label = labelFromDraft(labelDraft);
    setLabelDraft(null);
    if (label === "tooLong") {
      setError("invalidLabel");
      return;
    }
    if (label !== current.label) send({ label, config });
  }

  function remove() {
    setRemoving(true);
    setError(null);
    onRemove().catch((reason: unknown) => {
      setRemoving(false);
      setError(errorCode(reason));
    });
  }

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        value={labelText}
        placeholder={t.widgets.kinds.clock}
        invalid={error === "invalidLabel"}
        onChange={setLabelDraft}
        onCommit={commitLabel}
      />
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
      <PanelFooter alert={error ? t.widgets.errors[error] : null}>
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default ClockSettings;
