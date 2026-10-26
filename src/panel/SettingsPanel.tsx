import {
  MAX_AUTO_HIDE_DELAY,
  MAX_AUTO_HIDE_REVEAL,
  SIZE_PRESETS,
  type DockEdge,
  type DockOpacity,
  type MiddleClick,
  type Settings,
  type SizePreset,
  type TaskbarMode,
  type Theme,
} from "../core/settings";
import { MAX_APPS, type AppsSnapshot } from "../core/apps";
import { MAX_WIDGETS, type WidgetsSnapshot } from "../core/widgets";
import { format, useT, type Messages } from "../i18n";
import OptionGroup, { type Option } from "./OptionGroup";
import PanelFooter from "./PanelFooter";
import NavRow from "./parts/NavRow";
import RangeField from "./parts/RangeField";
import ToggleField from "./parts/ToggleField";
import TrayInDockField from "./TrayInDockField";
import { useSettingsEditor } from "./useSettingsEditor";

const THEMES: readonly Theme[] = ["system", "light", "dark"];
const EDGES: readonly DockEdge[] = ["bottom", "top"];
const SIZES: readonly SizePreset[] = ["small", "medium", "large"];
const OPACITIES: readonly DockOpacity[] = ["translucent", "solid"];
const TASKBARS: readonly TaskbarMode[] = ["visible", "autohide", "hidden"];
const MIDDLE_CLICKS: readonly MiddleClick[] = ["newInstance", "none"];

/** Milliseconds as seconds, the Brazilian way: `0,3 s`. */
function seconds(ms: number): string {
  return `${(ms / 1000).toLocaleString("pt-BR", { maximumFractionDigits: 2 })} s`;
}

/** The preset whose icon size is `iconSize`, or null when none is (a size
 * set by hand in the file): then no size option shows as chosen. */
function sizePresetOf(iconSize: number): SizePreset | null {
  return SIZES.find((preset) => SIZE_PRESETS[preset] === iconSize) ?? null;
}

function options<V extends string>(
  values: readonly V[],
  labels: Readonly<Record<V, string>>,
): Option<V>[] {
  return values.map((value) => ({ value, label: labels[value] }));
}

/** The value of the "Itens do dock" row: `3 de 12`, or `Indisponível`
 * while the widgets can't be used or read. */
function itemsValue(t: Messages, widgets: WidgetsSnapshot | null): string {
  if (widgets === null || widgets.storage.state === "unavailable") {
    return t.widgets.items.unavailable;
  }
  return format(t.widgets.items.count, {
    count: widgets.widgets.length,
    max: MAX_WIDGETS,
  });
}

/** The value of the "Apps do dock" row: `3 de 40`, or `Indisponível`
 * while the apps can't be used or read. */
function appsValue(t: Messages, apps: AppsSnapshot | null): string {
  if (apps === null || apps.storage.state === "unavailable") {
    return t.apps.unavailable;
  }
  return format(t.apps.count, { count: apps.apps.length, max: MAX_APPS });
}

type SettingsPanelProps = {
  /** The settings in use (`useSettings`), or null while they can't be
   * read. The only source of the confirmed values. */
  settings: Settings | null;
  /** The widgets and the storage state (`useWidgets`), for the count and
   * the warning dot of the "Itens do dock" row; null (the default) while
   * loading or unreadable: the row says `Indisponível`, unmarked. */
  widgets?: WidgetsSnapshot | null;
  /** Opens Configurações › Itens do dock (`DockItems`): the panel host
   * swaps this view for the subview. Without it there is no row. */
  onOpenItems?: () => void;
  /** The pinned programs (`useApps`), for the count of the "Apps do dock"
   * row; null (the default) while loading or unreadable. */
  apps?: AppsSnapshot | null;
  /** Opens Configurações › Apps do dock (`AppsPanel`). Without it there
   * is no row. */
  onOpenApps?: () => void;
  /** Opens Configurações › Aparência do dock (`LookPanel`). Without it
   * there is no row. */
  onOpenLook?: () => void;
  /** Focuses the "Itens do dock" row on mount: set it when coming back
   * from the subview (its Voltar). */
  focusItemsRow?: boolean;
  /** The same for the "Aparência do dock" row. */
  focusLookRow?: boolean;
  /** The same for the "Apps do dock" row. */
  focusAppsRow?: boolean;
  /** Shows which tray icons the dock shows itself (`TrayInDockField`), with
   * the taskbar hidden: the Configurações app's, which reads the tray. */
  withTrayInDock?: boolean;
};

/**
 * The Settings panel: one option group per setting, applied and saved on
 * every choice (spec 0002 §2.3 item 4, §2.4 item 3). A choice shows at once
 * while Rust saves it; if Rust refuses, the translated reason shows in the
 * footer and the options go back to the confirmed settings.
 */
