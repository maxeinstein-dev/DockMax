import { useRef, useState } from "react";
import {
  SIZE_PRESETS,
  isSettingsCommandError,
  updateSettings,
  type DockEdge,
  type DockOpacity,
  type Settings,
  type SettingsErrorCode,
  type SizePreset,
  type Theme,
} from "../core/settings";
import { useT } from "../i18n";
import OptionGroup, { type Option } from "./OptionGroup";
import PanelFooter from "./PanelFooter";

const THEMES: readonly Theme[] = ["system", "light", "dark"];
const EDGES: readonly DockEdge[] = ["bottom", "top"];
const SIZES: readonly SizePreset[] = ["small", "medium", "large"];
const OPACITIES: readonly DockOpacity[] = ["translucent", "solid"];

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

/** A change sent to Rust and not yet confirmed by `settings-changed`. */
interface Pending {
  /** Tells the latest change from older ones still answering. */
  id: number;
  /** The full settings sent. */
  value: Settings;
  /** Rust accepted it. */
  saved: boolean;
  /** New confirmed settings arrived (`settings-changed`) since it was
   * sent. */
  confirmed: boolean;
}

type SettingsPanelProps = {
  /** The settings in use (`useSettings`), or null while they can't be
   * read. The only source of the confirmed values. */
  settings: Settings | null;
};

/**
 * The Settings panel: one option group per setting, applied and saved on
 * every choice (spec 0002 §2.3 item 4, §2.4 item 3). A choice shows at once
 * while Rust saves it; if Rust refuses, the translated reason shows in the
 * footer and the options go back to the confirmed settings.
 */
function SettingsPanel({ settings }: SettingsPanelProps) {
  const t = useT();
  const [pending, setPending] = useState<Pending | null>(null);
  const [error, setError] = useState<SettingsErrorCode | null>(null);
  const lastId = useRef(0);
  const [lastSettings, setLastSettings] = useState(settings);

  // New confirmed settings: the pending change has its answer from
  // `settings-changed`, or will once saved (the event and the command's
  // answer may come in either order). A saved change is done then.
  if (lastSettings !== settings) {
    setLastSettings(settings);
    if (pending) {
      setPending(pending.saved ? null : { ...pending, confirmed: true });
    }
  }

  // Optimistic until both saved and confirmed.
  const shown =
    pending && !(pending.saved && pending.confirmed) ? pending.value : settings;

  function change(next: (current: Settings) => Settings) {
    setError(null);
    if (shown === null) {
      // Nothing to build the full object from.
      setError("notReady");
      return;
    }
    const value = next(shown);
    const id = ++lastId.current;
    setPending({ id, value, saved: false, confirmed: false });
    updateSettings(value).then(
      () => {
        setPending((current) => {
          if (current?.id !== id) return current;
          return current.confirmed ? null : { ...current, saved: true };
        });
      },
      (reason: unknown) => {
        // An older change answering late says nothing about the latest.
        if (id !== lastId.current) return;
        setPending(null);
        setError(isSettingsCommandError(reason) ? reason.code : "notReady");
      },
    );
  }

  const theme = shown?.theme ?? null;
  const edge = shown?.dock.edge ?? null;
  const size = shown ? sizePresetOf(shown.dock.iconSize) : null;
  const opacity = shown?.dock.opacity ?? null;

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
      <PanelFooter alert={error && t.settings.errors[error]}>
        {t.settings.footer}
      </PanelFooter>
    </>
  );
}

export default SettingsPanel;
