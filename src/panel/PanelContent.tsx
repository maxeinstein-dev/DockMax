import { isWidgetPanel, type PanelId } from "../core/panels";
import type { Settings } from "../core/settings";
import {
  removeWidget,
  updateWidget,
  type CreatableKind,
  type WidgetConfig,
  type WidgetInstance,
  type WidgetsSnapshot,
} from "../core/widgets";
import type { WidgetUpdate } from "../widgets/contract";
import { SECOND_MS, useNow } from "../widgets/now";
import { definitionOf } from "../widgets/registry";
import AboutPanel from "./AboutPanel";
import DockItems from "./DockItems";
import SettingsPanel from "./SettingsPanel";
import { BACK_VIEW, SUB_VIEW, type PanelView } from "./view";

type PanelContentProps = {
  panelId: PanelId;
  /** The settings in use, or null while they can't be read. */
  settings: Settings | null;
  /** The widgets (`useWidgets`), or null while loading or unreadable. */
  widgets: WidgetsSnapshot | null;
  /** The instance of a widget panel, or null when the list doesn't have
   * it (then the panel shows nothing and the host closes it). */
  instance: WidgetInstance<CreatableKind> | null;
  view: PanelView;
  onView: (view: PanelView) => void;
  /** The instance was removed from its Ajustes: the host closes the
   * panel. */
  onRemoved: () => void;
};

/** A widget's main view, with the panel's timer: its `now` moves every
 * second while the view shows, and starts fresh with each view. */
function WidgetMain({
  instance,
  returned,
  onAdjust,
}: {
  instance: WidgetInstance<CreatableKind>;
  returned: boolean;
  onAdjust: () => void;
}) {
  const now = useNow(SECOND_MS);
  const { Panel } = definitionOf(instance);
  return (
    <Panel
      instance={instance}
      now={now}
      onAdjust={onAdjust}
      returnedFromSettings={returned}
    />
  );
}

/** A widget's Ajustes view, saving through `update_widget` and removing
 * through `remove_widget`. */
function WidgetAdjust({
  instance,
  onBack,
  onRemoved,
}: {
  instance: WidgetInstance<CreatableKind>;
  onBack: () => void;
  onRemoved: () => void;
}) {
  const { Settings } = definitionOf(instance);
  const onUpdate = async ({ label, config }: WidgetUpdate<CreatableKind>) => {
    // Sound: the view of a kind sends that kind's config.
    const tagged = { kind: instance.kind, config } as WidgetConfig;
    await updateWidget({ id: instance.id, label, config: tagged });
  };
  const onRemove = async () => {
    await removeWidget({ id: instance.id });
    onRemoved();
  };
  return (
    <Settings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={onRemove}
      onBack={onBack}
    />
  );
}

/** The body of a panel, below the title of a system panel (a widget's
 * views and Itens do dock bring their own header). Runs inside the panel's
 * error boundary: a failure here leaves Esc working. */
function PanelContent({
  panelId,
  settings,
  widgets,
  instance,
  view,
  onView,
  onRemoved,
}: PanelContentProps) {
  if (isWidgetPanel(panelId)) {
    if (instance === null) return null;
    return view.sub ? (
      <WidgetAdjust
        instance={instance}
        onBack={() => onView(BACK_VIEW)}
        onRemoved={onRemoved}
      />
    ) : (
      <WidgetMain
        instance={instance}
        returned={view.returned}
        onAdjust={() => onView(SUB_VIEW)}
      />
    );
  }
  switch (panelId) {
    case "settings":
      return view.sub ? (
        <DockItems snapshot={widgets} onBack={() => onView(BACK_VIEW)} />
      ) : (
        <SettingsPanel
          settings={settings}
          widgets={widgets}
          onOpenItems={() => onView(SUB_VIEW)}
          focusItemsRow={view.returned}
        />
      );
    case "about":
      return <AboutPanel />;
  }
}

export default PanelContent;
