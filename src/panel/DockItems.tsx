import type { WidgetsSnapshot } from "../core/widgets";
import { useT } from "../i18n";
import PanelHeader from "./parts/PanelHeader";

/** Props of Configurações › Itens do dock (W4), a subview of the Settings
 * panel. It changes the list through `addWidget`, `moveWidget` and
 * `dismissStorageNotice` (`src/core/widgets.ts`); every accepted change
 * comes back in `snapshot` through `widgets-changed`, and a rejected one
 * shows its message (`t.widgets.errors[code]`). */
export interface DockItemsProps {
  /** The instances in dock order and the storage state (`useWidgets`);
   * null while loading, and for good when they can't be read. */
  snapshot: WidgetsSnapshot | null;
  /** Voltar: back to Configurações, where the "Itens do dock" row takes the
   * focus (`NavRow` `focusOnMount`). The view mounts with the focus on
   * Voltar. */
  onBack: () => void;
}

/** PROVISIONAL (W4 replaces it): the subview with only its header. */
function DockItems({ onBack }: DockItemsProps) {
  const t = useT();
  return (
    <PanelHeader title={t.widgets.items.title} onBack={onBack} focusAction />
  );
}

export default DockItems;
