import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetSettingsProps } from "../contract";

/** PROVISIONAL (W3 replaces it): the "Ajustes" view of a note panel,
 * with only its header. No "Remover do dock" yet, so a note with text is never removed without the confirmation. */
function NoteSettings({ onBack }: WidgetSettingsProps<"note">) {
  const t = useT();
  return (
    <PanelHeader
      title={t.widgets.note.settingsTitle}
      onBack={onBack}
      focusAction
    />
  );
}

export default NoteSettings;
