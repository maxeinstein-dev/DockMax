import { useId } from "react";
import { MAX_TRAY_IN_DOCK } from "../core/settings";
import { useTray, visibleIcons, withInDock } from "../core/tray";
import { format, useT } from "../i18n";
import OptionGroup from "./OptionGroup";
import "./parts/parts.css";

type Place = "panel" | "dock";

type TrayInDockFieldProps = {
  /** The tray icons the dock shows itself, by id, in dock order. */
  inDock: readonly string[];
  /** Called with the new list when an icon moves between the panel and the
   * dock. */
  onChange: (inDock: string[]) => void;
};

/**
 * Which tray icons the dock shows itself, at its end beside the battery,
 * the network and the volume (as the PrimoDock); the others stay behind the
 * arrow, in the tray panel. One row per icon in the tray now, with Painel ·
 * Dock; up to `MAX_TRAY_IN_DOCK` in the dock (a seventh is refused).
 */
function TrayInDockField({ inDock, onChange }: TrayInDockFieldProps) {
  const t = useT();
  const text = t.tray.inDock;
  const icons = useTray();
  const titleId = useId();
  // Only an icon with a lasting name can be kept across restarts.
  const shown =
    icons === null
      ? []
      : visibleIcons(icons).filter((icon) => icon.id !== null);
  const full = inDock.length >= MAX_TRAY_IN_DOCK;
  return (
    <section className="tray-in-dock" aria-labelledby={titleId}>
      <div className="toggle-text">
        <span id={titleId} className="toggle-label">
          {text.label}
        </span>
        <span className="toggle-hint">
          {format(text.hint, { max: MAX_TRAY_IN_DOCK })}
        </span>
      </div>
      {icons === null ? null : shown.length === 0 ? (
        <p className="toggle-hint">{text.none}</p>
      ) : (
        shown.map((icon) => {
          const id = icon.id ?? "";
          const selected: Place = inDock.includes(id) ? "dock" : "panel";
          return (
            <OptionGroup<Place>
              key={icon.key}
              label={icon.tip || t.tray.unnamed}
              options={[
                { value: "panel", label: text.panel },
                { value: "dock", label: text.dock },
              ]}
              selected={selected}
              onSelect={(place) => {
                if (place === "dock" && full) return;
                onChange(withInDock(inDock, id, place === "dock"));
              }}
            />
          );
        })
      )}
    </section>
  );
}

export default TrayInDockField;
