import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";
import "./todo.css";

/** The "Ajustes" view of a Lista de tarefas panel: label, "Limpar
 * feitas" and "Remover do dock". Each change is saved at once. */
function TodoSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"todo">) {
  const t = useT();
  const text = t.widgets.todo;
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { items } = settings.shown.config;
  const done = items.filter((item) => item.done).length;
  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.todo}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <button
        type="button"
        className="secondary-button todo-clear"
        disabled={done === 0}
        onClick={() =>
          settings.saveConfig({ items: items.filter((item) => !item.done) })
        }
      >
        {text.clearDone}
      </button>
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

export default TodoSettings;