function SettingsPanel({
  settings,
  widgets = null,
  apps = null,
  onOpenItems,
  onOpenApps,
  onOpenLook,
  focusItemsRow = false,
  focusLookRow = false,
  focusAppsRow = false,
  withTrayInDock = false,
}: SettingsPanelProps) {
  const t = useT();
  const { shown, change, error } = useSettingsEditor(settings);
  const theme = shown?.theme ?? null;
  const edge = shown?.dock.edge ?? null;
  const size = shown ? sizePresetOf(shown.dock.iconSize) : null;
  const opacity = shown?.dock.opacity ?? null;
  const taskbar = shown?.dock.taskbar ?? null;

  return (
    <>
      <OptionGroup
        label={t.settings.groups.theme}
        options={options(THEMES, t.settings.theme)}
        selected={theme}
        onSelect={(value) => change((s) => ({ ...s, theme: value }))}
      />
      <OptionGroup
        label={t.settings.groups.edge}
        options={options(EDGES, t.settings.edge)}
        selected={edge}
        onSelect={(value) =>
          change((s) => ({
            ...s,
            dock: { ...s.dock, edge: value },
          }))
        }
      />
      <OptionGroup
        label={t.settings.groups.size}
        options={options(SIZES, t.settings.size)}
        selected={size}
        onSelect={(value) =>
          change((s) => ({
            ...s,
            dock: { ...s.dock, iconSize: SIZE_PRESETS[value] },
          }))
        }
      />
      <OptionGroup
        label={t.settings.groups.opacity}
        options={options(OPACITIES, t.settings.opacity)}
        selected={opacity}
        onSelect={(value) =>
          change((s) => ({
            ...s,
            dock: { ...s.dock, opacity: value },
          }))
        }
      />
      <OptionGroup
        label={t.settings.groups.taskbar}
        options={options(TASKBARS, t.settings.taskbar)}
        selected={taskbar}
        onSelect={(value) =>
          change((s) => ({
            ...s,
            dock: { ...s.dock, taskbar: value },
          }))
        }
      />
      {/* The dock holds the tray only with the taskbar hidden altogether. */}
      {withTrayInDock && shown !== null && taskbar === "hidden" ? (
        <TrayInDockField
          inDock={shown.dock.trayInDock ?? []}
          onChange={(trayInDock) =>
            change((s) => ({ ...s, dock: { ...s.dock, trayInDock } }))
          }
        />
      ) : null}
      <ToggleField
        label={t.settings.everyMonitor}
        hint={t.settings.everyMonitorHint}
        checked={shown?.dock.everyMonitor ?? true}
        disabled={shown === null}
        onChange={(everyMonitor) =>
          change((s) => ({ ...s, dock: { ...s.dock, everyMonitor } }))
        }
      />
      <ToggleField
        label={t.settings.autoHide}
        hint={t.settings.autoHideHint}
        checked={shown?.dock.autoHide ?? false}
        disabled={shown === null}
        onChange={(autoHide) =>
          change((s) => ({ ...s, dock: { ...s.dock, autoHide } }))
        }
      />
      {shown?.dock.autoHide ? (
        <>
          <RangeField
            label={t.settings.autoHideReveal}
            hint={t.settings.autoHideRevealHint}
            value={shown.dock.autoHideReveal}
            min={0}
            max={MAX_AUTO_HIDE_REVEAL}
            step={50}
            display={seconds}
            onCommit={(autoHideReveal) =>
              change((s) => ({ ...s, dock: { ...s.dock, autoHideReveal } }))
            }
          />
          <RangeField
            label={t.settings.autoHideDelay}
            hint={t.settings.autoHideDelayHint}
            value={shown.dock.autoHideDelay}
            min={0}
            max={MAX_AUTO_HIDE_DELAY}
            step={100}
            display={seconds}
            onCommit={(autoHideDelay) =>
              change((s) => ({ ...s, dock: { ...s.dock, autoHideDelay } }))
            }
          />
        </>
      ) : null}
      <ToggleField
        label={t.settings.showOpenApps}
        hint={t.settings.showOpenAppsHint}
        checked={shown?.dock.showOpenApps ?? true}
        disabled={shown === null}
        onChange={(showOpenApps) =>
          change((s) => ({ ...s, dock: { ...s.dock, showOpenApps } }))
        }
      />
      <ToggleField
        label={t.settings.launchAtLogin}
        hint={t.settings.launchAtLoginHint}
        checked={shown?.launchAtLogin ?? false}
        disabled={shown === null}
        onChange={(launchAtLogin) => change((s) => ({ ...s, launchAtLogin }))}
      />
      <OptionGroup
        label={t.settings.groups.middleClick}
        options={options(MIDDLE_CLICKS, t.settings.middleClick)}
        selected={shown?.dock.middleClick ?? null}
        onSelect={(value) =>
          change((s) => ({ ...s, dock: { ...s.dock, middleClick: value } }))
        }
      />
      {onOpenLook ? (
        <NavRow
          label={t.settings.look.title}
          value={shown ? t.settings.look.styles[shown.dock.style] : ""}
          onClick={onOpenLook}
          focusOnMount={focusLookRow}
        />
      ) : null}
      {onOpenApps ? (
        <NavRow
          label={t.apps.title}
          value={appsValue(t, apps)}
          warning={apps !== null && apps.storage.state !== "ok"}
          onClick={onOpenApps}
          focusOnMount={focusAppsRow}
        />
      ) : null}
      {onOpenItems ? (
        <NavRow
          label={t.widgets.items.title}
          value={itemsValue(t, widgets)}
          // The notice lives in the subview (spec 0003 §2.4 item 7).
          warning={widgets !== null && widgets.storage.state !== "ok"}
          onClick={onOpenItems}
          focusOnMount={focusItemsRow}
        />
      ) : null}
      <PanelFooter alert={error && t.settings.errors[error]}>
        {t.settings.footer}
      </PanelFooter>
    </>
  );
}

export default SettingsPanel;
