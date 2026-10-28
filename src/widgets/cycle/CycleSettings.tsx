import { useState } from "react";
import { useCalendar } from "../../core/calendar";
import { CYCLE_WINDOWS } from "../../core/widgets";
import { useT } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";
import "./cycle.css";

/** The "Ajustes" view of a Ciclo panel: label, the symbol of the marks,
 * the calendar they are in, how many days before the sensitive window
 * opens, and "Remover do dock". Each change is saved at once. */
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
