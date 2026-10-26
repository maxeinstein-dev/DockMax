import type { AppsSnapshot, PinnedApp } from "../core/apps";
import {
  isAppPanel,
  isOpenPanel,
  isWidgetPanel,
  type PanelId,
} from "../core/panels";
import type { OpenNow, OpenOther } from "../core/openApps";
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
import AppMenuPanel from "./AppMenuPanel";
import AppsPanel from "./AppsPanel";
import OpenMenuPanel from "./OpenMenuPanel";
import DockItems from "./DockItems";
import SettingsPanel from "./SettingsPanel";
import TrayPanel from "./TrayPanel";
import LookPanel from "./LookPanel";
import {
  APPS_VIEW,
  BACK_FROM_APPS_VIEW,
  BACK_FROM_LOOK_VIEW,
  BACK_VIEW,
  LOOK_VIEW,
  SUB_VIEW,
  type PanelView,
} from "./view";

type PanelContentProps = {
  panelId: PanelId;
  /** The settings in use, or null while they can't be read. */
  settings: Settings | null;
  /** The widgets (`useWidgets`), or null while loading or unreadable. */
  widgets: WidgetsSnapshot | null;
  /** The pinned programs (`useApps`), or null while loading or
   * unreadable. */
  apps: AppsSnapshot | null;
  /** The pinned program of an app panel, or null when the list doesn't
   * have it (then the panel shows nothing and the host closes it). */
  app: PinnedApp | null;
  /** Who is open (`useOpenApps`): the menu of a pinned program offers to close
   * its windows while it has some. */
  open: OpenNow;
  /** The open program that is not pinned of an open-program panel, or null
   * when it is not open any more (then the panel shows nothing and the host
   * closes it). */
  other: OpenOther | null;
  /** The instance of a widget panel, or null when the list doesn't have
   * it (then the panel shows nothing and the host closes it). */
  instance: WidgetInstance<CreatableKind> | null;
  view: PanelView;
  onView: (view: PanelView) => void;
  /** The instance was removed from its Ajustes, or an entry of a
   * program's menu was done: the host closes the panel. */
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
  apps,
  app,
  open,
  other,
  instance,
  view,
  onView,
  onRemoved,
}: PanelContentProps) {
  if (isAppPanel(panelId)) {
    return app === null ? null : (
      <AppMenuPanel
        app={app}
        openCount={open.apps.get(app.id)?.count ?? 0}
        onDone={onRemoved}
      />
    );
  }
  if (isOpenPanel(panelId)) {
    return other === null ? null : (
      <OpenMenuPanel other={other} onDone={onRemoved} />
    );
  }
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
      if (view.sub === "items") {
        return (
          <DockItems snapshot={widgets} onBack={() => onView(BACK_VIEW)} />
        );
      }
      if (view.sub === "apps") {
        return (
          <AppsPanel
            snapshot={apps}
            onBack={() => onView(BACK_FROM_APPS_VIEW)}
          />
        );
      }
      if (view.sub === "look") {
        return (
          <LookPanel
            settings={settings}
            onBack={() => onView(BACK_FROM_LOOK_VIEW)}
          />
        );
      }
      return (
        <SettingsPanel
          settings={settings}
          widgets={widgets}
          apps={apps}
          onOpenItems={() => onView(SUB_VIEW)}
          onOpenApps={() => onView(APPS_VIEW)}
          onOpenLook={() => onView(LOOK_VIEW)}
          focusItemsRow={view.returned && view.from === "items"}
          focusLookRow={view.returned && view.from === "look"}
          focusAppsRow={view.returned && view.from === "apps"}
        />
      );
    case "about":
      return <AboutPanel />;
    case "tray":
      return <TrayPanel inDock={settings?.dock.trayInDock ?? []} />;
  }
}

export default PanelContent;
